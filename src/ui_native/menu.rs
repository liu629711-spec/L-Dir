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
    // 查看
    pub const VIEW_ICON: u32 = 30021;
    pub const VIEW_SMALL_ICON: u32 = 30022;
    pub const VIEW_LIST: u32 = 30023;
    pub const VIEW_DETAILS: u32 = 30024;
    pub const VIEW_GRIDLINES: u32 = 30025;
    pub const VIEW_FIT_COL_HEADER: u32 = 30026;
    pub const VIEW_FIT_COL: u32 = 30027;
    pub const VIEW_REFRESH: u32 = 30028;
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

/// 构建主菜单（自绘文案与结构）。调用方持有返回的 HMENU 生命周期。
pub fn build_main_menu() -> windows::core::Result<HMENU> {
    let file = unsafe { CreateMenu() }?;
    // —— 文件 ——
    let m = popup(file, "文件(&F)");
    add(m, "打开状态(&O)…", cmd_id::FILE_OPEN_STATE);
    add(m, "保存当前状态(&S)…", cmd_id::FILE_SAVE_STATE);
    add(m, "保存状态到桌面(&L)", cmd_id::FILE_SAVE_STATE_DESKTOP);
    add(m, "重复窗口(&D)", cmd_id::FILE_DUP_WINDOW);
    let _ = unsafe { AppendMenuW(m, MF_SEPARATOR, 0, PCWSTR::null()) };
    add(m, "命令行(&C)", cmd_id::FILE_CMD);
    add(m, "PowerShell(&P)", cmd_id::FILE_POWERSHELL);
    add(m, "运行(&R)…", cmd_id::FILE_RUN);
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
    fn 构建菜单句柄有效并释放() {
        let m = super::build_main_menu().unwrap();
        assert!(!m.is_invalid());
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyMenu(m) };
    }
}
