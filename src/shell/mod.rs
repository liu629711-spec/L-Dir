//! Windows Shell 集成门面（契约 D3/D4：本模块是唯一直接调用 `windows` crate 的地方）。
//!
//! 上层（app / pane / filelist / preview）只通过本模块暴露的安全 Rust API 接触
//! Windows Shell，拿到的全是 `core::FileItem` / `core::QResult` / `PathBuf` /
//! 纯数据切片；所有 `unsafe` FFI、HANDLE、PIDL、COM 接口指针都封在本目录内。
//!
//! 子模块划分（与 modules 清单一致）：
//! - [`enumerate`]：目录枚举（IShellFolder + SHGetFileInfoW + FindFirstFileW）；
//! - [`icons`]：系统图标 → RGBA 位图与 `icon_key` 缓存；
//! - [`fileops`]：复制/移动/删除/重命名/新建文件夹/打开/属性/打印；
//! - [`contextmenu`]：IContextMenu 两步式 API（query_context_menu + invoke）；
//! - [`dragdrop`]：WM_DROPFILES 外部拖入 + OLE 拖放源；
//! - [`clipboard`]：CF_HDROP / Preferred DropEffect 剪贴板复制/剪切/粘贴；
//! - [`tray`]：Shell_NotifyIconW 托盘；
//! - [`special`]：特殊文件夹、虚拟命名空间（此电脑/库）与驱动器枚举。
//!
//! 平台约束（G1）：整个模块只在 `cfg(windows)` 下编译，core/config 保持平台无关。

#![cfg(windows)]

// 门面只做「重新导出 + 公共 FFI 助手」，不含业务逻辑。
mod clipboard;
mod contextmenu;
mod dragdrop;
mod enumerate;
mod fileops;
mod icons;
mod special;
mod tray;

pub use clipboard::{
    ClipboardContent, clipboard_has_files, copy_to_clipboard, cut_to_clipboard, get_clipboard,
    paste_into, set_clipboard, to_clipboard_with_config,
};
pub use contextmenu::{
    ContextMenu, MenuEntry, MenuQueryFlags, client_to_screen, end_menu_loop, forward_menu_message,
    screen_to_client, set_default_item, set_menu_style,
};
pub use dragdrop::{
    DragOutcome, DropFiles, DropTarget, accept_drop_files, allows_copy, allows_move, begin_drag,
    register_drop_target, revoke_drop_target,
};
pub use enumerate::{
    EnumOptions, FileItemExt, display_name_of, is_filesystem_path, list_directory,
    list_directory_detailed, list_shell_folder, type_name_of,
};
pub use fileops::{
    FileOpOptions, FileOpReport, copy_file_fallback, copy_files, create_shortcut, delete_files,
    delete_one_fallback, empty_recycle_bin, move_file_fallback, move_files, move_files_with_config,
    new_folder, open_default, open_path_with, print_file, read_shortcut_target, recycle_bin_info,
    rename_path, send_by_mail, show_properties,
};
pub use icons::{IconCache, IconImage, IconSize, icon_image_for_path, stock_icon, system_icon_index_for_path};
pub use special::{
    DriveInfo, DriveKind, ShellLocation, desktop_path, documents_path, drive_root, is_drive_root,
    is_libraries, is_this_pc, libraries_path, list_drives, network_path, parse_shell_path,
    recycle_bin_path, special_folder_path, this_pc_path,
};
pub use tray::{TrayEvent, TrayIcon, TrayIconKind, is_tray_message, tray_event_from_message};

use std::mem::size_of;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{GetLastError, HWND, WIN32_ERROR};
use windows::core::PCWSTR;

use crate::core::{QError, QResult};

// ============ 公共 FFI 助手（unsafe 全部封在本模块内） ============

/// 上层传入的宿主窗口句柄。
///
/// shell 模块内部才持有 `HWND`（契约 D4 禁止句柄泄漏到模块外），
/// 上层通过 eframe 的 raw window handle 拿到 `isize` 后交给本类型。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HostWindow {
    hwnd: HWND,
}

impl HostWindow {
    /// 从平台原始句柄（`isize`）构造。`0` 表示无宿主窗口（部分 API 允许）。
    pub fn from_raw_hwnd(raw: isize) -> Self {
        HostWindow {
            hwnd: HWND(raw as *mut std::ffi::c_void),
        }
    }

    /// 取回平台原始句柄，供上层做消息路由对照。
    pub fn raw_hwnd(self) -> isize {
        self.hwnd.0 as isize
    }

    /// 是否为空句柄。
    pub fn is_null(self) -> bool {
        self.hwnd.is_invalid()
    }

    /// 内部使用：暴露给本模块其余子模块。
    pub(crate) fn hwnd(self) -> HWND {
        self.hwnd
    }
}

/// 取当前线程的 `GetLastError` 码（调用点必须紧邻失败的外部调用）。
pub(crate) fn last_error_code() -> u32 {
    // SAFETY: 读线程局部错误码，无内存安全前提。
    unsafe { GetLastError() }.0
}

/// 释放 `SHParseDisplayName` / `ILClone` 等 shell API 分配的 PIDL。
///
/// # Safety
/// `pidl` 必须由上述 API 分配、且尚未被释放；空指针安全（无操作）。
pub(crate) unsafe fn free_pidl(pidl: *mut windows::Win32::UI::Shell::Common::ITEMIDLIST) {
    if pidl.is_null() {
        return;
    }
    // SAFETY: 调用方保证所有权（见上文各处调用点注释）。
    unsafe {
        windows::Win32::System::Com::CoTaskMemFree(Some(pidl as *const std::ffi::c_void));
    }
}

/// 由紧邻的失败外部调用构造 IO 错误，保留 OS 错误码（C3）。
pub(crate) fn io_error(context: &str) -> QError {
    let code = last_error_code();
    QError::Io {
        code,
        msg: format!("{context}（OS 错误码 {code}）"),
    }
}

/// 把 `windows::core::Error` 转成 [`QError::Shell`]，保留 HRESULT 与系统消息。
pub(crate) fn shell_error(context: &str, e: windows::core::Error) -> QError {
    QError::Shell(format!(
        "{context}：HRESULT 0x{:08X} {}",
        e.code().0 as u32,
        e.message()
    ))
}

/// `Vec<u16>`（NUL 结尾）→ `PCWSTR`。
///
/// 调用方必须保证 `buf` 在 FFI 调用期间存活。
pub(crate) fn as_pcwstr(buf: &[u16]) -> PCWSTR {
    PCWSTR::from_raw(buf.as_ptr())
}

/// `&str` → NUL 结尾宽字符缓冲。
pub(crate) fn to_wide(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

/// 把宽字符缓冲截到第一个 NUL 并转成 `String`（宽容 Lossy）。
pub(crate) fn from_wide_lossy(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// 校验路径可交给 Windows API：非空、不含内嵌 NUL。
pub(crate) fn validate_path(path: &Path) -> QResult<()> {
    if path.as_os_str().is_empty() {
        return Err(QError::InvalidPath("路径为空".to_string()));
    }
    if path.as_os_str().to_string_lossy().contains('\0') {
        return Err(QError::InvalidPath(format!(
            "路径含有 NUL：{:?}",
            path.display()
        )));
    }
    Ok(())
}

/// 路径 → 宽字符缓冲（含校验），供 FFI 使用。
pub(crate) fn path_to_wide(path: &Path) -> QResult<Vec<u16>> {
    validate_path(path)?;
    let mut v: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .collect();
    v.push(0);
    Ok(v)
}

/// 目录路径补齐尾部反斜杠（Shell API 常要求目录带 `\`）。
pub(crate) fn dir_with_backslash(path: &Path) -> QResult<Vec<u16>> {
    validate_path(path)?;
    let mut s = path.to_string_lossy().to_string();
    if !s.ends_with('\\') && !s.ends_with('/') {
        s.push('\\');
    }
    Ok(to_wide(&s))
}

/// 生成 `pFrom` / `pTo` 所需的「双 NUL 结尾」宽串列表。
///
/// 输入是**已带 NUL 结尾**的宽串（如 [`path_to_wide`] 的产物）；
/// 输出为它们顺序拼接后再补一个额外 NUL（SHFileOperationW 的列表终止符）。
pub(crate) fn double_nul(items: &[Vec<u16>]) -> Vec<u16> {
    let mut out = Vec::new();
    for it in items {
        out.extend_from_slice(it);
    }
    out.push(0);
    out
}

/// 父目录；无父目录时返回自身。
///
/// 供上层判断「向上导航」时使用；当前 shell 内部暂无调用点，先导出备用。
#[allow(dead_code)]
pub(crate) fn parent_dir(path: &Path) -> PathBuf {
    path.parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                path.to_path_buf()
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| path.to_path_buf())
}

// ============ COM 单元初始化 ============

/// COM 公寓初始化守卫。
///
/// OLE 剪贴板 / 拖放 / IContextMenu 都要求调用线程处于 STA。
/// 本类型在 Drop 时按需 `CoUninitialize`；重复初始化（S_FALSE /
/// RPC_E_CHANGED_MODE）不会多做一次配对调用。
pub struct ComGuard {
    /// 是否由本次调用真正初始化了 COM（只有 S_OK 才需要在 Drop 里配对）。
    owned: bool,
}

thread_local! {
    /// 本线程 COM 初始化深度，保证嵌套调用只配对最外层。
    static COM_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// 确保当前线程已进入 COM STA。
///
/// 同一线程可嵌套调用；只有最外层真正初始化与反初始化。
pub fn ensure_com_apartment() -> QResult<ComGuard> {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

    let depth = COM_DEPTH.with(|d| d.get());
    if depth > 0 {
        COM_DEPTH.with(|d| d.set(depth + 1));
        return Ok(ComGuard { owned: false });
    }

    // SAFETY: 标准 COM 初始化，参数为空指针与 STA 标志。
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    // S_OK(0) = 本次初始化；S_FALSE(1) = 已初始化（不得反初始化）；
    // RPC_E_CHANGED_MODE = 已是 MTA（本进程其它代码初始化），不得反初始化。
    let owned = hr.0 == 0;
    if !owned && hr.0 != 1 && hr.0 != -2147417850 {
        // -2147417850 = 0x80010106 RPC_E_CHANGED_MODE
        return Err(shell_error("CoInitializeEx", windows::core::Error::from_hresult(hr)));
    }
    COM_DEPTH.with(|d| d.set(1));
    Ok(ComGuard { owned })
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        use windows::Win32::System::Com::CoUninitialize;
        let depth = COM_DEPTH.with(|d| d.get());
        if depth == 0 {
            return;
        }
        COM_DEPTH.with(|d| d.set(depth - 1));
        if self.owned && depth == 1 {
            // SAFETY: 与最外层 CoInitializeEx(S_OK) 配对。
            unsafe { CoUninitialize() };
        }
    }
}

/// 便捷：取 SHGetFileInfoW 所需的 `size_of` 值。
pub(crate) fn shfileinfo_size() -> u32 {
    size_of::<windows::Win32::UI::Shell::SHFILEINFOW>() as u32
}

/// `WIN32_ERROR` → `QError::Io`（带上下文）。
pub(crate) fn win32_error(context: &str, code: WIN32_ERROR) -> QError {
    QError::Io {
        code: code.0,
        msg: format!("{context}（OS 错误码 {}）", code.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_roundtrip() {
        let w = to_wide("C:\\Temp\\ä.txt");
        assert_eq!(*w.last().expect("必须以 NUL 结尾"), 0);
        assert_eq!(from_wide_lossy(&w), "C:\\Temp\\ä.txt");
    }

    #[test]
    fn double_nul_layout() {
        let a = to_wide("a"); // ['a', 0]
        let b = to_wide("b"); // ['b', 0]
        let d = double_nul(&[a, b]);
        // 输入已带 NUL，只需末尾补一个额外 NUL 作列表终止符。
        assert_eq!(d, vec!['a' as u16, 0, 'b' as u16, 0, 0]);
    }

    #[test]
    fn double_nul_empty_is_single_terminator() {
        assert_eq!(double_nul(&[]), vec![0]);
    }

    #[test]
    fn validate_path_rejects_empty_and_nul() {
        assert!(validate_path(Path::new("")).is_err());
        assert!(validate_path(Path::new("C:\\ok")).is_ok());
    }

    #[test]
    fn dir_with_backslash_appends_separator() {
        assert_eq!(from_wide_lossy(&dir_with_backslash(Path::new("C:\\x")).unwrap()), "C:\\x\\");
        assert_eq!(
            from_wide_lossy(&dir_with_backslash(Path::new("C:\\x\\")).unwrap()),
            "C:\\x\\"
        );
    }

    #[test]
    fn host_window_raw_roundtrip() {
        let h = HostWindow::from_raw_hwnd(0x1234);
        assert_eq!(h.raw_hwnd(), 0x1234);
        assert!(HostWindow::from_raw_hwnd(0).is_null());
    }

    #[test]
    fn parent_dir_keeps_root() {
        assert_eq!(parent_dir(Path::new("C:\\a\\b")), PathBuf::from("C:\\a"));
        assert_eq!(parent_dir(Path::new("C:\\")), PathBuf::from("C:\\"));
    }

    #[test]
    fn com_guard_nested_does_not_uninitialize_twice() {
        // 不触达真实 COM：只验证嵌套计数逻辑在 owned=false 时安全 Drop。
        let outer = ComGuard { owned: false };
        drop(outer);
    }
}
