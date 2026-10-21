//! 目录树：惰性展开的目录节点模型。
//!
//! 本模块是**纯状态层**：只描述「哪些节点被展开、哪些子目录已知」，
//! 不枚举文件系统。真正读盘由 shell 层（`crate::shell::enumerate`）完成，
//! 结果经 [`TreeModel::expand`] 灌进来，因此可在非 Windows 平台单测（G1）。
//!
//! 驱动配置全部来自 [`core::TreeConfig`]（ini 的 `tree_*` 键，由 config 层读取）：
//! - `tree_mode`：目录树全局开关（.qdr 的 `tree_mode` / 每窗格 `TreeN`）；
//! - `tree_single_expand`：同级只允许一个节点展开（手风琴）；
//! - `tree_auto_expand_from_list`：列表导航到的目录在树中自动展开；
//! - `tree_auto_sel`：树自动选中列表当前目录；
//! - `tree_indent_width`：每层缩进像素数（GUI 层消费，此处仅透传校验）；
//! - `tree_no_expando` / `tree_expand_when_select` 等其余开关由 GUI 层消费，
//!   本模块只在展开决策里用到会影响状态机的三个（上面列出的）。
//!
//! 依赖方向 D1：panes::tree 只依赖 core。

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::core::{QError, QResult, TreeConfig};

/// 单个树节点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    /// 目录绝对路径（规范化，分隔符为 `\`）。
    pub path: PathBuf,
    /// 显示名（通常是最后一段；盘符根用 `C:\` 形式）。
    pub name: String,
    /// 是否已加载过子目录。`false` 时 `children` 无意义（懒加载的「未加载」态）。
    pub loaded: bool,
    /// 已知子目录，**按路径排序**以保证渲染与测试可复现。
    pub children: Vec<TreeNode>,
    /// 是否展开。
    pub expanded: bool,
    /// 是否有子目录（懒加载后才知道；未加载时为 `None` = 未知）。
    pub has_children: Option<bool>,
}

impl TreeNode {
    /// 构造一个未加载的叶子节点。
    pub fn new(path: PathBuf) -> Self {
        let name = display_name(&path);
        TreeNode {
            path,
            name,
            loaded: false,
            children: Vec::new(),
            expanded: false,
            has_children: None,
        }
    }

    /// 子节点可变引用（按路径查找）。
    pub fn child_mut(&mut self, path: &Path) -> Option<&mut TreeNode> {
        self.children.iter_mut().find(|c| c.path == path)
    }

    /// 深度优先前序查找节点。
    pub fn find(&self, path: &Path) -> Option<&TreeNode> {
        if self.path == path {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(path))
    }

    /// 深度优先前序查找节点（可变）。
    pub fn find_mut(&mut self, path: &Path) -> Option<&mut TreeNode> {
        if self.path == path {
            return Some(self);
        }
        self.children.iter_mut().find_map(|c| c.find_mut(path))
    }

    /// 前序拍平所有节点路径（含自身）。
    pub fn flatten_paths(&self) -> Vec<PathBuf> {
        let mut out = vec![self.path.clone()];
        for c in &self.children {
            out.extend(c.flatten_paths());
        }
        out
    }
}

/// 路径的树显示名：盘符根（`C:\`）保留完整形式，其余取最后一段。
fn display_name(path: &Path) -> String {
    let s = path.to_string_lossy();
    let trimmed = s.trim_end_matches('\\');
    if trimmed.len() == s.len() - 1 && trimmed.ends_with(':') {
        // `C:\` -> `C:\`（去掉尾分隔符后只剩 `C:`）
        return format!("{trimmed}\\");
    }
    match path.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => s.to_string(),
    }
}

/// 路径的父目录。盘符根（`C:\`）的父是自身，防止无限上溯。
pub fn parent_dir(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let ps = parent.to_string_lossy();
    if ps.is_empty() || ps == "." {
        return None;
    }
    Some(parent.to_path_buf())
}

/// 目录树模型。
///
/// 只有一个「根」：当前窗格所在目录。原版 Q-Dir 的树可以从桌面一路展开到
/// 盘符根，但窗格的树根是 `pane.path()`；此处按同样的语义只维护一棵以
/// 当前目录为根的树，由 [`TreeModel::sync_root`] 在导航后重挂。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeModel {
    root: TreeNode,
    /// 展开中的节点路径集合。冗余于 `TreeNode::expanded`，用于 O(log n) 查询
    /// 与 `single_expand` 的同级判定。
    expanded: BTreeSet<PathBuf>,
    /// 树自动选中的路径（`tree_auto_sel` 生效时的当前目录）。
    selected: Option<PathBuf>,
    cfg: TreeConfig,
}

impl TreeModel {
    /// 以 `root` 为树根建模型。根节点初始为「未加载」。
    /// `indent_width` 为 0 时按 1 处理（与 [`TreeModel::set_config`] 同一约束，
    /// 保证无论从哪条路径进来都不会把 0 缩进泄给 GUI 层）。
    pub fn new(root: PathBuf, mut cfg: TreeConfig) -> Self {
        if cfg.indent_width == 0 {
            cfg.indent_width = 1;
        }
        TreeModel {
            root: TreeNode::new(root),
            expanded: BTreeSet::new(),
            selected: None,
            cfg,
        }
    }

    /// 当前生效的目录树配置引用。
    pub fn config(&self) -> &TreeConfig {
        &self.cfg
    }

    /// 替换配置（设置对话框改 `tree_*` 键后调用）。
    /// `indent_width` 为 0 时按 1 处理，避免 GUI 层除零（约定最小值 1）。
    pub fn set_config(&mut self, mut cfg: TreeConfig) {
        if cfg.indent_width == 0 {
            cfg.indent_width = 1;
        }
        self.cfg = cfg;
    }

    /// 树根路径。
    pub fn root_path(&self) -> &Path {
        &self.root.path
    }

    /// 树根节点。
    pub fn root(&self) -> &TreeNode {
        &self.root
    }

    /// 查找节点。
    pub fn find(&self, path: &Path) -> Option<&TreeNode> {
        self.root.find(path)
    }

    /// 查找节点（可变）。
    pub fn find_mut(&mut self, path: &Path) -> Option<&mut TreeNode> {
        self.root.find_mut(path)
    }

    /// 节点是否展开。
    pub fn is_expanded(&self, path: &Path) -> bool {
        self.expanded.contains(path)
    }

    /// 树当前选中项。
    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    /// 目录树是否启用（`tree_mode` 或窗格级 `TreeN`）。
    pub fn enabled(&self) -> bool {
        self.cfg.mode
    }

    /// 把树根重挂到 `new_root`。
    ///
    /// 若 `new_root` 在当前已加载的子树内，则**保留**该子树的展开/加载状态
    /// （避免每次导航都把用户的展开状态清空）；否则整棵树重建。
    /// 这是 `tree_auto_expand_from_list` 的基础：列表里进子目录时，
    /// 树不必从零懒加载。
    pub fn sync_root(&mut self, new_root: PathBuf) {
        if new_root == self.root.path {
            return;
        }
        // new_root 是旧根的后代：以它为新的根，砍掉上层。
        if new_root.starts_with(&self.root.path) {
            if let Some(node) = self.root.find(&new_root).cloned() {
                self.root = node;
                self.retain_expanded_under(&self.root.path.clone());
                self.selected = Some(self.root.path.clone());
                return;
            }
        }
        // 其它情况（跳到旁支/上层）：整树重建。
        self.root = TreeNode::new(new_root.clone());
        self.expanded.clear();
        self.selected = Some(new_root);
    }

    /// 丢弃不在 `under` 之下的展开记录。
    fn retain_expanded_under(&mut self, under: &Path) {
        self.expanded.retain(|p| p.starts_with(under));
    }

    /// 加载（或刷新）`path` 的子目录列表。
    ///
    /// `dirs` 由调用方（shell 层）枚举得到，**只需目录**；本模块按路径排序后
    /// 替换子节点，并保留已有子节点的 `expanded` / `loaded` 状态
    /// （刷新不丢展开态）。`path` 不在树内时返回 [`QError::Session`]。
    pub fn expand(&mut self, path: &Path, dirs: Vec<PathBuf>) -> QResult<()> {
        let node = self
            .root
            .find_mut(path)
            .ok_or_else(|| QError::Session(format!("目录树中不存在节点：{}", path.display())))?;
        // 旧子节点的展开态，按路径保存，刷新后恢复。
        let mut keep: Vec<(PathBuf, bool)> = Vec::new();
        for c in &node.children {
            collect_expanded(c, &mut keep);
        }
        let mut children: Vec<TreeNode> = dirs.into_iter().map(TreeNode::new).collect();
        children.sort_by(|a, b| a.path.cmp(&b.path));
        for c in &mut children {
            if let Some((_, was)) = keep.iter().find(|(p, _)| *p == c.path) {
                c.expanded = *was;
                if *was {
                    self.expanded.insert(c.path.clone());
                }
                // 刷新保留「已加载」标记：子节点列表下次刷新再覆盖，
                // 这样 GUI 层不会看到「已展开却未加载」的中间态。
                c.loaded = true;
            }
        }
        node.children = children;
        node.loaded = true;
        node.has_children = Some(!node.children.is_empty());
        Ok(())
    }

    /// 标记 `path` 已加载但无子目录（空目录也要消掉「未加载」的展开箭头）。
    pub fn mark_loaded_empty(&mut self, path: &Path) -> QResult<()> {
        let node = self
            .root
            .find_mut(path)
            .ok_or_else(|| QError::Session(format!("目录树中不存在节点：{}", path.display())))?;
        node.loaded = true;
        node.children.clear();
        node.has_children = Some(false);
        Ok(())
    }

    /// 是否需要为 `path` 加载子目录（懒加载判定）。
    pub fn needs_load(&self, path: &Path) -> bool {
        match self.root.find(path) {
            Some(n) => !n.loaded,
            None => false,
        }
    }

    /// 展开 `path`。
    ///
    /// `tree_single_expand` 生效时先折叠同一父节点下的其它已展开兄弟
    /// （原版「单一展开」语义：手风琴）。
    /// 返回是否需要调用方去枚举子目录（`true` = 尚未加载）。
    pub fn expand_node(&mut self, path: &Path) -> QResult<bool> {
        // single_expand：折叠同父的其它兄弟。
        if self.cfg.single_expand {
            if let Some(parent) = parent_dir(path) {
                let siblings: Vec<PathBuf> = self
                    .root
                    .find(&parent)
                    .map(|n| n.children.iter().map(|c| c.path.clone()).collect())
                    .unwrap_or_default();
                for sib in siblings {
                    if sib != path && self.expanded.contains(&sib) {
                        self.collapse_subtree(&sib);
                    }
                }
            }
        }
        let node = self
            .root
            .find_mut(path)
            .ok_or_else(|| QError::Session(format!("目录树中不存在节点：{}", path.display())))?;
        node.expanded = true;
        self.expanded.insert(node.path.clone());
        Ok(!node.loaded)
    }

    /// 折叠 `path` 及其整个子树。
    pub fn collapse_subtree(&mut self, path: &Path) {
        if let Some(node) = self.root.find(path) {
            let all = node.flatten_paths();
            for p in all {
                if let Some(n) = self.root.find_mut(&p) {
                    n.expanded = false;
                }
                self.expanded.remove(&p);
            }
        }
    }

    /// 折叠 `path`（只折一层，子节点的展开态保留，再次展开时恢复）。
    pub fn collapse(&mut self, path: &Path) {
        if let Some(node) = self.root.find_mut(path) {
            node.expanded = false;
        }
        self.expanded.remove(path);
    }

    /// 切换 `path` 的展开态；返回切换后是否需要调用方枚举子目录。
    pub fn toggle(&mut self, path: &Path) -> QResult<bool> {
        if self.is_expanded(path) {
            self.collapse(path);
            Ok(false)
        } else {
            self.expand_node(path)
        }
    }

    /// `tree_auto_sel` + `tree_auto_expand_from_list`：
    /// 列表导航到 `dir` 后调用。返回**需要加载的路径序列**（从根到 `dir`，
    /// 只含尚未加载的），调用方据此让 shell 层逐个枚举。
    ///
    /// 两个开关独立生效：
    /// - `auto_sel`：把树选中项移到 `dir`；
    /// - `auto_expand_from_list`：把 `dir` 的所有祖先（含自身）展开。
    pub fn sync_from_list(&mut self, dir: &Path) -> Vec<PathBuf> {
        if self.cfg.auto_sel {
            self.selected = Some(dir.to_path_buf());
        }
        if !self.cfg.auto_expand_from_list {
            return Vec::new();
        }
        // 祖先链：dir 的所有祖先 + dir 自身，从最上层开始。
        let mut chain: Vec<PathBuf> = Vec::new();
        let mut cur = Some(dir.to_path_buf());
        while let Some(p) = cur {
            chain.push(p.clone());
            cur = parent_dir(&p);
        }
        chain.reverse();
        let mut to_load = Vec::new();
        for p in &chain {
            if !p.starts_with(&self.root.path) && *p != self.root.path {
                continue; // 不在当前树根范围内
            }
            let need = match self.root.find(p) {
                Some(n) => !n.loaded,
                None => true,
            };
            if need {
                to_load.push(p.clone());
            }
            // 展开动作只在节点已存在时记录；不存在的节点等加载后由调用方再展开。
            if self.root.find(p).is_some() && !self.is_expanded(p) {
                let _ = self.expand_node(p);
            }
        }
        to_load
    }

    /// 前序拍平「可见」节点（折叠子树不展开），供 GUI 层直接渲染。
    /// 每项为 `(路径, 层级)`，层级从 0 开始。
    pub fn visible_rows(&self) -> Vec<(PathBuf, usize)> {
        let mut out = Vec::new();
        walk_visible(&self.root, 0, &mut out);
        out
    }

    /// 树中已加载的节点数（测试与诊断用）。
    pub fn loaded_count(&self) -> usize {
        self.root.flatten_paths().iter().filter(|p| self.root.find(p).is_some_and(|n| n.loaded)).count()
    }
}

/// 前序收集某节点的展开后代。
fn collect_expanded(node: &TreeNode, out: &mut Vec<(PathBuf, bool)>) {
    for c in &node.children {
        out.push((c.path.clone(), c.expanded));
        collect_expanded(c, out);
    }
}

/// 前序走可见节点。
fn walk_visible(node: &TreeNode, depth: usize, out: &mut Vec<(PathBuf, usize)>) {
    out.push((node.path.clone(), depth));
    if !node.expanded {
        return;
    }
    for c in &node.children {
        walk_visible(c, depth + 1, out);
    }
}

/// 路径标准化：把 `.` / `..` 按词法规则折叠，分隔符统一为 `\`。
///
/// 只做**词法**规范化（不触盘、不解析符号链接/ junction），因此可在任何
/// 平台单测；真实路径的最终规范化由 shell 枚举层保证。
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    let mut prefix: Option<std::ffi::OsString> = None;
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => prefix = Some(p.as_os_str().to_os_string()),
            Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                // 只在还能回退时才弹栈；盘符前的 `..` 忽略。
                if out.last().is_some() {
                    out.pop();
                }
            }
            Component::Normal(s) => out.push(s.to_os_string()),
        }
    }
    // 手工拼字符串而不是用 PathBuf::push：push 遇到「盘符前缀」会**替换**已有内容
    // （Windows 上 `C:\` 之后 push("\\") 再 push("root") 得到 `\root`），
    // 导致 C:\a\b\..\c 规范成 C:\c。拼接 OsString 再一次性构造可绕开该语义。
    let mut s = String::new();
    if let Some(p) = &prefix {
        s.push_str(&p.to_string_lossy());
        s.push('\\');
    }
    for (i, seg) in out.iter().enumerate() {
        if i > 0 {
            s.push('\\');
        }
        s.push_str(&seg.to_string_lossy());
    }
    if s.is_empty() {
        return path.to_path_buf();
    }
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> TreeConfig {
        TreeConfig::default()
    }

    fn item(p: &str) -> PathBuf {
        PathBuf::from(p)
    }

    #[test]
    fn lazy_load_starts_unloaded() {
        let t = TreeModel::new(item(r"C:\root"), cfg());
        assert!(!t.root().loaded);
        assert!(t.needs_load(Path::new(r"C:\root")));
        assert_eq!(t.visible_rows(), vec![(item(r"C:\root"), 0)]);
    }

    #[test]
    fn expand_loads_children_and_keeps_sort_order() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        t.expand(
            Path::new(r"C:\root"),
            vec![item(r"C:\root\zeta"), item(r"C:\root\alpha")],
        )
        .expect("展开根");
        assert!(t.root().loaded);
        assert_eq!(t.root().has_children, Some(true));
        assert_eq!(t.root().children.len(), 2);
        assert_eq!(t.root().children[0].path, item(r"C:\root\alpha"));
        assert_eq!(t.root().children[1].path, item(r"C:\root\zeta"));

        // 只加载数据不改变展开态：expand() 后仍只有根一行可见。
        let rows = t.visible_rows();
        assert_eq!(rows.len(), 1, "expand 只灌数据，展开由 expand_node/toggle 决定");
        // 展开后子节点才可见，且按排序后的顺序。
        t.expand_node(Path::new(r"C:\root")).expect("展开根");
        let rows = t.visible_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], (item(r"C:\root"), 0));
        assert_eq!(rows[1], (item(r"C:\root\alpha"), 1));
        assert_eq!(rows[2], (item(r"C:\root\zeta"), 1));
    }

    #[test]
    fn expand_unknown_node_is_session_error() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        let err = t.expand(Path::new(r"C:\nope"), vec![]).unwrap_err();
        assert!(matches!(err, QError::Session(_)));
    }

    #[test]
    fn toggle_collapses_and_reexpands() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        assert!(t.toggle(Path::new(r"C:\root")).expect("展开"), "首次展开需要加载");
        assert!(t.is_expanded(Path::new(r"C:\root")));
        assert!(!t.toggle(Path::new(r"C:\root")).expect("折叠"), "折叠不需要加载");
        assert!(!t.is_expanded(Path::new(r"C:\root")));
    }

    #[test]
    fn single_expand_collapses_siblings_only() {
        let mut cfg = cfg();
        cfg.single_expand = true;
        let mut t = TreeModel::new(item(r"C:\root"), cfg);
        t.expand(
            Path::new(r"C:\root"),
            vec![item(r"C:\root\a"), item(r"C:\root\b")],
        )
        .expect("加载根");
        t.expand_node(Path::new(r"C:\root")).expect("展开根");
        t.expand(Path::new(r"C:\root\a"), vec![item(r"C:\root\a\x")])
            .expect("加载 a");
        t.expand_node(Path::new(r"C:\root\a")).expect("展开 a");
        t.expand(Path::new(r"C:\root\b"), vec![]).expect("加载 b");
        t.expand_node(Path::new(r"C:\root\b")).expect("展开 b");
        // b 展开后，同父的 a 必须被折叠（手风琴），但 a 的子节点 x 的展开态要保留。
        assert!(!t.is_expanded(Path::new(r"C:\root\a")));
        assert!(t.is_expanded(Path::new(r"C:\root\b")));
        // a 的子节点仍在模型里（折叠不卸载）。
        let a = t.find(Path::new(r"C:\root\a")).expect("a 还在");
        assert_eq!(a.children.len(), 1);
        assert_eq!(a.children[0].path, item(r"C:\root\a\x"));
    }

    #[test]
    fn single_expand_disabled_keeps_both_siblings() {
        let mut cfg = cfg();
        cfg.single_expand = false;
        let mut t = TreeModel::new(item(r"C:\root"), cfg);
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\a"), item(r"C:\root\b")])
            .expect("加载根");
        t.expand(Path::new(r"C:\root\a"), vec![]).expect("加载 a");
        t.expand_node(Path::new(r"C:\root\a")).expect("展开 a");
        t.expand(Path::new(r"C:\root\b"), vec![]).expect("加载 b");
        t.expand_node(Path::new(r"C:\root\b")).expect("展开 b");
        assert!(t.is_expanded(Path::new(r"C:\root\a")));
        assert!(t.is_expanded(Path::new(r"C:\root\b")));
    }

    #[test]
    fn sync_root_keeps_subtree_when_descending() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\sub")])
            .expect("加载根");
        t.expand_node(Path::new(r"C:\root")).expect("展开根");
        t.expand(Path::new(r"C:\root\sub"), vec![item(r"C:\root\sub\leaf")])
            .expect("加载 sub");
        t.expand_node(Path::new(r"C:\root\sub")).expect("展开 sub");
        assert!(t.is_expanded(Path::new(r"C:\root\sub")));

        // 下潜到 sub：整棵 sub 子树（含 leaf）必须保留。
        t.sync_root(item(r"C:\root\sub"));
        assert_eq!(t.root_path(), Path::new(r"C:\root\sub"));
        assert!(t.is_expanded(Path::new(r"C:\root\sub")));
        assert_eq!(
            t.find(Path::new(r"C:\root\sub\leaf")).map(|n| n.path.clone()),
            Some(item(r"C:\root\sub\leaf"))
        );
    }

    #[test]
    fn sync_root_rebuilds_on_jump() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\sub")])
            .expect("展开根");
        t.sync_root(item(r"C:\other"));
        assert_eq!(t.root_path(), Path::new(r"C:\other"));
        assert!(!t.is_expanded(Path::new(r"C:\root\sub")));
        assert!(t.find(Path::new(r"C:\root\sub")).is_none());
    }

    #[test]
    fn sync_from_list_expands_chain_and_reports_loads() {
        let mut cfg = cfg();
        cfg.auto_expand_from_list = true;
        cfg.auto_sel = true;
        let mut t = TreeModel::new(item(r"C:\root"), cfg);
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\a")])
            .expect("展开根");
        // a 未加载：同步到 a 时应报告要加载 a，并展开 a。
        let loads = t.sync_from_list(Path::new(r"C:\root\a"));
        assert_eq!(loads, vec![item(r"C:\root\a")]);
        assert!(t.is_expanded(Path::new(r"C:\root\a")));
        assert_eq!(t.selected(), Some(Path::new(r"C:\root\a")));
        // 根已加载：不再重复要求加载。
        let loads = t.sync_from_list(Path::new(r"C:\root"));
        assert!(loads.is_empty());
    }

    #[test]
    fn sync_from_list_respects_disabled_switches() {
        let mut cfg = cfg();
        cfg.auto_expand_from_list = false;
        cfg.auto_sel = false;
        let mut t = TreeModel::new(item(r"C:\root"), cfg);
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\a")])
            .expect("展开根");
        assert!(t.sync_from_list(Path::new(r"C:\root\a")).is_empty());
        assert!(!t.is_expanded(Path::new(r"C:\root\a")));
        assert_eq!(t.selected(), None);
    }

    #[test]
    fn refresh_keeps_expand_state_of_children() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\a")])
            .expect("加载根");
        t.expand_node(Path::new(r"C:\root")).expect("展开根");
        t.expand(Path::new(r"C:\root\a"), vec![item(r"C:\root\a\x")])
            .expect("加载 a");
        t.expand_node(Path::new(r"C:\root\a")).expect("展开 a");
        assert!(t.is_expanded(Path::new(r"C:\root\a")));

        // 只刷新根：a 的展开态保留，但 a 自身是新建的未加载节点
        // —— 深一层的数据要等 GUI 层再次请求加载（懒加载语义：
        //   刷新只换当前层的子目录列表，不递归重读整棵树）。
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\a")])
            .expect("刷新根");
        assert!(t.is_expanded(Path::new(r"C:\root\a")));
        let a = t.find(Path::new(r"C:\root\a")).expect("a 还在");
        assert!(!a.loaded, "刷新后的子节点是新的未加载节点");
        assert!(a.children.is_empty());

        // 重新加载 a 后 x 回来。
        t.expand(Path::new(r"C:\root\a"), vec![item(r"C:\root\a\x")])
            .expect("再加载 a");
        assert_eq!(
            t.find(Path::new(r"C:\root\a\x")).map(|n| n.path.clone()),
            Some(item(r"C:\root\a\x"))
        );
        assert!(t.is_expanded(Path::new(r"C:\root\a")), "展开态跨刷新保留");
    }

    #[test]
    fn mark_loaded_empty_clears_expando() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        t.mark_loaded_empty(Path::new(r"C:\root")).expect("标记空");
        assert!(t.root().loaded);
        assert_eq!(t.root().has_children, Some(false));
        assert!(!t.needs_load(Path::new(r"C:\root")));
    }

    #[test]
    fn indent_width_zero_is_clamped_to_one() {
        let mut cfg = cfg();
        cfg.indent_width = 0;
        let t = TreeModel::new(item(r"C:\root"), cfg);
        assert_eq!(t.config().indent_width, 1, "0 缩进会让 GUI 层除零/无法分辨层级");
    }

    #[test]
    fn parent_dir_stops_at_drive_root() {
        assert_eq!(parent_dir(Path::new(r"C:\a\b")), Some(item(r"C:\a")));
        assert_eq!(parent_dir(Path::new(r"C:\")), None);
        assert_eq!(parent_dir(Path::new("relative")), None);
    }

    #[test]
    fn normalize_path_lexical_only() {
        assert_eq!(normalize_path(Path::new(r"C:\a\b\..\c")), item(r"C:\a\c"));
        assert_eq!(normalize_path(Path::new(r"C:\a\.\b")), item(r"C:\a\b"));
        assert_eq!(normalize_path(Path::new(r"C:\..\a")), item(r"C:\a"));
        // 不触盘：不存在的路径也照样规范化。
        assert_eq!(normalize_path(Path::new("a/b/../c")), item("a/c"));
    }

    #[test]
    fn display_name_keeps_drive_root_form() {
        let n = TreeNode::new(item(r"C:\"));
        assert_eq!(n.name, r"C:\");
        let n2 = TreeNode::new(item(r"C:\Users\bob"));
        assert_eq!(n2.name, "bob");
    }

    #[test]
    fn collapse_subtree_folds_descendants() {
        let mut t = TreeModel::new(item(r"C:\root"), cfg());
        t.expand(Path::new(r"C:\root"), vec![item(r"C:\root\a")])
            .expect("加载根");
        t.expand_node(Path::new(r"C:\root")).expect("展开根");
        t.expand(Path::new(r"C:\root\a"), vec![item(r"C:\root\a\b")])
            .expect("加载 a");
        t.expand_node(Path::new(r"C:\root\a")).expect("展开 a");
        t.expand(Path::new(r"C:\root\a\b"), vec![]).expect("加载 b");
        t.expand_node(Path::new(r"C:\root\a\b")).expect("展开 b");
        assert!(t.is_expanded(Path::new(r"C:\root\a\b")));
        t.collapse_subtree(Path::new(r"C:\root\a"));
        assert!(!t.is_expanded(Path::new(r"C:\root\a")));
        assert!(!t.is_expanded(Path::new(r"C:\root\a\b")));
    }
}
