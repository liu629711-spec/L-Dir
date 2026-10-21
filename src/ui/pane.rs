//! 单窗格渲染：地址栏、标签页条、目录树、文件列表、预览面板。
//!
//! 只做渲染与事件收集，不持有状态——状态全在 app.rs 的 QDirApp，
//! 通过参数/控制器借用传入。事件以 [`PaneEvent`] 返回，由 app 统一执行。

use std::path::{Path, PathBuf};

use egui::{Color32, RichText, ScrollArea, Sense, TextEdit, Ui};

use crate::core::{FileItem, PaneId, SortKey};
use crate::panes::tree::TreeNode;
use crate::panes::preview::PreviewData;
use crate::panes::PaneController;
use crate::panes::{TabAction, TabClick};
use crate::shell::{IconCache, IconImage, IconSize};

/// 纹理缓存（icon_key → 纹理句柄）。app 持有，这里只借用。
pub type Textures = std::collections::HashMap<u32, egui::TextureHandle>;

/// 每窗格每帧产出的事件（app 侧翻译成具体操作执行）。
#[derive(Debug, Clone)]
pub enum PaneEvent {
    /// 双击条目 → 打开（is_dir=true 则进入目录）。
    Activate(PaneId, PathBuf, bool),
    /// 右键 → 系统上下文菜单。
    ContextMenu(PaneId, egui::Pos2),
    /// 地址栏回车跳转。
    Goto(PaneId, PathBuf),
    /// 表头点击 → 翻转排序。
    Sort(PaneId, SortKey),
    /// 标签动作已由控制器结算，列表需重枚举。
    TabSettled(PaneId),
    /// 树节点展开请求（app 枚举子目录后喂 TreeModel::expand）。
    TreeExpand(PaneId, PathBuf),
    /// 树节点选中 → 导航。
    TreeSelect(PaneId, PathBuf),
    /// 键盘导航（Alt+←/→、Backspace）。
    Nav(PaneId, NavKind),
}

/// 一行的着色结果（app 侧由 ListEngine 算好）。
#[derive(Clone, Copy, Default)]
pub struct RowPaint {
    pub fg: Option<Color32>,
    pub bg: Option<Color32>,
}

/// 把 IconImage 注册/取回为 egui 纹理。
pub fn texture_for(
    ui: &Ui,
    key: u32,
    img: &IconImage,
    textures: &mut Textures,
) -> Option<egui::TextureId> {
    if !textures.contains_key(&key) {
        let size = [img.width.max(1) as usize, img.height.max(1) as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, &img.pixels);
        let handle = ui
            .ctx()
            .load_texture(format!("icon-{key}"), color, Default::default());
        textures.insert(key, handle);
    }
    textures.get(&key).map(|t| t.id())
}

/// 取条目图标纹理（未命中时同步提取一次并缓存）。
pub fn item_texture(
    ui: &Ui,
    icons: &mut IconCache,
    textures: &mut Textures,
    item: &FileItem,
    size: IconSize,
) -> Option<egui::TextureId> {
    let key = match item.icon_index {
        Some(idx) => icons.key_for_index(idx),
        None => return None,
    };
    if !textures.contains_key(&key) {
        let img = match item.icon_index {
            Some(idx) => icons
                .image_for_index(idx, size)
                .or_else(|_| icons.image_for_path(&item.path, size))
                .unwrap_or_else(|_| IconImage::solid(size, [128, 128, 128, 255])),
            None => IconImage::solid(size, [128, 128, 128, 255]),
        };
        return texture_for(ui, key, &img, textures);
    }
    textures.get(&key).map(|t| t.id())
}

/// 条目的纹理缓存键（与 IconCache::key_for_index 同源）。
pub fn icon_key(item: &FileItem) -> u32 {
    item.icon_index.unwrap_or(-1) as u32
}

// ---------- 各子部件 ----------

/// 地址栏（只读显示；单击进入编辑，Enter 提交跳转，失焦取消）。
pub(crate) fn address_bar(
    ui: &mut Ui,
    id: PaneId,
    current: &Path,
    draft: &mut Option<String>,
    events: &mut Vec<PaneEvent>,
) {
    ui.horizontal(|ui| {
        let resp = match draft {
            Some(text) => {
                let r = ui.add(TextEdit::singleline(text).desired_width(f32::INFINITY));
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let p = PathBuf::from(text.trim());
                    *draft = None;
                    events.push(PaneEvent::Goto(id, p));
                } else if r.lost_focus() {
                    *draft = None; // Esc / 点击别处：放弃编辑
                }
                r
            }
            None => {
                let shown = current.display().to_string();
                let mut ro = shown.clone();
                let r = ui.add_sized(
                    [ui.available_width() - 30.0, 18.0],
                    TextEdit::singleline(&mut ro),
                );
                if r.clicked() {
                    *draft = Some(shown);
                }
                r
            }
        };
        let _ = resp;
        if ui.button("📁").clicked() && draft.is_none() {
            *draft = Some(current.display().to_string());
        }
    });
}

/// 标签页条：开关语义交给 PaneController::tab_click 结算（tab_* 配置）。
pub(crate) fn tab_bar(
    ui: &mut Ui,
    c: &mut PaneController<'_>,
    cfg: &crate::core::TabConfig,
    id: PaneId,
    events: &mut Vec<PaneEvent>,
) {
    let tabs: Vec<(usize, String, PathBuf)> = c
        .state
        .tabs
        .iter()
        .enumerate()
        .map(|(i, t)| (i, tab_title(&t.path), t.path.clone()))
        .collect();
    let active = c.state.tabs_pos;
    ui.horizontal_wrapped(|ui| {
        for (idx, title, path) in tabs {
            let resp = ui
                .selectable_label(idx == active, RichText::new(title))
                .on_hover_text(path.display().to_string());
            let click = if resp.double_clicked() {
                Some(TabClick::LeftDouble)
            } else if resp.middle_clicked() {
                Some(TabClick::Middle)
            } else if resp.clicked() {
                Some(TabClick::Left)
            } else {
                None
            };
            if let Some(click) = click {
                let action = c.tab_click(cfg, Some(idx), click);
                if action != TabAction::Ignored {
                    events.push(PaneEvent::TabSettled(id));
                }
            }
        }
        // 空白处（tab_new_tab_l_clk / dblclk 语义）。
        let blank = ui.allocate_response(
            egui::vec2(40.0, 18.0),
            Sense::click(),
        );
        let click = if blank.double_clicked() {
            Some(TabClick::LeftDouble)
        } else if blank.clicked() {
            Some(TabClick::Left)
        } else {
            None
        };
        if let Some(click) = click {
            let action = c.tab_click(cfg, None, click);
            if action != TabAction::Ignored {
                events.push(PaneEvent::TabSettled(id));
            }
        }
        if ui.button("+").clicked() {
            c.new_tab(None);
            events.push(PaneEvent::TabSettled(id));
        }
    });
}

fn tab_title(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

/// 目录树（懒加载：未加载节点展开时发 TreeExpand 事件）。
pub(crate) fn tree_view(
    ui: &mut Ui,
    c: &mut PaneController<'_>,
    id: PaneId,
    events: &mut Vec<PaneEvent>,
) {
    if !c.view_tree().config().mode {
        return;
    }
    ScrollArea::vertical().show(ui, |ui| {
        // 先克隆根避免借用冲突：节点数据在渲染中不变（展开事件下一帧生效）。
        let snapshot = c.view_tree().root().clone();
        render_node(ui, c, &snapshot, id, events);
    });
}

fn render_node(
    ui: &mut Ui,
    c: &mut PaneController<'_>,
    node: &TreeNode,
    id: PaneId,
    events: &mut Vec<PaneEvent>,
) {
    let expanded = c.view_tree().is_expanded(&node.path);
    let loaded = node.loaded;
    ui.indent(node.path.as_os_str(), |ui| {
        ui.horizontal(|ui| {
            let arrow = if expanded { "▼" } else { "▶" };
            let arrow_resp =
                ui.add_sized([14.0, 14.0], egui::Label::new(arrow).sense(Sense::click()));
            if arrow_resp.clicked() {
                if !loaded {
                    events.push(PaneEvent::TreeExpand(id, node.path.clone()));
                }
                let _ = c.view_tree_mut().toggle(&node.path);
            }
            let sel = c.view_tree().selected() == Some(&node.path);
            if ui.selectable_label(sel, &node.name).clicked() {
                // TreeConfig.auto_sel / auto_expand 由 tree_select_dir 语义统一处理。
                events.push(PaneEvent::TreeSelect(id, node.path.clone()));
            }
        });
        if expanded {
            for child in &node.children {
                render_node(ui, c, child, id, events);
            }
        }
    });
}

/// 详情视图表头（点击翻转排序）。
pub(crate) fn details_header(ui: &mut Ui, id: PaneId, events: &mut Vec<PaneEvent>) {
    ui.horizontal(|ui| {
        for (label, key) in [
            ("名称", SortKey::Name),
            ("大小", SortKey::Size),
            ("类型", SortKey::Type),
            ("修改时间", SortKey::Modified),
        ] {
            if ui.button(label).clicked() {
                events.push(PaneEvent::Sort(id, key));
            }
        }
    });
}

/// 详情视图行。
pub(crate) fn details_row(
    ui: &mut Ui,
    c: &mut PaneController<'_>,
    vis_idx: usize,
    item: &FileItem,
    paint: RowPaint,
    id: PaneId,
    events: &mut Vec<PaneEvent>,
) {
    let selected = c.view.is_selected(&item.name);
    let fg = paint.fg.unwrap_or(ui.visuals().text_color());
    let mut name_t = RichText::new(&item.name).color(fg);
    if item.is_dir {
        name_t = name_t.strong();
    }
    let r1 = ui.selectable_label(selected, name_t);
    ui.label(RichText::new(crate::ui::app::human_size(item.size)).color(fg));
    ui.label(
        RichText::new(if item.is_dir {
            "文件夹".to_string()
        } else {
            item.ext_lower().to_uppercase()
        })
        .color(fg),
    );
    ui.label(RichText::new(format_filetime(item.modified)).color(fg));
    handle_row_interaction(ui, c, r1, vis_idx, item, id, events);
}

/// 列表/图标视图行。
pub(crate) fn simple_row(
    ui: &mut Ui,
    c: &mut PaneController<'_>,
    vis_idx: usize,
    item: &FileItem,
    paint: RowPaint,
    id: PaneId,
    events: &mut Vec<PaneEvent>,
) {
    let selected = c.view.is_selected(&item.name);
    let mut label = RichText::new(&item.name);
    if let Some(fg) = paint.fg {
        label = label.color(fg);
    }
    let resp = ui
        .selectable_label(selected, label)
        .on_hover_text(item.path.display().to_string());
    handle_row_interaction(ui, c, resp, vis_idx, item, id, events);
}

/// 行交互：单击/Ctrl 选中、双击打开、右键菜单。
fn handle_row_interaction(
    ui: &Ui,
    c: &mut PaneController<'_>,
    resp: egui::Response,
    vis_idx: usize,
    item: &FileItem,
    id: PaneId,
    events: &mut Vec<PaneEvent>,
) {
    let ctrl = ui.input(|i| i.modifiers.ctrl);
    if resp.clicked() {
        if ctrl {
            c.view.toggle_selected(&item.name);
        } else {
            c.view.select_one(&item.name);
        }
        c.view.focus_row = Some(vis_idx);
    } else if resp.secondary_clicked() {
        if !c.view.is_selected(&item.name) {
            c.view.select_one(&item.name);
        }
        events.push(PaneEvent::ContextMenu(
            id,
            resp.interact_pointer_pos().unwrap_or_default(),
        ));
    }
    if resp.double_clicked() {
        events.push(PaneEvent::Activate(id, item.path.clone(), item.is_dir));
    }
}

/// FILETIME(100ns) → `yyyy-MM-dd HH:mm:ss`（本地时区差忽略，与导出格式一致）。
pub fn format_filetime(ft: u64) -> String {
    if ft == 0 {
        return String::new();
    }
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    let total = ft.saturating_sub(EPOCH_DIFF);
    let secs = total / 10_000_000;
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// Howard Hinnant 的 days→civil 算法（公有领域）。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 预览面板。
pub(crate) fn preview_pane(ui: &mut Ui, data: Option<&PreviewData>) {
    ui.heading("预览");
    match data {
        Some(PreviewData::Image(img)) => {
            let w = img.width.max(1) as usize;
            let h = img.height.max(1) as usize;
            let tex = ui.ctx().load_texture(
                "qdir-preview",
                egui::ColorImage::from_rgba_unmultiplied([w, h], &img.rgba),
                Default::default(),
            );
            ui.image((
                tex.id(),
                egui::vec2(img.width as f32, img.height as f32),
            ));
        }
        Some(PreviewData::Text(t)) => {
            ScrollArea::vertical().show(ui, |ui| {
                for line in &t.lines {
                    ui.monospace(line);
                }
                if t.truncated {
                    ui.weak("…（已截断）");
                }
            });
        }
        Some(PreviewData::Binary(b)) => {
            ui.label(format!("二进制文件 · {} 字节", b.total_bytes));
            let mut hex = String::new();
            for (i, byte) in b.head.iter().take(64).enumerate() {
                if i > 0 && i % 16 == 0 {
                    hex.push('\n');
                }
                hex.push_str(&format!("{byte:02X} "));
            }
            ui.monospace(hex);
        }
        None => {
            ui.weak("（无可预览内容）");
        }
    }
}

/// 键盘快捷键统一入口（app 在 run_pending 前调用）。
pub fn global_shortcuts(ui: &Ui, id: PaneId, events: &mut Vec<PaneEvent>) {
    let (pressed, mods) = ui.input(|i| {
        let keys = [
            egui::Key::ArrowLeft,
            egui::Key::ArrowRight,
            egui::Key::Backspace,
        ];
        let hit = keys.iter().find(|k| i.key_pressed(**k)).copied();
        (hit, i.modifiers)
    });
    if let Some(key) = pressed {
        match key {
            egui::Key::ArrowLeft if mods.alt => events.push(PaneEvent::Nav(id, NavKind::Back)),
            egui::Key::ArrowRight if mods.alt => events.push(PaneEvent::Nav(id, NavKind::Forward)),
            egui::Key::Backspace if !mods.ctrl && !mods.alt => {
                events.push(PaneEvent::Nav(id, NavKind::Up))
            }
            _ => {}
        }
    }
    // F2/F5/F6/Delete/Ctrl+T/W/L 需要 Pending 级动作，由 app 侧直接读输入处理。
}

/// 导航动作（Backspace / Alt+方向键）。
#[derive(Debug, Clone, Copy)]
pub enum NavKind {
    Back,
    Forward,
    Up,
}
