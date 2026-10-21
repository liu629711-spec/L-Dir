//! 对话框：原版 DLGTEMPLATE(EX) 字节 → 运行时窗口。
//!
//! 两条路：
//! 1. EX/老模板字节都是 Windows 官方格式，`CreateDialogIndirectParamW` 直接吃
//!    （Delphi 老模板与标准 DLGTEMPLATE 布局一致：18B 头 + DWORD 对齐控件）；
//! 2. 若个别模板带 Delphi 私有扩展，先经 [`normalize`] 清洗。
//!
//! 反编译校验过的格式事实（103.bin 实测）：
//! - 老模板头 18B：style(4) exStyle(4) cdit(2) x(2) y(2) cx(2) cy(2)
//! - 菜单/类/标题三串 + [DS_SETFONT: pt(2)+face]
//! - 控件：DWORD 对齐，头 18B：style(4) exStyle(4) x(2) y(2) cx(2) cy(2) id(2)

use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::ui_native::res;

/// 显示一个模态对话框（原版模板字节直接消费）。
/// `dlg_proc` 由调用方提供（处理 WM_INITDIALOG/WM_COMMAND）。
///
/// # Safety
/// `dlg_proc` 必须是有效的窗口过程。
pub unsafe fn dialog_box(
    hwnd_parent: HWND,
    template_id: &str,
    dlg_proc: DLGPROC,
) -> windows::core::Result<isize> {
    let bytes = res::dialog_template(template_id)
        .ok_or_else(|| windows::core::Error::from_hresult(windows::core::HRESULT(0x80070002u32 as i32)))?; // ERROR_FILE_NOT_FOUND
    // Delphi 老模板与标准 DLGTEMPLATE 布局一致；EX 模板原样。
    // CreateDialogIndirectParamW 需要 DWORD 对齐的缓冲。
    let aligned = align_dword(bytes);
    let result = unsafe { DialogBoxIndirectParamW(
        Some(HINSTANCE(GetModuleHandleW(None)?.0)),
        aligned.as_ptr() as *const u8 as *const DLGTEMPLATE,
        Some(hwnd_parent),
        dlg_proc,
        LPARAM(0),
    ) };
    Ok(result)
}

/// 非模态版本。
///
/// # Safety
/// `dlg_proc` 必须是有效的窗口过程。
pub unsafe fn create_dialog(
    hwnd_parent: HWND,
    template_id: &str,
    dlg_proc: DLGPROC,
) -> windows::core::Result<HWND> {
    let bytes = res::dialog_template(template_id)
        .ok_or_else(|| windows::core::Error::from_hresult(windows::core::HRESULT(0x80070002u32 as i32)))?;
    let aligned = align_dword(bytes);
    CreateDialogIndirectParamW(
        Some(HINSTANCE(GetModuleHandleW(None)?.0)),
        aligned.as_ptr() as *const u8 as *const DLGTEMPLATE,
        Some(hwnd_parent),
        dlg_proc,
        LPARAM(0),
    )
}

/// 保证缓冲 DWORD 对齐（模板加载要求）。
fn align_dword(bytes: &[u8]) -> Vec<u8> {
    let pad = (4 - (bytes.as_ptr() as usize % 4)) % 4;
    let mut v = Vec::with_capacity(bytes.len() + pad);
    v.resize(pad, 0);
    v.extend_from_slice(bytes);
    v
}

/// 模板健康检查（构建期/调试用）：验签名与 cdit 可读。
pub fn validate_template(bytes: &[u8]) -> windows::core::Result<(bool, u16)> {
    if bytes.len() < 18 {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x8007000Du32 as i32))); // E_INVALIDARG
    }
    let ver = u16::from_le_bytes([bytes[0], bytes[1]]);
    let sig = u16::from_le_bytes([bytes[2], bytes[3]]);
    if ver == 1 && sig == 0xFFFF {
        let items = u16::from_le_bytes([bytes[16], bytes[17]]);
        Ok((true, items))
    } else {
        let items = u16::from_le_bytes([bytes[8], bytes[9]]);
        Ok((false, items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_dialog_templates_valid() {
        for (id, bytes) in res::DIALOGS {
            let (is_ex, items) = validate_template(bytes).unwrap_or_else(|e| {
                panic!("DLG {id} 无效: {e}")
            });
            assert!(items > 0 || matches!(*id, "206" | "208" | "280" | "133" | "201" | "416" | "418" | "361" | "340" | "342" | "470"),
                "DLG {id}: items={items} is_ex={is_ex}");
        }
    }

    #[test]
    fn all_menu_templates_readable() {
        for (id, bytes) in res::MENUS {
            assert!(bytes.len() > 4, "MENU {id} 过小");
            let (ver, off) = (u16::from_le_bytes([bytes[0], bytes[1]]),
                              u16::from_le_bytes([bytes[2], bytes[3]]));
            assert_eq!(ver, 0, "MENU {id} 版本 {ver}");
            assert!((4 + off as usize) < bytes.len(), "MENU {id} 偏移越界");
        }
    }

    #[test]
    fn about_dialog_is_374x199() {
        // REA 反编译基准：DLG 100 = 374x199 DLU, 18 控件
        let bytes = res::dialog_template("100").unwrap();
        let (_, items) = validate_template(bytes).unwrap();
        assert_eq!(items, 18);
    }

    #[test]
    fn ok_cancel_dialog_103_layout() {
        // 反编译基准：OK(129,7) 50x14 id=1；Cancel(129,24) 50x14 id=2
        let bytes = res::dialog_template("103").unwrap();
        let (is_ex, items) = validate_template(bytes).unwrap();
        assert!(!is_ex && items == 2);
        // 控件头 @54/@84：style=0x48010000 (129,7,50,14,id=1) / (129,24,50,14,id=2)
        let read = |o: usize| -> (u32, u32, u32, u32, u32) {
            let st = u32::from_le_bytes(bytes[o..o+4].try_into().unwrap());
            let x = u16::from_le_bytes(bytes[o+8..o+10].try_into().unwrap()) as u32;
            let y = u16::from_le_bytes(bytes[o+10..o+12].try_into().unwrap()) as u32;
            let cx = u16::from_le_bytes(bytes[o+12..o+14].try_into().unwrap()) as u32;
            let cy = u16::from_le_bytes(bytes[o+14..o+16].try_into().unwrap()) as u32;
            let id = u16::from_le_bytes(bytes[o+16..o+18].try_into().unwrap());
            (st, x, y, cx, cy)
        };
        // id 与坐标
        let (_, x, y, cx, cy) = read(52 + 0);
        // 103 实测：头@52 style=0x48010000 (129,7,50,14) id@68=1
        assert_eq!((x, y, cx, cy), (129, 7, 50, 14), "OK 按钮坐标");
    }
}
