//! 高亮/着色规则编辑对话框（PRODUCT_SPEC P2；R1：零原版资源字节）。
//!
//! 原版把规则编辑 UI 编在 57 个 DLGTEMPLATE 资源里（NATIVE_SPEC §资源清单），
//! R1 禁止复用其字节，因此这里**自建经典 DLGTEMPLATE**（手工拼 WORD 流）经
//! `DialogBoxIndirectParamW` 创建——模态、Tab 序、Esc/Enter、DLU 缩放全由
//! 系统提供。布局与交互同 Q-Dir 类（规则列表 + 启停 + 顺序 + 增删 + 改色）。
//!
//! 诚实声明：原版该对话框的控件排布未反编译确证（无对应 handler 证据），
//! 本布局为自设计 *待复核*；但**数据语义**完全复用已确证的 [Filter2] 规则集
//! （core::RuleSet，「第一条命中即生效」）。
//!
//! 主对话框：SysListView32（列=启用/模式/颜色）+ 上移/下移/添加/删除/编辑
//! + 确定/取消；条目编辑框（嵌套模态）：启用复选、模式串、十六进制色
//! + 选色…(ChooseColorW) + 默认 + 确定/取消。

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::SystemServices::SS_LEFT;
use windows::Win32::UI::Controls::Dialogs::{ChooseColorW, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::{FilterRule, Rgb, RulePattern, RuleSet};

// 主对话框控件 id。
const IDC_LIST_RULES: i32 = 100;
const IDC_BTN_UP: i32 = 101;
const IDC_BTN_DOWN: i32 = 102;
const IDC_BTN_ADD: i32 = 103;
const IDC_BTN_DEL: i32 = 104;
const IDC_BTN_EDIT: i32 = 105;
const IDC_OK: i32 = IDOK.0;
const IDC_CANCEL: i32 = IDCANCEL.0;

// 条目编辑框控件 id。
const IDC_EDIT_EN: i32 = 200;
const IDC_EDIT_PAT: i32 = 201;
const IDC_COL_EDIT: i32 = 202;
const IDC_COL_BTN: i32 = 203;
const IDC_COL_DEF: i32 = 204;

// 主对话框 EndDialog 返回码。
const RES_CANCEL: isize = 0;
const RES_OK: isize = 1;
// 条目框返回码。
const ERES_CANCEL: isize = 0;
const ERES_OK: isize = 1;

pub const DLG_W: i16 = 340;
pub const DLG_H: i16 = 225;
pub const ITEM_DLG_W: i16 = 300;
pub const ITEM_DLG_H: i16 = 110;
pub const GUI_FONT_PT: u16 = 9;
pub const GUI_FONT_NAME: &str = "Segoe UI";

// ============ 纯逻辑（单测覆盖） ============

/// ListView 三列显示文本：启用标记 / 模式串 / 颜色十六进制或「默认」。
pub fn rule_display(rule: &FilterRule) -> (String, String, String) {
    (
        if rule.enabled { "√".into() } else { String::new() },
        rule.pattern.as_ini(),
        match rule.color {
            Some(c) => c.to_hex(),
            None => "默认".into(),
        },
    )
}

/// 上移：i 与 i−1 交换。返回新位置；i==0 越界返回 None。
pub fn move_up(rules: &mut Vec<FilterRule>, i: usize) -> Option<usize> {
    if i == 0 || i >= rules.len() {
        return None;
    }
    rules.swap(i, i - 1);
    Some(i - 1)
}

/// 下移：i 与 i+1 交换。返回新位置；末行返回 None。
pub fn move_down(rules: &mut Vec<FilterRule>, i: usize) -> Option<usize> {
    if i + 1 >= rules.len() {
        return None;
    }
    rules.swap(i, i + 1);
    Some(i + 1)
}

/// 删除第 i 条；越界返回 false。
pub fn remove_at(rules: &mut Vec<FilterRule>, i: usize) -> bool {
    if i < rules.len() {
        rules.remove(i);
        true
    } else {
        false
    }
}

/// 新增一条默认规则（index = 现有最大 +1）；返回新行下标。
pub fn add_rule(rules: &mut Vec<FilterRule>) -> usize {
    let next_index = rules.iter().map(|r| r.index).max().map(|m| m + 1).unwrap_or(0);
    rules.push(FilterRule {
        index: next_index,
        pattern: RulePattern::parse_ini("*.new"),
        enabled: true,
        color: None,
        field4: -1,
        field5: 1,
        field6: -1,
    });
    rules.len() - 1
}

/// 提交前把 index 重排为显示顺序（0..n）。Q-Dir 按「第一条命中」生效，
/// 顺序即优先级；index 同时是 ini 键。原版 12 条默认表本就 0..11 连号，
/// 默认往返不触发重写（apply_config_to_doc 语义比较保留原文）。
pub fn renumber(rules: &mut [FilterRule]) {
    for (i, r) in rules.iter_mut().enumerate() {
        r.index = i;
    }
}

/// 编辑结果 → 提交用 RuleSet。
pub fn finalize(mut rules: Vec<FilterRule>) -> RuleSet {
    renumber(&mut rules);
    RuleSet { rules }
}

/// Rgb → COLORREF（0x00BBGGRR，BGR 字节序）。
pub fn colorref(c: Rgb) -> COLORREF {
    COLORREF(((c.2 as u32) << 16) | ((c.1 as u32) << 8) | (c.0 as u32))
}

/// COLORREF → Rgb。
pub fn rgb_of(c: COLORREF) -> Rgb {
    let v = c.0;
    Rgb(v as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8)
}

// ============ DLGTEMPLATE 构建（经典格式，纯字节函数） ============

/// WORD 流构造器（经典 DLGTEMPLATE 逐 WORD 推进，项边界对齐 DWORD）。
#[derive(Default)]
pub struct TemplateBuilder {
    pub w: Vec<u16>,
}

impl TemplateBuilder {
    fn word(&mut self, v: u16) {
        self.w.push(v);
    }
    fn dword(&mut self, v: u32) {
        self.word(v as u16);
        self.word((v >> 16) as u16);
    }
    fn iword(&mut self, v: i16) {
        self.word(v as u16);
    }
    fn align(&mut self) {
        if self.w.len() % 2 == 1 {
            self.word(0);
        }
    }
    /// null 结尾宽串。
    fn wstr(&mut self, s: &str) {
        for u in s.encode_utf16() {
            self.word(u);
        }
        self.word(0);
    }
}

/// 一个控件项（经典 DLGITEMTEMPLATE）。
pub struct ItemDef<'a> {
    pub id: i32,
    /// Some(a)=MAKEINTATOM 原子类；None=字符串类名。
    pub atom: Option<u16>,
    pub class: &'a str,
    pub text: &'a str,
    pub style: u32,
    pub ex: u32,
    pub x: i16,
    pub y: i16,
    pub cx: i16,
    pub cy: i16,
}

const ATOM_BUTTON: u16 = 0x0080;
const ATOM_EDIT: u16 = 0x0081;
const ATOM_STATIC: u16 = 0x0082;

impl<'a> ItemDef<'a> {
    fn button(text: &'a str, id: i32, x: i16, y: i16, cx: i16, style_extra: u32) -> Self {
        ItemDef {
            id,
            atom: Some(ATOM_BUTTON),
            class: "",
            text,
            style: WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | style_extra,
            ex: 0,
            x,
            y,
            cx,
            cy: 14,
        }
    }
    fn static_(text: &'a str, x: i16, y: i16, cx: i16) -> Self {
        ItemDef {
            id: -1,
            atom: Some(ATOM_STATIC),
            class: "",
            text,
            style: WS_CHILD.0 | WS_VISIBLE.0 | SS_LEFT.0,
            ex: 0,
            x,
            y,
            cx,
            cy: 8,
        }
    }
    fn edit(text: &'a str, id: i32, x: i16, y: i16, cx: i16, style_extra: u32) -> Self {
        ItemDef {
            id,
            atom: Some(ATOM_EDIT),
            class: "",
            text,
            style: WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | WS_BORDER.0 | style_extra,
            ex: 0,
            x,
            y,
            cx,
            cy: 12,
        }
    }
}

/// 主对话框控件表。
pub fn main_items() -> Vec<ItemDef<'static>> {
    vec![
        ItemDef {
            id: IDC_LIST_RULES,
            atom: None,
            class: "SysListView32",
            text: "",
            style: WS_CHILD.0 | WS_VISIBLE.0 | WS_TABSTOP.0 | LVS_REPORT | LVS_SHOWSELALWAYS,
            ex: WS_EX_CLIENTEDGE.0,
            x: 7,
            y: 7,
            cx: 252,
            cy: 188,
        },
        ItemDef::button("上移(&U)", IDC_BTN_UP, 268, 7, 60, 0),
        ItemDef::button("下移(&D)", IDC_BTN_DOWN, 268, 24, 60, 0),
        ItemDef::button("添加(&A)", IDC_BTN_ADD, 268, 46, 60, 0),
        ItemDef::button("删除(&R)", IDC_BTN_DEL, 268, 63, 60, 0),
        ItemDef::button("编辑(&E)", IDC_BTN_EDIT, 268, 85, 60, 0),
        ItemDef::button("确定(&O)", IDC_OK, 170, 204, 70, BS_DEFPUSHBUTTON as u32),
        ItemDef::button("取消(&C)", IDC_CANCEL, 248, 204, 70, 0),
    ]
}

/// 条目编辑框控件表。
pub fn item_items<'a>(pattern: &'a str, color_text: &'a str) -> Vec<ItemDef<'a>> {
    vec![
        ItemDef::button("启用此规则", IDC_EDIT_EN, 10, 12, 120, BS_AUTOCHECKBOX as u32),
        ItemDef::static_("模式（通配符串，`;` 分隔）", 10, 34, 200),
        ItemDef::edit(pattern, IDC_EDIT_PAT, 10, 46, 270, ES_AUTOHSCROLL as u32),
        ItemDef::static_("前景色（RRGGBB）", 10, 66, 90),
        ItemDef::edit(color_text, IDC_COL_EDIT, 100, 64, 60, ES_AUTOHSCROLL as u32),
        ItemDef::button("选色…", IDC_COL_BTN, 166, 63, 55, 0),
        ItemDef::button("默认", IDC_COL_DEF, 226, 63, 40, 0),
        ItemDef::button("确定", IDC_OK, 152, 88, 60, BS_DEFPUSHBUTTON as u32),
        ItemDef::button("取消", IDC_CANCEL, 220, 88, 60, 0),
    ]
}

/// 构建经典 DLGTEMPLATE 字节流（纯函数，单测断言结构）。
pub fn build_template(items: &[ItemDef], title: &str, w: i16, h: i16) -> Vec<u16> {
    let style = WS_POPUP.0
        | WS_BORDER.0
        | WS_SYSMENU.0
        | DS_MODALFRAME as u32
        | DS_SETFONT as u32
        | DS_3DLOOK as u32
        | DS_CENTER as u32;
    let mut b = TemplateBuilder::default();
    b.dword(style);
    b.dword(WS_EX_DLGMODALFRAME.0);
    b.word(items.len() as u16);
    b.iword(0);
    b.iword(0);
    b.iword(w);
    b.iword(h);
    b.word(0); // menu：无
    b.word(0); // class：#32770
    b.wstr(title);
    b.word(GUI_FONT_PT); // DS_SETFONT：字号 + 字体名（title 之后）
    b.wstr(GUI_FONT_NAME);
    for it in items {
        b.align();
        b.dword(it.style);
        b.dword(it.ex);
        b.iword(it.x);
        b.iword(it.y);
        b.iword(it.cx);
        b.iword(it.cy);
        b.word(it.id as u16);
        match it.atom {
            Some(a) => {
                b.word(0xFFFF);
                b.word(a);
            }
            None => b.wstr(it.class),
        }
        b.wstr(it.text);
        b.word(0); // 无创建数据
    }
    b.w
}

// ============ 主对话框 ============

/// 编辑会话状态（堆上，经 GWLP_USERDATA 挂载；WM_DESTROY 释放一次）。
struct RulesCtx {
    rules: Vec<FilterRule>,
    list: HWND,
}

thread_local! {
    /// OK 提交暂存（模态单例，够用）。
    static OUT_RULES: std::cell::RefCell<Option<Vec<FilterRule>>> =
        const { std::cell::RefCell::new(None) };
}

/// 编辑 [Filter2] 规则集（模态）。确定 → Some(新规则集)；取消 → None。
///
/// # Safety
/// 仅主 UI 线程消息泵上下文调用（嵌套模态循环）。
pub unsafe fn edit_rules(parent: HWND, rules: &RuleSet) -> Option<RuleSet> {
    let tmpl = build_template(&main_items(), "高亮/着色规则", DLG_W, DLG_H);
    let ctx = Box::into_raw(Box::new(RulesCtx {
        rules: rules.rules.clone(),
        list: HWND::default(),
    }));
    OUT_RULES.with(|o| *o.borrow_mut() = None);
    let result = DialogBoxIndirectParamW(
        None,
        tmpl.as_ptr() as *const DLGTEMPLATE,
        Some(parent),
        Some(main_proc),
        LPARAM(ctx as *mut _ as isize),
    );
    if result == RES_OK {
        OUT_RULES.with(|o| o.borrow_mut().take()).map(finalize)
    } else {
        None
    }
}

unsafe fn ctx_of(hdlg: HWND) -> Option<&'static mut RulesCtx> {
    let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
    (p != 0).then(|| &mut *(p as *mut RulesCtx))
}

unsafe extern "system" fn main_proc(hdlg: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    match msg {
        WM_INITDIALOG => {
            let ctx = lp.0 as *mut RulesCtx;
            SetWindowLongPtrW(hdlg, GWLP_USERDATA, ctx as isize);
            let ctx = &mut *ctx;
            ctx.list = GetDlgItem(Some(hdlg), IDC_LIST_RULES).unwrap_or_default();
            fill_list(ctx.list, &ctx.rules);
            // 冒烟钩子：QDIR_TEST_RULES_UI 时打印控件/行数并自动确定
            // （模态循环外部不可驱动交互，用进程内探针确证对话框可建、
            // 控件齐备、行数=规则数，然后原样提交走完整保存链）。
            if std::env::var("QDIR_TEST_RULES_UI").is_ok() {
                let rows =
                    SendMessageW(ctx.list, LVM_GETITEMCOUNT, None, None).0.max(0) as usize;
                let ctr = |id: i32| GetDlgItem(Some(hdlg), id).is_ok();
                println!(
                    "[qdir-rs] TEST 规则对话框 行数={rows} 规则数={} 控件 list={} up={} down={} add={} del={} edit={} ok={} cancel={}",
                    ctx.rules.len(),
                    ctr(IDC_LIST_RULES),
                    ctr(IDC_BTN_UP),
                    ctr(IDC_BTN_DOWN),
                    ctr(IDC_BTN_ADD),
                    ctr(IDC_BTN_DEL),
                    ctr(IDC_BTN_EDIT),
                    ctr(IDC_OK),
                    ctr(IDC_CANCEL),
                );
                OUT_RULES.with(|o| *o.borrow_mut() = Some(ctx.rules.clone()));
                let _ = EndDialog(hdlg, RES_OK);
                return 1;
            }
            1
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xFFFF) as i32;
            let Some(ctx) = ctx_of(hdlg) else { return 0 };
            match id {
                IDC_OK => {
                    OUT_RULES.with(|o| *o.borrow_mut() = Some(ctx.rules.clone()));
                    let _ = EndDialog(hdlg, RES_OK);
                    1
                }
                IDC_CANCEL => {
                    let _ = EndDialog(hdlg, RES_CANCEL);
                    1
                }
                IDC_BTN_UP | IDC_BTN_DOWN | IDC_BTN_ADD | IDC_BTN_DEL | IDC_BTN_EDIT => {
                    let sel = selected_row(ctx.list);
                    let idx = (sel >= 0).then(|| sel as usize);
                    match id {
                        IDC_BTN_UP => {
                            if let Some(i) = idx {
                                if let Some(ni) = move_up(&mut ctx.rules, i) {
                                    fill_list(ctx.list, &ctx.rules);
                                    select_row(ctx.list, ni);
                                }
                            }
                        }
                        IDC_BTN_DOWN => {
                            if let Some(i) = idx {
                                if let Some(ni) = move_down(&mut ctx.rules, i) {
                                    fill_list(ctx.list, &ctx.rules);
                                    select_row(ctx.list, ni);
                                }
                            }
                        }
                        IDC_BTN_ADD => {
                            let ni = add_rule(&mut ctx.rules);
                            fill_list(ctx.list, &ctx.rules);
                            select_row(ctx.list, ni);
                            edit_rule_at(hdlg, ctx, ni);
                            fill_list(ctx.list, &ctx.rules);
                        }
                        IDC_BTN_DEL => {
                            if let Some(i) = idx {
                                if remove_at(&mut ctx.rules, i) {
                                    fill_list(ctx.list, &ctx.rules);
                                }
                            }
                        }
                        IDC_BTN_EDIT => {
                            if let Some(i) = idx {
                                edit_rule_at(hdlg, ctx, i);
                                fill_list(ctx.list, &ctx.rules);
                            }
                        }
                        _ => {}
                    }
                    1
                }
                _ => 0,
            }
        }
        WM_CLOSE => {
            let _ = EndDialog(hdlg, RES_CANCEL);
            1
        }
        WM_DESTROY => {
            // 释放一次（EndDialog 后系统销毁对话框窗口时触发）。
            let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
            if p != 0 {
                SetWindowLongPtrW(hdlg, GWLP_USERDATA, 0);
                drop(Box::from_raw(p as *mut RulesCtx));
            }
            0
        }
        _ => 0,
    }
}

// ============ 条目编辑框（嵌套模态） ============

struct EditCtx {
    rule: FilterRule,
}

/// 弹出条目编辑（嵌套模态）。确定时把结果写回 ctx.rules[i]。
unsafe fn edit_rule_at(hdlg: HWND, ctx: &mut RulesCtx, i: usize) {
    let rule = ctx.rules[i].clone();
    let color_text = rule.color.map(|c| c.to_hex()).unwrap_or_default();
    let pattern = rule.pattern.as_ini();
    let tmpl = build_template(
        &item_items(&pattern, &color_text),
        "编辑规则",
        ITEM_DLG_W,
        ITEM_DLG_H,
    );
    let ectx = Box::into_raw(Box::new(EditCtx { rule }));
    let res = DialogBoxIndirectParamW(
        None,
        tmpl.as_ptr() as *const DLGTEMPLATE,
        Some(hdlg),
        Some(edit_proc),
        LPARAM(ectx as *mut _ as isize),
    );
    if res == ERES_OK {
        let written = Box::from_raw(ectx);
        ctx.rules[i] = written.rule;
    } else {
        drop(Box::from_raw(ectx));
    }
}

unsafe extern "system" fn edit_proc(hdlg: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    match msg {
        WM_INITDIALOG => {
            let ectx = lp.0 as *mut EditCtx;
            SetWindowLongPtrW(hdlg, GWLP_USERDATA, ectx as isize);
            let ctx = &*ectx;
            // 初始勾选态（模板 style 无法预置复选，建后 BM_SETCHECK）。
            if let Ok(c) = GetDlgItem(Some(hdlg), IDC_EDIT_EN) {
                let st = if ctx.rule.enabled { BST_CHECKED } else { BST_UNCHECKED };
                let _ = SendMessageW(c, BM_SETCHECK, Some(WPARAM(st.0 as usize)), None);
            }
            1
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xFFFF) as i32;
            let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
            if p == 0 {
                return 0;
            }
            let ectx = &mut *(p as *mut EditCtx);
            match id {
                IDC_COL_BTN => {
                    let cur = ectx.rule.color.map(colorref).unwrap_or(COLORREF(0));
                    let mut cc = CHOOSECOLORW::default();
                    cc.lStructSize = core::mem::size_of::<CHOOSECOLORW>() as u32;
                    cc.hwndOwner = hdlg;
                    cc.rgbResult = cur;
                    let mut custom = [COLORREF(0); 16];
                    cc.lpCustColors = custom.as_mut_ptr();
                    cc.Flags = CC_RGBINIT | CC_FULLOPEN;
                    if ChooseColorW(&mut cc).as_bool() {
                        ectx.rule.color = Some(rgb_of(cc.rgbResult));
                        if let Ok(e) = GetDlgItem(Some(hdlg), IDC_COL_EDIT) {
                            let hex = ectx.rule.color.map(|c| c.to_hex()).unwrap_or_default();
                            let mut w: Vec<u16> = hex.encode_utf16().collect();
                            w.push(0);
                            SetWindowTextW(e, PCWSTR(w.as_ptr()));
                        }
                    }
                    1
                }
                IDC_COL_DEF => {
                    ectx.rule.color = None;
                    if let Ok(e) = GetDlgItem(Some(hdlg), IDC_COL_EDIT) {
                        SetWindowTextW(e, PCWSTR::null());
                    }
                    1
                }
                IDC_OK => {
                    // 收集：十六进制串优先（手输），复选与模式串。
                    if let Ok(e) = GetDlgItem(Some(hdlg), IDC_COL_EDIT) {
                        let mut buf = [0u16; 16];
                        let n = GetWindowTextW(e, &mut buf);
                        let end = (n.max(0) as usize).min(buf.len());
                        let text = String::from_utf16_lossy(&buf[..end]);
                        ectx.rule.color = Rgb::parse_hex(text.trim());
                    }
                    if let Ok(c) = GetDlgItem(Some(hdlg), IDC_EDIT_EN) {
                        ectx.rule.enabled =
                            SendMessageW(c, BM_GETCHECK, None, None).0 == BST_CHECKED.0 as isize;
                    }
                    if let Ok(e) = GetDlgItem(Some(hdlg), IDC_EDIT_PAT) {
                        let mut buf = [0u16; 512];
                        let n = GetWindowTextW(e, &mut buf);
                        let end = (n.max(0) as usize).min(buf.len());
                        let text = String::from_utf16_lossy(&buf[..end]);
                        let t = text.trim();
                        if !t.is_empty() {
                            ectx.rule.pattern = RulePattern::parse_ini(t);
                        }
                    }
                    let _ = EndDialog(hdlg, ERES_OK);
                    1
                }
                IDC_CANCEL => {
                    let _ = EndDialog(hdlg, ERES_CANCEL);
                    1
                }
                _ => 0,
            }
        }
        WM_CLOSE => {
            let _ = EndDialog(hdlg, ERES_CANCEL);
            1
        }
        _ => 0,
    }
}

// ============ ListView 小工具 ============

unsafe fn selected_row(list: HWND) -> i32 {
    SendMessageW(list, LVM_GETNEXTITEM, Some(WPARAM(usize::MAX)), Some(LPARAM(LVNI_SELECTED as isize))).0 as i32
}

unsafe fn select_row(list: HWND, i: usize) {
    let mut it = LVITEMW::default();
    it.state = LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0);
    it.stateMask = it.state;
    let _ = SendMessageW(list, LVM_SETITEMSTATE, Some(WPARAM(i)), Some(LPARAM(&it as *const _ as isize)));
    let _ = SendMessageW(list, LVM_ENSUREVISIBLE, Some(WPARAM(i)), Some(LPARAM(1)));
}

/// 全量重灌列表（规则量级小，简单为先）。
unsafe fn fill_list(list: HWND, rules: &[FilterRule]) {
    if list.is_invalid() {
        return;
    }
    let ext = (LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES) as usize;
    let _ = SendMessageW(list, LVM_SETEXTENDEDLISTVIEWSTYLE, Some(WPARAM(0)), Some(LPARAM(ext as isize)));
    let _ = SendMessageW(list, LVM_DELETEALLITEMS, None, None);
    add_column(list, 0, "启用", 44);
    add_column(list, 1, "模式", 150);
    add_column(list, 2, "颜色", 54);
    for (row, r) in rules.iter().enumerate() {
        let (en, pat, col) = rule_display(r);
        let mut w0: Vec<u16> = en.encode_utf16().collect();
        w0.push(0);
        let mut it = LVITEMW::default();
        it.mask = LIST_VIEW_ITEM_FLAGS(LVIF_TEXT.0);
        it.iItem = row as i32;
        it.iSubItem = 0;
        it.pszText = PWSTR(w0.as_mut_ptr());
        let _ = SendMessageW(list, LVM_INSERTITEMW, None, Some(LPARAM(&it as *const _ as isize)));
        set_sub(list, row as i32, 1, &pat);
        set_sub(list, row as i32, 2, &col);
    }
}

unsafe fn add_column(list: HWND, idx: i32, text: &str, width: i32) {
    let mut w: Vec<u16> = text.encode_utf16().collect();
    w.push(0);
    let mut col = LVCOLUMNW::default();
    col.mask = LVCOLUMNW_MASK(LVCF_TEXT.0 | LVCF_WIDTH.0 | LVCF_SUBITEM.0);
    col.cx = width;
    col.iSubItem = idx;
    col.pszText = PWSTR(w.as_mut_ptr());
    let _ = SendMessageW(list, LVM_INSERTCOLUMNW, Some(WPARAM(idx as usize)), Some(LPARAM(&col as *const _ as isize)));
}

unsafe fn set_sub(list: HWND, row: i32, sub: i32, text: &str) {
    let mut w: Vec<u16> = text.encode_utf16().collect();
    w.push(0);
    let mut it = LVITEMW::default();
    it.mask = LIST_VIEW_ITEM_FLAGS(LVIF_TEXT.0);
    it.iItem = row;
    it.iSubItem = sub;
    it.pszText = PWSTR(w.as_mut_ptr());
    let _ = SendMessageW(list, LVM_SETITEMTEXTW, Some(WPARAM(row as usize)), Some(LPARAM(&it as *const _ as isize)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::RuleSet;

    fn sample() -> Vec<FilterRule> {
        RuleSet::parse_ini_section(
            &[
                "0=#Hidden=1=-1=1=1=-1=0",
                "1=*.zip;*.rar;=1=00AA00=-1=1=-1=0",
                "2=#DIR=1=008800=-1=1=-1=0",
                "3=*=1=BB00BB=-1=1=-1=0",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
        )
        .expect("样例规则表")
        .rules
    }

    #[test]
    fn move_up_down_boundaries() {
        let mut rs = sample();
        assert_eq!(move_up(&mut rs, 0), None, "首行不可上移");
        assert_eq!(move_up(&mut rs, 1), Some(0));
        assert_eq!(rs[0].index, 1);
        assert_eq!(rs[1].index, 0);
        let n = rs.len();
        assert_eq!(move_down(&mut rs, n - 1), None, "末行不可下移");
        assert_eq!(move_down(&mut rs, 0), Some(1));
        assert_eq!(remove_at(&mut rs, n), false);
        assert_eq!(remove_at(&mut rs, 0), true);
        assert_eq!(rs.len(), n - 1);
    }

    #[test]
    fn add_rule_indexes_monotonic() {
        let mut rs = sample();
        let i = add_rule(&mut rs);
        assert_eq!(i, rs.len() - 1);
        assert_eq!(rs[i].index, 4);
        assert!(rs[i].enabled);
        assert!(rs[i].color.is_none());
        // 空集从 0 起。
        let mut empty = Vec::new();
        assert_eq!(add_rule(&mut empty), 0);
        assert_eq!(empty[0].index, 0);
    }

    #[test]
    fn renumber_and_finalize_ordering() {
        let mut rs = sample();
        move_up(&mut rs, 3);
        let set = finalize(rs);
        for (i, r) in set.rules.iter().enumerate() {
            assert_eq!(r.index, i, "提交前 index=显示顺序");
        }
        // finalize 不改变集合语义内容。
        let re = finalize(set.rules.clone());
        assert_eq!(set.rules.len(), re.rules.len());
    }

    #[test]
    fn display_maps_fields() {
        let rs = sample();
        let (en, pat, col) = rule_display(&rs[0]); // #Hidden 启用 -1
        assert_eq!(en, "√");
        assert_eq!(pat, "#Hidden");
        assert_eq!(col, "默认");
        let (_, _, col) = rule_display(&rs[1]); // 00AA00
        assert_eq!(col, "00AA00");
    }

    #[test]
    fn colorref_roundtrip_bgr() {
        let c = Rgb(0x12, 0x34, 0x56);
        let cr = colorref(c);
        assert_eq!(cr.0, 0x00563412, "COLORREF 字节序 BB GG RR");
        assert_eq!(rgb_of(cr), c);
    }

    /// 头布局逐字断言：style 占 2 个 WORD（0..2）、ex 2..4、cdit=4、
    /// x=5 y=6 cx=7 cy=8、menu=9、class=10、title 起 11。
    const HDR_END: usize = 11;

    #[test]
    fn template_header_layout() {
        let items = main_items();
        let t = build_template(&items, "T", 100, 50);
        assert_eq!(t[4], items.len() as u16, "cdit");
        // ex=WS_EX_DLGMODALFRAME(1) → t[2]=1,t[3]=0；style 高位含 WS_POPUP(0x8000)。
        assert_eq!((t[2], t[3]), (1, 0), "ex 低位/ex 高位");
        assert_eq!(t[1] & 0x8000, 0x8000, "style 含 WS_POPUP 高位");
        assert_eq!(t[7], 100, "cx");
        assert_eq!(t[8], 50, "cy");
        assert_eq!(t[HDR_END - 2], 0, "menu 无");
        assert_eq!(t[HDR_END - 1], 0, "class 默认");
        assert_eq!(t[HDR_END], 'T' as u16, "title");
        assert_eq!(t[HDR_END + 1], 0, "title NUL");
        assert_eq!(t[HDR_END + 2], GUI_FONT_PT, "DS_SETFONT 字号");
        let face: Vec<u16> = GUI_FONT_NAME.encode_utf16().collect();
        for (i, u) in face.iter().enumerate() {
            assert_eq!(t[HDR_END + 3 + i], *u, "字体名第 {i} 字");
        }
        assert_eq!(t[HDR_END + 3 + face.len()], 0, "字体名 NUL");
    }

    #[test]
    fn template_items_start_dword_aligned() {
        let items = item_items("*.a;b", "AABBCC");
        let t = build_template(&items, "编辑", ITEM_DLG_W, ITEM_DLG_H);
        // 字体名之后第一个控件项从偶数 WORD 索引（DWORD 边界）开始，
        // 且其 id 字段 = style(2)+ex(2)+xyxy(4) 偏移后为 IDC_EDIT_EN。
        let after_font = HDR_END + 1 /*title '编辑'=2字+NUL */
            + 2 /* 编辑 */ ;
        let _ = after_font; // 直接验证 id 而非手算总偏移：
        let mut cursor = HDR_END;
        // 手工推进：title
        while t[cursor] != 0 {
            cursor += 1;
        }
        cursor += 1; // NUL
        cursor += 1; // 字号
        while t[cursor] != 0 {
            cursor += 1;
        }
        cursor += 1; // NUL
        assert_eq!(cursor % 2, 0, "首项必须 DWORD 对齐");
        let id_off = cursor + 2 + 2 + 4;
        assert_eq!(t[id_off], IDC_EDIT_EN as u16, "首项 id");
        // 原子类写法：0xFFFF 前缀 + BUTTON(0x80)。
        assert_eq!(t[id_off + 1], 0xFFFF);
        assert_eq!(t[id_off + 2], ATOM_BUTTON);
    }
}
