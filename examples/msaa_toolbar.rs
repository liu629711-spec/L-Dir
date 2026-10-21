//! MSAA 工具栏探针：对原版 Q-Dir 的每个 ToolbarWindow32，
//! 通过 IAccessible 枚举子按钮（childId = 命令 id，accName = tooltip），
//! 得到按钮 id↔语义的权威映射。用法: msaa_toolbar <pid>

use windows::core::Interface;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::System::Variant::{VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4};
use windows::Win32::UI::Accessibility::{AccessibleObjectFromWindow, IAccessible};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetWindowThreadProcessId, IsWindowVisible,
};

struct Ctx {
    pid: u32,
    toolbars: Vec<HWND>,
}

/// 顶层窗口里找到原版主框架（类名前缀 "ATL:"，含子工具栏的那个）。
unsafe extern "system" fn top_proc(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let ctx = &mut *(lp.0 as *mut Ctx);
    if pid != ctx.pid {
        return true.into();
    }
    let mut cls = [0u16; 64];
    let n = GetClassNameW(hwnd, &mut cls);
    if n > 0 {
        let name = String::from_utf16_lossy(&cls[..n as usize]);
        if name.starts_with("ATL:") {
            let mut title = [0u16; 64];
            let t = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut title);
            let ts = String::from_utf16_lossy(&title[..(t.max(0)) as usize]);
            if ts.starts_with("Q-Dir") {
                ctx.toolbars.push(hwnd); // 借用该槽临时存放主窗口
            }
        }
    }
    true.into()
}

/// 从根递归收集所有工具栏。
unsafe extern "system" fn child_proc(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
    let mut cls = [0u16; 64];
    let n = GetClassNameW(hwnd, &mut cls);
    if n > 0 {
        let name = String::from_utf16_lossy(&cls[..n as usize]);
        if name == "ToolbarWindow32" {
            let ctx = &mut *(lp.0 as *mut Ctx);
            ctx.toolbars.push(hwnd);
        }
    }
    true.into()
}

fn child_variant(id: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: core::mem::ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { lVal: id },
            }),
        },
    }
}

fn variant_i4(v: &VARIANT) -> Option<i32> {
    let inner = unsafe { &*v.Anonymous.Anonymous };
    if inner.vt == VT_I4 {
        Some(unsafe { inner.Anonymous.lVal })
    } else {
        None
    }
}

unsafe fn dump_toolbar(hwnd: HWND) {
    // OBJID_WINDOW = 0。Option<IAccessible> 以薄指针 niche 表示，
    // 双层指针强转后交给 COM 填充。
    let mut obj: Option<IAccessible> = None;
    let hr = AccessibleObjectFromWindow(
        hwnd,
        0,
        &IAccessible::IID,
        &mut obj as *mut Option<IAccessible> as *mut *mut core::ffi::c_void,
    );
    if hr.is_err() || obj.is_none() {
        println!("  (不可用: {hr:?})");
        return;
    }
    let acc = obj.unwrap();
    let count = match acc.accChildCount() {
        Ok(c) => c,
        Err(_) => return,
    };
    if count <= 0 {
        println!("  (无子元素)");
        return;
    }
    println!("  子元素数 = {count}");
    for id in 1..=count {
        let v = child_variant(id);
        let name = acc.get_accName(&v).map(|b| b.to_string()).unwrap_or_default();
        let role = acc
            .get_accRole(&v)
            .ok()
            .and_then(|r| variant_i4(&r))
            .unwrap_or(-99);
        let nm = name.replace('\n', "\\n");
        println!("   id={id} (0x{id:X}) role={role} name={nm:?}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pid: u32 = args[0].parse().expect("pid");
    unsafe {
        // 1. 找主框架窗口
        let mut root = Ctx { pid, toolbars: Vec::new() };
        let _ = EnumWindows(Some(top_proc), LPARAM(&mut root as *mut Ctx as isize));
        let Some(main) = root.toolbars.first().copied() else {
            println!("未找到 Q-Dir 主窗口 (pid={pid})");
            return;
        };
        println!("主窗口 {main:?}");
        // 2. 递归收集其工具栏
        let mut ctx = Ctx { pid, toolbars: Vec::new() };
        let _ = EnumChildWindows(Some(main), Some(child_proc), LPARAM(&mut ctx as *mut Ctx as isize));
        println!("找到 {} 个 ToolbarWindow32", ctx.toolbars.len());
        for h in &ctx.toolbars {
            if IsWindowVisible(*h).as_bool() {
                println!("toolbar {h:?}");
                dump_toolbar(*h);
            }
        }
    }
}
