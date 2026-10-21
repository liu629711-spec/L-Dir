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
use crate::ui_native::res;

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
                hbrBackground: Default::default(), // 背景由 Q-Dir Colors 刷子接管
                lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
                ..Default::default()
            };
            RegisterClassW(&wc);

            // 5. 主菜单（原版菜单模板 128）。
            // 注：LoadMenuIndirectW 偶发返回 NULL 且 GetLastError==0（实测连续启停
            // 进程时复现）。重试 5 次 + 间隔；重试时复制到可写内存再加载。
            let menu = {
                let tmpl_bytes: Vec<u8> = res::menu_template("128")
                    .map(|b| b.to_vec())
                    .ok_or("menu 128 模板缺失")?;
                let mut last: Option<String> = None;
                let mut got: Option<HMENU> = None;
                for attempt in 0..5 {
                    SetLastError(WIN32_ERROR(0));
                    match load_menu_indirect(&tmpl_bytes) {
                        Ok(m) if !m.is_invalid() => {
                            got = Some(m);
                            break;
                        }
                        _ => {}
                    }
                    let e = windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(
                        GetLastError().0,
                    ));
                    last = Some(format!("{e}"));
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                match got {
                    Some(m) => Some(m),
                    None => return Err(format!("menu 128 加载失败: {:?}", last).into()),
                }
            };

            // 6. 创建主窗口（初始尺寸 Q-Dir.ini WinRC=0;0;800;600）。
            //    config/session 经 lpCreateParams 传入：WM_CREATE 在 CreateWindowExW
            //    返回之前就触发，只有这样才能让 on_create 用真实配置建 frame。
            let (w, h) = (config.win_rc[2].max(200), config.win_rc[3].max(150));
            let mut pending = PendingFrame::new(config, session, hinstance);
            let hwnd = CreateWindowExW(
                WS_EX_ACCEPTFILES, // 原版 DragAcceptFiles 语义
                PCWSTR(class_name.as_ptr()),
                windows::core::w!("Q-Dir"),
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

/// 原版菜单模板字节 → HMENU。
///
/// 与对话框同一条原则：**原字节原样交给 Windows**。Delphi 的 NEWMENU 模板
/// 本身就是标准 MENUTEMPLATE 的超集（弹出层多带 2 字节 id 字段），
/// LoadMenuIndirectW 能直接接受，不需要重建。
///
/// 依据：`examples/menu_probe.rs` 实测——原字节加载成功返回 HMENU；
/// 而"重建"版本会让 LoadMenuIndirectW 访问越界（STATUS_ACCESS_VIOLATION）。
fn load_menu_indirect(bytes: &[u8]) -> std::result::Result<HMENU, windows::core::Error> {
    unsafe { LoadMenuIndirectW(bytes.as_ptr() as *const core::ffi::c_void) }
}

fn load_app_icon(_hinstance: HINSTANCE) -> HICON {
    unsafe {
        // 原版图标组 128 的 .ico 转换后续补；先系统默认图标兜底。
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }
}
