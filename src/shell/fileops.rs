//! 文件操作：复制/移动/删除/重命名/新建文件夹/打开/属性/打印。
//!
//! 分层策略（与契约 modules 一致）：
//! - 主路径：`SHFileOperationW` —— 带系统进度框、回收站（FOF_ALLOWUNDO）、
//!   冲突处理、UNC/压缩包支持；
//! - 兜底：`CopyFileW` / `MoveFileW` / `DeleteFileW` / `RemoveDirectoryW`
//!   （SHFileOperationW 不可用或调用方显式要求静默时）；
//! - `move_file_confirmation` 语义：开启时移动操作附加
//!   `FOF_SIMPLEPROGRESS`（系统进度框自带确认），关闭时加 `FOF_NOCONFIRMATION`。
//!
//! 所有函数返回 [`FileOpReport`] 而非 unit，让上层知道「有多少项被处理、
//! 是否被用户中止、失败项是哪些」——原版 Q-Dir 的状态栏需要这些计数。

use std::path::{Path, PathBuf};

use windows::Win32::Storage::FileSystem::{
    CopyFileW, CreateDirectoryW, DeleteFileW, MoveFileW, RemoveDirectoryW,
};
use windows::Win32::UI::Shell::{
    FO_COPY, FO_DELETE, FO_MOVE, FO_RENAME, FOF_ALLOWUNDO, FOF_NOCONFIRMATION,
    FOF_NOCONFIRMMKDIR, FOF_SILENT, FOF_SIMPLEPROGRESS,
    SE_ERR_ACCESSDENIED, SE_ERR_DLLNOTFOUND, SE_ERR_FNF, SE_ERR_NOASSOC, SE_ERR_OOM,
    SE_ERR_PNF, SHFileOperationW, ShellExecuteW, SHFILEOPSTRUCTW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::{
    as_pcwstr, dir_with_backslash, double_nul, last_error_code, path_to_wide, to_wide,
    win32_error, HostWindow,
};
use crate::core::{QError, QResult};

/// 文件操作选项。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileOpOptions {
    /// 允许撤销（送回收站 / Ctrl+Z）。
    pub allow_undo: bool,
    /// 多目标时是否静默（不弹进度框）。
    pub silent: bool,
    /// 新建目标目录（FOF_NOCONFIRMMKDIR 的补足）。
    pub create_missing_dirs: bool,
    /// 重命名时遇冲突自动改名（FOF_RENAMEONCOLLISION）。
    pub rename_on_collision: bool,
    /// 移动前确认（`move_file_confirmation`）。
    pub move_confirmation: bool,
}

impl Default for FileOpOptions {
    fn default() -> Self {
        FileOpOptions {
            allow_undo: false,
            silent: false,
            create_missing_dirs: true,
            rename_on_collision: false,
            move_confirmation: true,
        }
    }
}

/// 单次操作的结果统计。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileOpReport {
    /// 成功处理的条目数。
    pub succeeded: usize,
    /// 用户中止（SHFileOperationW 的 fAnyOperationsAborted）。
    pub aborted: bool,
    /// 失败条目及其原因（路径 → OS 错误码）。
    pub failures: Vec<(PathBuf, u32)>,
}

impl FileOpReport {
    fn ok(n: usize) -> Self {
        FileOpReport {
            succeeded: n,
            ..Default::default()
        }
    }

    /// 构造「单条目失败」报告（当前仅测试与兜底路径使用）。
    #[allow(dead_code)]
    fn failed(path: PathBuf, code: u32) -> Self {
        FileOpReport {
            succeeded: 0,
            aborted: false,
            failures: vec![(path, code)],
        }
    }

    /// 是否全部成功。
    pub fn all_ok(&self) -> bool {
        self.failures.is_empty() && !self.aborted
    }
}

/// 复制文件/目录（SHFileOperationW）。
pub fn copy_files(
    hwnd: HostWindow,
    sources: &[PathBuf],
    dest_dir: &Path,
    opts: &FileOpOptions,
) -> QResult<FileOpReport> {
    if sources.is_empty() {
        return Ok(FileOpReport::default());
    }
    let from = build_double_nul(sources)?;
    let to = dir_with_backslash(dest_dir)?;
    let flags = op_flags(opts, false);
    sh_file_op(hwnd, FO_COPY, &from, &to, flags)
}

/// 移动文件/目录（SHFileOperationW）。
///
/// `opts.move_confirmation` 决定是否让系统进度框带确认步骤
/// （原版 `move_file_confirmation` 语义）。
pub fn move_files(
    hwnd: HostWindow,
    sources: &[PathBuf],
    dest_dir: &Path,
    opts: &FileOpOptions,
) -> QResult<FileOpReport> {
    if sources.is_empty() {
        return Ok(FileOpReport::default());
    }
    let from = build_double_nul(sources)?;
    let to = dir_with_backslash(dest_dir)?;
    let flags = op_flags(opts, true);
    sh_file_op(hwnd, FO_MOVE, &from, &to, flags)
}

/// 用 `core::FileOpsConfig` 的语义执行移动（上层直接传 ini 配置）。
pub fn move_files_with_config(
    hwnd: HostWindow,
    sources: &[PathBuf],
    dest_dir: &Path,
    cfg: &crate::core::FileOpsConfig,
) -> QResult<FileOpReport> {
    let opts = FileOpOptions {
        allow_undo: false,
        silent: false,
        create_missing_dirs: true,
        rename_on_collision: false,
        move_confirmation: cfg.move_file_confirmation,
    };
    move_files(hwnd, sources, dest_dir, &opts)
}

/// 删除文件/目录。
///
/// `opts.allow_undo = true` 时送回收站（FOF_ALLOWUNDO），否则永久删除。
pub fn delete_files(
    hwnd: HostWindow,
    sources: &[PathBuf],
    opts: &FileOpOptions,
) -> QResult<FileOpReport> {
    if sources.is_empty() {
        return Ok(FileOpReport::default());
    }
    let from = build_double_nul(sources)?;
    let flags = op_flags(opts, false) | FOF_NOCONFIRMMKDIR;
    sh_file_op(hwnd, FO_DELETE, &from, &[], flags)
}

/// 重命名单个路径（同目录内改名，用 FO_RENAME）。
///
/// SHFileOperationW(FO_RENAME) 要求 `pTo` 为**完整目标路径**（与源同目录 +
/// 新名）；只传裸名会被判为跨目录而返回 DE_DIFFDIR(115)。故此处把
/// `new_name` 拼到源路径的父目录后再交给 shell API。
pub fn rename_path(
    hwnd: HostWindow,
    path: &Path,
    new_name: &str,
    opts: &FileOpOptions,
) -> QResult<FileOpReport> {
    if new_name.is_empty() {
        return Err(QError::InvalidPath("新名称为空".to_string()));
    }
    if new_name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(QError::InvalidPath(format!(
            "新名称含有非法字符：{new_name:?}"
        )));
    }
    // 目标 = 源父目录 + 新名（FO_RENAME 同目录语义）。无父目录（卷根）不可改名。
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).ok_or_else(|| {
        QError::InvalidPath(format!("无父目录，无法重命名：{}", path.display()))
    })?;
    let dest = parent.join(new_name);
    let from = path_to_wide(path)?;
    let to = path_to_wide(&dest)?;
    let flags = op_flags(opts, false);
    sh_file_op(hwnd, FO_RENAME, &from, &to, flags)
}

/// 新建文件夹。返回新建目录的完整路径。
pub fn new_folder(hwnd: HostWindow, parent: &Path, name: &str) -> QResult<PathBuf> {
    if name.is_empty() {
        return Err(QError::InvalidPath("文件夹名为空".to_string()));
    }
    if name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(QError::InvalidPath(format!(
            "文件夹名含有非法字符：{name:?}"
        )));
    }
    let target = parent.join(name);
    if target.exists() {
        return Err(QError::Io {
            code: 183, // ERROR_ALREADY_EXISTS
            msg: format!("目标已存在：{}", target.display()),
        });
    }
    let wide = path_to_wide(&target)?;
    // SAFETY: wide 为 NUL 结尾宽串，安全属性为空。
    unsafe { CreateDirectoryW(as_pcwstr(&wide), None) }
        .map_err(|e| super::shell_error("CreateDirectoryW", e))?;
    let _ = hwnd;
    Ok(target)
}

/// 用默认关联程序打开路径（ShellExecuteW "open"）。
pub fn open_default(hwnd: HostWindow, path: &Path) -> QResult<()> {
    open_path_with(hwnd, path, "open", None)
}

/// 打印一组文件（shell `print` 动词，逐个交给关联程序）。
/// 返回成功投递的数量；单个失败（无关联/无 print 动词）跳过不阻断。
///
/// 注：原版打印命令 id 未反编译确证（*待复核*）；语义按 Explorer 右键
/// 「打印」惯例：选中文件逐个 shell print。
pub fn print_paths(hwnd: HostWindow, paths: &[std::path::PathBuf]) -> usize {
    let mut ok = 0;
    for p in paths {
        if open_path_with(hwnd, p, "print", None).is_ok() {
            ok += 1;
        }
    }
    ok
}

/// 通过电子邮件发送（命令按钮「通过电子邮件发送」）。
///
/// 走 Windows Shell 的 `mail` 动词（MAPI 外壳扩展；无邮件客户端时由
/// ShellExecuteW 返回错误码）。原版按钮语义即 shell send-mail，*行为待复核*。
pub fn send_by_mail(hwnd: HostWindow, path: &Path) -> QResult<()> {
    open_path_with(hwnd, path, "mail", None)
}

/// 创建 .lnk 快捷方式（收藏夹「添加到收藏夹…」= 原版 Favoriten 目录里的
/// shell 链接）。target 指向目标路径，lnk 落盘。
///
/// 需要 STA（IPersistFile::Save 走 COM）。*注：原版 .lnk 的具体命名规则
/// （前缀/编号）未从原版 Favoriten 目录取得样本（用户侧为空目录），
/// 这里用目标名直译命名，待样本复核*。
pub fn create_shortcut(lnk: &Path, target: &Path) -> QResult<()> {
    super::validate_path(lnk)?;
    super::validate_path(target)?;
    let _com = super::ensure_com_apartment()?;
    let link: windows::Win32::UI::Shell::IShellLinkW = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &windows::Win32::UI::Shell::ShellLink,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
        .map_err(|e| super::shell_error("CoCreateInstance ShellLink", e))?
    };
    let target_wide = path_to_wide(target)?;
    unsafe {
        link.SetPath(windows::core::PCWSTR::from_raw(target_wide.as_ptr()))
            .map_err(|e| super::shell_error("IShellLinkW::SetPath", e))?;
    }
    let persist: windows::Win32::System::Com::IPersistFile = unsafe {
        windows::core::Interface::cast(&link)
            .map_err(|e| super::shell_error("cast IPersistFile", e))?
    };
    let lnk_wide = path_to_wide(lnk)?;
    unsafe {
        persist
            .Save(windows::core::PCWSTR::from_raw(lnk_wide.as_ptr()), true)
            .map_err(|e| super::shell_error("IPersistFile::Save", e))?;
    }
    Ok(())
}

/// 读取 .lnk 的目标路径（Quick-Link 菜单项 → 导航目标）。
///
/// 解析链：CoCreateInstance(ShellLink) → IPersistFile::Load → GetPath
/// （SLGP_RAWPATH，不触发链接解析对话框）。需要 STA。
pub fn read_shortcut_target(lnk: &Path) -> QResult<PathBuf> {
    super::validate_path(lnk)?;
    let _com = super::ensure_com_apartment()?;
    let link: windows::Win32::UI::Shell::IShellLinkW = unsafe {
        windows::Win32::System::Com::CoCreateInstance(
            &windows::Win32::UI::Shell::ShellLink,
            None,
            windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
        )
        .map_err(|e| super::shell_error("CoCreateInstance ShellLink", e))?
    };
    let persist: windows::Win32::System::Com::IPersistFile = unsafe {
        windows::core::Interface::cast(&link)
            .map_err(|e| super::shell_error("cast IPersistFile", e))?
    };
    let lnk_wide = path_to_wide(lnk)?;
    unsafe {
        persist
            .Load(
                windows::core::PCWSTR::from_raw(lnk_wide.as_ptr()),
                windows::Win32::System::Com::STGM_READ,
            )
            .map_err(|e| super::shell_error("IPersistFile::Load", e))?;
        let mut buf = [0u16; 260];
        link.GetPath(
            &mut buf,
            std::ptr::null_mut(),
            windows::Win32::UI::Shell::SLGP_RAWPATH.0 as u32,
        )
        .map_err(|e| super::shell_error("IShellLinkW::GetPath", e))?;
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let s = String::from_utf16_lossy(&buf[..end]);
        if s.is_empty() {
            return Err(QError::Shell("快捷方式目标为空".to_string()));
        }
        Ok(PathBuf::from(s))
    }
}

/// 指定动词执行（open / edit / print / properties / ...）。
///
/// `parameters` 仅对可执行文件有意义。
pub fn open_path_with(
    hwnd: HostWindow,
    path: &Path,
    verb: &str,
    parameters: Option<&str>,
) -> QResult<()> {
    let file = path_to_wide(path)?;
    let op = to_wide(verb);
    let params = parameters.map(to_wide);
    // ShellExecuteW 的宽串形参是 `Option<&PCWSTR>`（None = 空串）。
    let op_w = super::as_pcwstr(&op);
    let file_w = super::as_pcwstr(&file);
    let params_w = params.as_ref().map(|p| super::as_pcwstr(p));
    // SAFETY: 全部为 NUL 结尾宽串；hwnd 可为空句柄。
    let inst = unsafe {
        ShellExecuteW(
            Some(hwnd.hwnd()),
            Some(&op_w),
            Some(&file_w),
            params_w.as_ref(),
            None,
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW 用 HINSTANCE 返回值传错误码；>32 表示成功。
    let code = inst.0 as usize;
    if code <= 32 {
        let qcode = match code {
            2 => SE_ERR_FNF,
            3 => SE_ERR_PNF,
            5 => SE_ERR_ACCESSDENIED,
            8 => SE_ERR_OOM,
            31 => SE_ERR_NOASSOC,
            32 => SE_ERR_DLLNOTFOUND,
            other => other as u32,
        };
        return Err(QError::Io {
            code: qcode,
            msg: format!("ShellExecuteW({verb}) 失败于 {}", path.display()),
        });
    }
    Ok(())
}

/// 显示系统属性对话框（等价资源管理器「属性」）。
pub fn show_properties(hwnd: HostWindow, path: &Path) -> QResult<()> {
    open_path_with(hwnd, path, "properties", None)
}

/// 打印文件（走系统的 print 动词）。
pub fn print_file(hwnd: HostWindow, path: &Path) -> QResult<()> {
    open_path_with(hwnd, path, "print", None)
}

/// 查询回收站占用。
pub fn recycle_bin_info() -> QResult<(u64, u64)> {
    use windows::Win32::UI::Shell::{SHQUERYRBINFO, SHQueryRecycleBinW};
    let root = to_wide("C:\\");
    let mut info: SHQUERYRBINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHQUERYRBINFO>() as u32;
    // SAFETY: root 合法，info 为出参。
    unsafe { SHQueryRecycleBinW(as_pcwstr(&root), &mut info) }
        .map_err(|e| super::shell_error("SHQueryRecycleBinW", e))?;
    Ok((info.i64Size as u64, info.i64NumItems as u64))
}

/// 清空回收站。
pub fn empty_recycle_bin(hwnd: HostWindow) -> QResult<()> {
    use windows::Win32::UI::Shell::SHEmptyRecycleBinW;
    let root = to_wide("C:\\");
    // SAFETY: root 合法；hwnd 可为空。
    unsafe { SHEmptyRecycleBinW(Some(hwnd.hwnd()), as_pcwstr(&root), 0) }
        .map_err(|e| super::shell_error("SHEmptyRecycleBinW", e))
}

// ============ 内部实现 ============

/// 构造双 NUL 结尾的来源列表。
fn build_double_nul(sources: &[PathBuf]) -> QResult<Vec<u16>> {
    let mut wides = Vec::with_capacity(sources.len());
    for s in sources {
        wides.push(path_to_wide(s)?);
    }
    Ok(double_nul(&wides))
}

/// 由选项计算 FOF_* 标志。
fn op_flags(opts: &FileOpOptions, is_move: bool) -> windows::Win32::UI::Shell::FILEOPERATION_FLAGS {
    use windows::Win32::UI::Shell::FILEOPERATION_FLAGS;
    let mut f = FILEOPERATION_FLAGS(0);
    if opts.allow_undo {
        f |= FOF_ALLOWUNDO;
    }
    if opts.silent {
        f |= FOF_SILENT | FOF_NOCONFIRMATION;
    }
    if opts.create_missing_dirs {
        f |= FOF_NOCONFIRMMKDIR;
    }
    if opts.rename_on_collision {
        f |= windows::Win32::UI::Shell::FOF_RENAMEONCOLLISION;
    }
    // move_file_confirmation：开 → 让系统进度框走确认路径；关 → 全部静默确认。
    if is_move {
        if opts.move_confirmation {
            f |= FOF_SIMPLEPROGRESS;
        } else {
            f |= FOF_NOCONFIRMATION;
        }
    }
    f
}

/// SHFileOperationW 统一入口。
///
/// `to` 为空表示无目标（删除）。
fn sh_file_op(
    hwnd: HostWindow,
    func: u32,
    from: &[u16],
    to: &[u16],
    flags: windows::Win32::UI::Shell::FILEOPERATION_FLAGS,
) -> QResult<FileOpReport> {
    let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
    op.hwnd = hwnd.hwnd();
    op.wFunc = func;
    op.pFrom = as_pcwstr(from);
    op.pTo = if to.is_empty() {
        windows::core::PCWSTR::null()
    } else {
        as_pcwstr(to)
    };
    op.fFlags = flags.0 as u16;
    op.fAnyOperationsAborted = windows::core::BOOL(0);

    // SAFETY: op 已完整初始化，from/to 存活至调用返回。
    let r = unsafe { SHFileOperationW(&mut op) };
    if r != 0 {
        let code = last_error_code();
        return Err(QError::Io {
            code,
            msg: format!("SHFileOperationW 失败（返回 {r}，OS 错误码 {code}）"),
        });
    }
    if op.fAnyOperationsAborted.as_bool() {
        return Err(QError::Cancelled);
    }
    // SHFileOperationW 不给逐项结果，只能报「整批成功」。
    Ok(FileOpReport::ok(count_items(from)))
}

/// 统计双 NUL 列表中的条目数（用于报告）。
fn count_items(list: &[u16]) -> usize {
    if list.len() < 2 {
        return 0;
    }
    let mut n = 0;
    let mut start = 0;
    for i in 0..list.len() {
        if list[i] == 0 {
            if i > start {
                n += 1;
            }
            start = i + 1;
        }
    }
    n
}

// ============ 兜底路径（SHFileOperationW 不可用时） ============

/// 单文件复制兜底（CopyFileW）。失败不重试，返回 OS 错误码。
pub fn copy_file_fallback(src: &Path, dst: &Path, overwrite: bool) -> QResult<FileOpReport> {
    let s = path_to_wide(src)?;
    let d = path_to_wide(dst)?;
    // SAFETY: 两个宽串均合法。
    unsafe { CopyFileW(as_pcwstr(&s), as_pcwstr(&d), overwrite) }
        .map_err(|e| win32_error("CopyFileW", windows::Win32::Foundation::WIN32_ERROR(e.code().0 as u32)))?;
    Ok(FileOpReport::ok(1))
}

/// 单文件移动兜底（MoveFileW）。
pub fn move_file_fallback(src: &Path, dst: &Path) -> QResult<FileOpReport> {
    let s = path_to_wide(src)?;
    let d = path_to_wide(dst)?;
    // SAFETY: 两个宽串均合法。
    unsafe { MoveFileW(as_pcwstr(&s), as_pcwstr(&d)) }
        .map_err(|e| win32_error("MoveFileW", windows::Win32::Foundation::WIN32_ERROR(e.code().0 as u32)))?;
    Ok(FileOpReport::ok(1))
}

/// 删除单个文件/空目录兜底。
pub fn delete_one_fallback(path: &Path) -> QResult<FileOpReport> {
    let w = path_to_wide(path)?;
    // 先按文件删，再按空目录删。
    // SAFETY: w 合法。
    let r = unsafe { DeleteFileW(as_pcwstr(&w)) };
    if r.is_ok() {
        return Ok(FileOpReport::ok(1));
    }
    // SAFETY: w 合法。
    let r2 = unsafe { RemoveDirectoryW(as_pcwstr(&w)) };
    r2.map(|_| FileOpReport::ok(1))
        .map_err(|e| win32_error("RemoveDirectoryW", windows::Win32::Foundation::WIN32_ERROR(e.code().0 as u32)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> FileOpOptions {
        FileOpOptions::default()
    }

    #[test]
    fn report_all_ok_semantics() {
        let r = FileOpReport::ok(3);
        assert!(r.all_ok());
        assert_eq!(r.succeeded, 3);
        assert!(!r.aborted);

        let r = FileOpReport::failed(PathBuf::from("C:\\a"), 5);
        assert!(!r.all_ok());
        assert_eq!(r.failures.len(), 1);
        assert_eq!(r.failures[0].1, 5);

        let mut r = FileOpReport::ok(1);
        r.aborted = true;
        assert!(!r.all_ok());
    }

    #[test]
    fn count_items_handles_double_nul() {
        let a = to_wide("a");
        let b = to_wide("b");
        let list = double_nul(&[a, b]);
        assert_eq!(count_items(&list), 2);
        assert_eq!(count_items(&[0, 0]), 0);
        assert_eq!(count_items(&to_wide("only")), 1);
    }

    #[test]
    fn empty_sources_short_circuit() {
        // 不触达 Windows API：空列表必须直接返回空报告。
        let h = HostWindow::from_raw_hwnd(0);
        let r = copy_files(h, &[], Path::new("C:\\tmp"), &opts()).expect("空列表应成功");
        assert_eq!(r.succeeded, 0);
        assert!(r.failures.is_empty());
        let r = delete_files(h, &[], &opts()).expect("空列表应成功");
        assert_eq!(r.succeeded, 0);
        let r = move_files(h, &[], Path::new("C:\\tmp"), &opts()).expect("空列表应成功");
        assert_eq!(r.succeeded, 0);
    }

    #[test]
    fn rename_rejects_illegal_names() {
        let h = HostWindow::from_raw_hwnd(0);
        // 只验证校验层：非法名必须在触达 SHFileOperationW 之前就被拒。
        // 合法名会真正调用 shell API（需要真实文件），不在单测里执行。
        assert!(rename_path(h, Path::new("C:\\a"), "", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a/b", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a:b", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a?b", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a*b", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a\"b", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a<b", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a>b", &opts()).is_err());
        assert!(rename_path(h, Path::new("C:\\a"), "a|b", &opts()).is_err());
    }

    #[test]
    fn new_folder_rejects_illegal_names() {
        let h = HostWindow::from_raw_hwnd(0);
        assert!(new_folder(h, Path::new("C:\\"), "").is_err());
        assert!(new_folder(h, Path::new("C:\\"), "a|b").is_err());
        // 合法名会真正调用 CreateDirectoryW；此处只验证校验层不放行非法名。
    }

    #[test]
    fn flags_follow_options() {
        use windows::Win32::UI::Shell::FILEOPERATION_FLAGS;
        let f = op_flags(&opts(), false);
        assert!(f.contains(FOF_NOCONFIRMMKDIR));
        assert!(!f.contains(FOF_ALLOWUNDO));

        let with_undo = FileOpOptions {
            allow_undo: true,
            ..opts()
        };
        assert!(op_flags(&with_undo, false).contains(FOF_ALLOWUNDO));

        let silent = FileOpOptions {
            silent: true,
            ..opts()
        };
        assert!(op_flags(&silent, false).contains(FOF_SILENT));
        assert!(op_flags(&silent, false).contains(FOF_NOCONFIRMATION));

        // move_file_confirmation 开 → SIMPLEPROGRESS；关 → NOCONFIRMATION。
        let confirm = FileOpOptions {
            move_confirmation: true,
            ..opts()
        };
        assert!(op_flags(&confirm, true).contains(FOF_SIMPLEPROGRESS));
        let no_confirm = FileOpOptions {
            move_confirmation: false,
            ..opts()
        };
        assert!(op_flags(&no_confirm, true).contains(FOF_NOCONFIRMATION));
        let _ = FILEOPERATION_FLAGS(0);
    }

    #[test]
    fn shortcut_write_then_read_roundtrip() {
        // create → read 往返：读回的应是同一目标（大小写由 shell 归一，
        // 用同一常量路径避开）。
        let dir = std::env::temp_dir().join(format!("qdir-rs-lnk-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let lnk = dir.join("rt.lnk");
        let _ = std::fs::remove_file(&lnk);
        create_shortcut(&lnk, std::path::Path::new(r"C:\Windows")).unwrap();
        let got = read_shortcut_target(&lnk).unwrap();
        assert_eq!(
            got.to_string_lossy().to_ascii_lowercase(),
            r"c:\windows",
            "往返目标不一致：{got:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_ui_flag_exists_for_future_use() {
        // 记录：FOF_NOERRORUI 未启用（原版会显示错误），仅确认常量值。
        assert_eq!(windows::Win32::UI::Shell::FOF_NOERRORUI.0, 1024);
    }
    #[test]
    fn create_shortcut_writes_valid_lnk() {
        // 真写一个 .lnk 到临时目录（create_shortcut 内部自管 STA）。
        let dir = std::env::temp_dir().join(format!("qdir-rs-fav-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let lnk = dir.join("test.lnk");
        let _ = std::fs::remove_file(&lnk);
        create_shortcut(&lnk, std::path::Path::new(r"C:\Windows")).unwrap();
        let bytes = std::fs::read(&lnk).unwrap();
        // .lnk HEADER：尺寸 0x4C + 类 GUID {00021401-0000-0000-C000-000000000046}（LE）。
        assert_eq!(&bytes[0..4], &[0x4C, 0, 0, 0]);
        assert_eq!(
            &bytes[4..20],
            &[
                0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x46
            ][..]
        );
        // 目标路径以 UTF-16LE 出现在文件里（Target 区段）。shell 会做
        // 路径规范化（如大小写归一），故对 "windows" 逐 UTF-16 码元
        // 做大小写无关查找。
        let needle: Vec<u16> = "windows".encode_utf16().collect();
        let found = bytes.chunks_exact(2).collect::<Vec<_>>();
        let hay = (0..found.len()).any(|i| {
            found[i..].len() >= needle.len()
                && needle.iter().enumerate().all(|(k, &n)| {
                    {
                        let c = u16::from_le_bytes([found[i + k][0], found[i + k][1]]);
                        // "windows" 全 ASCII：直接按 ASCII 规则归一。
                        let lc = |x: u16| {
                            if x >= 'A' as u16 && x <= 'Z' as u16 {
                                x + 32
                            } else {
                                x
                            }
                        };
                        lc(c) == lc(n)
                    }
                })
        });
        assert!(hay, ".lnk 中未找到目标路径");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
