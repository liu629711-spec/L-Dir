//! 特殊文件夹、虚拟命名空间与驱动器枚举。
//!
//! 覆盖任务要求的三类路径：
//! - 特殊文件夹：`SHGetSpecialFolderPathW` / `SHGetFolderPathW`（文档、下载等）；
//! - 虚拟命名空间：我的电脑 `::{20D04FE0-3AEA-1069-A2D8-08002B30309D}`、
//!   库 `::{031E4825-7B94-4DC3-B131-E946B44C8DD5}`（CLSID 形式）；
//! - 驱动器枚举：`GetLogicalDrives` + `GetDriveTypeW`，返回 [`DriveInfo`]
//!   （上层用 `is_dir`/`is_hidden` 之外的信息渲染盘符图标）。
//!
//! 命名空间串的双向转换：[`parse_shell_path`] 把 UI 上的字符串解析成
//! [`ShellLocation`]（文件系统 / 虚拟 / 驱动器根），`*_path()` 系列生成
//! 可直接交给 [`super::enumerate::list_shell_folder`] 的串。

use std::path::PathBuf;

use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
use windows::Win32::UI::Shell::{
    CSIDL_DESKTOP, CSIDL_PERSONAL,
    SHGetFolderPathW, SHGetSpecialFolderPathW,
};

use super::{io_error, shell_error, to_wide, from_wide_lossy};
use crate::core::QResult;

/// 我的电脑 CLSID（任务要求：`::{20D04FE0-...}`）。
pub const CLSID_THIS_PC: &str = "::{20D04FE0-3AEA-1069-A2D8-08002B30309D}";
/// 库 CLSID（任务要求：`::{031E4825-7B94-4DC3-B131-E946B44C8DD5}`）。
pub const CLSID_LIBRARIES: &str = "::{031E4825-7B94-4DC3-B131-E946B44C8DD5}";
/// 回收站 CLSID（常见形态）。
pub const CLSID_RECYCLE_BIN: &str = "::{645FF040-5081-101B-9F08-00AA002F954E}";
/// 网络 CLSID。
pub const CLSID_NETWORK: &str = "::{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}";

/// 驱动器种类。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DriveKind {
    /// 本地固定磁盘。
    Fixed,
    /// 可移动磁盘（U 盘等）。
    Removable,
    /// 光盘。
    Optical,
    /// 网络驱动器。
    Network,
    /// RAM 磁盘。
    RamDisk,
    /// 未知/未就绪。
    Unknown,
}

impl DriveKind {
    /// 从 `GetDriveTypeW` 返回值转换。
    fn from_raw(v: u32) -> Self {
        match v {
            0 => DriveKind::Unknown,
            2 => DriveKind::Removable,
            3 => DriveKind::Fixed,
            4 => DriveKind::Network,
            5 => DriveKind::Optical,
            6 => DriveKind::RamDisk,
            _ => DriveKind::Unknown,
        }
    }
}

/// 一个驱动器的信息。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DriveInfo {
    /// 根路径，如 `C:\`（尾部反斜杠，与 GetDriveTypeW 的输入一致）。
    pub root: PathBuf,
    /// 盘符（大写，如 `C`）。
    pub letter: char,
    /// 种类。
    pub kind: DriveKind,
}

/// 命名空间位置（UI 串的解析结果）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellLocation {
    /// 文件系统路径。
    FileSystem(PathBuf),
    /// 虚拟命名空间（CLSID 形式）。
    Virtual(String),
    /// 驱动器根（`C:\`）。
    Drive(DriveInfo),
}

/// 解析 UI 上的路径串。
///
/// `C:\x` → FileSystem；`::{GUID}` → Virtual；`C:\` → Drive。
pub fn parse_shell_path(s: &str) -> ShellLocation {
    let trimmed = s.trim();
    if trimmed.starts_with("::") {
        return ShellLocation::Virtual(trimmed.to_string());
    }
    // 驱动器根形如 "C:\" 或 "C:"。
    let bytes: Vec<char> = trimmed.chars().collect();
    if bytes.len() >= 2 && bytes[1] == ':' && (bytes.len() == 2 || (bytes.len() == 3 && bytes[2] == '\\'))
    {
        let letter = bytes[0].to_ascii_uppercase();
        let root = format!("{letter}:\\");
        let kind = drive_kind(&root);
        return ShellLocation::Drive(DriveInfo {
            root: PathBuf::from(root),
            letter,
            kind,
        });
    }
    ShellLocation::FileSystem(PathBuf::from(trimmed))
}

/// 枚举全部逻辑驱动器。
pub fn list_drives() -> Vec<DriveInfo> {
    let mut out = Vec::new();
    // SAFETY: 无参数，返回位掩码。
    let mask = unsafe { GetLogicalDrives() };
    for i in 0..26u32 {
        if mask & (1 << i) != 0 {
            let letter = (b'A' + i as u8) as char;
            let root = format!("{letter}:\\");
            let kind = drive_kind(&root);
            out.push(DriveInfo {
                root: PathBuf::from(root),
                letter,
                kind,
            });
        }
    }
    out
}

/// 查询单个驱动器的种类。
fn drive_kind(root: &str) -> DriveKind {
    let wide = to_wide(root);
    // SAFETY: wide 为 NUL 结尾宽串。
    let v = unsafe { GetDriveTypeW(super::as_pcwstr(&wide)) };
    DriveKind::from_raw(v)
}

/// 特殊文件夹路径（CSIDL 版本，兼容性好）。
///
/// `csidl` 取 `windows::Win32::UI::Shell::CSIDL_*` 常量。
pub fn special_folder_path(csidl: i32) -> QResult<PathBuf> {
    let mut buf = [0u16; 260];
    // SAFETY: buf 为出参（260 = MAX_PATH）。
    let ok = unsafe { SHGetSpecialFolderPathW(None, &mut buf, csidl, false) };
    if !ok.as_bool() {
        return Err(io_error("SHGetSpecialFolderPathW"));
    }
    Ok(PathBuf::from(from_wide_lossy(&buf)))
}

/// 特殊文件夹路径（`SHGetFolderPathW`，支持 KNOWNFOLDER）。
pub fn folder_path(csidl: i32) -> QResult<PathBuf> {
    let mut buf = [0u16; 260];
    // SAFETY: buf 为出参。
    unsafe { SHGetFolderPathW(None, csidl, None, 0, &mut buf) }
        .map_err(|e| shell_error("SHGetFolderPathW", e))?;
    Ok(PathBuf::from(from_wide_lossy(&buf)))
}

/// 「我的电脑」命名空间串。
pub fn this_pc_path() -> String {
    CLSID_THIS_PC.to_string()
}

/// 「库」命名空间串。
pub fn libraries_path() -> String {
    CLSID_LIBRARIES.to_string()
}

/// 「网络」命名空间串。
pub fn network_path() -> String {
    CLSID_NETWORK.to_string()
}

/// 回收站命名空间串。
pub fn recycle_bin_path() -> String {
    CLSID_RECYCLE_BIN.to_string()
}

/// 便捷：用户「文档」文件夹。
pub fn documents_path() -> QResult<PathBuf> {
    folder_path(CSIDL_PERSONAL as i32)
}

/// 便捷：用户「桌面」文件夹。
pub fn desktop_path() -> QResult<PathBuf> {
    folder_path(CSIDL_DESKTOP as i32)
}

/// 便捷：判断是否为「我的电脑」命名空间。
pub fn is_this_pc(s: &str) -> bool {
    s.eq_ignore_ascii_case(CLSID_THIS_PC)
}

/// 便捷：判断是否为「库」命名空间。
pub fn is_libraries(s: &str) -> bool {
    s.eq_ignore_ascii_case(CLSID_LIBRARIES)
}

/// 便捷：判断是否为驱动器根。
pub fn is_drive_root(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy();
    let chars: Vec<char> = s.chars().collect();
    chars.len() == 3 && chars[1] == ':' && chars[2] == '\\'
}

/// 便捷：取路径的驱动器根（`C:\a\b` → `C:\`）。
pub fn drive_root(path: &std::path::Path) -> Option<PathBuf> {
    let s = path.to_string_lossy();
    let chars: Vec<char> = s.chars().collect();
    if chars.len() >= 2 && chars[1] == ':' {
        let mut root = String::new();
        root.push(chars[0].to_ascii_uppercase());
        root.push(':');
        root.push('\\');
        return Some(PathBuf::from(root));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_filesystem_path() {
        assert_eq!(
            parse_shell_path("C:\\Windows\\System32"),
            ShellLocation::FileSystem(PathBuf::from("C:\\Windows\\System32"))
        );
        assert_eq!(
            parse_shell_path("  C:\\tmp  "),
            ShellLocation::FileSystem(PathBuf::from("C:\\tmp"))
        );
    }

    #[test]
    fn parse_virtual_namespace() {
        assert_eq!(
            parse_shell_path(CLSID_THIS_PC),
            ShellLocation::Virtual(CLSID_THIS_PC.to_string())
        );
        assert_eq!(
            parse_shell_path(CLSID_LIBRARIES),
            ShellLocation::Virtual(CLSID_LIBRARIES.to_string())
        );
    }

    #[test]
    fn parse_drive_root() {
        match parse_shell_path("C:\\") {
            ShellLocation::Drive(d) => {
                assert_eq!(d.letter, 'C');
                assert_eq!(d.root, PathBuf::from("C:\\"));
            }
            other => panic!("应解析为驱动器根，实际 {other:?}"),
        }
        match parse_shell_path("d:") {
            ShellLocation::Drive(d) => assert_eq!(d.letter, 'D'),
            other => panic!("应解析为驱动器根，实际 {other:?}"),
        }
        // 两位以上（如 C:\x）不是根。
        assert!(matches!(
            parse_shell_path("C:\\x"),
            ShellLocation::FileSystem(_)
        ));
    }

    #[test]
    fn drive_kind_from_raw_values() {
        assert_eq!(DriveKind::from_raw(0), DriveKind::Unknown);
        assert_eq!(DriveKind::from_raw(2), DriveKind::Removable);
        assert_eq!(DriveKind::from_raw(3), DriveKind::Fixed);
        assert_eq!(DriveKind::from_raw(4), DriveKind::Network);
        assert_eq!(DriveKind::from_raw(5), DriveKind::Optical);
        assert_eq!(DriveKind::from_raw(6), DriveKind::RamDisk);
        assert_eq!(DriveKind::from_raw(99), DriveKind::Unknown);
    }

    #[test]
    fn namespace_helpers_return_clsid_strings() {
        assert_eq!(this_pc_path(), CLSID_THIS_PC);
        assert_eq!(libraries_path(), CLSID_LIBRARIES);
        assert_eq!(network_path(), CLSID_NETWORK);
        assert_eq!(recycle_bin_path(), CLSID_RECYCLE_BIN);
        assert!(is_this_pc(CLSID_THIS_PC));
        assert!(is_libraries(CLSID_LIBRARIES));
        assert!(!is_this_pc(CLSID_LIBRARIES));
    }

    #[test]
    fn drive_root_detection_and_extraction() {
        assert!(is_drive_root(Path::new("C:\\")));
        assert!(!is_drive_root(Path::new("C:\\x")));
        assert!(!is_drive_root(Path::new("C:")));
        assert_eq!(
            drive_root(Path::new("C:\\a\\b")),
            Some(PathBuf::from("C:\\"))
        );
        assert_eq!(
            drive_root(Path::new("d:\\")),
            Some(PathBuf::from("D:\\"))
        );
        assert_eq!(drive_root(Path::new("\\\\server\\share")), None);
    }

    #[test]
    fn clsid_constants_are_lowercase_braced() {
        // 与任务要求的字面量一致。
        assert!(CLSID_THIS_PC.starts_with("::{20D04FE0-"));
        assert!(CLSID_LIBRARIES.starts_with("::{031E4825-"));
    }
}
