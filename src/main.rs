//! Q-Dir 重写工程入口。
//!
//! A3：本文件只做组装 —— 读 `core::Config` 与 `core::SessionState`，
//! 构造 `ui_native::App`（1:1 原生 Win32 路线）并跑消息循环。
//! egui 版（ui::QDirApp）保留作对照参考，经 `--egui` 参数进入。

use qdir_rs::config;

fn main() -> std::process::ExitCode {
    // 配置与会话：文件缺失时各自返回默认值（config::load_* 约定）。
    let config = match config::load_config(std::path::Path::new("Q-Dir.ini")) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[qdir-rs] 加载 Q-Dir.ini 失败：{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let session = match config::load_session(std::path::Path::new("start.qdr")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[qdir-rs] 加载 start.qdr 失败：{e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--egui") {
        // 对照路线：egui 立即模式版本。
        return match qdir_rs::ui::run_egui(config, session) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("[qdir-rs] egui 运行失败：{e:?}");
                std::process::ExitCode::FAILURE
            }
        };
    }

    // 默认：原生 Win32 路线（1:1 复刻）。
    match qdir_rs::ui_native::App::run(config, session) {
        Ok(code) => std::process::ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("[qdir-rs] 运行失败：{e}");
            std::process::ExitCode::FAILURE
        }
    }
}
