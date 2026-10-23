//! 自绘主菜单（PRODUCT_SPEC R1：不使用原版菜单模板字节与文案）。
//!
//! 结构按 §3 P1 功能清单自建；布局体验与 Q-Dir 同类（文件/布局/查看/
//! 收藏夹/帮助），文案为自有中文。命令 id 段：30000–30999（自有约定：
//! 避开窗格控件 1000–1399、Quick-Link 弹层 0x5A00 段、CID 0–99）。

use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::*;

/// 菜单命令 id（dispatch 与 build 共用）。
pub mod cmd_id {
    // 文件
    pub const FILE_OPEN_STATE: u32 = 30001; // 打开状态…
    pub const FILE_SAVE_STATE: u32 = 30002; // 保存当前状态…
    pub const FILE_SAVE_STATE_DESKTOP: u32 = 30003; // 保存状态到桌面
    pub const FILE_DUP_WINDOW: u32 = 30004; // 重复窗口（新实例）
    pub const FILE_CMD: u32 = 30005; // 命令行
    pub const FILE_POWERSHELL: u32 = 30006; // PowerShell
    pub const FILE_RUN: u32 = 30007; // 运行…（系统运行对话框）
    pub const FILE_EXIT: u32 = 30008; // 退出
    // 重启对（附录 A.1 确证：0xBD77 重新启动 / 0xBD75 runas 管理员重启）
    pub const FILE_RESTART: u32 = 30012;
    pub const FILE_RESTART_ADMIN: u32 = 30013;
    pub const FILE_EXPORT: u32 = 30009; // 导出列表…（原 0x8030=32816）
    pub const FILE_PRINT: u32 = 30010; // 打印（原版菜单项确证存在；id 未确证*待复核*）
    pub const FIND: u32 = 30011; // 查找 F3（=聚焦活动窗格过滤框；原版 id 未确证*待复核*）
    // 批次A：新建文件夹（A4）/ 新建标签页（A5，Ctrl+T）
    pub const FILE_NEW_FOLDER: u32 = 30014; // 新建文件夹(&N)
    pub const FILE_NEW_TAB: u32 = 30015; // 新建标签页(Ctrl+T)
    // 查看
    pub const VIEW_ICON: u32 = 30021;
    pub const VIEW_SMALL_ICON: u32 = 30022;
    pub const VIEW_LIST: u32 = 30023;
    pub const VIEW_DETAILS: u32 = 30024;
    pub const VIEW_GRIDLINES: u32 = 30025;
    pub const VIEW_FIT_COL_HEADER: u32 = 30026;
    pub const VIEW_FIT_COL: u32 = 30027;
    pub const VIEW_REFRESH: u32 = 30028;
    pub const VIEW_PREVIEW_TOGGLE: u32 = 30034; // 预览面板 Vorschau 开关
    // 规则编辑（附录 A.1：原版窗格按钮 tooltip「过滤器或高亮」+ 查看系；
    // 原版 id 未反编译确证*待复核*，此处自定 30035）
    pub const RULES_EDIT: u32 = 30035;
    pub const VIEW_RENAME: u32 = 30036; // 批次A/A3：重命名(F2)，走同一条 LVM_EDITLABEL
    pub const VIEW_SHOW_HIDDEN: u32 = 30037; // 批次C/C1：显示隐藏/系统文件开关
    pub const HIST_BACK: u32 = 30038; // 批次C/C4：历史后退（Alt+Left）
    pub const HIST_FORWARD: u32 = 30039; // 批次C/C4：历史前进（Alt+Right）
    /// 批次D/D3 列显隐子菜单基址（COL_BASE+0..4 = Name/Ext/Size/Modified/Attr）。
    pub const COL_BASE: u32 = 30040;
    /// 批次D/D2 布局槽对话框（文件菜单）。
    pub const FILE_SLOTS: u32 = 30046;
    pub const VIEW_TREE_TOGGLE: u32 = 30029;
    // 树模式三档 + 循环（附录 A.3：原 54001/54002/54003/54006 系）。
    pub const VIEW_TREE_SPLIT: u32 = 30030; // tree_mode=1 双列
    pub const VIEW_TREE_OVERLAY: u32 = 30031; // tree_mode=2 覆盖
    pub const VIEW_TREE_OFF: u32 = 30032; // tree_mode=0 关
    pub const VIEW_TREE_CYCLE: u32 = 30033; // +1 循环
    // 缩放（60/80/100/110/120/130/140/150/160/180/200）
    pub const ZOOM_BASE: u32 = 30040;
    // 配色（5 档）
    pub const THEME_BASE: u32 = 30060;
    // 活动窗口 #1..#4
    pub const ACTIVE_BASE: u32 = 30080;
    // 收藏夹
    pub const FAV_ADD: u32 = 30100;
    pub const FAV_MANAGE: u32 = 30101;
    // 选项
    pub const OPTIONS_SETTINGS: u32 = 30210; // 设置…（tree_*/tab_*/文件操作）
    // 托盘右键菜单（自有 id）
    pub const TRAY_SHOW: u32 = 30220;
    pub const TRAY_EXIT: u32 = 30221;
    // 帮助
    pub const ABOUT: u32 = 30200;
    // 布局弹出菜单 12 档（Layout::ALL 序）
    pub const LAYOUT_BASE: u32 = 30090;

    pub const ZOOM_STEPS: [u32; 11] = [60, 80, 100, 110, 120, 130, 140, 150, 160, 180, 200];
}

fn s(t: &str) -> Vec<u16> {
    t.encode_utf16().chain(std::iter::once(0)).collect()
}

fn add(menu: HMENU, text: &str, id: u32) {
    let w = s(text);
    let _ = unsafe { AppendMenuW(menu, MF_STRING, id as usize, PCWSTR(w.as_ptr())) };
}

fn popup(parent: HMENU, text: &str) -> HMENU {
    let m = unsafe { CreatePopupMenu() }.unwrap_or_default();
    let w = s(text);
    let _ = unsafe { AppendMenuW(parent, MF_POPUP, m.0 as usize, PCWSTR(w.as_ptr())) };
    m
}

/// 配色档（自设计，R3；原版菜单字面 `#14F3FF (B)/#000000/#14F3FF/#999999/#matrix`
/// 只确证了档位数量与字面值，渲染语义无字节取证——这里把字面值落为窗格列表底色，
/// (B) 变体按原版按钮观感解读为深色底。`(bg, text)` 均为 RGB；None=系统默认。
/// 档名即 THEMES 顺序（0..4）。
pub fn theme_palette(idx: u32) -> Option<(crate::core::Rgb, Option<crate::core::Rgb>)> {
    use crate::core::Rgb;
    match idx {
        0 => None, // 默认：系统色（CLR_DEFAULT 复位路径在 mainframe）
        1 => Some((Rgb(0x14, 0x14, 0x1B), Some(Rgb(0xE0, 0xE6, 0xE2)))), // #14F3FF (B)：深色底+浅字
        2 => Some((Rgb(0x14, 0xF3, 0xFF), Some(Rgb(0x10, 0x2A, 0x33)))), // #14F3FF：亮青底+深字
        3 => Some((Rgb(0x99, 0x99, 0x99), Some(Rgb(0x1A, 0x1A, 0x1A)))), // #999999：灰底+深字
        4 => Some((Rgb(0x00, 0x18, 0x08), Some(Rgb(0x00, 0xD9, 0x4A)))), // matrix：黑底绿字
        _ => None,
    }
}

/// 托盘右键菜单（还原 / 退出）。
pub fn build_tray_menu() -> windows::core::Result<HMENU> {
    let m = unsafe { CreatePopupMenu() }?;
    add(m, "打开 L-Dir(&O)", cmd_id::TRAY_SHOW);
    let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(m, "退出(&X)", cmd_id::TRAY_EXIT);
    Ok(m)
}

/// 构建主菜单（自绘文案与结构）。调用方持有返回的 HMENU 生命周期。
pub fn build_main_menu() -> windows::core::Result<HMENU> {
    let file = unsafe { CreateMenu() }?;
    // —— 文件 ——
    let m = popup(file, "文件(&F)");
    add(m, "打开状态(&O)…", cmd_id::FILE_OPEN_STATE);
    add(m, "保存当前状态(&S)…", cmd_id::FILE_SAVE_STATE);
    add(m, "保存状态到桌面(&L)", cmd_id::FILE_SAVE_STATE_DESKTOP);
    add(m, "重复窗口(&D)", cmd_id::FILE_DUP_WINDOW);
    add(m, "布局槽(&L)…", cmd_id::FILE_SLOTS);
    let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(m, "新建文件夹(&N)", cmd_id::FILE_NEW_FOLDER);
    add(m, "新建标签页(Ctrl+T)", cmd_id::FILE_NEW_TAB);
    let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(m, "命令行(&C)", cmd_id::FILE_CMD);
    add(m, "PowerShell(&P)", cmd_id::FILE_POWERSHELL);
    add(m, "运行(&R)…", cmd_id::FILE_RUN);
    add(m, "导出列表(&E)…", cmd_id::FILE_EXPORT);
    add(m, "重新启动(&T)", cmd_id::FILE_RESTART);
    add(m, "以管理员身份重新启动(&G)", cmd_id::FILE_RESTART_ADMIN);
    add(m, "打印(&I)", cmd_id::FILE_PRINT);
    add(m, "查找(&F)", cmd_id::FIND);
    let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(m, "退出(&X)", cmd_id::FILE_EXIT);

    // —— 查看 ——
    let v = popup(file, "查看(&V)");
    let vm = popup(v, "视图模式(&M)");
    add(vm, "大图标(&B)", cmd_id::VIEW_ICON);
    add(vm, "小图标(&K)", cmd_id::VIEW_SMALL_ICON);
    add(vm, "列表(&L)", cmd_id::VIEW_LIST);
    add(vm, "详细信息(&D)", cmd_id::VIEW_DETAILS);
    add(v, "显示网格线(&G)", cmd_id::VIEW_GRIDLINES);
    add(v, "重命名(&F2)", cmd_id::VIEW_RENAME);
    add(v, "显示隐藏文件(&H)", cmd_id::VIEW_SHOW_HIDDEN);
    add(v, "后退(&B)\tAlt+Left", cmd_id::HIST_BACK);
    add(v, "前进(&F)\tAlt+Right", cmd_id::HIST_FORWARD);
    add(v, "新建标签页(&E)\tCtrl+T", cmd_id::FILE_NEW_TAB);
    // 批次D/D3：列显隐子菜单（勾选态在 WM_INITMENUPOPUP 按活动窗格刷新）。
    {
        let cols = popup(v, "列(&C)");
        let items: [(&str, &str); 5] = [
            ("名称(&N)", "Name"),
            ("扩展名(&X)", "Ext"),
            ("大小(&S)", "Size"),
            ("修改日期(&M)", "Modified"),
            ("属性(&A)", "Attr"),
        ];
        for (i, (label, key)) in items.iter().enumerate() {
            let w = s(label);
            let _ = unsafe {
                AppendMenuW(
                    cols,
                    MF_STRING,
                    (cmd_id::COL_BASE + i as u32) as usize,
                    PCWSTR(w.as_ptr()),
                )
            };
            let _ = key; // key 由 COLUMN_KEYS 按序对应（menu 顺序=COLUMN_KEYS 顺序）
        }
    }
    add(v, "预览面板(&P)", cmd_id::VIEW_PREVIEW_TOGGLE);
    add(v, "高亮/着色规则(&K)…", cmd_id::RULES_EDIT);
    let _ = unsafe { AppendMenuW(v, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(v, "列宽自适应（含列头）(&W)", cmd_id::VIEW_FIT_COL_HEADER);
    add(v, "列宽自适应(&H)", cmd_id::VIEW_FIT_COL);
    add(v, "目录树显隐(&T)", cmd_id::VIEW_TREE_TOGGLE);
    add(v, "树：双列(&1)", cmd_id::VIEW_TREE_SPLIT);
    add(v, "树：覆盖(&2)", cmd_id::VIEW_TREE_OVERLAY);
    add(v, "树：关闭(&0)", cmd_id::VIEW_TREE_OFF);
    add(v, "树：模式循环", cmd_id::VIEW_TREE_CYCLE);
    let z = popup(v, "缩放(&Z)");
    for (i, pct) in cmd_id::ZOOM_STEPS.iter().enumerate() {
        add(z, &format!("{}%(&{})", pct, (b'A' + i as u8) as char), cmd_id::ZOOM_BASE + i as u32);
    }
    let th = popup(v, "配色(&C)");
    const THEMES: [&str; 5] = ["默认", "深色", "浅蓝", "灰色", "矩阵"];
    for (i, name) in THEMES.iter().enumerate() {
        add(th, name, cmd_id::THEME_BASE + i as u32);
    }
    let _ = unsafe { AppendMenuW(v, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(v, "刷新(&F5)", cmd_id::VIEW_REFRESH);

    // —— 活动窗口 ——
    let aw = popup(file, "活动窗口(&W)");
    for i in 0..4u32 {
        add(aw, &format!("窗口 &{}", i + 1), cmd_id::ACTIVE_BASE + i);
    }

    // —— 收藏夹 ——
    let fv = popup(file, "收藏夹(&A)");
    add(fv, "添加到收藏夹…(&N)", cmd_id::FAV_ADD);
    add(fv, "打开收藏夹文件夹(&O)", cmd_id::FAV_MANAGE);

    // —— 选项 ——
    let op = popup(file, "选项(&O)");
    add(op, "设置(&S)…", cmd_id::OPTIONS_SETTINGS);

    // —— 帮助 ——
    let hp = popup(file, "帮助(&H)");
    add(hp, "关于 L-Dir(&A)", cmd_id::ABOUT);
    Ok(file)
}

/// id → 菜单文案（未接命令日志用；从活的 HMENU 反查，单一事实源）。
pub fn label_of(menu: HMENU, id: u32) -> Option<String> {
    unsafe {
        let mut buf = [0u16; 64];
        let n = GetMenuStringW(menu, id, Some(&mut buf), MF_BYCOMMAND);
        if n > 0 {
            // 去掉助记符 &
            Some(String::from_utf16_lossy(&buf[..n as usize]).replace('&', ""))
        } else {
            None
        }
    }
}

/// 构建 12 档布局弹出菜单（TrackPopupMenu 宿主；选中 id=30090+idx）。
pub fn build_layout_menu() -> windows::core::Result<HMENU> {
    use crate::ui_native::splitter::Layout;
    let m = unsafe { CreatePopupMenu() }?;
    for (i, l) in Layout::ALL.iter().enumerate() {
        add(m, l.label(), cmd_id::LAYOUT_BASE + i as u32);
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::cmd_id::*;

    #[test]
    fn id_段与既有控件段无冲突() {
        // 窗格控件 1000..1399、QuickLink 弹层 0x5A00..、CID 0..99。
        for id in (ZOOM_BASE..ZOOM_BASE + 11).chain(THEME_BASE..THEME_BASE + 5)
            .chain(ACTIVE_BASE..ACTIVE_BASE + 4)
            .chain(FILE_OPEN_STATE..ABOUT + 1)
        {
            assert!(!(1000..=1399).contains(&id), "与窗格控件冲突：{id}");
            assert!(id != 0x5A00, "与 QuickLink 段冲突");
            assert!(id >= 30000, "应全部落在自有 30000+ 段：{id}");
        }
    }

    #[test]
    fn 配色档自设计表() {
        use crate::core::Rgb;
        assert!(super::theme_palette(0).is_none(), "默认=系统色");
        assert_eq!(super::theme_palette(1).unwrap().0, Rgb(0x14, 0x14, 0x1B));
        assert_eq!(super::theme_palette(2).unwrap().0, Rgb(0x14, 0xF3, 0xFF), "菜单字面 #14F3FF");
        assert_eq!(super::theme_palette(3).unwrap().0, Rgb(0x99, 0x99, 0x99), "菜单字面 #999999");
        assert_eq!(super::theme_palette(4).unwrap().1, Some(Rgb(0x00, 0xD9, 0x4A)), "matrix 绿字");
        assert!(super::theme_palette(7).is_none(), "野值=默认");
    }

    #[test]
    fn 构建菜单句柄有效并释放() {
        let m = super::build_main_menu().unwrap();
        assert!(!m.is_invalid());
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyMenu(m) };
    }
}
