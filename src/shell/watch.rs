//! 文件夹内容监视（批次D/D1；原版 AF:19522/180492 `SHChangeNotifyRegister` 确证
//! 「目录变化自动刷新」行为）。
//!
//! # 实现选型：ReadDirectoryChangesW（kernel32，overlapped I/O）
//!
//! 曾用 `SHChangeNotifyRegister`（shell32），止步于两个问题：
//! 1. **多条目 AV**：windows crate 的 `SHChangeNotifyEntry` 是
//!    `#[repr(C, packed(1))]`（12 字节步长），用 repr(C)（16 字节步长）时
//!    条目 ≥2 shell 从偏移 12 读到全零 padding 当 pidl → 解引用 NULL →
//!    STATUS_ACCESS_VIOLATION（单条目正常、多条目必崩；普通启动即段错误）。
//! 2. **修好步长后注册成功但收不到任何通知**：消息-only/真实窗口、WM_USER/WM_APP
//!    消息段、ShellLevel/InterruptLevel、事件掩码 0x201ff/ALLEVENTS、递归开关
//!    全矩阵实测零投递——本机 shell 投递链不通，根因未明；不再阻塞交付。
//!
//! RDC 语义等价（目录内容变化 → `PostMessage` 通知上层 → 防抖重灌），且不依赖
//! shell 投递。每个被监视路径一个后台线程（overlapped 读 + 200ms 轮询停止标志）。

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED,
    FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_DIR_NAME,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForSingleObjectEx};

use super::{as_pcwstr, HostWindow};

/// 通知缓冲区（64KB：一次读可携带多条 FILE_NOTIFY_INFORMATION）。
const BUF_LEN: usize = 64 * 1024;
/// 停止标志轮询间隔（兼顾响应速度与空转开销）。
const POLL_MS: u32 = 200;
/// WAIT_OBJECT_0：等待成功。
const WAIT_OBJECT_0: u32 = 0;

/// 监视注册句柄（批次D/D1）。
///
/// 持有各路径的停止标志与线程（RDC 用目录 HANDLE，无 PIDL 保活问题）。
pub struct WatchHandle {
    id: u32,
    stops: Vec<Arc<AtomicBool>>,
    threads: Vec<JoinHandle<()>>,
}

impl WatchHandle {
    /// 注册 id（日志用；RDC 无 shell id，用内部计数器）。
    pub fn id(&self) -> u32 {
        self.id
    }

    /// 释放：置停止标志并 join 全部线程（幂等；`Drop` 也调用）。
    pub fn release(&mut self) {
        for s in &self.stops {
            s.store(true, Ordering::SeqCst);
        }
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        self.stops.clear();
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        self.release();
    }
}

/// 注册目录内容监视（RDC）：每条路径一个后台线程，变化时向 `hwnd`
/// `PostMessage(msg_id, 0, 0)`（线程安全；由上层防抖重灌）。
/// 空路径列表返回 None。
pub fn register_change_notify(
    hwnd: HostWindow,
    paths: &[&Path],
    msg_id: u32,
) -> Option<WatchHandle> {
    if paths.is_empty() {
        return None;
    }
    static NEXT_ID: AtomicU32 = AtomicU32::new(1);
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let hwnd_raw = hwnd.hwnd().0 as isize;
    let mut stops = Vec::with_capacity(paths.len());
    let mut threads = Vec::with_capacity(paths.len());
    for p in paths {
        let stop = Arc::new(AtomicBool::new(false));
        let path = p.to_path_buf();
        let stop2 = Arc::clone(&stop);
        let handle = std::thread::Builder::new()
            .name(format!("ldir-watch-{id}-{}", stops.len()))
            .spawn(move || watch_path_loop(hwnd_raw, &path, msg_id, stop2))
            .ok()?;
        stops.push(stop);
        threads.push(handle);
    }
    Some(WatchHandle { id, stops, threads })
}

/// 单路径监视循环：打开目录 → overlapped `ReadDirectoryChangesW` → 有变化即
/// PostMessage → 继续；停止标志或句柄失效时退出。
fn watch_path_loop(hwnd_raw: isize, path: &Path, msg_id: u32, stop: Arc<AtomicBool>) {
    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: wide 合法（NUL 结尾）；目录句柄参数组合为 RDC 标准用法。
    let opened = unsafe {
        CreateFileW(
            as_pcwstr(&wide),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            None,
        )
    };
    let Ok(handle) = opened else {
        eprintln!("[l-dir] 监视打开目录失败 {path:?}");
        return;
    };
    if handle == INVALID_HANDLE_VALUE {
        eprintln!("[l-dir] 监视打开目录失败 {path:?}");
        return;
    }

    let mut buf = vec![0u8; BUF_LEN];
    'outer: loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // SAFETY: 默认属性的手动复位事件。
        let event = unsafe { CreateEventW(None, true, false, None) };
        let Ok(event) = event else {
            break; // 无法建事件：放弃该路径监视
        };
        let mut overlapped = OVERLAPPED {
            hEvent: HANDLE(event.0),
            ..Default::default()
        };
        let filter = FILE_NOTIFY_CHANGE_FILE_NAME
            | FILE_NOTIFY_CHANGE_DIR_NAME
            | FILE_NOTIFY_CHANGE_SIZE
            | FILE_NOTIFY_CHANGE_LAST_WRITE
            | FILE_NOTIFY_CHANGE_ATTRIBUTES;
        // SAFETY: handle 为目录句柄；buf 足够；overlapped 带有效事件。
        let ok = unsafe {
            ReadDirectoryChangesW(
                handle,
                buf.as_mut_ptr() as *mut _,
                BUF_LEN as u32,
                false, // 非递归（只监视目录自身内容）
                filter,
                None,
                Some(&mut overlapped),
                None,
            )
        };
        if ok.is_err() {
            // 目录被删除/重命名等：放弃该路径。
            let _ = unsafe { CloseHandle(event) };
            break;
        }
        loop {
            let wait = unsafe { WaitForSingleObjectEx(event, POLL_MS, true) };
            if stop.load(Ordering::SeqCst) {
                // SAFETY: 取消挂起 I/O 后关闭事件与目录句柄。
                let _ = unsafe { CancelIoEx(handle, Some(&overlapped)) };
                let _ = unsafe { CloseHandle(event) };
                let _ = unsafe { CloseHandle(handle) };
                return;
            }
            if wait.0 == WAIT_OBJECT_0 {
                let mut transferred = 0u32;
                // SAFETY: overlapped 已完成，transferred 为出参。
                let got = unsafe { GetOverlappedResult(handle, &overlapped, &mut transferred, false) };
                let _ = unsafe { ResetEvent(event) };
                if got.is_ok() && transferred > 0 {
                    // 通知上层（跨线程 PostMessage 安全）。
                    // SAFETY: hwnd_raw 为主窗口句柄（注册时快照）。
                    // SAFETY: hwnd_raw 为主窗口句柄（注册时快照）。
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                            Some(windows::Win32::Foundation::HWND(hwnd_raw as *mut _)),
                            msg_id,
                            windows::Win32::Foundation::WPARAM(0),
                            windows::Win32::Foundation::LPARAM(0),
                        );
                    }
                }
                let _ = unsafe { CloseHandle(event) };
                continue 'outer; // 重新发起一次读
            }
            // WAIT_TIMEOUT：继续等（停止标志下轮检查）。
        }
    }
    // SAFETY: 句柄归本线程所有。
    let _ = unsafe { CloseHandle(handle) };
    let _: HANDLE = handle;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_rejects_empty_paths() {
        let r = register_change_notify(HostWindow::from_raw_hwnd(0), &[], 0x8001);
        assert!(r.is_none(), "空路径列表不应注册");
    }

    #[test]
    fn watch_handle_release_is_idempotent() {
        // 无效句柄（空 stops/threads）release 不 panic；Drop 再调一次也安全。
        let mut h = WatchHandle {
            id: 0,
            stops: Vec::new(),
            threads: Vec::new(),
        };
        h.release();
        h.release();
        drop(h);
    }
}
