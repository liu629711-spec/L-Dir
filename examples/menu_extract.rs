//! 菜单文案提取器（真实输入驱动）：对原版 Q-Dir 用 SendInput 逐项展开
//! 菜单，从 UIA 读每个 MenuItem 的 Name（本地化文案）与自动化结构，
//! 与 menus.json 的位置序一一对应。
//! 用法: menu_extract <pid> [输出文件]

use windows::core::BSTR;
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Variant::VT_BSTR;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, SendInput,
    VK_DOWN, VK_ESCAPE, VK_LEFT, VK_MENU, VK_RETURN, VK_RIGHT, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, SetForegroundWindow, FindWindowExW,
};

fn key(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn key_up(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> INPUT {
    let mut k = key(vk);
    unsafe {
        k.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
    }
    k
}

/// 屏幕坐标点击（SendInput 绝对坐标）。
fn click(x: i32, y: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE};
    let vx = 65535;
    let (sw, sh) = unsafe {
        (
            windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows::Win32::UI::WindowsAndMessaging::SM_CXSCREEN,
            ),
            windows::Win32::UI::WindowsAndMessaging::GetSystemMetrics(
                windows::Win32::UI::WindowsAndMessaging::SM_CYSCREEN,
            ),
        )
    };
    let ax = ((x as i64) * vx as i64 / sw as i64) as i32;
    let ay = ((y as i64) * 65535 / sh as i64) as i32;
    unsafe {
        let mk = |flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS| INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: ax,
                    dy: ay,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let move_flags = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE;
        let _ = SendInput(&[mk(move_flags)], core::mem::size_of::<INPUT>() as i32);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = SendInput(&[mk(MOUSEEVENTF_LEFTDOWN)], core::mem::size_of::<INPUT>() as i32);
        std::thread::sleep(std::time::Duration::from_millis(60));
        let _ = SendInput(&[mk(MOUSEEVENTF_LEFTUP)], core::mem::size_of::<INPUT>() as i32);
    }
}

fn tap(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) {
    unsafe {
        SendInput(&[key(vk), key_up(vk)], core::mem::size_of::<INPUT>() as i32);
    }
    std::thread::sleep(std::time::Duration::from_millis(90));
}

fn prop(e: &IUIAutomationElement, id: UIA_PROPERTY_ID) -> String {
    let Ok(v) = (unsafe { e.GetCurrentPropertyValue(id) }) else { return String::new() };
    if v.vt() != VT_BSTR { return String::new(); }
    let b: &BSTR = unsafe { &v.Anonymous.Anonymous.Anonymous.bstrVal };
    b.to_string()
}

struct Found {
    pid: u32,
    hwnd: windows::Win32::Foundation::HWND,
}

unsafe extern "system" fn top_proc(
    h: windows::Win32::Foundation::HWND,
    lp: LPARAM,
) -> windows::core::BOOL {
    let mut p = 0u32;
    GetWindowThreadProcessId(h, Some(&mut p));
    let f = &mut *(lp.0 as *mut Found);
    if p == f.pid {
        let mut cls = [0u16; 64];
        let n = GetClassNameW(h, &mut cls);
        if n > 0 && String::from_utf16_lossy(&cls[..n as usize]).contains("546C90") {
            f.hwnd = h;
            return false.into();
        }
    }
    true.into()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid: u32 = args[0].parse().expect("pid");
    let out = args.get(1).cloned();

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
        let ua: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL).unwrap();

        let mut f = Found { pid, hwnd: windows::Win32::Foundation::HWND::default() };
        let _ = EnumWindows(Some(top_proc), LPARAM(&mut f as *mut Found as isize));
        assert!(!f.hwnd.is_invalid(), "未找到主窗口");
        let _ = SetForegroundWindow(f.hwnd);
        std::thread::sleep(std::time::Duration::from_millis(400));

        let mut log: Vec<String> = Vec::new();

        // 顶级菜单项坐标从 UIA 取（避免屏幕坐标估算在最大化/多 DPI 下失准）。
        let mi_cond = ua
            .CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &windows::Win32::System::Variant::VARIANT::from(UIA_MenuItemControlTypeId.0),
            )
            .unwrap();
        let root = ua.ElementFromHandle(f.hwnd).unwrap();
        let tops = root.FindAll(TreeScope_Descendants, &mi_cond).unwrap();
        let nt = tops.Length().unwrap_or(0);
        println!("UIA 顶级 MenuItem = {nt}");
        for menu_idx in 0..nt.min(8) {
            let el = tops.GetElement(menu_idx).unwrap();
            let name = prop(&el, UIA_NamePropertyId);
            // WTL 自绘菜单条不提供 UIA 可点击点，改用 BoundingRectangle 中心。
            let r = el.CurrentBoundingRectangle();
            match r {
                Ok(br) if br.right > br.left && br.bottom > br.top => {
                    let x = (br.left + br.right) / 2;
                    let y = (br.top + br.bottom) / 2;
                    click(x, y);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let popup = FindWindowExW(None, None, w!("#32768"), None);
                    match popup {
                        Ok(pw) if IsWindowVisible(pw).as_bool() => {
                            log.push(format!("## 顶级菜单 #{menu_idx}: {name}"));
                            walk_menu(&ua, 0, &mut log);
                        }
                        _ => log.push(format!("## 顶级菜单 #{menu_idx}: {name} 无弹出")),
                    }
                    tap(VK_ESCAPE);
                    tap(VK_ESCAPE);
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
                _ => log.push(format!("## {name}: 无边界矩形")),
            }
        }
        tap(VK_ESCAPE);
        CoUninitialize();

        let text = log.join("\n");
        match out {
            Some(p) => {
                std::fs::write(&p, &text).expect("写输出");
                println!("写出 {} 行 -> {p}", log.len());
            }
            None => println!("{text}"),
        }
    }
}

use windows::core::w;

/// 读当前弹出菜单的全部项（含已展开子菜单的可见项）。
unsafe fn read_popup(ua: &IUIAutomation) -> Vec<String> {
    let Ok(pw) = FindWindowExW(None, None, w!("#32768"), None) else { return Vec::new() };
    if !IsWindowVisible(pw).as_bool() {
        return Vec::new();
    }
    let Ok(el) = ua.ElementFromHandle(pw) else { return Vec::new() };
    let cond = ua
        .CreatePropertyCondition(
            UIA_ControlTypePropertyId,
            &windows::Win32::System::Variant::VARIANT::from(UIA_MenuItemControlTypeId.0),
        )
        .unwrap();
    let items = el.FindAll(TreeScope_Descendants, &cond).unwrap();
    let n = items.Length().unwrap_or(0);
    let mut out = Vec::new();
    for i in 0..n {
        if let Ok(it) = items.GetElement(i) {
            let name = prop(&it, UIA_NamePropertyId);
            if !name.is_empty() {
                out.push(name);
            }
        }
    }
    out
}

/// 逐项下移并在每项上尝试 Right 展开子菜单（规范递归，无状态漂移）。
unsafe fn walk_menu(ua: &IUIAutomation, depth: usize, log: &mut Vec<String>) {
    // 当前弹出菜单的快照（不含子菜单）。
    let base = read_popup(ua);
    let n = base.len();
    for i in 0..n {
        // 重新打开顶层菜单由调用方保证；这里每轮从第 0 项下移 i+1 次。
        for _ in 0..=i {
            tap(VK_DOWN);
            std::thread::sleep(std::time::Duration::from_millis(55));
        }
        // 试探子菜单：Right 后若出现新项即视为有子菜单。
        let before = read_popup(ua).len();
        tap(VK_RIGHT);
        std::thread::sleep(std::time::Duration::from_millis(220));
        let after = read_popup(ua);
        let opened = after.len() > before || (after.len() == before && before > 1 && i + 1 >= base.len());
        if opened && after.len() > 1 {
            log.push(format!("{}[{}] {}  ── 子菜单：", "  ".repeat(depth + 1), i, base[i]));
            walk_menu(ua, depth + 1, log);
            // 收回子菜单
            tap(VK_ESCAPE);
            std::thread::sleep(std::time::Duration::from_millis(150));
            // 重新定位到当前项：重新点击顶级项太贵，用 Down 步进（已在该层循环）。
            for _ in 0..=i {
                tap(VK_DOWN);
                std::thread::sleep(std::time::Duration::from_millis(45));
            }
        } else {
            log.push(format!("{}[{}] {}", "  ".repeat(depth + 1), i, base[i]));
        }
        // 回到第 0 项供下一轮步进
        for _ in 0..=i {
            tap(VK_UP);
            std::thread::sleep(std::time::Duration::from_millis(35));
        }
    }
    let _ = n;
}
