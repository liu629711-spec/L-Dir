//! 托盘：`Shell_NotifyIconW`（最小化到托盘、单击还原）。
//!
//! 设计约束：托盘是**窗口消息驱动**的，本模块不自己创建窗口
//! （那属于 app 层职责），只提供：
//! - [`TrayIcon`]：对 `NOTIFYICONDATAW` 的薄封装（add/modify/delete）；
//! - [`tray_event_from_message`]：把窗口过程收到的 `uCallbackMessage`
//!   映射成 [`TrayEvent`]，供 app 层分发。
//!
//! `NIN_SELECT`（单击）→ 还原窗口；`WM_RBUTTONUP` → 上层弹自己的菜单；
//! 气泡回调事件只透传，不解释。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFYICONDATAW, NOTIFY_ICON_DATA_FLAGS, NOTIFY_ICON_STATE,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{HICON, WM_LBUTTONDBLCLK, WM_RBUTTONUP};

use super::{io_error, to_wide, HostWindow};
use crate::core::{QError, QResult};

/// 托盘事件（由窗口过程的 `uCallbackMessage` 的 lParam 推出）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    /// 单击（NIN_SELECT，等价 WM_LBUTTONDOWN 在 4.0+ 上不再发）。
    Select,
    /// 双击（WM_LBUTTONDBLCLK）。
    DoubleClick,
    /// 右键抬起 → 上层应弹上下文菜单。
    ContextMenu,
    /// 气泡显示/隐藏/超时/点击等（只透传，语义由上层解释）。
    BalloonShow,
    BalloonHide,
    BalloonTimeout,
    BalloonClick,
    PopupOpen,
    PopupClose,
    /// 鼠标中键。
    MiddleClick,
    /// 其它/未知。
    Other(u32),
}

/// 托盘图标种类（[`TrayIconKind`] 决定用哪个 stock 图标）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayIconKind {
    /// 应用主图标（上层从自己的资源/ICO 文件加载后传入 HICON）。
    App,
    /// 信息气泡。
    Info,
    /// 警告气泡。
    Warning,
    /// 错误气泡。
    Error,
}

/// 一个托盘图标实例。
///
/// 生命周期：`add` 之后必须 `delete`（Drop 自动做），否则进程退出后
/// 托盘会残留幽灵图标直到鼠标划过。
pub struct TrayIcon {
    hwnd: HWND,
    /// 任务栏图标 id（同一窗口内多个图标时区分）。
    id: u32,
    /// 回调消息号（上层自定，如 WM_APP + 1）。
    callback_message: u32,
    /// 当前提示文本（上限 127 字符）。
    tip: String,
    /// 是否已添加。
    added: bool,
    /// 图标句柄（由调用方提供，本类型不拥有，不负责销毁）。
    icon: HICON,
}

impl TrayIcon {
    /// 创建（未添加）。
    ///
    /// `icon` 必须是已加载的 HICON；生命周期由调用方管理
    /// （一般跟随进程，卸载时 `DestroyIcon`）。
    pub fn new(hwnd: HostWindow, id: u32, callback_message: u32, icon: HICON) -> Self {
        TrayIcon {
            hwnd: hwnd.hwnd(),
            id,
            callback_message,
            tip: String::new(),
            added: false,
            icon,
        }
    }

    /// 添加到托盘。
    pub fn add(&mut self) -> QResult<()> {
        let mut data = self.base_data(NIF_MESSAGE | NIF_ICON | NIF_TIP);
        // SAFETY: data 已完整初始化。
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &data) };
        if !ok.as_bool() {
            return Err(io_error("Shell_NotifyIconW(NIM_ADD)"));
        }
        self.added = true;
        let _ = &mut data;
        Ok(())
    }

    /// 修改提示文本。
    pub fn set_tip(&mut self, tip: &str) -> QResult<()> {
        // szTip 是 [u16; 128]，最多 127 字符 + NUL。
        let truncated: String = tip.chars().take(127).collect();
        self.tip = truncated;
        if !self.added {
            return Ok(());
        }
        let mut data = self.base_data(NIF_TIP);
        // SAFETY: data 已完整初始化。
        let ok = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
        if !ok.as_bool() {
            return Err(io_error("Shell_NotifyIconW(NIM_MODIFY, TIP)"));
        }
        let _ = &mut data;
        Ok(())
    }

    /// 修改图标。
    pub fn set_icon(&mut self, icon: HICON) -> QResult<()> {
        self.icon = icon;
        if !self.added {
            return Ok(());
        }
        let mut data = self.base_data(NIF_ICON);
        // SAFETY: data 已完整初始化。
        let ok = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
        if !ok.as_bool() {
            return Err(io_error("Shell_NotifyIconW(NIM_MODIFY, ICON)"));
        }
        let _ = &mut data;
        Ok(())
    }

    /// 升级到 4.0 行为（NIN_SELECT 等）。Vista+ 默认已是 4.0，此调用为兼容旧版。
    pub fn set_version_4(&mut self) -> QResult<()> {
        if !self.added {
            return Err(QError::Shell("托盘图标尚未添加".to_string()));
        }
        let mut data = self.base_data(NOTIFY_ICON_DATA_FLAGS(0));
        // uVersion 与 uTimeout 共用 union；NIM_SETVERSION 用 uVersion。
        data.Anonymous = notifyicondataw_version_union(4);
        // SAFETY: data 已完整初始化。
        let ok = unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) };
        if !ok.as_bool() {
            return Err(io_error("Shell_NotifyIconW(NIM_SETVERSION)"));
        }
        let _ = &mut data;
        Ok(())
    }

    /// 显示气泡提示（info/warning/error）。
    pub fn show_balloon(&mut self, kind: TrayIconKind, title: &str, text: &str) -> QResult<()> {
        if !self.added {
            return Err(QError::Shell("托盘图标尚未添加".to_string()));
        }
        let mut data = self.base_data(NOTIFY_ICON_DATA_FLAGS(0x10 /* NIF_INFO */));
        // szInfoTitle 是 [u16; 64]（最多 63 字符 + NUL），szInfo 是 [u16; 256]。
        // 先按字符数截断再编码，保证不溢出（代理项算两个 u16）。
        let title_w = to_wide_truncated(title, 63);
        let text_w = to_wide_truncated(text, 255);
        data.szInfoTitle[..title_w.len()].copy_from_slice(&title_w);
        data.szInfo[..text_w.len()].copy_from_slice(&text_w);
        // NIIF_INFO / NIIF_WARNING / NIIF_ERROR。
        data.dwInfoFlags = match kind {
            TrayIconKind::Info => windows::Win32::UI::Shell::NIIF_INFO,
            TrayIconKind::Warning => windows::Win32::UI::Shell::NIIF_WARNING,
            TrayIconKind::Error => windows::Win32::UI::Shell::NIIF_ERROR,
            TrayIconKind::App => windows::Win32::UI::Shell::NIIF_NONE,
        };
        // SAFETY: data 已完整初始化。
        let ok = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
        if !ok.as_bool() {
            return Err(io_error("Shell_NotifyIconW(NIM_MODIFY, INFO)"));
        }
        let _ = &mut data;
        Ok(())
    }

    /// 从托盘移除。
    pub fn delete(&mut self) {
        if !self.added {
            return;
        }
        let mut data = self.base_data(NOTIFY_ICON_DATA_FLAGS(0));
        // SAFETY: data 已完整初始化。
        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
        let _ = &mut data;
        self.added = false;
    }

    /// 当前提示文本。
    pub fn tip(&self) -> &str {
        &self.tip
    }

    /// 回调消息号。
    pub fn callback_message(&self) -> u32 {
        self.callback_message
    }

    /// 是否已添加。
    pub fn is_added(&self) -> bool {
        self.added
    }

    /// 构造 NOTIFYICONDATAW 基础字段。
    fn base_data(&self, flags: NOTIFY_ICON_DATA_FLAGS) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = self.hwnd;
        data.uID = self.id;
        data.uFlags = flags;
        data.uCallbackMessage = self.callback_message;
        data.hIcon = self.icon;
        data.dwState = NOTIFY_ICON_STATE(0);
        data.dwStateMask = NOTIFY_ICON_STATE(0);
        let tip_w = to_wide(&self.tip);
        let n = tip_w.len().min(127);
        data.szTip[..n].copy_from_slice(&tip_w[..n]);
        data
    }
}

/// 把字符串转成宽字符并截断到 `max_units` 个 u16（不含 NUL）。
///
/// 按字符截断：代理项（>BMP 字符）算两个 u16，若截断点落在代理对中间
/// 则退格一个字符，避免留下孤立代理项（Windows API 会拒绝）。
fn to_wide_truncated(s: &str, max_units: usize) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    for c in s.chars() {
        let mut buf = [0u16; 2];
        let encoded = c.encode_utf16(&mut buf);
        if out.len() + encoded.len() > max_units {
            break;
        }
        out.extend_from_slice(encoded);
    }
    out
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.delete();
    }
}

/// `NOTIFYICONDATAW_0`（uTimeout/uVersion union）构造。
fn notifyicondataw_version_union(version: u32) -> windows::Win32::UI::Shell::NOTIFYICONDATAW_0 {
    let mut u: windows::Win32::UI::Shell::NOTIFYICONDATAW_0 = unsafe { std::mem::zeroed() };
    // SAFETY: union 的 uVersion 字段写入。
    unsafe {
        *(&mut u as *mut _ as *mut u32) = version;
    }
    u
}

/// 把窗口过程收到的回调消息 `lParam` 映射成 [`TrayEvent`]。
///
/// `wParam` 是图标 id，`lParam` 是鼠标消息。
pub fn tray_event_from_message(msg: u32, lparam: isize) -> TrayEvent {
    match lparam as u32 {
        x if x == WM_LBUTTONDBLCLK => TrayEvent::DoubleClick,
        x if x == WM_RBUTTONUP => TrayEvent::ContextMenu,
        // NIN_SELECT（单击）在 4.0 版本下仍是 WM_LBUTTONDOWN；
        // windows crate 里 NIN_SELECT = 1024，与 WM_LBUTTONDOWN 同值。
        x if x == 0x0201 /* WM_LBUTTONDOWN */ => TrayEvent::Select,
        0x0209 /* WM_MBUTTONDOWN */ => TrayEvent::MiddleClick,
        1026 => TrayEvent::BalloonShow,
        1027 => TrayEvent::BalloonHide,
        1028 => TrayEvent::BalloonTimeout,
        1029 => TrayEvent::BalloonClick,
        1030 => TrayEvent::PopupOpen,
        1031 => TrayEvent::PopupClose,
        other => TrayEvent::Other(other),
    }
    .tap(|_| {
        let _ = msg;
    })
}

/// 小工具：把 `Option<TrayEvent>` 的映射结果直接返回（保持上面的 match 可读）。
trait Tap: Sized {
    fn tap(self, f: impl FnOnce(&Self)) -> Self {
        f(&self);
        self
    }
}
impl<T> Tap for T {}

/// 便捷：判断是否是本模块关心的托盘回调（供窗口过程快速分流）。
pub fn is_tray_message(msg: u32, callback_message: u32) -> bool {
    msg == callback_message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_event_mapping() {
        assert_eq!(
            tray_event_from_message(0, WM_LBUTTONDBLCLK as isize),
            TrayEvent::DoubleClick
        );
        assert_eq!(
            tray_event_from_message(0, WM_RBUTTONUP as isize),
            TrayEvent::ContextMenu
        );
        assert_eq!(tray_event_from_message(0, 0x0201), TrayEvent::Select);
        assert_eq!(tray_event_from_message(0, 0x0209), TrayEvent::MiddleClick);
        assert_eq!(tray_event_from_message(0, 1026), TrayEvent::BalloonShow);
        assert_eq!(tray_event_from_message(0, 1027), TrayEvent::BalloonHide);
        assert_eq!(tray_event_from_message(0, 1028), TrayEvent::BalloonTimeout);
        assert_eq!(tray_event_from_message(0, 1029), TrayEvent::BalloonClick);
        assert_eq!(tray_event_from_message(0, 1030), TrayEvent::PopupOpen);
        assert_eq!(tray_event_from_message(0, 1031), TrayEvent::PopupClose);
        assert_eq!(tray_event_from_message(0, 12345), TrayEvent::Other(12345));
    }

    #[test]
    fn is_tray_message_filters() {
        assert!(is_tray_message(0x8001, 0x8001));
        assert!(!is_tray_message(0x8002, 0x8001));
    }

    #[test]
    fn tray_icon_new_is_not_added() {
        let t = TrayIcon::new(HostWindow::from_raw_hwnd(0), 1, 0x8001, HICON::default());
        assert!(!t.is_added());
        assert_eq!(t.callback_message(), 0x8001);
        assert_eq!(t.tip(), "");
        // Drop 时 delete() 会提前返回（未添加）。
    }

    #[test]
    fn set_tip_truncates_to_127_chars() {
        let mut t = TrayIcon::new(HostWindow::from_raw_hwnd(0), 1, 0x8001, HICON::default());
        let long: String = "x".repeat(500);
        // 未添加时只更新内部字段（不触达 Shell_NotifyIconW）。
        assert!(t.set_tip(&long).is_ok());
        assert_eq!(t.tip().chars().count(), 127);
        assert!(t.set_tip("短提示").is_ok());
        assert_eq!(t.tip(), "短提示");
    }

    #[test]
    fn balloon_and_version_require_add() {
        let mut t = TrayIcon::new(HostWindow::from_raw_hwnd(0), 1, 0x8001, HICON::default());
        assert!(t.set_version_4().is_err());
        assert!(t.show_balloon(TrayIconKind::Info, "t", "x").is_err());
        // set_icon 在未添加时只更新字段。
        assert!(t.set_icon(HICON::default()).is_ok());
    }
}
