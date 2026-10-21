//! 目录枚举：IShellFolder + SHGetFileInfoW + FindFirstFileW → `core::FileItem`。
//!
//! 三条路径按可靠性分层：
//! 1. [`list_directory`]：FindFirstFileW + SHGetFileInfoW，纯文件系统目录（最快、最稳）；
//! 2. [`list_directory_detailed`]：同上，但额外取 szDisplayName / szTypeName /
//!    完整属性位，供详情视图与「类型」列使用；
//! 3. [`list_shell_folder`]：IShellFolder::EnumObjects，用于虚拟文件夹
//!    （此电脑、库、压缩包等非文件系统命名空间）。
//!
//! 转换规则与证据/契约的对应：
//! - `FileItem.is_dir` ← FILE_ATTRIBUTE_DIRECTORY 或 SFGAO_FOLDER；
//! - `FileItem.is_hidden` ← HIDDEN | SYSTEM（#Hidden 规则消费此位，core.rs:126）；
//! - `FileItem.icon_index` ← SHGetFileInfoW 的 `iIcon`（SHGFI_SYSICONINDEX），
//!   由上层交给 icons 模块缓存（契约 modules 中 icons.rs 的职责）；
//! - 大小/时间只对文件系统项有意义，虚拟项填 0。

use std::path::Path;

use windows::Win32::Foundation::{HWND, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    FindClose, FindFirstFileW, FindNextFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN,
    FILE_ATTRIBUTE_SYSTEM, WIN32_FIND_DATAW,
};
use windows::Win32::System::Com::IBindCtx;
use windows::Win32::UI::Shell::{
    IShellFolder, SHCONTF_FOLDERS, SHCONTF_INCLUDEHIDDEN, SHCONTF_NONFOLDERS,
    SHGFI_DISPLAYNAME, SHGFI_SYSICONINDEX, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES,
    SHGetDesktopFolder, SHGetFileInfoW, SHGDN_FORPARSING, SHParseDisplayName, StrRetToBufW,
};
use windows::Win32::UI::Shell::Common::{ITEMIDLIST, STRRET};

use super::{
    as_pcwstr, dir_with_backslash, free_pidl, from_wide_lossy, io_error, last_error_code,
    path_to_wide, shell_error, to_wide,
};
use crate::core::{FileItem, QError, QResult};

/// 枚举选项。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumOptions {
    /// 是否包含隐藏/系统项。false 时调用方拿到的是过滤后的列表。
    pub include_hidden: bool,
    /// 是否请求系统图标索引（SHGFI_SYSICONINDEX）。关掉可省一次图标查询。
    pub want_icon_index: bool,
    /// 是否请求显示名与类型名（详情视图的「名称」「类型」列）。
    pub want_display_info: bool,
}

impl Default for EnumOptions {
    fn default() -> Self {
        EnumOptions {
            include_hidden: true,
            want_icon_index: true,
            want_display_info: true,
        }
    }
}

/// `core::FileItem` 之外的 Shell 附加信息（不进 core 类型，契约 A1）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileItemExt {
    /// SHGetFileInfoW 的 szDisplayName（可能与文件系统名不同，如本地化名）。
    pub display_name: Option<String>,
    /// SHGetFileInfoW 的 szTypeName（如「应用程序」「文本文档」）。
    pub type_name: Option<String>,
    /// SHGetFileInfoW 的 dwAttributes 原始位（SHGFI_ATTRIBUTES）。
    pub attributes: u32,
    /// 是否为快捷方式（SFGAO_LINK）。
    pub is_shortcut: bool,
    /// 是否为驱动器根（枚举「此电脑」时使用）。
    pub is_drive: bool,
}

/// 目录枚举主入口：FindFirstFileW + SHGetFileInfoW。
///
/// `path` 必须是已存在的文件系统目录；返回的列表**不含** `.` 与 `..`。
/// 上层按 `include_hidden` 自行决定是否过滤隐藏项（本函数始终返回全部，
/// 过滤属于视图职责；选项仅控制是否查询隐藏位）。
pub fn list_directory(path: &Path, opts: &EnumOptions) -> QResult<Vec<FileItem>> {
    // dir_with_backslash 返回 NUL 结尾宽串；先去掉 NUL 再拼 "*"，
    // 否则 pattern 变成 "目录<NUL>*<NUL>"，FindFirstFileW 截断后行为未定义。
    let mut pattern = dir_with_backslash(path)?;
    truncate_trailing_nul(&mut pattern);
    pattern.push(b'*' as u16);
    pattern.push(0);

    let mut items = Vec::new();
    enumerate_find_first(&pattern, path, opts, &mut items)?;
    Ok(items)
}

/// 去掉宽串末尾的 NUL（`dir_with_backslash`/`path_to_wide` 的产物）。
fn truncate_trailing_nul(v: &mut Vec<u16>) {
    while v.last() == Some(&0) {
        v.pop();
    }
}

/// 同 [`list_directory`]，但同时返回 [`FileItemExt`]（详情视图使用）。
pub fn list_directory_detailed(
    path: &Path,
    opts: &EnumOptions,
) -> QResult<Vec<(FileItem, FileItemExt)>> {
    let mut pattern = dir_with_backslash(path)?;
    truncate_trailing_nul(&mut pattern);
    pattern.push(b'*' as u16);
    pattern.push(0);

    let mut out = Vec::new();
    enumerate_find_first_ext(&pattern, path, opts, &mut out)?;
    Ok(out)
}

/// 用 IShellFolder::EnumObjects 枚举**任意**命名空间（含虚拟文件夹）。
///
/// `parsing_path` 既可传文件系统路径，也可传 `::{GUID}` 形式的命名空间串。
pub fn list_shell_folder(parsing_path: &str) -> QResult<Vec<FileItem>> {
    let path = Path::new(parsing_path);
    let (folder, _folder_pidl) = shell_folder_for(parsing_path)?;
    let items = enum_folder(&folder, path)?;
    Ok(items)
}

/// 判断字符串是否指向文件系统路径（而非 `::{GUID}` 虚拟命名空间）。
pub fn is_filesystem_path(s: &str) -> bool {
    !s.starts_with("::") && !s.contains("::")
}

// ============ FindFirstFileW 路径 ============

fn enumerate_find_first(
    pattern: &[u16],
    dir: &Path,
    opts: &EnumOptions,
    out: &mut Vec<FileItem>,
) -> QResult<()> {
    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    // SAFETY: pattern 为 NUL 结尾宽串，data 为合法出参；句柄在下方统一关闭。
    let handle = unsafe { FindFirstFileW(as_pcwstr(pattern), &mut data) }
        .map_err(|e| shell_error("FindFirstFileW", e))?;

    let result = (|| {
        loop {
            push_find_data(dir, &data, opts, out)?;
            // SAFETY: handle 有效，data 可重复写入。
            if let Err(e) = unsafe { FindNextFileW(handle, &mut data) } {
                let code = last_error_code();
                // 18 = ERROR_NO_MORE_FILES：正常结束。
                if code == 18 {
                    return Ok(());
                }
                return Err(shell_error("FindNextFileW", e));
            }
        }
    })();

    // SAFETY: handle 由 FindFirstFileW 成功返回，未重复关闭。
    let _ = unsafe { FindClose(handle) };
    result
}

fn enumerate_find_first_ext(
    pattern: &[u16],
    dir: &Path,
    opts: &EnumOptions,
    out: &mut Vec<(FileItem, FileItemExt)>,
) -> QResult<()> {
    let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    // SAFETY: 同上。
    let handle = unsafe { FindFirstFileW(as_pcwstr(pattern), &mut data) }
        .map_err(|e| shell_error("FindFirstFileW", e))?;

    let result = (|| {
        loop {
            push_find_data_ext(dir, &data, opts, out)?;
            // SAFETY: handle 有效，data 可重复写入。
            if let Err(e) = unsafe { FindNextFileW(handle, &mut data) } {
                let code = last_error_code();
                if code == 18 {
                    return Ok(());
                }
                return Err(shell_error("FindNextFileW", e));
            }
        }
    })();

    // SAFETY: handle 由 FindFirstFileW 成功返回。
    let _ = unsafe { FindClose(handle) };
    result
}

/// 跳过 `.` / `..`。
fn is_dot_entry(name: &str) -> bool {
    name == "." || name == ".."
}

/// FILE_ATTRIBUTE_* 位 → FileItem。
fn attrs_to_flags(attrs: u32) -> (bool, bool) {
    let is_dir = attrs & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    let is_hidden = attrs & (FILE_ATTRIBUTE_HIDDEN.0 | FILE_ATTRIBUTE_SYSTEM.0) != 0;
    (is_dir, is_hidden)
}

fn find_data_to_item(dir: &Path, data: &WIN32_FIND_DATAW) -> Option<FileItem> {
    let name = from_wide_lossy(&data.cFileName);
    if is_dot_entry(&name) {
        return None;
    }
    let (is_dir, is_hidden) = attrs_to_flags(data.dwFileAttributes);
    let size = ((data.nFileSizeHigh as u64) << 32) | data.nFileSizeLow as u64;
    let modified = ((data.ftLastWriteTime.dwHighDateTime as u64) << 32)
        | data.ftLastWriteTime.dwLowDateTime as u64;
    let path = dir.join(&name);
    Some(FileItem {
        path,
        name,
        is_dir,
        is_hidden,
        size: if is_dir { 0 } else { size },
        modified,
        icon_index: None,
    })
}

fn push_find_data(
    dir: &Path,
    data: &WIN32_FIND_DATAW,
    opts: &EnumOptions,
    out: &mut Vec<FileItem>,
) -> QResult<()> {
    let Some(mut item) = find_data_to_item(dir, data) else {
        return Ok(());
    };
    if opts.want_icon_index {
        item.icon_index = query_icon_index(&item.path, data.dwFileAttributes);
    }
    if !opts.include_hidden {
        // 调用方要求排除隐藏项：这里丢弃，位已由 core 消费（#Hidden 规则）。
        if item.is_hidden {
            return Ok(());
        }
    }
    out.push(item);
    Ok(())
}

fn push_find_data_ext(
    dir: &Path,
    data: &WIN32_FIND_DATAW,
    opts: &EnumOptions,
    out: &mut Vec<(FileItem, FileItemExt)>,
) -> QResult<()> {
    let Some(mut item) = find_data_to_item(dir, data) else {
        return Ok(());
    };
    if !opts.include_hidden && item.is_hidden {
        return Ok(());
    }
    let mut ext = FileItemExt {
        attributes: data.dwFileAttributes,
        ..Default::default()
    };
    if opts.want_icon_index {
        item.icon_index = query_icon_index(&item.path, data.dwFileAttributes);
    }
    if opts.want_display_info {
        let info = query_file_info(&item.path, data.dwFileAttributes, SHGFI_DISPLAYNAME | SHGFI_TYPENAME);
        if let Some(sfi) = info {
            ext.display_name = Some(from_wide_lossy(&sfi.szDisplayName));
            ext.type_name = Some(from_wide_lossy(&sfi.szTypeName));
            if ext.attributes == 0 {
                ext.attributes = sfi.dwAttributes;
            }
        }
    }
    out.push((item, ext));
    Ok(())
}

// ============ SHGetFileInfoW 封装 ============

/// SHFILEINFOW 的 Rust 侧视图（windows crate 的字段是固定数组，直接复用）。
type ShFileInfoW = windows::Win32::UI::Shell::SHFILEINFOW;

/// 取系统图标索引（iIcon，SHGFI_SYSICONINDEX）。
///
/// 失败返回 None —— 上层用内置兜底图标（core.rs:126 的约定）。
fn query_icon_index(path: &Path, attrs: u32) -> Option<i32> {
    let info = query_file_info(
        path,
        attrs,
        SHGFI_SYSICONINDEX | SHGFI_USEFILEATTRIBUTES,
    )?;
    Some(info.iIcon)
}

/// 通用 SHGetFileInfoW 调用：按 `flags` 取信息。
///
/// `attrs` 是调用方已知的文件属性；传 0 时 API 自己去查（路径必须存在）。
fn query_file_info(
    path: &Path,
    attrs: u32,
    flags: windows::Win32::UI::Shell::SHGFI_FLAGS,
) -> Option<ShFileInfoW> {
    let wide = path_to_wide(path).ok()?;
    let mut sfi: ShFileInfoW = unsafe { std::mem::zeroed() };
    // SAFETY: wide 为 NUL 结尾宽串，sfi 为合法出参，cb 为 size_of。
    let r = unsafe {
        SHGetFileInfoW(
            as_pcwstr(&wide),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(attrs),
            Some(&mut sfi),
            super::shfileinfo_size(),
            flags,
        )
    };
    if r == 0 {
        return None;
    }
    Some(sfi)
}

// ============ IShellFolder 路径（虚拟文件夹） ============

/// 取得命名空间的 IShellFolder 与其 PIDL。
///
/// 返回的 PIDL 由调用方负责 `CoTaskMemFree`。
fn shell_folder_for(
    parsing_path: &str,
) -> QResult<(IShellFolder, *mut ITEMIDLIST)> {
    let wide = to_wide(parsing_path);
    let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
    let mut attrs: u32 = 0;
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
    .map_err(|e| shell_error(&format!("SHParseDisplayName({parsing_path})"), e))?;

    if pidl.is_null() {
        return Err(QError::Shell(format!(
            "SHParseDisplayName 未产出 PIDL：{parsing_path}"
        )));
    }

    let _desktop = unsafe { SHGetDesktopFolder() }
        .map_err(|e| shell_error("SHGetDesktopFolder", e))?;
    let mut last: *mut ITEMIDLIST = std::ptr::null_mut();
    // SHBindToParent 返回父文件夹（桌面），last 指向 pidl 的最后一个标识符。
    let folder: IShellFolder = unsafe {
        windows::Win32::UI::Shell::SHBindToParent(pidl, Some(&mut last))
    }
    .map_err(|e| {
        // 绑定失败也要释放 pidl。
        // SAFETY: pidl 由 SHParseDisplayName 分配，尚未交给任何人。
        unsafe { free_pidl(pidl) };
        shell_error("SHBindToParent", e)
    })?;

    Ok((folder, pidl))
}

/// 枚举 IShellFolder 的全部子项。
fn enum_folder(folder: &IShellFolder, base: &Path) -> QResult<Vec<FileItem>> {
    let mut enum_list = None;
    // SAFETY: folder 为有效接口，ppenumidlist 为出参；hwnd 传空（枚举不需要窗口）。
    let hr = unsafe {
        folder.EnumObjects(
            HWND(std::ptr::null_mut()),
            (SHCONTF_FOLDERS.0 | SHCONTF_NONFOLDERS.0 | SHCONTF_INCLUDEHIDDEN.0) as u32,
            &mut enum_list,
        )
    };
    // S_FALSE(1) 表示无子项，属正常情况。
    if hr.0 != 0 && hr.0 != 1 {
        return Err(shell_error("IShellFolder::EnumObjects", windows::core::Error::from_hresult(hr)));
    }
    let Some(enum_list) = enum_list else {
        return Ok(Vec::new());
    };

    let mut out = Vec::new();
    loop {
        let mut fetched: u32 = 0;
        let mut batch: [*mut ITEMIDLIST; 32] = [std::ptr::null_mut(); 32];
        // SAFETY: batch 为合法出参数组，fetched 为计数出参。
        let hr = unsafe { enum_list.Next(&mut batch, Some(&mut fetched)) };
        if fetched == 0 {
            // S_OK/S_FALSE 均表示结束（windows crate 的 Next 返回 HRESULT）。
            let _ = hr;
            break;
        }
        for &pidl in batch.iter().take(fetched as usize) {
            if pidl.is_null() {
                continue;
            }
            let (item, ext) = pidl_to_item(folder, pidl, base);
            let _ = ext;
            if let Some(item) = item {
                out.push(item);
            }
            // SAFETY: 枚举器每次 Next 分配的 PIDL 归调用方。
            unsafe { free_pidl(pidl) };
        }
        if hr.0 != 0 {
            break;
        }
    }
    Ok(out)
}

/// PIDL → FileItem（显示名 + 属性 + 解析路径）。
fn pidl_to_item(
    folder: &IShellFolder,
    pidl: *mut ITEMIDLIST,
    base: &Path,
) -> (Option<FileItem>, FileItemExt) {
    let mut ext = FileItemExt::default();

    // 显示名（SHGDN_INFOLDER 给「在文件夹内」的名字）。
    let mut strret: STRRET = unsafe { std::mem::zeroed() };
    // SAFETY: strret 为出参。
    let name = unsafe { folder.GetDisplayNameOf(pidl, SHGDN_FORPARSING, &mut strret) }
        .ok()
        .and_then(|_| strret_to_string(&mut strret, Some(pidl)))
        .unwrap_or_default();
    ext.display_name = Some(name.clone());
    if name.is_empty() {
        return (None, ext);
    }

    // 属性位。
    let mut attrs: u32 = 0xFFFFFFFF;
    let single = [pidl as *const ITEMIDLIST];
    // SAFETY: single 为 1 元素切片，rgfinout 为出参。
    if unsafe { folder.GetAttributesOf(&single, &mut attrs) }.is_ok() {
        ext.attributes = attrs;
    }

    let is_dir = attrs & 0x20000000 != 0; // SFGAO_FOLDER
    let is_hidden = attrs & 0x00080000 != 0; // SFGAO_HIDDEN
    ext.is_shortcut = attrs & 0x00010000 != 0; // SFGAO_LINK
    ext.is_drive = attrs & 0x08000000 != 0; // SFGAO_DRIVE

    let path = if name.starts_with("::") {
        base.join(name.trim_start_matches(':'))
    } else {
        base.join(&name)
    };

    let item = FileItem {
        path,
        name: name
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(name.as_str())
            .to_string(),
        is_dir,
        is_hidden,
        size: 0,
        modified: 0,
        icon_index: None,
    };
    (Some(item), ext)
}

/// STRRET → String（WSTR 类型由 CoTaskMem 分配，需释放）。
fn strret_to_string(strret: &mut STRRET, pidl: Option<*const ITEMIDLIST>) -> Option<String> {
    let mut buf = [0u16; 520];
    // SAFETY: strret/pidl 由调用方保证有效，buf 为出参。
    unsafe { StrRetToBufW(strret as *mut STRRET, pidl, &mut buf) }
        .ok()
        .map(|_| from_wide_lossy(&buf))
}

/// 便捷：一次性取某路径的类型名（属性对话框/详情视图的「类型」列）。
pub fn type_name_of(path: &Path) -> QResult<String> {
    let wide = path_to_wide(path)?;
    let mut sfi: ShFileInfoW = unsafe { std::mem::zeroed() };
    // SAFETY: wide 合法，sfi 为出参。
    let r = unsafe {
        SHGetFileInfoW(
            as_pcwstr(&wide),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi),
            super::shfileinfo_size(),
            SHGFI_TYPENAME,
        )
    };
    if r == 0 {
        return Err(io_error("SHGetFileInfoW(TYPENAME)"));
    }
    Ok(from_wide_lossy(&sfi.szTypeName))
}

/// 便捷：一次性取某路径的本地化显示名。
pub fn display_name_of(path: &Path) -> QResult<String> {
    let wide = path_to_wide(path)?;
    let mut sfi: ShFileInfoW = unsafe { std::mem::zeroed() };
    // SAFETY: 同上。
    let r = unsafe {
        SHGetFileInfoW(
            as_pcwstr(&wide),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi),
            super::shfileinfo_size(),
            SHGFI_DISPLAYNAME,
        )
    };
    if r == 0 {
        return Err(io_error("SHGetFileInfoW(DISPLAYNAME)"));
    }
    Ok(from_wide_lossy(&sfi.szDisplayName))
}

/// 关闭 FindFirstFileW 句柄的小工具（测试与错误路径共用）。
#[allow(dead_code)]
fn close_find_handle(handle: HANDLE) {
    if !handle.is_invalid() {
        // SAFETY: handle 来自 FindFirstFileW。
        let _ = unsafe { FindClose(handle) };
    }
}

/// `INVALID_HANDLE_VALUE` 判定（windows crate 已提供 is_invalid，这里只做桥接）。
#[allow(dead_code)]
fn is_invalid(handle: HANDLE) -> bool {
    handle == INVALID_HANDLE_VALUE
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn item(name: &str, is_dir: bool, hidden: bool) -> FileItem {
        FileItem {
            path: PathBuf::from(format!("C:\\x\\{name}")),
            name: name.to_string(),
            is_dir,
            is_hidden: hidden,
            size: 1,
            modified: 2,
            icon_index: None,
        }
    }

    #[test]
    fn attrs_map_to_dir_and_hidden_bits() {
        let (d, h) = attrs_to_flags(FILE_ATTRIBUTE_DIRECTORY.0);
        assert!(d && !h);
        let (d, h) = attrs_to_flags(FILE_ATTRIBUTE_HIDDEN.0);
        assert!(!d && h);
        let (d, h) = attrs_to_flags(FILE_ATTRIBUTE_SYSTEM.0);
        assert!(!d && h, "SYSTEM 与 HIDDEN 一样触发 #Hidden 规则");
        let (d, h) = attrs_to_flags(0);
        assert!(!d && !h);
    }

    #[test]
    fn dot_entries_are_skipped() {
        assert!(is_dot_entry("."));
        assert!(is_dot_entry(".."));
        assert!(!is_dot_entry("..."));
        assert!(!is_dot_entry("a.txt"));
    }

    #[test]
    fn filesystem_path_detection() {
        assert!(is_filesystem_path("C:\\Windows"));
        assert!(is_filesystem_path("\\\\server\\share"));
        assert!(!is_filesystem_path("::{20D04FE0-3AEA-1069-A2D8-08002B30309D}"));
        assert!(!is_filesystem_path("::{031E4825-7B94-4DC3-B131-E946B44C8DD5}"));
    }

    #[test]
    fn find_data_conversion_keeps_size_and_time() {
        let mut data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
        data.dwFileAttributes = FILE_ATTRIBUTE_DIRECTORY.0;
        data.nFileSizeLow = 1234;
        data.nFileSizeHigh = 0;
        data.ftLastWriteTime = windows::Win32::Foundation::FILETIME {
            dwLowDateTime: 7,
            dwHighDateTime: 0,
        };
        let name: Vec<u16> = "sub".encode_utf16().chain([0]).collect();
        data.cFileName[..name.len()].copy_from_slice(&name);

        let it = find_data_to_item(Path::new("C:\\base"), &data).expect("应产出条目");
        assert_eq!(it.name, "sub");
        assert!(it.is_dir);
        assert_eq!(it.size, 0, "目录 size 归零");
        assert_eq!(it.modified, 7);
        assert_eq!(it.path, PathBuf::from("C:\\base\\sub"));

        // 文件保留 size。
        data.dwFileAttributes = 0;
        let it = find_data_to_item(Path::new("C:\\base"), &data).expect("应产出条目");
        assert_eq!(it.size, 1234);
        assert!(!it.is_dir);
    }

    #[test]
    fn enum_options_default_wants_everything() {
        let o = EnumOptions::default();
        assert!(o.include_hidden);
        assert!(o.want_icon_index);
        assert!(o.want_display_info);
    }

    #[test]
    fn ext_default_is_empty() {
        assert_eq!(FileItemExt::default(), FileItemExt {
            display_name: None,
            type_name: None,
            attributes: 0,
            is_shortcut: false,
            is_drive: false,
        });
    }

    #[test]
    fn size_of_shfileinfo_is_reasonable() {
        // x64 上 SHFILEINFOW 应为 0x2E0 字节左右（0x218 + 0x1A0）。
        assert!(super::super::shfileinfo_size() >= 0x200);
    }

    #[test]
    fn dummy_item_compiles() {
        let i = item("a", true, false);
        assert!(i.is_dir);
    }
}
