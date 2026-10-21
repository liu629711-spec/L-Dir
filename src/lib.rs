//! qdir-rs 库门面。
//!
//! 依赖方向 D1：core 是最底层，config 依赖 core。
//! `src/main.rs` 的 eframe 入口与 `tests/` 下的集成测试都通过本 crate 访问它们，
//! 因此这里只做 `pub mod` 声明，不含任何逻辑。
//!
//! 注意：本文件与 `src/main.rs` 各自编译成一个 crate 目标（bin 与 lib 同名）。
//! bin 侧直接 `mod core; mod config;` 使用同一份源文件，不存在代码重复。

pub mod brand;
pub mod config;
pub mod core;
pub mod filters;
pub mod panes;
pub mod services;
pub mod shell;
pub mod ui;
pub mod ui_native;
