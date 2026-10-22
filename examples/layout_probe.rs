//! 布局按钮语义探针：向原版 Q-Dir 主窗口逐个发 WM_COMMAND，
//! 每次间隔后枚举窗格容器（ATL:00000000004189CC）矩形。
//! 用法: layout_probe <pid> <id1> <id2> ...

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetWindowRect, IsWindowVisible, PostMessageW,
    WM_COMMAND,
};
use windows::Win32::System::Threading::Sleep;

struct Ctx {
    pid: u32,
    found: Option<HWND>,
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let ctx = &*(lparam.0 as *mut Ctx);
    let mut pid = 0u32;
    windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid != ctx.pid {
        return true.into();
    }
    let mut cls = [0u16; 128];
    let n = GetClassNameW(hwnd, &mut cls);
    if n > 0 {
        let name = String::from_utf16_lossy(&cls[..n as usize]);
        if name.starts_with("ATL:") {
            let mut title = [0u16; 128];
            let t = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut title);
            if t > 0 {
                let title_s = String::from_utf16_lossy(&title[..t as usize]);
                if title_s == "L-Dir" || title_s.starts_with("Q-Dir ") {
                    let ctx = &mut *(lparam.0 as *mut Ctx);
                    ctx.found = Some(hwnd);
                    return false.into();
                }
            }
        }
    }
    true.into()
}

fn collect_panes(main: HWND, pid: u32) -> Vec<(i32, i32, i32, i32, bool)> {
    struct Sink {
        rects: Vec<(i32, i32, i32, i32, bool)>,
    }
    unsafe extern "system" fn child_proc(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
        let sink = &mut *(lp.0 as *mut Sink);
        let mut cls = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut cls);
        if n > 0 {
            let name = String::from_utf16_lossy(&cls[..n as usize]);
            if name.contains("4189CC") {
                let mut rc = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rc);
                let vis = IsWindowVisible(hwnd).as_bool();
                sink.rects.push((rc.left, rc.top, rc.right, rc.bottom, vis));
            }
        }
        true.into()
    }
    let _ = pid;
    let mut sink = Sink { rects: Vec::new() };
    unsafe {
        let _ = EnumChildWindows(
            Some(main),
            Some(child_proc),
            LPARAM(&mut sink as *mut Sink as isize),
        );
    }
    sink.rects.sort();
    sink.rects
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid: u32 = args[0].parse().expect("pid");
    let ids: Vec<u32> = args[1..].iter().map(|s| s.parse().unwrap()).collect();
    unsafe {
        let mut ctx = Ctx { pid, found: None };
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut Ctx as isize));
        let main = ctx.found.expect("未找到原版主窗口");
        println!("main = {main:?}");
        println!("== 基线 ==");
        for (l, t, r, b, v) in collect_panes(main, pid) {
            println!("   pane rect=({l},{t})-({r},{b}) w={} vis={v}", r - l);
        }
        for id in ids {
            println!("== 发 WM_COMMAND id={id} (0x{id:X}) ==");
            let _ = PostMessageW(Some(main), WM_COMMAND, WPARAM(id as usize), LPARAM(0));
            // WM_COMMAND 投递到目标线程队列，等待其自行处理
            Sleep(1000);
            for (l, t, r, b, v) in collect_panes(main, pid) {
                println!("   pane rect=({l},{t})-({r},{b}) w={} vis={v}", r - l);
            }
        }
    }
}
