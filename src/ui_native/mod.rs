//! ui-native 门面：原生 Win32 UI 层（1:1 复刻路线）。
//!
//! 与 egui 版并存；bin 入口切换到本模块。
//! 资源策略：原版模板字节（assets/extracted）运行时直接消费——布局同源。

pub mod app;
pub mod commands;
pub mod mainframe;
pub mod menu;
pub mod splitter;
pub mod subclass;
pub mod toolbar;
pub mod tree;

pub use app::App;
pub use mainframe::MainFrame;
