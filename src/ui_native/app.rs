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
    pub fn run(config: Config, session: SessionState) -> std::result::Result<i32, Box<dyn std::error::Error>> {
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
            let mut pending = PendingFrame::new(config, session, hinstance);
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

            // 9. 消息循环。
            let mut msg = MSG::default();
            let mut exit_code = 0i32;
            loop {
                let r = GetMessageW(&mut msg, None, 0, 0);
                if !r.as_bool() {
                    exit_code = msg.wParam.0 as i32;
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

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
