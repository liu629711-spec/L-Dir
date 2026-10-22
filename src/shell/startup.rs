//! 开机恢复（登录时启动）：HKCU Run 键读写（PRODUCT_SPEC P3）。
//!
//! 语义（自设计，*待复核*——原版「开机恢复」的反编译行为链未确证，
//! 按行业惯例实现为 HKCU `Software\Microsoft\Windows\CurrentVersion\Run`
//! 值注册表自启动，值为带引号的 exe 路径）。
//!
//! 契约 D4：所有 advapi32 FFI 封在本模块；上层只见 bool/String 进出。
//! `*_at` 变体把子键参数化，供单测/钩子在 `Software\L-Dir-tests` 下
//! 做真实 API 往返（不碰真 Run 键），`autostart_set/get` 用生产路径。

use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, WIN32_ERROR};
use windows::Win32::System::Registry::HKEY;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

use super::{win32_error, QResult};

/// 生产：自启动值名（品牌名，R1 自有）。
const VALUE_NAME: &str = "L-Dir";
/// 生产：Run 键路径。
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// exe 路径 → Run 键值（始终加引号；空格/特殊字符安全）。纯函数单测。
pub fn run_command_value(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// Run 键值 → exe 路径（去外层引号）。纯函数单测。
pub fn parse_run_command_value(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(t)
        .to_string()
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 打开（或创建）子键；`read_only=false` 时以可写权限打开。
unsafe fn open(subkey: &str, writable: bool) -> Result<HKEY, WIN32_ERROR> {
    let mut hk = HKEY::default();
    let w = wide(subkey);
    let sam = if writable {
        KEY_SET_VALUE | KEY_READ
    } else {
        KEY_READ
    };
    let e = if writable {
        // Create：Run 键恒在，但测试子键可能不存在。
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(w.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            sam,
            None,
            &mut hk,
            None,
        )
    } else {
        RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(w.as_ptr()), None, sam, &mut hk)
    };
    if e == WIN32_ERROR(0) {
        Ok(hk)
    } else {
        Err(e)
    }
}

/// 在任意 Run 语义子键 `subkey` 下设置/删除值 `value`。
/// `cmd=None` 删除（不存在时幂等成功）。
///
/// # Safety
/// 注册表 FFI；仅本模块内部使用（D4）。
pub unsafe fn set_autostart_at(subkey: &str, value: &str, cmd: Option<&str>) -> QResult<()> {
    let hk = match open(subkey, true) {
        Ok(h) => h,
        Err(e) if e == ERROR_FILE_NOT_FOUND => {
            // 只读打开不该出现；可写打开走 Create。真失败才报错。
            return Err(win32_error("RegCreateKeyExW", e));
        }
        Err(e) => return Err(win32_error("打开 Run 键", e)),
    };
    let r = match cmd {
        Some(c) => {
            let wv = wide(value);
            let data: Vec<u8> = c
                .encode_utf16()
                .chain(std::iter::once(0))
                .flat_map(|u| u.to_ne_bytes())
                .collect();
            RegSetValueExW(hk, PCWSTR(wv.as_ptr()), None, REG_SZ, Some(&data))
        }
        None => {
            let wv = wide(value);
            let e = RegDeleteValueW(hk, PCWSTR(wv.as_ptr()));
            if e == ERROR_FILE_NOT_FOUND {
                WIN32_ERROR(0) // 幂等
            } else {
                e
            }
        }
    };
    let _ = RegCloseKey(hk);
    if r == WIN32_ERROR(0) {
        Ok(())
    } else {
        Err(win32_error("写 Run 值", r))
    }
}

/// 读回 `subkey` 下值 `value`（缺键/缺值 → Ok(None)）。
///
/// # Safety
/// 注册表 FFI；仅本模块内部使用（D4）。
pub unsafe fn get_autostart_at(subkey: &str, value: &str) -> QResult<Option<String>> {
    let hk = match open(subkey, false) {
        Ok(h) => h,
        Err(e) if e == ERROR_FILE_NOT_FOUND => return Ok(None),
        Err(e) => return Err(win32_error("打开 Run 键", e)),
    };
    let wv = wide(value);
    let mut buf = [0u16; 512];
    let mut cb = (buf.len() * 2) as u32;
    let mut kind = REG_SZ;
    let e = RegQueryValueExW(
        hk,
        PCWSTR(wv.as_ptr()),
        None,
        Some(&mut kind),
        Some(buf.as_mut_ptr() as *mut u8),
        Some(&mut cb),
    );
    let _ = RegCloseKey(hk);
    if e == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if e != WIN32_ERROR(0) {
        return Err(win32_error("读 Run 值", e));
    }
    let chars = (cb as usize / 2).min(buf.len());
    let end = buf[..chars].iter().position(|&c| c == 0).unwrap_or(chars);
    Ok(Some(String::from_utf16_lossy(&buf[..end])))
}

/// 删除测试子树（单测/钩子自清理）。
///
/// # Safety
/// 注册表 FFI；仅测试/钩子路径使用。
pub unsafe fn purge_tree(subkey: &str) -> QResult<()> {
    let w = wide(subkey);
    let e = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(w.as_ptr()));
    if e == ERROR_FILE_NOT_FOUND || e == WIN32_ERROR(0) {
        Ok(())
    } else {
        Err(win32_error("删除测试键", e))
    }
}

/// 生产：启用/禁用登录时启动（写入当前 exe 路径）。
pub unsafe fn autostart_set(enabled: bool, exe: &Path) -> QResult<()> {
    set_autostart_at(
        RUN_KEY,
        VALUE_NAME,
        enabled.then(|| run_command_value(exe)).as_deref(),
    )
}

/// 生产：当前是否启用登录时启动。
pub unsafe fn autostart_get(exe: &Path) -> QResult<bool> {
    Ok(get_autostart_at(RUN_KEY, VALUE_NAME)?
        .map(|v| parse_run_command_value(&v).eq_ignore_ascii_case(&exe.display().to_string()))
        .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_key() -> String {
        format!(r"Software\L-Dir-tests\{}", std::process::id())
    }

    #[test]
    fn run_command_quoting_roundtrip() {
        let p = PathBuf::from(r"C:\Program Files\L-Dir\L-Dir.exe");
        let v = run_command_value(&p);
        assert_eq!(v, "\"C:\\Program Files\\L-Dir\\L-Dir.exe\"");
        assert_eq!(parse_run_command_value(&v), p.display().to_string());
        // 无引号值（手输）原样返回。
        assert_eq!(parse_run_command_value("plain.exe"), "plain.exe");
    }

    #[test]
    fn autostart_roundtrip_on_test_key() {
        let key = test_key();
        unsafe {
            let _ = purge_tree(&key);
            // 初始：缺。
            assert_eq!(get_autostart_at(&key, "L-Dir").unwrap(), None);
            // 启用。
            let cmd = run_command_value(Path::new(r"D:\L-dir\qdir-rs\target\release\qdir-rs.exe"));
            set_autostart_at(&key, "L-Dir", Some(&cmd)).unwrap();
            assert_eq!(
                get_autostart_at(&key, "L-Dir").unwrap().as_deref(),
                Some(cmd.as_str())
            );
            // 禁用（删除值）+ 幂等再删。
            set_autostart_at(&key, "L-Dir", None).unwrap();
            assert_eq!(get_autostart_at(&key, "L-Dir").unwrap(), None);
            set_autostart_at(&key, "L-Dir", None).unwrap();
            // 清理测试子树。
            purge_tree(&key).unwrap();
            assert_eq!(get_autostart_at(&key, "L-Dir").unwrap(), None);
        }
    }
}
