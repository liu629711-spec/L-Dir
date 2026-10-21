//! 向指定 PID 的主窗口发送 WM_COMMAND（模拟菜单点击）。
//! 用法：send_command <PID> <命令ID>
use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

fn main() {
    let pid: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let cmd: u64 = std::env::args().nth(2).unwrap().parse().unwrap();
    let packed = (pid << 32) | cmd; // LPARAM 低 32 位 = 命令 id，高 32 位 = PID
    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let want_pid = (lparam.0 as u64) >> 32;
        let cmd = (lparam.0 as u64) & 0xFFFF_FFFF;
        let mut p = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut p)) };
        if p as u64 == want_pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let title_len = unsafe { GetWindowTextLengthW(hwnd) };
            if title_len > 0 {
                let mut buf = vec![0u16; (title_len + 1) as usize];
                unsafe { GetWindowTextW(hwnd, &mut buf) };
                let t = String::from_utf16_lossy(&buf[..title_len as usize]);
                if t == "Q-Dir" {
                    unsafe { PostMessageW(Some(hwnd), WM_COMMAND, WPARAM(cmd as usize), LPARAM(0)) };
                    println!("[send] WM_COMMAND {cmd} -> {hwnd:?}");
                    return BOOL(0);
                }
            }
        }
        BOOL(1)
    }
    unsafe { EnumWindows(Some(cb), LPARAM(packed as isize)) };
}
