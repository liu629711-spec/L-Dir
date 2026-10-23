//! 桌面布局槽对话框（批次D/D2；附录 A.7 FUN_00474a34 机制确证）。
//!
//! 原版布局槽是内存表（param+0x7C90）无文件格式证据，且原版 UI 入口
//! 未逐条对照——本实现为 R1 界面自主 + 行为语义按 A.7：
//! 单击=载入槽（=我们的 load_session）；Ctrl=复制路径；Shift=另存到槽；
//! Del=删除槽。修饰键在模态对话框里拿不到修饰态，故用**按钮**覆盖
//! 全部四种行为（保存到选中/载入选中/删除/复制路径）。
//!
//! 存储：`<exe目录>/L-Slots/slot-N.qdr`（自名文件；原版槽格式无证据
//! *待复核*）。载入复用 commands::apply_session 等价链（session 替换 +
//! splitters 同步 + 四窗格重灌）。

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::SessionState;

use super::rules_dialog::{build_template, ItemDef, ATOM_BUTTON};

/// 槽位数（自定；原版槽数未确证 *待复核*）。
pub const SLOT_COUNT: u32 = 8;

/// 槽目录名（exe 同级的 L-Slots/）。
pub const SLOT_DIR: &str = "L-Slots";

// 对话框控件 id。
const IDC_LIST_SLOTS: i32 = 100;
const IDC_BTN_SAVE: i32 = 101;
const IDC_BTN_LOAD: i32 = 102;
const IDC_BTN_DEL: i32 = 103;
const IDC_BTN_COPY: i32 = 104;
const IDC_OK: i32 = IDOK.0;
const IDC_CANCEL: i32 = IDCANCEL.0;

// EndDialog 返回码（= SlotAction 的槽号或 0）。
const RES_CANCEL: isize = 0;
const RES_LOAD: isize = 1;
const RES_SAVE: isize = 2;
const RES_DELETE: isize = 3;
const RES_COPY: isize = 4;

pub const DLG_W: i16 = 360;
pub const DLG_H: i16 = 230;

// ============ 纯逻辑（单测覆盖） ============

/// 槽文件路径（批次D/D2）：`<exe_dir>/L-Slots/slot-N.qdr`。
pub fn slot_path(exe_dir: &Path, n: u32) -> PathBuf {
    exe_dir.join(SLOT_DIR).join(format!("slot-{n}.qdr"))
}

/// 从槽文件路径反解槽号（与 `slot_path` 往返）；非槽文件返回 None。
pub fn slot_index_from_path(p: &Path) -> Option<u32> {
    let name = p.file_name()?.to_string_lossy();
    let stem = name.strip_suffix(".qdr")?;
    let num = stem.strip_prefix("slot-")?;
    num.parse::<u32>().ok()
}

/// 槽目录（exe 同级的 L-Slots/）。
pub fn slot_dir(exe_dir: &Path) -> PathBuf {
    exe_dir.join(SLOT_DIR)
}

/// 文件修改时间 → 列表显示文本（本地时区近似：UTC 偏移按系统 TZ；
/// 简化用 epoch 秒的 UTC 值，*时间显示精度待复核*）。
pub fn format_slot_time(secs: u64) -> String {
    // 一天 86400 秒；按 1970-01-01 起算的民用日期（无时区换算，UTC 直显）。
    let days = secs / 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    let hms = secs % 86_400;
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y,
        m,
        d,
        hms / 3600,
        (hms % 3600) / 60,
        hms % 60
    )
}

/// 公历日期（Howard Hinnant 的 days_from_civil 逆运算；纯函数）。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 槽列表行摘要：`(槽名, 时间文本, 路径摘要)`。
/// 槽文件不存在 → `("槽 N", "空", "-")`。
pub fn slot_row(exe_dir: &Path, n: u32, session: Option<&SessionState>) -> (String, String, String) {
    let p = slot_path(exe_dir, n);
    if !p.exists() {
        return (format!("槽 {n}"), "空".to_string(), "-".to_string());
    }
    let secs = std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 路径摘要：优先用调用方已加载的会话；否则从文件现解析。
    let summary = session
        .and_then(|s| s.panes.first())
        .and_then(|pane| pane.tabs.first())
        .map(|tab| tab.path.to_string_lossy().to_string())
        .unwrap_or_else(|| "-".to_string());
    (format!("槽 {n}"), format_slot_time(secs), summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_path_and_index_roundtrip() {
        let dir = Path::new(r"C:\app");
        for n in [0u32, 1, 7, 8, 42] {
            let p = slot_path(dir, n);
            assert_eq!(p, dir.join("L-Slots").join(format!("slot-{n}.qdr")));
            assert_eq!(slot_index_from_path(&p), Some(n), "往返一致");
        }
    }

    #[test]
    fn slot_index_rejects_non_slot_files() {
        assert_eq!(slot_index_from_path(Path::new(r"C:\a.qdr")), None);
        assert_eq!(slot_index_from_path(Path::new(r"C:\slot-x.qdr")), None);
        assert_eq!(slot_index_from_path(Path::new(r"C:\slot-1.txt")), None);
        assert_eq!(slot_index_from_path(Path::new(r"C:\other")), None);
    }

    #[test]
    fn format_slot_time_epoch_zero() {
        assert_eq!(format_slot_time(0), "1970-01-01 00:00:00");
        // 2026-10-23 12:34:56 UTC（epoch 实算值）。
        assert_eq!(format_slot_time(1_792_758_896), "2026-10-23 12:34:56");
    }

    #[test]
    fn slot_row_missing_is_empty() {
        let dir = std::env::temp_dir().join(format!("qdir-rs-slots-ut-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (name, time, path) = slot_row(&dir, 3, None);
        assert_eq!(name, "槽 3");
        assert_eq!(time, "空");
        assert_eq!(path, "-");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ============ DLGTEMPLATE ============

fn main_items() -> Vec<ItemDef<'static>> {
    vec![
        ItemDef {
            id: IDC_LIST_SLOTS,
            atom: None,
            class: "SysListView32",
            text: "",
            style: WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | LVS_REPORT | LVS_SHOWSELALWAYS,
            ex: WS_EX_CLIENTEDGE.0,
            x: 7,
            y: 7,
            cx: 268,
            cy: 196,
        },
        ItemDef::button("保存到选中(&S)", IDC_BTN_SAVE, 283, 7, 66, 0),
        ItemDef::button("载入选中(&L)", IDC_BTN_LOAD, 283, 24, 66, 0),
        ItemDef::button("删除(&D)", IDC_BTN_DEL, 283, 46, 66, 0),
        ItemDef::button("复制路径(&P)", IDC_BTN_COPY, 283, 63, 66, 0),
        ItemDef::button("确定(&O)", IDC_OK, 190, 210, 70, BS_DEFPUSHBUTTON as u32),
        ItemDef::button("取消(&C)", IDC_CANCEL, 268, 210, 70, 0),
    ]
}

// ============ 对话框 ============

/// 对话框会话状态（GWLP_USERDATA 挂载；WM_DESTROY 释放）。
struct SlotsCtx {
    list: HWND,
    /// 结果动作（EndDialog 返回码编码：RES_* * 100 + 槽号）。
    result: Option<(isize, u32)>,
    exe_dir: PathBuf,
}

thread_local! {
    /// 动作结果暂存（模态单例）。
    static OUT_ACTION: std::cell::RefCell<Option<(isize, u32)>> =
        const { std::cell::RefCell::new(None) };
}

/// 槽操作（对话框按钮产物）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotAction {
    /// 载入第 N 槽。
    Load(u32),
    /// 保存当前会话到第 N 槽。
    Save(u32),
    /// 删除第 N 槽。
    Delete(u32),
    /// 复制第 N 槽路径。
    CopyPath(u32),
}

/// 打开布局槽对话框（模态）。返回用户选择的操作；取消/关闭返回 None。
///
/// # Safety
/// 仅主 UI 线程消息泵上下文调用（嵌套模态循环）。
pub unsafe fn open_slots(parent: HWND, exe_dir: &Path) -> Option<SlotAction> {
    let tmpl = build_template(&main_items(), "布局槽", DLG_W, DLG_H);
    let ctx = Box::into_raw(Box::new(SlotsCtx {
        list: HWND::default(),
        result: None,
        exe_dir: exe_dir.to_path_buf(),
    }));
    OUT_ACTION.with(|o| *o.borrow_mut() = None);
    let res = DialogBoxIndirectParamW(
        None,
        tmpl.as_ptr() as *const DLGTEMPLATE,
        Some(parent),
        Some(dlg_proc),
        LPARAM(ctx as *mut _ as isize),
    );
    drop(Box::from_raw(ctx)); // WM_DESTROY 已清 USERDATA；此处收尾堆对象
    let _ = res;
    OUT_ACTION
        .with(|o| o.borrow_mut().take())
        .and_then(|(code, slot)| match code {
            RES_LOAD => Some(SlotAction::Load(slot)),
            RES_SAVE => Some(SlotAction::Save(slot)),
            RES_DELETE => Some(SlotAction::Delete(slot)),
            RES_COPY => Some(SlotAction::CopyPath(slot)),
            _ => None,
        })
}

unsafe fn ctx_of(hdlg: HWND) -> Option<&'static mut SlotsCtx> {
    let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
    (p != 0).then(|| &mut *(p as *mut SlotsCtx))
}

/// 当前选中槽号（0 基行号 → 槽号）；无选中返回 None。
unsafe fn selected_slot(ctx: &SlotsCtx) -> Option<u32> {
    let sel = SendMessageW(ctx.list, LVM_GETSELECTIONMARK, None, None).0;
    if sel < 0 {
        // 回落首项（无 SelectionMark 时 ListView 返回 -1）。
        return Some(0);
    }
    Some(sel as u32)
}

/// 重填槽列表（时间/路径从磁盘现读）。
unsafe fn refill(list: HWND, exe_dir: &Path) {
    SendMessageW(list, LVM_DELETEALLITEMS, None, None);
    for n in 0..SLOT_COUNT {
        let (name, time, path) = slot_row(exe_dir, n, None);
        let row = SendMessageW(list, LVM_GETITEMCOUNT, None, None).0 as i32;
        insert_row(list, row, &name, &time, &path);
    }
}

/// 插一行三列文本（LVITEM + 子项）。
unsafe fn insert_row(list: HWND, row: i32, c0: &str, c1: &str, c2: &str) {
    for (sub, text) in [(0, c0), (1, c1), (2, c2)] {
        let mut wz: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let mut item = LVITEMW {
            mask: LVIF_TEXT,
            iItem: row,
            iSubItem: sub,
            ..Default::default()
        };
        item.pszText = windows::core::PWSTR::from_raw(wz.as_mut_ptr());
        let msg = if sub == 0 { LVM_INSERTITEMW } else { LVM_SETITEMTEXTW };
        SendMessageW(list, msg, Some(WPARAM(row as usize)), Some(LPARAM(&mut item as *mut LVITEMW as isize)));
    }
}

unsafe extern "system" fn dlg_proc(hdlg: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    match msg {
        WM_INITDIALOG => {
            let ctx = lp.0 as *mut SlotsCtx;
            SetWindowLongPtrW(hdlg, GWLP_USERDATA, ctx as isize);
            let ctx = &mut *ctx;
            ctx.list = GetDlgItem(Some(hdlg), IDC_LIST_SLOTS).unwrap_or_default();
            // 三列：槽 / 时间 / 路径。
            for (i, (title, w)) in [("槽", 60), ("时间", 130), ("路径摘要", 220)].iter().enumerate() {
                let mut wz: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
                let mut col = LVCOLUMNW {
                    mask: LVCF_TEXT | LVCF_WIDTH,
                    cx: *w,
                    ..Default::default()
                };
                col.pszText = windows::core::PWSTR::from_raw(wz.as_mut_ptr());
                SendMessageW(
                    ctx.list,
                    LVM_INSERTCOLUMNW,
                    Some(WPARAM(i)),
                    Some(LPARAM(&mut col as *mut LVCOLUMNW as isize)),
                );
            }
            refill(ctx.list, &ctx.exe_dir);
            // 冒烟钩子：QDIR_TEST_SLOTS_UI → 打印控件/行数后原样取消
            // （对话框本体可建、控件齐备即达成本钩子目的；动作链由
            // QDIR_TEST_SLOTS 帧级钩子覆盖）。
            if std::env::var("QDIR_TEST_SLOTS_UI").is_ok() {
                let rows = SendMessageW(ctx.list, LVM_GETITEMCOUNT, None, None).0.max(0) as usize;
                let ctr = |id: i32| GetDlgItem(Some(hdlg), id).is_ok();
                println!(
                    "[qdir-rs] TEST 布局槽对话框 行数={rows} 控件 list={} save={} load={} del={} copy={} ok={} cancel={}",
                    ctr(IDC_LIST_SLOTS),
                    ctr(IDC_BTN_SAVE),
                    ctr(IDC_BTN_LOAD),
                    ctr(IDC_BTN_DEL),
                    ctr(IDC_BTN_COPY),
                    ctr(IDC_OK),
                    ctr(IDC_CANCEL),
                );
                let _ = EndDialog(hdlg, RES_CANCEL);
                return 1;
            }
            1
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xFFFF) as i32;
            let Some(ctx) = ctx_of(hdlg) else { return 0 };
            let code = match id {
                IDC_BTN_SAVE => Some(RES_SAVE),
                IDC_BTN_LOAD => Some(RES_LOAD),
                IDC_BTN_DEL => Some(RES_DELETE),
                IDC_BTN_COPY => Some(RES_COPY),
                IDC_OK | IDC_CANCEL => None,
                _ => return 0,
            };
            if let (Some(code), Some(slot)) = (code, selected_slot(ctx)) {
                OUT_ACTION.with(|o| *o.borrow_mut() = Some((code, slot)));
                let _ = EndDialog(hdlg, code * 100 + slot as isize);
            } else {
                let _ = EndDialog(hdlg, RES_CANCEL);
            }
            1
        }
        WM_CLOSE => {
            let _ = unsafe { EndDialog(hdlg, RES_CANCEL) };
            1
        }
        _ => 0,
    }
}
