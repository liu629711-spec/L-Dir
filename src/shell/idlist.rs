//! 路径 ↔ `.qdr` `FolderN` 的 IDLIST 百分号编码（E3 格式）。
//!
//! 原版实测（`D:\Q-Dir_Portable_x64\Q-Dir\start.qdr`）：
//! `Folder2=%14%00%1F%50…%9D%00%00&ViewModeFlags2=…&ViewMode2=4&UNICODE=1`
//! —— IDLIST 是 PIDL 原始字节的 `%XX` **大写**十六进制串（无尖括号），
//! `%00%00` = 空 PIDL（默认文件夹）。
//!
//! 分层：`config` 只做文本拆解（`QdrFolder`，core 无 shell 依赖），
//! 字节 ↔ 路径的换算在本模块。历史上 load 侧从未解码 IDLIST（差异已登记），
//! 导致「打开 .qdr / 重启恢复 / 布局槽载入」一律丢失窗格路径——本模块补齐
//! 解码；保存侧对**新文件**（无原 IDLIST 可保留）编码当前路径。

use std::path::{Path, PathBuf};

use windows::Win32::UI::Shell::{ILGetSize, SHGetPathFromIDListW, SHParseDisplayName};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;

use super::{as_pcwstr, free_pidl, path_to_wide, shell_error, QError, QResult};

/// PIDL 字节 → `%XX%XX...`（大写十六进制；纯函数）。
pub fn encode_pidl_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for b in bytes {
        s.push('%');
        // 大写十六进制两位（原版实测为大写）。
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    s
}

/// `%XX%XX...` → PIDL 字节（大小写不敏感；空串/纯空白 → 空 Vec=空 PIDL；
/// 非 `%` 起始、奇数位、非十六进制 → Err）。
pub fn decode_pidl_hex(s: &str) -> QResult<Vec<u8>> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(Vec::new());
    }
    if !t.starts_with('%') {
        return Err(QError::config("IDLIST", "不是 %XX 百分号编码".to_string()));
    }
    let mut out: Vec<u8> = Vec::with_capacity(t.len() / 3);
    let mut chars = t.chars();
    while let Some(_pct) = chars.next() {
        let hi = chars
            .next()
            .ok_or_else(|| QError::config("IDLIST", "截断的十六进制位".to_string()))?;
        let lo = chars
            .next()
            .ok_or_else(|| QError::config("IDLIST", "截断的十六进制位".to_string()))?;
        let byte = u8::from_str_radix(&format!("{hi}{lo}"), 16)
            .map_err(|_| QError::config("IDLIST", format!("非十六进制位：{hi}{lo}")))?;
        out.push(byte);
    }
    Ok(out)
}

/// 路径 → IDLIST 十六进制串（`SHParseDisplayName` 取 PIDL 后编码）。
/// 路径不存在/不可解析时 Err（调用方应保持默认路径，不写坏档）。
pub fn path_to_idlist_hex(path: &Path) -> QResult<String> {
    let wide = path_to_wide(path)?;
    let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
    let mut attrs = 0u32;
    // SAFETY: wide 合法（NUL 结尾），出参有效。
    unsafe { SHParseDisplayName(as_pcwstr(&wide), None, &mut pidl, 0, Some(&mut attrs)) }
        .map_err(|e| shell_error(&format!("SHParseDisplayName({})", path.display()), e))?;
    if pidl.is_null() {
        return Err(QError::Shell(format!(
            "SHParseDisplayName 未产出 PIDL：{}",
            path.display()
        )));
    }
    // SAFETY: pidl 有效；ILGetSize 返回含终结 0 的字节数。
    let size = unsafe { ILGetSize(Some(pidl)) } as usize;
    if size == 0 {
        // SAFETY: pidl 归本函数所有。
        unsafe { free_pidl(pidl) };
        return Err(QError::Shell(format!("ILGetSize=0：{}", path.display())));
    }
    // SAFETY: pidl 有效且 size 正确；拷贝进 String 后才释放。
    let bytes = unsafe { std::slice::from_raw_parts(pidl as *const u8, size) };
    let hex = encode_pidl_hex(bytes);
    // SAFETY: pidl 归本函数所有（CoTaskMemAlloc）。
    unsafe { free_pidl(pidl) };
    Ok(hex)
}

/// IDLIST 十六进制串 → 文件系统路径（PIDL 重建后 `SHGetPathFromIDListW`）。
/// 空 PIDL（`%00%00`/空串）、非文件系统 PIDL（如「此电脑」GUID PIDL）、
/// 超长路径 → Err；调用方保持默认路径。
pub fn idlist_hex_to_path(hex: &str) -> QResult<PathBuf> {
    let bytes = decode_pidl_hex(hex)?;
    // 空 PIDL（`%00%00`/全零）：shell 语义是「桌面/默认文件夹」
    // （SHGetPathFromIDListW 会解出桌面路径）。原版 Folder1=%00%00 的语义是
    // 用默认窗格路径，故此处判失败，由调用方保持会话默认——不跳桌面。
    if bytes.is_empty() || bytes.iter().all(|&b| b == 0) {
        return Err(QError::InvalidPath("空 IDLIST（默认文件夹）".to_string()));
    }
    let mut buf = [0u16; 260];
    // SAFETY: bytes 是完整 PIDL（调用方保证含终结 0；原版实测均含），
    // buf 为 260 宽字符出参（MAX_PATH）。
    let ok = unsafe { SHGetPathFromIDListW(bytes.as_ptr() as *const ITEMIDLIST, &mut buf) };
    if !ok.as_bool() {
        return Err(QError::InvalidPath(
            "IDLIST 不是文件系统 PIDL（如虚拟文件夹）".to_string(),
        ));
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if end == 0 {
        // 空 PIDL 在 shell 语义里是「桌面」，SHGetPathFromIDListW 返回 TRUE +
        // 空串（原版 Folder1=%00%00 即此）——对窗格路径无意义，判失败。
        return Err(QError::InvalidPath("IDLIST 解析为空路径（桌面）".to_string()));
    }
    let mut s = String::from_utf16_lossy(&buf[..end]);
    // 归一化去掉尾部分隔符（PIDL 往返不产生尾杠；其余路径存储也无尾杠，
    // 见 shell::dir_with_backslash 的用法）。根路径 "C:\"（3 字符）保留。
    while s.len() > 3 && s.ends_with('\\') {
        s.pop();
    }
    Ok(PathBuf::from(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_hex_is_uppercase_percent_prefixed() {
        assert_eq!(encode_pidl_hex(&[0x00, 0x14, 0xff, 0x0a]), "%00%14%FF%0A");
        assert_eq!(encode_pidl_hex(&[]), "");
    }

    #[test]
    fn decode_hex_roundtrip_and_case_insensitive() {
        let bytes = [0x14u8, 0x00, 0x1F, 0x50, 0xE0, 0x9D, 0x00, 0x00];
        let hex = encode_pidl_hex(&bytes);
        assert_eq!(decode_pidl_hex(&hex).unwrap(), bytes);
        // 小写等价（原版为大写，读取侧宽容）。
        assert_eq!(decode_pidl_hex(&hex.to_lowercase()).unwrap(), bytes);
    }

    #[test]
    fn decode_empty_and_malformed() {
        assert_eq!(decode_pidl_hex("").unwrap(), Vec::<u8>::new());
        assert_eq!(decode_pidl_hex("%00%00").unwrap(), vec![0, 0]);
        assert!(decode_pidl_hex("14%00").is_err(), "非 % 起始");
        assert!(decode_pidl_hex("%1").is_err(), "奇数位");
        assert!(decode_pidl_hex("%ZZ").is_err(), "非十六进制");
    }

    #[test]
    fn idlist_hex_to_path_rejects_empty_and_virtual() {
        // 空 PIDL（原版 Folder1 实测值）= 默认文件夹，不解析出路径。
        assert!(idlist_hex_to_path("%00%00").is_err());
        // 「此电脑」GUID PIDL（原版 Folder2..4 实测值）：虚拟文件夹，无文件系统路径。
        let this_pc = "%14%00%1F%50%E0%4F%D0%20%EA%3A%69%10%A2%D8%08%00%2B%30%30%9D%00%00";
        assert!(idlist_hex_to_path(this_pc).is_err());
    }

    #[test]
    fn path_idlist_roundtrip_on_real_dir() {
        // 真实 FFI 往返：temp 目录 → PIDL 编码 → 解码回路径。
        // temp_dir() 自带尾杠而 PIDL 往返不产生——两侧同规则归一后比较。
        let norm = |p: &std::path::Path| {
            p.to_string_lossy()
                .trim_end_matches('\\')
                .to_lowercase()
        };
        let dir = std::env::temp_dir();
        let hex = path_to_idlist_hex(&dir).expect("temp 目录应可编码");
        assert!(hex.starts_with('%'), "编码应为 %XX 串：{hex}");
        let back = idlist_hex_to_path(&hex).expect("应可解码");
        assert_eq!(norm(&back), norm(&dir), "往返路径不一致：{back:?} vs {dir:?}");
    }
}
