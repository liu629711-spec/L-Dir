//! 控件子类化：给每窗格的 list/addr_edit 挂键盘导航。
//!
//! 原版（WTL）在窗格视图上处理：Enter=打开选中、Backspace=返回/后退。
//! 这里用 comctl32 的 SetWindowSubclass 等价实现；回调里经全局
//! MAIN_HWND 找到主框架帧对象。
//!
//! refdata 编码：`pane_index << 1 | role`（role 0=list, 1=addr_edit）。

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Controls::HDN_ITEMCLICKW;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_BACK, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::WM_NOTIFY;

/// NMHEADERW 布局（header 通知发给其父 ListView）；仅取 hdr.code 与 iItem。
#[repr(C)]
struct NmHeader {
    hdr: windows::Win32::UI::Controls::NMHDR,
    i_item: i32,
    i_button: i32,
    p_item: *mut core::ffi::c_void,
}
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, WM_KEYDOWN,
};

use crate::ui_native::mainframe::MainFrame;

/// 主框架 HWND（单实例进程；on_create 时设置一次）。
static mut MAIN_HWND: isize = 0;

const ROLE_LIST: usize = 0;
const ROLE_ADDR: usize = 1;

pub unsafe fn set_main_hwnd(h: HWND) {
    MAIN_HWND = h.0 as isize;
}

fn main_hwnd() -> Option<HWND> {
    let v = unsafe { MAIN_HWND };
    (v != 0).then(|| HWND(v as *mut core::ffi::c_void))
}

/// 子类回调：list 处理 Enter(打开选中)/Backspace(返回)；
/// addr_edit 处理 Enter(导航到输入路径)。
unsafe extern "system" fn sub_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _uid: usize,
    refdata: usize,
) -> LRESULT {
    if msg == WM_NOTIFY {
        // header 通知发到其父窗（ListView），这里截列头点击做排序。
        if lp.0 != 0 {
            let nmi = unsafe { &*(lp.0 as *const NmHeader) };
            if nmi.hdr.code == HDN_ITEMCLICKW {
                let pidx = refdata >> 1;
                let role = refdata & 1;
                if role == ROLE_LIST && pidx < 4 {
                    if let Some(main) = main_hwnd() {
                        if let Some(frame) = MainFrame::frame_of(main) {
                            unsafe { frame.apply_column_sort(pidx, nmi.i_item.max(0) as usize) };
                            return LRESULT(0);
                        }
                    }
                }
            }
        }
        return DefSubclassProc(hwnd, msg, wp, lp);
    }
    if msg == WM_KEYDOWN {
        let pidx = refdata >> 1;
        let role = refdata & 1;
        let vk = wp.0 as u32;
        if pidx >= 4 {
            return DefSubclassProc(hwnd, msg, wp, lp);
        }
        if let Some(main) = main_hwnd() {
            if let Some(frame) = MainFrame::frame_of(main) {
                let handled = match role {
                    ROLE_LIST if vk == VK_RETURN.0 as u32 => {
                        frame.open_selection(pidx);
                        true
                    }
                    ROLE_LIST if vk == VK_BACK.0 as u32 => {
                        frame.go_parent(pidx);
                        true
                    }
                    ROLE_ADDR if vk == VK_RETURN.0 as u32 => {
                        frame.addr_enter(pidx);
                        true
                    }
                    _ => false,
                };
                if handled {
                    return LRESULT(0);
                }
            }
        }
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

/// 挂接一个窗格的可交互控件。
pub unsafe fn install(hwnd_list: Option<HWND>, hwnd_addr: Option<HWND>, pidx: usize) {
    if let Some(h) = hwnd_list {
        // 防御：仅对 SysListView32 挂 list 角色。
        let mut cls = [0u16; 32];
        let n = GetClassNameW(h, &mut cls);
        if n > 0 && String::from_utf16_lossy(&cls[..n as usize]) == "SysListView32" {
            let _ = SetWindowSubclass(h, Some(sub_proc), 1, (pidx << 1) | ROLE_LIST);
        }
    }
    if let Some(h) = hwnd_addr {
        let _ = SetWindowSubclass(h, Some(sub_proc), 1, (pidx << 1) | ROLE_ADDR);
    }
}

