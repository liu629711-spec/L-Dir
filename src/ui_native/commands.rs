//! 菜单命令分发：命令 id（原版菜单模板 128）→ shell 层操作。
//!
//! id 依据 assets/rc/menus.json（原版模板解析基准）。
//! 已核对的常用命令（语言键 → 语义由 id 段与位置推断，后续用 UIA 行为验证）：
//! - 57600=10001(新建文件夹) 57601=10002(复制) 57603=10003(剪切) 32820=10004(粘贴)
//! - 45912=205(删除) 45877=204(打开) 45913=206(属性)
//! - 34037=53011(刷新) 57664=60001(关于)
//!
//! 未接的 id 走 `_ =>` 分支留痕，逐个按原版行为补齐。


use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, DeleteObject, HFONT, HGDIOBJ, LOGFONTW};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::shell;
use crate::ui_native::mainframe::MainFrame;

/// 布局切换命令（导航工具栏，FUN_00429bc4）。
/// 语义映射以原版按钮 tooltip 为准（UIA dump 已给出各按钮文案）。
pub mod layout {
    pub const Z6: u32 = 0xA76C;   // "Q-Dir (默认 4 个窗口)"
    pub const V6: u32 = 0xA76D;   // "2-Dir (2 个水平的)"*待 UIA 复核
    pub const UPF: u32 = 0xA771;  // "3-Dir (1 个在上面 2 个在下面)"*待复核
    pub const NEW: u32 = 0xA772;  // *待复核
    pub const VIEW: u32 = 0xA775; // *待复核
    pub const UPDATE: u32 = 0xA770; // *待复核
    pub const GO: u32 = 0xB379;   // SplitButton：更多布局下拉
    pub const VIEWTREE: u32 = 0xA773; // "1-Dir (一个窗口)"*待复核
}

/// 命令 id 常量（原版菜单模板 128 的 id 字段，16 位）。
pub mod cmd {
    pub const NEW_FOLDER: u32 = 57600; // 10001
    pub const COPY: u32 = 57601; // 10002
    pub const CUT: u32 = 57603; // 10003
    pub const PASTE: u32 = 32820; // 10004
    pub const DELETE: u32 = 45912; // 205
    pub const OPEN: u32 = 45877; // 204
    pub const PROPERTIES: u32 = 45913; // 206
    pub const REFRESH: u32 = 34037; // 53011
    pub const ABOUT: u32 = 57664; // 60001
    pub const POWERSHELL: u32 = 48504; // 文件>PowerShell（菜单字面文本）
}

/// UI 缩放档（视图菜单字面文本 60%..200%，id 45978..45990 顺序核对）。
pub fn scale_for_zoom_id(id: u32) -> Option<u32> {
    const STEPS: [u32; 11] = [60, 80, 100, 110, 120, 130, 140, 150, 160, 180, 200];
    const IDS: [u32; 11] = [45978, 45979, 45980, 45982, 45984, 45985, 45986, 45987, 45988, 45989, 45990];
    IDS.iter().position(|&x| x == id).map(|i| STEPS[i])
}

/// 配色方案档位（视图菜单字面文本：#14F3FF (B)/#000000/#14F3FF/#999999/#matrix，
/// id 34064..34068 顺序核对）。*各档具体配色方案待截图级对照确认*。
pub fn theme_for_id(id: u32) -> Option<u32> {
    const IDS: [u32; 5] = [34064, 34065, 34066, 34067, 34068];
    IDS.iter().position(|&x| x == id).map(|i| i as u32)
}

/// 已实证的命令 id → 原版中文文案（用于「未接命令」日志可读化）。
///
/// 收录标准：**语义已在早前各棒验证过**（实发命令、UIA tooltip 对照、
/// 反编译确认或字面文本菜单项）。未实证的 id 一律不收——原版菜单项
/// 会动态增删，位置对齐不可靠（已逐项 diff 确认），只收硬证据。
pub fn known_label(id: u32) -> Option<&'static str> {
    Some(match id {
        57600 => "新建文件夹",           // 10001，实发验证
        57601 => "复制",                 // 10002，剪贴板验证
        57603 => "剪切",                 // 10003
        32820 => "粘贴",                 // 10004
        45912 => "删除",                 // 205
        45877 => "打开",                 // 204
        45913 => "属性",                 // 206
        34037 => "刷新",                 // 53011
        57664 => "关于",                 // 60001
        48504 => "PowerShell",           // 字面文本菜单项
        45978 => "缩放 60%",
        45979 => "缩放 80%",
        45980 => "缩放 100%",
        45982 => "缩放 110%",
        45984 => "缩放 120%",
        45985 => "缩放 130%",
        45986 => "缩放 140%",
        45987 => "缩放 150%",
        45988 => "缩放 160%",
        45989 => "缩放 180%",
        45990 => "缩放 200%",
        34064 => "颜色和设计 > #14F3FF (B)",
        34065 => "颜色和设计 > #000000",
        34066 => "颜色和设计 > #14F3FF",
        34067 => "颜色和设计 > #999999",
        34068 => "颜色和设计 > #matrix",
        // 德语模板 #216/#217 字面文本硬证据（本棒接线）。
        33931 => "视图 > 大图标",
        33932 => "视图 > 小图标",
        33933 => "视图 > 列表",
        33934 => "视图 > 详细信息",
        32864 => "视图 > 显示网格线",
        33938 => "列宽自适应(含列头)",
        33939 => "列宽自适应",
        32833 => "活动窗口 #1",
        32834 => "活动窗口 #2",
        32835 => "活动窗口 #3",
        32836 => "活动窗口 #4",
        32919 => "添加到收藏夹",
        32779 => "管理收藏夹",
        _ => return None,
    })
}

/// 窗格命令按钮 id 解码：每窗格 10 个按钮，id = 1000 + 100*pane + 20 + bi。
/// 返回 (窗格索引, 按钮序号 0..9)；非按钮 id 返回 None。
///
/// 按钮序号语义（原版 UIA tooltip 顺序，见 mainframe.rs CMD_BUTTONS）：
/// 0 到上一个选中项 / 1 到下一个选中项 / 2 运行 / 3 通过电子邮件发送 /
/// 4 删除 / 5 剪切 / 6 复制 / 7 粘贴 / 8 过滤器或高亮 / 9 快速链接菜单。
/// 注意：原版这些按钮的实际命令 id 从未反编译确认——`1000+100*pane+20+bi`
/// 是本重写版 create_pane 的自定约定（与窗格容器 id=base、其余控件同段），
/// 故按钮序号→操作按 tooltip 字面直译，*待复核*。
/// 与 cmd::*（57601 等）、layout::*（0xA7xx）均无 id 重叠。
pub fn pane_button_id(id: u32) -> Option<(usize, usize)> {
    if !(1000..1400).contains(&id) {
        return None;
    }
    let rem = id - 1000;
    let pane = (rem / 100) as usize;
    let bi = rem % 100;
    if (20..30).contains(&bi) {
        Some((pane, (bi - 20) as usize))
    } else {
        None
    }
}

/// 视图模式菜单 id → LV_VIEW_*（33931 大图标 / 33932 小图标 /
/// 33933 列表 / 33934 详细信息；德语模板 #216 字面文本
/// &Große Symbole / &Kleine Symbole / &Liste / &Details 为硬证据）。
pub fn view_mode_for_id(id: u32) -> Option<u32> {
    // commctrl.h：LV_VIEW_ICON=0 / SMALLICON=2 / LIST=3 / DETAILS=1。
    // 33931/33932 → ICON/SMALLICON 按语义推断，*待复核*；
    // 33933→LIST、33934→DETAILS 字面对应。
    match id {
        33931 => Some(0),
        33932 => Some(2),
        33933 => Some(3),
        33934 => Some(1),
        _ => None,
    }
}

/// 「活动窗口 #N」菜单 id → 窗格索引 0..3（#217 字面文本 `# 1..# 4`）。
pub fn active_pane_for_id(id: u32) -> Option<usize> {
    match id {
        32833 => Some(0),
        32834 => Some(1),
        32835 => Some(2),
        32836 => Some(3),
        _ => None,
    }
}

/// 原版便携布局的相对路径键（[Favoriten] Ordner / Link）按 exe 所在目录
/// 解析；已是绝对路径则原样返回。
pub fn resolve_relative(p: std::path::PathBuf) -> std::path::PathBuf {
    if p.is_absolute() {
        return p;
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join(&p)))
        .unwrap_or(p)
}

/// 分发一条 WM_COMMAND；返回是否已处理。
pub unsafe fn dispatch(frame: &mut MainFrame, main: HWND, id: u32) -> bool {
    let focus = frame.session.focus;
    let pane_idx = (focus.number() - 1) as usize;

    // 活动窗格的选中项（ListView 实时查询，与状态保持无关）。
    let selected: Vec<std::path::PathBuf> = frame.selected_paths(pane_idx);
    let pane_path = frame.pane_path(pane_idx);

    let host = shell::HostWindow::from_raw_hwnd(main.0 as isize);
    match id {
        cmd::COPY => {
            if !selected.is_empty() {
                if let Err(e) = shell::copy_to_clipboard(host, &selected) {
                    eprintln!("[qdir-rs] 复制失败：{e}");
                }
            }
            true
        }
        cmd::CUT => {
            if !selected.is_empty() {
                if let Err(e) = shell::cut_to_clipboard(host, &selected) {
                    eprintln!("[qdir-rs] 剪切失败：{e}");
                }
            }
            true
        }
        cmd::PASTE => {
            if !pane_path.as_os_str().is_empty() {
                if let Err(e) = shell::paste_into(host, &pane_path, false) {
                    eprintln!("[qdir-rs] 粘贴失败：{e}");
                } else {
                    unsafe { frame.fill_pane(pane_idx) };
                }
            }
            true
        }
        cmd::DELETE => {
            if !selected.is_empty() {
                let opts = shell::FileOpOptions::default();
                if let Err(e) = shell::delete_files(host, &selected, &opts) {
                    eprintln!("[qdir-rs] 删除失败：{e}");
                } else {
                    unsafe { frame.fill_pane(pane_idx) };
                }
            }
            true
        }
        cmd::OPEN => {
            if let Some(p) = selected.first() {
                if let Err(e) = shell::open_default(host, p) {
                    eprintln!("[qdir-rs] 打开失败：{e}");
                }
            }
            true
        }
        cmd::PROPERTIES => {
            if let Some(p) = selected.first() {
                if let Err(e) = shell::show_properties(host, p) {
                    eprintln!("[qdir-rs] 属性失败：{e}");
                }
            }
            true
        }
        cmd::NEW_FOLDER => {
            let name = format!("新建文件夹 {}", next_folder_seq(&pane_path));
            match shell::new_folder(host, &pane_path, &name) {
                Ok(_) => unsafe { frame.fill_pane(pane_idx) },
                Err(e) => eprintln!("[qdir-rs] 新建文件夹失败：{e}"),
            }
            true
        }
        cmd::REFRESH => {
            unsafe { frame.fill_pane(pane_idx) };
            true
        }
        cmd::POWERSHELL => {
            // 文件>PowerShell：在活动窗格路径开一个 PowerShell（原版语义）。
            if !pane_path.as_os_str().is_empty() {
                let _ = std::process::Command::new("powershell.exe")
                    .current_dir(&pane_path)
                    .spawn();
            }
            true
        }
        id if scale_for_zoom_id(id).is_some() => {
            if let Some(pct) = scale_for_zoom_id(id) {
                apply_ui_scale(frame, main, pct);
            }
            true
        }
        id if theme_for_id(id).is_some() => {
            let theme = theme_for_id(id).unwrap_or(0);
            frame.config.colorthema = theme;
            frame.session.colorthema = theme;
            println!("[qdir-rs] TEST 配色档位 colorthema={theme}");
            // 重灌全部窗格：行色预算（Filter2）随之按新档重算。
            for i in 0..4 {
                unsafe { frame.fill_pane(i) };
            }
            true
        }
        // 导航布局按钮：反编译（FUN_00429bc4 注册顺序）+ 原版 UIA tooltip
        // 交叉确认了 5 个 id 的语义；其余 3 个（GO/VIEW/UPDATE）按
        // PNG 命名与 tooltip 顺序推断为三/四格类，*待 UI 内点击复核*。
        id if crate::ui_native::splitter::Layout::from_nav_id(id).is_some() => {
            let mode = crate::ui_native::splitter::Layout::from_nav_id(id).unwrap();
            set_layout(frame, main, mode)
        }
        layout::GO | layout::VIEW | layout::UPDATE => {
            // 推断映射：view=4 垂直列、update=4 水平行、go=下拉（这里先
            // 落 3 垂直）。原版外部 PostMessage 不响应，待复核。
            set_layout(
                frame,
                main,
                match id {
                    layout::VIEW => crate::ui_native::splitter::Layout::FourVertical,
                    layout::UPDATE => crate::ui_native::splitter::Layout::FourHorizontal,
                    _ => crate::ui_native::splitter::Layout::ThreeVertical,
                },
            )
        }
        32919 => {
            // 收藏夹「添加到收藏夹…」(语言键 40001)：把活动窗格首个
            // 选中项写成 Favoriten 目录里的 .lnk（原版 ini
            // [Favoriten] Ordner=.\Favoriten\ 布局；文案「添加到收藏夹...」
            // 来自原版 UIA 一级菜单 dump，id 32919 来自模板 #216 字面文本
            // "Zu Favoriten hinzufügen"——双语硬证据）。
            if let Some(p) = selected.first() {
                let name = p
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "收藏夹".to_string());
                let fav_dir = resolve_relative(frame.config.favoriten_dir.clone());
                let _ = std::fs::create_dir_all(&fav_dir);
                let lnk = fav_dir.join(format!("{name}.lnk"));
                match shell::create_shortcut(&lnk, p) {
                    Ok(()) => println!("[qdir-rs] TEST 添加到收藏夹 {lnk:?}"),
                    Err(e) => eprintln!("[qdir-rs] 添加到收藏夹失败：{e}"),
                }
            }
            true
        }
        id if pane_button_id(id).is_some() => {
            // 窗格命令按钮排：作用域 = 按钮所在窗格（不是活动焦点窗格）。
            let (bp, bi) = pane_button_id(id).unwrap();
            if bp >= 4 {
                return false;
            }
            let bsel: Vec<std::path::PathBuf> = frame.selected_paths(bp);
            let bpath = frame.pane_path(bp);
            let host = shell::HostWindow::from_raw_hwnd(main.0 as isize);
            match bi {
                0 => frame.move_selection_to_next(bp, false),
                1 => frame.move_selection_to_next(bp, true),
                2 => {
                    // 运行：对首个选中项走默认打开（shell "open"）。
                    if let Some(p) = bsel.first() {
                        if let Err(e) = shell::open_default(host, p) {
                            eprintln!("[qdir-rs] 运行失败：{e}");
                        }
                    }
                }
                3 => {
                    // 通过电子邮件发送：shell "mail" 动词（MAPI 外壳扩展）。
                    if let Some(p) = bsel.first() {
                        if let Err(e) = shell::send_by_mail(host, p) {
                            eprintln!("[qdir-rs] 邮件发送失败：{e}");
                        }
                    }
                }
                4 => {
                    if !bsel.is_empty() {
                        let opts = shell::FileOpOptions::default();
                        if let Err(e) = shell::delete_files(host, &bsel, &opts) {
                            eprintln!("[qdir-rs] 删除失败：{e}");
                        } else {
                            unsafe { frame.fill_pane(bp) };
                        }
                    }
                }
                5 => {
                    if !bsel.is_empty() {
                        if let Err(e) = shell::cut_to_clipboard(host, &bsel) {
                            eprintln!("[qdir-rs] 剪切失败：{e}");
                        }
                    }
                }
                6 => {
                    if !bsel.is_empty() {
                        if let Err(e) = shell::copy_to_clipboard(host, &bsel) {
                            eprintln!("[qdir-rs] 复制失败：{e}");
                        }
                    }
                }
                7 => {
                    if !bpath.as_os_str().is_empty() {
                        if let Err(e) = shell::paste_into(host, &bpath, false) {
                            eprintln!("[qdir-rs] 粘贴失败：{e}");
                        } else {
                            unsafe { frame.fill_pane(bp) };
                        }
                    }
                }
                8 => frame.focus_filter_edit(bp),
                9 => {
                    // 快速链接菜单：弹 Quick-Link 目录 .lnk 列表，选中
                    // 即导航该窗格（原版 ini Link=.\Favoriten\Quick-Link\）。
                    let ql_dir = resolve_relative(frame.config.quick_link_dir.clone());
                    frame.show_quick_link_menu(main, bp, ql_dir);
                }
                _ => return false,
            }
            println!("[qdir-rs] TEST 命令按钮 窗格{} 序号{bi}", bp + 1);
            true
        }
        id if view_mode_for_id(id).is_some() => {
            // 视图模式（大/小图标、列表、详细信息）。
            let mode = view_mode_for_id(id).unwrap();
            frame.set_view_mode_all(mode);
            println!("[qdir-rs] TEST 视图模式 id={id} view={mode}");
            true
        }
        id if active_pane_for_id(id).is_some() => {
            // 「活动窗口 #N」。
            let p = active_pane_for_id(id).unwrap();
            if frame.set_active_pane(p) {
                println!("[qdir-rs] TEST 活动窗口 #{}", p + 1);
                true
            } else {
                false
            }
        }
        32864 => {
            // &Linien anzeigen：切换网格线。
            frame.toggle_grid_lines_all();
            println!("[qdir-rs] TEST 网格线切换");
            true
        }
        33938 => {
            // Spaltenbreite an Text anpassen (mit Spaltenkopf)。
            frame.autofit_columns_all(true);
            println!("[qdir-rs] TEST 列宽自适应(含列头)");
            true
        }
        33939 => {
            // Spaltenbreite an Text anpassen（不含列头）。
            frame.autofit_columns_all(false);
            println!("[qdir-rs] TEST 列宽自适应");
            true
        }
        _ => false,
    }
}

/// 切换布局模式并强制重排（原版布局按钮行为：不隐藏会话数据，只重排）。
fn set_layout(frame: &mut MainFrame, main: HWND, mode: crate::ui_native::splitter::Layout) -> bool {
    frame.splitters.layout = mode;
    // on_size 里读的是客户区，显式补一次 WM_SIZE（SWP_FRAMECHANGED
    // 配 NOMOVE|NOSIZE 不产生 WM_SIZE）。
    unsafe {
        let _ = SetWindowPos(
            main,
            None,
            0,
            0,
            0,
            0,
            SET_WINDOW_POS_FLAGS(
                SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOZORDER.0 | SWP_FRAMECHANGED.0,
            ),
        );
        SendMessageW(main, WM_SIZE, Some(WPARAM(SIZE_RESTORED as usize)), None);
    }
    true
}

/// 「新建文件夹」编号：数目录里已存在的 `新建文件夹*`，避免重名。
fn next_folder_seq(dir: &std::path::Path) -> usize {
    let mut n = 0;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with("新建文件夹")
            {
                n += 1;
            }
        }
    }
    n + 1
}

// ---------- UI 缩放 ----------

unsafe extern "system" fn setfont_child(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
    // 给所有子控件换字体（原版缩放档行为：文本按档位重绘）。
    // ListView/ToolBar 等对 WM_SETFONT 响应有限，但 Edit/Static/Button 有效。
    let _ = SendMessageW(hwnd, WM_SETFONT, Some(WPARAM(lp.0 as usize)), Some(LPARAM(1)));
    true.into()
}

/// 按百分比重建 UI 字体并广播 WM_SETFONT。
///
/// 基准 = NONCLIENTMETRICS.lfMessageFont（当前进程取系统值即可，
/// 无需跨进程）。*注：原版缩放还会重排工具栏高度，属后续对齐项。
pub fn apply_ui_scale(frame: &mut MainFrame, main: HWND, pct: u32) {
    unsafe {
        let mut ncm = NONCLIENTMETRICSW::default();
        ncm.cbSize = core::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        let base_h = if SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            core::mem::size_of::<NONCLIENTMETRICSW>() as u32,
            Some(&mut ncm as *mut _ as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
            && ncm.lfMessageFont.lfHeight != 0
        {
            ncm.lfMessageFont.lfHeight
        } else {
            -12
        };
        let mut lf = ncm.lfMessageFont;
        lf.lfHeight = (base_h as i64 * pct as i64 / 100) as i32;
        let font = CreateFontIndirectW(&lf);
        if font.is_invalid() {
            eprintln!("[qdir-rs] 缩放字体创建失败 pct={pct}");
            return;
        }
        if !frame.ui_font.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(frame.ui_font.0));
        }
        frame.ui_font = font;
        frame.ui_scale = pct;
        let _ = EnumChildWindows(Some(main), Some(setfont_child), LPARAM(font.0 as isize));
        let _ = SendMessageW(main, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
    }
}

#[cfg(test)]
mod tests {
    use super::pane_button_id;

    #[test]
    fn 解码十个按钮_每窗格() {
        for pane in 0..4usize {
            for bi in 0..10usize {
                let id = (1000 + 100 * pane as u32 + 20 + bi as u32) as u32;
                assert_eq!(pane_button_id(id), Some((pane, bi)), "id={id}");
            }
        }
    }

    #[test]
    fn 非按钮id不误接() {
        // 窗格容器（base）、其它控件槽（base+0..19、30..99）不匹配。
        for id in [1000u32, 1001, 1019, 1030, 1050, 1099, 1399] {
            assert_eq!(pane_button_id(id), None, "id={id}");
        }
        // 段外：菜单命令与布局按钮 id 不受影响。
        for id in [0u32, 999, 1400, 57601, 45912, 0xA76C, 0xB379] {
            assert_eq!(pane_button_id(id), None, "id={id}");
        }
    }

    #[test]
    fn 按钮id与既有命令常量无重叠() {
        use super::{cmd, layout};
        let consts = [
            cmd::NEW_FOLDER, cmd::COPY, cmd::CUT, cmd::PASTE, cmd::DELETE, cmd::OPEN,
            cmd::PROPERTIES, cmd::REFRESH, cmd::ABOUT, cmd::POWERSHELL,
            layout::Z6, layout::V6, layout::UPF, layout::NEW, layout::VIEW,
            layout::UPDATE, layout::GO, layout::VIEWTREE,
        ];
        for id in consts {
            assert_eq!(pane_button_id(id), None, "与按钮段冲突：id={id}");
        }
    }

    #[test]
    fn 视图模式id映射() {
        use super::view_mode_for_id;
        // commctrl.h LV_VIEW_*：ICON=0 DETAILS=1 SMALLICON=2 LIST=3。
        assert_eq!(view_mode_for_id(33931), Some(0));
        assert_eq!(view_mode_for_id(33932), Some(2));
        assert_eq!(view_mode_for_id(33933), Some(3));
        assert_eq!(view_mode_for_id(33934), Some(1));
        for id in [33930u32, 33935, 32864, 32833] {
            assert_eq!(view_mode_for_id(id), None, "id={id}");
        }
    }

    #[test]
    fn 活动窗口id映射() {
        use super::active_pane_for_id;
        for (id, p) in [(32833u32, 0usize), (32834, 1), (32835, 2), (32836, 3)] {
            assert_eq!(active_pane_for_id(id), Some(p), "id={id}");
        }
        for id in [32832u32, 32837, 57601, 0xA76C] {
            assert_eq!(active_pane_for_id(id), None, "id={id}");
        }
    }
}
