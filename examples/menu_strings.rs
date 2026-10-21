//! 跨进程读原版 HMENU：GetMenu → 递归 GetSubMenu/GetMenuStringW，
//! 得到命令 id → 已本地化文案 的权威映射（绕开语言块解码）。
//! 用法: menu_strings <pid>

use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetMenu, GetMenuItemCount, GetMenuItemID, GetSubMenu, GetMenuStringW, HMENU, MF_STRING,
};

unsafe fn walk(hm: HMENU, depth: usize, _path: &str) {
    let count = GetMenuItemCount(Some(hm));
    for i in 0..count {
        let mut buf = [0u16; 256];
        let n = GetMenuStringW(hm, i as u32, Some(&mut buf), MF_STRING);
        let text = String::from_utf16_lossy(&buf[..(n.max(0)) as usize]);
        let sub = GetSubMenu(hm, i);
        if !sub.is_invalid() {
            let tag = if text.is_empty() { format!("popup#{i}") } else { text.clone() };
            println!("{}[{}] {tag}", "  ".repeat(depth), "sub");
            walk(sub, depth + 1, &tag);
        } else {
            let id = GetMenuItemID(hm, i);
            if id != u32::MAX {
                println!("{}{id:>8} | {text:?}", "  ".repeat(depth));
            }
        }
    }
}

fn main() {
    let pid: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    unsafe {
        // 找主窗口（标题以 Q-Dir 开头，本进程枚举全部窗口再比 pid）
        struct Ctx {
            pid: u32,
            found: Option<HWND>,
        }
        unsafe extern "system" fn e(hwnd: HWND, lp: windows::Win32::Foundation::LPARAM) -> windows::core::BOOL {
            let mut p = 0u32;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut p));
            let ctx = &mut *(lp.0 as *mut Ctx);
            if p == ctx.pid && ctx.found.is_none() {
                let mut cls = [0u16; 64];
                let cn = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut cls);
                let cname = String::from_utf16_lossy(&cls[..(cn.max(0)) as usize]);
                let mut t = [0u16; 64];
                let n = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut t);
                let title = String::from_utf16_lossy(&t[..(n.max(0)) as usize]);
                if title.starts_with("Q-Dir") && cname.contains("546C90") {
                    ctx.found = Some(hwnd);
                    return false.into();
                }
            }
            true.into()
        }
        let mut ctx = Ctx { pid, found: None };
        windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(e),
            windows::Win32::Foundation::LPARAM(&mut ctx as *mut Ctx as isize),
        );
        let Some(hwnd) = ctx.found else {
            println!("未找到窗口 pid={pid}");
            return;
        };
        // 诊断：pid 内所有窗口的 GetMenu
        struct DiagCtx { pid: u32 }
        unsafe extern "system" fn d(hwnd: HWND, lp: windows::Win32::Foundation::LPARAM) -> windows::core::BOOL {
            let mut p = 0u32;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut p));
            let dc = &*(lp.0 as *const DiagCtx);
            if p == dc.pid {
                let mut cls = [0u16; 64];
                let cn = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut cls);
                let hm = GetMenu(hwnd);
                println!("DIAG {hwnd:?} cls={} menu_invalid={}", String::from_utf16_lossy(&cls[..cn.max(0) as usize]), hm.is_invalid());
            }
            true.into()
        }
        windows::Win32::UI::WindowsAndMessaging::EnumWindows(Some(d), windows::Win32::Foundation::LPARAM(&DiagCtx{pid} as *const DiagCtx as isize));
        let hm = GetMenu(hwnd);
        if hm.is_invalid() {
            println!("无菜单 hwnd={hwnd:?}");
            return;
        }
        walk(hm, 0, "");
        let _ = w!("");
    }
}
