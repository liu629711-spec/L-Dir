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

/// 启动参数分类（批次C/C3 纯函数）：
/// - `.qdr` 后缀（大小写不敏感）→ [`LaunchArg::Qdr`]（状态文件）；
/// - 已存在的目录 → [`LaunchArg::Dir`]（目录启动，覆盖窗格1 初始路径）；
/// - 其余（旗标如 `--egui`、不存在的路径、空）→ [`LaunchArg::Ignore`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchArg {
    Qdr,
    Dir,
    Ignore,
}

/// 分类第一启动参数。纯函数（触达文件系统 `is_dir` 的部分由调用方传入
/// 已解析路径；单测用临时目录/不存路径覆盖两条分支）。
pub fn parse_launch_arg(arg: &Path) -> LaunchArg {
    if arg.as_os_str().is_empty() {
        return LaunchArg::Ignore;
    }
    let s = arg.to_string_lossy();
    // 旗标（- 开头）不参与路径解析。
    if s.starts_with('-') {
        return LaunchArg::Ignore;
    }
    if s.to_ascii_lowercase().ends_with(".qdr") {
        return LaunchArg::Qdr;
    }
    if arg.is_dir() {
        return LaunchArg::Dir;
    }
    LaunchArg::Ignore
}

/// 目录启动（批次C/C3）：把会话的窗格1 首个标签页指到该目录，历史重置。
/// 其余会话状态（布局/树/预览等）保持默认/文件值不动。
fn apply_dir_launch(session: &mut qdir_rs::core::SessionState, dir: &Path) {
    if let Some(tab) = session.panes[0].tabs.get_mut(0) {
        tab.path = dir.to_path_buf();
        tab.history = vec![dir.to_path_buf()];
        tab.history_pos = 0;
        tab.title = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.to_string_lossy().to_string());
    }
    session.focus = qdir_rs::core::PaneId::P1;
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
    // 启动参数分类（批次C/C3）：.qdr 状态文件 / 目录 / 其它（旗标/不存在）。
    let arg = std::env::args().nth(1).map(PathBuf::from);
    let arg_kind = arg.as_deref().map(parse_launch_arg).unwrap_or(LaunchArg::Ignore);
    let (qdr_path, dir_launch): (PathBuf, Option<PathBuf>) = match (&arg, arg_kind) {
        (Some(p), LaunchArg::Qdr) => (p.clone(), None),
        (Some(p), LaunchArg::Dir) => (
            pick(brand::SESSION_FILE, brand::LEGACY_SESSION_FILE),
            Some(p.clone()),
        ),
        _ => (pick(brand::SESSION_FILE, brand::LEGACY_SESSION_FILE), None),
    };
    // IDLIST 解码恢复窗格路径（批次D/D2：.qdr 的 FolderN 存的是 PIDL 百分号
    // 编码，此前 load 侧直接丢弃——启动恢复/重复窗口从未恢复过窗格路径）。
    let mut session = match config::load_session_with_idlists(&qdr_path) {
        Ok((mut s, idlists)) => {
            qdir_rs::ui_native::apply_idlist_paths(&mut s, &idlists);
            s
        }
        Err(e) => {
            eprintln!("[l-dir] 加载 {} 失败：{e}", qdr_path.display());
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Some(dir) = &dir_launch {
        apply_dir_launch(&mut session, dir);
        println!("[l-dir] TEST 目录启动 {dir:?}");
    }

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

#[cfg(test)]
mod tests {
    use super::{parse_launch_arg, LaunchArg};
    use std::path::Path;

    #[test]
    fn qdr_suffix_is_qdr() {
        assert_eq!(parse_launch_arg(Path::new(r"C:\x\a.qdr")), LaunchArg::Qdr);
        // 大小写不敏感（原版接受；旧实现大小写敏感是缺口）。
        assert_eq!(parse_launch_arg(Path::new(r"C:\x\A.QDR")), LaunchArg::Qdr);
    }

    #[test]
    fn existing_dir_is_dir() {
        let dir = std::env::temp_dir().join(format!("qdir-rs-launch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(parse_launch_arg(&dir), LaunchArg::Dir);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn missing_path_and_flags_are_ignored() {
        assert_eq!(
            parse_launch_arg(Path::new(r"C:\no\such\dir-9137")),
            LaunchArg::Ignore
        );
        // 旗标不受影响（--egui 由 args 迭代单独处理）。
        assert_eq!(parse_launch_arg(Path::new("--egui")), LaunchArg::Ignore);
        assert_eq!(parse_launch_arg(Path::new("")), LaunchArg::Ignore);
    }
}
