//! 控件子类化：给每窗格的 list/addr_edit 挂键盘导航，tab 挂鼠标交互。
//!
//! 原版（WTL）在窗格视图上处理：Enter=打开选中、Backspace=返回/后退；
//! 标签中键/双击关闭由 TabConfig 开关决策（core::tab_click_action）。
//! 这里用 comctl32 的 SetWindowSubclass 等价实现；回调里经全局
//! MAIN_HWND 找到主框架帧对象。
//!
//! 批次A（FEATURE_GAP_AUDIT #15/#16/#17/#18/#20/#4）键盘接线（list 角色）：
//! Ctrl+C/X/V（CF_HDROP 剪贴板，作用域=发键窗格）、Ctrl+A 全选、Del 删除
//! （默认进回收站）、Ctrl+T 新建标签页、F2 内联重命名；外加 ComboBox
//! 选盘导航（CBN_SELCHANGE）。与加速键表（app.rs）双通道并存：Ctrl+C/A
//! 等保持**局部**（仅列表焦点时生效，Explorer 语义），不进全局加速键。
//!
//! refdata 编码：`pane_index << 2 | role`（role 0=list, 1=addr_edit, 2=tab）。

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Controls::{
    HDN_ITEMCLICKW, TCM_HITTEST, TCHITTESTINFO, TCHITTESTINFO_FLAGS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_A, VK_BACK, VK_C, VK_CONTROL, VK_DELETE, VK_F2,
    VK_RETURN, VK_SHIFT, VK_T, VK_V, VK_X,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetSystemMetrics, SendMessageW, SYSTEM_METRICS_INDEX, WM_KEYDOWN,
    WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONUP, WM_MOUSEMOVE, WM_NOTIFY,
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

/// 拖拽起点（批次B/B2）：list 角色 WM_LBUTTONDOWN 记录（客户端坐标+窗格），
/// WM_MOUSEMOVE 超 SM_CXDRAG/SM_CYDRAG 阈值且左键仍按下 → 触发拖出。
/// 鼠标抬起/捕获丢失时清除。
static mut DRAG_START: Option<(POINT, usize)> = None;

const ROLE_LIST: usize = 0;
const ROLE_ADDR: usize = 1;
const ROLE_TAB: usize = 2;

/// `MK_LBUTTON`（WM_MOUSEMOVE wParam 位）。
const MK_LBUTTON_BIT: usize = 0x0001;
/// `SM_CXDRAG` / `SM_CYDRAG`（GetSystemMetrics 索引；WinUser.h）。
const SM_CXDRAG: i32 = 69;
const SM_CYDRAG: i32 = 70;

pub unsafe fn set_main_hwnd(h: HWND) {
    MAIN_HWND = h.0 as isize;
}

fn main_hwnd() -> Option<HWND> {
    let v = unsafe { MAIN_HWND };
    (v != 0).then(|| HWND(v as *mut core::ffi::c_void))
}

/// Ctrl 是否按下：真键盘走 GetKeyState；进程内钩子合成消息（SendMessageW
/// 不更新键盘状态）用 WM_KEYDOWN lParam bit24（context code）声明。
/// 纯函数只看该位，便于单测。
fn key_ctrl_held(lp: LPARAM) -> bool {
    ctrl_bit_from_lparam(lp) || (unsafe { GetKeyState(VK_CONTROL.0 as i32) } as i16) < 0
}

/// WM_KEYDOWN lParam 的 context-code 位（bit24）——Ctrl 修饰声明。纯函数。
pub fn ctrl_bit_from_lparam(lp: LPARAM) -> bool {
    (lp.0 >> 24) & 1 != 0
}

/// 列表 Ctrl 组合键 → 逻辑动作（纯函数，可单测）。未知键返回 None（放行）。
pub fn ctrl_list_action(vk: u32) -> Option<crate::ui_native::mainframe::KbAction> {
    use crate::ui_native::mainframe::KbAction;
    let a = match vk {
        v if v == VK_C.0 as u32 => KbAction::Copy,
        v if v == VK_X.0 as u32 => KbAction::Cut,
        v if v == VK_V.0 as u32 => KbAction::Paste,
        v if v == VK_A.0 as u32 => KbAction::SelectAll,
        v if v == VK_DELETE.0 as u32 => KbAction::Delete,
        v if v == VK_T.0 as u32 => KbAction::NewTab,
        _ => return None,
    };
    Some(a)
}

/// 子类回调：list 处理 Enter(打开选中)/Backspace(返回)/Ctrl 组合键/F2；
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
    // 拖拽起点链（批次B/B2）：list 角色左键按下记起点（默认处理照走选中，
    // 之后 SetCapture 保证移动事件不丢）；WM_MOUSEMOVE 超系统拖拽阈值且左键
    // 仍按下 → ReleaseCapture 后 begin_drag_ex（Ctrl→COPY；Shift/默认→MOVE，
    // Q-Dir 窗格间默认移动 *待实测复核*）。DoDragDrop 模态在 WM_MOUSEMOVE 内
    // 运行属正常用法；Move 成功由 start_pane_drag 重灌源窗格。
    if msg == WM_LBUTTONDOWN {
        let pidx = refdata >> 2;
        let role = refdata & 3;
        if role == ROLE_LIST && pidx < 4 {
            let pt = POINT {
                x: ((lp.0 & 0xFFFF) as i16) as i32,
                y: ((lp.0 >> 16) as i16) as i32,
            };
            unsafe { DRAG_START = Some((pt, pidx)) };
            let r = DefSubclassProc(hwnd, msg, wp, lp);
            let _ = unsafe { SetCapture(hwnd) };
            return r;
        }
    }
    if msg == WM_LBUTTONUP {
        let pidx = refdata >> 2;
        let role = refdata & 3;
        if role == ROLE_LIST && pidx < 4 {
            unsafe { DRAG_START = None };
            let r = DefSubclassProc(hwnd, msg, wp, lp);
            let _ = unsafe { ReleaseCapture() };
            return r;
        }
    }
    if msg == WM_MOUSEMOVE {
        let pidx = refdata >> 2;
        let role = refdata & 3;
        if role == ROLE_LIST && pidx < 4 {
            if let Some((s, _)) = unsafe { DRAG_START } {
                if wp.0 & MK_LBUTTON_BIT != 0 {
                    let pt = POINT {
                        x: ((lp.0 & 0xFFFF) as i16) as i32,
                        y: ((lp.0 >> 16) as i16) as i32,
                    };
                    let cx = unsafe { GetSystemMetrics(SYSTEM_METRICS_INDEX(SM_CXDRAG)) };
                    let cy = unsafe { GetSystemMetrics(SYSTEM_METRICS_INDEX(SM_CYDRAG)) };
                    if (pt.x - s.x).abs() > cx || (pt.y - s.y).abs() > cy {
                        unsafe { DRAG_START = None };
                        let _ = unsafe { ReleaseCapture() };
                        let done = unsafe { start_pane_drag(pidx) };
                        if done {
                            return LRESULT(0);
                        }
                    }
                } else {
                    // 左键已松开（捕获外抬起）：清起点不拖。
                    unsafe { DRAG_START = None };
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
                // list 角色 Ctrl 组合键（批次A）：作用域=发键窗格。
                // 吞掉（返回 0）以免原生 ListView 把 Ctrl+C 当文本复制。
                if role == ROLE_LIST && key_ctrl_held(lp) {
                    if let Some(action) = ctrl_list_action(vk) {
                        let done = frame.keyboard_action(pidx, action, main);
                        println!(
                            "[qdir-rs] TEST 键盘 Ctrl+{:?} 窗格{} -> {} (lparam声明={})",
                            action,
                            pidx + 1,
                            if done { "完成" } else { "空操作" },
                            ctrl_bit_from_lparam(lp)
                        );
                        return LRESULT(0);
                    }
                }
                // Del（无修饰）= 删除选中 → 回收站（批次A/A1+A2）。
                if role == ROLE_LIST && vk == VK_DELETE.0 as u32 && !key_ctrl_held(lp) {
                    let done = frame.keyboard_action(pidx, crate::ui_native::mainframe::KbAction::Delete, main);
                    println!(
                        "[qdir-rs] TEST 键盘 Del 窗格{} -> {}",
                        pidx + 1,
                        if done { "删除完成" } else { "空操作(无选中)" }
                    );
                    return LRESULT(0);
                }
                // F2（无修饰）= 内联重命名（选中恰一行时）。
                if role == ROLE_LIST && vk == VK_F2.0 as u32 && !key_ctrl_held(lp) {
                    let ok = frame.start_inline_rename(pidx);
                    println!(
                        "[qdir-rs] TEST 键盘 F2 窗格{} -> {}",
                        pidx + 1,
                        if ok { "进入编辑" } else { "不可编辑(非单行选中)" }
                    );
                    return LRESULT(0);
                }
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

/// 触发一次窗格拖出（批次B/B2）：读选中路径 → 按键决议 preferred →
/// `begin_drag_ex`（allowed=COPY|MOVE 双许可）→ Move 成功则重灌源窗格。
/// 返回是否真的进入了拖放（无选中/启动失败返回 false，调用方继续默认处理）。
unsafe fn start_pane_drag(pidx: usize) -> bool {
    let Some(main) = main_hwnd() else {
        return false;
    };
    let Some(frame) = MainFrame::frame_of(main) else {
        return false;
    };
    let paths = frame.selected_paths(pidx);
    if paths.is_empty() {
        return false;
    }
    let host = crate::shell::HostWindow::from_raw_hwnd(main.0 as isize);
    let ctrl = (unsafe { GetKeyState(VK_CONTROL.0 as i32) } as i16) < 0;
    let shift = (unsafe { GetKeyState(VK_SHIFT.0 as i32) } as i16) < 0;
    // Ctrl→COPY；Shift/默认→MOVE（Q-Dir 窗格间默认移动 *待实测复核*）。
    let preferred = if ctrl {
        crate::shell::DROPEFFECT_COPY_VALUE
    } else if shift {
        crate::shell::DROPEFFECT_MOVE_VALUE
    } else {
        crate::shell::DROPEFFECT_MOVE_VALUE
    };
    let allowed = crate::shell::DROPEFFECT_COPY_VALUE | crate::shell::DROPEFFECT_MOVE_VALUE;
    println!(
        "[qdir-rs] TEST 拖出 窗格{} {} 项 ctrl={ctrl} shift={shift}",
        pidx + 1,
        paths.len()
    );
    match crate::shell::begin_drag_ex(host, &paths, allowed, preferred) {
        Ok(crate::shell::DragOutcome::Move) => {
            // 移动成功：源文件已不在，重灌源窗格。
            unsafe { frame.fill_pane(pidx) };
            println!("[qdir-rs] TEST 拖出结果 移动 窗格{} 已重灌", pidx + 1);
            true
        }
        Ok(o) => {
            println!("[qdir-rs] TEST 拖出结果 {o:?} 窗格{}", pidx + 1);
            true
        }
        Err(e) => {
            eprintln!("[qdir-rs] 拖出失败：{e}");
            false
        }
    }
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

/// 合成一次 list 角色的 Ctrl 组合键（进程内测试钩子用；SendMessageW 走
/// 完整子类化链，lParam bit24 声明 Ctrl 态——SendMessage 不更新键盘状态）。
pub unsafe fn synth_ctrl_key(list: HWND, vk: u32) {
    // bit0=1 重复计数 | bit24=1 context code（Ctrl 声明）| bit30=1 转换状态。
    let lp = LPARAM(1 | (1 << 24) | (1 << 30));
    let _ = SendMessageW(list, WM_KEYDOWN, Some(WPARAM(vk as usize)), Some(lp));
}

/// 合成一次 list 角色的**无修饰** WM_KEYDOWN（F2 等；bit24=0 → 子类化
/// 按无 Ctrl 处理）。
pub unsafe fn synth_plain_key(list: HWND, vk: u32) {
    let lp = LPARAM(1 | (1 << 30));
    let _ = SendMessageW(list, WM_KEYDOWN, Some(WPARAM(vk as usize)), Some(lp));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_native::mainframe::KbAction;

    #[test]
    fn ctrl位仅认bit24() {
        assert!(ctrl_bit_from_lparam(LPARAM(1 << 24)));
        assert!(ctrl_bit_from_lparam(LPARAM((1 << 24) | 1)));
        assert!(!ctrl_bit_from_lparam(LPARAM(1)));
        assert!(!ctrl_bit_from_lparam(LPARAM(1 << 23)));
        assert!(!ctrl_bit_from_lparam(LPARAM(0)));
    }

    #[test]
    fn ctrl组合键映射六动作() {
        assert_eq!(ctrl_list_action(VK_C.0 as u32), Some(KbAction::Copy));
        assert_eq!(ctrl_list_action(VK_X.0 as u32), Some(KbAction::Cut));
        assert_eq!(ctrl_list_action(VK_V.0 as u32), Some(KbAction::Paste));
        assert_eq!(ctrl_list_action(VK_A.0 as u32), Some(KbAction::SelectAll));
        assert_eq!(ctrl_list_action(VK_DELETE.0 as u32), Some(KbAction::Delete));
        assert_eq!(ctrl_list_action(VK_T.0 as u32), Some(KbAction::NewTab));
        // 未绑定的组合键放行（返回 None，交默认过程）。
        assert_eq!(ctrl_list_action(VK_RETURN.0 as u32), None);
        assert_eq!(ctrl_list_action(windows::Win32::UI::Input::KeyboardAndMouse::VK_F3.0 as u32), None);
    }
}
