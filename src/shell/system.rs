//! 系统级 shell 便利函数（终端启动 / IShellDispatch 对话框）。
//!
//! 行为语义来源：NATIVE_SPEC 附录 A.2（反编译确证）——
//! 命令行/PowerShell = 窗格路径为工作目录起进程；「运行」=
//! IShellDispatch::FileRun（原版 CoCreateInstance Shell.Application
//! {13709620-C279-11CE-A49E-444553540000}，vtable 槽 0x80）。

use std::path::Path;
use std::process::Command;

use windows::Win32::UI::Shell::IShellDispatch;

use super::{shell_error, QResult};

/// 在 `cwd` 目录起一个控制台进程（cmd.exe / powershell.exe）。
pub fn spawn_terminal(exe: &str, cwd: &Path) -> QResult<()> {
    Command::new(exe)
        .current_dir(cwd)
        .spawn()
        .map(|_| ())
        .map_err(|e| crate::core::QError::from_io(&e))
}

/// 系统「运行」对话框（IShellDispatch::FileRun；主线程 COM 由 App::run 初始化）。
pub fn shell_file_run() -> QResult<()> {
    let disp: IShellDispatch = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &windows::Win32::UI::Shell::Shell,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
        .map_err(|e| shell_error("CoCreateInstance Shell", e))?
    };
    unsafe { disp.FileRun() }.map_err(|e| shell_error("IShellDispatch::FileRun", e))
}

/// 关机设置对话框（IShellDispatch::ShutdownWindows；原版确证槽 0xA0）。
pub fn shell_shutdown_windows() -> QResult<()> {
    let disp: IShellDispatch = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &windows::Win32::UI::Shell::Shell,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
        .map_err(|e| shell_error("CoCreateInstance Shell", e))?
    };
    unsafe { disp.ShutdownWindows() }.map_err(|e| shell_error("IShellDispatch::ShutdownWindows", e))
}
