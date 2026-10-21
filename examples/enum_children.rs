//! 枚举指定 PID 顶层窗口的全部子窗口（EnumChildWindows，Win32 原始视角）。
use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut buf = [0u16; 128];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) };
    let class = String::from_utf16_lossy(&buf[..n as usize]);
    let mut tbuf = [0u16; 128];
    let tn = unsafe { GetWindowTextW(hwnd, &mut tbuf) };
    let title = String::from_utf16_lossy(&tbuf[..tn as usize]);
    let mut rc = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rc) };
    let vis = unsafe { IsWindowVisible(hwnd) }.as_bool();
    println!(
        "hwnd={hwnd:?} class={class:<24} vis={vis:<5} rect=({},{},{},{}) {title:?}",
        rc.left, rc.top, rc.right, rc.bottom
    );
    // 继续枚举下一层
    let _ = lparam;
    BOOL(1)
}

unsafe extern "system" fn top_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let pid = unsafe {
        let mut p = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut p));
        p
    };
    if pid == lparam.0 as u32 {
        println!("=== TOP {hwnd:?} ===");
        unsafe { EnumChildWindows(Some(hwnd), Some(cb), LPARAM(0)) };
    }
    BOOL(1)
}

fn main() {
    let pid: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    unsafe { EnumWindows(Some(top_cb), LPARAM(pid as isize)) };
}

// —— 样式解码辅助（供列头对齐核对）——
