//! 列头探针：对给定 pid，找每个 SysListView32 的 SysHeader32，
//! 打印窗口样式(GWL_STYLE)与每个 header item 的 fmt（解码 HDF_* 位）。
//! 用法: header_probe <pid>

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Controls::{
    HDITEMW, HDI_MASK, HDS_BUTTONS, HDS_DRAGDROP, HDS_HOTTRACK, HDF_FIXEDWIDTH, HDF_SPLITBUTTON,
    HDF_SORTDOWN, HDF_SORTUP, LVM_GETHEADER,
};

// HD 消息（windows-rs 0.62 未导出，值来自 commctrl.h）
const HDM_GETITEMCOUNT: u32 = 0x1204;
const HDM_GETITEMW: u32 = 0x120B;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetWindowLongW, GetWindowThreadProcessId,
    GWL_STYLE, SendMessageW,
    GetWindowTextW,
};

struct Sink {
    pid: u32,
    main: Option<HWND>,
    lists: Vec<HWND>,
}

unsafe extern "system" fn top_proc(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let ctx = &mut *(lp.0 as *mut Sink);
    if pid == ctx.pid && ctx.main.is_none() {
        let mut title = [0u16; 64];
        let t = GetWindowTextW(hwnd, &mut title);
        let ts = String::from_utf16_lossy(&title[..(t.max(0)) as usize]);
        if ts.starts_with("Q-Dir") {
            ctx.main = Some(hwnd);
        }
    }
    true.into()
}

unsafe extern "system" fn enum_children(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
    let mut cls = [0u16; 64];
    let n = GetClassNameW(hwnd, &mut cls);
    if n > 0 && String::from_utf16_lossy(&cls[..n as usize]) == "SysListView32" {
        (&mut *(lp.0 as *mut Sink)).lists.push(hwnd);
    }
    true.into()
}

fn style_names(style: u32) -> String {
    let mut v = vec![];
    if style & HDS_BUTTONS != 0 { v.push("HDS_BUTTONS"); }
    if style & HDS_DRAGDROP != 0 { v.push("HDS_DRAGDROP"); }
    if style & HDS_HOTTRACK != 0 { v.push("HDS_HOTTRACK"); }
    v.join("|")
}

fn main() {
    let pid: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    unsafe {
        let pid_arg = pid;
        let mut sink = Sink { pid, main: None, lists: Vec::new() };
        let _ = EnumWindows(Some(top_proc), LPARAM(&mut sink as *mut Sink as isize));
        let Some(main) = sink.main else {
            println!("未找到主窗口 pid={pid_arg}");
            return;
        };
        let _ = EnumChildWindows(
            Some(main),
            Some(enum_children),
            LPARAM(&mut sink as *mut Sink as isize),
        );
        let sink = sink;
        let mut n = 0u32;
        for lv in &sink.lists {
            GetWindowThreadProcessId(*lv, Some(&mut n));
            if n != pid {
                continue;
            }
            let hdr = SendMessageW(*lv, LVM_GETHEADER, None, None).0;
            if hdr == 0 {
                continue;
            }
            let hdr_h = HWND(hdr as *mut core::ffi::c_void);
            let style = GetWindowLongW(hdr_h, GWL_STYLE) as u32;
            let count = SendMessageW(hdr_h, HDM_GETITEMCOUNT, None, None).0;
            println!("list {lv:?} header {hdr_h:?} style={style:#x} [{}] items={count}", style_names(style));
            for i in 0..count {
                let mut it = HDITEMW::default();
                it.mask = HDI_MASK(4); // HDI_FORMAT
                let r = SendMessageW(
                    hdr_h,
                    HDM_GETITEMW,
                    Some(WPARAM(i as usize)),
                    Some(LPARAM(&mut it as *mut HDITEMW as isize)),
                );
                if r.0 == 0 {
                    println!("  [{i}] getitem 失败");
                    continue;
                }
                let fmt = it.fmt.0;
                let mut tags = vec![];
                if fmt & HDF_FIXEDWIDTH.0 != 0 { tags.push("FIXEDWIDTH"); }
                if fmt & HDF_SPLITBUTTON.0 != 0 { tags.push("SPLITBUTTON"); }
                if fmt & HDF_SORTUP.0 != 0 { tags.push("SORTUP"); }
                if fmt & HDF_SORTDOWN.0 != 0 { tags.push("SORTDOWN"); }
                println!("  [{i}] fmt={fmt:#x} [{}]", tags.join("|"));
            }
        }
    }
}
