//! Q-Dir 主应用：状态聚合 + 每帧渲染入口 + 布局管理。
//!
//! 契约 A4：本文件是唯一持有子系统实例的地方。每帧流程：
//! 渲染（filters 着色 / panes 排序选择）→ 收集事件 → 统一执行
//! （PaneSet / shell / services 调用）→ dirty 时落盘（save_config/save_session）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui::{Color32, Ui};

use crate::config::{load_config, load_session, save_config, save_session};
use crate::core::{
    Config, FileItem, PaneId, SessionState, SortDir, SortKey, SplitPermille, ViewMode, sort_items,
};
use crate::filters::ListEngine;
use crate::panes::preview::PreviewData;
use crate::panes::{PaneSet, PaneViews, apply_tree_config};
use crate::core::ExportFormat;
use crate::services::export::{ExportOptions, write_export};
use crate::services::favorites::FavoriteStore;
use crate::services::i18n::Translator;
use crate::shell::{
    ContextMenu, EnumOptions, FileOpOptions, HostWindow, copy_to_clipboard, cut_to_clipboard,
    delete_files, list_directory, new_folder, open_default, paste_into, print_file, rename_path,
    show_properties,
};
use crate::shell::{IconCache, IconSize};

use crate::ui::pane::{NavKind, PaneEvent, RowPaint, Textures, format_filetime, icon_key, texture_for};

/// 窗口默认尺寸（Q-Dir.ini `WinRC=0;0;800;600`）。
pub const DEFAULT_WINDOW_SIZE: [f32; 2] = [800.0, 600.0];
pub const WINDOW_TITLE: &str = crate::brand::PRODUCT_NAME;

/// 窗格数布局模式（原版「窗口」菜单的 1-4 窗格切换）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    One,
    Two,
    Three,
    Four,
}

impl LayoutMode {
    fn pane_ids(self) -> &'static [PaneId] {
        match self {
            LayoutMode::One => &[PaneId::P1],
            LayoutMode::Two => &[PaneId::P1, PaneId::P2],
            LayoutMode::Three => &[PaneId::P1, PaneId::P2, PaneId::P3],
            LayoutMode::Four => &[PaneId::P1, PaneId::P2, PaneId::P3, PaneId::P4],
        }
    }
}

/// 帧末统一执行的操作（渲染期间只收集，避免借用冲突）。
enum Pending {
    Navigate(PaneId, PathBuf),
    Nav(PaneId, NavKind),
    Refresh(PaneId),
    Open(PaneId, PathBuf),
    NewFolder(PaneId),
    Delete(PaneId),
    Rename(PaneId, String),
    Properties(PaneId),
    CopySel(PaneId, bool),
    Paste(PaneId),
    ContextMenu(PaneId, egui::Pos2),
    Sort(PaneId, SortKey),
    TreeExpand(PaneId, PathBuf),
    Export(ExportFormat),
    AddFavorite,
}

/// 主应用。
pub struct QDirApp {
    pub(crate) config: Config,
    pub(crate) session: SessionState,
    pub(crate) views: PaneViews,
    pub(crate) engine: ListEngine,
    pub(crate) icons: IconCache,
    pub(crate) tr: Translator,
    pub(crate) favorites: FavoriteStore,

    pub(crate) layout: LayoutMode,
    pub(crate) show_grid_lines: bool,

    /// 每窗格最近一次枚举并排序的条目。
    pub(crate) items: HashMap<PaneId, Vec<FileItem>>,
    /// 图标纹理缓存。
    pub(crate) textures: Textures,
    /// 地址栏编辑草稿。
    pub(crate) addr_drafts: HashMap<PaneId, String>,
    /// 设置对话框开关。
    pub(crate) settings_open: bool,
    /// 状态栏错误（帧末清除）。
    pub(crate) status_error: Option<String>,
    pub(crate) start_ms: u64,
    pub(crate) dirty: bool,
}

impl QDirApp {
    /// 由 main.rs 组装（配置读取失败已在 main 处理）。
    pub fn new(config: Config, session: SessionState) -> Self {
        let engine = ListEngine::from_config(&config);
        let tr = Translator::from_config(&config);
        let favorites = FavoriteStore::from_config(&config);
        let views = PaneViews::from_session(&session, config.tree.clone());
        QDirApp {
            config,
            session,
            views,
            engine,
            icons: IconCache::new(),
            tr,
            favorites,
            layout: LayoutMode::Four,
            show_grid_lines: false,
            items: HashMap::new(),
            textures: HashMap::new(),
            addr_drafts: HashMap::new(),
            settings_open: false,
            status_error: None,
            start_ms: now_ms(),
            dirty: false,
        }
    }

    // ---------- 数据 ----------

    /// 确保窗格条目已枚举并按当前排序键排好。
    pub(crate) fn ensure_items(&mut self, id: PaneId) {
        if self.items.contains_key(&id) {
            return;
        }
        let path = self.session.pane(id).path();
        let opts = EnumOptions::default();
        let mut items = match list_directory(&path, &opts) {
            Ok(v) => v,
            Err(e) => {
                self.status_error = Some(e.to_string());
                Vec::new()
            }
        };
        let pane = self.session.pane(id);
        let tab = pane.current_tab();
        let (key, dir) = match tab {
            Some(t) => (t.sort_key, t.sort_dir),
            None => (SortKey::Name, SortDir::Asc),
        };
        sort_items(&mut items, key, dir);
        self.items.insert(id, items);
    }

    fn selected_paths(&self, id: PaneId) -> Vec<PathBuf> {
        let Some(items) = self.items.get(&id) else {
            return Vec::new();
        };
        let view = self.views.get(id);
        items
            .iter()
            .filter(|i| view.is_selected(&i.name))
            .map(|i| i.path.clone())
            .collect()
    }

    fn with_pane<R>(
        &mut self,
        id: PaneId,
        f: impl FnOnce(&mut crate::panes::PaneController<'_>) -> R,
    ) -> R {
        let mut set = PaneSet::new(&mut self.session, &mut self.views);
        let mut c = set.pane(id);
        f(&mut c)
    }

    fn invalidate(&mut self, id: PaneId) {
        self.items.remove(&id);
        self.dirty = true;
    }

    pub(crate) fn hwnd(&self, ctx: &egui::Context) -> HostWindow {
        HostWindow::from_raw_hwnd(hwnd_of(ctx))
    }

    fn save_all(&mut self) {
        if let Err(e) = save_config(Path::new("Q-Dir.ini"), &self.config) {
            self.status_error = Some(e.to_string());
        }
        if let Err(e) = save_session(Path::new("start.qdr"), &self.session) {
            self.status_error = Some(e.to_string());
        }
        self.dirty = false;
    }

    // ---------- 帧内渲染 ----------

    fn menu_bar(&mut self, ui: &mut Ui) -> Vec<Pending> {
        let mut out = Vec::new();
        let m_file = self.tr.tr("menu.file").to_string();
        let m_edit = self.tr.tr("menu.edit").to_string();
        let m_view = self.tr.tr("menu.view").to_string();
        let m_fav = self.tr.tr("menu.favorites").to_string();
        let m_win = self.tr.tr("menu.window").to_string();
        let m_help = self.tr.tr("menu.help").to_string();
        egui::containers::menu::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(&m_file, |ui| {
                if ui.button("新建文件夹").clicked() {
                    out.push(Pending::NewFolder(self.session.focus));
                }
                if ui.button("重命名 (F2)").clicked() {
                    let sel = self.selected_paths(self.session.focus);
                    if let Some(p) = sel.first() {
                        if let Some(name) = p.file_name().map(|n| n.to_string_lossy().to_string()) {
                            out.push(Pending::Rename(self.session.focus, name));
                        }
                    }
                }
                if ui.button("删除 (Del)").clicked() {
                    out.push(Pending::Delete(self.session.focus));
                }
                if ui.button("属性").clicked() {
                    out.push(Pending::Properties(self.session.focus));
                }
                if ui.button("打印").clicked() {
                    let hwnd = self.hwnd(ui.ctx());
                    if let Some(p) = self.selected_paths(self.session.focus).first().cloned() {
                        if let Err(e) = print_file(hwnd, &p) {
                            self.status_error = Some(e.to_string());
                        }
                    }
                    ui.close();
                }
                if ui.button("退出").clicked() {
                    self.save_all();
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(&m_edit, |ui| {
                if ui.button("复制 (Ctrl+C)").clicked() {
                    out.push(Pending::CopySel(self.session.focus, false));
                }
                if ui.button("剪切 (Ctrl+X)").clicked() {
                    out.push(Pending::CopySel(self.session.focus, true));
                }
                if ui.button("粘贴 (Ctrl+V)").clicked() {
                    out.push(Pending::Paste(self.session.focus));
                }
                ui.separator();
                if ui.button("全选 (Ctrl+A)").clicked() {
                    let names: Vec<String> = self
                        .items
                        .get(&self.session.focus)
                        .map(|v| v.iter().map(|i| i.name.clone()).collect())
                        .unwrap_or_default();
                    self.views.get_mut(self.session.focus).set_selection(&names);
                }
                if ui.button("反选").clicked() {
                    let Some(items) = self.items.get(&self.session.focus) else {
                        return;
                    };
                    let view = self.views.get(self.session.focus);
                    let names: Vec<String> = items
                        .iter()
                        .filter(|i| !view.is_selected(&i.name))
                        .map(|i| i.name.clone())
                        .collect();
                    self.views.get_mut(self.session.focus).set_selection(&names);
                }
            });
            ui.menu_button(&m_view, |ui| {
                let cur = self.session.focused().view_mode();
                for (label, mode) in [
                    ("大图标", ViewMode::LargeIcon),
                    ("小图标", ViewMode::SmallIcon),
                    ("列表", ViewMode::List),
                    ("详情", ViewMode::Detail),
                ] {
                    if ui.radio(cur == mode, label).clicked() {
                        self.with_pane(self.session.focus, |c| c.set_view_mode(mode));
                        self.dirty = true;
                    }
                }
                ui.separator();
                let mut tree_on = self.session.tree_mode.is_active();
                if ui.checkbox(&mut tree_on, "显示目录树").clicked() {
                    // 复选框 = 开关；勾选恢复上次模式（默认双列），取消 = 关。
                    self.session.tree_mode = if tree_on {
                        if self.session.tree_mode == crate::core::TreeMode::Off {
                            crate::core::TreeMode::Split
                        } else {
                            self.session.tree_mode
                        }
                    } else {
                        crate::core::TreeMode::Off
                    };
                    // 同步四个窗格的树可见性。
                    for i in 1..=4u32 {
                        if let Some(pid) = PaneId::from_number(i) {
                            self.session.pane_mut(pid).tree_visible = tree_on;
                        }
                    }
                    self.dirty = true;
                }
                if ui
                    .checkbox(&mut self.session.preview_visible, "显示预览")
                    .clicked()
                {
                    self.dirty = true;
                }
                if ui.checkbox(&mut self.show_grid_lines, "网格线").clicked() {}
                ui.separator();
                ui.menu_button("窗格数", |ui| {
                    for (label, m) in [
                        ("1", LayoutMode::One),
                        ("2", LayoutMode::Two),
                        ("3", LayoutMode::Three),
                        ("4", LayoutMode::Four),
                    ] {
                        if ui.radio(self.layout == m, label).clicked() {
                            self.layout = m;
                        }
                    }
                });
                if ui.button("刷新 (F5)").clicked() {
                    out.push(Pending::Refresh(self.session.focus));
                }
            });
            ui.menu_button(&m_fav, |ui| {
                if ui.button("添加当前目录到收藏").clicked() {
                    out.push(Pending::AddFavorite);
                }
                let quick = self.favorites.load_quick_links();
                let favs = self.favorites.load_favoriten();
                if !quick.is_empty() {
                    ui.separator();
                    for f in &quick {
                        if ui.button(f.name()).clicked() {
                            out.push(Pending::Navigate(self.session.focus, f.favorite.path.clone()));
                        }
                    }
                }
                if !favs.is_empty() {
                    ui.separator();
                    for f in &favs {
                        if ui.button(f.name()).clicked() {
                            out.push(Pending::Navigate(self.session.focus, f.favorite.path.clone()));
                        }
                    }
                }
            });
            ui.menu_button(&m_win, |ui| {
                if ui.button("导出列表 TXT").clicked() {
                    out.push(Pending::Export(ExportFormat::Txt));
                }
                if ui.button("导出列表 CSV").clicked() {
                    out.push(Pending::Export(ExportFormat::Csv));
                }
                if ui.button("导出列表 HTML").clicked() {
                    out.push(Pending::Export(ExportFormat::Html));
                }
                if ui.button("导出列表 XLS").clicked() {
                    out.push(Pending::Export(ExportFormat::Xls));
                }
            });
            ui.menu_button(&m_help, |ui| {
                if ui.button("设置…").clicked() {
                    self.settings_open = true;
                }
            });
        });
        out
    }

    fn toolbar(&mut self, ui: &mut Ui) -> Vec<Pending> {
        let mut out = Vec::new();
        let id = self.session.focus;
        ui.horizontal(|ui| {
            if ui.button("◀").clicked() {
                out.push(Pending::Nav(id, NavKind::Back));
            }
            if ui.button("▶").clicked() {
                out.push(Pending::Nav(id, NavKind::Forward));
            }
            if ui.button("▲").clicked() {
                out.push(Pending::Nav(id, NavKind::Up));
            }
            if ui.button("⟳").clicked() {
                out.push(Pending::Refresh(id));
            }
            if ui.button("📁+").clicked() {
                out.push(Pending::NewFolder(id));
            }
            ui.separator();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.config.last_filter)
                        .hint_text("过滤…")
                        .desired_width(160.0),
                )
                .changed()
            {
                self.engine = ListEngine::from_config(&self.config);
                self.dirty = true;
            }
        });
        out
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let id = self.session.focus;
            let path = self.session.pane(id).path();
            ui.label(path.display().to_string());
            ui.separator();
            if let Some(items) = self.items.get(&id) {
                let view = self.views.get(id);
                let sel_n = items.iter().filter(|i| view.is_selected(&i.name)).count();
                let sel_bytes: u64 = items
                    .iter()
                    .filter(|i| view.is_selected(&i.name))
                    .map(|i| i.size)
                    .sum();
                ui.label(format!(
                    "已选 {sel_n} / {} 项 · {}",
                    items.len(),
                    human_size(sel_bytes)
                ));
            }
            if let Some(err) = self.status_error.take() {
                ui.separator();
                ui.colored_label(Color32::LIGHT_RED, err);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!(
                    "L-Dir v0 {} msek",
                    now_ms().saturating_sub(self.start_ms)
                ));
            });
        });
    }

    /// 中央四窗格区：竖直分割条按 splits_h 切分；每窗格内部
    /// 树/列表按 pane.tree_split、列表/预览按 preview_pos 切分。
    fn pane_area(&mut self, ui: &mut Ui) -> Vec<Pending> {
        let ids: Vec<PaneId> = self.layout.pane_ids().to_vec();
        let mut out = Vec::new();

        match ids.len() {
            1 => out.extend(self.render_one_pane(ui, ids[0])),
            2 => {
                // 上下分割（egui 0.37 无 Splitter，用 TopBottomPanel::resizable）。
                egui::containers::panel::Panel::top("sp0")
                    .resizable(true)
                    .min_size(80.0)
                    .show_inside(ui, |ui| {
                        out.extend(self.render_one_pane(ui, ids[0]));
                    });
                out.extend(self.render_one_pane(ui, ids[1]));
            }
            _ => {
                // 3/4 窗格：两行布局。上行 P1/P2，下行 P3/P4（3 窗格下行只有 P3）。
                let (r0, r1) = (self.session.splits_h[1].ratio(), self.session.splits_h[2].ratio());
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::containers::panel::Panel::top("row0")
                        .resizable(true)
                        .min_size(80.0)
                        .size_range(80.0..=f32::INFINITY)
                        .show_inside(ui, |ui| {
                            self.hsplit_inside(ui, "row0-h", ids[0], ids.get(1).copied(), r0, &mut out);
                        });
                    egui::CentralPanel::default().show(ui, |ui| {
                        match ids.get(2) {
                            Some(p3) => {
                                self.hsplit_inside(ui, "row1-h", *p3, ids.get(3).copied(), r1, &mut out);
                            }
                            None => {}
                        }
                    });
                });
            }
        }
        out
    }

    fn hsplit_inside(
        &mut self,
        ui: &mut Ui,
        tag: &str,
        left: PaneId,
        right: Option<PaneId>,
        ratio: f32,
        out: &mut Vec<Pending>,
    ) {
        match right {
            None => out.extend(self.render_one_pane(ui, left)),
            Some(r) => {
                // 简易可拖拽分割：egui 原生 SidePanel::show_inside。
                egui::containers::panel::Panel::right(format!("split-{tag}-{r:?}"))
                    .resizable(true)
                    .exact_size(ui.available_width() * ratio.clamp(0.1, 0.9))
                    .min_size(120.0)
                    .show_inside(ui, |ui| {
                        out.extend(self.render_one_pane(ui, r));
                    });
                out.extend(self.render_one_pane(ui, left));
            }
        }
        let _ = SplitPermille(0); // 保持引用
    }

    /// 单窗格完整渲染：地址栏/标签栏/树+列表/预览。
    fn render_one_pane(&mut self, ui: &mut Ui, id: PaneId) -> Vec<Pending> {
        let mut events: Vec<PaneEvent> = Vec::new();
        self.ensure_items(id);

        // 键盘导航（Alt+方向 / Backspace）。
        crate::ui::pane::global_shortcuts(ui, id, &mut events);

        egui::Grid::new(("pane-frame", id.number()))
            .num_columns(1)
            .show(ui, |ui| {
                // 地址栏 + 标签栏。
                let current = self.session.pane(id).path();
                let mut draft_opt = self.addr_drafts.remove(&id);
                crate::ui::pane::address_bar(ui, id, &current, &mut draft_opt, &mut events);
                if let Some(d) = draft_opt.take() {
                    self.addr_drafts.insert(id, d);
                }

                // 标签栏：通过控制器结算。
                {
                    let mut set = PaneSet::new(&mut self.session, &mut self.views);
                    let mut c = set.pane(id);
                    let tab_cfg = self.config.tab.clone();
                    crate::ui::pane::tab_bar(ui, &mut c, &tab_cfg, id, &mut events);
                }

                // 主体：树 | 列表 | 预览 三段。
                let tree_visible = self.session.pane(id).tree_visible;
                let preview_visible = self.session.preview_visible;
                let tree_ratio = (self.session.pane(id).tree_split as f32 / 1000.0).clamp(0.05, 0.6);
                let preview_ratio = self.session.preview_pos.ratio();

                ui.push_id(("body", id.number()), |ui| {
                    if preview_visible {
                        egui::containers::panel::Panel::right(format!("preview-{id:?}"))
                            .resizable(true)
                            .exact_size(ui.available_width() * (1.0 - preview_ratio).clamp(0.15, 0.5))
                            .min_size(120.0)
                            .show_inside(ui, |ui| {
                                let data = self.preview_data(id);
                                crate::ui::pane::preview_pane(ui, data.as_ref());
                            });
                    }
                    if tree_visible {
                        egui::containers::panel::Panel::left(format!("tree-{id:?}"))
                            .resizable(true)
                            .exact_size(ui.available_width() * tree_ratio)
                            .min_size(100.0)
                            .show_inside(ui, |ui| {
                                let mut set = PaneSet::new(&mut self.session, &mut self.views);
                                let mut c = set.pane(id);
                                crate::ui::pane::tree_view(ui, &mut c, id, &mut events);
                            });
                    }
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE)
                        .show_inside(ui, |ui| {
                            crate::ui::pane::details_header(ui, id, &mut events);
                            self.render_file_list(ui, id, &mut events);
                        });
                });
            });

        // 翻译事件为 Pending。
        let mut out = Vec::new();
        for ev in events {
            out.extend(self.translate_event(ev));
        }
        out
    }

    fn preview_data(&mut self, id: PaneId) -> Option<PreviewData> {
        let items = self.items.get(&id)?;
        let view = self.views.get(id);
        let name = view.selection.first()?;
        let item = items.iter().find(|i| &i.name == name)?;
        if item.is_dir {
            return None;
        }
        let cfg = self.config.clone();
        crate::panes::preview::build_preview(&item.path, &cfg.preview_filter, None).ok()
    }

    /// 文件列表（按 engine 过滤+着色，按视图模式渲染）。
    fn render_file_list(&mut self, ui: &mut Ui, id: PaneId, events: &mut Vec<PaneEvent>) {
        let all = match self.items.get(&id) {
            Some(v) => v.clone(),
            None => return,
        };
        // 过滤 + 着色。
        let visible: Vec<(usize, &FileItem)> = all
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                let f = self.config.last_filter.trim();
                f.is_empty() || item.name_lower().contains(&f.to_lowercase())
            })
            .collect();
        let paints: Vec<RowPaint> = visible
            .iter()
            .map(|(vis_idx, item)| {
                let row = self.engine.row(item, *vis_idx);
                RowPaint {
                    fg: row.colors.fg.map(crate::ui::app::rgb_to_color32),
                    bg: row.colors.bg.map(crate::ui::app::rgb_to_color32),
                }
            })
            .collect();

        let mode = self.session.focused().view_mode();
        let grid = self.show_grid_lines;
        let icons = &mut self.icons;
        let textures = &mut self.textures;
        let mut set = PaneSet::new(&mut self.session, &mut self.views);
        let mut c = set.pane(id);

        match mode {
            ViewMode::Detail => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new(("details", id.number()))
                        .striped(grid)
                        .num_columns(4)
                        .min_col_width(70.0)
                        .show(ui, |ui| {
                            for (row, &(vis_idx, item)) in visible.iter().enumerate() {
                                let paint = paints.get(row).copied().unwrap_or_default();
                                crate::ui::pane::details_row(ui, &mut c, vis_idx, item, paint, id, events);
                            }
                        });
                });
            }
            _ => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new(("list", id.number()))
                        .striped(grid)
                        .num_columns(1)
                        .show(ui, |ui| {
                            for (row, &(vis_idx, item)) in visible.iter().enumerate() {
                                let paint = paints.get(row).copied().unwrap_or_default();
                                // 图标（大图标模式放大一格）。
                                if mode == ViewMode::LargeIcon || mode == ViewMode::SmallIcon {
                                    let size = if mode == ViewMode::LargeIcon {
                                        IconSize::Large
                                    } else {
                                        IconSize::Small
                                    };
                                    let key = icon_key(item);
                                    let img = icons.image_for_index(item.icon_index.unwrap_or(-1), size)
                                        .or_else(|_| icons.image_for_path(&item.path, size))
                                        .unwrap_or_else(|_| crate::shell::IconImage::solid(size, [128, 128, 128, 255]));
                                    if let Some(tid) = texture_for(ui, key, &img, textures) {
                                        ui.image((tid, egui::vec2(16.0, 16.0)));
                                    }
                                }
                                crate::ui::pane::simple_row(ui, &mut c, vis_idx, item, paint, id, events);
                            }
                        });
                });
            }
        }
    }

    fn translate_event(&mut self, ev: PaneEvent) -> Vec<Pending> {
        match ev {
            PaneEvent::Activate(id, path, is_dir) => {
                if is_dir {
                    vec![Pending::Navigate(id, path)]
                } else {
                    vec![Pending::Open(id, path)]
                }
            }
            PaneEvent::ContextMenu(id, pos) => vec![Pending::ContextMenu(id, pos)],
            PaneEvent::Goto(id, p) => vec![Pending::Navigate(id, p)],
            PaneEvent::Sort(id, key) => vec![Pending::Sort(id, key)],
            PaneEvent::TabSettled(id) => {
                self.invalidate(id);
                Vec::new()
            }
            PaneEvent::TreeExpand(id, path) => vec![Pending::TreeExpand(id, path)],
            PaneEvent::TreeSelect(id, path) => vec![Pending::Navigate(id, path)],
            PaneEvent::Nav(id, kind) => vec![Pending::Nav(id, kind)],
        }
    }

    // ---------- 设置对话框 ----------

    fn settings_window(&mut self, ctx: &egui::Context) {
        // 用菜单帮助里的"设置…"入口替代独立开关：原版菜单「帮助」。
        if !self.settings_open {
            return;
        }
        egui::Window::new("设置")
            .resizable(true)
            .default_width(520.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    ui.collapsing("常规", |ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.config.langid)
                                .range(1031..=2052)
                                .prefix("langid="),
                        );
                        ui.text_edit_singleline(&mut self.config.font_face_name);
                        ui.add(egui::DragValue::new(&mut self.config.colorthema));
                        ui.checkbox(&mut self.config.columns_plus_view, "columns_plus_view");
                        ui.checkbox(&mut self.config.show_filter, "show_filter");
                    });
                    ui.collapsing("标签页", |ui| {
                        let t = &mut self.config.tab;
                        ui.checkbox(&mut t.new_tab_l_dblclk, "tab_new_tab_l_dblclk");
                        ui.checkbox(&mut t.new_tab_l_clk, "tab_new_tab_l_clk");
                        ui.checkbox(&mut t.show_tooltips, "tab_show_tooltips");
                        ui.checkbox(&mut t.close_by_mbt_clk, "tab_close_by_mbt_clk");
                        ui.checkbox(&mut t.close_by_lbt_dblclk, "tab_close_by_lbt_dblclk");
                        ui.add(egui::DragValue::new(&mut t.max_width).prefix("tab_max_width="));
                        ui.add(
                            egui::DragValue::new(&mut t.width_mode)
                                .range(0..=1)
                                .prefix("tab_width_mode="),
                        );
                        ui.checkbox(&mut t.use_context_menu, "tab_use_context_menu");
                    });
                    ui.collapsing("目录树", |ui| {
                        let t = &mut self.config.tree;
                        ui.checkbox(&mut t.no_expando, "tree_no_expando");
                        ui.checkbox(&mut t.single_expand, "tree_single_expand");
                        ui.checkbox(&mut t.fullrowselect, "tree_fullrowselect");
                        ui.checkbox(&mut t.expand_when_select, "tree_expand_when_select");
                        ui.checkbox(&mut t.all_browsable, "tree_all_browsable");
                        ui.checkbox(&mut t.line_at_root, "tree_line_at_root");
                        ui.checkbox(&mut t.disable_line, "tree_disable_line");
                        ui.checkbox(&mut t.auto_chk_subdir, "tree_auto_chk_subdir");
                        ui.add(egui::DragValue::new(&mut t.indent_width).prefix("tree_indent_width="));
                        ui.checkbox(&mut t.auto_expand_from_list, "tree_auto_expand_from_list");
                        ui.checkbox(&mut t.auto_sel, "tree_auto_sel");
                        ui.add(egui::DragValue::new(&mut t.hover_time).prefix("tree_hover_time="));
                    });
                    ui.collapsing("文件操作", |ui| {
                        let f = &mut self.config.file_ops;
                        ui.add(
                            egui::DragValue::new(&mut f.copy_cut_paste_mode)
                                .range(0..=1)
                                .prefix("copy_cut_paste_mode="),
                        );
                        ui.checkbox(&mut f.move_file_confirmation, "move_file_confirmation");
                        ui.checkbox(&mut f.refresh_keep_sel, "refresh_keep_sel");
                        ui.checkbox(&mut f.refresh_keep_scroll, "refresh_keep_scroll");
                        ui.checkbox(&mut f.select_last_if_navigate_up, "select_last_if_navigate_up");
                    });
                    ui.collapsing("过滤器", |ui| {
                        ui.text_edit_singleline(&mut self.config.markierungsfilter);
                        ui.text_edit_singleline(&mut self.config.last_file_filter);
                        ui.text_edit_singleline(&mut self.config.by_filter);
                        ui.text_edit_singleline(&mut self.config.by_dir_filter);
                        ui.text_edit_singleline(&mut self.config.fav_filter);
                    });
                    ui.separator();
                    if ui.button("保存设置").clicked() {
                        self.engine = ListEngine::from_config(&self.config);
                        self.tr = Translator::from_config(&self.config);
                        self.favorites = FavoriteStore::from_config(&self.config);
                        apply_tree_config(&mut self.views, &self.config.tree);
                        self.save_all();
                        self.settings_open = false;
                    }
                });
            });
    }

    // ---------- 帧末执行 ----------

    fn run_pending(&mut self, ctx: &egui::Context, pending: Vec<Pending>) {
        let hwnd = self.hwnd(ctx);
        for p in pending {
            match p {
                Pending::Navigate(id, path) => {
                    let ops = self.config.file_ops.clone();
                    if let Err(e) = self.with_pane(id, |c| c.navigate_to(&path, &ops)) {
                        self.status_error = Some(e.to_string());
                    } else {
                        self.addr_drafts.remove(&id);
                        self.invalidate(id);
                    }
                }
                Pending::Nav(id, kind) => {
                    let ops = self.config.file_ops.clone();
                    let r = match kind {
                        NavKind::Back => self.with_pane(id, |c| c.back(&ops)).map(|_| ()),
                        NavKind::Forward => self.with_pane(id, |c| c.forward(&ops)).map(|_| ()),
                        NavKind::Up => match self.with_pane(id, |c| c.up(&ops)) {
                            Ok(Some(name)) => {
                                self.views.get_mut(id).select_one(&name);
                                Ok(())
                            }
                            Ok(None) => Ok(()),
                            Err(e) => Err(e),
                        },
                    };
                    match r {
                        Ok(()) => self.invalidate(id),
                        Err(e) => self.status_error = Some(e.to_string()),
                    }
                }
                Pending::Refresh(id) => self.invalidate(id),
                Pending::Open(_id, path) => {
                    if let Err(e) = open_default(hwnd, &path) {
                        self.status_error = Some(e.to_string());
                    }
                }
                Pending::NewFolder(id) => {
                    let parent = self.session.pane(id).path();
                    match new_folder(hwnd, &parent, "新建文件夹") {
                        Ok(_) => self.invalidate(id),
                        Err(e) => self.status_error = Some(e.to_string()),
                    }
                }
                Pending::Delete(id) => {
                    let paths = self.selected_paths(id);
                    if paths.is_empty() {
                        continue;
                    }
                    match delete_files(hwnd, &paths, &FileOpOptions::default()) {
                        Ok(_) => self.invalidate(id),
                        Err(e) => self.status_error = Some(e.to_string()),
                    }
                }
                Pending::Rename(id, new_name) => {
                    let sel = self.selected_paths(id);
                    if let Some(src) = sel.first() {
                        if let Err(e) =
                            rename_path(hwnd, src, &new_name, &FileOpOptions::default())
                        {
                            self.status_error = Some(e.to_string());
                        }
                        self.invalidate(id);
                    }
                }
                Pending::Properties(id) => {
                    for p in self.selected_paths(id) {
                        if let Err(e) = show_properties(hwnd, &p) {
                            self.status_error = Some(e.to_string());
                            break;
                        }
                    }
                }
                Pending::CopySel(id, cut) => {
                    let paths = self.selected_paths(id);
                    if paths.is_empty() {
                        continue;
                    }
                    let r = if cut {
                        cut_to_clipboard(hwnd, &paths)
                    } else {
                        copy_to_clipboard(hwnd, &paths)
                    };
                    if let Err(e) = r {
                        self.status_error = Some(e.to_string());
                    }
                }
                Pending::Paste(id) => {
                    let dst = self.session.pane(id).path();
                    if let Err(e) = paste_into(hwnd, &dst, false) {
                        self.status_error = Some(e.to_string());
                    }
                    self.invalidate(id);
                }
                Pending::ContextMenu(id, _pos) => {
                    let paths = self.selected_paths(id);
                    if paths.is_empty() {
                        continue;
                    }
                    match ContextMenu::query_and_run(&paths, hwnd) {
                        Ok(_) => self.invalidate(id),
                        Err(crate::core::QError::Cancelled) => {}
                        Err(e) => self.status_error = Some(e.to_string()),
                    }
                }
                Pending::Sort(id, key) => {
                    self.with_pane(id, |c| c.toggle_sort(key));
                    self.invalidate(id);
                }
                Pending::TreeExpand(id, path) => {
                    // 枚举子目录喂 TreeModel::expand。
                    let dirs = list_directory(&path, &EnumOptions::default())
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|i| i.is_dir)
                        .map(|i| i.path)
                        .collect();
                    if let Err(e) = self.with_pane(id, |c| c.view_tree_mut().expand(&path, dirs)) {
                        self.status_error = Some(e.to_string());
                    }
                }
                Pending::Export(fmt) => self.export_current(fmt),
                Pending::AddFavorite => {
                    let id = self.session.focus;
                    let path = self.session.pane(id).path();
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "收藏".to_string());
                    if let Err(e) = self.favorites.add(&path, &name, None) {
                        self.status_error = Some(e.to_string());
                    }
                }
            }
        }
    }

    fn export_current(&mut self, fmt: ExportFormat) {
        let id = self.session.focus;
        self.ensure_items(id);
        let Some(items) = self.items.get(&id).cloned() else {
            return;
        };
        let opts = ExportOptions::default();
        let file = std::env::temp_dir().join(format!("qdir-export-{}.{}", now_ms(), fmt.ext()));
        match write_export(&file, &items, fmt, &opts) {
            Ok(()) => {
                if let Err(e) = open_default(self.hwnd_of_pub(), &file) {
                    self.status_error = Some(e.to_string());
                }
            }
            Err(e) => self.status_error = Some(e.to_string()),
        }
    }

    fn hwnd_of_pub(&self) -> HostWindow {
        HostWindow::from_raw_hwnd(0)
    }
}

impl eframe::App for QDirApp {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        let mut pending: Vec<Pending> = Vec::new();

        egui::containers::panel::Panel::top("menu").show(root, |ui| {
            pending.extend(self.menu_bar(ui));
        });
        egui::containers::panel::Panel::top("toolbar").show(root, |ui| {
            pending.extend(self.toolbar(ui));
        });
        egui::containers::panel::Panel::bottom("status").show(root, |ui| {
            self.status_bar(ui);
        });

        egui::CentralPanel::default().show(root, |ui| {
            pending.extend(self.pane_area(ui));
        });

        let ctx = root.ctx().clone();
        self.settings_window(&ctx);
        self.run_pending(&ctx, pending);

        if self.dirty {
            self.dirty = false;
            self.save_all();
        }
    }

    fn on_exit(&mut self) {
        // 退出持久化（复核员指出 save_config/save_session 无调用者——修复点）。
        self.save_all();
    }
}

// ---------- 工具 ----------

/// RGB（Filter2）→ egui 颜色。
pub fn rgb_to_color32(c: crate::core::Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

/// 字节数人读格式。
pub fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    match bytes {
        0..=1023 => format!("{bytes} B"),
        b if b < MB => format!("{:.1} KB", b as f64 / KB as f64),
        b if b < GB => format!("{:.1} MB", b as f64 / MB as f64),
        b => format!("{:.2} GB", b as f64 / GB as f64),
    }
}

/// 系统毫秒时钟（"Q-Dir v12 %d msek" 状态栏）。
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 取当前窗口 HWND（供 Shell API 使用）。
///
/// eframe 0.37 的 Context 不直接暴露 raw-window-handle；
/// 通过 egui 内部 viewport info 的透明通道取，失败回 0
/// （多数 Shell API 对 NULL HWND 仍可工作）。
pub fn hwnd_of(_ctx: &egui::Context) -> isize {
    // eframe/egui 不公开 HWND；由 main.rs 创建窗口后经 set_hwnd 注入。
    HWND_SLOT.with(|s| s.get())
}

thread_local! {
    static HWND_SLOT: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
}

/// main.rs 创建窗口后调用：注入真实 HWND 供右键菜单/拖放/托盘。
pub fn set_hwnd(v: isize) {
    HWND_SLOT.with(|s| s.set(v));
}

/// 供 main.rs 在退出时保存（on_exit 已覆盖；此函数留给外部触发）。
pub fn format_filetime_pub(ft: u64) -> String {
    format_filetime(ft)
}


/// egui 对照路线入口（原 main.rs 逻辑迁入）。
pub fn run_egui(
    config: crate::core::Config,
    session: crate::core::SessionState,
) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size(DEFAULT_WINDOW_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(move |_cc| Ok(Box::new(QDirApp::new(config, session)))),
    )
    .map(|_| ())
}
