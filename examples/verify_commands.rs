//! 命令分发端到端验证：找主窗口 → 发数值 WM_COMMAND → 读剪贴板。
//! 选行由重写版自身的 QDIR_TEST_SELECT_ROW 钩子完成（同进程安全）。
//! 用法：verify_commands <PID> <命令ID>
use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::*;

static mut MAIN: Option<HWND> = None;
static mut WANT_PID: u32 = 0;

unsafe extern "system" fn find_top(hwnd: HWND, _l: LPARAM) -> BOOL {
    let mut p = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut p)) };
    if p == unsafe { WANT_PID } && unsafe { IsWindowVisible(hwnd) }.as_bool() {
        let mut buf = [0u16; 64];
        let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
        let t = String::from_utf16_lossy(&buf[..n as usize]);
        if t == "Q-Dir" {
            unsafe { (*std::ptr::addr_of_mut!(MAIN)) = Some(hwnd) };
            return BOOL(0);
        }
    }
    BOOL(1)
}

fn main() {
    let pid: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let cmd: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    unsafe {
        WANT_PID = pid;
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
        EnumWindows(Some(find_top), LPARAM(0));
        let main = unsafe { *std::ptr::addr_of!(MAIN) }.expect("未找到 Q-Dir 主窗口");
        PostMessageW(Some(main), WM_COMMAND, WPARAM(cmd), LPARAM(0));
        println!("[verify] WM_COMMAND {cmd} -> {main:?}");
    }
    std::thread::sleep(std::time::Duration::from_millis(600));
    match qdir_rs::shell::get_clipboard() {
        Ok(c) => println!("[verify] 剪贴板 {} 条 cut={} {:?}", c.len(), c.cut, c.paths),
        Err(e) => println!("[verify] 读剪贴板：{e}"),
    }
}
