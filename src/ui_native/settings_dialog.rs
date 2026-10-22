//! 设置对话框（P2：tree_*/tab_*/文件操作开关键，选项菜单「设置(&X)…」）。
//!
//! 同 rules_dialog：自建经典 DLGTEMPLATE 经 `DialogBoxIndirectParamW`
//! （R1 零原版资源字节；原版模板字节不加载）。三组：目录树 / 标签页 /
//! 文件操作。核心决策全部走纯函数（[`tree_styles`] / [`tab_tooltips_style`] /
//! [`state_from_config`] / [`apply_state`]），对话框过程只做控件↔状态搬运。
//!
//! 生效范围（诚实划分）：
//! - 目录树样式位 + 缩进：确定后**即时**应用到四窗格树控件（TVM_SETINDENT /
//!   GWL_STYLE），并持久化 ini；
//! - 标签 TCS_TOOLTIPS：即时（GWL_STYLE）；中键/双击关闭开关本就
//!   实时读 config（subclass），无需重建；
//! - 其余键（width_mode、file_ops.*、hover_time 等）：写回 `Config` 并
//!   持久化，渲染端消费*待后续项*。

use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::{Config, TreeConfig};
use crate::ui_native::rules_dialog::{
    ATOM_BUTTON, ATOM_EDIT, ATOM_STATIC, ItemDef, build_template,
};

// 控件 id 段：300..399（避开规则框 100..204）。
const GRP_TREE: i32 = 300;
const IDC_TREE_NOEXPANDO: i32 = 301;
const IDC_TREE_SINGLEEXPAND: i32 = 302;
const IDC_TREE_FULLROW: i32 = 303;
const IDC_TREE_LINEATROOT: i32 = 304;
const IDC_TREE_DISABLELINE: i32 = 305;
const IDC_TREE_INDENT: i32 = 306;
const IDC_TREE_HOVER: i32 = 307;

const GRP_TAB: i32 = 310;
const IDC_TAB_CLOSE_MBT: i32 = 311;
const IDC_TAB_CLOSE_DBL: i32 = 312;
const IDC_TAB_TOOLTIPS: i32 = 313;
const IDC_TAB_MAXW: i32 = 314;
const IDC_TAB_WMODE_FIXED: i32 = 315;
const IDC_TAB_WMODE_AUTO: i32 = 316;

const IDC_STARTUP: i32 = 325;

const GRP_OPS: i32 = 320;
const IDC_OPS_MOVECONF: i32 = 321;
const IDC_OPS_KEEPSEL: i32 = 322;
const IDC_OPS_SELECTLAST: i32 = 323;

const IDC_OK: i32 = IDOK.0;
const IDC_CANCEL: i32 = IDCANCEL.0;

pub const DLG_W: i16 = 320;
pub const DLG_H: i16 = 268;

const RES_CANCEL: isize = 0;
const RES_OK: isize = 1;

/// 对话框承载的扁平状态（全部可编辑值；checkbox=u8 0/1）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SettingsState {
    /// 开机恢复（登录时启动；同时是 HKCU Run 值的期望态）。
    pub startup: bool,
    pub tree_noexpando: bool,
    pub tree_single_expand: bool,
    pub tree_fullrow: bool,
    pub tree_line_at_root: bool,
    pub tree_disable_line: bool,
    pub tree_indent: u32,
    pub tree_hover: u32,
    pub tab_close_mbt: bool,
    pub tab_close_dbl: bool,
    pub tab_tooltips: bool,
    pub tab_max_width: u32,
    pub tab_width_mode: u32,
    pub ops_move_conf: bool,
    pub ops_keep_sel: bool,
    pub ops_select_last: bool,
}

/// config → 对话框状态（纯函数，单测）。
pub fn state_from_config(cfg: &Config) -> SettingsState {
    SettingsState {
        startup: cfg.startup,
        tree_noexpando: cfg.tree.no_expando,
        tree_single_expand: cfg.tree.single_expand,
        tree_fullrow: cfg.tree.fullrowselect,
        tree_line_at_root: cfg.tree.line_at_root,
        tree_disable_line: cfg.tree.disable_line,
        tree_indent: cfg.tree.indent_width,
        tree_hover: cfg.tree.hover_time,
        tab_close_mbt: cfg.tab.close_by_mbt_clk,
        tab_close_dbl: cfg.tab.close_by_lbt_dblclk,
        tab_tooltips: cfg.tab.show_tooltips,
        tab_max_width: cfg.tab.max_width,
        tab_width_mode: cfg.tab.width_mode,
        ops_move_conf: cfg.file_ops.move_file_confirmation,
        ops_keep_sel: cfg.file_ops.refresh_keep_sel,
        ops_select_last: cfg.file_ops.select_last_if_navigate_up,
    }
}

/// 对话框状态 → 写回 config（纯函数，单测；只覆盖对应键）。
pub fn apply_state(cfg: &mut Config, st: &SettingsState) {
    cfg.startup = st.startup;
    cfg.tree.no_expando = st.tree_noexpando;
    cfg.tree.single_expand = st.tree_single_expand;
    cfg.tree.fullrowselect = st.tree_fullrow;
    cfg.tree.line_at_root = st.tree_line_at_root;
    cfg.tree.disable_line = st.tree_disable_line;
    cfg.tree.indent_width = st.tree_indent;
    cfg.tree.hover_time = st.tree_hover;
    cfg.tab.close_by_mbt_clk = st.tab_close_mbt;
    cfg.tab.close_by_lbt_dblclk = st.tab_close_dbl;
    cfg.tab.show_tooltips = st.tab_tooltips;
    cfg.tab.max_width = st.tab_max_width;
    cfg.tab.width_mode = st.tab_width_mode;
    cfg.file_ops.move_file_confirmation = st.ops_move_conf;
    cfg.file_ops.refresh_keep_sel = st.ops_keep_sel;
    cfg.file_ops.select_last_if_navigate_up = st.ops_select_last;
}

/// TreeConfig → 树控件样式位（TVM/GWL_STYLE 用）。固定位与
/// create_pane 现用位一致（SHOWSELALWAYS/CHECKBOXES/EDITLABELS 由我们
/// 常开）；开关位映射：
/// - no_expando=1 → 关 HASBUTTONS（无展开钮）；
/// - disable_line=1 → 关 HASLINES；
/// - line_at_root=0 → 关 LINESATROOT；
/// - single_expand=1 → 开 TVS_SINGLEEXPAND；
/// - fullrowselect=1 → 开 TVS_FULLROWSELECT。
pub fn tree_styles(t: &TreeConfig) -> u32 {
    let base = TVS_SHOWSELALWAYS | TVS_CHECKBOXES | TVS_DISABLEDRAGDROP;
    let mut s = base;
    if !t.disable_line {
        s |= TVS_HASLINES;
    }
    if t.line_at_root {
        s |= TVS_LINESATROOT;
    }
    if !t.no_expando {
        s |= TVS_HASBUTTONS;
    }
    if t.single_expand {
        s |= TVS_SINGLEEXPAND;
    }
    if t.fullrowselect {
        s |= TVS_FULLROWSELECT;
    }
    s
}

/// 标签样式：TCS_TOOLTIPS 位按开关（其余位保留现值由调用方或运算）。
pub fn tab_tooltips_bit(on: bool) -> u32 {
    if on {
        TCS_TOOLTIPS
    } else {
        0
    }
}

/// 数值串解析（越界回落默认；纯函数单测）。
pub fn parse_u32(text: &str, default: u32, max: u32) -> u32 {
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|v| *v <= max)
        .unwrap_or(default)
}

// ============ 控件表与对话框 ============

fn checkbox(text: &'static str, id: i32, x: i16, y: i16, cx: i16) -> ItemDef<'static> {
    ItemDef::button(text, id, x, y, cx, BS_AUTOCHECKBOX as u32)
}

fn group(text: &'static str, x: i16, y: i16, cx: i16, cy: i16) -> ItemDef<'static> {
    ItemDef {
        id: -1,
        atom: Some(ATOM_BUTTON),
        class: "",
        text,
        style: WS_CHILD.0 | WS_VISIBLE.0 | BS_GROUPBOX as u32,
        ex: 0,
        x,
        y,
        cx,
        cy,
    }
}

/// 完整控件表（初态勾选/文本由 open_settings 预置，模板里只写静态默认）。
pub fn items() -> Vec<ItemDef<'static>> {
    let mut v = Vec::new();
    // —— 目录树 ——
    v.push(group("目录树", 7, 5, 300, 88));
    v.push(checkbox("无展开按钮", IDC_TREE_NOEXPANDO, 16, 20, 90));
    v.push(checkbox("单支展开", IDC_TREE_SINGLEEXPAND, 16, 36, 90));
    v.push(checkbox("整行选择", IDC_TREE_FULLROW, 16, 52, 90));
    v.push(checkbox("根节点连线", IDC_TREE_LINEATROOT, 110, 20, 90));
    v.push(checkbox("禁用连线", IDC_TREE_DISABLELINE, 110, 36, 90));
    v.push(ItemDef::static_("缩进(像素)", 200, 22, 60, 0));
    v.push(ItemDef::edit("20", IDC_TREE_INDENT, 200, 34, 40, ES_NUMBER as u32));
    v.push(ItemDef::static_("悬停(ms)", 200, 52, 60, 0));
    v.push(ItemDef::edit("500", IDC_TREE_HOVER, 200, 64, 40, ES_NUMBER as u32));
    // —— 标签页 ——
    v.push(group("标签页", 7, 98, 300, 76));
    v.push(checkbox("中键关闭标签", IDC_TAB_CLOSE_MBT, 16, 114, 110));
    v.push(checkbox("双击关闭标签", IDC_TAB_CLOSE_DBL, 16, 130, 110));
    v.push(checkbox("悬停提示", IDC_TAB_TOOLTIPS, 16, 146, 110));
    v.push(ItemDef::static_("最大宽(像素)", 150, 116, 70, 0));
    v.push(ItemDef::edit("180", IDC_TAB_MAXW, 150, 130, 40, ES_NUMBER as u32));
    v.push(ItemDef::button(
        "固定宽",
        IDC_TAB_WMODE_FIXED,
        200,
        114,
        70,
        BS_AUTORADIOBUTTON as u32 | WS_GROUP.0,
    ));
    v.push(ItemDef::button("自适应宽", IDC_TAB_WMODE_AUTO, 200, 130, 70, BS_AUTORADIOBUTTON as u32));
    // —— 文件操作 ——
    v.push(group("常规与文件操作", 7, 180, 300, 70));
    v.push(checkbox("开机恢复（登录时启动）", IDC_STARTUP, 16, 196, 160));
    v.push(checkbox("移动前确认", IDC_OPS_MOVECONF, 16, 212, 90));
    v.push(checkbox("刷新保持选择", IDC_OPS_KEEPSEL, 110, 212, 100));
    v.push(checkbox("向上选中上次项", IDC_OPS_SELECTLAST, 214, 196, 90));
    // —— 确定/取消 ——
    v.push(ItemDef::button("确定(&O)", IDC_OK, 130, 254, 80, BS_DEFPUSHBUTTON as u32));
    v.push(ItemDef::button("取消(&C)", IDC_CANCEL, 214, 254, 80, 0));
    v
}

struct SetCtx {
    st: SettingsState,
}

thread_local! {
    static OUT_SETTINGS: std::cell::RefCell<Option<SettingsState>> =
        const { std::cell::RefCell::new(None) };
}

/// 打开设置对话框（模态）。确定 → Some(新状态)。
///
/// # Safety
/// 主 UI 线程消息泵上下文。
pub unsafe fn open_settings(parent: HWND, cfg: &Config) -> Option<SettingsState> {
    let st = state_from_config(cfg);
    let tmpl = build_template(&items(), "L-Dir 设置", DLG_W, DLG_H);
    let ctx = Box::into_raw(Box::new(SetCtx { st }));
    OUT_SETTINGS.with(|o| *o.borrow_mut() = None);
    let res = DialogBoxIndirectParamW(
        None,
        tmpl.as_ptr() as *const DLGTEMPLATE,
        Some(parent),
        Some(proc),
        LPARAM(ctx as *mut _ as isize),
    );
    if res == RES_OK {
        OUT_SETTINGS.with(|o| *o.borrow_mut())
    } else {
        // 取消路径：ctx 仍需释放（proc 的 WM_DESTROY 负责）。
        None
    }
}

unsafe fn ctx_of(hdlg: HWND) -> Option<&'static mut SetCtx> {
    let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
    (p != 0).then(|| &mut *(p as *mut SetCtx))
}

fn set_check(hdlg: HWND, id: i32, on: bool) {
    if let Ok(c) = unsafe { GetDlgItem(Some(hdlg), id) } {
        let st = if on { BST_CHECKED } else { BST_UNCHECKED };
        unsafe {
            let _ = SendMessageW(c, BM_SETCHECK, Some(WPARAM(st.0 as usize)), None);
        }
    }
}

fn get_check(hdlg: HWND, id: i32, cur: bool) -> bool {
    match unsafe { GetDlgItem(Some(hdlg), id) } {
        Ok(c) => unsafe { SendMessageW(c, BM_GETCHECK, None, None).0 == BST_CHECKED.0 as isize },
        Err(_) => cur,
    }
}

fn set_edit(hdlg: HWND, id: i32, v: u32) {
    if let Ok(e) = unsafe { GetDlgItem(Some(hdlg), id) } {
        let mut w: Vec<u16> = v.to_string().encode_utf16().collect();
        w.push(0);
        unsafe {
            let _ = SendMessageW(e, WM_SETTEXT, None, Some(LPARAM(w.as_ptr() as isize)));
        }
    }
}

fn get_edit(hdlg: HWND, id: i32, cur: u32, max: u32, default: u32) -> u32 {
    match unsafe { GetDlgItem(Some(hdlg), id) } {
        Ok(e) => {
            let mut buf = [0u16; 16];
            let n = unsafe { GetWindowTextW(e, &mut buf) };
            let end = (n.max(0) as usize).min(buf.len());
            parse_u32(&String::from_utf16_lossy(&buf[..end]), if cur == 0 { default } else { cur }, max)
        }
        Err(_) => cur,
    }
}

unsafe extern "system" fn proc(hdlg: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    match msg {
        WM_INITDIALOG => {
            let ctx = lp.0 as *mut SetCtx;
            SetWindowLongPtrW(hdlg, GWLP_USERDATA, ctx as isize);
            let st = (*ctx).st;
            set_check(hdlg, IDC_TREE_NOEXPANDO, st.tree_noexpando);
            set_check(hdlg, IDC_TREE_SINGLEEXPAND, st.tree_single_expand);
            set_check(hdlg, IDC_TREE_FULLROW, st.tree_fullrow);
            set_check(hdlg, IDC_TREE_LINEATROOT, st.tree_line_at_root);
            set_check(hdlg, IDC_TREE_DISABLELINE, st.tree_disable_line);
            set_edit(hdlg, IDC_TREE_INDENT, st.tree_indent);
            set_edit(hdlg, IDC_TREE_HOVER, st.tree_hover);
            set_check(hdlg, IDC_TAB_CLOSE_MBT, st.tab_close_mbt);
            set_check(hdlg, IDC_TAB_CLOSE_DBL, st.tab_close_dbl);
            set_check(hdlg, IDC_TAB_TOOLTIPS, st.tab_tooltips);
            set_edit(hdlg, IDC_TAB_MAXW, st.tab_max_width);
            set_check(hdlg, IDC_TAB_WMODE_FIXED, st.tab_width_mode == 0);
            set_check(hdlg, IDC_TAB_WMODE_AUTO, st.tab_width_mode == 1);
            set_check(hdlg, IDC_STARTUP, st.startup);
            set_check(hdlg, IDC_OPS_MOVECONF, st.ops_move_conf);
            set_check(hdlg, IDC_OPS_KEEPSEL, st.ops_keep_sel);
            set_check(hdlg, IDC_OPS_SELECTLAST, st.ops_select_last);
            // 冒烟钩子：控件齐备性 + 初态读回（模态外部不可交互，进程内自证）。
            if std::env::var("QDIR_TEST_SETTINGS").is_ok() {
                let ctr = |id: i32| GetDlgItem(Some(hdlg), id).is_ok();
                let ck = |id: i32| get_check(hdlg, id, false);
                println!(
                    "[qdir-rs] TEST 设置对话框 控件齐={} 开机={} 树钮={} 单支={} 整行={} 根线={} 禁线={} 缩进={} 提示={} 中键关={} 双击关={} 移动确认={} 固定宽={}",
                    ctr(IDC_TREE_INDENT)
                        && ctr(IDC_TAB_MAXW)
                        && ctr(IDC_OK)
                        && ctr(IDC_CANCEL),
                    ck(IDC_STARTUP),
                    ck(IDC_TREE_NOEXPANDO),
                    ck(IDC_TREE_SINGLEEXPAND),
                    ck(IDC_TREE_FULLROW),
                    ck(IDC_TREE_LINEATROOT),
                    ck(IDC_TREE_DISABLELINE),
                    get_edit(hdlg, IDC_TREE_INDENT, 0, 200, 20),
                    ck(IDC_TAB_TOOLTIPS),
                    ck(IDC_TAB_CLOSE_MBT),
                    ck(IDC_TAB_CLOSE_DBL),
                    ck(IDC_OPS_MOVECONF),
                    ck(IDC_TAB_WMODE_FIXED),
                );
                // 冒烟自动化：提交当前状态（无法外部驱动模态交互）；
                // =flip 时翻转 tree_no_expando/缩进，验证补写持久化链路。
                let mut st = (*ctx).st;
                if std::env::var("QDIR_TEST_SETTINGS").as_deref() == Ok("flip") {
                    st.tree_noexpando = true;
                    st.tree_indent = 30;
                    st.startup = true;
                }
                OUT_SETTINGS.with(|o| *o.borrow_mut() = Some(st));
                let _ = EndDialog(hdlg, RES_OK);
            }
            1
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xFFFF) as i32;
            match id {
                IDC_OK => {
                    if let Some(ctx) = ctx_of(hdlg) {
                        let mut st = ctx.st;
                        st.startup = get_check(hdlg, IDC_STARTUP, st.startup);
                        st.tree_noexpando = get_check(hdlg, IDC_TREE_NOEXPANDO, st.tree_noexpando);
                        st.tree_single_expand =
                            get_check(hdlg, IDC_TREE_SINGLEEXPAND, st.tree_single_expand);
                        st.tree_fullrow = get_check(hdlg, IDC_TREE_FULLROW, st.tree_fullrow);
                        st.tree_line_at_root =
                            get_check(hdlg, IDC_TREE_LINEATROOT, st.tree_line_at_root);
                        st.tree_disable_line =
                            get_check(hdlg, IDC_TREE_DISABLELINE, st.tree_disable_line);
                        st.tree_indent = get_edit(hdlg, IDC_TREE_INDENT, st.tree_indent, 200, 20);
                        st.tree_hover = get_edit(hdlg, IDC_TREE_HOVER, st.tree_hover, 10_000, 500);
                        st.tab_close_mbt = get_check(hdlg, IDC_TAB_CLOSE_MBT, st.tab_close_mbt);
                        st.tab_close_dbl = get_check(hdlg, IDC_TAB_CLOSE_DBL, st.tab_close_dbl);
                        st.tab_tooltips = get_check(hdlg, IDC_TAB_TOOLTIPS, st.tab_tooltips);
                        st.tab_max_width = get_edit(hdlg, IDC_TAB_MAXW, st.tab_max_width, 2000, 180);
                        st.tab_width_mode = if get_check(hdlg, IDC_TAB_WMODE_AUTO, false) {
                            1
                        } else {
                            0
                        };
                        st.ops_move_conf = get_check(hdlg, IDC_OPS_MOVECONF, st.ops_move_conf);
                        st.ops_keep_sel = get_check(hdlg, IDC_OPS_KEEPSEL, st.ops_keep_sel);
                        st.ops_select_last =
                            get_check(hdlg, IDC_OPS_SELECTLAST, st.ops_select_last);
                        OUT_SETTINGS.with(|o| *o.borrow_mut() = Some(st));
                    }
                    let _ = EndDialog(hdlg, RES_OK);
                    1
                }
                IDC_CANCEL => {
                    let _ = EndDialog(hdlg, RES_CANCEL);
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
            let p = GetWindowLongPtrW(hdlg, GWLP_USERDATA);
            if p != 0 {
                SetWindowLongPtrW(hdlg, GWLP_USERDATA, 0);
                drop(Box::from_raw(p as *mut SetCtx));
            }
            0
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Config;

    fn cfg_with(f: impl FnOnce(&mut Config)) -> Config {
        let mut c = Config::default();
        f(&mut c);
        c
    }

    #[test]
    fn state_roundtrip_config_to_state_to_config() {
        let c = cfg_with(|c| {
            c.tree.no_expando = true;
            c.tree.fullrowselect = true;
            c.tree.indent_width = 24;
            c.tab.close_by_mbt_clk = true;
            c.tab.show_tooltips = false;
            c.tab.width_mode = 1;
            c.file_ops.move_file_confirmation = true;
        });
        let st = state_from_config(&c);
        assert!(st.tree_noexpando && st.tree_fullrow && st.tab_close_mbt);
        assert!(!st.tab_tooltips);
        assert_eq!(st.tree_indent, 24);
        assert_eq!(st.tab_width_mode, 1);
        let mut c2 = Config::default();
        apply_state(&mut c2, &st);
        assert_eq!(state_from_config(&c2), st, "config→state→config→state 幂等");
    }

    #[test]
    fn tree_styles_bit_mapping() {
        let t = TreeConfig {
            no_expando: false,
            single_expand: true,
            fullrowselect: true,
            line_at_root: true,
            disable_line: false,
            ..TreeConfig::default()
        };
        let s = tree_styles(&t);
        assert_ne!(s & TVS_HASBUTTONS, 0);
        assert_ne!(s & TVS_SINGLEEXPAND, 0);
        assert_ne!(s & TVS_FULLROWSELECT, 0);
        assert_ne!(s & TVS_HASLINES, 0);
        assert_ne!(s & TVS_LINESATROOT, 0);
        // no_expando=1 关钮；disable_line=1 关线；line_at_root=0 关根线。
        let t2 = TreeConfig {
            no_expando: true,
            disable_line: true,
            line_at_root: false,
            single_expand: false,
            fullrowselect: false,
            ..TreeConfig::default()
        };
        let s2 = tree_styles(&t2);
        assert_eq!(s2 & TVS_HASBUTTONS, 0);
        assert_eq!(s2 & TVS_HASLINES, 0);
        assert_eq!(s2 & TVS_LINESATROOT, 0);
        assert_eq!(s2 & TVS_SINGLEEXPAND, 0);
        // 常开位保持。
        assert_ne!(s2 & TVS_SHOWSELALWAYS, 0);
        assert_ne!(s2 & TVS_CHECKBOXES, 0);
    }

    #[test]
    fn parse_u32_bounds_and_default() {
        assert_eq!(parse_u32("42", 10, 100), 42);
        assert_eq!(parse_u32(" 7 ", 10, 100), 7);
        assert_eq!(parse_u32("999", 10, 100), 10, "越界回落默认");
        assert_eq!(parse_u32("abc", 10, 100), 10);
        assert_eq!(parse_u32("", 10, 100), 10);
        assert_eq!(parse_u32("-1", 10, 100), 10, "负数不是 u32");
    }

    #[test]
    fn tab_tooltips_bit_mapping() {
        assert_eq!(tab_tooltips_bit(true), TCS_TOOLTIPS);
        assert_eq!(tab_tooltips_bit(false), 0);
    }

    #[test]
    fn template_ids_unique() {
        let its = items();
        let mut ids: Vec<i32> = its.iter().map(|i| i.id).filter(|&id| id != -1).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "对话框控件 id 不得重复");
        // 构建模板字节成功且首项 id 落在预期位置（组框 id=-1 不写控件）。
        let t = build_template(&its, "L-Dir 设置", DLG_W, DLG_H);
        assert!(t.len() > 40);
    }
}
