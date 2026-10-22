//! 控件子类化：给每窗格的 list/addr_edit 挂键盘导航，tab 挂鼠标交互。
//!
//! 原版（WTL）在窗格视图上处理：Enter=打开选中、Backspace=返回/后退；
//! 标签中键/双击关闭由 TabConfig 开关决策（core::tab_click_action）。
//! 这里用 comctl32 的 SetWindowSubclass 等价实现；回调里经全局
//! MAIN_HWND 找到主框架帧对象。
//!
//! refdata 编码：`pane_index << 2 | role`（role 0=list, 1=addr_edit, 2=tab）。

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Controls::{
    HDN_ITEMCLICKW, TCM_HITTEST, TCHITTESTINFO, TCHITTESTINFO_FLAGS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_BACK, VK_RETURN};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, SendMessageW, WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_MBUTTONUP, WM_NOTIFY,
};

/// NMHEADERW 布局（header 通知发给其父 ListView）；仅取 hdr.code 与 iItem。
#[repr(C)]
struct NmHeader {
    hdr: windows::Win32::UI::Controls::NMHDR,
    i_item: i32,
    i_button: i32,
    p_item: *mut core::ffi::c_void,
}

use crate::ui_native::mainframe::MainFrame;

/// 主框架 HWND（单实例进程；on_create 时设置一次）。
static mut MAIN_HWND: isize = 0;

const ROLE_LIST: usize = 0;
const ROLE_ADDR: usize = 1;
const ROLE_TAB: usize = 2;

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
                let pidx = refdata >> 2;
                let role = refdata & 3;
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
    // 标签交互（P2 开关，core::tab_click_action 决策）：
    // 中键=关闭（close_by_mbt_clk），左键双击=关闭（close_by_lbt_dblclk，
    // 开关关时也吞掉 dblclk 防标签跳变）。TCM_HITTEST 定位命中标签。
    if msg == WM_MBUTTONUP || msg == WM_LBUTTONDBLCLK {
        let pidx = refdata >> 2;
        let role = refdata & 3;
        if role == ROLE_TAB && pidx < 4 {
            let m = if msg == WM_MBUTTONUP {
                crate::core::TabMouse::MiddleUp
            } else {
                crate::core::TabMouse::LeftDblClk
            };
            if let Some(main) = main_hwnd() {
                if let Some(frame) = MainFrame::frame_of(main) {
                    let action = crate::core::tab_click_action(&frame.config.tab, m);
                    if let Some(close) = action {
                        // 命中检测（客户端坐标）
                        let pt = POINT {
                            x: ((lp.0 & 0xFFFF) as i16) as i32,
                            y: ((lp.0 >> 16) as i16) as i32,
                        };
                        let mut hti = TCHITTESTINFO {
                            pt,
                            flags: TCHITTESTINFO_FLAGS(0),
                        };
                        let r = unsafe {
                            SendMessageW(
                                hwnd,
                                TCM_HITTEST,
                                Some(WPARAM(0)),
                                Some(LPARAM(&mut hti as *mut _ as isize)),
                            )
                        };
                        if r.0 >= 0 && close {
                            let tab_idx = r.0 as usize;
                            let closed = unsafe { frame.close_tab(pidx, tab_idx) };
                            println!(
                                "[qdir-rs] TEST 标签交互 窗格{} {:?} 关闭标签{} -> {}",
                                pidx + 1,
                                m,
                                tab_idx,
                                closed.map_or("拒绝(最后一个)".to_string(), |_| "完成".to_string())
                            );
                            return LRESULT(0);
                        }
                        // close=false：吞事件不关闭。
                        return LRESULT(0);
                    }
                }
            }
        }
    }
    if msg == WM_KEYDOWN {
        let pidx = refdata >> 2;
        let role = refdata & 3;
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
pub unsafe fn install(
    hwnd_list: Option<HWND>,
    hwnd_addr: Option<HWND>,
    hwnd_tab: Option<HWND>,
    pidx: usize,
) {
    if let Some(h) = hwnd_tab {
        // 防御：仅对 SysTabControl32 挂 tab 角色。
        let mut cls = [0u16; 32];
        let n = GetClassNameW(h, &mut cls);
        if n > 0 && String::from_utf16_lossy(&cls[..n as usize]) == "SysTabControl32" {
            let _ = SetWindowSubclass(h, Some(sub_proc), 1, (pidx << 2) | ROLE_TAB);
        }
    }
    if let Some(h) = hwnd_list {
        // 防御：仅对 SysListView32 挂 list 角色。
        let mut cls = [0u16; 32];
        let n = GetClassNameW(h, &mut cls);
        if n > 0 && String::from_utf16_lossy(&cls[..n as usize]) == "SysListView32" {
            let _ = SetWindowSubclass(h, Some(sub_proc), 1, (pidx << 2) | ROLE_LIST);
        }
    }
    if let Some(h) = hwnd_addr {
        let _ = SetWindowSubclass(h, Some(sub_proc), 1, (pidx << 2) | ROLE_ADDR);
    }
}

