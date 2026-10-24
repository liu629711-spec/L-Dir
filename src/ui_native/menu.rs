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
    /// **不得与 ZOOM_BASE(30040) 同段**：历史值 30040 与缩放 30040–30050 撞车，
    /// dispatch 列臂在前吞掉缩放 60–120% 五档（2026-10-24 修复）。
    pub const COL_BASE: u32 = 30110;
    /// 批次D/D2 布局槽对话框（文件菜单）。
    pub const FILE_SLOTS: u32 = 30046;
    /// P1.1 撤消 删除（Ctrl+Z；原版 menu_texts:79 确证存在，id 未确证*待复核*）。
    pub const VIEW_UNDO_DELETE: u32 = 30047;
    /// P2.9 窗口排列（i18n「窗口排列（菜单 216 实测）」四词条；原版入口
    /// 位置未确证*待复核*，挂活动窗口菜单尾部）。
    pub const WIN_CASCADE: u32 = 30048; // 层叠
    pub const WIN_TILE_H: u32 = 30049; // 横向平铺（Nebeneinander，左右列）
    pub const WIN_TILE_V: u32 = 30050; // 纵向平铺（Untereinander，上下行）
    pub const WIN_ARRANGE_ICONS: u32 = 30051; // 排列图标（LVM_ARRANGE）
    /// P2.5 分组依据子菜单基址（+0 无 / +1 扩展名 / +2 类型；
    /// .qdr 视图串 GroupBy/Group 建模早已往返，本项接通 UI 消费）。
    pub const GROUP_BASE: u32 = 30052;
    /// P2（copy_cut_paste_mode 接线）：复制/剪切模式对（互斥勾选；
    /// 原版 0x8568=复制档 / 0x8569=剪切档，AF:84516/84627）。
    pub const CC_MODE_COPY: u32 = 30055;
    pub const CC_MODE_CUT: u32 = 30056;
    /// P2.10 历史下拉弹层基址（TrackPopupMenu 宿主，id = HIST_BASE + 显示序；
    /// 上限 40 条。ZOOM 30250..30261 之后）。
    pub const HIST_BASE: u32 = 30270;
    /// 历史下拉显示上限。
    pub const HIST_MENU_MAX: usize = 40;
    /// P2 排序方式子菜单基址（+0 名称 /+1 扩展名 /+2 大小 /+3 修改日期
    /// /+4 类型 = core::SortKey 序；menu_texts:75「排序方式(O)」确证存在
    /// 于视图菜单）。
    pub const SORT_BASE: u32 = 30065;
    pub const SORT_ASC: u32 = 30070; // 递增（core::SortDir::Asc=0）
    pub const SORT_DESC: u32 = 30071; // 递减
    /// P2 全屏 F11（menu_texts:73「全屏 (F)\tF11」确证存在）。
    pub const VIEW_FULLSCREEN: u32 = 30072;
    /// 本批（2026-10-24）：编辑菜单族 + 数据大小 + 标签重复。
    /// 30073..30079 段空闲（SORT 止于 30071、全屏 30072、收藏夹自 30100）。
    pub const FILE_DUP_TAB: u32 = 30057; // 重复标签页 Ctrl+K（menu_texts:40）
    pub const EDIT_SELECT_ALL: u32 = 30073; // 全选 Ctrl+A（menu_texts:37）
    pub const EDIT_SELECT_INVERT: u32 = 30074; // 反选 Ctrl+Shift+A（menu_texts:38）
    pub const EDIT_OPEN_IN_NEW_TAB: u32 = 30075; // 在新标签页中打开（menu_texts:39）
    pub const EDIT_OPEN_SEL_NEW_TAB: u32 = 30076; // 在新标签页中打开选择（menu_texts:42）
    pub const EDIT_PASTE_SPECIAL: u32 = 30077; // 选择性粘贴 Ctrl+Shift+V（menu_texts:43）
    pub const FILE_SIZE_INFO: u32 = 30078; // 数据大小信息 F9（menu_texts:102）
    /// 标签右键弹层基址（TrackPopupMenu TPM_RETURNCMD 宿主，show_tab_menu
    /// 内处理，不进 dispatch；仍纳入段互斥回归）。**不得落在
    /// HIST_BASE(30270)..30270+40**——初值 30290 与历史下拉段重叠（回归
    /// 测试抓出），顺延到 HIST 段后。
    pub const TABMENU_BASE: u32 = 30310;
    pub const TABMENU_NEW: u32 = TABMENU_BASE; // 新建标签页（menu_texts:41）
    pub const TABMENU_DUP: u32 = TABMENU_BASE + 1; // 重复标签页（menu_texts:40）
    pub const TABMENU_CLOSE: u32 = TABMENU_BASE + 2; // 关闭标签页（推断：菜单文案无取证，标签菜单常规语义）
    pub const TABMENU_CLOSE_OTHERS: u32 = TABMENU_BASE + 3; // 关闭其他标签页（推断同上）
    pub const VIEW_TREE_TOGGLE: u32 = 30029;
    // 树模式三档 + 循环（附录 A.3：原 54001/54002/54003/54006 系）。
    pub const VIEW_TREE_SPLIT: u32 = 30030; // tree_mode=1 双列
    pub const VIEW_TREE_OVERLAY: u32 = 30031; // tree_mode=2 覆盖
    pub const VIEW_TREE_OFF: u32 = 30032; // tree_mode=0 关
    pub const VIEW_TREE_CYCLE: u32 = 30033; // +1 循环
    // 缩放（60/80/100/110/120/130/140/150/160/180/200）。**不得落在
    // 30040–30050**：与列显隐旧位（COL_BASE）、布局槽(30046) 撞车——
    // 点缩放 130% 会先匹配 FILE_SLOTS 臂弹出布局槽对话框（match 先中
    // 先得，测试命令段两两不交 2026-10-24 抓出）。
    pub const ZOOM_BASE: u32 = 30250;
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
    // 布局弹出菜单 12 档（Layout::ALL 序）。**不得落在 30090–30101**：
    // 历史值 30090 与 FAV_ADD(30100)/FAV_MANAGE(30101) 命名空间重叠，
    // 靠 TrackPopupMenu 不走 WM_COMMAND 才未出事（2026-10-24 重编号消歧）。
    pub const LAYOUT_BASE: u32 = 30230;

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
    // P2 audit #45：数据大小信息（menu_texts:102「数据大小信息 [F9]」）。
    add(m, "数据大小信息(&Z)\tF9", cmd_id::FILE_SIZE_INFO);
    let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(m, "退出(&X)", cmd_id::FILE_EXIT);

    // —— 编辑（menu_texts 编辑(E) 段 36–43 文案；Ctrl+A/Ctrl+Shift+A 保持
    // 列表局部键盘（app.rs 注释），菜单文案作提示）——
    let ed = popup(file, "编辑(&E)");
    add(ed, "全选(&L)  Ctrl+A", cmd_id::EDIT_SELECT_ALL);
    add(ed, "反选(&I)  Ctrl+Shift+A", cmd_id::EDIT_SELECT_INVERT);
    let _ = unsafe { AppendMenuW(ed, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(ed, "在新标签页中打开(&T)", cmd_id::EDIT_OPEN_IN_NEW_TAB);
    add(ed, "重复标签页(&D)  Ctrl+K", cmd_id::FILE_DUP_TAB);
    add(ed, "新建标签页(&N)  Ctrl+T", cmd_id::FILE_NEW_TAB);
    add(ed, "在新标签页中打开选择(&S)  Shift+Enter", cmd_id::EDIT_OPEN_SEL_NEW_TAB);
    let _ = unsafe { AppendMenuW(ed, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(ed, "选择性粘贴(&P)  Ctrl+Shift+V", cmd_id::EDIT_PASTE_SPECIAL);

    // —— 查看 ——
    let v = popup(file, "查看(&V)");
    let vm = popup(v, "视图模式(&M)");
    add(vm, "大图标(&B)", cmd_id::VIEW_ICON);
    add(vm, "小图标(&K)", cmd_id::VIEW_SMALL_ICON);
    add(vm, "列表(&L)", cmd_id::VIEW_LIST);
    add(vm, "详细信息(&D)", cmd_id::VIEW_DETAILS);
    add(v, "显示网格线(&G)", cmd_id::VIEW_GRIDLINES);
    add(v, "重命名(&F2)", cmd_id::VIEW_RENAME);
    add(v, "全屏(&F)\tF11", cmd_id::VIEW_FULLSCREEN);
    add(v, "撤消 删除(&U)\tCtrl+Z", cmd_id::VIEW_UNDO_DELETE);
    add(v, "显示隐藏文件(&H)", cmd_id::VIEW_SHOW_HIDDEN);
    // P2：排序方式子菜单（menu_texts:75 确证存在；键序=core::SortKey）。
    {
        let sp = popup(v, "排序方式(&O)");
        add(sp, "名称(&N)", cmd_id::SORT_BASE);
        add(sp, "扩展名(&E)", cmd_id::SORT_BASE + 1);
        add(sp, "大小(&S)", cmd_id::SORT_BASE + 2);
        add(sp, "修改日期(&M)", cmd_id::SORT_BASE + 3);
        add(sp, "类型(&T)", cmd_id::SORT_BASE + 4);
        let _ = unsafe { AppendMenuW(sp, MF_SEPARATOR, 0, PCWSTR::null()) };
        add(sp, "递增(&A)", cmd_id::SORT_ASC);
        add(sp, "递减(&D)", cmd_id::SORT_DESC);
    }
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
    // P2.5：分组依据子菜单（GroupBy,Ext;Group,1; 状态串消费；勾选态
    // 未做 WM_INITMENUPOPUP 刷新，按命令即效）。
    {
        let gp = popup(v, "分组依据(&G)");
        add(gp, "无(&N)", cmd_id::GROUP_BASE);
        add(gp, "按扩展名(&E)", cmd_id::GROUP_BASE + 1);
        add(gp, "按类型(&T)", cmd_id::GROUP_BASE + 2);
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
    // P2.9：窗口排列（层叠/横向平铺/纵向平铺/排列图标）。
    let _ = unsafe { AppendMenuW(aw, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(aw, "层叠(&C)", cmd_id::WIN_CASCADE);
    add(aw, "横向平铺(&H)", cmd_id::WIN_TILE_H);
    add(aw, "纵向平铺(&V)", cmd_id::WIN_TILE_V);
    add(aw, "排列图标(&A)", cmd_id::WIN_ARRANGE_ICONS);

    // —— 收藏夹 ——
    let fv = popup(file, "收藏夹(&A)");
    add(fv, "添加到收藏夹…(&N)", cmd_id::FAV_ADD);
    add(fv, "打开收藏夹文件夹(&O)", cmd_id::FAV_MANAGE);

    // —— 选项 ——
    let op = popup(file, "选项(&O)");
    add(op, "设置(&S)…", cmd_id::OPTIONS_SETTINGS);
    // P2：复制/剪切模式对（原版 0x8568/0x8569 互斥勾选；mode=1 时
    // Ctrl+C/复制按钮以剪切语义写剪贴板，AF:108064 证据）。
    let _ = unsafe { AppendMenuW(op, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(op, "复制模式：Ctrl+C=复制(&C)", cmd_id::CC_MODE_COPY);
    add(op, "剪切模式：Ctrl+C=剪切(&K)", cmd_id::CC_MODE_CUT);

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

/// P2.10：历史下拉弹层（TrackPopupMenu 宿主；id = HIST_BASE + 显示序，
/// 返回值由调用方按 entries 映射回 history 下标）。
pub fn build_history_menu(entries: &[(usize, String)]) -> windows::core::Result<HMENU> {
    let m = unsafe { CreatePopupMenu() }?;
    for (i, (_, label)) in entries.iter().enumerate() {
        let w = s(label);
        let _ = unsafe {
            AppendMenuW(
                m,
                MF_STRING,
                (cmd_id::HIST_BASE + i as u32) as usize,
                PCWSTR(w.as_ptr()),
            )
        };
    }
    Ok(m)
}

/// P2 audit #12：标签右键弹层（文案：新建/重复取 menu_texts 41/40 确证；
/// 关闭族无文案取证，按标签菜单常规语义 *待复核*）。
pub fn build_tab_menu() -> windows::core::Result<HMENU> {
    let m = unsafe { CreatePopupMenu() }?;
    add(m, "新建标签页(&N)  Ctrl+T", cmd_id::TABMENU_NEW);
    add(m, "重复标签页(&D)  Ctrl+K", cmd_id::TABMENU_DUP);
    let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(m, "关闭标签页(&C)", cmd_id::TABMENU_CLOSE);
    add(m, "关闭其他标签页(&O)", cmd_id::TABMENU_CLOSE_OTHERS);
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

    /// 自有命令段两两不交（回归锁：COL_BASE 曾与 ZOOM_BASE 同为 30040，
    /// 缩放 60–120% 五档被列显隐臂吞掉——match 先中先得）。
    #[test]
    fn 命令段两两不交() {
        let ranges: [(u32, u32, &str); 10] = [
            (COL_BASE, COL_BASE + 5, "列显隐"),
            (ZOOM_BASE, ZOOM_BASE + 11, "缩放"),
            (THEME_BASE, THEME_BASE + 5, "配色"),
            (ACTIVE_BASE, ACTIVE_BASE + 4, "活动窗口"),
            (LAYOUT_BASE, LAYOUT_BASE + 12, "布局弹出"),
            (GROUP_BASE, GROUP_BASE + 3, "分组依据"),
            (HIST_BASE, HIST_BASE + 40, "历史下拉"),
            (SORT_BASE, SORT_DESC + 1, "排序方式"),
            (TABMENU_BASE, TABMENU_CLOSE_OTHERS + 1, "标签右键弹层"),
            (0x5A00, 0x5A00 + 1, "QuickLink 弹层"),
        ];
        for i in 0..ranges.len() {
            for j in i + 1..ranges.len() {
                let (a0, a1, na) = ranges[i];
                let (b0, b1, nb) = ranges[j];
                let overlap = a0 < b1 && b0 < a1;
                assert!(!overlap, "段重叠：{na}[{a0},{a1}) × {nb}[{b0},{b1})");
            }
        }
        // 散点单值 id 不得落入任何连续段。
        let singles: [u32; 27] = [
            FILE_OPEN_STATE, FILE_SAVE_STATE, FILE_SAVE_STATE_DESKTOP, FILE_DUP_WINDOW,
            FILE_CMD, FILE_POWERSHELL, FILE_RUN, FILE_EXIT, FILE_EXPORT, FILE_PRINT, FIND,
            FILE_RESTART, FILE_RESTART_ADMIN, FILE_NEW_FOLDER, FILE_NEW_TAB,
            VIEW_ICON, VIEW_SMALL_ICON, VIEW_LIST, VIEW_DETAILS, VIEW_GRIDLINES,
            VIEW_FIT_COL_HEADER, VIEW_FIT_COL, VIEW_REFRESH, VIEW_PREVIEW_TOGGLE,
            RULES_EDIT, VIEW_RENAME, VIEW_SHOW_HIDDEN,
        ];
        let more: [u32; 13] = [
            HIST_BACK, HIST_FORWARD, FILE_SLOTS, VIEW_UNDO_DELETE, VIEW_TREE_TOGGLE,
            VIEW_TREE_SPLIT, VIEW_TREE_OVERLAY, VIEW_TREE_OFF, FAV_ADD,
            WIN_CASCADE, WIN_TILE_H, WIN_TILE_V, WIN_ARRANGE_ICONS,
        ];
        for s in singles.into_iter().chain(more).chain([
            FAV_MANAGE, ABOUT, OPTIONS_SETTINGS, TRAY_SHOW, TRAY_EXIT,
            CC_MODE_COPY, CC_MODE_CUT, VIEW_FULLSCREEN,
            FILE_DUP_TAB, EDIT_SELECT_ALL, EDIT_SELECT_INVERT,
            EDIT_OPEN_IN_NEW_TAB, EDIT_OPEN_SEL_NEW_TAB, EDIT_PASTE_SPECIAL,
            FILE_SIZE_INFO,
        ]) {
            for (r0, r1, name) in ranges {
                assert!(!(s >= r0 && s < r1), "散点 {s} 落入 {name}[{r0},{r1})");
            }
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
