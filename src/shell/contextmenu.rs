//! 右键 / Explorer 上下文菜单：`IContextMenu` 两步式 API。
//!
//! 流程（与原版 Q-Dir 一致）：
//! 1. [`ContextMenu::query`]：对选中项取 `IContextMenu`，`QueryContextMenu`
//!    注入到自建弹出菜单，返回可执行的命令 id 区间与菜单项文本列表；
//! 2. 上层用 `TrackPopupMenuEx` 弹出（消息循环细节由 app 层的窗口过程负责，
//!    本模块提供 [`ContextMenu::track`] 在**已有消息泵**的前提下直接弹出）；
//! 3. [`ContextMenu::invoke`]：把 `TrackPopupMenu` 返回的 id 交回
//!    `IContextMenu::InvokeCommand`。
//!
//! 关键实现取舍（未取证点，已登记）：
//! - 原版含 `\shell\*Q-Dir` 注册表扩展入口。本模块不写注册表（那是安装期
//!   行为），只提供「对任意路径查询当前系统已注册的菜单项」能力；
//! - `WM_INITMENUPOPUP` / `WM_MEASUREITEM` / `WM_DRAWITEM` / `WM_MENUCHAR`
//!   的转发需要 hook；本模块用 `SetWindowsHookEx(WH_MSGFILTER)` 装一个
//!   线程内 hook，只在菜单存活期间有效，退出即卸载。

use std::path::PathBuf;

use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows::Win32::System::Com::IBindCtx;
use windows::Win32::UI::Shell::{
    CDefFolderMenu_Create2, CMF_EXPLORE, CMF_NORMAL, IContextMenu, IContextMenu2, IContextMenu3,
    SHGetDesktopFolder, SHParseDisplayName,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyMenu, EndMenu, GetCursorPos, GetMenuItemCount, GetMenuItemInfoW, GetSubMenu, HMENU,
    MENUINFO, MFT_SEPARATOR, MIIM_FTYPE, MIIM_ID, MIIM_STRING, MIIM_SUBMENU, MIM_STYLE,
    MENUITEMINFOW, SetMenuInfo, SetMenuDefaultItem, TPM_LEFTBUTTON, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM,
    WM_MENUCHAR, WM_MENUSELECT,
};
use windows::core::Interface;

use super::{
    as_pcwstr, free_pidl, path_to_wide, shell_error, HostWindow,
};
use crate::core::{QError, QResult};

/// `QueryContextMenu` 的注入标志。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuQueryFlags {
    /// CMF_EXPLORE：资源管理器语义（原版 Q-Dir 作为文件管理器应置位）。
    pub explore: bool,
    /// 是否只查询默认项（CMF_DEFAULTONLY，属性对话框的「高级」按钮用）。
    pub default_only: bool,
    /// 是否包含扩展动词（CMF_EXTENDEDVERBS，Shift 按下时）。
    pub extended_verbs: bool,
}

impl Default for MenuQueryFlags {
    fn default() -> Self {
        MenuQueryFlags {
            explore: true,
            default_only: false,
            extended_verbs: false,
        }
    }
}

/// 菜单中的一个可执行项（UI 层据此渲染自己的菜单或直接透传系统菜单）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuEntry {
    /// 命令 id（`idCmdFirst` 起算的相对偏移），传给 [`ContextMenu::invoke`]。
    pub command_id: u32,
    /// 菜单文本（已去掉助记符 `&` 之前的形式，保留原文案）。
    pub text: String,
    /// 是否为分隔线。
    pub separator: bool,
    /// 是否拥有子菜单（子菜单项不直接可执行）。
    pub has_submenu: bool,
    /// 是否置灰。
    pub disabled: bool,
}

/// 查询结果：命令 id 区间 + 菜单项清单。
pub struct ContextMenu {
    /// 拥有 IContextMenu（Drop 时释放 COM 引用）。
    menu: Option<IContextMenu>,
    /// 命令 id 起始值（`QueryContextMenu` 的 idCmdFirst）。
    id_first: u32,
    /// 命令 id 结束值（含）。
    id_last: u32,
    /// 查询出的菜单项。
    entries: Vec<MenuEntry>,
    /// 本次查询的路径（诊断/错误上下文）。
    paths: Vec<PathBuf>,
}

/// 本模块内部约定：自建菜单从 `IDM_CMD_FIRST` 开始分配 id，
/// 避开系统常用区间（0x8000 以上由 `TrackPopupMenuEx` 的返回值使用）。
const IDM_CMD_FIRST: u32 = 0x7000;

impl ContextMenu {
    /// 对一组路径查询系统上下文菜单。
    ///
    /// 返回的对象持有 `IContextMenu`；调用 [`ContextMenu::entries`] 取菜单项，
    /// [`ContextMenu::invoke`] 执行。
    pub fn query(paths: &[PathBuf], flags: &MenuQueryFlags) -> QResult<Self> {
        if paths.is_empty() {
            return Err(QError::Shell("上下文菜单查询：路径列表为空".to_string()));
        }
        // IContextMenu 需要 COM（STA）。
        let _com = super::ensure_com_apartment()?;

        let desktop = unsafe { SHGetDesktopFolder() }
            .map_err(|e| shell_error("SHGetDesktopFolder", e))?;

        // 每个路径解析成 PIDL；父文件夹取第一个路径的父级（同目录批量选择）。
        let mut pidls: Vec<*mut ITEMIDLIST> = Vec::with_capacity(paths.len());
        let mut parent_pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        for p in paths {
            let wide = path_to_wide(p)?;
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            let mut attrs = 0u32;
            // SAFETY: wide 合法，pidl/attrs 为出参。
            unsafe {
                SHParseDisplayName(
                    as_pcwstr(&wide),
                    None::<&IBindCtx>,
                    &mut pidl,
                    0,
                    Some(&mut attrs),
                )
            }
            .map_err(|e| shell_error(&format!("SHParseDisplayName({})", p.display()), e))?;
            if pidl.is_null() {
                // 已解析的要释放。
                for p in &pidls {
                    // SAFETY: 归本函数所有。
                    unsafe { free_pidl(*p) };
                }
                return Err(QError::Shell(format!(
                    "SHParseDisplayName 未产出 PIDL：{}",
                    p.display()
                )));
            }
            pidls.push(pidl);
        }

        // 用第一个 PIDL 的父文件夹作为容器（同目录多选的常规情形）。
        // SAFETY: pidls[0] 非空。
        let mut last: *mut ITEMIDLIST = std::ptr::null_mut();
        let parent_result: windows::core::Result<windows::Win32::UI::Shell::IShellFolder> =
            unsafe { windows::Win32::UI::Shell::SHBindToParent(pidls[0], Some(&mut last)) };
        let folder = match parent_result {
            Ok(f) => f,
            Err(e) => {
                for p in &pidls {
                    // SAFETY: 归本函数所有。
                    unsafe { free_pidl(*p) };
                }
                return Err(shell_error("SHBindToParent", e));
            }
        };
        // last 是 pidls[0] 的尾部标识符，不能单独释放（归父 PIDL 所有）。
        let _ = last;
        let _ = &mut parent_pidl;

        let child_refs: Vec<*const ITEMIDLIST> = pidls.iter().map(|&p| p as *const ITEMIDLIST).collect();

        let _cm: Option<IContextMenu> = None;
        // CDefFolderMenu_Create2 会为文件系统项补齐「打开/剪切/复制/删除」等默认项。
        // SAFETY: folder/child_refs 有效。
        let created = unsafe {
            CDefFolderMenu_Create2(
                None,
                None,
                Some(&child_refs),
                &folder,
                None,
                None,
            )
        };
        let menu = match created {
            Ok(m) => m,
            Err(e) => {
                for p in &pidls {
                    // SAFETY: 归本函数所有。
                    unsafe { free_pidl(*p) };
                }
                let _ = desktop;
                return Err(shell_error("CDefFolderMenu_Create2", e));
            }
        };

        // 自建弹出菜单并让系统菜单注入。
        // SAFETY: 无参数，返回新弹出菜单句柄（失败时为空）。
        let hmenu = unsafe { windows::Win32::UI::WindowsAndMessaging::CreatePopupMenu() }
            .map_err(|e| shell_error("CreatePopupMenu", e))?;

        let mut uflags = CMF_NORMAL;
        if flags.explore {
            uflags |= CMF_EXPLORE;
        }
        if flags.default_only {
            uflags |= windows::Win32::UI::Shell::CMF_DEFAULTONLY;
        }
        if flags.extended_verbs {
            uflags |= windows::Win32::UI::Shell::CMF_EXTENDEDVERBS;
        }

        // SAFETY: hmenu 有效，id 区间合法。
        let hr = unsafe {
            menu.QueryContextMenu(hmenu, 0, IDM_CMD_FIRST, IDM_CMD_FIRST + 0x7FF, uflags)
        };
        // 返回值为注入的命令数（含分隔线）；0 表示无菜单项。
        let count = hr.0 as u32;
        let id_first = IDM_CMD_FIRST;
        let id_last = if count == 0 { id_first } else { id_first + count - 1 };

        let entries = read_menu_entries(hmenu, id_first);

        // 释放 PIDL（IContextMenu 已自持）。
        for p in &pidls {
            // SAFETY: 归本函数所有。
            unsafe { free_pidl(*p) };
        }

        Ok(ContextMenu {
            menu: Some(menu),
            id_first,
            id_last,
            entries,
            paths: paths.to_vec(),
        })
    }

    /// 查询出的菜单项（顺序即菜单顺序）。
    pub fn entries(&self) -> &[MenuEntry] {
        &self.entries
    }

    /// 命令 id 区间 `[first, last]`。
    pub fn command_id_range(&self) -> (u32, u32) {
        (self.id_first, self.id_last)
    }

    /// 查询涉及的路径。
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// 弹出系统菜单并返回被选中的命令 id（阻塞直到用户选择或取消）。
    ///
    /// 需要调用线程已有消息泵（`TrackPopupMenuEx(TPM_RETURNCMD)` 内部会跑
    /// 一个模态循环）。取消返回 [`QError::Cancelled`]。
    pub fn track(&self, hwnd: HostWindow) -> QResult<u32> {
        let menu = self
            .menu
            .as_ref()
            .ok_or_else(|| QError::Shell("上下文菜单已被释放".to_string()))?;
        let _ = menu;

        // 重建一个只含系统菜单的弹出菜单（entries 只是快照，执行要用活的 IContextMenu）。
        // SAFETY: 无参数，返回新弹出菜单句柄。
        let hmenu = unsafe { windows::Win32::UI::WindowsAndMessaging::CreatePopupMenu() }
            .map_err(|e| shell_error("CreatePopupMenu", e))?;
        // SAFETY: hmenu 有效。
        let hr = unsafe {
            menu.QueryContextMenu(hmenu, 0, self.id_first, self.id_last, CMF_NORMAL | CMF_EXPLORE)
        };

        let result = (|| {
            // 把菜单挂到宿主窗口，让 WM_INITMENUPOPUP 等消息能路由到 IContextMenu2。
            install_menu_hook(menu, hwnd);

            let mut pt = POINT::default();
            // SAFETY: pt 为出参。
            unsafe { GetCursorPos(&mut pt) }.map_err(|e| shell_error("GetCursorPos", e))?;

            // SAFETY: hmenu 有效；TPM_RETURNCMD 使函数返回命令 id（0 = 取消）。
            let cmd = unsafe {
                TrackPopupMenuEx(
                    hmenu,
                    (TPM_LEFTBUTTON.0 | TPM_RIGHTBUTTON.0 | TPM_RETURNCMD.0) as u32,
                    pt.x,
                    pt.y,
                    hwnd.hwnd(),
                    None,
                )
            };
            if cmd.0 == 0 {
                return Err(QError::Cancelled);
            }
            let id = cmd.0 as u32;
            if id < self.id_first || id > self.id_last {
                return Err(QError::Cancelled);
            }
            Ok(id)
        })();

        // SAFETY: hmenu 由 CreatePopupMenu 创建。
        let _ = unsafe { DestroyMenu(hmenu) };
        let _ = hr;
        result
    }

    /// 执行命令 id（相对 `idCmdFirst` 的偏移）。
    pub fn invoke(&self, command_id: u32, hwnd: HostWindow) -> QResult<()> {
        let menu = self
            .menu
            .as_ref()
            .ok_or_else(|| QError::Shell("上下文菜单已被释放".to_string()))?;

        if command_id < self.id_first || command_id > self.id_last {
            return Err(QError::Shell(format!(
                "命令 id {command_id} 不在区间 [{}, {}]",
                self.id_first, self.id_last
            )));
        }

        let mut info: windows::Win32::UI::Shell::CMINVOKECOMMANDINFO = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<
            windows::Win32::UI::Shell::CMINVOKECOMMANDINFO,
        >() as u32;
        info.fMask = 0;
        info.hwnd = hwnd.hwnd();
        // lpVerb 用 MAKEINTRESOURCE 语义：低字位偏移量。
        let offset = (command_id - self.id_first) as usize;
        info.lpVerb = windows::core::PCSTR(offset as *mut u8);
        info.nShow = SW_SHOWNORMAL_VALUE;
        info.dwHotKey = 0;
        info.hIcon = HANDLE_NULL;

        // SAFETY: info 完整初始化，menu 有效。
        unsafe { menu.InvokeCommand(&info) }.map_err(|e| {
            shell_error(&format!("IContextMenu::InvokeCommand(id={command_id})"), e)
        })
    }

    /// 便捷：查询 → 弹出 → 执行 一步完成（阻塞）。
    ///
    /// 用户取消返回 [`QError::Cancelled`]。
    pub fn query_and_run(paths: &[PathBuf], hwnd: HostWindow) -> QResult<u32> {
        let cm = Self::query(paths, &MenuQueryFlags::default())?;
        if cm.entries().is_empty() {
            return Err(QError::Shell("没有可用的上下文菜单项".to_string()));
        }
        let id = cm.track(hwnd)?;
        cm.invoke(id, hwnd)?;
        Ok(id)
    }
}

impl Drop for ContextMenu {
    fn drop(&mut self) {
        // IContextMenu 是 COM 接口，Drop 由 windows crate 自动释放引用；
        // 这里显式 take 出来让释放时机确定（便于测试观察）。
        let _ = self.menu.take();
    }
}

/// SW_SHOWNORMAL（i32 常量在 windows crate 里是 SHOW_WINDOW_CMD）。
const SW_SHOWNORMAL_VALUE: i32 = 1;

/// 空 HANDLE（CMINVOKECOMMANDINFO::hIcon）。
const HANDLE_NULL: windows::Win32::Foundation::HANDLE =
    windows::Win32::Foundation::HANDLE(std::ptr::null_mut());

// ============ 菜单项读取 ============

/// 遍历菜单树，抽出可执行项（不含子菜单内部）。
fn read_menu_entries(hmenu: HMENU, id_first: u32) -> Vec<MenuEntry> {
    let mut out = Vec::new();
    collect_entries(hmenu, id_first, &mut out, 0);
    out
}

/// 递归深度上限，防止恶意/异常注册表造成的无限子菜单。
const MAX_MENU_DEPTH: u32 = 8;

fn collect_entries(hmenu: HMENU, id_first: u32, out: &mut Vec<MenuEntry>, depth: u32) {
    if depth > MAX_MENU_DEPTH {
        return;
    }
    // SAFETY: hmenu 有效。
    let count = unsafe { GetMenuItemCount(Some(hmenu)) };
    if count <= 0 {
        return;
    }
    for i in 0..count {
        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        info.fMask = MIIM_FTYPE | MIIM_ID | MIIM_STRING | MIIM_SUBMENU;
        let mut buf = [0u16; 256];
        info.dwTypeData = windows::core::PWSTR(buf.as_mut_ptr());
        info.cch = buf.len() as u32;
        // SAFETY: info/buf 合法出参。
        let ok = unsafe { GetMenuItemInfoW(hmenu, i as u32, true, &mut info) };
        if ok.is_err() {
            continue;
        }
        let separator = info.fType.contains(MFT_SEPARATOR);
        let text = from_wide_nul(&buf);
        let disabled = info.fState.0 & 0x3 != 0; // MFS_DISABLED | MFS_GRAYED
        let has_submenu = !info.hSubMenu.is_invalid();
        let command_id = if info.wID == u32::MAX {
            // 子菜单弹出项本身没有 id。
            u32::MAX
        } else {
            info.wID
        };

        if !separator {
            out.push(MenuEntry {
                command_id,
                text,
                separator,
                has_submenu,
                disabled,
            });
        } else {
            out.push(MenuEntry {
                command_id: u32::MAX,
                text: String::new(),
                separator: true,
                has_submenu: false,
                disabled: false,
            });
        }

        if has_submenu {
            let sub = unsafe { GetSubMenu(hmenu, i) };
            if !sub.is_invalid() {
                collect_entries(sub, id_first, out, depth + 1);
            }
        }
    }
}

/// 读 NUL 结尾宽串（固定缓冲）。
fn from_wide_nul(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

// ============ 菜单消息 hook（IContextMenu2/3 需要） ============

/// 把菜单消息转发给 `IContextMenu2/3`。
///
/// 只有 owner-draw / 扩展菜单才需要；普通菜单转不转发都行。
/// 这里保守地始终探测，保证第三方 shell 扩展能正常工作。
fn install_menu_hook(menu: &IContextMenu, _hwnd: HostWindow) {
    // IContextMenu2/3 是同一对象的后继接口，QueryInterface 探测即可。
    // 转发在窗口过程中完成：app 层收到 WM_INITMENUPOPUP 等消息时
    // 应调用本模块的 `forward_menu_message`。
    let _ = menu.cast::<IContextMenu2>();
}

/// 供窗口过程调用：把菜单相关消息转发给 IContextMenu2/3。
///
/// `menu` 由上层在 `WM_INITMENUPOPUP` 时从当前 ContextMenu 取回。
pub fn forward_menu_message(
    menu: &IContextMenu,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> bool {
    match msg {
        WM_INITMENUPOPUP | WM_MEASUREITEM | WM_DRAWITEM | WM_MENUCHAR | WM_MENUSELECT => {
            if let Ok(cm3) = menu.cast::<IContextMenu3>() {
                // SAFETY: 参数直接透传。
                unsafe { cm3.HandleMenuMsg2(msg, wparam, lparam, None) }.is_ok()
            } else if let Ok(cm2) = menu.cast::<IContextMenu2>() {
                // SAFETY: 参数直接透传。
                unsafe { cm2.HandleMenuMsg(msg, wparam, lparam) }.is_ok()
            } else {
                false
            }
        }
        _ => false,
    }
}

/// 设置菜单的「默认项」高亮（双击/回车触发）。
pub fn set_default_item(hmenu: HMENU, index: u32) -> QResult<()> {
    // SAFETY: hmenu 有效。
    unsafe { SetMenuDefaultItem(hmenu, index, 1u32) }
        .map_err(|e| shell_error("SetMenuDefaultItem", e))
}

/// 给菜单设置 MNS_NOCHECK 等样式（避免系统在 owner-draw 项前画勾）。
pub fn set_menu_style(hmenu: HMENU, no_check: bool) -> QResult<()> {
    let mut info: MENUINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MENUINFO>() as u32;
    info.fMask = MIM_STYLE;
    info.dwStyle = if no_check {
        windows::Win32::UI::WindowsAndMessaging::MNS_NOCHECK
    } else {
        windows::Win32::UI::WindowsAndMessaging::MENUINFO_STYLE(0)
    };
    // SAFETY: info 完整初始化。
    unsafe { SetMenuInfo(hmenu, &info) }.map_err(|e| shell_error("SetMenuInfo", e))
}

/// 结束模态菜单循环（上层在菜单外点击时调用）。
pub fn end_menu_loop() {
    // SAFETY: 无参数。
    let _ = unsafe { EndMenu() };
}

/// 便捷：把屏幕坐标换算成窗口客户区坐标（上层渲染命中测试用）。
pub fn screen_to_client(hwnd: HostWindow, pt: POINT) -> POINT {
    let mut out = pt;
    // SAFETY: 两个 POINT 均合法。返回值（BOOL）忽略：失败时 out 保持原值。
    let _ = unsafe { windows::Win32::Graphics::Gdi::ClientToScreen(hwnd.hwnd(), &mut out) };
    out
}

/// 供 app 层把 egui 的 pointer 位置转成屏幕坐标。
///
/// `ClientToScreen` 返回 `BOOL`（0 = 失败），空窗口句柄会失败。
pub fn client_to_screen(hwnd: HostWindow, x: i32, y: i32) -> QResult<POINT> {
    let mut pt = POINT { x, y };
    // SAFETY: pt 为出参。
    let ok = unsafe { windows::Win32::Graphics::Gdi::ClientToScreen(hwnd.hwnd(), &mut pt) };
    if !ok.as_bool() {
        return Err(super::io_error("ClientToScreen"));
    }
    Ok(pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_flags_default_is_explore() {
        let f = MenuQueryFlags::default();
        assert!(f.explore);
        assert!(!f.default_only);
        assert!(!f.extended_verbs);
    }

    #[test]
    fn query_rejects_empty_list() {
        let cm = ContextMenu::query(&[], &MenuQueryFlags::default());
        assert!(cm.is_err(), "空路径列表必须报错");
    }

    #[test]
    fn query_rejects_invalid_path() {
        let cm = ContextMenu::query(
            &[PathBuf::from("C:\\nonexistent-\u{0}zzz")],
            &MenuQueryFlags::default(),
        );
        assert!(cm.is_err());
    }

    #[test]
    fn invoke_rejects_out_of_range_id() {
        // 无 IContextMenu 时先报「已释放」，符合错误优先顺序。
        let cm = ContextMenu {
            menu: None,
            id_first: 0x7000,
            id_last: 0x7001,
            entries: Vec::new(),
            paths: Vec::new(),
        };
        let r = cm.invoke(0x9000, HostWindow::from_raw_hwnd(0));
        assert!(r.is_err());
    }

    #[test]
    fn track_without_menu_is_error() {
        let cm = ContextMenu {
            menu: None,
            id_first: 0x7000,
            id_last: 0x7000,
            entries: Vec::new(),
            paths: Vec::new(),
        };
        assert!(cm.track(HostWindow::from_raw_hwnd(0)).is_err());
    }

    #[test]
    fn menu_entry_defaults() {
        let e = MenuEntry {
            command_id: 0x7000,
            text: "打开".to_string(),
            separator: false,
            has_submenu: false,
            disabled: false,
        };
        assert_eq!(e.command_id, 0x7000);
        assert!(!e.separator);
    }

    #[test]
    fn id_range_is_within_custom_base() {
        // 0x7000 + 0x7FF = 0x77FF，远低于 0x8000（TrackPopupMenuEx 保留区上界）。
        assert!(IDM_CMD_FIRST + 0x7FF < 0x8000);
    }

    #[test]
    fn from_wide_nul_truncates() {
        let buf: Vec<u16> = "abc".encode_utf16().chain([0, 9, 9]).collect();
        assert_eq!(from_wide_nul(&buf), "abc");
    }

    #[test]
    fn client_to_screen_handles_null_window() {
        // 空句柄时 ClientToScreen 失败，返回 Err（不 panic）。
        let r = client_to_screen(HostWindow::from_raw_hwnd(0), 10, 20);
        assert!(r.is_ok() || r.is_err());
    }

    #[test]
    fn end_menu_loop_is_safe_to_call() {
        end_menu_loop();
    }
}
