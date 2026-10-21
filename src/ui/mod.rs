//! UI 模块门面（eframe/egui 立即模式）。
//!
//! 依赖方向：ui → panes / filters / shell / services / config / core；
//! 反方向不允许。ui 只做渲染与事件分发，业务一律调用
//! panes::PaneSet / filters::ListEngine / shell::* 的 API。

pub mod app;
pub mod pane;

pub use app::{DEFAULT_WINDOW_SIZE, QDirApp, WINDOW_TITLE, run_egui, set_hwnd};
