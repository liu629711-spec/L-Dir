//! 运行时菜单 dump：EnumThreadWindows 找指定 PID 的主窗口 → GetMenu → 递归
//! GetMenuItemInfoW 输出 (id, text, state, depth) JSON 行。
//! 用途：菜单模板里的文本是 i18n 键，运行时已被原版替换为本地化串；
//! id 不变 → dump 即得「命令 id → 语义」真值表（1:1 命令分发依据）。
//! 用法：cargo run --release --example menu_dump -- <PID>

use windows::core::{BOOL, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

unsafe fn dump_menu(hmenu: HMENU, depth: usize, out: &mut Vec<serde_json::Value>) {
    let n = unsafe { GetMenuItemCount(Some(hmenu)) };
    for i in 0..n {
        let mut buf = [0u16; 512];
        let mut item = MENUITEMINFOW {
            cbSize: core::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_ID | MIIM_STRING | MIIM_STATE | MIIM_FTYPE | MIIM_SUBMENU,
            dwTypeData: PWSTR(buf.as_mut_ptr()),
            cch: buf.len() as u32,
            ..Default::default()
        };
        let ok = unsafe { GetMenuItemInfoW(hmenu, i as u32, true, &mut item) };
        if ok.is_ok() {
            let text = String::from_utf16_lossy(&buf[..item.cch as usize]);
            out.push(serde_json::json!({
                "depth": depth,
                "pos": i,
                "id": item.wID,
                "text": text,
                "state": item.fState.0,
                "ftype": item.fType.0,
                "popup": !item.hSubMenu.is_invalid(),
            }));
            if !item.hSubMenu.is_invalid() {
                unsafe { dump_menu(item.hSubMenu, depth + 1, out) };
            }
        }
    }
}

unsafe extern "system" fn top_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid != lparam.0 as u32 {
        return BOOL(1);
    }
    let mut buf = [0u16; 256];
    let _ = unsafe { GetWindowTextW(hwnd, &mut buf) };
    let title = String::from_utf16_lossy(&buf.iter().take_while(|&&c| c != 0).cloned().collect::<Vec<_>>());
    let menu = unsafe { GetMenu(hwnd) };
    if menu.is_invalid() {
        return BOOL(1);
    }
    let mut out = Vec::new();
    unsafe { dump_menu(menu, 0, &mut out) };
    println!("=== hwnd {:?} {} ===", hwnd, title);
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    BOOL(0)
}

fn main() {
    let pid: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    unsafe { EnumWindows(Some(top_cb), LPARAM(pid as isize)) };
}
