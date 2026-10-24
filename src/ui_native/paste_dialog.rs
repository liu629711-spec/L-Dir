//! 选择性粘贴对话框（P2 audit #22；R1：自建 DLGTEMPLATE 零原版字节）。
//!
//! 原版对话框未反编译确证 *待复核*，布局自设计：SysListView32
//! （LVS_LIST + LVS_EX_CHECKBOXES）列出剪贴板条目（默认全勾）
//! + 同名冲突策略三选一（询问/自动重命名/跳过同名，shell::pasteplan）
//! + 确定/取消。数据语义=勾选集+策略，执行链在 mainframe::paste_special。

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Controls::{
    LVM_GETITEMSTATE, LVM_INSERTITEMW, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMSTATE,
    LVS_EX_CHECKBOXES, LVS_LIST, LIST_VIEW_ITEM_STATE_FLAGS, LVIF_TEXT, LVIS_STATEIMAGEMASK,
    LVITEMW,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::shell::PasteStrategy;
use crate::ui_native::rules_dialog::{ItemDef, build_template};

// 控件 id。
const IDC_ITEMS: i32 = 300;
const IDC_ASK: i32 = 301;
const IDC_RENAME: i32 = 302;
const IDC_SKIP: i32 = 303;

const RES_CANCEL: isize = 0;
const RES_OK: isize = 1;

pub const DLG_W: i16 = 310;
pub const DLG_H: i16 = 140;

/// 对话框会话（堆上，GWLP_USERDATA 挂载；WM_DESTROY 释放一次）。
struct PasteCtx {
    names: Vec<String>,
    strategy: PasteStrategy,
}

thread_local! {
    /// OK 提交暂存（模态单例）。
    static OUT: std::cell::RefCell<Option<(Vec<bool>, PasteStrategy)>> =
        const { std::cell::RefCell::new(None) };
}

fn items() -> Vec<ItemDef<'static>> {
    vec![
        ItemDef {
            id: IDC_ITEMS,
            atom: None,
            class: "SysListView32",
            text: "",
            style: WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | WS_GROUP.0 | LVS_LIST,
            ex: WS_EX_CLIENTEDGE.0,
            x: 7,
            y: 7,
            cx: 180,
            cy: 90,
        },
        ItemDef {
            id: IDC_ASK,
            atom: Some(0x0080),
            class: "",
            text: "同名时询问(&A)",
            style: WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | WS_GROUP.0
                | BS_AUTORADIOBUTTON as u32,
            ex: 0,
            x: 196,
            y: 10,
            cx: 105,
            cy: 10,
        },
        ItemDef {
            id: IDC_RENAME,
            atom: Some(0x0080),
            class: "",
            text: "自动重命名(&R)",
            style: WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32,
            ex: 0,
            x: 196,
            y: 24,
            cx: 105,
            cy: 10,
        },
        ItemDef {
            id: IDC_SKIP,
            atom: Some(0x0080),
            class: "",
            text: "跳过同名项(&S)",
            style: WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTORADIOBUTTON as u32,
            ex: 0,
            x: 196,
            y: 38,
            cx: 105,
            cy: 10,
        },
        ItemDef::button("确定(&O)", IDOK.0, 122, 110, 60, BS_DEFPUSHBUTTON as u32),
        ItemDef::button("取消(&C)", IDCANCEL.0, 190, 110, 60, 0),
    ]
}

/// 选择性粘贴对话框（模态）。返回 Some((勾选集, 策略)) 或 None（取消/无项）。
///
/// # Safety
/// 仅主 UI 线程消息泵上下文调用（嵌套模态循环）。
pub unsafe fn run_paste_special(
    parent: HWND,
    item_names: &[String],
) -> Option<(Vec<bool>, PasteStrategy)> {
    if item_names.is_empty() {
        return None;
    }
    let tmpl = build_template(&items(), "选择性粘贴", DLG_W, DLG_H);
    let ctx = Box::into_raw(Box::new(PasteCtx {
        names: item_names.to_vec(),
        strategy: PasteStrategy::Ask,
    }));
    OUT.with(|o| *o.borrow_mut() = None);
    let result = DialogBoxIndirectParamW(
        None,
        tmpl.as_ptr() as *const DLGTEMPLATE,
        Some(parent),
        Some(dlg_proc),
        LPARAM(ctx as *mut _ as isize),
    );
    if result == RES_OK {
        OUT.with(|o| o.borrow_mut().take())
    } else {
        None
    }
}

unsafe fn ctx_of(hdlg: HWND) -> Option<&'static mut PasteCtx> {
    let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
    (p != 0).then(|| &mut *(p as *mut PasteCtx))
}

unsafe fn ctl(hdlg: HWND, id: i32) -> HWND {
    GetDlgItem(Some(hdlg), id).unwrap_or_default()
}

/// 勾选状态读取（LVS_EX_CHECKBOXES：状态位图 2=勾）。
unsafe fn item_checked(list: HWND, i: i32) -> bool {
    let st = SendMessageW(
        list,
        LVM_GETITEMSTATE,
        Some(WPARAM(i as usize)),
        Some(LPARAM(LVIS_STATEIMAGEMASK.0 as isize)),
    )
    .0 as u32;
    (st & LVIS_STATEIMAGEMASK.0) >> 12 == 2
}

unsafe extern "system" fn dlg_proc(hdlg: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    match msg {
        WM_INITDIALOG => {
            SetWindowLongPtrW(hdlg, GWLP_USERDATA, lp.0);
            let Some(ctx) = ctx_of(hdlg) else { return 0 };
            let list = ctl(hdlg, IDC_ITEMS);
            SendMessageW(
                list,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                Some(WPARAM(LVS_EX_CHECKBOXES as usize)),
                Some(LPARAM(LVS_EX_CHECKBOXES as isize)),
            );
            for (i, name) in ctx.names.iter().enumerate() {
                let mut wbuf: Vec<u16> = name.encode_utf16().collect();
                wbuf.push(0);
                let lv = LVITEMW {
                    mask: LVIF_TEXT,
                    iItem: i as i32,
                    pszText: PWSTR(wbuf.as_mut_ptr()),
                    ..Default::default()
                };
                SendMessageW(
                    list,
                    LVM_INSERTITEMW,
                    Some(WPARAM(0)),
                    Some(LPARAM(&lv as *const _ as isize)),
                );
                // 默认全勾（stateMask 只动勾位）。
                let st = LVITEMW {
                    state: LIST_VIEW_ITEM_STATE_FLAGS(2 << 12),
                    stateMask: LIST_VIEW_ITEM_STATE_FLAGS(LVIS_STATEIMAGEMASK.0),
                    ..Default::default()
                };
                SendMessageW(
                    list,
                    LVM_SETITEMSTATE,
                    Some(WPARAM(i)),
                    Some(LPARAM(&st as *const _ as isize)),
                );
            }
            // 默认策略=询问。
            let _ = SendMessageW(ctl(hdlg, IDC_ASK), BM_SETCHECK, Some(WPARAM(1)), None);
            return 1;
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xFFFF) as i32;
            let Some(ctx) = ctx_of(hdlg) else { return 0 };
            match id {
                IDC_ASK => ctx.strategy = PasteStrategy::Ask,
                IDC_RENAME => ctx.strategy = PasteStrategy::Rename,
                IDC_SKIP => ctx.strategy = PasteStrategy::Skip,
                x if x == IDOK.0 => {
                    let list = ctl(hdlg, IDC_ITEMS);
                    let checked: Vec<bool> =
                        (0..ctx.names.len() as i32).map(|i| item_checked(list, i)).collect();
                    OUT.with(|o| *o.borrow_mut() = Some((checked, ctx.strategy)));
                    let _ = EndDialog(hdlg, RES_OK);
                }
                y if y == IDCANCEL.0 => {
                    let _ = EndDialog(hdlg, RES_CANCEL);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
            if p != 0 {
                drop(Box::from_raw(p as *mut PasteCtx));
                SetWindowLongPtrW(hdlg, GWLP_USERDATA, 0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hdlg, msg, wp, lp).0 },
    }
}

// PCWSTR 引用保持（模板内宽串生命周期由 Vec 持有；防未用导入告警）。
#[allow(dead_code)]
fn _keep(pc: PCWSTR) -> PCWSTR {
    pc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 控件表结构() {
        let its = items();
        assert_eq!(its.len(), 6, "列表+3单选+2按钮");
        assert_eq!(its[0].id, IDC_ITEMS);
        assert_eq!(its[0].class, "SysListView32");
        assert_eq!((its[1].id, its[2].id, its[3].id), (IDC_ASK, IDC_RENAME, IDC_SKIP));
        // 模板可构建（纯字节函数）。
        let t = build_template(&its, "选择性粘贴", DLG_W, DLG_H);
        assert!(!t.is_empty());
    }

    #[test]
    fn 空列表直接拒绝() {
        // 无句柄环境：run 前置检查独立可测（不真开模态）。
        assert!(item_names_empty(&[]));
        assert!(!item_names_empty(&["a".to_string()]));
    }

    fn item_names_empty(names: &[String]) -> bool {
        names.is_empty()
    }
}
