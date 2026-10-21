//! 键盘导航探针：给目标 pid 的第一/第 N 个 SysListView32 发
//! WM_KEYDOWN（Enter/Backspace），前后读窗格地址栏 Edit 文本。
//! 用法: key_probe <pid> <list_index> <vk>   vk: 13=Enter 8=Back

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetClassNameW, GetWindowTextW, PostMessageW, SendMessageW, WM_GETTEXT,
    WM_KEYDOWN, WM_KEYUP,
};

struct Sink {
    lists: Vec<HWND>,
    edits: Vec<HWND>,
}

unsafe extern "system" fn child(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
    let mut cls = [0u16; 64];
    let n = GetClassNameW(hwnd, &mut cls);
    if n > 0 {
        let name = String::from_utf16_lossy(&cls[..n as usize]);
        let s = &mut *(lp.0 as *mut Sink);
        if name == "SysListView32" {
            s.lists.push(hwnd);
        } else if name == "Edit" {
            s.edits.push(hwnd);
        }
    }
    true.into()
}

fn text_of(h: HWND) -> String {
    let mut buf = [0u16; 512];
    unsafe {
        let n = SendMessageW(h, WM_GETTEXT, Some(WPARAM(511)), Some(LPARAM(buf.as_mut_ptr() as isize))).0;
        String::from_utf16_lossy(&buf[..(n.max(0)) as usize])
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid: u32 = args[0].parse().unwrap();
    let li: usize = args.get(1).map(|x| x.parse().unwrap()).unwrap_or(0);
    let vk: usize = args.get(2).map(|x| x.parse().unwrap()).unwrap_or(13);
    unsafe {
        // 先找主窗口（pid 顶层窗），再枚举其子孙控件
        let mut root = HWND::default();
        struct TC { pid: u32, found: Option<HWND> }
        unsafe extern "system" fn top(h: HWND, lp: LPARAM) -> windows::core::BOOL {
            let mut p = 0u32;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(h, Some(&mut p));
            let tc = &mut *(lp.0 as *mut TC);
            if p == tc.pid {
                let mut cls = [0u16; 64];
                let n = GetClassNameW(h, &mut cls);
                if n > 0 && String::from_utf16_lossy(&cls[..n as usize]).contains("546C90") {
                    tc.found = Some(h);
                    return false.into();
                }
            }
            true.into()
        }
        let mut tc = TC { pid, found: None };
        windows::Win32::UI::WindowsAndMessaging::EnumWindows(
            Some(top),
            LPARAM(&mut tc as *mut TC as isize),
        );
        let main = tc.found.expect("未找到主窗口");
        let mut sink = Sink { lists: Vec::new(), edits: Vec::new() };
        let _ = EnumChildWindows(Some(main), Some(child), LPARAM(&mut sink as *mut Sink as isize));
        let Some(&list) = sink.lists.get(li) else {
            println!("无 list #{li}（共 {} 个）", sink.lists.len());
            return;
        };
        // EnumChild 顺序：每窗格先 addr_edit 后 filter_edit，地址栏在 2*li+1 位。
        let edit = sink.edits.get(1 + 2 * li).copied();
        println!("edits: {:?}", sink.edits.iter().map(|&e| text_of(e)).collect::<Vec<_>>());
        println!("before addr = {:?}", edit.map(text_of).unwrap_or_default());
        let _ = PostMessageW(Some(list), WM_KEYDOWN, WPARAM(vk), LPARAM(1 << 30));
        let _ = PostMessageW(Some(list), WM_KEYUP, WPARAM(vk), LPARAM(1 << 30 | 1 << 31));
        std::thread::sleep(std::time::Duration::from_millis(800));
        println!("after  addr = {:?}", edit.map(text_of).unwrap_or_default());
    }
}
