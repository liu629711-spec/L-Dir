//! 菜单命令分发：自绘菜单命令 id（`menu::cmd_id`，30000+ 段）→ 操作。
//!
//! 历史注记：1:1 时代曾按原版模板 128 的 id（57600 等）分发，且多项语义
//! 靠推断（后来反编译证伪：57600=重复窗口、57601=打开状态、57603=保存状态、
//! 32820=保存状态到桌面——**都不是**剪贴板命令；剪贴板走右键菜单，见
//! NATIVE_SPEC 附录 A）。转向后菜单由 `menu.rs` 自绘，id 为自有常量；
//! 行为语义仍对标原版确证逻辑（附录 A 是功能来源）。

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, DeleteObject, HFONT, HGDIOBJ, LOGFONTW};
use windows::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, GetSaveFileNameW, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW, OFN_FILEMUSTEXIST};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::brand;
use crate::core::TreeMode;
use crate::shell;
use crate::ui_native::mainframe::MainFrame;
use crate::ui_native::menu::cmd_id;
use crate::ui_native::splitter::Layout;

/// 布局切换命令（导航工具栏，FUN_00429bc4）。
/// 语义按反编译确证（附录 A.4）：z6/v6 = 布局索引 −1/+1 循环；
/// update = 重绘刷新；viewtree = 树显隐；view/go = 转发窗格内容。
/// 当前实现先保留具体布局档位映射（自定 id 仍走此表）。
pub mod layout {
    pub const Z6: u32 = 0xA76C;   // 上一档布局（原版确证：索引 −1）
    pub const V6: u32 = 0xA76D;   // 下一档布局（原版确证：索引 +1）
    pub const UPF: u32 = 0xA771;  // 弹出布局列表（原版确证）
    pub const NEW: u32 = 0xA772;  // 新窗类（FUN_00481604）*待复核*
    pub const VIEW: u32 = 0xA775; // 转发窗格内容 0xd1f
    pub const UPDATE: u32 = 0xA770; // 刷新重绘
    pub const GO: u32 = 0xB379;   // 转发窗格内容 0xd20（下拉）
    pub const VIEWTREE: u32 = 0xA773; // 树显隐切换（原版确证：非 1-Dir！）
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
        // —— 文件：状态（附录 A.1 确证语义；自绘 id）——
        cmd_id::FILE_OPEN_STATE => {
            if let Some(path) = run_state_dialog(main, false) {
                match crate::config::load_session(&path) {
                    Ok(st) => {
                        apply_session(frame, st);
                        println!("[qdir-rs] TEST 状态载入 {path:?}");
                    }
                    Err(e) => eprintln!("[qdir-rs] 状态载入失败：{e}"),
                }
            }
            true
        }
        cmd_id::FILE_SAVE_STATE => {
            if let Some(path) = run_state_dialog(main, true) {
                match crate::config::save_session(&path, &frame.session) {
                    Ok(()) => println!("[qdir-rs] TEST 状态保存 {path:?}"),
                    Err(e) => eprintln!("[qdir-rs] 状态保存失败：{e}"),
                }
            }
            true
        }
        cmd_id::FILE_SAVE_STATE_DESKTOP => {
            let dir = shell::desktop_path().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let path = dir.join(brand::SESSION_FILE);
            match crate::config::save_session(&path, &frame.session) {
                Ok(()) => println!("[qdir-rs] TEST 状态存桌面 {path:?}"),
                Err(e) => eprintln!("[qdir-rs] 状态存桌面失败：{e}"),
            }
            true
        }
        cmd_id::FILE_DUP_WINDOW => {
            // 重复窗口（附录 A.1：原 E100=存临时状态 + 新实例加载）。
            let tmp = std::env::temp_dir().join("L-Dir.clone.qdr");
            match crate::config::save_session(&tmp, &frame.session)
                .and_then(|_| duplicate_window(&tmp))
            {
                Ok(()) => println!("[qdir-rs] TEST 重复窗口 -> {tmp:?}"),
                Err(e) => eprintln!("[qdir-rs] 重复窗口失败：{e}"),
            }
            true
        }
        // —— 文件：工具（附录 A.2：cmd/powershell=窗格路径起终端；
        // 45912/45913 原语义=FileRun/ShutdownWindows 系统对话框）——
        cmd_id::FILE_CMD | cmd_id::FILE_POWERSHELL => {
            // 原版 FUN_004cc524：cmd.exe / powershell.exe，工作目录=窗格路径。
            if !pane_path.as_os_str().is_empty() {
                let exe = if id == cmd_id::FILE_CMD { "cmd.exe" } else { "powershell.exe" };
                if let Err(e) = shell::spawn_terminal(exe, &pane_path) {
                    eprintln!("[qdir-rs] {exe} 启动失败：{e}");
                }
            }
            true
        }
        cmd_id::FILE_RUN => {
            // IShellDispatch::FileRun（附录 A.2 确证槽 0x80）。
            match shell::shell_file_run() {
                Ok(()) => {}
                Err(e) => eprintln!("[qdir-rs] 运行对话框失败：{e}"),
            }
            true
        }
        cmd_id::FILE_EXPORT => {
            // 导出列表（附录 A.1：原 0x8030=收集活动窗格列表→TXT/CSV/HTML/XLS）。
            // 导出整个活动窗格当前列表（原版行为：非选中项，是全表）。
            // 钩子：QDIR_TEST_EXPORT=<out.csv> 时跳过对话框直接导出（冒烟安全）。
            let path = match std::env::var("QDIR_TEST_EXPORT") {
                Ok(p) if !p.trim().is_empty() => std::path::PathBuf::from(p.trim()),
                _ => {
                    let Some(p) = run_export_dialog(main) else { return true };
                    p
                }
            };
            let fmt = crate::services::export::format_from_ext(
                path.extension().and_then(|e| e.to_str()).unwrap_or("txt"),
            )
            .unwrap_or(crate::core::ExportFormat::Txt);
            let pane = &frame.session.panes[pane_idx];
            let tab = match pane.tabs.get(pane.tabs_pos) {
                Some(t) => t,
                None => return true,
            };
            let opts = crate::shell::EnumOptions {
                include_hidden: true,
                want_icon_index: false,
                want_display_info: false,
            };
            match crate::shell::list_directory(&tab.path, &opts) {
                Ok(mut items) => {
                    crate::core::sort_items(&mut items, tab.sort_key, tab.sort_dir);
                    let eo = crate::services::export::ExportOptions::default();
                    match crate::services::export::write_export(&path, &items, fmt, &eo) {
                        Ok(()) => println!(
                            "[qdir-rs] TEST 导出 {path:?} fmt={fmt:?} {} 项",
                            items.len()
                        ),
                        Err(e) => eprintln!("[qdir-rs] 导出失败：{e}"),
                    }
                }
                Err(e) => eprintln!("[qdir-rs] 导出读取目录失败：{e}"),
            }
            true
        }
        cmd_id::FILE_PRINT => {
            // 打印选中文件（shell `print` 动词）。原版命令 id 未反编译确证，
            // 行为按 Explorer 右键打印惯例 *待复核*。无选中项时只留痕。
            if selected.is_empty() {
                println!("[qdir-rs] TEST 打印：无选中项");
            } else {
                let n = shell::print_paths(host, &selected);
                println!("[qdir-rs] TEST 打印投递 {n}/{} 项", selected.len());
            }
            true
        }
        cmd_id::FIND => {
            // 查找（F3）= 聚焦活动窗格过滤框（键入即快速筛名）。
            frame.focus_filter_edit(pane_idx);
            println!("[qdir-rs] TEST 查找：聚焦窗格{}过滤框", pane_idx + 1);
            true
        }
        cmd_id::FILE_EXIT => {
            let _ = PostMessageW(Some(main), WM_CLOSE, WPARAM(0), LPARAM(0));
            true
        }
        cmd_id::VIEW_REFRESH => {
            unsafe { frame.fill_pane(pane_idx) };
            true
        }
        cmd_id::ABOUT => {
            crate::ui_native::mainframe::show_about_brand(main);
            true
        }
        cmd_id::FAV_ADD => {
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
        cmd_id::FAV_MANAGE => {
            let fav_dir = resolve_relative(frame.config.favoriten_dir.clone());
            let _ = std::fs::create_dir_all(&fav_dir);
            let _ = shell::open_default(host, &fav_dir);
            true
        }
        cmd_id::VIEW_TREE_SPLIT => {
            frame.set_tree_mode(main, TreeMode::Split);
            true
        }
        cmd_id::VIEW_TREE_OVERLAY => {
            frame.set_tree_mode(main, TreeMode::Overlay);
            true
        }
        cmd_id::VIEW_TREE_OFF => {
            frame.set_tree_mode(main, TreeMode::Off);
            true
        }
        cmd_id::VIEW_TREE_CYCLE => {
            frame.set_tree_mode(main, frame.session.tree_mode.cycled());
            true
        }
        cmd_id::VIEW_TREE_TOGGLE => {
            // 树显隐（附录 A.3 确证：开关切换 0<->1，原 0x806c/0x806b）。
            frame.set_tree_mode(main, frame.session.tree_mode.toggled());
            true
        }
        // —— 导航工具栏按钮（附录 A.4 确证语义：z6/v6=布局索引循环、
        // upf=布局弹出菜单、go=下拉弹出、update=刷新重绘、viewtree=树切换、
        // new/view 保留 *待复核* 现状）——
        layout::Z6 => {
            // 上一档布局（索引 −1 回绕）。
            let idx = Layout::ALL.iter().position(|l| *l == frame.splitters.layout).unwrap_or(0);
            let next = Layout::ALL[(idx + Layout::ALL.len() - 1) % Layout::ALL.len()];
            set_layout(frame, main, next);
            println!("[qdir-rs] TEST 布局循环- -> {}", next.label());
            true
        }
        layout::V6 => {
            let idx = Layout::ALL.iter().position(|l| *l == frame.splitters.layout).unwrap_or(0);
            let next = Layout::ALL[(idx + 1) % Layout::ALL.len()];
            set_layout(frame, main, next);
            println!("[qdir-rs] TEST 布局循环+ -> {}", next.label());
            true
        }
        layout::UPF => {
            // 弹出 12 档布局菜单（原版 TrackPopupMenu 语义）。
            show_layout_menu(frame, main);
            true
        }
        layout::GO => {
            show_layout_menu(frame, main);
            true
        }
        layout::UPDATE => {
            // 刷新重绘（原版 FUN_00465054 近似：全部窗格重灌）。
            for i in 0..4 {
                unsafe { frame.fill_pane(i) };
            }
            println!("[qdir-rs] TEST 工具栏刷新");
            true
        }
        layout::VIEW => {
            // 原版转发给窗格内容窗 0xd1f；当前内容窗无该私有消息处理，留痕。
            println!("[qdir-rs] TEST 工具栏 view（0xd1f 语义待内容窗实现）");
            true
        }
        layout::VIEWTREE => {
            // viewtree 按钮 = 树开/关切换（附录 A.3 确证 0x806c/0x806b，非布局按钮）。
            frame.set_tree_mode(main, frame.session.tree_mode.toggled());
            true
        }
                id if (cmd_id::VIEW_ICON..=cmd_id::VIEW_DETAILS).contains(&id) => {
            // 视图模式：LV_VIEW_ICON=0 / DETAILS=1 / SMALLICON=2 / LIST=3。
            let mode = match id {
                cmd_id::VIEW_ICON => 0,
                cmd_id::VIEW_SMALL_ICON => 2,
                cmd_id::VIEW_LIST => 3,
                _ => 1,
            };
            frame.set_view_mode_all(mode);
            println!("[qdir-rs] TEST 视图模式 id={id} view={mode}");
            true
        }
        cmd_id::VIEW_GRIDLINES => {
            frame.toggle_grid_lines_all();
            println!("[qdir-rs] TEST 网格线切换");
            true
        }
        cmd_id::VIEW_FIT_COL_HEADER => {
            frame.autofit_columns_all(true);
            println!("[qdir-rs] TEST 列宽自适应(含列头)");
            true
        }
        cmd_id::VIEW_FIT_COL => {
            frame.autofit_columns_all(false);
            println!("[qdir-rs] TEST 列宽自适应");
            true
        }
        id if (cmd_id::LAYOUT_BASE..cmd_id::LAYOUT_BASE + 12).contains(&id) => {
            let idx = (id - cmd_id::LAYOUT_BASE) as usize;
            if let Some(&l) = Layout::ALL.get(idx) {
                set_layout(frame, main, l);
                println!("[qdir-rs] TEST 布局选择 {} -> {}", id, l.label());
            }
            true
        }
                id if (cmd_id::ZOOM_BASE..cmd_id::ZOOM_BASE + 11).contains(&id) => {
            let pct = cmd_id::ZOOM_STEPS[(id - cmd_id::ZOOM_BASE) as usize];
            apply_ui_scale(frame, main, pct);
            true
        }
        id if (cmd_id::THEME_BASE..cmd_id::THEME_BASE + 5).contains(&id) => {
            let theme = id - cmd_id::THEME_BASE;
            frame.config.colorthema = theme;
            frame.session.colorthema = theme;
            frame.apply_theme_palette(main, theme);
            true
        }
        id if (cmd_id::ACTIVE_BASE..cmd_id::ACTIVE_BASE + 4).contains(&id) => {
            let p = (id - cmd_id::ACTIVE_BASE) as usize;
            if frame.set_active_pane(p) {
                println!("[qdir-rs] TEST 活动窗口 #{}", p + 1);
                true
            } else {
                false
            }
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
        _ => false,
    }
}

/// 状态文件对话框（附录 A.1：原版用 GetOpen/SaveFileNameW 过滤 *.qdr）。
/// `save=true` 存盘，false 打开；取消返回 None。
fn run_state_dialog(main: HWND, save: bool) -> Option<std::path::PathBuf> {
    let mut buf = [0u16; 260];
    let desc: Vec<u16> = brand::STATE_FILTER_DESC
        .encode_utf16().chain(std::iter::once(0)).collect();
    // 过滤器：描述 *.qdr  全零结尾。
    let mut filter = desc.clone();
    filter.extend("*.qdr".encode_utf16().chain(std::iter::once(0)));
    filter.push(0);
    let mut ofn = OPENFILENAMEW::default();
    ofn.lStructSize = core::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = main;
    ofn.lpstrFilter = PCWSTR::from_raw(filter.as_ptr());
    ofn.lpstrFile = PWSTR::from_raw(buf.as_mut_ptr());
    ofn.nMaxFile = buf.len() as u32;
    let defext: Vec<u16> = "qdr".encode_utf16().chain(std::iter::once(0)).collect();
    ofn.lpstrDefExt = PCWSTR::from_raw(defext.as_ptr());
    ofn.Flags = OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT;
    let ok = unsafe {
        if save {
            ofn.Flags |= windows::Win32::UI::Controls::Dialogs::OFN_FILEMUSTEXIST;
            GetSaveFileNameW(&mut ofn)
        } else {
            ofn.Flags |= OFN_FILEMUSTEXIST;
            GetOpenFileNameW(&mut ofn)
        }
    };
    if ok.as_bool() {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(std::path::PathBuf::from(String::from_utf16_lossy(&buf[..end])))
    } else {
        None
    }
}

/// 导出保存对话框：四档 filter（TXT/CSV/HTML/XLS），默认 .txt。
fn run_export_dialog(main: HWND) -> Option<std::path::PathBuf> {
    let mut buf = [0u16; 260];
    let mut filter: Vec<u16> = Vec::new();
    for (desc, pat) in [
        ("文本文件 (*.txt)", "*.txt"),
        ("CSV (*.csv)", "*.csv"),
        ("HTML (*.html)", "*.htm;*.html"),
        ("Excel XML (*.xls)", "*.xls"),
    ] {
        filter.extend(desc.encode_utf16().chain(std::iter::once(0)));
        filter.extend(pat.encode_utf16().chain(std::iter::once(0)));
    }
    filter.push(0);
    let mut ofn = OPENFILENAMEW::default();
    ofn.lStructSize = core::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = main;
    ofn.lpstrFilter = PCWSTR::from_raw(filter.as_ptr());
    ofn.lpstrFile = PWSTR::from_raw(buf.as_mut_ptr());
    ofn.nMaxFile = buf.len() as u32;
    let defext: Vec<u16> = "txt".encode_utf16().chain(std::iter::once(0)).collect();
    ofn.lpstrDefExt = PCWSTR::from_raw(defext.as_ptr());
    ofn.lpstrTitle = PCWSTR::from_raw(
        "导出列表"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<u16>>()
            .as_ptr(),
    );
    ofn.Flags = OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT;
    if unsafe { GetSaveFileNameW(&mut ofn) }.as_bool() {
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(std::path::PathBuf::from(String::from_utf16_lossy(&buf[..end])))
    } else {
        None
    }
}


fn show_layout_menu(frame: &mut MainFrame, main: HWND) {
    let _ = frame;
    unsafe {
        let menu = match crate::ui_native::menu::build_layout_menu() {
            Ok(m) => m,
            Err(_) => return,
        };
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(main);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN,
            pt.x,
            pt.y,
            None,
            main,
            None,
        );
        let _ = DestroyMenu(menu);
        if cmd.0 >= crate::ui_native::menu::cmd_id::LAYOUT_BASE as i32 {
            let idx = (cmd.0 - crate::ui_native::menu::cmd_id::LAYOUT_BASE as i32) as usize;
            if let Some(&l) = Layout::ALL.get(idx) {
                set_layout(frame, main, l);
                println!("[qdir-rs] TEST 布局弹出选择 -> {}", l.label());
            }
        }
    }
}

/// 把载入的会话应用到界面（原版 E101=FUN_0046cc14 语义：全窗格重灌）。
fn apply_session(frame: &mut MainFrame, st: crate::core::SessionState) {
    frame.session = st;
    frame.splitters.from_permille_ten(&frame.session.splits_h);
    unsafe {
        for i in 0..4 {
            frame.fill_pane(i);
        }
    }
}

/// 以自有 exe 重复一个窗口（附录 A.1 E100=自启动；实现为 spawn）。
fn duplicate_window(state: &std::path::Path) -> crate::core::QResult<()> {
    let exe = std::env::current_exe().map_err(|e| crate::core::QError::from_io(&e))?;
    std::process::Command::new(exe)
        .arg(state)
        .spawn()
        .map(|_| ())
        .map_err(|e| crate::core::QError::from_io(&e))
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



}
