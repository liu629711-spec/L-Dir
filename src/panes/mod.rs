//! 窗格、标签页、导航与目录树/预览的状态层。
//!
//! 纯状态层（D2：不画界面，GUI 只读本模块产出的 core 类型）。
//! 分三个子模块：
//! - [`tree`]：惰性加载的目录树模型；
//! - [`preview`]：按 `[Preview] Filter` 的预览数据；
//! - 本文件：窗格/标签页包装、导航历史、`tree_*` / `tab_*` / `refresh_*` 等
//!   ini 开关的行为语义，以及与 config 模块的 .qdr 读写对接。
//!
//! 关键实现取舍（架构契约 A1/A2 边界）：
//! - [`core::PaneState`] / [`core::TabState`] 是**序列化契约类型**，字段与签名
//!   由架构师固定，本模块不得改其形状。因此「选择集 / 过滤串 / 列宽」等
//!   **不序列化**的运行时状态不能塞进 core 类型，只能由本模块的
//!   [`PaneViewState`] 在 core 类型之外持有。.qdr 只保存 core 类型覆盖到的那部分。
//! - core 已提供 `TabState::back/forward/navigate`（历史栈）与
//!   `PaneState::add_tab/close_tab`，本模块在其上补齐原版行为：
//!   `select_last_if_navigate_up`（上级时选中来源目录）、
//!   `refresh_keep_sel` / `refresh_keep_scroll`、`tab_*` 开关的点击语义。
//!
//! 依赖方向 D1：panes 只依赖 core；不 use 任何其它业务模块。

pub mod preview;
pub mod tree;

use std::path::{Path, PathBuf};

use crate::core::{
    ColumnConfig, Config, FileOpsConfig, FileItem, PaneId, PaneState, QError, QResult, SessionState,
    SortDir, SortKey, TabConfig, TabState, TreeConfig, ViewMode,
};
use crate::panes::tree::{normalize_path, TreeModel};
use crate::panes::preview as preview_mod;

pub use crate::panes::tree::TreeNode;

/// 单窗格的运行时（不序列化）状态：选择集、滚动位置、目录树模型。
///
/// 为什么不并进 [`core::PaneState`]：core 是序列化契约（A1），.qdr 里没有
/// 选择集/滚动位置对应的键，塞进去会让 `PartialEq`/往返测试把它们也算进去。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneViewState {
    /// 选中的条目名集合（显示名，大小写保留）。
    pub selection: Vec<String>,
    /// 列表滚动位置（行索引）。`refresh_keep_scroll` 关时刷新重置为 0。
    pub scroll_row: usize,
    /// 焦点行（键盘导航位置），无焦点时为 `None`。
    pub focus_row: Option<usize>,
    /// 目录树模型（懒加载状态）。
    pub tree: TreeModel,
    /// 该窗格预览面板是否展开（.qdr 的 `Vorschau` 是全局的，这里是窗格级覆盖）。
    pub preview_open: bool,
}

impl PaneViewState {
    /// 按当前路径与目录树配置建运行时状态。
    pub fn new(path: &Path, tree_cfg: TreeConfig) -> Self {
        PaneViewState {
            selection: Vec::new(),
            scroll_row: 0,
            focus_row: None,
            tree: TreeModel::new(normalize_path(path), tree_cfg),
            preview_open: false,
        }
    }

    /// 清空选择与焦点（导航到别处时调用）。
    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.focus_row = None;
    }

    /// 条目是否被选中。
    pub fn is_selected(&self, name: &str) -> bool {
        self.selection.iter().any(|n| n == name)
    }

    /// 单选（替换选择集）。
    pub fn select_one(&mut self, name: &str) {
        self.selection.clear();
        self.selection.push(name.to_string());
    }

    /// 切换某项选中态；返回切换后是否选中。
    pub fn toggle_selected(&mut self, name: &str) -> bool {
        if let Some(pos) = self.selection.iter().position(|n| n == name) {
            self.selection.remove(pos);
            false
        } else {
            self.selection.push(name.to_string());
            true
        }
    }

    /// 全选/全不选给定名字列表。
    pub fn set_selection(&mut self, names: &[String]) {
        self.selection = names.to_vec();
    }
}

/// 四个窗格的运行时状态集合，键为 [`PaneId`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneViews {
    views: [PaneViewState; 4],
}

impl PaneViews {
    /// 按会话状态为四个窗格建运行时状态。
    pub fn from_session(session: &SessionState, tree_cfg: TreeConfig) -> Self {
        let make = |id: PaneId| {
            let pane = session.pane(id);
            PaneViewState::new(&pane.path(), tree_cfg.clone())
        };
        PaneViews {
            views: [
                make(PaneId::P1),
                make(PaneId::P2),
                make(PaneId::P3),
                make(PaneId::P4),
            ],
        }
    }

    /// 取窗格运行时状态。
    pub fn get(&self, id: PaneId) -> &PaneViewState {
        &self.views[(id.number() - 1) as usize]
    }

    /// 取窗格运行时状态（可变）。
    pub fn get_mut(&mut self, id: PaneId) -> &mut PaneViewState {
        &mut self.views[(id.number() - 1) as usize]
    }
}

/// 鼠标/键盘在标签页条上的动作。GUI 层把原始事件翻译成这个枚举再交给
/// [`PaneController`]，使点击语义可单测（不依赖 egui）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabClick {
    /// 左键单击标签页本体。
    Left,
    /// 左键双击标签页本体。
    LeftDouble,
    /// 中键单击标签页本体。
    Middle,
}

/// 标签页动作的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabAction {
    /// 无需处理（开关未开）。
    Ignored,
    /// 切换到已有标签页。
    Switched { index: usize },
    /// 新建标签页并切换过去。
    Opened { index: usize },
    /// 关闭标签页并切换到 `next`。
    Closed { next: usize },
}

/// 单窗格控制器：把 core 的 [`PaneState`] 与运行时 [`PaneViewState`] 绑在一起，
/// 并施加 [`TabConfig`] / [`FileOpsConfig`] / [`TreeConfig`] 的行为语义。
///
/// 设计要点：控制器**不持有** config 的副本，只在方法参数里接收需要的配置节，
/// 这样用户在设置里改键后立即生效（无需重建控制器）。
#[derive(Debug)]
pub struct PaneController<'a> {
    /// 序列化状态（core 契约类型）。
    pub state: &'a mut PaneState,
    /// 运行时状态（选择集/滚动/目录树）。
    pub view: &'a mut PaneViewState,
}

impl<'a> PaneController<'a> {
    /// 绑定一个窗格。
    pub fn new(state: &'a mut PaneState, view: &'a mut PaneViewState) -> Self {
        PaneController { state, view }
    }

    // ---------- 基础访问 ----------

    /// 当前目录。
    pub fn path(&self) -> PathBuf {
        self.state.path()
    }

    /// 当前标签页。
    pub fn tab(&self) -> Option<&TabState> {
        self.state.current_tab()
    }

    /// 当前标签页（可变）。
    pub fn tab_mut(&mut self) -> Option<&mut TabState> {
        self.state.current_tab_mut()
    }

    /// 生效视图模式（标签页覆盖 > 默认）。
    pub fn view_mode(&self) -> ViewMode {
        self.state.view_mode()
    }

    // ---------- 视图模式 / 排序 / 列宽 ----------

    /// 设置视图模式（写标签页覆盖位，与 FolderN 的 `ViewModeN` 子字段对应，E3）。
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        if let Some(t) = self.state.current_tab_mut() {
            t.view_mode = Some(mode);
        }
    }

    /// 清空视图模式覆盖（回落到默认 LargeIcon）。
    pub fn clear_view_mode_override(&mut self) {
        if let Some(t) = self.state.current_tab_mut() {
            t.view_mode = None;
        }
    }

    /// 设置排序列与方向（写当前标签页）。
    pub fn set_sort(&mut self, key: SortKey, dir: SortDir) {
        if let Some(t) = self.state.current_tab_mut() {
            t.sort_key = key;
            t.sort_dir = dir;
        }
    }

    /// 切换排序方向；同列再点则反向（资源管理器惯例）。
    pub fn toggle_sort(&mut self, key: SortKey) {
        if let Some(t) = self.state.current_tab_mut() {
            if t.sort_key == key {
                t.sort_dir = match t.sort_dir {
                    SortDir::Asc => SortDir::Desc,
                    SortDir::Desc => SortDir::Asc,
                };
            } else {
                t.sort_key = key;
                t.sort_dir = SortDir::Asc;
            }
        }
    }

    /// 设置列宽（详情视图拖动列边缘）。
    pub fn set_column_width(&mut self, key: &str, width: u32) -> bool {
        let cols: &mut ColumnConfig = &mut self.state.columns;
        match cols.columns.iter_mut().find(|c| c.key == key) {
            Some(c) => {
                c.width = width;
                true
            }
            None => false, // 未知列键：不动数据（前向兼容）
        }
    }

    /// 列宽（不存在返回 `None`）。
    pub fn column_width(&self, key: &str) -> Option<u32> {
        self.state.columns.columns.iter().find(|c| c.key == key).map(|c| c.width)
    }

    /// 设置过滤串（`by_filter` / 工具栏过滤框）。
    /// 空串表示不过滤；返回是否真的变化。
    pub fn set_filter(&mut self, filter: &str) -> bool {
        match self.state.current_tab_mut() {
            Some(t) if t.filter != filter => {
                t.filter = filter.to_string();
                true
            }
            _ => false,
        }
    }

    /// 当前过滤串。
    pub fn filter(&self) -> String {
        self.state.current_tab().map(|t| t.filter.clone()).unwrap_or_default()
    }

    /// 过滤串是否生效（非空且非纯分隔符）。
    pub fn has_filter(&self) -> bool {
        !self.filter().trim().is_empty()
    }

    // ---------- 导航 ----------

    /// 进入子目录（双击目录 / 回车）。
    ///
    /// 目录才允许进入；文件返回 [`QError::InvalidPath`]（调用方应改为打开文件）。
    /// 成功后：历史入栈、选择集与滚动按 `refresh_*` 语义重置。
    pub fn enter_dir(&mut self, item: &FileItem, file_ops: &FileOpsConfig) -> QResult<()> {
        if !item.is_dir {
            return Err(QError::InvalidPath(format!(
                "{} 不是目录，不能进入",
                item.path.display()
            )));
        }
        let target = normalize_path(&item.path);
        if let Some(t) = self.state.current_tab_mut() {
            t.navigate(target);
        }
        self.after_navigate(file_ops, None);
        Ok(())
    }

    /// 导航到任意目录（收藏夹/地址栏/树点击）。
    pub fn navigate_to(&mut self, path: &Path, file_ops: &FileOpsConfig) -> QResult<()> {
        let target = normalize_path(path);
        if target.as_os_str().is_empty() {
            return Err(QError::InvalidPath("导航目标为空路径".to_string()));
        }
        if let Some(t) = self.state.current_tab_mut() {
            t.navigate(target);
        }
        self.after_navigate(file_ops, None);
        Ok(())
    }

    /// 后退一步。
    pub fn back(&mut self, file_ops: &FileOpsConfig) -> QResult<()> {
        let from = self.path();
        if let Some(t) = self.state.current_tab_mut() {
            t.back()?;
        }
        self.after_navigate(file_ops, Some(from));
        Ok(())
    }

    /// 前进一步。
    pub fn forward(&mut self, file_ops: &FileOpsConfig) -> QResult<()> {
        let from = self.path();
        if let Some(t) = self.state.current_tab_mut() {
            t.forward()?;
        }
        self.after_navigate(file_ops, Some(from));
        Ok(())
    }

    /// 上一级目录。
    ///
    /// `select_last_if_navigate_up` 生效时：返回**应该在上级里选中的名字**
    /// （即来源目录名），由调用方在列表渲染出来后设置选中项。
    /// 这样本函数保持纯状态操作，不依赖列表内容。
    pub fn up(
        &mut self,
        file_ops: &FileOpsConfig,
    ) -> QResult<Option<String>> {
        let current = self.path();
        let parent = match crate::panes::tree::parent_dir(&current) {
            Some(p) => p,
            None => {
                return Err(QError::Session(format!(
                    "{} 已是根目录，没有上一级",
                    current.display()
                )))
            }
        };
        let source_name = current
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(t) = self.state.current_tab_mut() {
            t.navigate(parent);
        }
        self.after_navigate(file_ops, None);
        if file_ops.select_last_if_navigate_up && !source_name.is_empty() {
            Ok(Some(source_name))
        } else {
            Ok(None)
        }
    }

    /// 导航完成后的公共收尾：清/保选择与滚动、目录树同步。
    ///
    /// `from` 为 `Some` 时表示是「后退/前进」——原版在这两种操作里同样重置
    /// 选择与滚动（历史导航等价于打开新目录），因此与 `enter_dir` 走同一路径。
    fn after_navigate(&mut self, file_ops: &FileOpsConfig, _from: Option<PathBuf>) {
        if !file_ops.refresh_keep_sel {
            self.view.clear_selection();
        } else {
            // 保持选择语义：保留名字集合（目录已变，名字可能不存在，
            // 由列表层按新内容过滤掉失效项），但焦点行失效。
            self.view.focus_row = None;
        }
        if !file_ops.refresh_keep_scroll {
            self.view.scroll_row = 0;
        }
        // 目录树跟随：树根重挂到新目录（auto_expand/auto_sel 在树模型里生效）。
        let new_root = normalize_path(&self.path());
        self.view.tree.sync_root(new_root);
    }

    /// 刷新当前目录（F5）。
    ///
    /// `refresh_keep_sel` 开：保留仍存在的选择项，丢弃失效项；
    /// `refresh_keep_scroll` 开：保留滚动位置。
    /// `new_items` 是重新枚举出来的列表，用于筛选失效选择项。
    pub fn refresh(&mut self, file_ops: &FileOpsConfig, new_items: &[FileItem]) {
        if file_ops.refresh_keep_sel {
            let names: Vec<String> = new_items.iter().map(|i| i.name.clone()).collect();
            self.view.selection.retain(|s| names.iter().any(|n| n == s));
            self.view.focus_row = None;
        } else {
            self.view.clear_selection();
        }
        if !file_ops.refresh_keep_scroll {
            self.view.scroll_row = 0;
        }
    }

    // ---------- 标签页 ----------

    /// 标签页条上的点击，按 [`TabConfig`] 开关决定动作。
    ///
    /// 原版语义（键名即语义）：
    /// - `tab_new_tab_l_clk`：左键单击**标签页条空白处**新建（不是点已有标签页）；
    /// - `tab_new_tab_l_dblclk`：左键双击标签页条空白处新建；
    /// - `tab_close_by_mbt_clk`：中键单击标签页关闭；
    /// - `tab_close_by_lbt_dblclk`：左键双击标签页关闭（与「切换到该标签页」互斥，
    ///   双击时先切换再关闭没有意义，故关优先于切）。
    ///
    /// `index = None` 表示点的是标签页条空白处（只能触发新建）。
    pub fn tab_click(
        &mut self,
        tab_cfg: &TabConfig,
        index: Option<usize>,
        click: TabClick,
    ) -> TabAction {
        match index {
            None => {
                // 空白处：只有「新建」类开关有意义。
                let open = match click {
                    TabClick::Left => tab_cfg.new_tab_l_clk,
                    TabClick::LeftDouble => tab_cfg.new_tab_l_dblclk,
                    TabClick::Middle => false,
                };
                if open {
                    let path = self.path();
                    self.state.add_tab(path);
                    TabAction::Opened {
                        index: self.state.tabs_pos,
                    }
                } else {
                    TabAction::Ignored
                }
            }
            Some(idx) => {
                if idx >= self.state.tabs.len() {
                    return TabAction::Ignored; // 越界点击忽略，不 panic
                }
                match click {
                    TabClick::Middle => {
                        if tab_cfg.close_by_mbt_clk {
                            self.close_tab(idx).map_or(TabAction::Ignored, |next| {
                                TabAction::Closed { next }
                            })
                        } else {
                            // 中键不支持「切换」：原版中键只绑定关闭。
                            TabAction::Ignored
                        }
                    }
                    TabClick::LeftDouble => {
                        if tab_cfg.close_by_lbt_dblclk {
                            self.close_tab(idx).map_or(TabAction::Ignored, |next| {
                                TabAction::Closed { next }
                            })
                        } else {
                            self.switch_tab(idx)
                        }
                    }
                    TabClick::Left => self.switch_tab(idx),
                }
            }
        }
    }

    /// 切换到标签页；越界返回 [`QError::Session`]。
    pub fn switch_tab(&mut self, idx: usize) -> TabAction {
        if idx >= self.state.tabs.len() {
            return TabAction::Ignored;
        }
        self.state.tabs_pos = idx;
        // 切换标签页等价于导航到该标签页的目录：重置滚动，选择按 refresh 语义。
        self.view.scroll_row = 0;
        self.view.focus_row = None;
        TabAction::Switched { index: idx }
    }

    /// 新建标签页（Ctrl+T / 标签页条 '+' 按钮），可指定初始目录。
    pub fn new_tab(&mut self, path: Option<PathBuf>) -> TabAction {
        let p = path.unwrap_or_else(|| self.path());
        self.state.add_tab(normalize_path(&p));
        self.view.scroll_row = 0;
        self.view.clear_selection();
        TabAction::Opened {
            index: self.state.tabs_pos,
        }
    }

    /// 关闭标签页；最后一个不可关闭（core 的约定）。
    /// 返回关闭后应切换到的下标。
    pub fn close_tab(&mut self, idx: usize) -> QResult<usize> {
        self.state.close_tab(idx)?;
        self.view.scroll_row = 0;
        self.view.clear_selection();
        Ok(self.state.tabs_pos)
    }

    // ---------- 目录树 ----------

    /// 目录树模型只读引用（GUI 层渲染树用）。
    pub fn view_tree(&self) -> &TreeModel {
        &self.view.tree
    }

    /// 目录树模型可变引用（懒加载灌数据用）。
    pub fn view_tree_mut(&mut self) -> &mut TreeModel {
        &mut self.view.tree
    }

    /// 目录树是否显示。`tree_mode`（全局）关时一律不显示。
    pub fn tree_visible(&self) -> bool {
        self.state.tree_visible && self.view.tree.config().mode.is_active()
    }

    /// 用 ini 的 `tree_*` 配置刷新树行为。
    pub fn apply_tree_config(&mut self, cfg: TreeConfig) {
        self.view.tree.set_config(cfg);
    }

    /// 树节点点击：展开/折叠。返回是否需要调用方枚举子目录。
    pub fn tree_toggle(&mut self, path: &Path) -> QResult<bool> {
        self.view.tree.toggle(path)
    }

    /// 树节点点击（选中目录）：导航过去（与列表双击目录等价）。
    pub fn tree_select_dir(
        &mut self,
        path: &Path,
        file_ops: &FileOpsConfig,
    ) -> QResult<()> {
        self.navigate_to(path, file_ops)
    }

    /// 列表导航后同步目录树（`tree_auto_expand_from_list` / `tree_auto_sel`）。
    /// 返回需要加载的路径序列（从上层到目标）。
    pub fn tree_sync_from_list(&mut self) -> Vec<PathBuf> {
        let cur = normalize_path(&self.path());
        self.view.tree.sync_from_list(&cur)
    }

    /// 树分割位置（千分比）。`tree_indent_width` 只影响渲染，不碰分割。
    pub fn tree_split(&self) -> u32 {
        self.state.tree_split
    }

    /// 设置树分割位置（千分比，0..=1000）。
    pub fn set_tree_split(&mut self, permille: u32) {
        self.state.tree_split = permille.min(1000);
    }

    // ---------- 预览 ----------

    /// 该窗格预览面板是否展开。
    pub fn preview_open(&self) -> bool {
        self.view.preview_open
    }

    /// 设置预览面板展开态。
    pub fn set_preview_open(&mut self, open: bool) {
        self.view.preview_open = open;
    }

    /// 当前选中项的预览数据（按 `[Preview] Filter`）。
    /// 未选中/不可预览返回 `Ok(None)`（不是错误：空目录本来就没东西可预览）。
    pub fn preview_for_selection(
        &self,
        items: &[FileItem],
        cfg: &Config,
    ) -> QResult<Option<preview_mod::PreviewData>> {
        let Some(name) = self.view.selection.first() else {
            return Ok(None);
        };
        let Some(item) = items.iter().find(|i| &i.name == name) else {
            return Ok(None); // 选择集里是失效项
        };
        if item.is_dir {
            return Ok(None); // 目录不预览
        }
        if !preview_mod::previewable(&item.path, &cfg.preview_filter) {
            return Ok(None);
        }
        Ok(Some(preview_mod::build_preview(&item.path, &cfg.preview_filter, None)?))
    }
}

/// 四个窗格控制器的入口集合。
///
/// app.rs 持有它（A4：app.rs 是唯一持有子系统实例的地方），
/// 把 GUI 事件翻译成控制器方法调用，再读回 core 状态渲染。
#[derive(Debug)]
pub struct PaneSet<'a> {
    states: [&'a mut PaneState; 4],
    views: [&'a mut PaneViewState; 4],
}

impl<'a> PaneSet<'a> {
    /// 从会话状态与运行时状态集合构造。
    ///
    /// 借用拆分：`states` 与 `views` 是两块独立数组，互不重叠，故可同时持有
    /// 四个窗格的可变引用（Rust 借用的安全拆分点）。
    pub fn new(session: &'a mut SessionState, views: &'a mut PaneViews) -> Self {
        let PaneViews { views: v } = views;
        // 数组长度恒为 4（core 契约）。`each_mut()` 产出恰好 4 个不重叠的可变引用，
        // 一次性收成数组是 Rust 里同时持有同数组多个 &mut 的安全写法
        // （逐项 `&mut p[i]` 会被借用检查器拒绝：同一表达式里两次 IndexMut）。
        let states = <[&mut PaneState; 4]>::try_from(session.panes.each_mut())
            .expect("SessionState::panes 恒为 4 个窗格（core 契约）");
        let views = <[&mut PaneViewState; 4]>::try_from(v.each_mut())
            .expect("PaneViews 恒为 4 个窗格视图");
        PaneSet { states, views }
    }

    /// 取某窗格的控制器。
    pub fn pane(&mut self, id: PaneId) -> PaneController<'_> {
        let i = (id.number() - 1) as usize;
        PaneController::new(self.states[i], self.views[i])
    }

    /// 只读取某窗格的序列化状态。
    pub fn state(&self, id: PaneId) -> &PaneState {
        self.states[(id.number() - 1) as usize]
    }

    /// 只读取某窗格的运行时状态。
    pub fn view(&self, id: PaneId) -> &PaneViewState {
        self.views[(id.number() - 1) as usize]
    }

    /// 设置焦点窗格（.qdr 的 `Focus`）。
    pub fn set_focus(&self, session: &mut SessionState, id: PaneId) {
        session.focus = id;
    }
}

/// 把 [`Config`] 的目录树配置灌进运行时视图（设置对话框改键后调用）。
pub fn apply_tree_config(views: &mut PaneViews, cfg: &TreeConfig) {
    for id in PaneId::ALL {
        views.get_mut(id).tree.set_config(cfg.clone());
    }
}

/// 便捷入口：当前焦点窗格的目录。
pub fn focused_path(session: &SessionState) -> PathBuf {
    session.focused().path()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, is_dir: bool) -> FileItem {
        FileItem {
            path: PathBuf::from(format!(r"C:\root\{name}")),
            name: name.to_string(),
            is_dir,
            is_hidden: false,
            size: 10,
            modified: 100,
            icon_index: None,
        }
    }

    fn ops() -> FileOpsConfig {
        FileOpsConfig::default()
    }

    fn pane(id: PaneId, path: &str) -> PaneState {
        PaneState::new(id, PathBuf::from(path))
    }

    fn view(path: &str) -> PaneViewState {
        PaneViewState::new(Path::new(path), TreeConfig::default())
    }

    fn ctrl<'a>(p: &'a mut PaneState, v: &'a mut PaneViewState) -> PaneController<'a> {
        PaneController::new(p, v)
    }

    // ---------- 导航历史 ----------

    #[test]
    fn enter_dir_pushes_history_and_resets_selection() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        v.select_one("keepme");
        v.scroll_row = 42;
        let mut c = ctrl(&mut p, &mut v);
        let child = item("child", true);
        c.enter_dir(&child, &ops()).expect("进入目录");
        assert_eq!(c.path(), PathBuf::from(r"C:\root\child"));
        assert_eq!(c.tab().expect("标签页").history.len(), 2);
        // refresh_keep_sel 默认开：选择保留；滚动也默认保留。
        assert_eq!(v.selection, vec!["keepme".to_string()]);
        assert_eq!(v.scroll_row, 42);
    }

    #[test]
    fn enter_dir_rejects_files() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        let f = item("a.txt", false);
        let err = c.enter_dir(&f, &ops()).unwrap_err();
        assert!(matches!(err, QError::InvalidPath(_)), "文件不能进入，实际 {err:?}");
    }

    #[test]
    fn back_and_forward_walk_history() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        {
            let mut c = ctrl(&mut p, &mut v);
            // item() 的路径前缀恒为 C:\root\，因此这里用显式路径条目，
            // 让「当前目录 + 子目录名」的拼接符合真实枚举结果。
            let child_of = |name: &str, base: &str| FileItem {
                path: PathBuf::from(format!(r"{base}\{name}")),
                name: name.to_string(),
                is_dir: true,
                is_hidden: false,
                size: 0,
                modified: 0,
                icon_index: None,
            };
            c.enter_dir(&child_of("a", r"C:\root"), &ops()).expect("进 a");
            c.enter_dir(&child_of("b", r"C:\root\a"), &ops()).expect("进 b");
            assert_eq!(c.path(), PathBuf::from(r"C:\root\a\b"));
            c.back(&ops()).expect("后退");
            assert_eq!(c.path(), PathBuf::from(r"C:\root\a"));
            c.forward(&ops()).expect("前进");
            assert_eq!(c.path(), PathBuf::from(r"C:\root\a\b"));
            // 栈底再后退：core 报 Session。
            assert!(c.back(&ops()).is_ok());
            assert!(c.back(&ops()).is_ok(), "回到 root");
            assert!(c.back(&ops()).is_err(), "栈底必须报 Session");
        }
    }

    #[test]
    fn navigate_to_rejects_empty_path() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert!(c.navigate_to(Path::new(""), &ops()).is_err());
        assert_eq!(c.path(), PathBuf::from(r"C:\root"), "失败不得改状态");
    }

    #[test]
    fn navigate_to_normalizes_path() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        c.navigate_to(Path::new(r"C:\root\sub\..\other"), &ops())
            .expect("导航");
        assert_eq!(c.path(), PathBuf::from(r"C:\root\other"));
    }

    // ---------- 上级与选中来源目录 ----------

    #[test]
    fn up_selects_source_dir_when_enabled() {
        let mut ops = ops();
        ops.select_last_if_navigate_up = true;
        let mut p = pane(PaneId::P1, r"C:\root\child");
        let mut v = view(r"C:\root\child");
        let mut c = ctrl(&mut p, &mut v);
        let sel = c.up(&ops).expect("上一级");
        assert_eq!(c.path(), PathBuf::from(r"C:\root"));
        assert_eq!(sel, Some("child".to_string()), "上级时应选中来源目录");
    }

    #[test]
    fn up_returns_none_when_disabled() {
        let mut ops = ops();
        ops.select_last_if_navigate_up = false;
        let mut p = pane(PaneId::P1, r"C:\root\child");
        let mut v = view(r"C:\root\child");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.up(&ops).expect("上一级"), None);
    }

    #[test]
    fn up_at_root_is_session_error() {
        let mut p = pane(PaneId::P1, r"C:\");
        let mut v = view(r"C:\");
        let mut c = ctrl(&mut p, &mut v);
        let err = c.up(&ops()).unwrap_err();
        assert!(matches!(err, QError::Session(_)), "盘符根没有上一级，实际 {err:?}");
    }

    #[test]
    fn up_keeps_selection_when_refresh_keep_sel_on() {
        let mut ops = ops();
        ops.refresh_keep_sel = true;
        ops.select_last_if_navigate_up = true;
        let mut p = pane(PaneId::P1, r"C:\root\child");
        let mut v = view(r"C:\root\child");
        v.select_one("old");
        let mut c = ctrl(&mut p, &mut v);
        c.up(&ops).expect("上一级");
        // 选择保留（调用方拿到 Some("child") 后再决定改成选来源目录）。
        assert_eq!(v.selection, vec!["old".to_string()]);
    }

    // ---------- 刷新 ----------

    #[test]
    fn refresh_keep_sel_drops_vanished_entries() {
        let mut ops = ops();
        ops.refresh_keep_sel = true;
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        v.set_selection(&["gone".to_string(), "still".to_string()]);
        v.scroll_row = 7;
        let mut c = ctrl(&mut p, &mut v);
        let items = vec![item("still", false), item("new", false)];
        c.refresh(&ops, &items);
        assert_eq!(v.selection, vec!["still".to_string()], "失效项被丢弃");
        assert_eq!(v.scroll_row, 7, "refresh_keep_scroll 默认开");
    }

    #[test]
    fn refresh_clears_when_switches_off() {
        let mut ops = ops();
        ops.refresh_keep_sel = false;
        ops.refresh_keep_scroll = false;
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        v.set_selection(&["a".to_string()]);
        v.scroll_row = 7;
        let mut c = ctrl(&mut p, &mut v);
        c.refresh(&ops, &[item("a", false)]);
        assert!(v.selection.is_empty());
        assert_eq!(v.scroll_row, 0);
    }

    // ---------- 排序与视图模式 ----------

    #[test]
    fn toggle_sort_flips_direction_on_same_key() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.tab().expect("tab").sort_key, SortKey::Name);
        assert_eq!(c.tab().expect("tab").sort_dir, SortDir::Asc);
        c.toggle_sort(SortKey::Name);
        assert_eq!(c.tab().expect("tab").sort_dir, SortDir::Desc);
        c.toggle_sort(SortKey::Name);
        assert_eq!(c.tab().expect("tab").sort_dir, SortDir::Asc);
        // 换列重置为升序。
        c.toggle_sort(SortKey::Size);
        assert_eq!(c.tab().expect("tab").sort_key, SortKey::Size);
        assert_eq!(c.tab().expect("tab").sort_dir, SortDir::Asc);
    }

    #[test]
    fn set_view_mode_writes_tab_override() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        assert_eq!(ctrl(&mut p, &mut v).view_mode(), ViewMode::LargeIcon);
        let mut c = ctrl(&mut p, &mut v);
        c.set_view_mode(ViewMode::Detail);
        assert_eq!(c.view_mode(), ViewMode::Detail);
        c.clear_view_mode_override();
        assert_eq!(c.view_mode(), ViewMode::LargeIcon);
    }

    #[test]
    fn sort_items_applies_key_and_dir_with_dirs_first() {
        let mut items = vec![
            item("b.zip", false),
            item("zdir", true),
            item("a.zip", false),
            item("adir", true),
        ];
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        c.set_sort(SortKey::Name, SortDir::Asc);
        let (k, d) = {
            let t = c.tab().expect("tab");
            (t.sort_key, t.sort_dir)
        };
        crate::core::sort_items(&mut items, k, d);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["adir", "zdir", "a.zip", "b.zip"]);
        crate::core::sort_items(&mut items, SortKey::Name, SortDir::Desc);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["zdir", "adir", "b.zip", "a.zip"]);
    }

    // ---------- 列宽 ----------

    #[test]
    fn column_width_roundtrip_and_unknown_key() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.column_width("Name"), Some(200));
        assert!(c.set_column_width("Name", 333));
        assert_eq!(c.column_width("Name"), Some(333));
        assert!(!c.set_column_width("NoSuchColumn", 10), "未知列键不得改数据");
    }

    // ---------- 过滤串 ----------

    #[test]
    fn filter_set_and_has_filter() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert!(!c.has_filter());
        assert!(c.set_filter("*.txt"));
        assert!(c.has_filter());
        assert_eq!(c.filter(), "*.txt");
        assert!(!c.set_filter("*.txt"), "同值不算变化");
        assert!(c.set_filter(""));
        assert!(!c.has_filter());
    }

    // ---------- 标签页开关 ----------

    #[test]
    fn tab_left_click_switches() {
        let cfg = TabConfig::default();
        let mut p = pane(PaneId::P1, r"C:\root");
        p.add_tab(PathBuf::from(r"C:\other"));
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.tab_click(&cfg, Some(0), TabClick::Left), TabAction::Switched { index: 0 });
        assert_eq!(c.path(), PathBuf::from(r"C:\root"));
        assert_eq!(c.tab_click(&cfg, Some(9), TabClick::Left), TabAction::Ignored, "越界忽略");
    }

    #[test]
    fn tab_middle_click_closes_when_enabled() {
        let cfg = TabConfig::default(); // close_by_mbt_clk = true
        let mut p = pane(PaneId::P1, r"C:\root");
        p.add_tab(PathBuf::from(r"C:\a"));
        p.add_tab(PathBuf::from(r"C:\b"));
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.tab_click(&cfg, Some(1), TabClick::Middle), TabAction::Closed { next: 1 });
        assert_eq!(p.tabs.len(), 2);
        assert_eq!(p.path(), PathBuf::from(r"C:\b"));
    }

    #[test]
    fn tab_middle_click_ignored_when_disabled() {
        let mut cfg = TabConfig::default();
        cfg.close_by_mbt_clk = false;
        let mut p = pane(PaneId::P1, r"C:\root");
        p.add_tab(PathBuf::from(r"C:\a"));
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.tab_click(&cfg, Some(0), TabClick::Middle), TabAction::Ignored);
        assert_eq!(p.tabs.len(), 2, "关掉后中键不得关闭");
    }

    #[test]
    fn tab_double_click_closes_when_enabled_else_switches() {
        let mut cfg = TabConfig::default();
        cfg.close_by_lbt_dblclk = false;
        let mut p = pane(PaneId::P1, r"C:\root");
        p.add_tab(PathBuf::from(r"C:\a"));
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.tab_click(&cfg, Some(0), TabClick::LeftDouble), TabAction::Switched { index: 0 });

        cfg.close_by_lbt_dblclk = true;
        let mut p2 = pane(PaneId::P1, r"C:\root");
        p2.add_tab(PathBuf::from(r"C:\a"));
        let mut v2 = view(r"C:\root");
        let mut c2 = ctrl(&mut p2, &mut v2);
        assert_eq!(c2.tab_click(&cfg, Some(0), TabClick::LeftDouble), TabAction::Closed { next: 0 });
        assert_eq!(p2.tabs.len(), 1);
    }

    #[test]
    fn tab_bar_blank_click_opens_new_tab_when_enabled() {
        let mut cfg = TabConfig::default();
        cfg.new_tab_l_clk = true;
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        {
            let mut c = ctrl(&mut p, &mut v);
            assert_eq!(c.tab_click(&cfg, None, TabClick::Left), TabAction::Opened { index: 1 });
            // 新标签页继承当前目录（原版行为：新开标签页打开同一目录）。
            assert_eq!(c.path(), PathBuf::from(r"C:\root"));
            // 关掉后空白处单击无效。
            cfg.new_tab_l_clk = false;
            assert_eq!(c.tab_click(&cfg, None, TabClick::Left), TabAction::Ignored);
        }
        assert_eq!(p.tabs.len(), 2);
    }

    #[test]
    fn tab_bar_blank_double_click_opens_new_tab_when_enabled() {
        let mut cfg = TabConfig::default();
        cfg.new_tab_l_dblclk = true;
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        {
            let mut c = ctrl(&mut p, &mut v);
            assert_eq!(
                c.tab_click(&cfg, None, TabClick::LeftDouble),
                TabAction::Opened { index: 1 }
            );
        }
        assert_eq!(p.tabs.len(), 2);
    }

    #[test]
    fn last_tab_cannot_be_closed() {
        let cfg = TabConfig::default();
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        {
            let mut c = ctrl(&mut p, &mut v);
            assert_eq!(c.tab_click(&cfg, Some(0), TabClick::Middle), TabAction::Ignored);
            assert!(c.close_tab(0).is_err());
        }
        assert_eq!(p.tabs.len(), 1, "最后一个标签页不可关闭");
    }

    #[test]
    fn new_tab_with_explicit_path() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert_eq!(c.new_tab(Some(PathBuf::from(r"D:\data"))), TabAction::Opened { index: 1 });
        assert_eq!(p.path(), PathBuf::from(r"D:\data"));
    }

    // ---------- 目录树 ----------

    #[test]
    fn tree_visible_requires_both_switches() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let c = ctrl(&mut p, &mut v);
        assert!(!c.tree_visible(), "默认 Tree1=0 且 tree_mode=0");
        p.tree_visible = true;
        assert!(!ctrl(&mut p, &mut v).tree_visible(), "tree_mode 仍关");
        v.tree.set_config(TreeConfig {
            mode: crate::core::TreeMode::Split,
            ..TreeConfig::default()
        });
        assert!(ctrl(&mut p, &mut v).tree_visible(), "两个开关都开才显示");
    }

    #[test]
    fn tree_toggle_and_select_dir() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        assert!(c.tree_toggle(Path::new(r"C:\root")).expect("展开"), "首次要加载");
        c.tree_select_dir(Path::new(r"C:\root"), &ops()).expect("树选中");
        assert_eq!(c.path(), PathBuf::from(r"C:\root"));
    }

    #[test]
    fn tree_split_clamped_to_permille_max() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        c.set_tree_split(250);
        assert_eq!(c.tree_split(), 250);
        c.set_tree_split(99999);
        assert_eq!(c.tree_split(), 1000, "必须截断到 1000");
    }

    #[test]
    fn apply_tree_config_propagates() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let mut c = ctrl(&mut p, &mut v);
        c.apply_tree_config(TreeConfig {
            single_expand: true,
            ..TreeConfig::default()
        });
        assert!(c.view_tree().config().single_expand);
    }

    #[test]
    fn tree_sync_from_list_uses_pane_path() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        v.tree.set_config(TreeConfig {
            mode: crate::core::TreeMode::Split,
            auto_expand_from_list: true,
            auto_sel: true,
            ..TreeConfig::default()
        });
        let mut c = ctrl(&mut p, &mut v);
        c.navigate_to(Path::new(r"C:\root\sub"), &ops()).expect("导航");
        let loads = c.tree_sync_from_list();
        // 新树根是 sub（sync_root 已重挂），sub 自身未加载 -> 要加载 sub。
        assert_eq!(loads, vec![PathBuf::from(r"C:\root\sub")]);
        assert_eq!(c.view_tree().selected(), Some(Path::new(r"C:\root\sub")));
    }

    // ---------- 预览 ----------

    #[test]
    fn preview_for_selection_none_without_selection() {
        let mut p = pane(PaneId::P1, r"C:\root");
        let mut v = view(r"C:\root");
        let c = ctrl(&mut p, &mut v);
        let cfg = Config::default();
        assert_eq!(c.preview_for_selection(&[], &cfg).expect("无错误"), None);
    }

    #[test]
    fn preview_for_selection_skips_dirs_and_unlisted() {
        let dir = std::env::temp_dir().join("qdir-rs-pane-preview-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        let txt = dir.join("a.txt");
        std::fs::write(&txt, "hello\nworld").expect("写文本");

        let mut p = pane(PaneId::P1, &dir.to_string_lossy());
        let mut v = view(&dir.to_string_lossy());
        let cfg = Config::default();
        // 用真实路径构造条目：previewable 会去 stat 文件大小。
        let dir_item = FileItem {
            path: dir.clone(),
            name: "somedir".into(),
            is_dir: true,
            is_hidden: false,
            size: 0,
            modified: 0,
            icon_index: None,
        };
        let txt_item = FileItem {
            path: txt.clone(),
            name: "a.txt".into(),
            is_dir: false,
            is_hidden: false,
            size: 12,
            modified: 0,
            icon_index: None,
        };

        // 选中目录 -> None。
        v.select_one("somedir");
        assert_eq!(
            ctrl(&mut p, &mut v)
                .preview_for_selection(&[dir_item.clone()], &cfg)
                .expect("ok"),
            None
        );
        // 选中 txt，Filter=*; 放行 -> 文本预览。
        v.select_one("a.txt");
        let d = ctrl(&mut p, &mut v)
            .preview_for_selection(&[txt_item.clone()], &cfg)
            .expect("ok")
            .expect("应有预览");
        assert_eq!(d.kind(), preview_mod::PreviewKind::Text);
        // Filter 只放行 png -> None（不是错误）。
        let mut cfg2 = Config::default();
        cfg2.preview_filter = "*.png;".to_string();
        assert_eq!(
            ctrl(&mut p, &mut v)
                .preview_for_selection(&[txt_item], &cfg2)
                .expect("ok"),
            None
        );
    }

    // ---------- 会话桥接 ----------

    #[test]
    fn pane_views_from_session_matches_pane_paths() {
        let mut session = SessionState::default();
        // 四个窗格都显式设成可预期路径，避免依赖宿主机的 USERPROFILE。
        for (i, id) in PaneId::ALL.iter().enumerate() {
            session.pane_mut(*id).tabs[0].path = PathBuf::from(format!(r"C:\pane{}", i + 1));
        }
        let views = PaneViews::from_session(&session, TreeConfig::default());
        assert_eq!(views.get(PaneId::P2).tree.root_path(), Path::new(r"C:\pane2"));
        assert_eq!(
            views.get(PaneId::P1).tree.root_path(),
            Path::new(&session.pane(PaneId::P1).path())
        );
        assert!(!views.get(PaneId::P1).tree.config().mode.is_active());
    }

    #[test]
    fn pane_set_gives_independent_controllers() {
        let mut session = SessionState::default();
        let mut views = PaneViews::from_session(&session, TreeConfig::default());
        {
            let mut set = PaneSet::new(&mut session, &mut views);
            set.pane(PaneId::P1)
                .navigate_to(Path::new(r"C:\one"), &ops())
                .expect("P1 导航");
            set.pane(PaneId::P2)
                .navigate_to(Path::new(r"C:\two"), &ops())
                .expect("P2 导航");
            assert_eq!(set.state(PaneId::P1).path(), PathBuf::from(r"C:\one"));
            assert_eq!(set.state(PaneId::P2).path(), PathBuf::from(r"C:\two"));
            assert_eq!(set.view(PaneId::P1).tree.root_path(), Path::new(r"C:\one"));
        }
        // 会话状态真的被写回去了（PaneSet 持有的是 session 的可变引用）。
        assert_eq!(session.pane(PaneId::P1).path(), PathBuf::from(r"C:\one"));
        assert_eq!(session.pane(PaneId::P2).path(), PathBuf::from(r"C:\two"));
    }

    #[test]
    fn focused_path_helper() {
        let mut session = SessionState::default();
        session.focus = PaneId::P3;
        session.pane_mut(PaneId::P3).add_tab(PathBuf::from(r"C:\three"));
        assert_eq!(focused_path(&session), PathBuf::from(r"C:\three"));
    }

    #[test]
    fn apply_tree_config_touches_all_panes() {
        let mut views = PaneViews::from_session(&SessionState::default(), TreeConfig::default());
        apply_tree_config(
            &mut views,
            &TreeConfig {
                mode: crate::core::TreeMode::Split,
                indent_width: 24,
                ..TreeConfig::default()
            },
        );
        for id in PaneId::ALL {
            let c = views.get(id).tree.config();
            assert!(c.mode.is_active());
            assert_eq!(c.indent_width, 24);
        }
    }

    // ---------- 选择集 ----------

    #[test]
    fn selection_toggle_and_multi() {
        let mut v = view(r"C:\root");
        v.select_one("a");
        assert!(v.is_selected("a"));
        assert!(!v.is_selected("b"));
        assert!(v.toggle_selected("b"));
        assert_eq!(v.selection.len(), 2);
        assert!(!v.toggle_selected("b"));
        assert_eq!(v.selection.len(), 1);
        v.clear_selection();
        assert!(v.selection.is_empty());
        assert_eq!(v.focus_row, None);
    }
}
