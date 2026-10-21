//! 复现主程序的初始化顺序：CoInitialize → OleInitialize → ICC → LoadMenuIndirectW。
use windows::core::*;
use windows::Win32::System::Com::{CoInitialize, CoUninitialize};
use windows::Win32::System::Ole::{OleInitialize, OleUninitialize};
use windows::Win32::UI::Controls::{INITCOMMONCONTROLSEX, INITCOMMONCONTROLSEX_ICC, InitCommonControlsEx};
use windows::Win32::UI::WindowsAndMessaging::LoadMenuIndirectW;

fn main() -> Result<()> {
    unsafe {
        let _ = CoInitialize(None);
        let ole = OleInitialize(None);
        println!("[probe2] ole = {:?}", ole);

        let icc = INITCOMMONCONTROLSEX { dwSize: 8, dwICC: INITCOMMONCONTROLSEX_ICC(0x6FF) };
        InitCommonControlsEx(&icc);

        let bytes: &[u8] = include_bytes!("../assets/extracted/4/128.bin");
        println!("[probe2] bytes addr={:p} len={}", bytes.as_ptr(), bytes.len());
        let h = LoadMenuIndirectW(bytes.as_ptr() as *const core::ffi::c_void);
        match h {
            Ok(m) => println!("[probe2] 成功 -> {m:?}"),
            Err(e) => println!("[probe2] 失败 -> {e}"),
        }

        if ole.is_ok() { OleUninitialize(); }
        CoUninitialize();
        Ok(())
    }
}
