//! 原生 Win32 应用骨架：消息循环 + 初始化，规格来自 REA 反编译的 WinMain（0x4e5f20）。
//!
//! 初始化顺序与原版一致：CoInitialize → OleInitialize → InitCommonControlsEx(dwICC=0x6FF)
//! → 注册窗口类 → 主窗口 → 消息循环 → 逆序反初始化。

use windows::core::PCWSTR;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::{CoInitialize, CoUninitialize};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{OleInitialize, OleUninitialize};
use windows::Win32::UI::Controls::InitCommonControlsEx;
use windows::Win32::UI::Controls::INITCOMMONCONTROLSEX_ICC;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::{Config, SessionState};
use crate::ui_native::mainframe::{FRAME_CLASS_NAME, MainFrame, PendingFrame};

/// 与原版 WinMain 一致的 Common Controls 类集（0x6FF）：
/// 列表视图/树/页签/热键/Animate/Rebar/日期等。
const ICC_SET: u32 = 0x6FF;

/// 简单 OnceLock（不引依赖）。
pub mod once_cell_sync {
    use std::sync::OnceLock;
    pub type OnceCell<T> = OnceLock<T>;
}

/// 消息循环 + 生命周期，等价原版 WinMain。
pub struct App;

impl App {
    pub fn run(
        config: Config,
        session: SessionState,
        ini_path: std::path::PathBuf,
    ) -> std::result::Result<i32, Box<dyn std::error::Error>> {
        unsafe {
            // 1. COM/Ole（顺序照抄 0x4e5f20）。
            let _ = CoInitialize(None);
            let ole = OleInitialize(None);

            // 2. Common Controls（dwICC=0x6FF，与原版一致）。
            let icc = windows::Win32::UI::Controls::INITCOMMONCONTROLSEX {
                dwSize: 8,
                dwICC: INITCOMMONCONTROLSEX_ICC(ICC_SET),
            };
            InitCommonControlsEx(&icc);

            // 3. 实例句柄。
            let hmodule = GetModuleHandleW(None)?;
            let hinstance = HINSTANCE(hmodule.0);

            // 4. 注册主窗口类（Delphi 的类名规则：TApplication.<标题>）。
            // 类名宽串：编译期生成（w! 只接受字面量），这里用静态缓冲。
            let class_name: Vec<u16> = FRAME_CLASS_NAME.encode_utf16().chain(std::iter::once(0)).collect();
            let hCursor = LoadCursorW(None, IDC_ARROW)?;
            let hIcon = load_app_icon(hinstance);
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(MainFrame::wnd_proc),
                hInstance: hinstance,
                hCursor,
                hIcon,
                hbrBackground: Default::default(), // 背景由列表自绘刷子接管
                lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
                ..Default::default()
            };
            RegisterClassW(&wc);

            // 5. 主菜单（自绘文案与结构，见 ui_native::menu——PRODUCT_SPEC R1）。
            let menu = Some(crate::ui_native::menu::build_main_menu()?);

            // 6. 创建主窗口（初始尺寸 ini WinRC=0;0;800;600）。
            //    config/session 经 lpCreateParams 传入：WM_CREATE 在 CreateWindowExW
            //    返回之前就触发，只有这样才能让 on_create 用真实配置建 frame。
            let (w, h) = (config.win_rc[2].max(200), config.win_rc[3].max(150));
            let mut pending = PendingFrame::new(config, session, hinstance, ini_path);
            // 窗口标题宽串（w! 仅字面量，品牌常量经静态缓冲转换）。
            let title: Vec<u16> = crate::brand::PRODUCT_NAME
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let hwnd = CreateWindowExW(
                WS_EX_ACCEPTFILES, // 拖放文件语义
                PCWSTR(class_name.as_ptr()),
                PCWSTR(title.as_ptr()),
                WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0 | WS_CLIPSIBLINGS.0 | WS_CLIPCHILDREN.0),
                CW_USEDEFAULT, CW_USEDEFAULT, w, h,
                None,
                menu,
                Some(hinstance),
                Some(&mut pending as *mut PendingFrame as *const core::ffi::c_void),
            )
            .map_err(|e| format!("CreateWindowExW 失败: {e}"))?;
            // `pending` 位于本函数栈帧，生命周期覆盖整个 CreateWindowExW 调用，
            // 因此 on_create 从中取走所有权是安全的；未被取走时在此自然析构，不泄漏。
            let _ = ole;
            let _ = pending;

            // 8. 显示（showCmd 语义：SW_SHOWNORMAL 简化，原版读 GetStartupInfoW）。
            let _ = ShowWindow(hwnd, SW_SHOW);
            // 0.62 无 UpdateWindow（RedrawWindow 取代）：WM_PAINT 会被自动调度。
            let _ = hwnd;

            // 9. 加速键：F3=查找（聚焦活动窗格过滤框，与文件菜单 FIND 同命令）；
            //    Ctrl+T=新建标签页（批次A/A5，menu_texts:41 确证）；
            //    Alt+Left/Right=历史后退/前进（批次C/C4）；
            //    Ctrl+K=重复标签页、Ctrl+Shift+V=选择性粘贴、F9=数据大小信息
            //    （menu_texts:40/43/102，2026-10-24 批）。
            //    注：Ctrl+C/X/V/A/Del 保持**局部**（仅列表焦点时经 subclass
            //    生效，Explorer 语义），不进全局加速键，避免劫持过滤框文本编辑。
            use windows::Win32::UI::Input::KeyboardAndMouse::{
                VK_F11, VK_F3, VK_F9, VK_K, VK_LEFT, VK_RIGHT, VK_T, VK_V,
            };
            let accels: Vec<ACCEL> = vec![
                ACCEL {
                    fVirt: FVIRTKEY,
                    key: VK_F3.0,
                    cmd: crate::ui_native::menu::cmd_id::FIND as u16,
                },
                // P2：F11 全屏（menu_texts:73「全屏 (F)	F11」确证入口）。
                ACCEL {
                    fVirt: FVIRTKEY,
                    key: VK_F11.0,
                    cmd: crate::ui_native::menu::cmd_id::VIEW_FULLSCREEN as u16,
                },
                ACCEL {
                    fVirt: FVIRTKEY | FCONTROL,
                    key: VK_T.0,
                    cmd: crate::ui_native::menu::cmd_id::FILE_NEW_TAB as u16,
                },
                ACCEL {
                    fVirt: FVIRTKEY | FCONTROL,
                    key: VK_K.0,
                    cmd: crate::ui_native::menu::cmd_id::FILE_DUP_TAB as u16,
                },
                // P2 audit #22：选择性粘贴（局部 Ctrl+V 会先吃 Ctrl+Shift+V，
                // subclass.rs 对 Shift+V 显式让路到同一命令，双通道一致）。
                ACCEL {
                    fVirt: FVIRTKEY | FCONTROL | FSHIFT,
                    key: VK_V.0,
                    cmd: crate::ui_native::menu::cmd_id::EDIT_PASTE_SPECIAL as u16,
                },
                ACCEL {
                    fVirt: FVIRTKEY,
                    key: VK_F9.0,
                    cmd: crate::ui_native::menu::cmd_id::FILE_SIZE_INFO as u16,
                },
                ACCEL {
                    fVirt: FVIRTKEY | FALT,
                    key: VK_LEFT.0,
                    cmd: crate::ui_native::menu::cmd_id::HIST_BACK as u16,
                },
                ACCEL {
                    fVirt: FVIRTKEY | FALT,
                    key: VK_RIGHT.0,
                    cmd: crate::ui_native::menu::cmd_id::HIST_FORWARD as u16,
                },
            ];
            let accel_table = unsafe { CreateAcceleratorTableW(&accels) }
                .unwrap_or(windows::Win32::UI::WindowsAndMessaging::HACCEL::default());

            // 10. 消息循环。
            let mut msg = MSG::default();
            let mut exit_code = 0i32;
            loop {
                let r = GetMessageW(&mut msg, None, 0, 0);
                if !r.as_bool() {
                    exit_code = msg.wParam.0 as i32;
                    break;
                }
                // 加速键优先（原版 F3 行为）。
                if !accel_table.is_invalid()
                    && unsafe { TranslateAcceleratorW(hwnd, accel_table, &msg) } != 0
                {
                    continue;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            unsafe { let _ = DestroyAcceleratorTable(accel_table); };

            // 10. 反初始化（逆序）。
            OleUninitialize();
            CoUninitialize();
            Ok(exit_code)
        }
    }
}

/// 应用图标：暂用系统默认；自绘图标后续接入（R1：不使用原版图标资源）。
fn load_app_icon(_hinstance: HINSTANCE) -> HICON {
    unsafe { LoadIconW(None, IDI_APPLICATION).unwrap_or_default() }
}
