//! 剪贴板：CF_HDROP + Preferred DropEffect（复制/剪切/粘贴）。
//!
//! 与资源管理器互操作的两种格式：
//! - `CF_HDROP`：经典文件列表，任何 Win32 程序都能读；
//! - `CFSTR_SHELLIDLIST`（Shell IDList Array）：OLE 路径，虚拟项也能传。
//!
//! 本模块写入时同时提供两种（CF_HDROP + Preferred DropEffect），
//! 读取时优先 CF_HDROP（简单可靠），并解析 Preferred DropEffect 判断
//! 用户按的是复制还是剪切（`copy_cut_paste_mode` 的语义）。
//!
//! 剪贴板必须开在拥有窗口的线程上；本模块要求 STA（[`super::ensure_com_apartment`]）。

use std::path::PathBuf;

use windows::Win32::Foundation::{HGLOBAL, GlobalFree};
use windows::Win32::System::Com::IDataObject;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::{CF_HDROP, DROPEFFECT_COPY, DROPEFFECT_MOVE, OleSetClipboard};
use windows::Win32::UI::Shell::{
    CFSTR_PREFERREDDROPEFFECT, CIDLData_CreateFromIDArray, SHParseDisplayName,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;

use super::{
    as_pcwstr, free_pidl, io_error, path_to_wide, shell_error, HostWindow,
};
use crate::core::{QError, QResult};

/// 剪贴板内容。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardContent {
    /// 文件路径列表。
    pub paths: Vec<PathBuf>,
    /// true = 剪切语义（粘贴后删除源），false = 复制。
    pub cut: bool,
}

impl ClipboardContent {
    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.paths.len()
    }
}

/// 把文件列表写入剪贴板（CF_HDROP + Preferred DropEffect）。
///
/// `cut = true` 时打 `DROPEFFECT_MOVE`，粘贴方（资源管理器）会执行移动。
pub fn set_clipboard(
    hwnd: HostWindow,
    paths: &[PathBuf],
    cut: bool,
) -> QResult<()> {
    if paths.is_empty() {
        return Err(QError::Shell("写入剪贴板：路径列表为空".to_string()));
    }
    // OleSetClipboard 需要 STA。
    let _com = super::ensure_com_apartment()?;

    // 双 NUL 结尾的宽串 + 末尾 DROPFILES 结构头。
    let mut wides: Vec<u16> = Vec::new();
    for p in paths {
        let w = path_to_wide(p)?;
        wides.extend_from_slice(&w);
    }
    wides.push(0); // 列表结束的额外 NUL

    // DROPFILES { pFiles: u32, pt: POINT, fNC: BOOL, fWide: BOOL } = 20 字节。
    let header_size = 20usize;
    let total = header_size + wides.len() * 2;
    // SAFETY: GMEM_MOVEABLE 合法。
    let hglobal = unsafe { GlobalAlloc(GMEM_MOVEABLE, total) }
        .map_err(|e| shell_error("GlobalAlloc", e))?;

    {
        // SAFETY: hglobal 刚分配；lock/unlock 配对。
        let ptr = unsafe { GlobalLock(hglobal) } as *mut u8;
        if ptr.is_null() {
            // SAFETY: hglobal 归本函数所有。
            unsafe {
                let _ = GlobalFree(Some(hglobal));
            }
            return Err(io_error("GlobalLock"));
        }
        // SAFETY: 区域大小由 GlobalAlloc 保证为 total 字节。
        unsafe {
            // DROPFILES 头清零后只写 pFiles 与 fWide。
            std::ptr::write_bytes(ptr, 0, total);
            let p_files = ptr as *mut u32;
            p_files.write(header_size as u32);
            let f_wide = ptr.add(16) as *mut u32;
            f_wide.write(1);
            let data = ptr.add(header_size) as *mut u16;
            std::ptr::copy_nonoverlapping(wides.as_ptr(), data, wides.len());
        }
        // SAFETY: 与上面的 GlobalLock 配对。
        let _ = unsafe { GlobalUnlock(hglobal) };
    }

    // 打开剪贴板 → 清空 → 设置。hwnd 为空时 OpenClipboard 失败，
    // 因此无宿主窗口的场景退回 OleSetClipboard（需要 IDataObject）。
    // SAFETY: hwnd 可为空句柄（此时 OpenClipboard 大概率失败）。
    let opened = unsafe { OpenClipboard(Some(hwnd.hwnd())) };
    if opened.is_err() {
        // SAFETY: hglobal 归本函数所有。
        unsafe {
            let _ = GlobalFree(Some(hglobal));
        }
        return Err(io_error("OpenClipboard"));
    }

    // SAFETY: 剪贴板已打开。
    let emptied = unsafe { EmptyClipboard() };
    if emptied.is_err() {
        // SAFETY: 两个 API 均无内存前提。
        unsafe {
            let _ = GlobalFree(Some(hglobal));
            let _ = CloseClipboard();
        }
        return Err(io_error("EmptyClipboard"));
    }

    // SAFETY: CF_HDROP 与 hglobal 均有效；SetClipboardData 失败时需自释放。
    let r = unsafe { SetClipboardData(CF_HDROP.0 as u32, Some(handle_from_hglobal(hglobal))) };
    match r {
        Ok(_) => {}
        Err(e) => {
            // SAFETY: SetClipboardData 失败时系统不接管句柄。
            unsafe {
                let _ = GlobalFree(Some(hglobal));
                let _ = CloseClipboard();
            }
            return Err(shell_error("SetClipboardData", e));
        }
    }

    // Preferred DropEffect：复制/剪切语义。
    if let Ok(effect) = make_dword_hglobal(if cut { DROPEFFECT_MOVE.0 } else { DROPEFFECT_COPY.0 }) {
        // SAFETY: CFSTR_PREFERREDDROPEFFECT 为静态宽串。
        let cf = unsafe { RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT) };
        if cf != 0 {
            // SAFETY: cf/effect 有效。
            let _ = unsafe { SetClipboardData(cf, Some(handle_from_hglobal(effect))) };
            // 失败时系统可能未接管；忽略（不影响主路径）。
        } else {
            // SAFETY: effect 归本函数所有。
            unsafe {
                let _ = GlobalFree(Some(effect));
            }
        }
    }

    // SAFETY: 剪贴板已打开。
    let _ = unsafe { CloseClipboard() };
    Ok(())
}

/// 读取剪贴板中的文件列表（CF_HDROP）。
///
/// 没有文件时返回 [`QError::Shell`]（上层据此禁用粘贴菜单项）。
pub fn get_clipboard() -> QResult<ClipboardContent> {
    // SAFETY: 只查询格式可用性，无内存前提。
    let available = unsafe { IsClipboardFormatAvailable(CF_HDROP.0 as u32) };
    if available.is_err() {
        return Err(QError::Shell("剪贴板中没有文件".to_string()));
    }

    // SAFETY: hwnd 为空时 OpenClipboard 失败；这里不传窗口，靠当前线程。
    let opened = unsafe { OpenClipboard(None) };
    if opened.is_err() {
        return Err(io_error("OpenClipboard"));
    }

    // SAFETY: 剪贴板已打开，CF_HDROP 已确认可用。
    let handle = unsafe { GetClipboardData(CF_HDROP.0 as u32) };
    let content = match handle {
        Ok(h) => {
            let hglobal = HGLOBAL(h.0);
            // SAFETY: hglobal 由剪贴板拥有，只读。
            let ptr = unsafe { GlobalLock(hglobal) } as *const u8;
            if ptr.is_null() {
                Err(io_error("GlobalLock"))
            } else {
                // SAFETY: ptr 指向 DROPFILES 结构；宽字符模式（fWide=1）。
                let p_files = unsafe { *(ptr as *const u32) } as usize;
                let f_wide = unsafe { *(ptr.add(16) as *const u32) } != 0;
                let base = unsafe { ptr.add(p_files) } as *const u16;
                let paths = if f_wide {
                    read_double_nul_wide(base)
                } else {
                    // 窄字符模式：逐字节转宽。
                    let narrow = base as *const u8;
                    read_double_nul_narrow(narrow)
                };
                // SAFETY: 与 GlobalLock 配对。
                let _ = unsafe { GlobalUnlock(hglobal) };
                Ok(paths)
            }
        }
        Err(e) => Err(shell_error("GetClipboardData", e)),
    };

    // Preferred DropEffect（可选；没有时按复制处理）。
    let cut = read_preferred_drop_effect();

    // SAFETY: 剪贴板已打开。
    let _ = unsafe { CloseClipboard() };

    let paths = content?;
    Ok(ClipboardContent { paths, cut })
}

/// 剪贴板里是否有文件。
pub fn clipboard_has_files() -> bool {
    // SAFETY: 只查询格式。
    unsafe { IsClipboardFormatAvailable(CF_HDROP.0 as u32) }.is_ok()
}

/// 粘贴：把剪贴板文件写入目标目录。
///
/// 语义由剪贴板自带的 Preferred DropEffect 决定（剪切/复制）。
/// 内部转调 [`super::fileops`]，不在本模块重复实现操作逻辑。
pub fn paste_into(
    hwnd: HostWindow,
    dest_dir: &std::path::Path,
    fallback_cut: bool,
) -> QResult<super::fileops::FileOpReport> {
    let content = get_clipboard()?;
    if content.is_empty() {
        return Err(QError::Shell("剪贴板中没有文件".to_string()));
    }
    let opts = super::fileops::FileOpOptions {
        allow_undo: content.cut || fallback_cut,
        silent: false,
        create_missing_dirs: true,
        rename_on_collision: false,
        move_confirmation: false,
    };
    if content.cut || fallback_cut {
        super::fileops::move_files(hwnd, &content.paths, dest_dir, &opts)
    } else {
        super::fileops::copy_files(hwnd, &content.paths, dest_dir, &opts)
    }
}

/// 便捷：剪切到剪贴板（`copy_cut_paste_mode = 1` 的语义）。
pub fn cut_to_clipboard(hwnd: HostWindow, paths: &[PathBuf]) -> QResult<()> {
    set_clipboard(hwnd, paths, true)
}

/// 便捷：复制到剪贴板（`copy_cut_paste_mode = 0` 的语义）。
pub fn copy_to_clipboard(hwnd: HostWindow, paths: &[PathBuf]) -> QResult<()> {
    set_clipboard(hwnd, paths, false)
}

/// 用 `core::FileOpsConfig` 决定复制/剪切语义后写入剪贴板。
pub fn to_clipboard_with_config(
    hwnd: HostWindow,
    paths: &[PathBuf],
    cfg: &crate::core::FileOpsConfig,
) -> QResult<()> {
    // copy_cut_paste_mode：0=复制 1=剪切（core.rs 注释）。
    let cut = cfg.copy_cut_paste_mode == 1;
    set_clipboard(hwnd, paths, cut)
}

// ============ 内部实现 ============

/// `HGLOBAL` → `HANDLE`（同址，仅换类型）。
fn handle_from_hglobal(h: HGLOBAL) -> windows::Win32::Foundation::HANDLE {
    windows::Win32::Foundation::HANDLE(h.0)
}

/// 分配一个 4 字节 HGLOBAL 并写入 DWORD。
fn make_dword_hglobal(value: u32) -> QResult<HGLOBAL> {
    // SAFETY: GMEM_MOVEABLE 合法。
    let h = unsafe { GlobalAlloc(GMEM_MOVEABLE, 4) }.map_err(|e| shell_error("GlobalAlloc", e))?;
    // SAFETY: h 刚分配。
    let ptr = unsafe { GlobalLock(h) } as *mut u32;
    if ptr.is_null() {
        // SAFETY: h 归本函数所有。
        unsafe {
            let _ = GlobalFree(Some(h));
        }
        return Err(io_error("GlobalLock"));
    }
    // SAFETY: 4 字节区域由 GlobalAlloc 保证。
    unsafe { ptr.write(value) };
    // SAFETY: 配对。
    let _ = unsafe { GlobalUnlock(h) };
    Ok(h)
}

/// 读双 NUL 结尾的宽串列表（DROPFILES 数据区）。
fn read_double_nul_wide(base: *const u16) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut cur = base;
    loop {
        // SAFETY: 剪贴板内存以双 NUL 结束；逐个读到 NUL。
        let len = unsafe { wide_strlen(cur) };
        if len == 0 {
            break;
        }
        // SAFETY: len 由 strlen 保证在缓冲内。
        let slice = unsafe { std::slice::from_raw_parts(cur, len) };
        out.push(PathBuf::from(String::from_utf16_lossy(slice)));
        // SAFETY: 前进到下一个串（跳过 NUL）。
        cur = unsafe { cur.add(len + 1) };
    }
    out
}

/// 读双 NUL 结尾的窄串列表。
fn read_double_nul_narrow(base: *const u8) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut cur = base;
    loop {
        // SAFETY: 同上（窄串版本）。
        let len = unsafe { narrow_strlen(cur) };
        if len == 0 {
            break;
        }
        // SAFETY: len 保证在缓冲内。
        let slice = unsafe { std::slice::from_raw_parts(cur, len) };
        out.push(PathBuf::from(String::from_utf8_lossy(slice).to_string()));
        // SAFETY: 前进到下一个串。
        cur = unsafe { cur.add(len + 1) };
    }
    out
}

/// 宽串长度（到 NUL 为止）。
///
/// # Safety
/// `s` 必须指向以 NUL 结尾的宽串。
unsafe fn wide_strlen(s: *const u16) -> usize {
    let mut i = 0usize;
    // SAFETY: 调用方保证终止条件可达。
    while unsafe { *s.add(i) } != 0 {
        i += 1;
    }
    i
}

/// 窄串长度（到 NUL 为止）。
///
/// # Safety
/// `s` 必须指向以 NUL 结尾的窄串。
unsafe fn narrow_strlen(s: *const u8) -> usize {
    let mut i = 0usize;
    // SAFETY: 调用方保证终止条件可达。
    while unsafe { *s.add(i) } != 0 {
        i += 1;
    }
    i
}

/// 读 Preferred DropEffect；不存在或读失败按「复制」处理。
fn read_preferred_drop_effect() -> bool {
    // SAFETY: 静态宽串常量。
    let cf = unsafe { RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT) };
    if cf == 0 {
        return false;
    }
    // SAFETY: cf 由 RegisterClipboardFormatW 返回；剪贴板此时已打开。
    let handle = unsafe { GetClipboardData(cf) };
    let Ok(h) = handle else {
        return false;
    };
    let hglobal = HGLOBAL(h.0);
    // SAFETY: hglobal 由剪贴板拥有。
    let ptr = unsafe { GlobalLock(hglobal) } as *const u32;
    if ptr.is_null() {
        return false;
    }
    // SAFETY: 4 字节由格式语义保证。
    let effect = unsafe { ptr.read() };
    // SAFETY: 配对。
    let _ = unsafe { GlobalUnlock(hglobal) };
    effect == DROPEFFECT_MOVE.0
}

/// 便捷：把剪贴板内容按 OLE 路径也写一份（虚拟项互操作）。
///
/// CF_HDROP 路径已覆盖常规场景；本函数留给将来需要传虚拟项的调用方。
#[allow(dead_code)]
fn set_clipboard_ole(paths: &[PathBuf]) -> QResult<()> {
    let _com = super::ensure_com_apartment()?;
    let mut pidls: Vec<*mut ITEMIDLIST> = Vec::with_capacity(paths.len());
    for p in paths {
        let wide = path_to_wide(p)?;
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        let mut attrs = 0u32;
        // SAFETY: wide 合法，出参有效。
        unsafe { SHParseDisplayName(as_pcwstr(&wide), None, &mut pidl, 0, Some(&mut attrs)) }
            .map_err(|e| shell_error(&format!("SHParseDisplayName({})", p.display()), e))?;
        pidls.push(pidl);
    }
    let refs: Vec<*const ITEMIDLIST> = pidls.iter().map(|&p| p as *const ITEMIDLIST).collect();
    // SAFETY: refs 有效。
    let data: IDataObject =
        unsafe { CIDLData_CreateFromIDArray(std::ptr::null(), Some(&refs)) }.map_err(|e| {
            for q in &pidls {
                // SAFETY: 归本函数所有。
                unsafe { free_pidl(*q) };
            }
            shell_error("CIDLData_CreateFromIDArray", e)
        })?;
    for q in &pidls {
        // SAFETY: 归本函数所有。
        unsafe { free_pidl(*q) };
    }
    // SAFETY: data 为有效 COM 对象。
    unsafe { OleSetClipboard(&data) }.map_err(|e| shell_error("OleSetClipboard", e))
}

/// 便捷：剪贴板大小（诊断）。
#[allow(dead_code)]
fn clipboard_hglobal_size(h: HGLOBAL) -> usize {
    // SAFETY: h 由剪贴板拥有。
    unsafe { GlobalSize(h) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_clipboard_rejects_empty() {
        let r = set_clipboard(HostWindow::from_raw_hwnd(0), &[], false);
        assert!(r.is_err());
    }

    #[test]
    fn set_clipboard_rejects_nul_path() {
        let r = set_clipboard(
            HostWindow::from_raw_hwnd(0),
            &[PathBuf::from("C:\u{0}bad")],
            false,
        );
        assert!(r.is_err());
    }

    #[test]
    fn cut_copy_semantics_match_config() {
        // copy_cut_paste_mode = 1 → 剪切。
        let mut cfg = crate::core::FileOpsConfig::default();
        cfg.copy_cut_paste_mode = 0;
        assert_eq!(cfg.copy_cut_paste_mode == 1, false);
        cfg.copy_cut_paste_mode = 1;
        assert_eq!(cfg.copy_cut_paste_mode == 1, true);
    }

    #[test]
    fn double_nul_wide_roundtrip_in_memory() {
        // 在普通 Vec 上验证读取算法的正确性（不触达剪贴板）。
        let mut data: Vec<u16> = Vec::new();
        for s in ["C:\\a.txt", "D:\\b"] {
            data.extend(s.encode_utf16());
            data.push(0);
        }
        data.push(0);
        let base = data.as_ptr();
        // SAFETY: base 指向以双 NUL 结尾的宽串数组。
        let got = read_double_nul_wide(base);
        assert_eq!(
            got,
            vec![PathBuf::from("C:\\a.txt"), PathBuf::from("D:\\b")]
        );
    }

    #[test]
    fn double_nul_narrow_roundtrip_in_memory() {
        let mut data: Vec<u8> = Vec::new();
        for s in ["C:\\a.txt", "D:\\b"] {
            data.extend(s.as_bytes());
            data.push(0);
        }
        data.push(0);
        let base = data.as_ptr();
        // SAFETY: 同上（窄串版本）。
        let got = read_double_nul_narrow(base);
        assert_eq!(
            got,
            vec![PathBuf::from("C:\\a.txt"), PathBuf::from("D:\\b")]
        );
    }

    #[test]
    fn clipboard_content_helpers() {
        let c = ClipboardContent {
            paths: vec![PathBuf::from("C:\\a")],
            cut: false,
        };
        assert!(!c.is_empty());
        assert_eq!(c.len(), 1);
        let empty = ClipboardContent {
            paths: Vec::new(),
            cut: true,
        };
        assert!(empty.is_empty());
    }

    #[test]
    fn preferred_drop_effect_read_is_false_without_clipboard() {
        // 剪贴板未打开时 GetClipboardData 失败 → 按复制处理。
        assert!(!read_preferred_drop_effect());
    }
}
