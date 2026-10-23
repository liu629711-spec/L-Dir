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
use windows::Win32::System::SystemServices::{SS_BITMAP, SS_NOTIFY};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW,
    GetVolumeInformationW,
};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::brand;
use crate::core::{ColumnDef, Config, FileItem, PaneId, PaneState, SessionState, sort_items};
use crate::shell::{
    EnumOptions, FileItemExt, HostWindow, IconCache, IconSize, TrayEvent, TrayIcon, is_tray_message,
    list_directory_detailed, tray_event_from_message,
};
use crate::ui::pane::format_filetime;
use crate::ui_native::splitter::{SPLITTER_WIDTH, SplitterGroup};
use crate::ui_native::tree::{self, TreeNodes};
use crate::core::TreeMode;

/// LVS_REPORT(0x0001) | LVS_SHOWSELALWAYS(0x0008) | LVS_EDITLABELS(0x200)
/// （批次A/A3：EDITLABELS 让窗格列表支持 F2 内联重命名）。
const LVS_REPORT_SHOWSEL: u32 = 0x0001 | 0x0008 | 0x0200;

/// 拖放队列到达的自定义消息（批次B/B3：WM_APP+2）。COM 回调只入队并
/// PostMessage，主线程在此消费，绝不在 COM 回调里直接操作 UI。
const WM_APP_DROP: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 2;
/// 文件夹变更通知消息（批次D/D1）。
/// **必须避开 0x8001**：WM_TRAY（托盘回调，历史硬编码 0x8001）与
/// WM_APP_DROP（WM_APP+2）已占用；曾用 WM_APP+1 与托盘撞车，监视通知
/// 全被 WM_TRAY 分支吞掉（窗口过程实测：消息到达但走错分支）。
const WM_APP_WATCH: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 3;
/// 拖放落点延迟注册的定时器 id（批次B/B3；0ms 一次性）。
const DROP_REGISTER_TIMER_ID: usize = 1;
/// 监视防抖定时器 id（批次D/D1；200ms 一次性）。
const WATCH_TIMER_ID: usize = 2;
/// 监视探针延迟定时器 id（批次D/D1；300ms 一次性）。
/// 探针不能在 on_create 内直接跑：其有界泵会把 0ms 注册定时器提前到
/// WM_CREATE 里触发，而 SHChangeNotifyRegister 在窗口创建期间段错误
/// （实测 WER 实锤）。延迟到主消息循环后执行，注册链才安全。
const WATCH_PROBE_TIMER_ID: usize = 3;
/// 监视防抖窗口（毫秒）。
const WATCH_DEBOUNCE_MS: u64 = 200;

// 内联重命名挂起的旧路径（LVN_BEGINLABELEDITW 置位、END/CANCEL 消费）。
// ListView 同一时刻全进程只有一个编辑框，单槽足够；线程局部（UI 线程独占）。
thread_local! {
    static PENDING_RENAME_PATH: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// 键盘动作（批次A/A1：子类化 list 角色 Ctrl 组合键与 F2 的路由目标）。
/// 作用域=发键窗格（非活动焦点窗格）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KbAction {
    Copy,
    Cut,
    Paste,
    SelectAll,
    Delete,
    NewTab,
}

/// 主窗口类名（自有，见 PRODUCT_SPEC R1）。
pub const FRAME_CLASS_NAME: &str = brand::WINDOW_CLASS;

/// 托盘回调消息（WM_APP+1；契约与 shell::tray 文档一致）。
pub const WM_TRAY: u32 = 0x8001;

/// 托盘事件 → 动作（纯函数单测；语义按 shell::tray 文档：
/// 单击/双击还原，右键弹菜单，其余忽略）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrayAction {
    Restore,
    ShowMenu,
    Ignore,
}

pub fn tray_action_for(ev: &TrayEvent) -> TrayAction {
    match ev {
        TrayEvent::Select | TrayEvent::DoubleClick => TrayAction::Restore,
        TrayEvent::ContextMenu => TrayAction::ShowMenu,
        _ => TrayAction::Ignore,
    }
}

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
/// 预览面板：图片 static / 文本 Edit。
const CID_PREVIEW_IMG: i32 = 10;
const CID_PREVIEW_TEXT: i32 = 11;

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

/// 原版详情视图列（UIA dump：名称/修改日期/类型/大小）——批次D/D3 前
/// 的硬编码列集，现由 `column_meta` + ColumnConfig 模型驱动，保留仅作
/// 历史对照（不再直接使用）。
#[allow(dead_code)]
const LEGACY_COLUMN_TITLES: &[&str] = &["名称", "修改日期", "类型", "大小"];
#[allow(dead_code)]
const LEGACY_COLUMN_WIDTHS: &[i32] = &[200, 150, 120, 90];

/// 列 key → (标题, 默认宽度)（批次D/D3：ColumnConfig 模型驱动列集）。
/// 原 UI 硬编码 4 列（名称/修改日期/类型/大小）；模型含 Ext/Attr 键，
/// 接入后列集可配（偏差记录：原「类型」列无对应键，默认多「扩展名/属性」列）。
pub(crate) fn column_meta(key: &str) -> (&'static str, i32) {
    match key {
        "Name" => ("名称", 200),
        "Ext" => ("扩展名", 60),
        "Size" => ("大小", 90),
        "Modified" => ("修改日期", 150),
        "Attr" => ("属性", 60),
        _ => ("列", 80),
    }
}

/// 列菜单项顺序（与 ColumnConfig 默认 columns 同序；id = 30040 + i）。
pub(crate) const COLUMN_KEYS: &[&str] = &["Name", "Ext", "Size", "Modified", "Attr"];

/// 可见列按 order 排序（批次D/D3 纯函数）：过滤 visible + 稳定排序
/// （order 并列保持 columns 原序，sort_by_key 稳定）。
pub fn visible_ordered_columns(cfg: &crate::core::ColumnConfig) -> Vec<crate::core::ColumnDef> {
    let mut v: Vec<crate::core::ColumnDef> =
        cfg.columns.iter().filter(|c| c.visible).cloned().collect();
    v.sort_by_key(|c| c.order);
    v
}

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
            // 驱动器 ComboBox 选盘（批次A/A6）：CBN_SELCHANGE 上送主框架，
            // 由其读 CB_GETCURSEL→CB_GETLBTEXT→navigate_to（该窗格）。
            if (1000..1400).contains(&id)
                && (id - 1000) % 100 == CID_COMBO_DRIVES as u32
                && notify == CBN_SELCHANGE
            {
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
    payload: Option<(Config, SessionState, HINSTANCE, std::path::PathBuf)>,
}

impl PendingFrame {
    pub fn new(
        config: Config,
        session: SessionState,
        hinstance: HINSTANCE,
        ini_path: std::path::PathBuf,
    ) -> Self {
        PendingFrame {
            payload: Some((config, session, hinstance, ini_path)),
        }
    }

    /// 取出载荷；重复调用返回 None。
    pub fn take(
        &mut self,
    ) -> Option<(Config, SessionState, HINSTANCE, std::path::PathBuf)> {
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
    /// 预览面板（Vorschau）：图片 static（SS_BITMAP）+ 文本只读 Edit。
    preview_img: Option<HWND>,
    preview_text: Option<HWND>,
    /// 当前挂在 preview_img 上的位图（换图时 DeleteObject）。
    preview_bmp: Option<isize>,
    /// 窗格目录树（SysTreeView32，tree_mode 非 0 时可见；附录 A.3）。
    tree: Option<HWND>,
    /// 树节点登记（HTREEITEM→路径）+ 回环抑制标志（busy）。
    tree_nodes: TreeNodes,
    drive_combo: Option<HWND>,
    /// 该窗格 list 的 OLE 落点对象（批次B/B3；注册进 ole32 后必须保活，
    /// 窗格销毁时先 revoke 再释放）。
    drop_target: Option<Box<crate::shell::PaneDropTarget>>,
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
    /// 主窗口句柄（on_create 设置；状态栏刷新取宽用）。
    pub main: HWND,
    /// 配置文件的实际加载路径（规则编辑确定后写回此处）。
    pub ini_path: std::path::PathBuf,
    /// 托盘图标（P3：最小化到托盘 + 单击还原 + 右键菜单）。
    pub tray: Option<TrayIcon>,
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
    /// 批次D/D1：文件夹变更通知 id（四窗格一次多条目注册；0=未注册）。
    pub watch: Option<crate::shell::WatchHandle>,
    /// 批次D/D1：文件夹监视是否已首次注册（WM_CREATE 期间不注册——
    /// SHChangeNotifyRegister 在窗口创建未完成时调用会段错误，同 B3
    /// RegisterDragDrop 教训；首次注册由 DROP_REGISTER_TIMER 到点做）。
    pub watch_started: bool,
    /// 批次D/D1：脏窗格标记（WM_APP+3 置位，防抖 timer 到点消费）。
    pub watch_dirty: [bool; 4],
    /// 窗格是否已灌过数据（避免重复枚举）。
    panes_filled: [bool; 4],
    /// 窗格过滤框文本（EN_CHANGE 更新；fill_pane 应用为快速筛名）。
    pane_filter: [String; 4],
    /// 预览面板每窗格内容类型：0=文本 1=图片 2=隐藏（布局 pass 消费）。
    preview_mode: [u8; 4],
    /// 系统小图标 ImageList 里的文件夹图标索引（首建树时解析一次）。
    folder_image: i32,
}

impl MainFrame {
    pub fn new(
        config: Config,
        session: SessionState,
        _hinstance: HINSTANCE,
        ini_path: std::path::PathBuf,
    ) -> Self {
        MainFrame {
            config,
            session,
            main: HWND::default(),
            tray: None,
            ini_path,
            panes: Default::default(),
            splitters: SplitterGroup::horizontal(4),
            menu: HMENU::default(),
            rebar: None,
            status: None,
            icons: IconCache::new(),
            ui_font: HFONT::default(),
            ui_scale: 100,
            watch: None,
            watch_started: false,
            watch_dirty: [false; 4],
            panes_filled: [false; 4],
            pane_filter: Default::default(),
            preview_mode: [2; 4],
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
            WM_SYSCOMMAND => Self::on_syscommand(hwnd, wp),
            WM_TRAY => Self::on_tray_message(hwnd, lp),
            WM_NOTIFY => {
                if let Some(frame) = Self::frame_of(hwnd) {
                    frame.on_notify(hwnd, lp)
                } else {
                    LRESULT(0)
                }
            }
            WM_DROPFILES => {
                // 批次B/B3 兜底路径：OLE 落点未注册（或注册失败）时 Explorer
                // 经 WM_DROPFILES 拖入。HDROP 在 **wParam**（lParam 是坐标）；
                // DropFiles::from_raw 取列表 + DragFinish。
                if let Some(frame) = Self::frame_of(hwnd) {
                    match crate::shell::DropFiles::from_raw(wp.0 as isize) {
                        Ok(df) => {
                            let paths = df.paths().to_vec();
                            // 落点窗格：按落点坐标命中窗格 list（纯函数+屏幕坐标）。
                            let pt = unsafe { frame_drop_point(wp.0 as isize) };
                            let pidx = frame.pane_at_screen_point(pt);
                            if !paths.is_empty() {
                                // WM_DROPFILES 默认复制（Ctrl=copy/Shift=move；
                                // 此处从简按 Explorer 默认 COPY）。
                                frame.on_dropped_paths(hwnd, pidx, &paths, crate::shell::DROPEFFECT_COPY_VALUE, pt);
                            }
                        }
                        Err(e) => eprintln!("[qdir-rs] WM_DROPFILES 解析失败：{e}"),
                    }
                }
                LRESULT(0)
            }
            WM_APP_DROP => {
                // 批次B/B3：COM Drop 回调只入队 + 本消息，主线程在此消费。
                if let Some(frame) = Self::frame_of(hwnd) {
                    frame.consume_dropped(hwnd);
                }
                LRESULT(0)
            }
            WM_APP_WATCH => {
                // 批次D/D1：文件夹变更通知（多条目单 id 注册无法区分窗格，
                // 保守全脏 + 200ms 防抖合并）。
                if let Some(frame) = Self::frame_of(hwnd) {
                    frame.on_watch_notify(hwnd);
                }
                LRESULT(0)
            }
            WM_INITMENUPOPUP => {
                // 批次D/D3：菜单弹出前按活动窗格刷新「列」子项勾选态。
                if let Some(frame) = Self::frame_of(hwnd) {
                    let pidx = frame
                        .session
                        .focus
                        .number()
                        .saturating_sub(1) as usize;
                    if let Some(pane) = frame.session.panes.get(pidx) {
                        for (i, key) in COLUMN_KEYS.iter().enumerate() {
                            let on = pane
                                .columns
                                .columns
                                .iter()
                                .find(|c| c.key == *key)
                                .map(|c| c.visible)
                                .unwrap_or(false);
                            let st = if on {
                                MF_CHECKED.0
                            } else {
                                MF_UNCHECKED.0
                            };
                            unsafe {
                                CheckMenuItem(
                                    HMENU(lp.0 as *mut core::ffi::c_void),
                                    (crate::ui_native::menu::cmd_id::COL_BASE + i as u32) as u32,
                                    MF_BYCOMMAND.0 | st,
                                );
                            }
                        }
                    }
                }
                LRESULT(0)
            }
            WM_TIMER if wp.0 == DROP_REGISTER_TIMER_ID => {
                // 批次B/B3：窗口创建完成后注册 OLE 落点（WM_CREATE 内禁止，
                // 见 register_all_drop_targets 文档）。一次性定时器。
                let _ = unsafe { KillTimer(Some(hwnd), DROP_REGISTER_TIMER_ID) };
                if let Some(frame) = Self::frame_of(hwnd) {
                    unsafe { frame.register_all_drop_targets() };
                    // 批次D/D1：同点做监视首次注册（SHChangeNotifyRegister
                    // 在 WM_CREATE 期间会段错误，延到消息循环后）。
                    if !frame.watch_started {
                        frame.reregister_watches(hwnd);
                    }
                }
                LRESULT(0)
            }
            WM_TIMER if wp.0 == WATCH_TIMER_ID => {
                // 批次D/D1：防抖到点，重灌脏窗格。
                let _ = unsafe { KillTimer(Some(hwnd), WATCH_TIMER_ID) };
                if let Some(frame) = Self::frame_of(hwnd) {
                    frame.flush_watch_dirty(hwnd);
                }
                LRESULT(0)
            }
            WM_TIMER if wp.0 == WATCH_PROBE_TIMER_ID => {
                // 批次D/D1：监视探针延迟自跑（主循环上下文，注册链安全）。
                let _ = unsafe { KillTimer(Some(hwnd), WATCH_PROBE_TIMER_ID) };
                if let Some(frame) = Self::frame_of(hwnd) {
                    unsafe { frame.watch_probe_run(hwnd) };
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                // 托盘图标随窗销毁（TrayIcon::drop → NIM_DELETE）。
                if let Some(frame) = Self::frame_of(hwnd) {
                    frame.tray = None;
                    // 批次B/B3：先反注册 OLE 落点再放对象（否则 ole32 持悬垂指针）。
                    unsafe { frame.revoke_pane_drop_targets() };
                    // 批次D/D1：反注册文件夹监视（WatchHandle::drop 兜底，
                    // 此处显式释放便于日志时序确定）。
                    if let Some(mut h) = frame.watch.take() {
                        h.release();
                    }
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }

    /// 最小化拦截：tray_minimize 开 → 藏窗（懒加托盘图标）吞事件；
    /// 关 → 默认任务栏最小化。原版托盘行为未反编译确证 *待复核*。
    unsafe fn on_syscommand(hwnd: HWND, wp: WPARAM) -> LRESULT {
        if (wp.0 as u32) & 0xFFF0 == SC_MINIMIZE {
            let tray_on = Self::frame_of(hwnd).map(|f| f.config.tray_minimize).unwrap_or(false);
            if tray_on {
                if let Some(frame) = Self::frame_of(hwnd) {
                    if frame.tray.is_none() {
                        frame.tray = Self::make_tray(hwnd);
                    }
                    if frame.tray.is_some() {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                        println!("[qdir-rs] TEST 托盘：最小化隐藏窗格主窗");
                        return LRESULT(0);
                    }
                }
                // 图标添加失败不吞事件（回落系统最小化）。
                eprintln!("[qdir-rs] 托盘添加失败，回落普通最小化");
            }
        }
        DefWindowProcW(hwnd, WM_SYSCOMMAND, wp, LPARAM(0))
    }

    /// 列表右键菜单（shell::contextmenu 接线，原版资源管理器语义 CMF_EXPLORE）。
    /// 有选中项=对选中项；点空白=对窗格目录本身（背景菜单）。菜单执行后重灌
    /// 该窗格（新建/删除/粘贴等都会改变目录内容）。
    unsafe fn pane_context_menu(&mut self, main: HWND, pidx: usize) {
        let mut paths = self.selected_paths(pidx);
        if paths.is_empty() {
            let p = self.pane_path(pidx);
            if p.as_os_str().is_empty() {
                return;
            }
            paths = vec![p];
        }
        let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
        match crate::shell::ContextMenu::query_and_run(&paths, host) {
            Ok(id) => {
                println!("[qdir-rs] TEST 右键菜单 id={id} 作用 {} 项", paths.len());
                self.fill_pane(pidx);
            }
            Err(e) => println!("[qdir-rs] TEST 右键菜单取消/失败：{e}"),
        }
    }


    fn make_tray(hwnd: HWND) -> Option<TrayIcon> {
        let hicon = unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or_default() };
        let mut t = TrayIcon::new(HostWindow::from_raw_hwnd(hwnd.0 as isize), 1, WM_TRAY, hicon);
        match t.add().and_then(|_| t.set_tip(brand::PRODUCT_NAME)).and_then(|_| {
            t.set_version_4()
        }) {
            Ok(()) => Some(t),
            Err(e) => {
                eprintln!("[qdir-rs] 托盘 add 失败：{e}");
                None
            }
        }
    }

    unsafe fn on_tray_message(hwnd: HWND, lp: LPARAM) -> LRESULT {
        let Some(frame) = Self::frame_of(hwnd) else {
            return LRESULT(0);
        };
        if !frame.tray.as_ref().is_some_and(|t| is_tray_message(WM_TRAY, t.callback_message())) {
            return LRESULT(0);
        }
        let ev = tray_event_from_message(WM_TRAY, lp.0);
        match tray_action_for(&ev) {
            TrayAction::Restore => {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
                println!("[qdir-rs] TEST 托盘还原 {ev:?}");
            }
            TrayAction::ShowMenu => {
                println!("[qdir-rs] TEST 托盘右键菜单");
                let menu = match crate::ui_native::menu::build_tray_menu() {
                    Ok(m) => m,
                    Err(_) => return LRESULT(0),
                };
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let _ = SetForegroundWindow(hwnd);
                let cmd = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN,
                    pt.x,
                    pt.y,
                    None,
                    hwnd,
                    None,
                );
                let _ = DestroyMenu(menu);
                match cmd.0 {
                    c if c == crate::ui_native::menu::cmd_id::TRAY_SHOW as i32 => {
                        let _ = ShowWindow(hwnd, SW_RESTORE);
                        let _ = SetForegroundWindow(hwnd);
                    }
                    c if c == crate::ui_native::menu::cmd_id::TRAY_EXIT as i32 => {
                        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                    _ => {}
                }
            }
            TrayAction::Ignore => {
                println!("[qdir-rs] TEST 托盘事件忽略 {ev:?}");
            }
        }
        LRESULT(0)
    }

    unsafe fn on_create(hwnd: HWND, lp: LPARAM) -> LRESULT {
        // 从 CREATESTRUCT.lpCreateParams 取回真实配置（取一次即空）。
        let mut taken: Option<(Config, SessionState, HINSTANCE, std::path::PathBuf)> = None;
        let mut hmenu = HMENU::default();
        if lp.0 != 0 {
            let cs = &*(lp.0 as *const CREATESTRUCTW);
            hmenu = cs.hMenu;
            if !cs.lpCreateParams.is_null() {
                let pending = &mut *(cs.lpCreateParams as *mut PendingFrame);
                taken = pending.take();
            }
        }
        let (config, session, hinstance, ini_path) = taken.unwrap_or_else(|| {
            (
                Config::default(),
                SessionState::default(),
                HINSTANCE::default(),
                std::path::PathBuf::from(crate::brand::INI_FILE),
            )
        });

        let frame = Box::leak(Box::new(MainFrame::new(config, session, hinstance, ini_path)));
        frame.menu = hmenu;
        frame.main = hwnd;
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
        if frame.status.is_some() {
            // 原版 UIA 确证「5 段文本」；边界按窗宽伸缩、内容按会话现算。
            unsafe { frame.refresh_status() };
        }

        // 3. 四个窗格：容器 + 状态栏 + 内容控件。
        for i in 0..4 {
            create_pane(frame, hwnd, hinstance, i);
        }

        // 托盘冒烟钩子：QDIR_TEST_TRAY=1 时立即添加图标并日志
        // （行为链由外部 PostMessage WM_SYSCOMMAND/WM_TRAY 驱动，
        // 两者 lParam 皆无指针，跨进程安全）。
        if std::env::var("QDIR_TEST_TRAY").is_ok() {
            // 钩子同时打开拦截开关（等价设置对话框勾选），测完整藏窗路径。
            frame.config.tray_minimize = true;
            if frame.tray.is_none() {
                frame.tray = Self::make_tray(hwnd);
            }
            println!("[qdir-rs] TEST 托盘 add={} tip={:?}", frame.tray.is_some(),
                frame.tray.as_ref().map(|t| t.tip().to_string()).unwrap_or_default());
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

        // 验证钩子：QDIR_TEST_PREVIEW=1 → 开预览并选中窗格1第一个可预览
        // 文件（=img 优先图片分支；QDIR_TEST_PREVIEW_DIR 覆盖窗格1目录）。
        if std::env::var("QDIR_TEST_PREVIEW").is_ok() {
            if let Ok(dir) = std::env::var("QDIR_TEST_PREVIEW_DIR") {
                let p = std::path::PathBuf::from(dir);
                if p.is_dir() {
                    let ts = frame.session.panes[0].tabs_pos;
                    let tab = &mut frame.session.panes[0].tabs[ts];
                    tab.path = p.clone();
                    tab.history.clear();
                    tab.history.push(p);
                    tab.history_pos = 0;
                    frame.fill_pane(0);
                }
            }
            let want_img = std::env::var("QDIR_TEST_PREVIEW")
                .map(|v| v == "img")
                .unwrap_or(false);
            let _ = frame.toggle_preview(hwnd);
            let is_img = |p: &std::path::Path| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| {
                        matches!(
                            e.to_ascii_lowercase().as_str(),
                            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp"
                        )
                    })
                    .unwrap_or(false)
            };
            let pick = frame.panes[0]
                .row_paths
                .iter()
                .position(|p| {
                    p.is_file()
                        && crate::panes::preview::previewable(p, &frame.config.preview_filter)
                        && (!want_img || is_img(p))
                });
            match pick {
                Some(r) => {
                    if let Some(list) = frame.panes[0].list {
                        unsafe { select_row_if_valid(list, r as i32) };
                        frame.update_preview_pane(0);
                        let mode = frame.preview_mode[0];
                        println!(
                            "[qdir-rs] TEST 预览钩子 选中行 {r} mode={} {:?}",
                            mode, frame.panes[0].row_paths[r]
                        );
                    }
                }
                None => println!("[qdir-rs] TEST 预览钩子：窗格1 无可预览文件"),
            }
        }

        // 批次B/B3：OLE 落点注册延到窗口创建完成后。WM_CREATE 期间
        // PostMessage 到本窗会被系统丢弃（队列关联未完成），故用 SetTimer
        // 0ms：WM_TIMER 必在消息循环启动后派发（RegisterDragDrop 若在
        // WM_CREATE 内调用会触发 ole32 0xC000041D 致命异常）。
        // 批次D/D1：监视首次注册同理（SHChangeNotifyRegister 在 WM_CREATE
        // 期间会段错误，实测复现），也由该定时器到点统一做。
        // **必须在测试钩子之前创建**：QDIR_TEST_WATCH 探针在 on_create 内
        // 有界泵消息等待 watch_started，而 WM_TIMER 只有定时器已存在且
        // 线程在泵消息时才派发——放钩子后则探针必然超时（批次D 实测教训）。
        unsafe {
            let _ = SetTimer(
                Some(hwnd),
                DROP_REGISTER_TIMER_ID,
                0,
                windows::Win32::UI::WindowsAndMessaging::TIMERPROC::None,
            );
        }

        // 批次A/A1 键盘钩子：QDIR_TEST_KEYS=1 → 选中窗格1第0行，向列表投递
        // Ctrl+C（SendMessageW WM_KEYDOWN，同进程安全），断言剪贴板出现该路径；
        // 再投递 Ctrl+A，用 LVM_GETSELECTEDCOUNT 断言计数==行数。
        if std::env::var("QDIR_TEST_KEYS").is_ok() {
            unsafe { frame.keys_probe(hwnd) };
        }
        // 批次A/A3 重命名钩子：QDIR_TEST_RENAME=1 → 临时目录建文件，选中，
        // 走 commit_inline_rename 落盘改名，断言磁盘上新名存在、旧名消失。
        if std::env::var("QDIR_TEST_RENAME").is_ok() {
            unsafe { frame.rename_probe(hwnd) };
        }
        // 批次A/A2+A4+A5 文件操作钩子：QDIR_TEST_OPS=1 → 临时目录内验证
        // 新建文件夹(唯一名落盘) / 新建标签页(tab 计数+1) / Del 进回收站(源消失)。
        if std::env::var("QDIR_TEST_OPS").is_ok() {
            unsafe { frame.ops_probe(hwnd) };
        }
        // 批次A/A1 剪切粘贴钩子：QDIR_TEST_CLIP=1 → 临时 src 建文件，Ctrl+X
        // 剪切(cut=true)，导航到 dst，Ctrl+V 粘贴，断言文件移动到 dst。
        if std::env::var("QDIR_TEST_CLIP").is_ok() {
            unsafe { frame.clip_probe(hwnd) };
        }
        // 批次A/A6 选盘钩子：QDIR_TEST_COMBO=1 → 对窗格1 ComboBox 设当前项到
        // 一个非当前盘，模拟 CBN_SELCHANGE，断言窗格导航到该盘根。
        if std::env::var("QDIR_TEST_COMBO").is_ok() {
            unsafe { frame.combo_probe(hwnd) };
        }
        // 批次B/B1 大图标钩子：QDIR_TEST_VIEW_LARGE=1 → 窗格1 切
        // LV_VIEW_ICON，回读 LVM_GETVIEW + LVSIL_NORMAL 图像列表非空。
        if std::env::var("QDIR_TEST_VIEW_LARGE").is_ok() {
            unsafe { frame.view_large_probe() };
        }
        // 批次B/B3 拖放钩子：QDIR_TEST_DND=1 → 进程内经丢弃队列向窗格1 注入
        // CF_HDROP 路径 + 自定义消息，断言落盘与窗格重灌（不经 COM 模态）。
        if std::env::var("QDIR_TEST_DND").is_ok() {
            unsafe { frame.dnd_probe(hwnd) };
        }
        // 批次B/B3 兜底钩子：QDIR_TEST_DROPFILES=1 → 进程内构造 DROPFILES
        // HGLOBAL 并 SendMessage WM_DROPFILES，验证兜底链（落点窗格判定 →
        // on_dropped_paths → 落盘）。
        if std::env::var("QDIR_TEST_DROPFILES").is_ok() {
            unsafe { frame.dropfiles_probe(hwnd) };
        }
        // 批次C/C2 自动补全钩子：QDIR_TEST_AUTOCOMPLETE=1 → 窗格1 地址栏
        // 写半个路径，断言 SHAutoComplete 挂载后窗口存活 + 文本可读回
        // （下拉内容为 *系统行为*，不断言）。
        if std::env::var("QDIR_TEST_AUTOCOMPLETE").is_ok() {
            unsafe { frame.autocomplete_probe(hwnd) };
        }
        // 批次C/C4 历史钩子：QDIR_TEST_HISTORY=1 → 合成两级导航 → 后退/
        // 前进，断言 history_pos 递减/递增与地址栏文本同步。
        if std::env::var("QDIR_TEST_HISTORY").is_ok() {
            unsafe { frame.history_probe(hwnd) };
        }
        // 批次D/D3 列显隐钩子：QDIR_TEST_COLS=1 → 切「扩展名」断言列头
        // 计数 N→N-1，切回 N。
        if std::env::var("QDIR_TEST_COLS").is_ok() {
            unsafe { frame.cols_probe() };
        }
        // 批次D/D2 布局槽钩子：QDIR_TEST_SLOTS=1 → 保存/载入/删除槽1，
        // 断言会话路径往返与槽文件生命周期（对话框模态另由
        // QDIR_TEST_SLOTS_UI 冒烟）。
        if std::env::var("QDIR_TEST_SLOTS").is_ok() {
            unsafe { frame.slots_probe() };
        }
        // 批次D/D1 监视钩子：QDIR_TEST_WATCH=1 → 向窗格1 当前目录写临时
        // 文件，经 WM_APP+3 + 防抖 timer 断言行数 +1；删文件断言恢复。
        if std::env::var("QDIR_TEST_WATCH").is_ok() {
            unsafe { frame.watch_probe(hwnd) };
        }
        // 批次B/B3：OLE 落点注册延到窗口创建完成后（SetTimer 已移至钩子区
        // 之前，见上；WM_CREATE 内 PostMessage 会被系统丢弃，故用 0ms 定时器，
        // WM_TIMER 必在消息循环启动后派发）。
        // 批次D/D1：监视首次注册**不在此处**做——SHChangeNotifyRegister
        // 在 WM_CREATE 期间（窗口创建未完成）调用会段错误（实测复现，
        // 同 RegisterDragDrop 教训），由 DROP_REGISTER_TIMER 到点
        // （消息循环启动后）统一注册；之后 navigate_to 换路径时
        // reregister_watches 即时重注册（watch_started 门控）。

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
            // 分段边界随宽度伸缩（refresh_status 统一重算+重设文本）。
            frame.refresh_status();
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
        // 驱动器 ComboBox 选盘导航（批次A/A6）：CBN_SELCHANGE → 读当前选中项
        // 文本 → 解析驱动器根 → navigate_to 该窗格。
        if (1000..1400).contains(&id)
            && (id - 1000) % 100 == CID_COMBO_DRIVES as u32
            && notify == CBN_SELCHANGE
        {
            let idx = ((id - 1000) / 100) as usize;
            if let Some(frame) = Self::frame_of(hwnd) {
                let combo = frame.panes.get(idx).and_then(|p| p.drive_combo);
                if let Some(cb) = combo {
                    let sel = unsafe { SendMessageW(cb, CB_GETCURSEL, None, None) }.0 as i32;
                    if sel >= 0 {
                        let len = unsafe {
                            SendMessageW(cb, CB_GETLBTEXTLEN, Some(WPARAM(sel as usize)), None)
                        }
                        .0 as usize;
                        let mut buf = vec![0u16; len + 1];
                        unsafe {
                            SendMessageW(
                                cb,
                                CB_GETLBTEXT,
                                Some(WPARAM(sel as usize)),
                                Some(LPARAM(buf.as_mut_ptr() as isize)),
                            );
                        }
                        let text = String::from_utf16_lossy(
                            &buf[..buf.iter().position(|&c| c == 0).unwrap_or(buf.len())],
                        );
                        frame.combo_navigate(idx, &text);
                    }
                }
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
                unsafe { self.refresh_status() };
                true
            }
            None => false,
        }
    }

    /// 即时应用设置对话框里的树样式位/缩进与标签 TCS_TOOLTIPS 位
    /// （设置对话框确定后调用；纯位映射见 settings_dialog::tree_styles）。
    pub unsafe fn apply_tree_settings(&mut self) {
        let styles = crate::ui_native::settings_dialog::tree_styles(&self.config.tree);
        for i in 0..4 {
            if let Some(tv) = self.panes[i].tree {
                // WS_CHILD|WS_VISIBLE 常位由 create 时已设；这里只翻样式位。
                let keep = WS_CHILD.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0 | WS_BORDER.0;
                let _ = SetWindowLongPtrW(
                    tv,
                    GWL_STYLE,
                    (keep | styles) as isize,
                );
                // 缩进：TVM_SETINDENT（像素→WORD）。
                let ind = self.config.tree.indent_width.min(200) as usize;
                let _ = SendMessageW(tv, TVM_SETINDENT, Some(WPARAM(ind)), None);
            }
            if let Some(tab) = self.panes[i].tab {
                let mut st = GetWindowLongPtrW(tab, GWL_STYLE) as u32;
                st &= !TCS_TOOLTIPS;
                st |= crate::ui_native::settings_dialog::tab_tooltips_bit(
                    self.config.tab.show_tooltips,
                );
                let _ = SetWindowLongPtrW(tab, GWL_STYLE, st as isize);
            }
        }
    }

    /// 统一刷新全局状态栏 5 段：0=活动窗格计数(+选中数)、1=版本、
    /// 2=用户名(x64)、3=盘符标签、4=空闲/总容量。分段边界随客户区宽伸缩。
    pub unsafe fn refresh_status(&mut self) {
        let Some(sb) = self.status else { return };
        let mut rc = RECT::default();
        let mut width = 800i32;
        if GetClientRect(self.main, &mut rc).is_ok() {
            width = (rc.right - rc.left).max(240);
        }
        let parts = status_parts(width);
        let _ = SendMessageW(
            sb,
            SB_SETPARTS,
            Some(WPARAM(parts.len())),
            Some(LPARAM(parts.as_ptr() as isize)),
        );
        // 段 0：活动窗格当前目录对象数 + 选中数（计数来自最近一次填充）。
        let focus = (self.session.focus.number() - 1) as usize;
        let count = self.panes.get(focus).map(|p| p.row_paths.len()).unwrap_or(0);
        let sel = self
            .panes
            .get(focus)
            .and_then(|p| p.list)
            .map(|l| unsafe { SendMessageW(l, LVM_GETSELECTEDCOUNT, None, None).0.max(0) as usize })
            .unwrap_or(0);
        let text0 = if sel > 0 {
            format!("{count} 个对象，已选 {sel}")
        } else {
            format!("{count} 个对象")
        };
        let (label, free) = match self.pane_path(focus) {
            p if !p.as_os_str().is_empty() => match drive_summary(&p) {
                Some((l, f)) => (l, f),
                None => (p.to_string_lossy().into_owned(), String::new()),
            },
            _ => (String::new(), String::new()),
        };
        let segs = [
            text0,
            brand::PRODUCT_VERSION.to_string(),
            format!("{} (x64)", std::env::var("USERNAME").unwrap_or_default()),
            label,
            free,
        ];
        for (i, t) in segs.iter().enumerate() {
            set_status_text(sb, i as i32, t);
        }
        // 验证钩子：QDIR_TEST_STATUS 打印 5 段文本与边界
        // （SB_GETTEXT 跨进程读指针失效，进程内自证）。
        if std::env::var("QDIR_TEST_STATUS").is_ok() {
            println!("[qdir-rs] TEST 状态栏 边界={parts:?} 文本={segs:?}");
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

    /// 刷新窗格预览面板：按当前选中项构建 PreviewData 并落到
    /// 文本 Edit / 图片 Static（SS_BITMAP）。无选中/目录/Filter 未命中=隐藏。
    pub fn update_preview_pane(&mut self, index: usize) {
        if !self.session.preview_visible {
            self.preview_mode[index] = 2;
            return;
        }
        let sel = self.selected_paths(index);
        let filter = self.config.preview_filter.clone();
        let data = match sel.first() {
            Some(path) if path.is_file() => {
                if !crate::panes::preview::previewable(path, &filter) {
                    None
                } else {
                    crate::panes::preview::build_preview(path, &filter, None).ok()
                }
            }
            _ => None,
        };
        let Some(data) = data else {
            self.preview_mode[index] = 2;
            self.preview_sync_visibility(index);
            return;
        };
        match data {
            crate::panes::preview::PreviewData::Text(tx) => {
                let mut body = tx.lines.join("\r\n");
                if tx.truncated {
                    body.push_str(&format!("\r\n… 截断（共 {} 字节）", tx.total_bytes));
                }
                self.preview_set_text(index, &body);
                self.preview_mode[index] = 0;
            }
            crate::panes::preview::PreviewData::Binary(b) => {
                let hex: String = b.head.iter().take(64).map(|x| format!("{x:02X} ")).collect();
                self.preview_set_text(
                    index,
                    &format!("二进制 {} 字节\r\n前导: {hex}", b.total_bytes),
                );
                self.preview_mode[index] = 0;
            }
            crate::panes::preview::PreviewData::Image(img) => {
                if unsafe { self.preview_set_bitmap(index, &img) } {
                    self.preview_mode[index] = 1;
                } else {
                    self.preview_mode[index] = 2;
                }
            }
        }
        self.preview_sync_visibility(index);
    }

    /// 按 preview_mode 直接切两控件显隐（几何不变时；开/关重排走 toggle）。
    pub fn preview_sync_visibility(&mut self, index: usize) {
        let mode = self.preview_mode[index];
        unsafe {
            if let Some(img) = self.panes[index].preview_img {
                let _ = ShowWindow(
                    img,
                    if mode == 1 && self.session.preview_visible {
                        SW_SHOWNA
                    } else {
                        SW_HIDE
                    },
                );
            }
            if let Some(txt) = self.panes[index].preview_text {
                let _ = ShowWindow(
                    txt,
                    if mode == 0 && self.session.preview_visible {
                        SW_SHOWNA
                    } else {
                        SW_HIDE
                    },
                );
            }
        }
    }

    /// Vorschau 开关：写会话、刷新/隐藏并重排（列表要腾出预览列）。
    pub unsafe fn toggle_preview(&mut self, main: HWND) -> bool {
        let on = !self.session.preview_visible;
        self.session.preview_visible = on;
        if on {
            let idx = (self.session.focus.number() - 1) as usize;
            self.update_preview_pane(idx);
        } else {
            for i in 0..4 {
                self.preview_mode[i] = 2;
            }
        }
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
        let _ = SendMessageW(main, WM_SIZE, Some(WPARAM(SIZE_RESTORED as usize)), None);
        on
    }

    fn preview_set_text(&mut self, index: usize, text: &str) {
        let Some(e) = self.panes[index].preview_text else { return };
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let _ = unsafe { SetWindowTextW(e, windows::core::PCWSTR::from_raw(wide.as_ptr())) };
    }

    /// RGBA8 → 32bpp DIB（BGRA、alpha 合成白底）并 STM_SETIMAGE；旧图删除。
    unsafe fn preview_set_bitmap(
        &mut self,
        index: usize,
        img: &crate::panes::preview::ImagePreview,
    ) -> bool {
        let Some(st) = self.panes[index].preview_img else {
            return false;
        };
        let (w, h) = (img.width.max(1) as i32, img.height.max(1) as i32);
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };
        let mut ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let Ok(hbm) = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut ptr, None, 0) else {
            return false;
        };
        if ptr.is_null() {
            let _ = DeleteObject(HGDIOBJ(hbm.0));
            return false;
        }
        let dst = core::slice::from_raw_parts_mut(ptr as *mut u8, (w * h * 4) as usize);
        dst.fill(0xFF);
        let n = (w as usize * h as usize).min(img.rgba.len() / 4);
        for i in 0..n {
            let s = &img.rgba[i * 4..i * 4 + 4];
            let a = s[3] as u32;
            let d = i * 4;
            dst[d] = ((s[2] as u32 * a + 255 * (255 - a)) / 255) as u8; // B
            dst[d + 1] = ((s[1] as u32 * a + 255 * (255 - a)) / 255) as u8; // G
            dst[d + 2] = ((s[0] as u32 * a + 255 * (255 - a)) / 255) as u8; // R
            dst[d + 3] = 255;
        }
        let old = SendMessageW(
            st,
            STM_SETIMAGE,
            Some(WPARAM(IMAGE_BITMAP.0 as usize)),
            Some(LPARAM(hbm.0 as isize)),
        );
        if old.0 != 0 {
            let _ = DeleteObject(HGDIOBJ(old.0 as *mut core::ffi::c_void));
        }
        self.panes[index].preview_bmp = Some(hbm.0 as isize);
        let _ = RedrawWindow(Some(st), None, None, RDW_INVALIDATE | RDW_UPDATENOW);
        true
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
            // 批次C/C1：隐藏/系统文件显示开关（config.show_hidden；默认 true）。
            include_hidden: self.config.show_hidden,
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
                // 批次D/D3：列集 = 该窗格 ColumnConfig 可见列按 order
                // （PaneState.columns 是 per-pane 列配置）。
                let cols = visible_ordered_columns(&self.session.panes[index].columns);
                unsafe { populate_list(hwnd, &mut self.icons, &sorted, &cols) };
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
                // 重灌后刷新预览（选中集已变；空选择自动隐藏）。
                self.update_preview_pane(index);
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
    ///
    /// 批次C/C5 修复：探针在 fill_pane 内位于树懒建（1593 行）**之前**执行，
    /// 且 TREE_PROBE_DONE 一次性消费——默认启动（tree_mode=0）与 .qdr
    /// tree_mode=1 启动都会「无盘根」。修法：探针自保底懒建（幂等，
    /// build_tree 内部已建过则直接返回）。
    fn tree_probe_select(&mut self, index: usize) {
        // 保底：树根未建则先建（无论 tree_mode/时序）。
        if self.panes[index].tree_nodes.path_of.is_empty() {
            self.build_tree(index);
        }
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
        // 批次D/D1：路径变化 → 重注册文件夹监视（dirty 标志防抖期内旧路径
        // 的通知到达时，flush 会因路径不符自动跳过）。WM_CREATE 期间
        // （watch_started=false）不注册——首次由 DROP_REGISTER_TIMER 做。
        if self.watch_started {
            self.reregister_watches(self.main);
        }
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

    /// 批次C/C4：历史后退/前进（Alt+Left/Right、菜单 30038/30039）。
    /// 直接用 TabState::back/forward 改 path+pos（**不重复压历史**——与
    /// navigate_to 的压栈语义区分），再 fill_pane 刷新视图与地址栏。
    /// 返回是否真的移动了。
    pub fn go_history(&mut self, pidx: usize, forward: bool) -> bool {
        let tabs_pos = self.session.panes[pidx].tabs_pos;
        let r = if forward {
            self.session.panes[pidx].tabs[tabs_pos].forward()
        } else {
            self.session.panes[pidx].tabs[tabs_pos].back()
        };
        match r {
            Ok(()) => {
                let path = self.session.panes[pidx].tabs[tabs_pos].path.clone();
                let pos = self.session.panes[pidx].tabs[tabs_pos].history_pos;
                let len = self.session.panes[pidx].tabs[tabs_pos].history.len();
                unsafe { self.fill_pane(pidx) };
                println!(
                    "[qdir-rs] TEST 历史移动 窗格{} -> {path:?} (pos={pos}/{len})",
                    pidx + 1
                );
                true
            }
            Err(_) => false,
        }
    }

    /// 批次D/D3：切换某窗格某列的可见性（菜单「查看→列」子项）→ 重灌。
    /// 返回切换后的可见态；未知 key 返回 None。
    /// 注意：列配置在 **PaneState.columns**（per-pane），不在 TabState。
    pub fn toggle_column(&mut self, pidx: usize, key: &str) -> Option<bool> {
        let col = self.session.panes[pidx].columns.columns.iter_mut().find(|c| c.key == key)?;
        col.visible = !col.visible;
        let now = col.visible;
        drop(col);
        unsafe { self.fill_pane(pidx) };
        println!("[qdir-rs] TEST 列显隐 窗格{} {key} -> {now}", pidx + 1);
        Some(now)
    }

    /// 批次D/D2：保存当前会话到布局槽 N（`<exe>/L-Slots/slot-N.qdr`）。
    /// 经 `save_session_with` 注入 IDLIST 路径编码——新文件此前会写出空路径
    /// 会话（FolderN 的 IDLIST 段为空），载入时全部窗格路径丢失。
    pub fn slot_save(&self, n: u32) -> crate::core::QResult<()> {
        let dir = slot_exe_dir()?;
        let p = crate::ui_native::slots_dialog::slot_path(&dir, n);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::core::QError::from_io(&e))?;
        }
        crate::config::save_session_with(&p, &self.session, Some(&qdr_idlist_encoder))
    }

    /// 批次D/D2：载入布局槽 N（替换会话 + splitters 同步 + 四窗格重灌）。
    /// 返回是否成功（文件不存在/解析失败返回 false，界面不动）。
    /// FolderN 的 IDLIST 经 `shell::idlist` 解码后覆盖各窗格路径（批次D/D2 修复：
    /// 此前 load 侧直接丢弃 IDLIST，槽/重启/.qdr 打开一律丢失窗格路径）。
    pub fn slot_load(&mut self, n: u32) -> bool {
        let Ok(dir) = slot_exe_dir() else { return false };
        let p = crate::ui_native::slots_dialog::slot_path(&dir, n);
        match crate::config::load_session_with_idlists(&p) {
            Ok((mut st, idlists)) => {
                apply_idlist_paths(&mut st, &idlists);
                self.session = st;
                self.splitters.from_permille_ten(&self.session.splits_h);
                for i in 0..4 {
                    unsafe { self.fill_pane(i) };
                }
                println!("[qdir-rs] TEST 布局槽载入 槽{n} -> {:?}", p);
                true
            }
            Err(e) => {
                println!("[qdir-rs] TEST 布局槽载入失败 槽{n}：{e}");
                false
            }
        }
    }

    /// 批次D/D2：删除布局槽 N 文件。返回文件是否原本存在。
    pub fn slot_delete(&self, n: u32) -> bool {
        let Ok(dir) = slot_exe_dir() else { return false };
        let p = crate::ui_native::slots_dialog::slot_path(&dir, n);
        match std::fs::remove_file(&p) {
            Ok(()) => {
                println!("[qdir-rs] TEST 布局槽删除 槽{n}");
                true
            }
            Err(_) => false,
        }
    }

    /// 批次D/D2：复制槽 N 文件路径到剪贴板。
    pub fn slot_copy_path(&self, n: u32, main: HWND) -> bool {
        let Ok(dir) = slot_exe_dir() else { return false };
        let p = crate::ui_native::slots_dialog::slot_path(&dir, n);
        let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
        match crate::shell::copy_to_clipboard(host, std::slice::from_ref(&p)) {
            Ok(()) => {
                println!("[qdir-rs] TEST 布局槽复制路径 槽{n} -> {p:?}");
                true
            }
            Err(e) => {
                eprintln!("[qdir-rs] 布局槽复制路径失败：{e}");
                false
            }
        }
    }

    // ===================== 批次D/D1：文件夹监视 =====================

    /// 按四窗格当前路径（重）注册文件夹变更通知（批次D/D1）。
    /// 先释放旧句柄（Deregister + PIDL free）再全量注册；失败只留痕不阻断。
    ///
    /// PIDL 生命周期：shell 不拷贝注册用的 PIDL（调用者拥有），故由
    /// `WatchHandle` 保活到下次重注册/销毁——提前释放会 SIGSEGV（实测）。
    pub fn reregister_watches(&mut self, main: HWND) {
        if let Some(mut h) = self.watch.take() {
            h.release();
        }
        let mut paths: Vec<std::path::PathBuf> = Vec::with_capacity(4);
        for i in 0..4 {
            let path = self.pane_path(i);
            if !path.as_os_str().is_empty() && path.is_dir() {
                paths.push(path);
            }
        }
        if paths.is_empty() {
            return;
        }
        let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
        let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
        match crate::shell::register_change_notify(host, &refs, WM_APP_WATCH) {
            Some(h) => {
                println!("[qdir-rs] TEST 监视注册 {} 路径 id={}", paths.len(), h.id());
                self.watch = Some(h);
                self.watch_started = true;
            }
            None => eprintln!("[qdir-rs] 监视注册失败（{} 路径）", paths.len()),
        }
    }

    /// WM_APP+3 到达：标记全部窗格脏 + 起 200ms 防抖定时器。
    /// （多条目单 id 注册无法从通知参数区分窗格，保守全脏；防抖合并。）
    pub fn on_watch_notify(&mut self, main: HWND) {
        for d in self.watch_dirty.iter_mut() {
            *d = true;
        }
        unsafe {
            let _ = SetTimer(
                Some(main),
                WATCH_TIMER_ID,
                WATCH_DEBOUNCE_MS as u32,
                windows::Win32::UI::WindowsAndMessaging::TIMERPROC::None,
            );
        }
    }

    /// 防抖 timer 到点：重灌脏窗格（仅路径仍存在的）。返回重灌数。
    pub fn flush_watch_dirty(&mut self, main: HWND) -> usize {
        let mut n = 0usize;
        for i in 0..4 {
            if !self.watch_dirty[i] {
                continue;
            }
            self.watch_dirty[i] = false;
            let path = self.pane_path(i);
            if path.as_os_str().is_empty() || !path.is_dir() {
                continue; // 导航已离开该目录：不灌
            }
            unsafe { self.fill_pane(i) };
            n += 1;
        }
        if n > 0 {
            println!("[qdir-rs] TEST 监视刷新 {n} 窗格");
        }
        let _ = main;
        n
    }

    /// 钩子 QDIR_TEST_WATCH=1 → 只登记一次性延迟定时器（300ms），由
    /// WM_TIMER（主消息循环上下文）执行 `watch_probe_run`。
    /// **不能在此直接跑**：探针的有界泵会把 0ms 的 DROP_REGISTER_TIMER 提前到
    /// WM_CREATE 内触发，SHChangeNotifyRegister 在窗口创建期间段错误。
    unsafe fn watch_probe(&mut self, main: HWND) {
        let _ = SetTimer(
            Some(main),
            WATCH_PROBE_TIMER_ID,
            300,
            windows::Win32::UI::WindowsAndMessaging::TIMERPROC::None,
        );
    }

    /// 钩子 QDIR_TEST_WATCH 主体（主循环上下文）：窗格1 导航到临时目录 →
    /// 写第二个文件触发真实 shell 通知（WM_APP_WATCH）→ 有界消息泵等 200ms
    /// 防抖 timer 重灌 → 断言行数 +1；删除文件 → 断言恢复。全链真实
    /// （SHChangeNotifyRegister→PostMessage→wnd_proc→timer→fill_pane）。
    unsafe fn watch_probe_run(&mut self, main: HWND) {
        let dir = std::env::temp_dir().join(format!("qdir-rs-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if std::fs::create_dir_all(&dir).is_err() {
            println!("[qdir-rs] TEST WATCH：建临时目录失败");
            return;
        }
        // 初始内容一个文件；导航过去（WM_CREATE 期间 watch_started=false，
        // navigate_to 不注册——等 DROP_REGISTER_TIMER 到点在消息循环内
        // 首次注册，此处用有界泵等它完成）。
        let m1 = dir.join("w1.txt");
        let m2 = dir.join("w2.txt");
        let _ = std::fs::write(&m1, "1");
        self.navigate_to(0, dir.clone());
        let started = unsafe { bounded_pump(main, || self.watch_started, 3000) };
        if !started {
            println!("[qdir-rs] TEST WATCH：监视首次注册超时");
            return;
        }
        let Some(list) = self.panes[0].list else {
            println!("[qdir-rs] TEST WATCH：窗格1 无列表");
            return;
        };
        let count = || unsafe { SendMessageW(list, LVM_GETITEMCOUNT, None, None).0.max(0) as usize };
        let base = count();
        // 写第二个文件：shell 投递 WM_APP_WATCH（异步）。
        let _ = std::fs::write(&m2, "2");
        let grew = unsafe { bounded_pump(main, || count() > base, 3000) };
        let after_add = count();
        // 删除它：等恢复。
        let _ = std::fs::remove_file(&m2);
        let shrank = unsafe { bounded_pump(main, || count() <= base, 3000) };
        let after_del = count();
        println!(
            "[qdir-rs] TEST WATCH 行数 {base} -> {after_add} (新增生效={grew}) -> {after_del} (删除生效={shrank})"
        );
        assert!(grew && after_add == base + 1, "监视新增未触发重灌");
        assert!(shrank && after_del == base, "监视删除未触发重灌");
        let _ = std::fs::remove_dir_all(&dir);
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
    /// 关闭窗格标签页（中键/双击关闭路径）。core::close_tab 拒绝最后一个
    /// （原版行为）；成功后重建页签控件并重灌窗格。
    pub unsafe fn close_tab(&mut self, pidx: usize, tab_idx: usize) -> crate::core::QResult<()> {
        self.session.panes[pidx].close_tab(tab_idx)?;
        rebuild_pane_tabs(self, pidx);
        self.fill_pane(pidx);
        Ok(())
    }

    unsafe fn tab_select(&mut self, pidx: usize, pos: usize) {
        let pane = &self.session.panes[pidx];
        if pos >= pane.tabs.len() || pos == pane.tabs_pos {
            return;
        }
        self.session.panes[pidx].tabs_pos = pos;
        self.fill_pane(pidx);
    }

    // ===================== 批次A：键盘与文件操作 =====================

    /// 键盘动作路由（子类化 list 角色）。作用域=发键窗格 `pidx`。
    /// 复用 commands.rs 窗格按钮 bi=4/5/6/7 的实现语义（同一 shell API），
    /// 只是作用域从按钮窗格换成发键窗格。返回是否产生了实际操作。
    pub fn keyboard_action(&mut self, pidx: usize, action: KbAction, main: HWND) -> bool {
        let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
        let sel = self.selected_paths(pidx);
        match action {
            KbAction::Copy => {
                if sel.is_empty() {
                    return false;
                }
                match crate::shell::copy_to_clipboard(host, &sel) {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("[qdir-rs] 键盘复制失败：{e}");
                        false
                    }
                }
            }
            KbAction::Cut => {
                if sel.is_empty() {
                    return false;
                }
                match crate::shell::cut_to_clipboard(host, &sel) {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("[qdir-rs] 键盘剪切失败：{e}");
                        false
                    }
                }
            }
            KbAction::Paste => {
                let path = self.pane_path(pidx);
                if path.as_os_str().is_empty() {
                    return false;
                }
                match crate::shell::paste_into(host, &path, false) {
                    Ok(_) => {
                        unsafe { self.fill_pane(pidx) };
                        true
                    }
                    Err(e) => {
                        eprintln!("[qdir-rs] 键盘粘贴失败：{e}");
                        false
                    }
                }
            }
            KbAction::SelectAll => {
                let n = unsafe { self.select_all_rows(pidx) };
                n > 0
            }
            KbAction::Delete => {
                if sel.is_empty() {
                    return false;
                }
                // A2：删除默认进回收站（allow_undo=true）。
                let opts = crate::shell::FileOpOptions {
                    allow_undo: true,
                    ..Default::default()
                };
                match crate::shell::delete_files(host, &sel, &opts) {
                    Ok(_) => {
                        unsafe { self.fill_pane(pidx) };
                        true
                    }
                    Err(e) => {
                        eprintln!("[qdir-rs] 键盘删除失败：{e}");
                        false
                    }
                }
            }
            KbAction::NewTab => {
                let path = self.pane_path(pidx);
                unsafe { self.pane_add_tab(pidx, path) };
                true
            }
        }
    }

    /// Ctrl+A 全选：LVM_SETITEMSTATE(-1, stateMask=LVIS_SELECTED) 把所有行置选中。
    /// 返回被选中的行数（mainframe.rs:1177 现有 LVM_SETITEMSTATE 用法同法）。
    unsafe fn select_all_rows(&mut self, pidx: usize) -> usize {
        let Some(list) = self.panes[pidx].list else {
            return 0;
        };
        let st = LVITEMW {
            mask: LVIF_STATE,
            state: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0),
            stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0),
            ..Default::default()
        };
        // wParam = -1 → 应用到所有项。
        unsafe {
            let _ = SendMessageW(
                list,
                LVM_SETITEMSTATE,
                Some(WPARAM(usize::MAX)),
                Some(LPARAM(&st as *const LVITEMW as isize)),
            );
            let cnt = SendMessageW(list, LVM_GETSELECTEDCOUNT, None, None).0.max(0) as usize;
            let _ = self.refresh_status();
            cnt
        }
    }

    /// F2/菜单进入内联重命名：仅当**恰好选中一行**时对该行发 LVM_EDITLABEL。
    /// 返回是否进入编辑态。
    pub fn start_inline_rename(&mut self, pidx: usize) -> bool {
        let Some(list) = self.panes[pidx].list else {
            return false;
        };
        let sel = self.selected_paths(pidx);
        if sel.len() != 1 {
            return false;
        }
        // 找到选中行号（第一行被选）。
        let row = unsafe {
            SendMessageW(
                list,
                LVM_GETNEXTITEM,
                Some(WPARAM(usize::MAX)),
                Some(LPARAM(LVNI_SELECTED as isize)),
            )
            .0 as i32
        };
        if row < 0 {
            return false;
        }
        let _ = unsafe { SetFocus(Some(list)) };
        // LVM_EDITLABELW：wParam=项索引，返回编辑框 HWND（非空即进入编辑）。
        let edit = unsafe { SendMessageW(list, LVM_EDITLABELW, Some(WPARAM(row as usize)), None) };
        edit.0 != 0
    }

    /// 重命名提交（LVN_ENDLABELEDITW 且新名有效时）：调 shell::rename_path，
    /// 重灌窗格并保持新名选中。返回 (旧路径, 新名, 是否成功)。
    pub fn commit_inline_rename(&mut self, pidx: usize, old_path: std::path::PathBuf, new_name: &str, main: HWND) -> bool {
        let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
        // 纯函数决策：拒绝非法/同名/空。
        let decision = rename_decision(&old_path, new_name);
        if decision != RenameDecision::Ok {
            println!("[qdir-rs] TEST 重命名拒绝 {decision:?} 新名={new_name:?}");
            return false;
        }
        let opts = crate::shell::FileOpOptions::default();
        match crate::shell::rename_path(host, &old_path, new_name, &opts) {
            Ok(_) => {
                let new_path = old_path
                    .parent()
                    .map(|d| d.join(new_name))
                    .unwrap_or_else(|| std::path::PathBuf::from(new_name));
                unsafe { self.fill_pane(pidx) };
                // 保持新名选中。
                if let Some(row) = self.panes[pidx].row_paths.iter().position(|p| *p == new_path) {
                    if let Some(list) = self.panes[pidx].list {
                        unsafe { select_row_if_valid(list, row as i32) };
                    }
                }
                println!(
                    "[qdir-rs] TEST 重命名 {:?} → {new_name:?} 落盘={:?}",
                    old_path
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    new_path.exists()
                );
                true
            }
            Err(e) => {
                eprintln!("[qdir-rs] 重命名失败：{e}");
                false
            }
        }
    }

    /// 新建文件夹（A4）：在活动窗格路径下创建唯一名「新建文件夹[ N]」，
    /// 重灌、选中新文件夹并立即触发内联重命名。返回新建完整路径。
    pub fn new_folder_in_pane(&mut self, pidx: usize, main: HWND) -> Option<std::path::PathBuf> {
        let parent = self.pane_path(pidx);
        if parent.as_os_str().is_empty() {
            println!("[qdir-rs] TEST 新建文件夹：窗格路径为空");
            return None;
        }
        // 纯函数唯一化名（exists 探针走文件系统）。
        let exists = |name: &str| parent.join(name).exists();
        let name = unique_folder_name("新建文件夹", &exists);
        let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
        match crate::shell::new_folder(host, &parent, &name) {
            Ok(created) => {
                unsafe { self.fill_pane(pidx) };
                // 选中新文件夹 + 进入重命名（复用 F2 链）。
                if let Some(row) = self
                    .panes[pidx]
                    .row_paths
                    .iter()
                    .position(|p| *p == created)
                {
                    if let Some(list) = self.panes[pidx].list {
                        unsafe { select_row_if_valid(list, row as i32) };
                    }
                    self.start_inline_rename(pidx);
                }
                println!("[qdir-rs] TEST 新建文件夹 {name:?} 落盘={} -> {:?}", created.exists(), created);
                Some(created)
            }
            Err(e) => {
                eprintln!("[qdir-rs] 新建文件夹失败：{e}");
                None
            }
        }
    }

    /// 新建标签页（A5）：窗格当前路径加新 tab、重建页签条、重灌窗格。
    pub unsafe fn pane_add_tab(&mut self, pidx: usize, path: std::path::PathBuf) {
        self.session.panes[pidx].add_tab(path);
        rebuild_pane_tabs(self, pidx);
        self.fill_pane(pidx);
        let tabs = self.session.panes[pidx].tabs.len();
        println!("[qdir-rs] TEST 新建标签页 窗格{} tabs={}", pidx + 1, tabs);
    }

    /// ComboBox 选盘导航（A6）：读 CB_GETCURSEL → 取该选项文本 → 解析出
    /// 驱动器路径 → navigate_to。`sel_path` 已由调用方从 CB_GETLBTEXT 取出。
    pub fn combo_navigate(&mut self, pidx: usize, sel_path: &str) -> bool {
        let path = combo_sel_to_path(sel_path);
        match path {
            Some(p) => {
                println!("[qdir-rs] TEST 选盘导航 窗格{} {sel_path:?} -> {p:?}", pidx + 1);
                self.navigate_to(pidx, p);
                true
            }
            None => false,
        }
    }

    /// 钩子 QDIR_TEST_KEYS：选中窗格1第0行 → 向列表投递 Ctrl+C（同进程
    /// SendMessageW）→ 断言剪贴板读回 CF_HDROP 含该行路径；再投递 Ctrl+A →
    /// 用 LVM_GETSELECTEDCOUNT 断言计数==行数。全程进程内自证（R4）。
    unsafe fn keys_probe(&mut self, main: HWND) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{VK_A, VK_C};
        let Some(list) = self.panes[0].list else {
            println!("[qdir-rs] TEST KEYS 钩子：窗格1 无列表");
            return;
        };
        let rows = unsafe { SendMessageW(list, LVM_GETITEMCOUNT, None, None).0 as usize };
        if rows == 0 {
            println!("[qdir-rs] TEST KEYS 钩子：窗格1 空目录，跳过");
            return;
        }
        // 选中第 0 行（清空其余选中）。
        unsafe { select_row_if_valid(list, 0) };
        let want = self.panes[0].row_paths.get(0).cloned().unwrap_or_default();
        // 1) Ctrl+C：经子类化 list 角色走真实 WM_KEYDOWN 链。
        unsafe { crate::ui_native::subclass::synth_ctrl_key(list, VK_C.0 as u32) };
        let clip = crate::shell::get_clipboard();
        match clip {
            Ok(c) => {
                let hit = c.paths.iter().any(|p| *p == want);
                println!(
                    "[qdir-rs] TEST KEYS Ctrl+C 剪贴板 {} 项 cut={} 命中={hit} 目标={:?}",
                    c.len(),
                    c.cut,
                    want
                );
                assert!(hit, "Ctrl+C 后剪贴板未含选中路径 {want:?}");
            }
            Err(e) => panic!("Ctrl+C 后剪贴板读回失败：{e}"),
        }
        // 2) Ctrl+A：全选，断言选中计数==行数。
        unsafe { crate::ui_native::subclass::synth_ctrl_key(list, VK_A.0 as u32) };
        let sel = unsafe { SendMessageW(list, LVM_GETSELECTEDCOUNT, None, None).0 as usize };
        println!("[qdir-rs] TEST KEYS Ctrl+A 选中={sel} 总行={rows}");
        assert_eq!(sel, rows, "Ctrl+A 后选中计数({sel}) != 行数({rows})");
        let _ = main;
    }

    /// 钩子 QDIR_TEST_RENAME：三段验证（R4 行为断言）——
    /// ① commit_inline_rename 直调：临时目录建文件 → 选中 → 改名落盘；
    /// ② 合成 WM_NOTIFY/LVN_ENDLABELEDITW：走真实通知处理器再改一次名；
    /// ③ F2 通道：合成 Ctrl 位为 0 的 WM_KEYDOWN/F2 → 子类化 → LVM_EDITLABEL，
    ///    断言 LVM_GETEDITCONTROL 出现编辑框，ESC 取消不动磁盘。
    unsafe fn rename_probe(&mut self, main: HWND) {
        use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
        use windows::Win32::UI::Controls::{NMLVDISPINFOW, LVM_GETEDITCONTROL};
        let dir = std::env::temp_dir().join(format!("qdir-rs-rename-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let old_name = "重命名前.txt";
        let new_name = "重命名后.txt";
        let new2_name = "重命名后2.txt";
        let old_path = dir.join(old_name);
        let new_path = dir.join(new_name);
        let new2_path = dir.join(new2_name);
        let _ = std::fs::remove_file(&new_path);
        let _ = std::fs::remove_file(&new2_path);
        std::fs::write(&old_path, "rename-me").expect("写测试文件失败");
        // 窗格1 导航到临时目录并重灌。
        self.navigate_to(0, dir.clone());
        // 找到该文件所在行并选中。
        let row = self
            .panes[0]
            .row_paths
            .iter()
            .position(|p| *p == old_path);
        let Some(r) = row else {
            println!("[qdir-rs] TEST RENAME 钩子：未在窗格1 找到 {old_path:?}");
            return;
        };
        let list = self.panes[0].list;
        if let Some(list) = list {
            unsafe { select_row_if_valid(list, r as i32) };
        }
        // ① 直调提交链。
        let ok = self.commit_inline_rename(0, old_path.clone(), new_name, main);
        let disk_new = new_path.exists();
        let disk_old_gone = !old_path.exists();
        println!(
            "[qdir-rs] TEST RENAME ①提交={ok} 新名落盘={disk_new} 旧名消失={disk_old_gone}"
        );
        assert!(ok && disk_new && disk_old_gone, "重命名直调链失败");

        // ② 合成 LVN_ENDLABELEDITW 通知（真实处理器；PENDING 槽已空→回落行路径）。
        if let Some(list) = list {
            let row2 = self.panes[0].row_paths.iter().position(|p| *p == new_path);
            if let Some(r2) = row2 {
                let mut wz: Vec<u16> = new2_name.encode_utf16().chain(std::iter::once(0)).collect();
                let mut nm = NMLVDISPINFOW::default();
                nm.hdr = windows::Win32::UI::Controls::NMHDR {
                    hwndFrom: list,
                    idFrom: (ID_FIRST_PANE + CID_LIST) as usize,
                    code: LVN_ENDLABELEDITW,
                };
                nm.item.iItem = r2 as i32;
                nm.item.pszText = windows::core::PWSTR::from_raw(wz.as_mut_ptr());
                unsafe {
                    self.on_notify(main, LPARAM(&mut nm as *mut NMLVDISPINFOW as isize));
                }
                let d2 = new2_path.exists();
                let d1gone = !new_path.exists();
                println!("[qdir-rs] TEST RENAME ②通知链 新名2落盘={d2} 新名1消失={d1gone}");
                assert!(d2 && d1gone, "LVN_ENDLABELEDITW 通知处理器失败");
            }
        }

        // ③ F2 通道：合成无 Ctrl 的 WM_KEYDOWN/F2 → 子类化 → LVM_EDITLABEL。
        self.navigate_to(0, dir.clone());
        let row3 = self.panes[0].row_paths.iter().position(|p| *p == new2_path);
        if let (Some(r3), Some(list)) = (row3, list) {
            unsafe {
                select_row_if_valid(list, r3 as i32);
                let _ = crate::ui_native::subclass::synth_plain_key(list, windows::Win32::UI::Input::KeyboardAndMouse::VK_F2.0 as u32);
                let edit = SendMessageW(list, LVM_GETEDITCONTROL, None, None);
                let opened = edit.0 != 0;
                println!("[qdir-rs] TEST RENAME ③F2 编辑框出现={opened}");
                assert!(opened, "F2 未进入内联编辑（LVS_EDITLABELS/LVM_EDITLABEL 链断）");
                // ESC 取消：不产生磁盘变化。
                if opened {
                    let eh = HWND(edit.0 as *mut core::ffi::c_void);
                    let _ = SendMessageW(eh, WM_KEYDOWN, Some(WPARAM(VK_ESCAPE.0 as usize)), None);
                }
                let unchanged = new2_path.exists();
                println!("[qdir-rs] TEST RENAME ③取消后原名保持={unchanged}");
                assert!(unchanged, "取消编辑不应破坏文件");
            }
        }
        // 清理。
        let _ = std::fs::remove_file(&new2_path);
        let _ = std::fs::remove_file(&new_path);
        let _ = std::fs::remove_file(&old_path);
        let _ = std::fs::remove_dir(&dir);
    }

    /// 钩子 QDIR_TEST_OPS：临时目录内验证 A4 新建文件夹（唯一名落盘 + 二次
    /// 调用得到「新建文件夹 2」）、A5 新建标签页（tab 计数 +1）、A2 Del 进
    /// 回收站（源从磁盘消失）。全程磁盘/状态级断言（R4）。
    unsafe fn ops_probe(&mut self, main: HWND) {
        let dir = std::env::temp_dir().join(format!("qdir-rs-ops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        // 目标文件（待 Del 送回收站）。
        let victim = dir.join("待删.txt");
        std::fs::write(&victim, "x").expect("写测试文件失败");
        // 窗格1 导航到临时目录。
        self.navigate_to(0, dir.clone());

        // A4 新建文件夹：唯一名应落盘；再建一次应得到「新建文件夹 2」。
        let f1 = self.new_folder_in_pane(0, main);
        let f1_name = f1.as_ref().map(|p| p.file_name().unwrap().to_string_lossy().into_owned());
        let f1_ok = f1.as_ref().map(|p| p.is_dir()).unwrap_or(false);
        // 重灌后新文件夹已存在；再建一次验证唯一化。
        let f2 = self.new_folder_in_pane(0, main);
        let f2_name = f2.as_ref().map(|p| p.file_name().unwrap().to_string_lossy().into_owned());
        println!(
            "[qdir-rs] TEST OPS 新建文件夹 首个={f1_name:?} 落盘={f1_ok} 次个={f2_name:?}"
        );
        assert!(f1_ok, "新建文件夹未落盘");
        assert_eq!(f1_name.as_deref(), Some("新建文件夹"));
        assert_eq!(f2_name.as_deref(), Some("新建文件夹 2"), "唯一化回落失败");

        // A5 新建标签页：tabs 计数 +1。
        let before = self.session.panes[0].tabs.len();
        unsafe { self.pane_add_tab(0, dir.clone()) };
        let after = self.session.panes[0].tabs.len();
        println!("[qdir-rs] TEST OPS 新建标签页 tabs {before} -> {after}");
        assert_eq!(after, before + 1, "新建标签页未增计数");
        // 回到原标签页做删除验证（add_tab 切到了新页，路径相同但 tabs_pos 变了）。
        self.session.panes[0].tabs_pos = 0;
        self.navigate_to(0, dir.clone());

        // A2 Del 进回收站：选中 victim → 投递裸 WM_KEYDOWN/VK_DELETE（bit24=0，
        // 无 Ctrl），走子类化全链（subclass → keyboard_action → 回收站删除）。
        let row = self.panes[0].row_paths.iter().position(|p| *p == victim);
        match row {
            Some(r) => {
                if let Some(list) = self.panes[0].list {
                    unsafe {
                        select_row_if_valid(list, r as i32);
                        crate::ui_native::subclass::synth_plain_key(
                            list,
                            windows::Win32::UI::Input::KeyboardAndMouse::VK_DELETE.0 as u32,
                        );
                    }
                }
                let gone = !victim.exists();
                println!("[qdir-rs] TEST OPS Del(全链) 源消失(进回收站)={gone}");
                assert!(gone, "Del 删除未使源从磁盘消失");
            }
            None => println!("[qdir-rs] TEST OPS：未找到待删行 {victim:?}"),
        }

        // 清理（回收站里的待删项交给系统回收站；临时目录整体删）。
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 钩子 QDIR_TEST_CLIP：临时 src 建文件 → Ctrl+X 剪切(断言 cut=true) →
    /// 导航到 dst → Ctrl+V 粘贴 → 断言文件移动到 dst（src 消失、dst 存在）。
    /// 走真实剪贴板 CF_HDROP + Preferred DropEffect 读写链（R4）。
    unsafe fn clip_probe(&mut self, main: HWND) {
        let base = std::env::temp_dir().join(format!("qdir-rs-clip-{}", std::process::id()));
        let src_dir = base.join("src");
        let dst_dir = base.join("dst");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&src_dir).expect("建 src 失败");
        std::fs::create_dir_all(&dst_dir).expect("建 dst 失败");
        let file = src_dir.join("待剪切.txt");
        std::fs::write(&file, "clip").expect("写测试文件失败");

        // 窗格1 导航到 src，选中该文件，Ctrl+X 剪切。
        self.navigate_to(0, src_dir.clone());
        let row = self.panes[0].row_paths.iter().position(|p| *p == file);
        let Some(r) = row else {
            println!("[qdir-rs] TEST CLIP：src 未找到待剪切行");
            return;
        };
        if let Some(list) = self.panes[0].list {
            unsafe { select_row_if_valid(list, r as i32) };
        }
        self.keyboard_action(0, KbAction::Cut, main);
        let clip = crate::shell::get_clipboard();
        let cut_flag = clip.as_ref().map(|c| c.cut).unwrap_or(false);
        println!(
            "[qdir-rs] TEST CLIP Ctrl+X cut={cut_flag} 命中={}",
            clip.as_ref()
                .map(|c| c.paths.iter().any(|p| *p == file))
                .unwrap_or(false)
        );
        assert!(cut_flag, "Ctrl+X 后剪贴板 DropEffect 应为 Move(cut=true)");

        // 导航到 dst，Ctrl+V 粘贴。
        self.navigate_to(0, dst_dir.clone());
        self.keyboard_action(0, KbAction::Paste, main);
        let moved = dst_dir.join("待剪切.txt");
        let dst_ok = moved.exists();
        let src_gone = !file.exists();
        println!("[qdir-rs] TEST CLIP Ctrl+V 落盘={dst_ok} 源消失(移动)={src_gone}");
        assert!(dst_ok && src_gone, "Ctrl+X/V 未把文件移动到目标目录");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 钩子 QDIR_TEST_COMBO：对窗格1 驱动器 ComboBox 逐项 CB_SETCURSEL 并
    /// 模拟 CBN_SELCHANGE（同进程 SendMessageW WM_COMMAND），断言窗格路径
    /// 跟随切换到对应盘根（R4）。选第一个存在的本地盘符作目标。
    unsafe fn combo_probe(&mut self, main: HWND) {
        let Some(cb) = self.panes[0].drive_combo else {
            println!("[qdir-rs] TEST COMBO：窗格1 无 ComboBox");
            return;
        };
        let count = unsafe { SendMessageW(cb, CB_GETCOUNT, None, None).0 as i32 };
        if count <= 0 {
            println!("[qdir-rs] TEST COMBO：驱动器下拉为空");
            return;
        }
        // 找一个盘符项（文本形如「… (X:)」），选它并触发 CBN_SELCHANGE。
        let combo_id = ID_FIRST_PANE + CID_COMBO_DRIVES; // 窗格1 ComboBox 控件 id
        for idx in 0..count {
            let len = unsafe { SendMessageW(cb, CB_GETLBTEXTLEN, Some(WPARAM(idx as usize)), None).0 } as usize;
            let mut buf = vec![0u16; len + 1];
            unsafe {
                SendMessageW(
                    cb,
                    CB_GETLBTEXT,
                    Some(WPARAM(idx as usize)),
                    Some(LPARAM(buf.as_mut_ptr() as isize)),
                );
            }
            let text = String::from_utf16_lossy(
                &buf[..buf.iter().position(|&c| c == 0).unwrap_or(buf.len())],
            );
            let Some(target) = combo_sel_to_path(&text) else { continue };
            if !target.is_dir() {
                continue;
            }
            // 设当前项 + 发 CBN_SELCHANGE（notify=1，wParam=id|CBN<<16）。
            unsafe { SendMessageW(cb, CB_SETCURSEL, Some(WPARAM(idx as usize)), None) };
            let wp = WPARAM((combo_id as usize) | ((CBN_SELCHANGE as usize) << 16));
            unsafe { SendMessageW(main, WM_COMMAND, Some(wp), Some(LPARAM(cb.0 as isize))) };
            let cur = self.pane_path(0);
            println!("[qdir-rs] TEST COMBO 选 {text:?} -> 窗格路径 {cur:?}");
            assert_eq!(cur, target, "CBN_SELCHANGE 后窗格未导航到 {target:?}");
            return;
        }
        println!("[qdir-rs] TEST COMBO：无可用盘符项");
    }

    /// 钩子 QDIR_TEST_VIEW_LARGE：窗格1 切 LV_VIEW_ICON（=0），回读
    /// LVM_GETVIEW 与 LVM_GETIMAGELIST(LVSIL_NORMAL)，断言大图列表已挂。
    unsafe fn view_large_probe(&self) {
        let Some(lv) = self.panes[0].list else {
            println!("[qdir-rs] TEST VIEW_LARGE：窗格1 无列表");
            return;
        };
        let set = unsafe { SendMessageW(lv, LVM_SETVIEW, Some(WPARAM(0)), None) };
        let got = unsafe { SendMessageW(lv, LVM_GETVIEW, None, None).0 as u32 };
        let himl = unsafe {
            SendMessageW(lv, LVM_GETIMAGELIST, Some(WPARAM(LVSIL_NORMAL as usize)), None)
        };
        println!(
            "[qdir-rs] TEST VIEW_LARGE SETVIEW={} GETVIEW={got} NORMAL_himl={}",
            set.0 != 0,
            himl.0 != 0
        );
        assert_eq!(got, 0, "大图标视图回读不一致（LV_VIEW_ICON=0）");
        assert!(himl.0 != 0, "LVSIL_NORMAL 大图标图像列表为空");
    }

    // ===================== 批次B/B3：OLE 落点与拖入消费 =====================

    /// 屏幕坐标 → 命中窗格索引（WM_DROPFILES 兜底用；无命中回落 0）。
    fn pane_at_screen_point(&self, pt: (i32, i32)) -> usize {
        let mut rects: Vec<(i32, i32, i32, i32)> = Vec::with_capacity(4);
        for p in &self.panes {
            if let Some(list) = p.list {
                let mut rc = RECT::default();
                if unsafe { GetWindowRect(list, &mut rc) }.is_ok() {
                    rects.push((rc.left, rc.top, rc.right, rc.bottom));
                }
            }
        }
        pane_hit_index(&rects, pt).unwrap_or(0)
    }

    /// 每窗格 list 建窗后调用（create_pane）：注册 OLE 落点对象并开
    /// WM_DROPFILES 兜底。注册失败只留痕（WM_DROPFILES 仍兜底）。
    ///
    /// **时机约束**：禁止在 WM_CREATE 期间调用（ole32 会以 0xC000041D
    /// 致命回调异常崩）；必须由 WM_APP_REGISTER_DROP（窗口创建完成后）驱动。
    unsafe fn register_all_drop_targets(&mut self) {
        for index in 0..4 {
            let Some(list) = self.panes[index].list else { continue };
            // 兜底：DragAcceptFiles（OLE 注册成功时 Explorer 走 OLE，不发
            // WM_DROPFILES；两条路径自然互斥）。
            crate::shell::accept_drop_files(
                crate::shell::HostWindow::from_raw_hwnd(list.0 as isize),
                true,
            );
            // 先入 frame（Box 地址自此稳定），再取其指针注册。
            self.panes[index].drop_target =
                Some(crate::shell::PaneDropTarget::new(index, self.main.0 as isize));
            let ptr = self.panes[index]
                .drop_target
                .as_ref()
                .expect("刚落盘的 drop target")
                .as_interface_ptr();
            let host = crate::shell::HostWindow::from_raw_hwnd(list.0 as isize);
            match crate::shell::register_drop_target(host, ptr) {
                Ok(()) => println!("[qdir-rs] TEST 拖放落点注册 窗格{}", index + 1),
                Err(e) => {
                    // 注册失败：撤对象保活，回落 WM_DROPFILES。
                    self.panes[index].drop_target = None;
                    println!(
                        "[qdir-rs] 拖放落点注册失败 窗格{}（回落 WM_DROPFILES）：{e}",
                        index + 1
                    );
                }
            }
        }
    }

    /// 反注册所有窗格的 OLE 落点（WM_DESTROY 路径）。
    unsafe fn revoke_pane_drop_targets(&mut self) {
        for p in self.panes.iter_mut() {
            if let Some(list) = p.list {
                let host = crate::shell::HostWindow::from_raw_hwnd(list.0 as isize);
                let _ = crate::shell::revoke_drop_target(host);
            }
            p.drop_target = None;
        }
    }

    /// 消费拖放队列（WM_APP+2 处理与钩子共用）：逐条执行复制/移动并重灌。
    fn consume_dropped(&mut self, main: HWND) {
        for d in crate::shell::take_dropped() {
            self.on_dropped_paths(main, d.pane_index, &d.paths, d.effect, d.screen_pt);
        }
    }

    /// 拖放落点执行：dest=该窗格当前 tab.path；effect 决议（纯函数预检）→
    /// copy_files/move_files → 重灌窗格。同目录/自拖拒绝（纯函数）。
    fn on_dropped_paths(
        &mut self,
        main: HWND,
        pidx: usize,
        paths: &[std::path::PathBuf],
        effect: u32,
        screen_pt: (i32, i32),
    ) {
        if pidx >= 4 {
            return;
        }
        let dest = self.pane_path(pidx);
        let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
        match crate::shell::drop_reject_reason(paths, &dest) {
            Some(reason) => {
                println!(
                    "[qdir-rs] TEST 拖放拒绝({reason:?}) 窗格{} dest={dest:?} pt={screen_pt:?}",
                    pidx + 1
                );
                return;
            }
            None => {}
        }
        let move_op = crate::shell::allows_move(effect);
        let opts = crate::shell::FileOpOptions {
            allow_undo: move_op,
            ..Default::default()
        };
        let r = if move_op {
            crate::shell::move_files(host, paths, &dest, &opts)
        } else {
            crate::shell::copy_files(host, paths, &dest, &opts)
        };
        match r {
            Ok(rep) => {
                unsafe { self.fill_pane(pidx) };
                println!(
                    "[qdir-rs] TEST 拖放落点 窗格{} {} {} 项 -> {:?} pt={:?}",
                    pidx + 1,
                    if move_op { "移动" } else { "复制" },
                    rep.succeeded,
                    dest,
                    screen_pt
                );
            }
            Err(e) => eprintln!("[qdir-rs] 拖放落点失败：{e}"),
        }
    }

    /// 钩子 QDIR_TEST_DND：进程内经丢弃队列向窗格1 注入（不触达 COM 模态
    /// DoDragDrop——那部分 *待实测*）：临时目录建源文件，push 队列 +
    /// SendMessage(WM_APP+2) 同步触发主线程消费，断言窗格1 目录落盘。
    unsafe fn dnd_probe(&mut self, main: HWND) {
        let dir = std::env::temp_dir().join(format!("qdir-rs-dnd-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let src_dir = dir.join("src");
        let dst_dir = dir.join("dst");
        if std::fs::create_dir_all(&src_dir).is_err() || std::fs::create_dir_all(&dst_dir).is_err() {
            println!("[qdir-rs] TEST DND：建临时目录失败");
            return;
        }
        let file = src_dir.join("拖入.txt");
        if std::fs::write(&file, "dnd").is_err() {
            println!("[qdir-rs] TEST DND：写测试文件失败");
            return;
        }
        // 窗格1 导航到 dst 作为落点目录。
        self.navigate_to(0, dst_dir.clone());
        // 队列注入：MOVE 效果（DROPEFFECT_MOVE=2）。
        crate::shell::push_dropped(crate::shell::DroppedPaths {
            pane_index: 0,
            paths: vec![file.clone()],
            effect: crate::shell::DROPEFFECT_MOVE_VALUE,
            screen_pt: (0, 0),
        });
        // SendMessage 同线程同步执行 WM_APP+2 处理。
        unsafe { SendMessageW(main, WM_APP_DROP, None, None) };
        let landed = dst_dir.join("拖入.txt");
        let src_gone = !file.exists();
        println!(
            "[qdir-rs] TEST DND 队列注入 落盘={} 源消失(移动)={}",
            landed.exists(),
            src_gone
        );
        assert!(landed.exists() && src_gone, "拖放队列注入未使文件落盘");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 钩子 QDIR_TEST_DROPFILES：进程内构造 DROPFILES HGLOBAL 并向主窗
    /// SendMessage WM_DROPFILES（OLE 未生效时的兜底路径；坐标取窗格1 list
    /// 中心），断言 pane_at_screen_point 命中 + on_dropped_paths 落盘。
    unsafe fn dropfiles_probe(&mut self, main: HWND) {
        let dir = std::env::temp_dir().join(format!("qdir-rs-dropfiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let src_dir = dir.join("src");
        let dst_dir = dir.join("dst");
        if std::fs::create_dir_all(&src_dir).is_err() || std::fs::create_dir_all(&dst_dir).is_err() {
            println!("[qdir-rs] TEST DROPFILES：建临时目录失败");
            return;
        }
        let file = src_dir.join("兜底拖入.txt");
        if std::fs::write(&file, "wm").is_err() {
            println!("[qdir-rs] TEST DROPFILES：写测试文件失败");
            return;
        }
        // 窗格1 导航到 dst 作为落点目录。
        self.navigate_to(0, dst_dir.clone());
        // 落点坐标：窗格1 list 的屏幕中心（pane_at_screen_point 应命中它）。
        let (cx, cy) = self
            .panes[0]
            .list
            .and_then(|lv| {
                let mut rc = RECT::default();
                unsafe { GetWindowRect(lv, &mut rc) }
                    .ok()
                    .map(|_| ((rc.left + rc.right) / 2, (rc.top + rc.bottom) / 2))
            })
            .unwrap_or((0, 0));
        // 构造 DROPFILES HGLOBAL（20 字节头 + 宽串 + 双 NUL；同进程传指针）。
        let mut wides: Vec<u16> = file
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        wides.push(0);
        let total = 20 + wides.len() * 2;
        let Some(hglobal) = (unsafe {
            windows::Win32::System::Memory::GlobalAlloc(
                windows::Win32::System::Memory::GMEM_MOVEABLE
                    | windows::Win32::System::Memory::GMEM_ZEROINIT,
                total,
            )
        })
        .ok() else {
            println!("[qdir-rs] TEST DROPFILES：GlobalAlloc 失败");
            return;
        };
        {
            let ptr = unsafe { windows::Win32::System::Memory::GlobalLock(hglobal) } as *mut u8;
            if ptr.is_null() {
                println!("[qdir-rs] TEST DROPFILES：GlobalLock 失败");
                return;
            }
            unsafe {
                (ptr as *mut u32).write(20);
                (ptr.add(16) as *mut u32).write(1); // fWide
                // DROPFILES.pt（offset 4，POINT：x/y 各 i32）写落点坐标，
                // 让 DragQueryPoint 坐标链路真实可测（而非全零兜底）。
                (ptr.add(4) as *mut i32).write(cx);
                (ptr.add(8) as *mut i32).write(cy);
                std::ptr::copy_nonoverlapping(wides.as_ptr(), ptr.add(20) as *mut u16, wides.len());
                let _ = windows::Win32::System::Memory::GlobalUnlock(hglobal);
            }
        }
        let pt_lp = LPARAM((cx as isize & 0xFFFF) | ((cy as isize & 0xFFFF) << 16));
        // SendMessage 同进程同步走 WM_DROPFILES 分支（DropFiles::from_raw 会
        // DragFinish 释放我们自造的句柄——DragFinish 即 GlobalFree 语义）。
        unsafe {
            SendMessageW(
                main,
                WM_DROPFILES,
                Some(WPARAM(hglobal.0 as isize as usize)),
                Some(pt_lp),
            );
        }
        let landed = dst_dir.join("兜底拖入.txt");
        let src_kept = file.exists(); // WM_DROPFILES 默认 COPY：源保留。
        println!(
            "[qdir-rs] TEST DROPFILES 兜底链 落盘={} 源保留(复制语义)={} pt=({cx},{cy})",
            landed.exists(),
            src_kept
        );
        assert!(
            landed.exists() && src_kept,
            "WM_DROPFILES 兜底链未使文件落盘（复制语义）"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 钩子 QDIR_TEST_AUTOCOMPLETE（批次C/C2）：向窗格1 地址栏写入半个
    /// 路径 `C:\Win`，断言：①SHAutoComplete 挂载调用已成功（日志行）；
    /// ②SetWindowTextW/GetWindowTextW 同进程读回一致；③窗口进程存活
    /// （探针跑完即证明）。下拉列表内容为 *系统行为*，不做断言。
    unsafe fn autocomplete_probe(&mut self, main: HWND) {
        let Some(e) = self.panes[0].addr_edit else {
            println!("[qdir-rs] TEST AUTOCOMPLETE：窗格1 无地址栏");
            return;
        };
        let half = "C:\\Win";
        let wz: Vec<u16> = half.encode_utf16().chain(std::iter::once(0)).collect();
        let set_ok = unsafe { SetWindowTextW(e, PCWSTR::from_raw(wz.as_ptr())) };
        let mut back = [0u16; 64];
        let n = unsafe { GetWindowTextW(e, &mut back) };
        let text = String::from_utf16_lossy(&back[..n.max(0) as usize]);
        println!(
            "[qdir-rs] TEST AUTOCOMPLETE 写入={set_ok:?} 读回={:?} 一致={}",
            text,
            text == half
        );
        assert!(text == half, "地址栏读回不一致（SHAutoComplete 不应破坏输入）");
        let _ = main;
    }

    /// 钩子 QDIR_TEST_COLS（批次D/D3）：切「扩展名」列 → 列头计数
    /// N→N-1（LVM_GETHEADER→HDM_GETITEMCOUNT），切回 → N。
    unsafe fn cols_probe(&mut self) {
        let Some(lv) = self.panes[0].list else {
            println!("[qdir-rs] TEST COLS：窗格1 无列表");
            return;
        };
        let header_count = |count_hwnd: HWND| -> usize {
            let hdr = unsafe { SendMessageW(count_hwnd, LVM_GETHEADER, None, None) };
            if hdr.0 == 0 {
                return 0;
            }
            unsafe {
                SendMessageW(
                    HWND(hdr.0 as *mut core::ffi::c_void),
                    HDM_GETITEMCOUNT,
                    None,
                    None,
                )
                .0 as usize
            }
        };
        let before = header_count(lv);
        self.toggle_column(0, "Ext");
        let after = header_count(lv);
        self.toggle_column(0, "Ext");
        let restored = header_count(lv);
        println!("[qdir-rs] TEST COLS 列头 {before} -> {after} -> {restored}");
        assert_eq!(after, before.saturating_sub(1), "切扩展名未减一列");
        assert_eq!(restored, before, "切回未复原列数");
    }

    /// 钩子 QDIR_TEST_SLOTS（批次D/D2）：保存/载入/删除槽1 全链断言。
    unsafe fn slots_probe(&mut self) {
        let dir = std::env::temp_dir().join(format!("qdir-rs-slot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if std::fs::create_dir_all(&dir).is_err() {
            println!("[qdir-rs] TEST SLOTS：建临时目录失败");
            return;
        }
        // 清旧槽1，保证断言从干净状态开始。
        let _ = self.slot_delete(1);
        // 导航到临时目录 → 保存槽1。
        self.navigate_to(0, dir.clone());
        let saved = self.slot_save(1);
        let slot_file = slot_exe_dir()
            .ok()
            .map(|d| crate::ui_native::slots_dialog::slot_path(&d, 1))
            .map(|p| p.exists())
            .unwrap_or(false);
        println!("[qdir-rs] TEST SLOTS 保存={saved:?} 槽文件存在={slot_file}");
        assert!(saved.is_ok() && slot_file, "保存槽1 失败");
        // 导航别处 → 载入槽1 → 断言路径恢复。
        self.navigate_to(0, std::path::PathBuf::from(r"C:\"));
        let loaded = self.slot_load(1);
        let tabs_pos = self.session.panes[0].tabs_pos;
        let path = self.session.panes[0].tabs[tabs_pos].path.clone();
        println!("[qdir-rs] TEST SLOTS 载入={loaded} 路径恢复={} ({path:?})", path == dir);
        assert!(loaded && path == dir, "载入槽1 未恢复会话路径");
        // 删除槽1 → 文件消失。
        let deleted = self.slot_delete(1);
        let gone = slot_exe_dir()
            .ok()
            .map(|d| !crate::ui_native::slots_dialog::slot_path(&d, 1).exists())
            .unwrap_or(false);
        println!("[qdir-rs] TEST SLOTS 删除={deleted} 文件消失={gone}");
        assert!(deleted && gone, "删除槽1 失败");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 钩子 QDIR_TEST_HISTORY（批次C/C4）：窗格1 合成两级导航
    /// （C:\ → C:\Windows），后退断言 pos 递减+地址栏=C:\，前进断言
    /// pos 复原+地址栏=C:\Windows。走 go_history 与菜单/加速键同一函数。
    unsafe fn history_probe(&mut self, main: HWND) {
        self.navigate_to(0, std::path::PathBuf::from(r"C:\"));
        self.navigate_to(0, std::path::PathBuf::from(r"C:\Windows"));
        let addr_text = |frame: &MainFrame| -> String {
            let Some(e) = frame.panes[0].addr_edit else { return String::new() };
            let mut buf = [0u16; 260];
            let n = unsafe { GetWindowTextW(e, &mut buf) };
            String::from_utf16_lossy(&buf[..n.max(0) as usize])
        };
        let pos0 = self.session.panes[0].tabs[self.session.panes[0].tabs_pos].history_pos;
        let back_ok = self.go_history(0, false);
        let pos1 = self.session.panes[0].tabs[self.session.panes[0].tabs_pos].history_pos;
        let addr_back = addr_text(self);
        let fwd_ok = self.go_history(0, true);
        let pos2 = self.session.panes[0].tabs[self.session.panes[0].tabs_pos].history_pos;
        let addr_fwd = addr_text(self);
        println!(
            "[qdir-rs] TEST HISTORY 后退={back_ok} pos {pos0}->{pos1} 地址栏={addr_back:?}；前进={fwd_ok} pos->{pos2} 地址栏={addr_fwd:?}"
        );
        assert!(back_ok && pos1 + 1 == pos0, "后退未使 history_pos 递减");
        assert_eq!(addr_back.trim_end_matches('\\'), r"C:", "后退后地址栏未回退");
        assert!(fwd_ok && pos2 == pos0, "前进未使 history_pos 复原");
        assert!(
            addr_fwd.to_ascii_lowercase().contains("windows"),
            "前进后地址栏未回到 Windows"
        );
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
        // 内联重命名通知（LVN_BEGIN/ENDLABELEDITW，list 角色）。
        if role == Src::List {
            if hdr.code == LVN_BEGINLABELEDITW {
                // 记录待改名的旧路径（此刻行文本仍是旧名）到线程局部槽。
                let nm = unsafe { &*(lp.0 as *const windows::Win32::UI::Controls::NMLVDISPINFOW) };
                let row = nm.item.iItem.max(0) as usize;
                let old = self.panes[pidx].row_paths.get(row).cloned();
                PENDING_RENAME_PATH.with(|s| *s.borrow_mut() = old);
                return LRESULT(0); // 放行编辑
            }
            if hdr.code == LVN_ENDLABELEDITW {
                let old = PENDING_RENAME_PATH.with(|s| s.borrow_mut().take()).unwrap_or_default();
                let nm = unsafe { &*(lp.0 as *const windows::Win32::UI::Controls::NMLVDISPINFOW) };
                // pszText == NULL → 用户取消（ESC），不落盘（返回 1 让控件保留旧名）。
                if nm.item.pszText.is_null() {
                    return LRESULT(1);
                }
                let buf = String::from_utf16_lossy(unsafe { nm.item.pszText.as_wide() });
                let row = nm.item.iItem.max(0) as usize;
                let src = if old.as_os_str().is_empty() {
                    self.panes[pidx].row_paths.get(row).cloned().unwrap_or_default()
                } else {
                    old
                };
                if !buf.is_empty() && !src.as_os_str().is_empty() {
                    self.commit_inline_rename(pidx, src, &buf, main);
                }
                return LRESULT(1);
            }
        }
        // 树通知：懒加载/选中/复选框（批次A 拆到 on_tree_notify，语义不变）。
        if role == Src::Tree {
            return unsafe { self.on_tree_notify(pidx, hdr.code, lp) };
        }
        match hdr.code {
            c if c == NM_DBLCLK && role == Src::List => {
                self.open_selection(pidx);
                LRESULT(0)
            }
            c if c == NM_RCLICK && role == Src::List => {
                // 右键：选中项（或目录本身=空白区）弹 shell 上下文菜单。
                self.pane_context_menu(main, pidx);
                LRESULT(0)
            }
            c if c == NM_CLICK && role == Src::List => {
                // 原版：点击窗格即激活它。
                let changed = if let Some(id) = PaneId::from_number(pidx as u32 + 1) {
                    if self.session.focus != id {
                        self.session.focus = id;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if changed {
                    unsafe { self.refresh_status() };
                }
                self.update_preview_pane(pidx);
                LRESULT(0)
            }
            c if c == LVN_ITEMCHANGED && role == Src::List => {
                // 选中变化（键盘/程序选择也走这里）：预览跟随 + 状态栏选中数。
                self.update_preview_pane(pidx);
                unsafe { self.refresh_status() };
                LRESULT(0)
            }
            c if c == TCN_SELCHANGE && role == Src::Tab => {
                let pos = unsafe {
                    SendMessageW(hdr.hwndFrom, TCM_GETCURSEL, None, None).0.max(0) as usize
                };
                self.tab_select(pidx, pos);
                LRESULT(0)
            }
            _ => LRESULT(0),
        }
    }
}

// ===================== 批次A 纯函数（可单测，无 FFI） =====================

/// 重命名决策（纯函数）：区分「合法/空名/非法字符/与原名相同」。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenameDecision {
    Ok,
    Empty,
    Illegal,
    Unchanged,
}

/// Windows 文件名字法（沿用 shell::rename_path 的字符集，另含首尾空格/点）。
const ILLEGAL_NAME_CHARS: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// 判定一个新名字能否作为 `old_path` 的重命名目标。纯函数。
pub fn rename_decision(old_path: &std::path::Path, new_name: &str) -> RenameDecision {
    let trimmed = new_name.trim_end();
    if trimmed.is_empty() {
        return RenameDecision::Empty;
    }
    if new_name.chars().any(|c| ILLEGAL_NAME_CHARS.contains(&c) || c == '\0') {
        return RenameDecision::Illegal;
    }
    // Windows 文件名不能以点结尾。
    if trimmed.ends_with('.') {
        return RenameDecision::Illegal;
    }
    let cur = old_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // 大小写无关比较（Windows 卷语义）。
    if cur.eq_ignore_ascii_case(new_name) {
        return RenameDecision::Unchanged;
    }
    RenameDecision::Ok
}

/// 生成唯一文件夹名：base 可用则用之，否则 base 2/base 3/…（纯函数）。
/// `exists` 探针由调用方注入（真实文件系统 or 测试桩）。语义对标 egui
/// 版 new_folder（ui/app.rs:825）+ Explorer「新建文件夹」回落惯例。
pub fn unique_folder_name(base: &str, exists: &dyn Fn(&str) -> bool) -> String {
    if !exists(base) {
        return base.to_string();
    }
    let mut n = 2u32;
    loop {
        let cand = format!("{base} {n}");
        if !exists(&cand) {
            return cand;
        }
        n += 1;
        if n > 9_999 {
            // 理论上到不了；防御死循环，返回带时间戳兜底名。
            return format!("{base} {}", std::process::id());
        }
    }
}

/// ComboBox 项文本 → 驱动器根路径（纯函数）。
/// 形如 `本地磁盘 (C:)` / `卷标 (D:)` / `CD 驱动器 (E:)` → `C:\`。
/// 解析括号内的盘符；无括号或盘符非单字母则 None。
pub fn combo_sel_to_path(text: &str) -> Option<std::path::PathBuf> {
    let open = text.rfind('(')?;
    let close = text.rfind(')')?;
    if close <= open + 1 {
        return None;
    }
    let inner = &text[open + 1..close];
    let chars: Vec<char> = inner.chars().collect();
    if chars.len() >= 2 && chars[1] == ':' && chars[0].is_ascii_alphabetic() {
        let drive = chars[0].to_ascii_uppercase();
        Some(std::path::PathBuf::from(format!("{drive}:\\")))
    } else {
        None
    }
}

// ---------- 批次B 纯函数（可单测，无 FFI） ----------

/// 槽 exe 目录（current_exe 的父目录；失败回落 "."）。
fn slot_exe_dir() -> crate::core::QResult<std::path::PathBuf> {
    let exe = std::env::current_exe().map_err(|e| crate::core::QError::from_io(&e))?;
    Ok(exe
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from(".")))
}

// ---------- 批次D/D2 IDLIST 路径恢复（shell↔config 接线） ----------

/// `.qdr` 保存用的 IDLIST 路径编码器（`shell::idlist::path_to_idlist_hex` 薄包装，
/// 供 `config::save_session_with` 注入；config 层无 shell 依赖）。
pub fn qdr_idlist_encoder(p: &std::path::Path) -> Option<String> {
    crate::shell::path_to_idlist_hex(p).ok()
}

/// 用 `.qdr` 的四个 `FolderN` IDLIST 解码结果覆盖各窗格当前标签页路径。
///
/// - 空 IDLIST（`%00%00`，原版默认文件夹语义）→ 保持会话默认路径；
/// - 虚拟文件夹 PIDL（「此电脑」GUID PIDL 等）→ `SHGetPathFromIDListW` 失败，
///   保持默认并留痕；
/// - 解码成功但目录已不存在 → 保持默认（防呆，不写坏档）。
///
/// 修复的历史缺陷：load 侧从未解码 IDLIST，.qdr 打开/重启恢复/布局槽载入
/// 一律丢失窗格路径（见 `shell/idlist.rs` 模块文档）。
pub fn apply_idlist_paths(session: &mut crate::core::SessionState, idlists: &[String; 4]) {
    for (i, hex) in idlists.iter().enumerate() {
        if hex.trim().is_empty() {
            continue;
        }
        let Some(id) = crate::core::PaneId::from_number(i as u32 + 1) else {
            continue;
        };
        match crate::shell::idlist_hex_to_path(hex) {
            Ok(p) if p.is_dir() => {
                if let Some(tab) = session.pane_mut(id).current_tab_mut() {
                    tab.path = p.clone();
                    tab.history = vec![p];
                    tab.history_pos = 0;
                }
            }
            Ok(_) => {} // 解码成功但目录不存在：保持默认路径
            Err(e) => eprintln!("[l-dir] IDLIST 解码失败 窗格{}：{e}", i + 1),
        }
    }
}

// ---------- 批次B 纯函数（可单测，无 FFI） ----------

/// 有界消息泵（批次D/D1 钩子专用）：在 on_create 同步上下文内驱动本线程
/// 消息循环（PeekMessage+Translate+Dispatch），直到 `cond` 满足或超时。
/// 返回 `cond` 最终是否满足。仅测试钩子使用（重入风险已评估：on_size/
/// fill_pane 均幂等）。
///
/// # Safety
/// 仅主 UI 线程、已有帧挂载的窗口上下文调用。
unsafe fn bounded_pump(main: HWND, cond: impl Fn() -> bool, max_ms: u64) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, PM_REMOVE, TranslateMessage, MSG,
    };
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < max_ms as u128 {
        let mut msg = MSG::default();
        while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if cond() {
            return true;
        }
        unsafe { windows::Win32::System::Threading::Sleep(10) };
    }
    let _ = main;
    cond()
}

/// 监视防抖判定（批次D/D1 纯函数）：距上次刷新 ≥ 200ms 才允许再灌。
pub fn watch_debounce_elapsed(
    last: std::time::Instant,
    now: std::time::Instant,
) -> bool {
    now.saturating_duration_since(last) >= std::time::Duration::from_millis(WATCH_DEBOUNCE_MS)
}

/// 屏幕坐标命中哪个 list 矩形（批次B/B3：WM_DROPFILES 落点窗格判定）。
/// `rects` 与窗格序一一对应；矩形为 `(left, top, right, bottom)` 屏幕坐标。
/// 无命中返回 None（调用方回落活动窗格）。
pub fn pane_hit_index(rects: &[(i32, i32, i32, i32)], pt: (i32, i32)) -> Option<usize> {
    rects
        .iter()
        .position(|&(l, t, r, b)| pt.0 >= l && pt.0 < r && pt.1 >= t && pt.1 < b)
}

/// WM_DROPFILES 的落点屏幕坐标（DragQueryPointW；失败 (0,0)）。
///
/// # Safety
/// `hdrop_raw` 必须是 WM_DROPFILES 的 wParam（有效 HDROP 句柄）。
unsafe fn frame_drop_point(hdrop_raw: isize) -> (i32, i32) {
    let hdrop = windows::Win32::UI::Shell::HDROP(hdrop_raw as *mut core::ffi::c_void);
    let mut pt = POINT::default();
    let ok = unsafe { windows::Win32::UI::Shell::DragQueryPoint(hdrop, &mut pt) };
    if ok.as_bool() {
        (pt.x, pt.y)
    } else {
        (0, 0)
    }
}

// ---------- 批次A：从 MainFrame 拆出的树通知处理（保持原行为） ----------

impl MainFrame {
    /// 树通知（懒加载/选中/复选框）。批次A 把原 on_notify 的树分支拆到这里，
    /// 由 on_notify 在路由后调用；语义与拆前完全一致。
    pub(crate) unsafe fn on_tree_notify(&mut self, pidx: usize, hdr_code: u32, lp: LPARAM) -> LRESULT {
        match hdr_code {
            c if c == tree::TVN_ITEMEXPANDEDW => {
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
            c if c == tree::TVN_SELCHANGEDW => {
                if self.panes[pidx].tree_nodes.busy {
                    return LRESULT(0);
                }
                let Some(tv) = self.panes[pidx].tree else { return LRESULT(0) };
                let h = tree::tree_next(tv, TVGN_CARET, HTREEITEM(0));
                let Some(dir) = self.panes[pidx].tree_nodes.path(h).map(|x| x.to_path_buf()) else {
                    return LRESULT(0)
                };
                if dir.is_dir() {
                    self.navigate_to(pidx, dir);
                }
                LRESULT(0)
            }
            c if c == tree::TVN_ITEMSTATEICONCLICKW => {
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
    // 批次C/C2：地址栏自动补全（SHAutoComplete，AF:72/51599/199020 三处
    // 确证原版有此行为）。SHACF_FILESYS_ONLY=仅文件系统路径；延迟绑定
    // shlwapi（windows crate link! 内部处理）。失败（老系统无 shlwapi 导出）
    // 只留痕，不影响地址栏手动输入。
    if let Some(addr) = frame.panes[index].addr_edit {
        match unsafe {
            windows::Win32::UI::Shell::SHAutoComplete(
                addr,
                windows::Win32::UI::Shell::SHACF_FILESYS_ONLY,
            )
        } {
            Ok(()) => println!("[qdir-rs] TEST 地址栏自动补全 窗格{} 已挂", index + 1),
            Err(e) => eprintln!("[qdir-rs] 地址栏 SHAutoComplete 失败 窗格{}：{e}", index + 1),
        }
    }

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
    let tab_style = WS_VISIBLE | WS_CHILD | WS_CLIPSIBLINGS
        | WINDOW_STYLE(if frame.config.tab.show_tooltips {
            TCS_TOOLTIPS
        } else {
            0
        });
    frame.panes[index].tab = create_child(
        content,
        hinstance,
        WC_TABCONTROL,
        w!(""),
        tab_style,
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
    // 预览面板控件（默认隐藏；session.preview_visible 时由布局 pass 显示）。
    frame.panes[index].preview_img = create_child(
        content,
        hinstance,
        w!("Static"),
        w!(""),
        WS_CHILD | WS_CLIPSIBLINGS | WINDOW_STYLE(SS_BITMAP.0 | SS_NOTIFY.0),
        base + CID_PREVIEW_IMG,
    )
    .ok();
    frame.panes[index].preview_text = create_child(
        content,
        hinstance,
        w!("Edit"),
        w!(""),
        WS_CHILD
            | WS_CLIPSIBLINGS
            | WS_BORDER
            | WINDOW_STYLE((ES_MULTILINE | ES_READONLY) as u32 | WS_VSCROLL.0),
        base + CID_PREVIEW_TEXT,
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
        // 挂系统小图标列表（LVSIL_SMALL）；FileItem.icon_index 即其索引
        // （enumerate 以 SHGFI_SMALLICON|SHGFI_SYSICONINDEX 查询，尺寸一致）。
        if let Some(hilm) = unsafe { system_small_imagelist() } {
            unsafe {
                let _ = SendMessageW(
                    lv,
                    LVM_SETIMAGELIST,
                    Some(WPARAM(LVSIL_SMALL as usize)),
                    Some(LPARAM(hilm.0)),
                );
            }
        }
        // 挂系统大图标列表（LVSIL_NORMAL；批次B/B1：大图标视图出图）。
        // 系统图标索引在大小两个图像列表间通用（同一 iIcon），无需逐项换索引；
        // comctl32 在 LV_VIEW_ICON 下自动取 LVSIL_NORMAL 渲染。
        if let Some(hilm) = unsafe { system_large_imagelist() } {
            unsafe {
                let _ = SendMessageW(
                    lv,
                    LVM_SETIMAGELIST,
                    Some(WPARAM(LVSIL_NORMAL as usize)),
                    Some(LPARAM(hilm.0)),
                );
            }
        }
        // 键盘导航子类化（Enter 打开/Backspace 返回/地址栏回车）。
        unsafe {
            crate::ui_native::subclass::install(
                frame.panes[index].list,
                frame.panes[index].addr_edit,
                frame.panes[index].tab,
                index,
            )
        };
        // 批次B/B3 注意：OLE 落点注册**不在此处**做——RegisterDragDrop
        // 禁止在 WM_CREATE 期间调用（ole32 0xC000041D）。on_create 末尾
        // PostMessage(WM_APP+1)，消息循环启动后统一注册。
        // 验证钩子：QDIR_TEST_TAB_CLOSE=1 → 窗格1 加第二个标签后向其
        // 页签控件发 WM_MBUTTONUP（tab 条坐标 (5,5)），走完整
        // 子类化→TCM_HITTEST→close_tab→重建 链（同进程真实消息）。
        if index == 0 && std::env::var("QDIR_TEST_TAB_CLOSE").is_ok() {
            frame.session.panes[0].add_tab(std::path::PathBuf::from(r"C:\Windows"));
            unsafe { fill_pane_tabs(frame, 0) };
            if let Some(tab_ctl) = frame.panes[0].tab {
                let lp_pt: isize = 5i32 as isize | ((5i32 as isize) << 16);
                unsafe {
                    let dbl = std::env::var("QDIR_TEST_TAB_CLOSE").map(|v| v == "2").unwrap_or(false);
                    if dbl {
                        // =2：左键双击路径（默认开关关=吞事件不关，tabs 应保 2）。
                        let _ = SendMessageW(
                            tab_ctl,
                            WM_LBUTTONDBLCLK,
                            Some(WPARAM(0x0001)), // MK_LBUTTON
                            Some(LPARAM(lp_pt)),
                        );
                    } else {
                        let _ = SendMessageW(
                            tab_ctl,
                            WM_MBUTTONUP,
                            Some(WPARAM(0x10usize)), // MK_MBUTTON
                            Some(LPARAM(lp_pt)),
                        );
                    }
                }
                println!(
                    "[qdir-rs] TEST 标签钩子 关后 tabs={}",
                    frame.session.panes[0].tabs.len()
                );
            }
        }
        // 首次填充（建列头 + 灌行）。
        unsafe { frame.fill_pane(index) };
    }
}

/// Tab 页签标题 = 窗格各标签页 title（原版 dump：TabItem 数 == tabs 数）。
unsafe fn fill_pane_tabs(frame: &mut MainFrame, index: usize) {
    let Some(tab_ctl) = frame.panes[index].tab else { return };
    // 幂等：先清空再灌（钩子/重建都走这里）。
    let _ = SendMessageW(tab_ctl, TCM_DELETEALLITEMS, None, None);
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

/// 重建页签控件内容（关标签/载入状态后调用）。
unsafe fn rebuild_pane_tabs(frame: &mut MainFrame, index: usize) {
    let Some(tab_ctl) = frame.panes[index].tab else { return };
    fill_pane_tabs(frame, index);
    let cur = frame.session.panes[index].tabs_pos as usize;
    let _ = SendMessageW(tab_ctl, TCM_SETCURSEL, Some(WPARAM(cur)), None);
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
///
/// `cols` 为批次D/D3 传入的可见列集（visible_ordered_columns 结果）；
/// 空表示无可见列（不插列，仅清行）。
unsafe fn populate_list(
    hwnd: HWND,
    icons: &mut IconCache,
    entries: &[(FileItem, FileItemExt)],
    cols: &[crate::core::ColumnDef],
) {
    unsafe { SendMessageW(hwnd, LVM_DELETEALLITEMS, None, None) };
    // 删旧列再按 cols 重建（D3 切换显隐时列数变化）。
    unsafe {
        while SendMessageW(hwnd, LVM_DELETECOLUMN, Some(WPARAM(0)), None).0 != 0 {}
    }

    // 列（批次D/D3：模型驱动；原硬编码 4 列见 COLUMN_TITLES 历史注释）。
    // LVCFMT_SPLITBUTTON（=HDF_SPLITBUTTON 0x1000000）：comctl32 v6 下让
    // 列头渲染/暴露为下拉拆分按钮，与原版 UIA 树（HeaderItem→SplitButton）
    // 对齐；HDF_SORTUP/DOWN 由排序状态另行维护（*排序箭头接线待补*）。
    for (i, col) in cols.iter().enumerate() {
        let (title, default_w) = column_meta(&col.key);
        let mut text: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let mut lvcol = LVCOLUMNW {
            mask: LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM | LVCF_FMT,
            fmt: LVCFMT_SPLITBUTTON,
            cx: if col.width > 0 { col.width as i32 } else { default_w },
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
    // 全局状态栏：统一走 refresh_status（计数/选中/盘符/空间一体现算）。
    // row_paths 在本函数调用点之前已写入，这里读到的是新值。
    unsafe { frame.refresh_status() };
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
    let full_body = (cx0, ccy0 + body_y, cw, body_h);
    // 预览面板（Vorschau）：右侧列，宽=(1-preview_pos) 比例钳 15%..50%。
    let (body, preview_rect) = if frame.session.preview_visible {
        let ratio = (1.0 - frame.session.preview_pos.ratio()).clamp(0.15, 0.5);
        let pw = (cw as f32 * ratio) as i32;
        (
            (full_body.0, full_body.1, (cw - pw - tree::TREE_GAP).max(60), full_body.3),
            Some((full_body.0 + cw - pw, full_body.1, pw, full_body.3)),
        )
    } else {
        (full_body, None)
    };
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
    // 预览面板显隐与几何：图片/文本二选一（preview_mode 决定）。
    match (preview_rect, p.preview_img, p.preview_text) {
        (Some((px, py, pw, ph)), Some(img), Some(txt)) => unsafe {
            let _ = SetWindowPos(img, None, px, py, pw.max(1), ph.max(1), swp);
            let _ = SetWindowPos(txt, None, px, py, pw.max(1), ph.max(1), swp);
            let mode = frame.preview_mode[index];
            let _ = ShowWindow(img, if mode == 1 { SW_SHOWNA } else { SW_HIDE });
            let _ = ShowWindow(txt, if mode == 0 { SW_SHOWNA } else { SW_HIDE });
        },
        (None, Some(img), Some(txt)) => unsafe {
            let _ = ShowWindow(img, SW_HIDE);
            let _ = ShowWindow(txt, SW_HIDE);
        },
        _ => {}
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

/// 进程级缓存：系统大图标 ImageList 句柄（批次B/B1：大图标视图 LVSIL_NORMAL）。
static LARGE_IL: std::sync::OnceLock<isize> = std::sync::OnceLock::new();

/// 取（或缓存）系统大图标列表句柄（SHGFI_LARGEICON|SHGFI_SYSICONINDEX）。
///
/// 与 `system_small_imagelist` 同法，只换 flag；系统图标索引在两表间通用。
unsafe fn system_large_imagelist() -> Option<HIMAGELIST> {
    let h = *LARGE_IL.get_or_init(|| unsafe {
        let mut fi = windows::Win32::UI::Shell::SHFILEINFOW::default();
        let dot: Vec<u16> = ".".encode_utf16().chain(std::iter::once(0)).collect();
        let use_shgfi = windows::Win32::UI::Shell::SHGFI_LARGEICON
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
            0
        } else {
            r as isize
        }
    });
    (h != 0).then(|| HIMAGELIST(h))
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

/// 状态栏 5 段的分段边界数组（SB_SETPARTS 语义：元素数=段数，最后一项
/// -1=拉伸到底），随客户区宽缩放：段0 计数(22%)、段1 版本(10%)、
/// 段2 用户(20%)、段3 盘符(26%)、段4 空间(剩余)。纯函数便于单测；
/// 比例按原版 UIA 段宽比例近似（5 段确证，具体宽度*待复核*）。
pub fn status_parts(width: i32) -> [i32; 5] {
    let w = width.max(240);
    [w * 22 / 100, w * 32 / 100, w * 52 / 100, w * 78 / 100, -1]
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
    use super::{
        column_sort_click, dir_of_selection, status_parts, tray_action_for, TrayAction,
    };

    #[test]
    fn tray_action_mapping() {
        use crate::shell::TrayEvent;
        assert_eq!(tray_action_for(&TrayEvent::Select), TrayAction::Restore);
        assert_eq!(tray_action_for(&TrayEvent::DoubleClick), TrayAction::Restore);
        assert_eq!(tray_action_for(&TrayEvent::ContextMenu), TrayAction::ShowMenu);
        assert_eq!(tray_action_for(&TrayEvent::MiddleClick), TrayAction::Ignore);
        assert_eq!(tray_action_for(&TrayEvent::BalloonShow), TrayAction::Ignore);
        assert_eq!(tray_action_for(&TrayEvent::BalloonClick), TrayAction::Ignore);
        assert_eq!(tray_action_for(&TrayEvent::Other(9)), TrayAction::Ignore);
    }
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

    #[test]
    fn status_parts_monotonic_and_clamped() {
        let p = status_parts(800);
        // 单调递增、比例近似、末段由 -1 拉伸（不在数组里）。
        assert!(p[0] < p[1] && p[1] < p[2] && p[2] < p[3]);
        assert_eq!(p[0], 176);
        assert_eq!(p[1], 256);
        assert_eq!(p[2], 416);
        assert_eq!(p[3], 624);
        assert_eq!(p.len(), 5);
        assert_eq!(p[4], -1, "末段拉伸");
        // 极窄窗口钳到 240 基线，边界仍在窗内。
        let q = status_parts(10);
        assert!(q[3] <= 240);
        let r = status_parts(4000);
        assert_eq!(r[3], 3120);
    }
}

#[cfg(test)]
mod batch_a_tests {
    use super::{
        column_meta, combo_sel_to_path, pane_hit_index, rename_decision, unique_folder_name,
        visible_ordered_columns, watch_debounce_elapsed, RenameDecision,
    };
    use std::path::Path;

    // ---------- A3 重命名决策（纯函数） ----------

    #[test]
    fn rename_ok_for_new_legal_name() {
        assert_eq!(rename_decision(Path::new(r"C:\a\old.txt"), "new.txt"), RenameDecision::Ok);
    }

    #[test]
    fn rename_rejects_empty_and_whitespace() {
        assert_eq!(rename_decision(Path::new(r"C:\a\old.txt"), ""), RenameDecision::Empty);
        assert_eq!(rename_decision(Path::new(r"C:\a\old.txt"), "   "), RenameDecision::Empty);
    }

    #[test]
    fn rename_rejects_illegal_chars() {
        for bad in ["a\\b", "a/b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b"] {
            assert_eq!(rename_decision(Path::new(r"C:\a\old.txt"), bad), RenameDecision::Illegal, "{bad}");
        }
    }

    #[test]
    fn rename_rejects_trailing_dot() {
        assert_eq!(rename_decision(Path::new(r"C:\a\old.txt"), "newname."), RenameDecision::Illegal);
    }

    #[test]
    fn rename_unchanged_is_case_insensitive() {
        // 同名（大小写无关）视为未改动，不触发磁盘操作。
        assert_eq!(rename_decision(Path::new(r"C:\a\old.txt"), "old.txt"), RenameDecision::Unchanged);
        assert_eq!(rename_decision(Path::new(r"C:\a\old.txt"), "OLD.TXT"), RenameDecision::Unchanged);
        // 仅大小写变化仍算 Unchanged（Windows 卷大小写无关）。
        assert_eq!(rename_decision(Path::new(r"C:\a\Old.TXT"), "old.txt"), RenameDecision::Unchanged);
    }

    // ---------- A4 唯一文件夹名（纯函数） ----------

    #[test]
    fn unique_name_first_free_slot() {
        let occupied: Vec<&str> = vec![];
        let exists = |n: &str| occupied.contains(&n);
        assert_eq!(unique_folder_name("新建文件夹", &exists), "新建文件夹");
    }

    #[test]
    fn unique_name_skips_taken() {
        let occupied = vec!["新建文件夹", "新建文件夹 2"];
        let exists = |n: &str| occupied.contains(&n);
        assert_eq!(unique_folder_name("新建文件夹", &exists), "新建文件夹 3");
    }

    #[test]
    fn unique_name_finds_gap_after_run() {
        let occupied = vec!["新建文件夹", "新建文件夹 2", "新建文件夹 3", "新建文件夹 4"];
        let exists = |n: &str| occupied.contains(&n);
        assert_eq!(unique_folder_name("新建文件夹", &exists), "新建文件夹 5");
    }

    // ---------- A6 ComboBox 选盘解析（纯函数） ----------

    #[test]
    fn combo_parses_drive_from_label() {
        assert_eq!(combo_sel_to_path("本地磁盘 (C:)"), Some(Path::new("C:\\").to_path_buf()));
        assert_eq!(combo_sel_to_path("卷标 (D:)"), Some(Path::new("D:\\").to_path_buf()));
        assert_eq!(combo_sel_to_path("CD 驱动器 (E:)"), Some(Path::new("E:\\").to_path_buf()));
    }

    #[test]
    fn combo_lowercases_to_upper_drive() {
        // 盘符归一到大写（drive_root 语义）。
        assert_eq!(combo_sel_to_path("x (d:)"), Some(Path::new("D:\\").to_path_buf()));
    }

    #[test]
    fn combo_rejects_malformed() {
        assert_eq!(combo_sel_to_path("没有括号"), None);
        assert_eq!(combo_sel_to_path("空 ()"), None);
        // 非单字母盘符（如 UNC 提示）不解析。
        assert_eq!(combo_sel_to_path("网络 (ab:)"), None);
    }

    #[test]
    fn combo_uses_last_paren_pair() {
        // 卷标里含括号时取最后一对（rfind）。
        assert_eq!(combo_sel_to_path("数据 (备份) (F:)"), Some(Path::new("F:\\").to_path_buf()));
    }

    // ---------- B3 拖放落点窗格判定（纯函数） ----------

    #[test]
    fn pane_hit_index_basic() {
        let rects = [
            (0, 0, 100, 100),
            (100, 0, 200, 100),
            (0, 100, 100, 200),
            (100, 100, 200, 200),
        ];
        assert_eq!(pane_hit_index(&rects, (50, 50)), Some(0));
        assert_eq!(pane_hit_index(&rects, (150, 50)), Some(1));
        assert_eq!(pane_hit_index(&rects, (50, 150)), Some(2));
        assert_eq!(pane_hit_index(&rects, (150, 150)), Some(3));
    }

    #[test]
    fn pane_hit_index_boundary_is_left_inclusive_right_exclusive() {
        let rects = [(0, 0, 100, 100)];
        // 右/下边界不属于该矩形（Windows 矩形命中语义）。
        assert_eq!(pane_hit_index(&rects, (99, 99)), Some(0));
        assert_eq!(pane_hit_index(&rects, (100, 50)), None);
        assert_eq!(pane_hit_index(&rects, (50, 100)), None);
        assert_eq!(pane_hit_index(&rects, (-1, 50)), None);
    }

    #[test]
    fn pane_hit_index_gap_and_empty() {
        // 矩形间有空隙 → None（调用方回落活动窗格）。
        let rects = [(0, 0, 50, 50), (100, 0, 150, 50)];
        assert_eq!(pane_hit_index(&rects, (75, 25)), None);
        // 空表 → None。
        assert_eq!(pane_hit_index(&[], (1, 1)), None);
    }

    // ---------- D3 列显隐（纯函数） ----------

    #[test]
    fn visible_ordered_columns_filters_and_sorts() {
        use crate::core::{ColumnConfig, ColumnDef};
        let mut cfg = ColumnConfig::default();
        // 乱序 order + 部分隐藏。
        cfg.columns = vec![
            ColumnDef { key: "Size".into(), width: 80, visible: true, order: 2 },
            ColumnDef { key: "Name".into(), width: 200, visible: true, order: 0 },
            ColumnDef { key: "Attr".into(), width: 60, visible: false, order: 1 },
            ColumnDef { key: "Ext".into(), width: 60, visible: true, order: 1 },
        ];
        let v = visible_ordered_columns(&cfg);
        let keys: Vec<&str> = v.iter().map(|c| c.key.as_str()).collect();
        // Ext(order=1) 在 Size(order=2) 前；Attr 不可见被过滤。
        assert_eq!(keys, vec!["Name", "Ext", "Size"]);
    }

    #[test]
    fn visible_ordered_columns_ties_keep_stable_order() {
        use crate::core::{ColumnConfig, ColumnDef};
        let mut cfg = ColumnConfig::default();
        // order 全并列：保持 columns 原序（sort_by_key 稳定）。
        cfg.columns = vec![
            ColumnDef { key: "Modified".into(), width: 1, visible: true, order: 7 },
            ColumnDef { key: "Name".into(), width: 1, visible: true, order: 7 },
        ];
        let keys: Vec<String> = visible_ordered_columns(&cfg).into_iter().map(|c| c.key).collect();
        assert_eq!(keys, vec!["Modified", "Name"]);
    }

    #[test]
    fn visible_ordered_columns_all_hidden_and_empty() {
        use crate::core::{ColumnConfig, ColumnDef};
        let mut cfg = ColumnConfig::default();
        cfg.columns = vec![
            ColumnDef { key: "Name".into(), width: 1, visible: false, order: 0 },
            ColumnDef { key: "Ext".into(), width: 1, visible: false, order: 1 },
        ];
        assert!(visible_ordered_columns(&cfg).is_empty());
        cfg.columns.clear();
        assert!(visible_ordered_columns(&cfg).is_empty(), "空列集不 panic");
    }

    #[test]
    fn column_meta_defaults_and_fallback() {
        assert_eq!(column_meta("Name"), ("名称", 200));
        assert_eq!(column_meta("Ext"), ("扩展名", 60));
        assert_eq!(column_meta("Size"), ("大小", 90));
        assert_eq!(column_meta("Modified"), ("修改日期", 150));
        assert_eq!(column_meta("Attr"), ("属性", 60));
        assert_eq!(column_meta("Unknown"), ("列", 80), "未知键回落");
    }

    // ---------- D1 监视防抖（纯函数） ----------

    #[test]
    fn watch_debounce_threshold() {
        let t0 = std::time::Instant::now();
        // 未到 200ms → 不灌。
        assert!(!watch_debounce_elapsed(t0, t0 + std::time::Duration::from_millis(199)));
        // 达到 200ms → 灌。
        assert!(watch_debounce_elapsed(t0, t0 + std::time::Duration::from_millis(200)));
        assert!(watch_debounce_elapsed(t0, t0 + std::time::Duration::from_millis(5000)));
        // now 早于 last（时钟倒退/重复触发）→ 不灌（saturating=0）。
        assert!(!watch_debounce_elapsed(t0 + std::time::Duration::from_secs(1), t0));
    }
}
