//! 主窗口：四窗格布局（布局体验对标 Q-Dir；外观自主，见 PRODUCT_SPEC）。
//!
//! 结构基准（历史证据：examples/control_tree.rs 对原版的 dump）：
//! - 全局：`ReBarWindow32`（2 个 ToolbarWindow32 + 菜单）+ 全局 `msctls_statusbar32`
//!   （5 段：N 个对象 / 版本 / 用户 (x64) / 驱动器名 / 空闲空间）
//! - 每窗格：内容容器 + 分割器容器 + 窗格状态栏；地址栏 Edit、3 个工具栏、
//!   对象计数、驱动器 ComboBox、命令按钮排、Tab 页签、SysListView32。
//!
//! 分割条逻辑按 REA 反编译的 0x42a3bc 规格：万分比存储
//! `MulDiv(pos, 10000, clientWidth - splitterWidth)`，与 .qdr 的 SP0/SP1/SP2 对应。

use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW,
    GetVolumeInformationW,
};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::brand;
use crate::core::{ColumnDef, Config, FileItem, PaneId, PaneState, SessionState, sort_items};
use crate::shell::{EnumOptions, FileItemExt, IconCache, IconSize, list_directory_detailed};
use crate::ui::pane::format_filetime;
use crate::ui_native::splitter::{SPLITTER_WIDTH, SplitterGroup};
use crate::ui_native::tree::{self, TreeNodes};
use crate::core::TreeMode;

/// LVS_REPORT(0x0001) | LVS_SHOWSELALWAYS(0x0008)
const LVS_REPORT_SHOWSEL: u32 = 0x0001 | 0x0008;

/// 主窗口类名（自有，见 PRODUCT_SPEC R1）。
pub const FRAME_CLASS_NAME: &str = brand::WINDOW_CLASS;

/// 子窗口命令 ID 段（0..999 给系统内部，窗格从 1000 起）。
const ID_FIRST_PANE: i32 = 1000;

// ---- 每窗格子控件 ID（相对段：pane_base + 以下值）----
const CID_ADDR_EDIT: i32 = 1;
const CID_OBJ_SPLIT: i32 = 2;
const CID_FILTER_EDIT: i32 = 3;
const CID_OPEN_BTN: i32 = 4;
const CID_TAB: i32 = 5;
const CID_LIST: i32 = 6;
const CID_COMBO_DRIVES: i32 = 7;
const CID_STATUS: i32 = 8;
/// 窗格目录树（SysTreeView32）。
const CID_TREE: i32 = 9;

// ---- 全局子控件 ID ----
const CID_REBAR: i32 = 90;
const CID_MAIN_STATUS: i32 = 91;

/// 命令按钮排：名称与快捷键（原版 UIA dump 的 Button 序列）。
const CMD_BUTTONS: &[&str] = &[
    "到上一个选中项",
    "到下一个选中项",
    "运行",
    "通过电子邮件发送",
    "删除 [Del]",
    "剪切 [Ctrl+X]",
    "复制 [Ctrl+C]",
    "粘贴 [Ctrl+V]",
    "过滤器或高亮 [Ctrl+M]",
    "快速链接菜单 [Ctrl+Q]\n 鼠标右击选择在哪个窗口中打开",
];

/// 原版详情视图 4 列（UIA dump：名称/修改日期/类型/大小）。
const COLUMN_TITLES: &[&str] = &["名称", "修改日期", "类型", "大小"];
const COLUMN_WIDTHS: &[i32] = &[200, 150, 120, 90];

/// 过滤框读写探针只跑一次。
static FILTER_PROBE_ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 树交互探针只跑一次。
static TREE_PROBE_DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 窗格容器类名（自有；原 ATL 私有类名属原版资产，不再复用——R1）。
const PANE_CLASS: PCWSTR = w!("LDir.Pane");

/// 是否已注册窗格容器类（进程内一次）。
static PANE_CLASS_REGISTERED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

/// 注册窗格容器窗口类（原版 ATL 容器的等价类：普通 Pane）。
unsafe fn ensure_pane_class(hinstance: HINSTANCE) {
    if PANE_CLASS_REGISTERED.get().is_some() {
        return;
    }
    unsafe extern "system" fn pane_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if msg == WM_NOTIFY {
            // ATL 容器把通知沿父链上送主框架处理。
            if let Ok(parent) = unsafe { GetParent(hwnd) } {
                if !parent.is_invalid() {
                    return unsafe { SendMessageW(parent, WM_NOTIFY, Some(wp), Some(lp)) };
                }
            }
        }
        if msg == WM_COMMAND {
            // 上送命令按钮排的 BN_CLICKED（id=base+20..29）与过滤框 EN_CHANGE
            // （id%100==3, 通知码 768）：不转发则默认 DefWindowProc 吞掉。
            // addr Edit 的其余 EN_* 维持不上送，避免噪声。
            let id = (wp.0 & 0xFFFF) as u32;
            let notify = ((wp.0 >> 16) & 0xFFFF) as u32;
            if (1000..1400).contains(&id) && (id - 1000) % 100 == CID_FILTER_EDIT as u32 && notify == EN_CHANGE {
                if let Ok(parent) = unsafe { GetParent(hwnd) } {
                    if !parent.is_invalid() {
                        return unsafe { SendMessageW(parent, msg, Some(wp), Some(lp)) };
                    }
                }
            }
            if crate::ui_native::commands::pane_button_id(id).is_some() {
                if let Ok(parent) = unsafe { GetParent(hwnd) } {
                    if !parent.is_invalid() {
                        return unsafe { SendMessageW(parent, msg, Some(wp), Some(lp)) };
                    }
                }
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
    }
    let wc = WNDCLASSW {
        lpfnWndProc: Some(pane_proc),
        hInstance: hinstance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
        hbrBackground: HBRUSH(((COLOR_WINDOW.0 as i32) + 1) as usize as *mut core::ffi::c_void),
        lpszClassName: PANE_CLASS,
        ..Default::default()
    };
    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 {
    } else {
        let _ = PANE_CLASS_REGISTERED.set(());
    }
}

/// 通知来源控件角色。
#[derive(PartialEq, Eq)]
enum Src {
    List,
    Tab,
    Tree,
}

/// Rgb(R,G,B) → Win32 COLORREF（0x00BBGGRR）。
pub fn rgb_to_colorref(c: crate::core::Rgb) -> u32 {
    ((c.2 as u32) << 16) | ((c.1 as u32) << 8) | (c.0 as u32)
}

/// 列头（0名称/1修改日期/2类型/3大小）点击 → 新排序键与方向。
/// 同列翻转方向，异列新列为升序（纯函数，可单测）。
pub fn column_sort_click(
    cur_key: crate::core::SortKey,
    cur_dir: crate::core::SortDir,
    col: usize,
) -> Option<(crate::core::SortKey, crate::core::SortDir)> {
    use crate::core::{SortDir, SortKey};
    let key = match col {
        0 => SortKey::Name,
        1 => SortKey::Modified,
        2 => SortKey::Type,
        3 => SortKey::Size,
        _ => return None,
    };
    let dir = if key == cur_key {
        match cur_dir {
            SortDir::Asc => SortDir::Desc,
            SortDir::Desc => SortDir::Asc,
        }
    } else {
        SortDir::Asc
    };
    Some((key, dir))
}

/// 双击语义决策：选中目录→进入；选中当前目录自身（列表首行 ".."）→ 上级；
/// 文件/无选中→ None（调用方走默认打开）。纯函数，可单测。
pub fn dir_of_selection(
    sel: Option<&std::path::Path>,
    cur: &std::path::Path,
) -> Option<std::path::PathBuf> {
    match sel {
        Some(p) if p == cur => cur.parent().map(|x| x.to_path_buf()),
        Some(p) if p.is_dir() => Some(p.to_path_buf()),
        _ => None,
    }
}

/// 由 WM_CREATE 载入的帧载荷。
///
/// WM_CREATE 在 CreateWindowExW **返回之前**触发，配置只能经 lpCreateParams 送入。
pub struct PendingFrame {
    payload: Option<(Config, SessionState, HINSTANCE)>,
}

impl PendingFrame {
    pub fn new(config: Config, session: SessionState, hinstance: HINSTANCE) -> Self {
        PendingFrame {
            payload: Some((config, session, hinstance)),
        }
    }

    /// 取出载荷；重复调用返回 None。
    pub fn take(&mut self) -> Option<(Config, SessionState, HINSTANCE)> {
        self.payload.take()
    }
}

/// 每窗格的子窗口句柄集合。
#[derive(Default)]
struct PaneWindows {
    container: Option<HWND>,
    /// d5 内容容器（命令按钮排/过滤/页签/列表的父级）。
    content: Option<HWND>,
    /// chrome 层 3 个工具栏（原版每窗格 d5 ToolBar x3）。
    toolbars: [Option<HWND>; 3],
    /// 对象计数文本（原版 d5 Text「N 个对象」）。
    count_text: Option<HWND>,
    /// d6 列表容器（SysListView32 的父级；原版每 TabItem 一个 List）。
    list_host: Option<HWND>,
    addr_edit: Option<HWND>,
    /// 命令按钮排（10 个）。
    cmd_buttons: Vec<HWND>,
    obj_split: Option<HWND>,
    filter_edit: Option<HWND>,
    open_btn: Option<HWND>,
    tab: Option<HWND>,
    status: Option<HWND>,
    list: Option<HWND>,
    /// 窗格目录树（SysTreeView32，tree_mode 非 0 时可见；附录 A.3）。
    tree: Option<HWND>,
    /// 树节点登记（HTREEITEM→路径）+ 回环抑制标志（busy）。
    tree_nodes: TreeNodes,
    drive_combo: Option<HWND>,
    /// 列表行号 → 完整路径（populate 时写入；命令分发按选中行取路径）。
    row_paths: Vec<std::path::PathBuf>,
    /// 列表行号 → Filter2 前景色（None=默认色）；populate 预算，custom-draw 只读。
    row_colors: Vec<Option<crate::core::Rgb>>,
    /// 列表行号 → 交替背景色（#BG/#BG-A）；同理预算。
    row_bg_colors: Vec<Option<crate::core::Rgb>>,
}

/// 主窗口状态（挂在 GWLP_USERDATA）。
pub struct MainFrame {
    pub config: Config,
    pub session: SessionState,
    /// 四窗格子窗口。
    pub panes: [PaneWindows; 4],
    /// 横向分割组（三条竖直分割条切四窗格）。
    pub splitters: SplitterGroup,
    /// 菜单句柄。
    pub menu: HMENU,
    /// 顶部 ReBar。
    pub rebar: Option<HWND>,
    /// 全局状态栏（5 段）。
    pub status: Option<HWND>,
    /// 系统图标缓存。
    pub icons: IconCache,
    /// 当前 UI 字体（缩放档命令重建；初值 = 系统默认 GUI 字体）。
    pub ui_font: HFONT,
    /// 当前缩放百分比（60..200）。
    pub ui_scale: u32,
    /// 窗格是否已灌过数据（避免重复枚举）。
    panes_filled: [bool; 4],
    /// 窗格过滤框文本（EN_CHANGE 更新；fill_pane 应用为快速筛名）。
    pane_filter: [String; 4],
    /// 系统小图标 ImageList 里的文件夹图标索引（首建树时解析一次）。
    folder_image: i32,
}

impl MainFrame {
    pub fn new(config: Config, session: SessionState, _hinstance: HINSTANCE) -> Self {
        MainFrame {
            config,
            session,
            panes: Default::default(),
            splitters: SplitterGroup::horizontal(4),
            menu: HMENU::default(),
            rebar: None,
            status: None,
            icons: IconCache::new(),
            ui_font: HFONT::default(),
            ui_scale: 100,
            panes_filled: [false; 4],
            pane_filter: Default::default(),
            folder_image: -1,
        }
    }

    /// WndProc 分发。
    pub(crate) unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => Self::on_create(hwnd, lp),
            WM_SIZE => Self::on_size(hwnd, wp, lp),
            WM_PAINT => Self::on_paint(hwnd),
            WM_COMMAND => Self::on_command(hwnd, wp, lp),
            WM_NOTIFY => {
                if let Some(frame) = Self::frame_of(hwnd) {
                    frame.on_notify(hwnd, lp)
                } else {
                    LRESULT(0)
                }
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }

    unsafe fn on_create(hwnd: HWND, lp: LPARAM) -> LRESULT {
        // 从 CREATESTRUCT.lpCreateParams 取回真实配置（取一次即空）。
        let mut taken: Option<(Config, SessionState, HINSTANCE)> = None;
        let mut hmenu = HMENU::default();
        if lp.0 != 0 {
            let cs = &*(lp.0 as *const CREATESTRUCTW);
            hmenu = cs.hMenu;
            if !cs.lpCreateParams.is_null() {
                let pending = &mut *(cs.lpCreateParams as *mut PendingFrame);
                taken = pending.take();
            }
        }
        let (config, session, hinstance) = taken
            .unwrap_or_else(|| (Config::default(), SessionState::default(), HINSTANCE::default()));

        let frame = Box::leak(Box::new(MainFrame::new(config, session, hinstance)));
        frame.menu = hmenu;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, frame as *mut _ as isize);
        unsafe { crate::ui_native::subclass::set_main_hwnd(hwnd) };

        // 原版窗格容器是进程私有 ATL 类；先注册等价类再建窗格。
        unsafe { ensure_pane_class(hinstance) };

        // 1. 全局 ReBar（原版 d1 ReBarWindow32，承载 2 个工具栏波段）。
        frame.rebar = create_child(
            hwnd,
            hinstance,
            REBARCLASSNAME,
            w!(""),
            WS_VISIBLE | WS_CHILD | WS_CLIPSIBLINGS | WINDOW_STYLE(RBS_VARHEIGHT | CCS_NODIVIDER as u32 | CCS_TOP as u32),
            CID_REBAR,
        ).ok();
        unsafe { setup_rebar(frame, hinstance) };

        // 2. 全局状态栏（原版 d1 msctls_statusbar32，5 段）。
        frame.status = create_child(
            hwnd,
            hinstance,
            STATUSCLASSNAME,
            w!(""),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(SBARS_SIZEGRIP),
            CID_MAIN_STATUS,
        ).ok();
        if let Some(sb) = frame.status {
            // 边界用 -1 伸缩（UIA 显示 5 段文本）。
            let parts: [i32; 4] = [140, 260, 420, 560];
            unsafe {
                SendMessageW(
                    sb,
                    SB_SETPARTS,
                    Some(WPARAM(parts.len())),
                    Some(LPARAM(parts.as_ptr() as isize)),
                )
            };
            set_status_text(sb, 0, &object_count_text(&frame));
            set_status_text(sb, 1, brand::PRODUCT_VERSION);
            set_status_text(sb, 2, &format!("{} (x64)", std::env::var("USERNAME").unwrap_or_default()));
        }

        // 3. 四个窗格：容器 + 状态栏 + 内容控件。
        for i in 0..4 {
            create_pane(frame, hwnd, hinstance, i);
        }

        // 端到端验证钩子：QDIR_TEST_DBLCLK_ROW 设置时对窗格1第 N 行
        // 模拟双击（同进程安全：直接走 list_dblclk，不经真实鼠标）。
        if let Ok(s) = std::env::var("QDIR_TEST_DBLCLK_ROW") {
            if let Ok(row) = s.trim().parse::<i32>() {
                unsafe {
                    if let (Some(list), Some(rp)) = (
                        frame.panes[0].list,
                        frame.panes[0].row_paths.get(row as usize).cloned(),
                    ) {
                        // 选中该行，然后走与 NM_DBLCLK 相同的处理器。
                        select_row_if_valid(list, row);
                        { let p = rp;
                            let cur = frame.panes[0]
                                .container
                                .map(|_| frame.session.panes[0].tabs[frame.session.panes[0].tabs_pos].path.clone())
                                .unwrap_or_default();
                            let tabs_pos = frame.session.panes[0].tabs_pos;
                            if let Some(dir) = dir_of_selection(Some(p.as_path()), &cur) {
                                let tab = &mut frame.session.panes[0].tabs[tabs_pos];
                                tab.history.truncate(tab.history_pos + 1);
                                tab.history.push(dir.clone());
                                tab.history_pos = tab.history.len() - 1;
                                tab.path = dir;
                                println!("[qdir-rs] TEST 双击第 {row} 行 → 进入 {:?}", frame.session.panes[0].tabs[tabs_pos].path);
                                frame.fill_pane(0);
                                // 再双击首行（回到父）验证返回路径
                                if std::env::var("QDIR_TEST_DBLCLK_BACK").is_ok() {
                                    let cur2 = frame.session.panes[0].tabs[tabs_pos].path.clone();
                                    if let Some(parent) = cur2.parent() {
                                        let tab = &mut frame.session.panes[0].tabs[tabs_pos];
                                        tab.path = parent.to_path_buf();
                                        tab.history_pos = tab.history_pos.saturating_sub(1);
                                        println!("[qdir-rs] TEST 返回 → {:?}", tab.path);
                                    }
                                }
                            } else {
                                println!("[qdir-rs] TEST 第 {row} 行非目录/不可导航: {p:?}");
                            }
                        }
                    }
                }
            }
        }

        // 4. 按会话恢复分割条比例（千分比 → 内部万分比）。
        frame.splitters.from_permille_ten(&frame.session.splits_h);

        LRESULT(0)
    }

    pub unsafe fn frame_of(hwnd: HWND) -> Option<&'static mut MainFrame> {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        (ptr != 0).then(|| &mut *(ptr as *mut MainFrame))
    }

    unsafe fn on_size(hwnd: HWND, _wp: WPARAM, _lp: LPARAM) -> LRESULT {
        let Some(frame) = Self::frame_of(hwnd) else {
            return LRESULT(0);
        };
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd, &mut rc);
        let width = (rc.right - rc.left).max(0);
        let height = (rc.bottom - rc.top).max(0);
        if width == 0 || height == 0 {
            return LRESULT(0);
        }

        // 顶部 ReBar：波段在 ReBar 收到 WM_SIZE 后才布局，两轮法——
        // 先拉一个初始高度，再读回真实带高，最后按真实带高定型。
        if let Some(rb) = frame.rebar {
            let _ = SetWindowPos(
                rb, None, 0, 0, width, 56,
                SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0),
            );
        }
        let rebar_h = rebar_height(frame.rebar);
        let status_h = statusbar_height(frame.status);

        if let Some(rb) = frame.rebar {
            // ReBar 报告的高度在尺寸为 0 时无效：用波段高度下限兜底，
            // 拉伸后 ReBar 会自动调整内部波段。
            let h = rebar_h.max(28);
            let _ = SetWindowPos(
                rb, None,
                0, 0, width, h,
                SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0),
            );
        }
        if let Some(sb) = frame.status {
            let _ = SetWindowPos(
                sb, None,
                0, height - status_h, width, status_h,
                SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0),
            );
        }

        // 窗格区：ReBar 之下、状态栏之上。
        let pane_top = rebar_h;
        let pane_h = (height - rebar_h - status_h).max(0);
        let rects = frame.splitters.rects(width, pane_h);
        for (i, r) in rects.0.iter().enumerate() {
            if let Some(c) = frame.panes[i].container {
                let _ = SetWindowPos(
                    c, None,
                    r.0, r.1 + pane_top, r.2, r.3,
                    SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0),
                );
            }
            layout_pane_children(frame, i, *r);
        }
        LRESULT(0)
    }

    unsafe fn on_paint(hwnd: HWND) -> LRESULT {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc.is_invalid() {
            return LRESULT(0);
        }
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd, &mut rc);
        let brush = HBRUSH(((COLOR_WINDOW.0 as i32) + 1) as usize as *mut core::ffi::c_void);
        FillRect(hdc, &rc, brush);
        let _ = EndPaint(hwnd, &ps);
        LRESULT(0)
    }

    unsafe fn on_command(hwnd: HWND, wp: WPARAM, lp: LPARAM) -> LRESULT {
        let id = (wp.0 & 0xFFFF) as u32;
        let notify = ((wp.0 >> 16) & 0xFFFF) as u32;
        // 窗格过滤框 EN_CHANGE（id=pane_base+3，768）：读文本、重灌该窗格。
        if (1000..1400).contains(&id)
            && (id - 1000) % 100 == CID_FILTER_EDIT as u32
            && notify == EN_CHANGE
        {
            let idx = ((id - 1000) / 100) as usize;
            if let Some(frame) = Self::frame_of(hwnd) {
                // 优先读 lParam 发件人句柄（Edit 自发 EN_CHANGE 时=控件自身）；
                // 外部投递 lParam=0 时回落 stored。
                let sender = HWND(lp.0 as *mut core::ffi::c_void);
                let target = if !sender.is_invalid() {
                    sender
                } else {
                    let Some(e) = frame.panes.get(idx).and_then(|p| p.filter_edit) else {
                        return LRESULT(0);
                    };
                    e
                };
                let mut buf = [0u16; 260];
                let n = GetWindowTextW(target, &mut buf);
                let end = (n.max(0) as usize).min(buf.len());
                frame.pane_filter[idx] = String::from_utf16_lossy(&buf[..end]);
                println!("[qdir-rs] TEST 过滤 窗格{idx} q={:?}", frame.pane_filter[idx]);
                frame.fill_pane(idx);
            }
            return LRESULT(0);
        }
        // 全部菜单命令走分发模块（自绘 id 段 30000+，见 commands/menu）。
        if let Some(frame) = Self::frame_of(hwnd) {
            let menu = frame.menu;
            if !crate::ui_native::commands::dispatch(frame, hwnd, id) {
                // 未识别的 id 留痕；从活的菜单反查文案，方便定位。
                let hint = crate::ui_native::menu::label_of(menu, id)
                    .map(|s| format!("（菜单文案：{s}）"))
                    .unwrap_or_default();
                eprintln!("[l-dir] 未接命令 id={id}{hint}");
            }
        }
        LRESULT(0)
    }

    /// 应用配色档（`menu::theme_palette` 自设计表，R3）到四窗格的列表底色/
    /// 前景与树底色/前景；档 0（默认）用 CLR_DEFAULT 复位系统色。
    /// 换档后重灌四窗格（行色预算里的 Filter2 命中色保持不变——
    /// 前景覆盖仅对**未命中规则**的行生效，custom-draw 已按此优先级写回）。
    pub unsafe fn apply_theme_palette(&mut self, main: HWND, theme: u32) {
        // CLR_DEFAULT(-1) = 恢复系统色；LVM_/TVM_ 颜色消息 wParam=0, lParam=COLORREF。
        let (bg, fg): (i32, i32) = match crate::ui_native::menu::theme_palette(theme) {
            None => (CLR_DEFAULT, CLR_DEFAULT),
            Some((b, f)) => (
                rgb_to_colorref(b) as i32,
                f.map(|c| rgb_to_colorref(c) as i32).unwrap_or(CLR_DEFAULT),
            ),
        };
        let mut applied = 0usize;
        for p in self.panes.iter() {
            if let Some(lv) = p.list {
                let _ = SendMessageW(lv, LVM_SETBKCOLOR, Some(WPARAM(0)), Some(LPARAM(bg as isize)));
                let _ = SendMessageW(lv, LVM_SETTEXTCOLOR, Some(WPARAM(0)), Some(LPARAM(fg as isize)));
                applied += 1;
            }
            // 树只有激活时才可见/存在节点；颜色设置无害，统一刷。
            if let Some(tv) = p.tree {
                let _ = SendMessageW(tv, TVM_SETBKCOLOR, Some(WPARAM(0)), Some(LPARAM(bg as isize)));
                let _ = SendMessageW(tv, TVM_SETTEXTCOLOR, Some(WPARAM(0)), Some(LPARAM(fg as isize)));
            }
        }
        println!(
            "[qdir-rs] TEST 配色档位 colorthema={theme} 应用窗格={applied} bg={bg:#x} fg={fg:#x}"
        );
        for i in 0..4 {
            self.fill_pane(i);
        }
        let _ = RedrawWindow(
            Some(main),
            None,
            None,
            RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN,
        );
    }

    /// 某个窗格当前选中行的路径集合（ListView 实时查询）。
    pub fn selected_paths(&self, index: usize) -> Vec<std::path::PathBuf> {
        let Some(list) = self.panes[index].list else {
            return Vec::new();
        };
        let mut out = Vec::new();
        unsafe {
            // 从 -1 起逐段找选中项；返回值是下一命中行号，-1 = 结束。
            let mut idx = SendMessageW(
                list,
                LVM_GETNEXTITEM,
                Some(WPARAM(usize::MAX)), // -1：从头搜索
                Some(LPARAM(LVNI_SELECTED as isize)),
            )
            .0 as i32;
            while idx >= 0 {
                if let Some(p) = self.panes[index].row_paths.get(idx as usize) {
                    out.push(p.clone());
                }
                idx = SendMessageW(
                    list,
                    LVM_GETNEXTITEM,
                    Some(WPARAM(idx as usize)),
                    Some(LPARAM(LVNI_SELECTED as isize)),
                )
                .0 as i32;
            }
        }
        out
    }

    /// 某个窗格活动标签页的当前目录。
    pub fn pane_path(&self, index: usize) -> std::path::PathBuf {
        let pane = &self.session.panes[index];
        pane.tabs
            .get(pane.tabs_pos)
            .map(|t| t.path.clone())
            .unwrap_or_default()
    }

    /// 聚焦某个窗格的列表（命令按钮作用域=该窗格，而非活动焦点窗格）。
    pub fn focus_pane_list(&self, index: usize) {
        if let Some(list) = self.panes.get(index).and_then(|p| p.list) {
            unsafe {
                let _ = SetFocus(Some(list));
            }
        }
    }

    /// 聚焦某个窗格的过滤输入框（命令「过滤器或高亮」）。
    pub fn focus_filter_edit(&self, index: usize) {
        if let Some(edit) = self.panes.get(index).and_then(|p| p.filter_edit) {
            unsafe {
                let _ = SetFocus(Some(edit));
            }
        }
    }

    /// 设置活动窗格（视图菜单「活动窗口 # 1..# 4」，id 32833..32836）。
    pub fn set_active_pane(&mut self, index: usize) -> bool {
        match PaneId::from_number(index as u32 + 1) {
            Some(id) => {
                self.session.focus = id;
                true
            }
            None => false,
        }
    }

    /// 快速链接菜单（命令按钮 bi=9「快速链接菜单 [Ctrl+Q]」）。
    ///
    /// 扫描 `Link=.\Favoriten\Quick-Link\`（原版 ini [Favoriten] 键，硬
    /// 证据）里的 .lnk，逐个读目标路径，在按钮下方弹 TrackPopupMenu；
    /// 选中即导航该窗格。用 TPM_RETURNCMD 同步取值（不产生 WM_COMMAND，
    /// 避免重入 dispatch）。*注：原版还支持子文件夹与右键选择目标窗格
    /// （tooltip 第二行），本实现先做一级 .lnk 列表，其余待样本复核*。
    pub fn show_quick_link_menu(
        &mut self,
        main: HWND,
        index: usize,
        quick_link_dir: std::path::PathBuf,
    ) {
        let mut items: Vec<(String, std::path::PathBuf)> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&quick_link_dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("lnk") {
                    continue;
                }
                let label = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if let Ok(target) = crate::shell::read_shortcut_target(&p) {
                    items.push((label, target));
                }
            }
        }
        items.sort();
        // 弹层是阻塞式的（TPM_RETURNCMD）：先把发现数落日志，外部探针
        // 即使在弹层阶段杀掉进程也能确认扫描结果。
        println!(
            "[qdir-rs] TEST 快速链接菜单 窗格{}：发现 {} 项",
            index + 1,
            items.len()
        );
        if items.is_empty() {
            return;
        }
        unsafe {
            let menu = match CreatePopupMenu() {
                Ok(m) => m,
                Err(_) => return,
            };
            // 菜单项 id 段：QL 基准 + 序号（TPM_RETURNCMD 直接回传该 id）。
            const QL_BASE: usize = 0x5A00;
            for (i, (label, _)) in items.iter().enumerate() {
                let wide: Vec<u16> =
                    label.encode_utf16().chain(std::iter::once(0)).collect();
                let _ = AppendMenuW(
                    menu,
                    MF_STRING,
                    QL_BASE + i,
                    windows::core::PCWSTR::from_raw(wide.as_ptr()),
                );
            }
            // 锚点：该窗格第 9 号按钮下沿；拿不到就退到主窗原点。
            let (ax, ay) = match self.panes[index].cmd_buttons.get(9) {
                Some(b) => {
                    let mut rc = RECT::default();
                    let _ = GetWindowRect(*b, &mut rc);
                    (rc.left, rc.bottom)
                }
                None => {
                    let mut rc = RECT::default();
                    let _ = GetWindowRect(main, &mut rc);
                    (rc.left, rc.top + 40)
                }
            };
            let _ = SetForegroundWindow(main);
            let cmd = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                ax,
                ay,
                None,
                main,
                None,
            );
            let _ = DestroyMenu(menu);
            if cmd.0 > 0 {
                if let Some(i) = (cmd.0 as usize).checked_sub(QL_BASE) {
                    if let Some((label, target)) = items.get(i) {
                        println!(
                            "[qdir-rs] TEST 快速链接 窗格{} 选择 {label:?} → 导航",
                            index + 1
                        );
                        self.navigate_to(index, target.clone());
                    }
                }
            } else {
                println!("[qdir-rs] TEST 快速链接菜单 窗格{} 取消", index + 1);
            }
        }
    }

    /// 全部窗格列表切换查看模式（LVM_SETVIEW，comctl v6）。
    /// id 33931 大图标 / 33932 小图标 / 33933 列表 / 33934 详细信息
    /// （德语菜单模板 #216 字面文本 &Große Symbole/&Kleine Symbole/
    /// &Liste/&Details 为硬证据；33931/33932 与 LV_VIEW_ICON/SMALLICON
    /// 的对应按语义推断，*待复核*）。
    pub fn set_view_mode_all(&self, mode: u32) {
        for p in &self.panes {
            if let Some(lv) = p.list {
                unsafe {
                    let _ = SendMessageW(lv, LVM_SETVIEW, Some(WPARAM(mode as usize)), None);
                }
            }
        }
    }

    /// 切换网格线（id 32864 &Linien anzeigen；读-改-写扩展样式）。
    pub fn toggle_grid_lines_all(&self) {
        for p in &self.panes {
            if let Some(lv) = p.list {
                unsafe {
                    let st = SendMessageW(lv, LVM_GETEXTENDEDLISTVIEWSTYLE, None, None).0 as u32;
                    let new = st ^ LVS_EX_GRIDLINES;
                    let _ = SendMessageW(
                        lv,
                        LVM_SETEXTENDEDLISTVIEWSTYLE,
                        Some(WPARAM(LVS_EX_GRIDLINES as usize)),
                        Some(LPARAM(new as isize)),
                    );
                }
            }
        }
    }

    /// 列宽自适应（id 33938 含列头 LVSCW_AUTOSIZE_USEHEADER /
    /// 33939 不含 LVSCW_AUTOSIZE；德语模板字面文本硬证据）。
    pub fn autofit_columns_all(&self, with_header: bool) {
        for p in &self.panes {
            if let Some(lv) = p.list {
                unsafe {
                    for col in 0..4 {
                        let w = if with_header {
                            LVSCW_AUTOSIZE_USEHEADER
                        } else {
                            LVSCW_AUTOSIZE
                        };
                        let _ = SendMessageW(
                            lv,
                            LVM_SETCOLUMNWIDTH,
                            Some(WPARAM(col as usize)),
                            Some(LPARAM(w as isize)),
                        );
                    }
                }
            }
        }
    }

    /// 窗格内选择跳转（命令按钮「到上一个/下一个选中项」）。
    ///
    /// 从当前焦点行出发，沿 `forward` 方向找下一个选中行；找到则把焦点移到
    /// 该行并确保可见（不改动整体选中集合）。语义按按钮 tooltip 字面直译，
    /// *待原版行为复核*。
    pub fn move_selection_to_next(&self, index: usize, forward: bool) {
        let Some(list) = self.panes.get(index).and_then(|p| p.list) else {
            return;
        };
        unsafe {
            // 当前焦点行（-1 = 无）。
            let cur = SendMessageW(
                list,
                LVM_GETNEXTITEM,
                Some(WPARAM(usize::MAX)),
                Some(LPARAM(LVNI_FOCUSED as isize)),
            )
            .0 as i32;
            let dir = if forward { 0 } else { LVNI_PREVIOUS };
            // 从焦点行向方向找下一个选中行；无焦点行时从边界起找。
            let start = if cur >= 0 { cur } else { if forward { -1 } else { 0 } };
            let found = SendMessageW(
                list,
                LVM_GETNEXTITEM,
                Some(WPARAM(start.max(-1) as usize)),
                Some(LPARAM((LVNI_SELECTED | dir) as isize)),
            )
            .0 as i32;
            if found < 0 {
                return;
            }
            // 仅移动焦点，保持选中集合不变。
            let st = LVITEMW {
                mask: LVIF_STATE,
                state: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_FOCUSED.0),
                stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_FOCUSED.0),
                ..Default::default()
            };
            let _ = SendMessageW(
                list,
                LVM_SETITEMSTATE,
                Some(WPARAM(found as usize)),
                Some(LPARAM(&st as *const LVITEMW as isize)),
            );
            let _ = SendMessageW(
                list,
                LVM_ENSUREVISIBLE,
                Some(WPARAM(found as usize)),
                Some(LPARAM(FALSE.0 as isize)),
            );
            let _ = SetFocus(Some(list));
        }
    }

    /// 重灌某个窗格（枚举 + 填充列表）。
    pub(crate) unsafe fn fill_pane(&mut self, index: usize) {
        let Some(hwnd) = self.panes[index].list else {
            return;
        };
        let pane = &self.session.panes[index];
        let Some(tab) = pane.tabs.get(pane.tabs_pos) else {
            return;
        };
        let path = tab.path.clone();
        let (sort_key, sort_dir) = (tab.sort_key, tab.sort_dir);

        let opts = EnumOptions {
            include_hidden: true, // 隐藏项按 #Hidden 规则在视图层着色，不在枚举层丢弃
            want_icon_index: true,
            want_display_info: true,
        };
        match list_directory_detailed(&path, &opts) {
            Ok(mut entries) => {
                // sort_items 只接受 &mut [FileItem]；排副本后按 path 对齐重排。
                let mut keys: Vec<FileItem> = entries.iter().map(|(i, _)| i.clone()).collect();
                sort_items(&mut keys, sort_key, sort_dir);
                let mut sorted: Vec<(FileItem, FileItemExt)> = Vec::with_capacity(keys.len());
                for k in keys {
                    if let Some(pos) = entries.iter().position(|(i, _)| i.path == k.path) {
                        sorted.push(entries.remove(pos));
                    }
                }
                // 快速过滤（Explorer 搜索框惯例，*原版行为待复核*）：
                // 按窗格过滤框文本做大小写无关「包含」筛选。
                let q = self.pane_filter[index].clone();
                let sorted: Vec<(FileItem, FileItemExt)> = if q.trim().is_empty() {
                    sorted
                } else {
                    let ql = q.trim().to_lowercase();
                    sorted
                        .into_iter()
                        .filter(|(i, _)| i.name.to_lowercase().contains(&ql))
                        .collect()
                };
                unsafe { populate_list(hwnd, &mut self.icons, &sorted) };
                unsafe { populate_pane_chrome(self, index, &path, sorted.len()) };
                // 行序→路径映射：命令分发按选中行号取路径。
                self.panes[index].row_paths =
                    sorted.iter().map(|(i, _)| i.path.clone()).collect();
                // 预计算每行前景色（Filter2 第一条命中规则；纯决策已单测）。
                self.panes[index].row_colors = sorted
                    .iter()
                    .map(|(i, _)| self.config.rules.color_for(i))
                    .collect();
                // 交替背景（#BG 偶数行 / #BG-A 奇数行；未启用为 None）。
                self.panes[index].row_bg_colors = (0..sorted.len())
                    .map(|r| self.config.rules.bg_variant(r))
                    .collect();
                // 验证钩子：QDIR_TEST_LOG_COLORS 时打印命中行色（custom-draw 预算核对）。
                if std::env::var("QDIR_TEST_LOG_COLORS").is_ok() {
                    let hits: Vec<(usize, String)> = self.panes[index]
                        .row_colors
                        .iter()
                        .enumerate()
                        .filter_map(|(r, c)| c.map(|rgb| (r, format!("{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2))))
                        .collect();
                    println!("[qdir-rs] TEST 窗格{} 着色行 {} 条：{hits:?}", index + 1, hits.len());
                }
                // 树联动：首次需要时懒建（on_size 可能晚于首轮 WM_CREATE 填充），
                // 再把树选中项定位到当前目录（tree_auto_expand_from_list）。
                if self.session.tree_mode.is_active() && self.panes[index].tree_nodes.path_of.is_empty() {
                    self.build_tree(index);
                }
                self.tree_sync_to_path(index);
                // 验证钩子：QDIR_TEST_FILTER=文本 → 同进程写入窗格0过滤框并读回，
                // 断言 GetWindowTextW 同进程语义（跨进程探针无法区分读写方）。
                if index == 0 && std::env::var("QDIR_TEST_FILTER").is_ok()
                    && FILTER_PROBE_ONCE
                        .compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
                        .is_ok()
                {
                    let q = std::env::var("QDIR_TEST_FILTER").unwrap();
                    if let Some(e) = self.panes[0].filter_edit {
                        let wz: Vec<u16> = q.encode_utf16().chain(std::iter::once(0)).collect();
                        let set_ok = unsafe { SetWindowTextW(e, PCWSTR::from_raw(wz.as_ptr())) };
                        let mut back = [0u16; 64];
                        let n = unsafe { GetWindowTextW(e, &mut back) };
                        println!(
                            "[qdir-rs] TEST 过滤探针 set={:?} readback n={} text={:?}",
                            set_ok,
                            n,
                            String::from_utf16_lossy(&back[..n.max(0).min(64) as usize])
                        );
                    }
                }
                // 验证钩子：QDIR_TEST_TREE_SELECT=1 → 展开首个盘根并选中其第一个
                // 子目录，走完整 TVN_SELCHANGED→navigate 链（同进程安全）。
                if std::env::var("QDIR_TEST_TREE_SELECT").is_ok()
                    && index == 0
                    && TREE_PROBE_DONE
                        .compare_exchange(false, true, std::sync::atomic::Ordering::SeqCst, std::sync::atomic::Ordering::SeqCst)
                        .is_ok()
                {
                    // 一次性触发：探针自身会 navigate_to → fill_pane，防止无限递归。
                    self.tree_probe_select(index);
                }
                self.panes_filled[index] = true;
                unsafe { self.paint_sort_glyphs(index) };
                // 验证钩子：QDIR_TEST_SELECT_ROW 设置时选中该行（外部命令测试用，同进程安全）。
                if let Ok(s) = std::env::var("QDIR_TEST_SELECT_ROW") {
                    if let Ok(row) = s.trim().parse::<i32>() {
                        unsafe { select_row_if_valid(hwnd, row) };
                    }
                }
            }
            Err(e) => eprintln!("[qdir-rs] 枚举 {path:?} 失败：{e}"),
        }
    }
    /// 设置树模式（附录 A.3 确证 0/1/2）并重排所有窗格。
    /// 0→非 0 时把四个窗格的 `tree_visible` 置真（viewtree 按钮观感：树全开）；
    /// 非 0→0 保留窗格位（回到原状态保存值无意义，保持当前勾选，下次开机恢复）。
    pub unsafe fn set_tree_mode(&mut self, main: HWND, mode: TreeMode) {
        if mode.is_active() {
            let any = self.session.panes.iter().any(|p| p.tree_visible);
            if !any {
                for i in 0..4 {
                    self.session.panes[i].tree_visible = true;
                }
            }
        }
        self.session.tree_mode = mode;
        if mode.is_active() {
            for i in 0..4 {
                self.build_tree(i);
            }
        }
        // 重排（SWP_FRAMECHANGED 不产 WM_SIZE，显式补发——与 set_layout 同法）。
        let _ = SetWindowPos(
            main,
            None,
            0,
            0,
            0,
            0,
            SET_WINDOW_POS_FLAGS(
                SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOZORDER.0 | SWP_FRAMECHANGED.0,
            ),
        );
        SendMessageW(main, WM_SIZE, Some(WPARAM(SIZE_RESTORED as usize)), None);
        println!("[qdir-rs] TEST tree_mode={}", mode.to_u32());
    }

    /// 首次需要时建树：盘符根并列 + 文件夹图标 + 当前目录复选框勾选态。
    pub fn build_tree(&mut self, index: usize) {
        let Some(tv) = self.panes[index].tree else { return };
        if !self.panes[index].tree_nodes.path_of.is_empty() {
            return; // 已建过：保留展开态
        }
        if self.folder_image < 0 {
            self.folder_image = unsafe { folder_icon_index() };
        }
        let img = self.folder_image;
        unsafe {
            // 清空再灌盘符根。
            let _ = SendMessageW(tv, TVM_DELETEITEM, Some(WPARAM(0)), Some(LPARAM(0)));
            self.panes[index].tree_nodes.clear();
            for d in tree::drive_roots() {
                tree::insert_dir_node(tv, tree::tvi_root(), &d, img, &mut self.panes[index].tree_nodes);
            }
            // TreeN 位 → 根行复选框（勾选=image 2）。
            if self.session.panes[index].tree_visible {
                let mut root = tree::tree_next(tv, TVGN_ROOT, HTREEITEM(0));
                while root.0 != 0 {
                    tree::set_state_image(tv, root, 2);
                    root = tree::tree_next(tv, TVGN_NEXT, root);
                }
            }
        }
    }

    /// 列表导航后把树定位到当前目录（`tree_auto_expand_from_list` 语义，busy 抑制回环）。
    pub fn tree_sync_to_path(&mut self, index: usize) {
        let Some(tv) = self.panes[index].tree else { return };
        if !self.session.tree_mode.is_active() {
            return;
        }
        let path = self.pane_path(index);
        if path.as_os_str().is_empty() {
            return;
        }
        if self.folder_image < 0 {
            self.folder_image = unsafe { folder_icon_index() };
        }
        let img = self.folder_image;
        self.panes[index].tree_nodes.busy = true;
        let found = unsafe { tree::reveal(tv, &path, &mut self.panes[index].tree_nodes, img) };
        self.panes[index].tree_nodes.busy = false;
        if !found {
            // 盘符不可达（网络盘掉线等）：只留痕不报错。
            println!("[qdir-rs] TEST 树定位失败 {path:?}");
        }
    }

    /// 钩子：展开树首个盘根、选中第一个子目录（验证 懒加载→选中→导航 闭环）。
    fn tree_probe_select(&mut self, index: usize) {
        let Some(tv) = self.panes[index].tree else { return };
        let img = self.folder_image;
        let root = unsafe { tree::tree_next(tv, TVGN_ROOT, tree::tvi_root()) };
        if root.0 == 0 {
            println!("[qdir-rs] TEST 树探针：无盘根");
            return;
        }
        unsafe { tree::set_expand(tv, root, true) };
        if !self.panes[index].tree_nodes.is_loaded(root) {
            let Some(rdir) = self.panes[index].tree_nodes.path(root).map(|x| x.to_path_buf()) else {
                return;
            };
            unsafe { tree::load_child_dirs(tv, root, &rdir, img, &mut self.panes[index].tree_nodes) };
        }
        let child = unsafe { tree::tree_next(tv, TVGN_CHILD, root) };
        if child.0 == 0 {
            println!("[qdir-rs] TEST 树探针：盘根无子目录");
            return;
        }
        // busy 抑制 TVN_SELCHANGEDW 回环（fill_pane 里会再次 sync），
        // 然后显式走与通知处理器完全相同的导航链，验证 懒加载→选中→导航 闭环。
        let Some(p) = self.panes[index].tree_nodes.path(child).map(|x| x.to_path_buf()) else {
            return;
        };
        self.panes[index].tree_nodes.busy = true;
        unsafe { tree::select_node(tv, child) };
        self.panes[index].tree_nodes.busy = false;
        unsafe { self.navigate_to(index, p.clone()) };
        println!("[qdir-rs] TEST 树探针选中导航 -> {p:?}");
    }

    /// 通知路由：子控件（list/tab）→ 主窗处理。
    unsafe fn on_notify(&mut self, main: HWND, lp: LPARAM) -> LRESULT {
        if lp.0 == 0 {
            return LRESULT(0);
        }
        let hdr = unsafe { &*(lp.0 as *const NMHDR) };
        // 自定义绘制（列表行着色）：不属于 list/tab 路由，直接处理。
        if hdr.code == NM_CUSTOMDRAW {
            return self.on_custom_draw(hdr.hwndFrom, lp);
        }
        // 找该 HWND 所属窗格
        let Some((pidx, role)) = self.find_notify_source(hdr.hwndFrom) else {
            return unsafe { DefWindowProcW(main, WM_NOTIFY, WPARAM(0), lp) };
        };
        match hdr.code {
            c if c == NM_DBLCLK && role == Src::List => {
                self.open_selection(pidx);
                LRESULT(0)
            }
            c if c == NM_CLICK && role == Src::List => {
                // 原版：点击窗格即激活它。
                if let Some(id) = PaneId::from_number(pidx as u32 + 1) {
                    if self.session.focus != id {
                        self.session.focus = id;
                    }
                }
                LRESULT(0)
            }
            c if c == TCN_SELCHANGE && role == Src::Tab => {
                let pos = unsafe {
                    SendMessageW(hdr.hwndFrom, TCM_GETCURSEL, None, None).0.max(0) as usize
                };
                self.tab_select(pidx, pos);
                LRESULT(0)
            }
            c if role == Src::Tree && c == tree::TVN_ITEMEXPANDEDW => {
                // 懒加载：首次展开枚举直接子目录（附录 A.3 树的形态）。
                let Some(tv) = self.panes[pidx].tree else { return LRESULT(0) };
                let nm = unsafe { &*(lp.0 as *const NMTREEVIEWW) };
                let h = nm.itemNew.hItem;
                let expanding = nm.action == tree::TVE_EXPAND;
                let img = self.folder_image;
                let pending = if expanding && !self.panes[pidx].tree_nodes.is_loaded(h) {
                    self.panes[pidx].tree_nodes.path(h).map(|x| x.to_path_buf())
                } else {
                    None
                };
                if let Some(dir) = pending {
                    tree::load_child_dirs(tv, h, &dir, img, &mut self.panes[pidx].tree_nodes);
                }
                LRESULT(0)
            }
            c if role == Src::Tree && c == tree::TVN_SELCHANGEDW => {
                if self.panes[pidx].tree_nodes.busy {
                    return LRESULT(0);
                }
                let Some(tv) = self.panes[pidx].tree else { return LRESULT(0) };
                let h = tree::tree_next(tv, TVGN_CARET, HTREEITEM(0));
                let Some(dir) = self.panes[pidx].tree_nodes.path(h).map(|x| x.to_path_buf()) else {
                    return LRESULT(0);
                };
                if dir.is_dir() {
                    self.navigate_to(pidx, dir);
                }
                LRESULT(0)
            }
            c if role == Src::Tree && c == tree::TVN_ITEMSTATEICONCLICKW => {
                // 树根复选框 = 窗格级 TreeN 位（自主解释：勾选=本窗格跟随子目录，
                // *行为待复核*）；此处记录状态并把所有盘根勾选态同步。
                let bit = !self.session.panes[pidx].tree_visible;
                self.session.panes[pidx].tree_visible = bit;
                if let Some(tv) = self.panes[pidx].tree {
                    let img = if bit { 2u32 } else { 1u32 };
                    let mut root = unsafe { tree::tree_next(tv, TVGN_ROOT, HTREEITEM(0)) };
                    while root.0 != 0 {
                        unsafe { tree::set_state_image(tv, root, img) };
                        root = unsafe { tree::tree_next(tv, TVGN_NEXT, root) };
                    }
                }
                println!("[qdir-rs] TEST 树窗格{} TreeN 位={}", pidx + 1, bit as u32);
                LRESULT(0)
            }
            _ => LRESULT(0),
        }
    }

    /// NM_CUSTOMDRAW：按预算的 row_colors 给命中行着色（返回给控件绘制）。
    unsafe fn on_custom_draw(&mut self, hwnd_from: HWND, lp: LPARAM) -> LRESULT {
        // 定位所属窗格（list）。
        let pidx = match self.panes.iter().position(|p| p.list == Some(hwnd_from)) {
            Some(i) => i,
            None => return LRESULT(CDRF_DODEFAULT as isize),
        };
        if lp.0 == 0 {
            return LRESULT(CDRF_DODEFAULT as isize);
        }
        let cd = unsafe { &mut *(lp.0 as *mut NMLVCUSTOMDRAW) };
        match cd.nmcd.dwDrawStage {
            CDDS_PREPAINT => LRESULT(CDRF_NOTIFYITEMDRAW as isize),
            stage if stage == CDDS_ITEMPREPAINT => {
                let row = cd.nmcd.dwItemSpec;
                let color = self
                    .panes[pidx]
                    .row_colors
                    .get(row)
                    .copied()
                    .flatten();
                let bg = self.panes[pidx].row_bg_colors.get(row).copied().flatten();
                match color {
                    Some(rgb) => {
                        cd.clrText = COLORREF(rgb_to_colorref(rgb));
                        if let Some(b) = bg {
                            cd.clrTextBk = COLORREF(rgb_to_colorref(b));
                        }
                        LRESULT(CDRF_NEWFONT as isize)
                    }
                    None => match bg {
                        Some(b) => {
                            cd.clrTextBk = COLORREF(rgb_to_colorref(b));
                            LRESULT(CDRF_NEWFONT as isize)
                        }
                        None => LRESULT(CDRF_DODEFAULT as isize),
                    },
                }
            }
            _ => LRESULT(CDRF_DODEFAULT as isize),
        }
    }

    fn find_notify_source(&self, hwnd: HWND) -> Option<(usize, Src)> {
        for (i, p) in self.panes.iter().enumerate() {
            if p.list == Some(hwnd) {
                return Some((i, Src::List));
            }
            if p.tab == Some(hwnd) {
                return Some((i, Src::Tab));
            }
            if p.tree == Some(hwnd) {
                return Some((i, Src::Tree));
            }
        }
        None
    }

    /// 双击/回车激活选中项：目录进入，文件走默认程序。
    pub unsafe fn open_selection(&mut self, pidx: usize) {
        let sel = self.selected_paths(pidx);
        let (cur, tabs_pos) = {
            let pane = &self.session.panes[pidx];
            (pane.tabs[pane.tabs_pos].path.clone(), pane.tabs_pos)
        };
        match dir_of_selection(sel.first().map(|x| x.as_path()), &cur) {
            Some(dir) => self.navigate_to(pidx, dir),
            None => {
                if let Some(path) = sel.first() {
                    let _ = crate::shell::open_default(
                        crate::shell::HostWindow::from_raw_hwnd(0),
                        path,
                    );
                }
            }
        }
    }

    /// 导航到目录：历史截断+压栈，更新标签页路径，重灌窗格。
    pub fn navigate_to(&mut self, pidx: usize, dir: std::path::PathBuf) {
        let tabs_pos = self.session.panes[pidx].tabs_pos;
        {
            let tab = &mut self.session.panes[pidx].tabs[tabs_pos];
            tab.history.truncate(tab.history_pos + 1);
            tab.history.push(dir.clone());
            tab.history_pos = tab.history.len() - 1;
            tab.path = dir;
        }
        unsafe { self.fill_pane(pidx) };
    }

    /// 返回上级（Backspace，原版语义）；卷根不动。
    pub fn go_parent(&mut self, pidx: usize) {
        let cur = self.session.panes[pidx].tabs[self.session.panes[pidx].tabs_pos]
            .path
            .clone();
        // 历史回退优先（原版 Backspace = 后退一步）：history_pos>0 则退栈。
        let tabs_pos = self.session.panes[pidx].tabs_pos;
        let hp = self.session.panes[pidx].tabs[tabs_pos].history_pos;
        if hp > 0 {
            let prev = self.session.panes[pidx].tabs[tabs_pos].history[hp - 1].clone();
            self.session.panes[pidx].tabs[tabs_pos].history_pos = hp - 1;
            self.session.panes[pidx].tabs[tabs_pos].path = prev;
            unsafe { self.fill_pane(pidx) };
            return;
        }
        if let Some(parent) = cur.parent() {
            self.navigate_to(pidx, parent.to_path_buf());
        }
    }

    /// 地址栏回车：读文本，是目录则导航。
    pub unsafe fn addr_enter(&mut self, pidx: usize) {
        let Some(e) = self.panes[pidx].addr_edit else { return };
        let mut buf = [0u16; 512];
        let n = unsafe { GetWindowTextW(e, &mut buf) };
        if n <= 0 {
            return;
        }
        let text = String::from_utf16_lossy(&buf[..n as usize]);
        let path = std::path::PathBuf::from(text.trim());
        if path.is_dir() {
            self.navigate_to(pidx, path);
        }
    }

    /// 列头点击 → 更新排序并重排、重灌。`col` 为视图列序（0..3）。
    pub unsafe fn apply_column_sort(&mut self, pidx: usize, col: usize) {
        let tabs_pos = self.session.panes[pidx].tabs_pos;
        let (ck, cd) = {
            let t = &self.session.panes[pidx].tabs[tabs_pos];
            (t.sort_key, t.sort_dir)
        };
        if let Some((k, d)) = column_sort_click(ck, cd, col) {
            let t = &mut self.session.panes[pidx].tabs[tabs_pos];
            t.sort_key = k;
            t.sort_dir = d;
            self.fill_pane(pidx);
        }
    }

    /// 重灌后按当前排序在列头挂 HDF_SORTUP/DOWN 箭头。
    unsafe fn paint_sort_glyphs(&self, pidx: usize) {
        let Some(lv) = self.panes[pidx].list else { return };
        let header = SendMessageW(lv, LVM_GETHEADER, None, None);
        if header.0 == 0 {
            return;
        }
        let hdr = HWND(header.0 as *mut core::ffi::c_void);
        let tabs_pos = self.session.panes[pidx].tabs_pos;
        let key = self.session.panes[pidx].tabs[tabs_pos].sort_key;
        let dir = self.session.panes[pidx].tabs[tabs_pos].sort_dir;
        // 视图列序→SortKey 反查：0名称 1修改 2类型 3大小。
        let active_col: Option<usize> = match key {
            crate::core::SortKey::Name => Some(0),
            crate::core::SortKey::Modified => Some(1),
            crate::core::SortKey::Type => Some(2),
            crate::core::SortKey::Size => Some(3),
            crate::core::SortKey::Ext => None,
        };
        for col in 0..4usize {
            let mut it = HDITEMW::default();
            it.mask = HDI_FORMAT;
            if SendMessageW(
                hdr,
                windows::Win32::UI::Controls::HDM_GETITEMW,
                Some(WPARAM(col)),
                Some(LPARAM(&mut it as *mut HDITEMW as isize)),
            )
            .0 == 0
            {
                continue;
            }
            // 清掉旧箭头再按需加。
            it.fmt = HEADER_CONTROL_FORMAT_FLAGS(it.fmt.0 & !(HDF_SORTUP.0 | HDF_SORTDOWN.0));
            if active_col == Some(col) {
                let glyph = match dir {
                    crate::core::SortDir::Asc => HDF_SORTUP,
                    crate::core::SortDir::Desc => HDF_SORTDOWN,
                };
                it.fmt = HEADER_CONTROL_FORMAT_FLAGS(it.fmt.0 | glyph.0);
            }
            let _ = SendMessageW(
                hdr,
                windows::Win32::UI::Controls::HDM_SETITEMW,
                Some(WPARAM(col)),
                Some(LPARAM(&mut it as *mut HDITEMW as isize)),
            );
        }
    }

    /// 页签切换：更新 tabs_pos 并重灌窗格（标题跟随）。
    unsafe fn tab_select(&mut self, pidx: usize, pos: usize) {
        let pane = &self.session.panes[pidx];
        if pos >= pane.tabs.len() || pos == pane.tabs_pos {
            return;
        }
        self.session.panes[pidx].tabs_pos = pos;
        self.fill_pane(pidx);
    }

}

#[cfg(test)]
mod colorref_tests {
    use super::rgb_to_colorref;
    use crate::core::Rgb;

    #[test]
    fn rgb_maps_to_bgr_colorref() {
        // Rgb(r,g,b) → 0x00BBGGRR。
        assert_eq!(rgb_to_colorref(Rgb(0xAA, 0x00, 0x00)), 0x000000AA);
        assert_eq!(rgb_to_colorref(Rgb(0x00, 0x00, 0xDD)), 0x00DD0000);
    }
}

// ---------- ReBar / 工具栏 ----------

/// ReBar 波段布局：2 个 ToolbarWindow32（原版 UIA dump 结构）。
unsafe fn setup_rebar(frame: &mut MainFrame, hinstance: HINSTANCE) {
    let Some(rb) = frame.rebar else { return };
    // REBARINFO（无图像列表）。
    let mut info = REBARINFO {
        cbSize: core::mem::size_of::<REBARINFO>() as u32,
        fMask: 0,
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            rb, RB_SETBARINFO, None,
            Some(LPARAM(&mut info as *mut REBARINFO as isize)),
        )
    };

    // 波段 0：原版主导航（FUN_00429bc4 的 48x48 PNG 按钮 8 个 + go 下拉）。
    if let Ok((tb0, _himl)) = crate::ui_native::toolbar::build_nav_toolbar(
        rb,
        hinstance,
        CID_REBAR + 10,
    ) {
        let mut band = REBARBANDINFOW {
            cbSize: core::mem::size_of::<REBARBANDINFOW>() as u32,
            fMask: RBBIM_STYLE | RBBIM_CHILD | RBBIM_CHILDSIZE | RBBIM_SIZE,
            fStyle: RBBS_CHILDEDGE | RBBS_GRIPPERALWAYS,
            hwndChild: tb0,
            cxMinChild: 60,
            cyMinChild: 54,
            cyChild: 54,
            cyMaxChild: 54,
            cx: 420,
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                rb,
                RB_INSERTBANDW,
                Some(WPARAM(usize::MAX)),
                Some(LPARAM(&mut band as *mut REBARBANDINFOW as isize)),
            )
        };    }

    // 波段 1：布局切换占位（真实按钮待补 PNG 组）。
    for i in 1..2 {
        let tb = create_child(
            rb,
            hinstance,
            TOOLBARCLASSNAME,
            w!(""),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(TBSTYLE_FLAT as u32 | CCS_NODIVIDER as u32),
            CID_REBAR + 10 + i as i32,
        );
        let Ok(tb) = tb else { continue };
        // TB_BUTTONSTRUCTSIZE 必须在建任何按钮前设置。
        unsafe {
            SendMessageW(
                tb, TB_BUTTONSTRUCTSIZE,
                Some(WPARAM(core::mem::size_of::<TBBUTTON>())),
                None,
            )
        };        // 占位按钮：分隔符（真实按钮/图标后续接命令表时填充）。
        let btn = TBBUTTON {
            iBitmap: 0,
            idCommand: 0,
            fsState: TBSTATE_ENABLED as u8,
            fsStyle: BTNS_SEP as u8,
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                tb, TB_ADDBUTTONSW,
                Some(WPARAM(1)),
                Some(LPARAM(&btn as *const TBBUTTON as isize)),
            )
        };
        // 波段。
        let mut band = REBARBANDINFOW {
            cbSize: core::mem::size_of::<REBARBANDINFOW>() as u32,
            fMask: RBBIM_STYLE | RBBIM_CHILD | RBBIM_CHILDSIZE | RBBIM_SIZE,
            fStyle: RBBS_CHILDEDGE | RBBS_USECHEVRON,
            hwndChild: tb,
            cxMinChild: 60,
            cyMinChild: 26,
            cyChild: 26,
            cyMaxChild: 26,
            cx: if i == 0 { 320 } else { 220 },
            ..Default::default()
        };
        unsafe {
            SendMessageW(
                rb, RB_INSERTBANDW,
                Some(WPARAM(usize::MAX)), // 追加到末尾
                Some(LPARAM(&mut band as *mut REBARBANDINFOW as isize)),
            )
        };    }
}

// ---------- 窗格构建 ----------

    /// 构建一个窗格的全部子控件（原版每窗格 d4..d7 结构）。
unsafe fn create_pane(frame: &mut MainFrame, parent: HWND, hinstance: HINSTANCE, index: usize) {
    let base = ID_FIRST_PANE + (index as i32) * 100;

    // 窗格容器（视觉上就是列表区外框）。
    let container = create_child(
        parent,
        hinstance,
        PANE_CLASS,
        w!(""),
        WS_VISIBLE | WS_CHILD | WS_CLIPSIBLINGS,
        base,
    )
    .ok();
    frame.panes[index].container = container;
    let Some(container) = container else { return };

    // 内容容器（d5）：承载命令按钮排/过滤/页签/列表。
    let content = create_child(
        container,
        hinstance,
        PANE_CLASS,
        w!(""),
        WS_VISIBLE | WS_CHILD | WS_CLIPSIBLINGS,
        base + 50,
    )
    .ok();
    frame.panes[index].content = content;
    let content = content.unwrap_or(container);

    // 列表容器（d6）：SysListView32 的父级。
    let list_host = create_child(
        content,
        hinstance,
        PANE_CLASS,
        w!(""),
        WS_VISIBLE | WS_CHILD | WS_CLIPSIBLINGS,
        base + 51,
    )
    .ok();
    frame.panes[index].list_host = list_host;
    let list_host = list_host.unwrap_or(content);

    // 地址栏 Edit。
    frame.panes[index].addr_edit = create_child(
        container,
        hinstance,
        w!("Edit"),
        w!(""),
        WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
        base + CID_ADDR_EDIT,
    )
    .ok();

    // chrome 工具栏 x3（原版每窗格 3 个 ToolbarWindow32；按钮后续接命令表填充）。
    for ti in 0..3 {
        let tb = create_child(
            container,
            hinstance,
            TOOLBARCLASSNAME,
            w!(""),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(TBSTYLE_FLAT as u32 | CCS_NODIVIDER as u32),
            base + 60 + ti as i32,
        );
        if let Ok(tb) = tb {
            unsafe {
                SendMessageW(
                    tb,
                    TB_BUTTONSTRUCTSIZE,
                    Some(WPARAM(core::mem::size_of::<TBBUTTON>())),
                    None,
                )
            };
            frame.panes[index].toolbars[ti] = Some(tb);
        }
    }

    // 对象计数文本（原版 d5 Text「N 个对象」）。
    frame.panes[index].count_text = create_child(
        container,
        hinstance,
        w!("Static"),
        w!(""),
        WS_VISIBLE | WS_CHILD,
        base + 64,
    )
    .ok();

    // 命令按钮排（10 个 Button）。
    for (bi, label) in CMD_BUTTONS.iter().enumerate() {
        let wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
        let b = create_child(
            content,
            hinstance,
            w!("Button"),
            PCWSTR::from_raw(wide.as_ptr()),
            WS_VISIBLE | WS_CHILD,
            base + 20 + bi as i32,
        );
        if let Ok(b) = b {
            frame.panes[index].cmd_buttons.push(b);
        }
    }

    // 对象计数 SplitButton（原版 d6 SplitButton「N 个对象」）。
    frame.panes[index].obj_split = create_child(
        content,
        hinstance,
        w!("Button"),
        w!(""),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_SPLITBUTTON as u32),
        base + CID_OBJ_SPLIT,
    )
    .ok();

    // 过滤 Edit + 打开 Button。
    frame.panes[index].filter_edit = create_child(
        content,
        hinstance,
        w!("Edit"),
        w!(""),
        WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
        base + CID_FILTER_EDIT,
    )
    .ok();
    frame.panes[index].open_btn = create_child(
        content,
        hinstance,
        w!("Button"),
        w!("打开"),
        WS_VISIBLE | WS_CHILD,
        base + CID_OPEN_BTN,
    )
    .ok();

    // 驱动器 ComboBox（原版 d5 ComboBox）。
    frame.panes[index].drive_combo = create_child(
        container,
        hinstance,
        w!("ComboBox"),
        w!(""),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        base + CID_COMBO_DRIVES,
    )
    .ok();
    fill_drive_combo(frame.panes[index].drive_combo);

    // Tab 页签（原版 d6 SysTabControl32）。
    frame.panes[index].tab = create_child(
        content,
        hinstance,
        WC_TABCONTROL,
        w!(""),
        WS_VISIBLE | WS_CHILD | WS_CLIPSIBLINGS,
        base + CID_TAB,
    )
    .ok();
    fill_pane_tabs(frame, index);

    // 窗格状态栏（原版每窗格一个 msctls_statusbar32）。
    frame.panes[index].status = create_child(
        container,
        hinstance,
        STATUSCLASSNAME,
        w!(""),
        WS_VISIBLE | WS_CHILD,
        base + CID_STATUS,
    )
    .ok();

    // 文件列表（核心）。
    frame.panes[index].list = create_child(
        list_host,
        hinstance,
        w!("SysListView32"),
        w!("文件夹视图"),
        WS_VISIBLE
            | WS_CHILD
            | WS_CLIPSIBLINGS
            | WINDOW_STYLE(LVS_REPORT_SHOWSEL),
        base + CID_LIST,
    )
    .ok();
    // 窗格目录树（tree_mode 非 0 时由 layout_pane_children 定位显示）。
    frame.panes[index].tree = create_child(
        content,
        hinstance,
        WC_TREEVIEW,
        w!(""),
        WINDOW_STYLE(
            WS_VISIBLE.0 | WS_CHILD.0 | WS_CLIPSIBLINGS.0
                | (TVS_HASLINES | TVS_LINESATROOT | TVS_HASBUTTONS | TVS_SHOWSELALWAYS | TVS_CHECKBOXES),
        ),
        base + CID_TREE,
    )
    .ok();
    if let Some(tv) = frame.panes[index].tree {
        // 图像列表：系统小图标（TVSIL_NORMAL）+ 状态复选框（TVSIL_STATE 由 comctl 内建）。
        if let Some(hilm) = unsafe { system_small_imagelist() } {
            unsafe {
                SendMessageW(tv, TVM_SETIMAGELIST, Some(WPARAM(TVSIL_NORMAL as usize)), Some(LPARAM(hilm.0)));
            }
        }
    }
    if let Some(lv) = frame.panes[index].list {
        // LVS_EX_FULLROWSELECT(0x20) + LVS_EX_DOUBLEBUFFER(0x10000)。
        unsafe {
            SendMessageW(
                lv,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                Some(WPARAM(0usize)),
                Some(LPARAM(0x20 | 0x10000)),
            );
        }
        // 键盘导航子类化（Enter 打开/Backspace 返回/地址栏回车）。
        unsafe {
            crate::ui_native::subclass::install(frame.panes[index].list, frame.panes[index].addr_edit, index)
        };
        // 首次填充（建列头 + 灌行）。
        unsafe { frame.fill_pane(index) };
    }
}

/// Tab 页签标题 = 窗格各标签页 title（原版 dump：TabItem 数 == tabs 数）。
unsafe fn fill_pane_tabs(frame: &mut MainFrame, index: usize) {
    let Some(tab_ctl) = frame.panes[index].tab else { return };
    let pane = &frame.session.panes[index];
    for (ti, t) in pane.tabs.iter().enumerate() {
        let mut text: Vec<u16> = t.title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut it = TCITEMW {
            mask: TCIF_TEXT,
            ..Default::default()
        };
        it.pszText = windows::core::PWSTR::from_raw(text.as_mut_ptr());
        unsafe {
            SendMessageW(
                tab_ctl,
                TCM_INSERTITEMW,
                Some(WPARAM(ti)),
                Some(LPARAM(&mut it as *mut TCITEMW as isize)),
            );
        }
    }
    let _ = TCIF_TEXT; // keep import
}

/// 驱动器 ComboBox：枚举本地盘符，文本 = 卷标 (C:)（原版观感）。
unsafe fn fill_drive_combo(combo: Option<HWND>) {
    let Some(cb) = combo else { return };
    let mut buf = [0u16; 512];
    let n = unsafe { GetLogicalDriveStringsW(Some(&mut buf)) };
    let mut drives: Vec<String> = Vec::new();
    let mut s: &[u16] = &buf[..n as usize];
    while !s.is_empty() {
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        if end == 0 {
            break;
        }
        drives.push(String::from_utf16_lossy(&s[..end]));
        s = &s[end + 1..];
    }
    for d in drives {
        let mut text = format!("{} ", d);
        let mut vol = [0u16; 260];
        let dwide: Vec<u16> = d.encode_utf16().chain(std::iter::once(0)).collect();
        let ok = unsafe {
            GetVolumeInformationW(
                PCWSTR::from_raw(dwide.as_ptr()),
                Some(&mut vol),
                None,
                None,
                None,
                None,
            )
        }
        .is_ok();
        if ok {
            let v: String = vol
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| char::from_u32(c as u32).unwrap_or('\u{fffd}'))
                .collect();
            if !v.is_empty() {
                text = format!("{v} ({})", &d[..2]);
            } else {
                text = format!("本地磁盘 ({})", &d[..2]);
            }
        } else {
            // 无卷标/不可访问的驱动器（光驱等）按类型命名。
            let dt = unsafe { GetDriveTypeW(PCWSTR::from_raw(dwide.as_ptr())) };
            text = match dt {
                5 => format!("CD 驱动器 ({})", &d[..2]),
                _ => format!("可移动磁盘 ({})", &d[..2]),
            };
        }
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            SendMessageW(
                cb,
                CB_ADDSTRING,
                None,
                Some(LPARAM(wide.as_ptr() as isize)),
            );
        }
    }
    unsafe { SendMessageW(cb, CB_SETCURSEL, Some(WPARAM(0)), None) };
}

// ---------- 列表填充 ----------

/// 清空并重建某个列表控件的列与行。
unsafe fn populate_list(
    hwnd: HWND,
    icons: &mut IconCache,
    entries: &[(FileItem, FileItemExt)],
) {
    unsafe { SendMessageW(hwnd, LVM_DELETEALLITEMS, None, None) };

    // 列（原版 4 列：名称/修改日期/类型/大小）。
    // LVCFMT_SPLITBUTTON（=HDF_SPLITBUTTON 0x1000000）：comctl32 v6 下让
    // 列头渲染/暴露为下拉拆分按钮，与原版 UIA 树（HeaderItem→SplitButton）
    // 对齐；HDF_SORTUP/DOWN 由排序状态另行维护（*排序箭头接线待补*）。
    for (i, title) in COLUMN_TITLES.iter().enumerate() {
        let mut text: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut lvcol = LVCOLUMNW {
            mask: LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM | LVCF_FMT,
            fmt: LVCFMT_SPLITBUTTON,
            cx: COLUMN_WIDTHS[i],
            iSubItem: i as i32,
            ..Default::default()
        };
        lvcol.pszText = windows::core::PWSTR::from_raw(text.as_mut_ptr());
        unsafe {
            SendMessageW(
                hwnd,
                LVM_INSERTCOLUMNW,
                Some(WPARAM(i)),
                Some(LPARAM(&mut lvcol as *mut LVCOLUMNW as isize)),
            );
        }
    }

    // 行。
    for (row, (item, ext)) in entries.iter().enumerate() {
        let img = match item.icon_index {
            Some(i) => icons
                .image_for_index(i, IconSize::Small)
                .map(|_| i)
                .unwrap_or(-1),
            None => -1,
        };

        let mut name: Vec<u16> = item.name.encode_utf16().chain(std::iter::once(0)).collect();
        let mut lvi = LVITEMW {
            mask: LVIF_TEXT | LVIF_IMAGE | LVIF_PARAM,
            iItem: row as i32,
            iImage: img,
            lParam: LPARAM(row as isize),
            ..Default::default()
        };
        lvi.pszText = windows::core::PWSTR::from_raw(name.as_mut_ptr());
        unsafe {
            SendMessageW(
                hwnd,
                LVM_INSERTITEMW,
                None,
                Some(LPARAM(&mut lvi as *mut LVITEMW as isize)),
            );
        }

        // 子列：修改日期 / 类型 / 大小。
        let cells: [String; 3] = [
            format_filetime(item.modified),
            ext.type_name.clone().unwrap_or_else(|| {
                if item.is_dir {
                    "文件夹".to_string()
                } else {
                    format!("{} 文件", item.ext_lower().to_uppercase())
                }
            }),
            if item.is_dir {
                String::new()
            } else {
                format_size(item.size)
            },
        ];
        for (si, cell) in cells.iter().enumerate() {
            let mut ctext: Vec<u16> = cell.encode_utf16().chain(std::iter::once(0)).collect();
            let mut sub = LVITEMW {
                mask: LVIF_TEXT,
                iItem: row as i32,
                iSubItem: si as i32 + 1,
                ..Default::default()
            };
            sub.pszText = windows::core::PWSTR::from_raw(ctext.as_mut_ptr());
            unsafe {
                SendMessageW(
                    hwnd,
                    LVM_SETITEMW,
                    None,
                    Some(LPARAM(&mut sub as *mut LVITEMW as isize)),
                );
            }
        }
    }
}

/// 窗格周边（chrome）文本更新：地址栏、对象计数、窗格状态栏。
unsafe fn populate_pane_chrome(frame: &mut MainFrame, index: usize, path: &std::path::Path, count: usize) {
    let p = &frame.panes[index];
    if let Some(e) = p.addr_edit {
        let wide: Vec<u16> = path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { SetWindowTextW(e, PCWSTR::from_raw(wide.as_ptr())) };
    }
    if let Some(sp) = p.obj_split {
        let wide: Vec<u16> = format!("{count} 个对象")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe { SetWindowTextW(sp, PCWSTR::from_raw(wide.as_ptr())) };
    }
    if let Some(ct) = p.count_text {
        let wide: Vec<u16> = format!("{count} 个对象")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe { SetWindowTextW(ct, PCWSTR::from_raw(wide.as_ptr())) };
    }
    if let Some(sb) = p.status {
        set_status_text(sb, 0, &format!("{count} 个对象"));
    }
    // 全局状态栏第 0 段 = 活动窗格对象数（原版行为）。
    if let Some(sb) = frame.status {
        set_status_text(sb, 0, &format!("{count} 个对象"));
        // 第 3/4 段：当前盘符与剩余空间。
        if let Some((label, free)) = drive_summary(path) {
            set_status_text(sb, 3, &label);
            set_status_text(sb, 4, &free);
        }
    }
}

// ---------- 窗格内部布局 ----------

/// 地址栏/工具条/按钮排/过滤行的固定行高（像素；缩放档只重设字体，
/// 行高对齐原版 chrome 近似值，*精确高度待截图级对照*）。
const ROW_ADDR: i32 = 22;
const ROW_TOOLBAR: i32 = 26;
const ROW_TAB: i32 = 24;
const ROW_BUTTON: i32 = 24;
const ROW_FILTER: i32 = 24;

/// 窗格内部几何：addr → 工具条排 → content（tab/按钮排/过滤行/树|列表）→ 状态栏。
/// 树按 tree_mode 几何（附录 A.3：1=双列左栏，2=覆盖浮层）；窗格 `TreeN` 位
/// 关闭时该窗格不显示树（panes::tree_visible 同规则）。
fn layout_pane_children(frame: &mut MainFrame, index: usize, r: (i32, i32, i32, i32)) {
    let (x, y, w, h) = r;
    if w < 40 || h < 60 {
        return;
    }
    // 懒建：tree_mode 开且尚未登记任何节点（.qdr 恢复 / 首次开启）→ 建盘根并定位。
    if frame.session.tree_mode.is_active() && frame.panes[index].tree_nodes.path_of.is_empty() {
        frame.build_tree(index);
        frame.tree_sync_to_path(index);
    }
    let p = &frame.panes[index];
    let status_h = unsafe { statusbar_height(p.status) }.max(18);
    let swp = SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0 | SWP_NOACTIVATE.0);
    // container 子窗：坐标 = 容器原点 + 相对偏移。
    let put = |hwnd: Option<HWND>, cx: i32, cy: i32, cw: i32, ch: i32| {
        if let Some(hq) = hwnd {
            let _ = unsafe { SetWindowPos(hq, None, x + cx, y + cy, cw.max(1), ch.max(1), swp) };
        }
    };
    // content 子窗：坐标 = content 原点（相对 content 客户区）。
    let put_c = |hwnd: Option<HWND>, cx: i32, cy: i32, cw: i32, ch: i32| {
        if let Some(hq) = hwnd {
            let _ = unsafe { SetWindowPos(hq, None, cx, cy, cw.max(1), ch.max(1), swp) };
        }
    };

    // 1. 地址栏（整行）。
    put(p.addr_edit, 0, 0, w, ROW_ADDR);
    // 2. 工具条排 x3 + 计数 + 驱动器下拉。
    let tb_w = 60;
    put(p.toolbars[0], 0, ROW_ADDR, tb_w, ROW_TOOLBAR);
    put(p.toolbars[1], tb_w + 2, ROW_ADDR, tb_w, ROW_TOOLBAR);
    put(p.toolbars[2], 2 * (tb_w + 2), ROW_ADDR, tb_w, ROW_TOOLBAR);
    put(p.drive_combo, w - 150, ROW_ADDR, 150, ROW_TOOLBAR);
    put(p.count_text, 3 * (tb_w + 2), ROW_ADDR, (w - 150 - 3 * (tb_w + 2)).max(1), ROW_TOOLBAR);

    // 3. content：工具条排下、状态栏上。
    let content_top = ROW_ADDR + ROW_TOOLBAR;
    let content_h = (h - content_top - status_h).max(40);
    put(p.content, 0, content_top, w, content_h);
    // content 内部坐标从 0 起；cy0 保留给容器坐标（按钮排绝对 y 用）。
    let (cx0, cy0, cw, _) = (0, content_top, w, content_h);
    let ccy0 = 0i32;

    // 3.1 页签。
    put_c(p.tab, 0, ccy0, cw, ROW_TAB);
    // 3.2 命令按钮排：两行五列。
    let n = p.cmd_buttons.len() as i32;
    let cols = ((n + 1) / 2).max(1);
    let bw = cw / cols;
    for (bi, b) in p.cmd_buttons.iter().enumerate() {
        let col = (bi as i32) % cols;
        let row = (bi as i32) / cols;
        let _ = unsafe {
            SetWindowPos(*b, None, col * bw, ccy0 + ROW_TAB + row * ROW_BUTTON, (bw - 2).max(1), ROW_BUTTON - 2, swp)
        };
    }
    // 按钮占用的高度 = 行数（向上取整），不是列数。
    let rows = (n + cols - 1) / cols;
    let buttons_h = rows * ROW_BUTTON;
    // 3.3 过滤行：obj_split | filter_edit | open。
    let filt_y = ROW_TAB + buttons_h;
    put_c(p.obj_split, 0, ccy0 + filt_y, 96, ROW_FILTER);
    put_c(p.open_btn, cw - 52, ccy0 + filt_y, 52, ROW_FILTER);
    put_c(p.filter_edit, 100, ccy0 + filt_y, (cw - 100 - 56).max(20), ROW_FILTER);
    let body_y = filt_y + ROW_FILTER;
    let body_h = (content_h - body_y).max(40);

    // 3.4 树 | 列表：显隐由全局 tree_mode 决定（附录 A.3）；窗格 TreeN 位
    // 只反映在树根复选框上（自主解释，*跟随子目录行为待复核*）。
    let mode = frame.session.tree_mode.to_u32();
    let geom = tree::tree_layout_for(mode);
    // body 矩形用 content 客户坐标（相对 content）。
    let body = (cx0, ccy0 + body_y, cw, body_h);
    let (tree_rect, list_rect) = match geom {
        None => (None, body),
        Some(false) => {
            // 双列：内容区 body 切分。
            match tree::tree_column_rect(body, frame.session.panes[index].tree_split) {
                Some((tr, lr)) => (Some(tr), lr),
                None => (None, body),
            }
        }
        Some(true) => (Some(tree::tree_overlay_rect(body)), body),
    };

    // 列表容器 + 列表占满 list_host 客户区。
    put_c(p.list_host, list_rect.0, list_rect.1, list_rect.2, list_rect.3);
    if let Some(lq) = p.list {
        let _ = unsafe { SetWindowPos(lq, None, 0, 0, list_rect.2.max(1), list_rect.3.max(1), swp) };
    }

    // 树显隐与几何。
    if let Some(tv) = p.tree {
        match tree_rect {
            Some((tx, ty, tw, th)) => {
                unsafe {
                    let _ = ShowWindow(tv, SW_SHOWNA);
                    // 覆盖模式需要树压在列表上（HWND_TOP）。
                    if geom == Some(true) {
                        let _ = SetWindowPos(
                            tv,
                            Some(HWND_TOP),
                            tx,
                            ty,
                            tw.max(1),
                            th.max(1),
                            SET_WINDOW_POS_FLAGS(SWP_NOACTIVATE.0),
                        );
                    } else {
                        let _ = SetWindowPos(tv, None, tx, ty, tw.max(1), th.max(1), swp);
                    }
                }
            }
            None => {
                let _ = unsafe { ShowWindow(tv, SW_HIDE) };
            }
        }
    }
    // 窗格状态栏（container 子窗）。
    put(p.status, 0, h - status_h, w, status_h);
}

// ---------- 系统图标列表 ----------

/// 进程级缓存：小图标系统 ImageList 句柄 + 文件夹图标索引。
static SYS_IL: std::sync::OnceLock<(isize, i32)> = std::sync::OnceLock::new();

/// `SHGetFileInfoW(SHGFI_SYSICONINDEX)` 返回系统小图标 ImageList 句柄；
/// 以 `.` + FILE_ATTRIBUTE_DIRECTORY + USEFILEATTRIBUTES 查询拿文件夹图标索引。
fn sys_small_imagelist() -> Option<(isize, i32)> {
    let pair = *SYS_IL.get_or_init(|| unsafe {
        let mut fi = windows::Win32::UI::Shell::SHFILEINFOW::default();
        let dot: Vec<u16> = ".".encode_utf16().chain(std::iter::once(0)).collect();
        let use_shgfi = windows::Win32::UI::Shell::SHGFI_SMALLICON
            | windows::Win32::UI::Shell::SHGFI_SYSICONINDEX
            | windows::Win32::UI::Shell::SHGFI_USEFILEATTRIBUTES;
        let r = windows::Win32::UI::Shell::SHGetFileInfoW(
            windows::core::PCWSTR::from_raw(dot.as_ptr()),
            FILE_ATTRIBUTE_DIRECTORY,
            Some(&mut fi),
            core::mem::size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32,
            use_shgfi,
        );
        if r == 0 {
            (0, -1)
        } else {
            (r as isize, fi.iIcon)
        }
    });
    if pair.0 != 0 {
        Some(pair)
    } else {
        None
    }
}

/// 取（或缓存）系统小图标列表句柄。
unsafe fn system_small_imagelist() -> Option<HIMAGELIST> {
    sys_small_imagelist().map(|(h, _)| HIMAGELIST(h))
}

/// 文件夹图标在系统小图标列表中的索引；失败返回 -1。
unsafe fn folder_icon_index() -> i32 {
    sys_small_imagelist().map(|(_, i)| i).unwrap_or(-1)
}

// ---------- 小工具 ----------

/// 建子窗口；失败返回 Err（0.62 的 BOOL 错误）。
unsafe fn create_child(
    parent: HWND,
    hinstance: HINSTANCE,
    class: PCWSTR,
    title: PCWSTR,
    style: WINDOW_STYLE,
    id: i32,
) -> windows::core::Result<HWND> {
    let r = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            title,
            style,
            0, 0, 10, 10,
            Some(parent),
            Some(HMENU(id as _)),
            Some(hinstance),
            None,
        )
    };
    match r {
        Ok(h) => Ok(h),
        Err(e) => {
            Err(e)
        }
    }
}

/// ReBar 当前高度：ReBar 自管布局，直接问它（波段插入后即有值）。
unsafe fn rebar_height(rb: Option<HWND>) -> i32 {
    match rb {
        Some(h) => {
            let n = unsafe { SendMessageW(h, RB_GETBARHEIGHT, None, None) }.0;
            if n > 0 {
                n as i32
            } else {
                28
            }
        }
        None => 28,
    }
}

/// 状态栏高度。
unsafe fn statusbar_height(sb: Option<HWND>) -> i32 {
    match sb {
        Some(h) => {
            let mut rc = RECT::default();
            if unsafe { GetWindowRect(h, &mut rc) }.is_ok() && rc.bottom > rc.top {
                rc.bottom - rc.top
            } else {
                22
            }
        }
        None => 22,
    }
}

/// 状态栏第 i 段文本（SB_SETTEXTW；i|0 = 左对齐）。
unsafe fn set_status_text(sb: HWND, part: i32, text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        SendMessageW(
            sb,
            SB_SETTEXTW,
            Some(WPARAM(part as usize)),
            Some(LPARAM(wide.as_ptr() as isize)),
        );
    }
}

/// 全局状态栏第 0 段：所有窗格对象数合计（原版语义：活动窗格计数，先用合计占位）。
fn object_count_text(frame: &MainFrame) -> String {
    let mut n = 0usize;
    for pane in &frame.session.panes {
        if let Some(t) = pane.tabs.get(pane.tabs_pos) {
            let _ = t;
            n += 0; // 未枚举时未知，首枚举后由 populate_pane_chrome 更新
        }
    }
    format!("{n} 个对象")
}

/// 路径所在驱动器的「本地磁盘 (D:)」与「空闲: X GB 在 Y TB」。
fn drive_summary(path: &std::path::Path) -> Option<(String, String)> {
    let prefix = path.to_string_lossy();
    let colon = prefix.find(':')?;
    if colon == 0 {
        return None;
    }
    let root = format!("{}:\\", &prefix[..colon]);
    let rootwide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();

    let mut vol = [0u16; 260];
    let mut label = format!("本地磁盘 ({}:)", &prefix[..colon]);
    unsafe {
        if GetVolumeInformationW(
            PCWSTR::from_raw(rootwide.as_ptr()),
            Some(&mut vol),
            None,
            None,
            None,
            None,
        )
        .is_ok()
        {
            let v: String = vol
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| char::from_u32(c as u32).unwrap_or('\u{fffd}'))
                .collect();
            if !v.is_empty() {
                label = format!("{v} ({}:)", &prefix[..colon]);
            }
        }

        let mut free: u64 = 0;
        let mut total: u64 = 0;
        if GetDiskFreeSpaceExW(
            PCWSTR::from_raw(rootwide.as_ptr()),
            None,
            Some(&mut total),
            Some(&mut free),
        )
        .is_ok()
        {
            let free_str = format_bytes(free);
            let total_str = format_bytes(total);
            return Some((label, format!("空闲: {free_str} 在 {total_str}")));
        }
    }
    Some((label, String::new()))
}

/// 字节数 → 人类可读（原版格式：GB/TB，一位小数）。
fn format_bytes(b: u64) -> String {
    const KB: f64 = 1024.0;
    let v = b as f64;
    if v >= KB * KB * KB * KB {
        format!("{:.2} TB", v / (KB * KB * KB * KB))
    } else if v >= KB * KB * KB {
        format!("{:.2} GB", v / (KB * KB * KB))
    } else if v >= KB * KB {
        format!("{:.1} MB", v / (KB * KB))
    } else if v >= KB {
        format!("{:.1} KB", v / KB)
    } else {
        format!("{b} B")
    }
}

/// 文件大小列（原版「大小」：KB/MB/GB）。
fn format_size(b: u64) -> String {
    format_bytes(b)
}

/// 选中列表的某一行（测试钩子；行号越界则忽略）。
unsafe fn select_row_if_valid(list: HWND, row: i32) {
    let cnt = unsafe { SendMessageW(list, LVM_GETITEMCOUNT, None, None) }.0 as i32;
    if row < 0 || row >= cnt {
        return;
    }
    let st = LVITEMW {
        mask: LVIF_STATE,
        state: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
        stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0),
        ..Default::default()
    };
    unsafe {
        SendMessageW(
            list,
            LVM_SETITEMSTATE,
            Some(WPARAM(row as usize)),
            Some(LPARAM(&st as *const LVITEMW as isize)),
        )
    };
}

/// 「关于」对话框（自绘文案；PRODUCT_SPEC R1 后不再使用原版模板 100）。
/// MessageBoxW 兜底；品牌文案的唯一来源是 `brand` 模块。
pub unsafe fn show_about_brand(parent: HWND) {
    let title: Vec<u16> = crate::brand::PRODUCT_NAME
        .encode_utf16().chain(std::iter::once(0)).collect();
    let text = format!(
        "{} {}
{}
Rust + 原生 Win32",
        crate::brand::PRODUCT_NAME,
        crate::brand::PRODUCT_VERSION,
        crate::brand::TAGLINE
    );
    let textw: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            Some(parent),
            windows::core::PCWSTR(textw.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::{column_sort_click, dir_of_selection};
    use std::path::{Path, PathBuf};

    #[test]
    fn entering_subdir_when_selection_differs_from_current() {
        // is_dir 依赖文件系统，用真实存在的临时子目录。
        let cur = std::env::temp_dir();
        let sub = std::env::temp_dir().join("qdir-sub-existing-9135");
        std::fs::create_dir_all(&sub).unwrap();
        assert_eq!(dir_of_selection(Some(sub.as_path()), &cur), Some(sub.clone()));
        let _ = std::fs::remove_dir(&sub);
    }

    #[test]
    fn parent_when_selection_is_current_itself() {
        let cur = Path::new(r"C:\Users\me");
        assert_eq!(dir_of_selection(Some(cur), cur), Some(PathBuf::from(r"C:\Users")));
    }

    #[test]
    fn root_selection_stays() {
        let cur = Path::new(r"D:\");
        assert_eq!(dir_of_selection(Some(cur), cur), None);
    }

    #[test]
    fn file_selection_does_not_navigate() {
        let cur = Path::new(r"C:\Users\me");
        let f = std::env::temp_dir().join("qdir-not-a-dir-9134.txt");
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(dir_of_selection(Some(f.as_path()), cur), None);
        let _ = std::fs::remove_file(&f);
    }

    #[test]
    fn click_same_column_flips_direction() {
        use crate::core::{SortDir, SortKey};
        // 名称列已升序 → 点名称列（col 0）→ 降序。
        let (k, d) = column_sort_click(SortKey::Name, SortDir::Asc, 0).unwrap();
        assert_eq!(k, SortKey::Name);
        assert_eq!(d, SortDir::Desc);
    }

    #[test]
    fn click_new_column_sorts_ascending() {
        use crate::core::{SortDir, SortKey};
        // 当前按名称升序，点大小列（col 3）→ 切到 Size 升序。
        let (k, d) = column_sort_click(SortKey::Name, SortDir::Asc, 3).unwrap();
        assert_eq!(k, SortKey::Size);
        assert_eq!(d, SortDir::Asc);
    }

    #[test]
    fn column_map_matches_original_layout() {
        use crate::core::SortKey;
        // 原版列头顺序：名称/修改日期/类型/大小。
        assert_eq!(column_sort_click(SortKey::Name, crate::core::SortDir::Desc, 1).unwrap().0, SortKey::Modified);
        assert_eq!(column_sort_click(SortKey::Name, crate::core::SortDir::Desc, 2).unwrap().0, SortKey::Type);
        assert_eq!(column_sort_click(SortKey::Name, crate::core::SortDir::Desc, 4), None);
    }

    #[test]
    fn no_selection_does_not_navigate() {
        let cur = Path::new(r"C:\Users\me");
        assert_eq!(dir_of_selection(None, cur), None);
    }
}
