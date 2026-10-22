//! L-Dir 入口（Q-Dir 同类产品，见 PRODUCT_SPEC）。
//!
//! 本文件只做组装 —— 读 `core::Config` 与 `core::SessionState`，
//! 构造 `ui_native::App`（原生 Win32 路线）并跑消息循环。
//! 配置加载：自有文件名优先；不存在时回落读取原版兼容文件（R2 导入特性）。
//! egui 版（ui::QDirApp）保留作对照参考，经 `--egui` 参数进入。

use std::path::{Path, PathBuf};

use qdir_rs::brand;
use qdir_rs::config;

/// 自有文件名存在则用之，否则回落原版兼容文件名（导入特性）。
fn pick(primary: &str, legacy: &str) -> PathBuf {
    let p = Path::new(primary);
    if p.exists() {
        p.to_path_buf()
    } else {
        PathBuf::from(legacy)
    }
}

fn main() -> std::process::ExitCode {
    // 配置与会话：文件缺失时各自返回默认值（config::load_* 约定）。
    let ini_path = pick(brand::INI_FILE, brand::LEGACY_INI_FILE);
    let config = match config::load_config(&ini_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[l-dir] 加载 {} 失败：{e}", ini_path.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    // 命令行第一参数若为 .qdr 路径（重复窗口/重新打开状态传入）优先加载之。
    let arg_state = std::env::args()
        .nth(1)
        .filter(|a| !a.starts_with('-') && a.ends_with(".qdr"))
        .map(PathBuf::from);
    let qdr_path = arg_state.unwrap_or_else(|| pick(brand::SESSION_FILE, brand::LEGACY_SESSION_FILE));
    let session = match config::load_session(&qdr_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[l-dir] 加载 {} 失败：{e}", qdr_path.display());
            return std::process::ExitCode::FAILURE;
        }
    };

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--egui") {
        // 对照路线：egui 立即模式版本。
        return match qdir_rs::ui::run_egui(config, session) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("[l-dir] egui 运行失败：{e:?}");
                std::process::ExitCode::FAILURE
            }
        };
    }

    // 默认：原生 Win32 路线（1:1 复刻）。
    match qdir_rs::ui_native::App::run(config, session, ini_path) {
        Ok(code) => std::process::ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("[l-dir] 运行失败：{e}");
            std::process::ExitCode::FAILURE
        }
    }
}
