//! 窗格目录树（原生 SysTreeView32）。
//!
//! 行为规格（NATIVE_SPEC 附录 A.3，Ghidra `FUN_0046dc10` 确证，配置键 `tree_mode`）：
//! - `tree_mode=0`：关树；
//! - `tree_mode=1`：双列——树在窗格内容区左侧，宽度 = `PaneState::tree_split`（千分比）；
//! - `tree_mode=2`：覆盖——树浮在列表区上方；
//! - viewtree 按钮（0xA773）/ 菜单 30029：0↔1 开/关（原 `0x806c/0x806b`）；
//!   54006 系：+1 循环（原 `0x806a`）。
//!
//! 树的形态按原版观感自设计（R3：界面自主；布局/交互/核心功能一致）：
//! 盘符根并列 + 懒加载子目录（展开时枚举）+ 根行复选框（状态=窗格 `TreeN` 位）。
//! 列表导航联动（`tree_auto_expand_from_list`/`tree_auto_sel` 语义）：进入目录时
//! 自动展开祖先链并选中；树选中变化驱动该窗格列表导航。
//!
//! 纯几何/解码函数（[`tree_layout_for`] / [`tree_column_rect`] / [`tree_overlay_rect`]）
//! 平台无关，单测覆盖。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Storage::FileSystem::GetLogicalDriveStringsW;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

/// 双列模式的分割带宽度（像素）。
pub const TREE_GAP: i32 = 4;

/// 千分比树宽钳制区间（太窄没法点，太宽挤死列表）。
const SPLIT_MIN: u32 = 80;
const SPLIT_MAX: u32 = 700;

/// 按 tree_mode 决定本窗格布局形态。
/// 返回 `None` = 无树；`Some(false)` = 双列；`Some(true)` = 覆盖。
/// 窗格级 `TreeN` 位（.qdr）由调用方叠加：位关则该窗格不显示树。
pub fn tree_layout_for(mode: u32) -> Option<bool> {
    match mode {
        0 => None,
        // 1=双列；2=覆盖。复选框位不改模式选择——它按原版语义勾选
        // 「本窗格跟随子目录」；模式映射严格按 tree_mode 数值。
        1 => Some(false),
        2 => Some(true),
        // >2 视为 1（防御：.qdr 手改出的野值）。
        _ => Some(false),
    }
}

/// 双列模式：从内容区切出 (树矩形, 列表矩形)。宽度为 0 的返回 `None`。
pub fn tree_column_rect(
    content: (i32, i32, i32, i32),
    split_permille: u32,
) -> Option<((i32, i32, i32, i32), (i32, i32, i32, i32))> {
    let (x, y, w, h) = content;
    if w <= TREE_GAP * 2 || h <= 0 {
        return None;
    }
    let perm = split_permille.clamp(SPLIT_MIN, SPLIT_MAX);
    let tw = ((w as i64 * perm as i64) / 1000).max(60) as i32;
    let tree = (x, y, tw, h);
    let list = (x + tw + TREE_GAP, y, (w - tw - TREE_GAP).max(1), h);
    Some((tree, list))
}

/// 覆盖模式：树浮在内容区左上（宽度 ~45%，保证列表右缘仍可交互）。
pub fn tree_overlay_rect(content: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    let (x, y, w, h) = content;
    let tw = ((w as i64 * 45) / 100).max(120) as i32;
    (x + 6, y + 4, tw.min((w - 12).max(1)), h.saturating_sub(8).max(20))
}

/// 显示名：盘符根（`C:\`）保留完整形式，其余取最后一段——与
/// `panes::tree::display_name` 同规则。
pub fn display_name(path: &Path) -> String {
    let s = path.to_string_lossy();
    let trimmed = s.trim_end_matches('\\');
    if trimmed.ends_with(':') {
        // 盘符根统一显示为 `C:\`（`C:` 与 `C:\` 都归一）。
        return format!("{trimmed}\\");
    }
    match path.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => s.to_string(),
    }
}

/// 盘符根列表（升序，`C:\` 形式）。
pub fn drive_roots() -> Vec<PathBuf> {
    let mut buf = [0u16; 512];
    let n = unsafe { GetLogicalDriveStringsW(Some(&mut buf)) };
    let mut out: Vec<PathBuf> = Vec::new();
    if n == 0 {
        return out;
    }
    let mut s: &[u16] = &buf[..n as usize];
    while !s.is_empty() {
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        if end == 0 {
            break;
        }
        out.push(PathBuf::from(String::from_utf16_lossy(&s[..end])));
        s = &s[end + 1..];
    }
    out.sort();
    out
}

/// 树根伪句柄（TVI_ROOT）：顶层插入必须用它，HTREEITEM(0) 会静默失败。
pub fn tvi_root() -> HTREEITEM {
    HTREEITEM(-65536isize as _)
}

/// 盘符路径规范化：`C:` -> `C:\`（树根存储形式，与 display_name 互逆）。
pub fn normalize_drive(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s.len() == 2 && s.as_bytes()[1] == b':' {
        return PathBuf::from(s.to_string() + "\\");
    }
    path.to_path_buf()
}

fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub use windows::Win32::UI::Controls::{TVN_ITEMEXPANDEDW, TVN_SELCHANGEDW, TVE_EXPAND};
/// TVN_ITEMSTATEICONCLICKW（`TVN_FIRST`-68，windows-rs 0.62 未导出，按 SDK 值定义）。
pub const TVN_ITEMSTATEICONCLICKW: u32 = 4294966896u32 - 68;

/// 节点登记簿：HTREEITEM.0 → 目录路径；已枚举过子目录的项集合。
#[derive(Default)]
pub struct TreeNodes {
    pub path_of: HashMap<isize, PathBuf>,
    pub loaded: Vec<isize>,
    /// 程序化操作（reveal/checkbox 同步）中，抑制通知回环。
    pub busy: bool,
}

impl TreeNodes {
    pub fn register(&mut self, h: HTREEITEM, path: &Path) -> isize {
        let key = h.0;
        self.path_of.insert(key, path.to_path_buf());
        key
    }
    pub fn path(&self, h: HTREEITEM) -> Option<&Path> {
        self.path_of.get(&h.0).map(|p| p.as_path())
    }
    pub fn is_loaded(&self, h: HTREEITEM) -> bool {
        self.loaded.contains(&h.0)
    }
    pub fn mark_loaded(&mut self, h: HTREEITEM) {
        if !self.loaded.contains(&h.0) {
            self.loaded.push(h.0);
        }
    }
    /// 窗格路径变化/重建前清空登记（HTREEITEM 失效）。
    pub fn clear(&mut self) {
        self.path_of.clear();
        self.loaded.clear();
    }
}

/// TVM_GETNEXTITEM 封装。
pub unsafe fn tree_next(tv: HWND, walk: u32, from: HTREEITEM) -> HTREEITEM {
    HTREEITEM(
        SendMessageW(tv, TVM_GETNEXTITEM, Some(WPARAM(walk as usize)), Some(LPARAM(from.0 as isize))).0,
    )
}

fn valid(h: HTREEITEM) -> bool {
    h.0 != 0
}

/// 插入目录节点（懒加载占位：cChildren=1，首次展开时枚举校正）。
/// 返回新节点；登记进 `nodes`。
pub unsafe fn insert_dir_node(
    tv: HWND,
    parent: HTREEITEM,
    path: &Path,
    image: i32,
    nodes: &mut TreeNodes,
) -> HTREEITEM {
    let mut name = display_name(path);
    if name.is_empty() {
        name = path.to_string_lossy().to_string();
    }
    let mut zbuf = wide_z(&name);
    let mask = TVITEM_MASK(TVIF_TEXT.0 | TVIF_CHILDREN.0 | TVIF_PARAM.0 | TVIF_IMAGE.0 | TVIF_SELECTEDIMAGE.0);
    let item = TVITEMW {
        mask,
        pszText: PWSTR::from_raw(zbuf.as_mut_ptr()),
        cChildren: TVITEMEXW_CHILDREN(1), // 占位「可能有子项」
        iImage: image,
        iSelectedImage: image,
        ..Default::default()
    };
    let mut ins = TVINSERTSTRUCTW {
        hParent: parent,
        hInsertAfter: HTREEITEM(-2isize as _), // TVI_LAST
        Anonymous: TVINSERTSTRUCTW_0 { item },
    };
    let r = SendMessageW(
        tv,
        TVM_INSERTITEMW,
        Some(WPARAM(0)),
        Some(LPARAM(&mut ins as *mut TVINSERTSTRUCTW as isize)),
    );
    let h = HTREEITEM(r.0);
    nodes.register(h, path);
    h
}

/// 懒加载 `hItem` 的直接子目录；空目录清掉展开箭头。
pub unsafe fn load_child_dirs(
    tv: HWND,
    hitem: HTREEITEM,
    dir: &Path,
    folder_image: i32,
    nodes: &mut TreeNodes,
) {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if e.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                dirs.push(e.path());
            }
        }
    }
    dirs.sort();
    if dirs.is_empty() {
        let mut it = TVITEMW {
            mask: TVIF_CHILDREN,
            hItem: hitem,
            cChildren: TVITEMEXW_CHILDREN(0),
            ..Default::default()
        };
        SendMessageW(
            tv,
            TVM_SETITEMW,
            Some(WPARAM(0)),
            Some(LPARAM(&mut it as *mut TVITEMW as isize)),
        );
    } else {
        for d in dirs {
            insert_dir_node(tv, hitem, &d, folder_image, nodes);
        }
    }
    nodes.mark_loaded(hitem);
}

/// 展开/折叠节点。`expand=true` 展开。
pub unsafe fn set_expand(tv: HWND, h: HTREEITEM, expand: bool) {
    let act = if expand { TVE_EXPAND.0 } else { TVE_COLLAPSE.0 };
    let _ = SendMessageW(tv, TVM_EXPAND, Some(WPARAM(act as usize)), Some(LPARAM(h.0 as isize)));
}

/// 选中并确保可见。
pub unsafe fn select_node(tv: HWND, h: HTREEITEM) {
    let _ = SendMessageW(tv, TVM_SELECTITEM, Some(WPARAM(TVGN_CARET as usize)), Some(LPARAM(h.0 as isize)));
    let _ = SendMessageW(tv, TVM_ENSUREVISIBLE, Some(WPARAM(0)), Some(LPARAM(h.0 as isize)));
}

/// 读节点 lParam（未用；保留 path_of 作唯一真相）。
pub unsafe fn node_state_image(tv: HWND, h: HTREEITEM) -> u32 {
    let mut it = TVITEMW {
        mask: TVIF_STATE,
        hItem: h,
        stateMask: TVIS_STATEIMAGEMASK,
        ..Default::default()
    };
    SendMessageW(tv, TVM_GETITEMW, Some(WPARAM(0)), Some(LPARAM(&mut it as *mut TVITEMW as isize)));
    ((it.state.0 & TVIS_STATEIMAGEMASK.0) >> 12) as u32
}

/// 设置根节点复选框状态图（1=空框，2=勾选）。
pub unsafe fn set_state_image(tv: HWND, h: HTREEITEM, idx: u32) {
    let mut it = TVITEMW {
        mask: TVIF_STATE,
        hItem: h,
        state: TREE_VIEW_ITEM_STATE_FLAGS((idx << 12) & TVIS_STATEIMAGEMASK.0),
        stateMask: TVIS_STATEIMAGEMASK,
        ..Default::default()
    };
    SendMessageW(tv, TVM_SETITEMW, Some(WPARAM(0)), Some(LPARAM(&mut it as *mut TVITEMW as isize)));
}

/// 在 `parent` 的直接孩子里找路径匹配 `target` 的节点。
pub unsafe fn find_child_by_path(tv: HWND, parent: HTREEITEM, target: &Path, nodes: &TreeNodes) -> HTREEITEM {
    let mut c = tree_next(tv, TVGN_CHILD, parent);
    while valid(c) {
        if nodes.path(c) == Some(target) {
            return c;
        }
        c = tree_next(tv, TVGN_NEXT, c);
    }
    HTREEITEM(0)
}

/// 在整棵树（各盘根之下）找目标路径的节点。
pub unsafe fn find_item_by_path(tv: HWND, target: &Path, nodes: &TreeNodes) -> HTREEITEM {
    let mut root = tree_next(tv, TVGN_ROOT, tvi_root());
    while valid(root) {
        if nodes.path(root) == Some(target) {
            return root;
        }
        // 深度优先（限已加载子树）。
        if let Some(h) = unsafe { find_item_below(tv, root, target, nodes) } {
            return h;
        }
        root = tree_next(tv, TVGN_NEXT, root);
    }
    HTREEITEM(0)
}

unsafe fn find_item_below(
    tv: HWND,
    cur: HTREEITEM,
    target: &Path,
    nodes: &TreeNodes,
) -> Option<HTREEITEM> {
    if !nodes.is_loaded(cur) {
        return None;
    }
    let mut c = tree_next(tv, TVGN_CHILD, cur);
    while valid(c) {
        if nodes.path(c) == Some(target) {
            return Some(c);
        }
        if let Some(h) = find_item_below(tv, c, target, nodes) {
            return Some(h);
        }
        c = tree_next(tv, TVGN_NEXT, c);
    }
    None
}

/// 展开祖先链使 `target` 可见并选中（自每个盘根尝试；懒枚举沿途缺失层）。
/// 调用方负责置 `nodes.busy` 抑制 TVN_* 回环。
pub unsafe fn reveal(
    tv: HWND,
    target: &Path,
    nodes: &mut TreeNodes,
    folder_image: i32,
) -> bool {
    let target = normalize_drive(target);
    let mut root = tree_next(tv, TVGN_ROOT, tvi_root());
    while valid(root) {
        let rp = match nodes.path(root) {
            Some(p) => normalize_drive(p),
            None => {
                root = tree_next(tv, TVGN_NEXT, root);
                continue;
            }
        };
        if target.starts_with(&rp) {
            let chain = ancestor_chain(&target);
            // 链可能多一层 `C:`（父级词法产物）；从根路径本身之后开始下潜。
            let start = chain.iter().position(|p| p == &rp).map(|i| i + 1).unwrap_or(0);
            let mut cur = root;
            for seg in chain.iter().skip(start) {
                let mut child = find_child_by_path(tv, cur, seg, nodes);
                if !valid(child) && !nodes.is_loaded(cur) {
                    let Some(dir) = nodes.path(cur).map(|x| x.to_path_buf()) else {
                        return false;
                    };
                    load_child_dirs(tv, cur, &dir, folder_image, nodes);
                    child = find_child_by_path(tv, cur, seg, nodes);
                }
                if !valid(child) {
                    return false;
                }
                set_expand(tv, cur, true);
                cur = child;
            }
            set_expand(tv, cur, true);
            select_node(tv, cur);
            return true;
        }
        root = tree_next(tv, TVGN_NEXT, root);
    }
    false
}

/// 祖先链（含自身）：`C:\a\b` → `C:\`、`C:\a`、`C:\a\b`。
pub fn ancestor_chain(target: &Path) -> Vec<PathBuf> {
    let mut chain: Vec<PathBuf> = Vec::new();
    let mut cur = Some(target.to_path_buf());
    while let Some(p) = cur {
        chain.push(p.clone());
        cur = p.parent().map(|x| x.to_path_buf());
    }
    chain.reverse();
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_decodes_to_layout() {
        assert_eq!(tree_layout_for(0), None, "tree_mode=0 一律无树");
        assert_eq!(tree_layout_for(1), Some(false), "双列");
        assert_eq!(tree_layout_for(2), Some(true), "覆盖");
        assert_eq!(tree_layout_for(3), Some(false), "野值兜底=双列");
    }

    #[test]
    fn column_rect_splits_by_permille() {
        let (t, l) = tree_column_rect((0, 0, 1000, 500), 250).expect("切分");
        assert_eq!(t.2, 250, "千分比 250 -> 250px");
        assert_eq!(l.0, 254, "列表从 树宽+分割带 起");
        assert_eq!(l.2, 1000 - 254, "列表吃掉剩余");
        let (t2, _) = tree_column_rect((0, 0, 1000, 500), 1).unwrap();
        assert_eq!(t2.2, 80, "下限钳制");
        let (t3, _) = tree_column_rect((0, 0, 1000, 500), 999).unwrap();
        assert_eq!(t3.2, 700, "上限钳制");
    }

    #[test]
    fn column_rect_none_on_tiny_width() {
        assert!(tree_column_rect((0, 0, TREE_GAP * 2, 500), 250).is_none());
    }

    #[test]
    fn overlay_rect_fits_content() {
        let o = tree_overlay_rect((10, 20, 800, 400));
        assert!(o.2 <= 800 - 12, "覆盖树必须留出列表右缘");
        assert_eq!(o.0, 16);
        assert_eq!(o.1, 24);
        let o2 = tree_overlay_rect((0, 0, 20, 30));
        assert!(o2.2 >= 1 && o2.3 >= 1);
    }

    #[test]
    fn display_name_rules() {
        assert_eq!(display_name(Path::new(r"C:\")), r"C:\");
        assert_eq!(display_name(Path::new(r"C:")), r"C:\");
        assert_eq!(display_name(Path::new(r"C:\Users\bob")), "bob");
    }

    #[test]
    fn ancestor_chain_deep_start() {
        let ch = ancestor_chain(Path::new(r"C:\a\b"));
        assert_eq!(ch[0], PathBuf::from(r"C:\"));
        assert_eq!(ch.len(), 3, r"C: + a + b");
    }

    #[test]
    fn state_icon_click_id_wraps_correctly() {
        assert_eq!(TVN_ITEMSTATEICONCLICKW, 4294966896 - 68, "SDK: TVN_FIRST-68");
        assert_eq!(TVIS_STATEIMAGEMASK.0, 0xF000);
    }

    #[test]
    fn ancestor_chain_root_to_leaf() {
        let ch = ancestor_chain(Path::new(r"C:\a\b"));
        assert_eq!(ch[0], PathBuf::from(r"C:\"), "盘符根带尾分隔符");
        assert_eq!(ch[1], PathBuf::from(r"C:\a"));
        assert_eq!(*ch.last().unwrap(), PathBuf::from(r"C:\a\b"));
    }
}
