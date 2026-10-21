//! 拖放：外部拖入（WM_DROPFILES）+ OLE 拖出源。
//!
//! 两条独立路径：
//! - **拖入**：`DragAcceptFiles` + `DragQueryFileW` + `DragFinish`。
//!   上层在窗口过程收到 `WM_DROPFILES` 时把 `HDROP` 的原始指针交给
//!   [`DropFiles::from_raw`]，本模块取出文件列表后必须 `DragFinish`；
//! - **拖出**：`DoDragDrop` 需要一个 `IDataObject` 与 `IDropSource`。
//!   本模块用 `CIDLData_CreateFromIDArray` 造 shell 数据对象，
//!   `IDropSource` 由 [`SimpleDropSource`] 实现（只回答 S_OK）。
//!
//! `copy_cut_paste_mode` 语义：拖出时按 ini 配置给 CFSTR_PREFERREDDROPEFFECT
//! 打上 DROPEFFECT_COPY（0）或 DROPEFFECT_MOVE（1），让资源管理器显示
//! 「移动」光标。

use std::path::PathBuf;

use windows::Win32::System::Com::{IDataObject, FORMATETC};
use windows::Win32::UI::Shell::{
    CFSTR_PREFERREDDROPEFFECT, DragAcceptFiles, DragFinish, DragQueryFileW, HDROP,
    SHGetDesktopFolder, SHParseDisplayName,
};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;

use super::{
    as_pcwstr, free_pidl, io_error, path_to_wide, shell_error, HostWindow,
};
use crate::core::{QError, QResult};

/// OLE 拖放效果（本模块不引用 windows 的 DROPEFFECT 类型，只用其位值）。
type DropEffect = u32;

/// `DROPEFFECT_COPY`（复制）。
const DROPEFFECT_COPY_VALUE: DropEffect = 1;
/// `DROPEFFECT_MOVE`（移动/剪切）。
const DROPEFFECT_MOVE_VALUE: DropEffect = 2;

/// 一次外部拖入的文件列表。
///
/// 必须在 `WM_DROPFILES` 处理期间构造并取走 [`DropFiles::paths`]；
/// Drop 时自动 `DragFinish`。
pub struct DropFiles {
    hdrop: HDROP,
    paths: Vec<PathBuf>,
    finished: bool,
}

impl DropFiles {
    /// 从窗口过程拿到的 `HDROP` 原始指针构造。
    ///
    /// # Safety 说明
    /// 本函数是安全 API：内部校验句柄非空后才读取；句柄所有权随之转移给
    /// 返回值（Drop 时 `DragFinish`）。调用方不得再碰该句柄。
    pub fn from_raw(raw: isize) -> QResult<Self> {
        let hdrop = HDROP(raw as *mut std::ffi::c_void);
        if hdrop.is_invalid() {
            return Err(QError::Shell("WM_DROPFILES 句柄无效".to_string()));
        }
        let mut paths = Vec::new();
        // SAFETY: hdrop 由 shell 在 WM_DROPFILES 时分配，本函数取得所有权。
        let count = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
        for i in 0..count {
            let mut buf = vec![0u16; 32768];
            // SAFETY: buf 为出参；句柄有效。
            let n = unsafe { DragQueryFileW(hdrop, i, Some(&mut buf)) };
            if n == 0 || n as usize >= buf.len() {
                continue;
            }
            paths.push(PathBuf::from(String::from_utf16_lossy(&buf[..n as usize])));
        }
        Ok(DropFiles {
            hdrop,
            paths,
            finished: false,
        })
    }

    /// 拖入的文件列表（顺序与 shell 报告一致）。
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// 是否为空拖入。
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// 提前释放句柄（一般不需要，Drop 会做）。
    pub fn finish(mut self) {
        if !self.finished && !self.hdrop.is_invalid() {
            // SAFETY: 句柄由本类型拥有，未重复释放。
            unsafe { DragFinish(self.hdrop) };
            self.finished = true;
        }
    }
}

impl Drop for DropFiles {
    fn drop(&mut self) {
        if !self.finished && !self.hdrop.is_invalid() {
            // SAFETY: 句柄由本类型拥有。
            unsafe { DragFinish(self.hdrop) };
            self.finished = true;
        }
    }
}

/// 允许/禁止窗口接受外部拖入。
pub fn accept_drop_files(hwnd: HostWindow, accept: bool) {
    // SAFETY: hwnd 可为空（此时操作桌面窗口）；无内存前提。
    unsafe { DragAcceptFiles(hwnd.hwnd(), accept) };
}

/// 拖放落点（供上层判断拖到了哪个窗格）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropTarget {
    /// 屏幕坐标。
    pub screen_x: i32,
    pub screen_y: i32,
    /// 落点窗口句柄（0 = 未知）。
    pub hwnd_raw: isize,
}

impl DropTarget {
    /// 由 `WM_DROPFILES` 的坐标构造。
    pub fn new(x: i32, y: i32, hwnd_raw: isize) -> Self {
        DropTarget {
            screen_x: x,
            screen_y: y,
            hwnd_raw,
        }
    }
}

/// 拖出结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragOutcome {
    /// 用户完成了一次复制。
    Copy,
    /// 用户完成了一次移动。
    Move,
    /// 用户取消（Esc / 右键）。
    Cancelled,
    /// 目标不接受。
    None,
}

impl DragOutcome {
    /// 由拖放效果位值映射。
    fn from_effect(e: DropEffect) -> Self {
        if e == DROPEFFECT_COPY_VALUE {
            DragOutcome::Copy
        } else if e == DROPEFFECT_MOVE_VALUE {
            DragOutcome::Move
        } else if e == 0 {
            DragOutcome::None
        } else {
            DragOutcome::Cancelled
        }
    }
}

/// 拖出：把一批路径交给系统拖放循环（阻塞直到放下或取消）。
///
/// `cut` 对应 `copy_cut_paste_mode = 1`（剪切语义）。
pub fn begin_drag(hwnd: HostWindow, paths: &[PathBuf], cut: bool) -> QResult<DragOutcome> {
    if paths.is_empty() {
        return Ok(DragOutcome::None);
    }
    // DoDragDrop 需要 STA。
    let _com = super::ensure_com_apartment()?;

    let desktop = unsafe { SHGetDesktopFolder() }
        .map_err(|e| shell_error("SHGetDesktopFolder", e))?;

    // 每个路径 → PIDL（CIDLData_CreateFromIDArray 的输入）。
    let mut pidls: Vec<*mut ITEMIDLIST> = Vec::with_capacity(paths.len());
    for p in paths {
        let wide = path_to_wide(p)?;
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        let mut attrs = 0u32;
        // SAFETY: wide 合法，出参有效。
        unsafe {
            SHParseDisplayName(
                as_pcwstr(&wide),
                None,
                &mut pidl,
                0,
                Some(&mut attrs),
            )
        }
        .map_err(|e| shell_error(&format!("SHParseDisplayName({})", p.display()), e))?;
        if pidl.is_null() {
            for q in &pidls {
                // SAFETY: 归本函数所有。
                unsafe { free_pidl(*q) };
            }
            return Err(QError::Shell(format!(
                "SHParseDisplayName 未产出 PIDL：{}",
                p.display()
            )));
        }
        pidls.push(pidl);
    }

    let refs: Vec<*const ITEMIDLIST> = pidls.iter().map(|&p| p as *const ITEMIDLIST).collect();
    // 容器取桌面（拖出到任意位置，shell 会自己解析相对路径）。
    // CIDLData_CreateFromIDArray 需要「父文件夹 PIDL + 子 PIDL 数组」。
    // 桌面 PIDL 为空是合法的（CIDLData_CreateFromIDArray 传 null 表示桌面）。
    // SAFETY: refs 有效。
    let data: IDataObject = unsafe {
        windows::Win32::UI::Shell::CIDLData_CreateFromIDArray(
            std::ptr::null(),
            Some(&refs),
        )
    }
    .map_err(|e| {
        for q in &pidls {
            // SAFETY: 归本函数所有。
            unsafe { free_pidl(*q) };
        }
        shell_error("CIDLData_CreateFromIDArray", e)
    })?;

    // 打上 Preferred DropEffect（复制/剪切语义）。
    let effect: u32 = if cut { DROPEFFECT_MOVE_VALUE } else { DROPEFFECT_COPY_VALUE };
    if let Err(e) = set_preferred_drop_effect(&data, effect) {
        for q in &pidls {
            // SAFETY: 归本函数所有。
            unsafe { free_pidl(*q) };
        }
        return Err(e);
    }

    // SimpleDropSource 是手写 COM 对象；Box 拥有它，接口指针在拖放期间有效。
    let mut source = SimpleDropSource::new();
    let source_ptr = source.as_interface_ptr();
    let allowed: DropEffect = if cut { DROPEFFECT_MOVE_VALUE } else { DROPEFFECT_COPY_VALUE };
    let mut actual: DropEffect = 0;

    // SAFETY: data 为有效 IDataObject，source_ptr 指向有效 IDropSource vtable，
    // actual 为出参。拖放循环在返回前结束，source 的生命周期覆盖整个调用。
    //
    // 这里不用 windows crate 的 `DoDragDrop`：它的 pdropsrc 形参要求
    // `Param<IDropSource>`，而 IDropSource 是 windows 的 Interface 实现，
    // 本模块手写的 SimpleDropSource 无法满足该 bound。故直接绑定 ole32 的
    // 原始 ABI（签名与 windows crate 的 link! 声明一致）。
    let hr = unsafe { do_drag_drop_raw(&data, source_ptr, allowed, &mut actual) };
    drop(source);

    for q in &pidls {
        // SAFETY: 归本函数所有。
        unsafe { free_pidl(*q) };
    }
    let _ = desktop;
    let _ = hwnd;
    let _ = &mut actual;

    // DRAGDROP_S_DROP = 0x00040100；DRAGDROP_S_CANCEL = 0x00040101。
    if hr.0 == DRAGDROP_S_DROP {
        Ok(DragOutcome::from_effect(actual))
    } else if hr.0 == DRAGDROP_S_CANCEL {
        Ok(DragOutcome::Cancelled)
    } else {
        Err(shell_error("DoDragDrop", windows::core::Error::from_hresult(hr)))
    }
}

/// ole32.dll 的 `DoDragDrop` 原始绑定（与 windows crate 的 link! 声明同签名）。
///
/// 需要它的原因见 `begin_drag` 中的注释：windows crate 的 `DoDragDrop` 要求
/// `Param<IDropSource>`，本模块手写的 COM 对象不满足该 bound。
///
/// # Safety
/// `data` / `drop_source` 必须是有效的 COM 接口指针，`actual` 是可写出参。
unsafe fn do_drag_drop_raw(
    data: &IDataObject,
    drop_source: *mut std::ffi::c_void,
    allowed: DropEffect,
    actual: &mut DropEffect,
) -> windows::core::HRESULT {
    windows::core::link!("ole32.dll" "system" fn DoDragDrop(
        pdataobj: *mut std::ffi::c_void,
        pdropsource: *mut std::ffi::c_void,
        dwokeffects: u32,
        pdweffect: *mut u32,
    ) -> i32);
    // SAFETY: 调用方保证三个指针有效；allowed/actual 为 u32 值语义。
    let hr = unsafe {
        DoDragDrop(
            data as *const IDataObject as *mut std::ffi::c_void,
            drop_source,
            allowed,
            actual as *mut DropEffect,
        )
    };
    windows::core::HRESULT(hr)
}

/// 往 IDataObject 里塞 CFSTR_PREFERREDDROPEFFECT（HGLOBAL，DWORD）。
///
/// `IDataObject::SetData` 在 windows crate 里 gate 在
/// `Win32_System_Com_StructuredStorage` 特性后（该特性未启用），
/// 因此这里直接按 COM ABI 调用 vtable 第 8 个槽位（索引 7）。
fn set_preferred_drop_effect(data: &IDataObject, effect: u32) -> QResult<()> {
    use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
    use windows::Win32::System::Com::TYMED_HGLOBAL;
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    // SAFETY: CFSTR_PREFERREDDROPEFFECT 是静态宽串常量（在 UI::Shell 模块）。
    let cf = unsafe { RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT) };
    if cf == 0 {
        return Err(io_error("RegisterClipboardFormatW"));
    }

    // SAFETY: GMEM_MOVEABLE 合法。
    let hglobal = unsafe { GlobalAlloc(GMEM_MOVEABLE, 4) }
        .map_err(|e| shell_error("GlobalAlloc", e))?;
    // SAFETY: hglobal 刚分配，lock/unlock 配对。
    let ptr = unsafe { GlobalLock(hglobal) } as *mut u32;
    if ptr.is_null() {
        // SAFETY: hglobal 归本函数所有。
        let _ = unsafe { GlobalFree(Some(hglobal)) };
        return Err(io_error("GlobalLock"));
    }
    // SAFETY: 指针来自 GlobalLock，长度 4 字节由 GlobalAlloc 保证。
    unsafe { ptr.write(effect) };
    // SAFETY: 与上面的 GlobalLock 配对。
    let _ = unsafe { GlobalUnlock(hglobal) };

    let fmt = FORMATETC {
        cfFormat: cf as u16,
        ptd: std::ptr::null_mut(),
        dwAspect: 1, // DVASPECT_CONTENT
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };

    // STGMEDIUM 手工布局：{ tymed: u32(+4 padding), union { hGlobal: HANDLE },
    // pUnkForRelease: *mut IUnknown }。x64 上共 24 字节。
    // fRelease=true：SetData 失败时由 shell 释放 medium（符合 OLE 语义）。
    let mut medium = [0u64; 3];
    medium[0] = TYMED_HGLOBAL.0 as u64;
    // SAFETY: HGLOBAL 指针写入 union 首 8 字节。
    unsafe {
        std::ptr::copy_nonoverlapping(
            &hglobal as *const HGLOBAL as *const u64,
            medium.as_mut_ptr().add(1),
            1,
        );
    }

    // IDataObject vtable: [0..2]=IUnknown, [3]=GetData, [4]=GetDataHere,
    // [5]=QueryGetData, [6]=GetCanonicalFormatEtc, [7]=SetData, ...
    type SetDataFn = unsafe extern "system" fn(
        *mut std::ffi::c_void,
        *const FORMATETC,
        *mut std::ffi::c_void,
        i32,
    ) -> i32;
    // SAFETY: data 为有效 IDataObject；vtable 布局如上注释所述。
    let hr = unsafe {
        let vtable = *(data as *const IDataObject as *const *const *const std::ffi::c_void);
        let set_data: SetDataFn = std::mem::transmute(*vtable.add(7));
        set_data(
            data as *const IDataObject as *mut std::ffi::c_void,
            &fmt,
            medium.as_mut_ptr() as *mut std::ffi::c_void,
            1, // fRelease = TRUE
        )
    };
    if hr < 0 {
        // SetData 失败且 fRelease=true 时 shell 已接管 medium；此处无需再释放。
        return Err(shell_error(
            "IDataObject::SetData",
            windows::core::Error::from_hresult(windows::core::HRESULT(hr)),
        ));
    }
    Ok(())
}

/// 注册 OLE 拖放目标（窗口级）。
///
/// 上层若想拿到完整 `IDataObject`（而不只是文件列表）时使用；
/// 注册后窗口过程会收到 `IDropTarget::DragEnter/Over/Drop` 回调，
/// 需要上层提供手写 COM 对象（同 [`SimpleDropSource`] 的理由）。
/// 只需文件列表时用 [`accept_drop_files`] 的 WM_DROPFILES 路径即可。
///
/// `target_raw` 是 `IDropTarget*` 原始指针（COM ABI）。
pub fn register_drop_target(hwnd: HostWindow, target_raw: *mut std::ffi::c_void) -> QResult<()> {
    // SAFETY: hwnd 有效，target_raw 由调用方保证是有效 IDropTarget 指针。
    // 同样避开 `Param<IDropTarget>` bound（见 do_drag_drop_raw 的注释）。
    windows::core::link!("ole32.dll" "system" fn RegisterDragDrop(
        hwnd: *mut std::ffi::c_void,
        pdroptarget: *mut std::ffi::c_void,
    ) -> i32);
    let hr = unsafe {
        RegisterDragDrop(
            hwnd.hwnd().0 as *mut std::ffi::c_void,
            target_raw,
        )
    };
    if hr < 0 {
        return Err(shell_error(
            "RegisterDragDrop",
            windows::core::Error::from_hresult(windows::core::HRESULT(hr)),
        ));
    }
    Ok(())
}

/// 反注册 OLE 拖放目标。
pub fn revoke_drop_target(hwnd: HostWindow) -> QResult<()> {
    // SAFETY: hwnd 为之前注册过的窗口。同样用原始绑定（见 register_drop_target）。
    windows::core::link!("ole32.dll" "system" fn RevokeDragDrop(hwnd: *mut std::ffi::c_void) -> i32);
    let hr = unsafe { RevokeDragDrop(hwnd.hwnd().0 as *mut std::ffi::c_void) };
    if hr < 0 {
        return Err(shell_error(
            "RevokeDragDrop",
            windows::core::Error::from_hresult(windows::core::HRESULT(hr)),
        ));
    }
    Ok(())
}

/// 便捷：判断拖放效果是否允许复制。
pub fn allows_copy(effect: DropEffect) -> bool {
    effect & DROPEFFECT_COPY_VALUE != 0
}

/// 便捷：判断拖放效果是否允许移动。
pub fn allows_move(effect: DropEffect) -> bool {
    effect & DROPEFFECT_MOVE_VALUE != 0
}

// ============ IDropSource 最小实现（手写 COM vtable） ============
//
// 为什么不用 `#[windows::core::implement]`：该宏生成的代码引用 `::windows_core::`
// 绝对路径，而 windows-core 不是本 crate 的直接依赖（Cargo.toml 只有 windows），
// 无法通过 `extern crate windows_core` 补上。故手写 vtable —— 逻辑很短
// （IUnknown 三方法 + IDropSource 两方法），手工实现比改依赖清单更稳妥。
//
// IDropSource vtable 布局（x64）：
//   [0] QueryInterface  [1] AddRef  [2] Release
//   [3] QueryContinueDrag  [4] GiveFeedback

/// `DRAGDROP_S_DROP`（0x00040100）。
const DRAGDROP_S_DROP: i32 = 0x00040100;
/// `DRAGDROP_S_CANCEL`（0x00040101）。
const DRAGDROP_S_CANCEL: i32 = 0x00040101;
/// `S_OK`。
const S_OK: i32 = 0;
/// `S_FALSE`。
const S_FALSE: i32 = 1;

/// IDropSource 的 IID：{00000121-0000-0000-C000-000000000046}。
const IID_IDROPSOURCE: [u8; 16] = [
    0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

/// 最小 `IDropSource`：Esc 取消、左键松开放下、其余继续。
pub struct SimpleDropSource {
    /// 指向 vtable 的指针（COM 对象首字段约定）。
    ///
    /// Rust 侧不直接读它 —— COM 调用方通过对象首 8 字节取 vtable。
    /// 字段必须存在且为第一个字段（COM ABI 布局要求）。
    #[allow(dead_code)]
    vtable: *const DropSourceVtbl,
    /// 引用计数（COM 约定从 1 开始）。
    refcount: std::cell::Cell<u32>,
}

/// IDropSource 的 vtable（`#[repr(C)]`，方法顺序即 COM ABI 顺序）。
#[repr(C)]
struct DropSourceVtbl {
    query_interface: unsafe extern "system" fn(*mut SimpleDropSource, *const [u8; 16], *mut *mut std::ffi::c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut SimpleDropSource) -> u32,
    release: unsafe extern "system" fn(*mut SimpleDropSource) -> u32,
    query_continue_drag: unsafe extern "system" fn(*mut SimpleDropSource, i32, u32) -> i32,
    give_feedback: unsafe extern "system" fn(*mut SimpleDropSource, u32) -> i32,
}

/// 静态 vtable 实例（所有 SimpleDropSource 共享）。
static DROP_SOURCE_VTABLE: DropSourceVtbl = DropSourceVtbl {
    query_interface: ds_query_interface,
    add_ref: ds_add_ref,
    release: ds_release,
    query_continue_drag: ds_query_continue_drag,
    give_feedback: ds_give_feedback,
};

impl SimpleDropSource {
    /// 构造（引用计数初值 1，符合 COM 创建语义）。
    pub fn new() -> Box<Self> {
        Box::new(SimpleDropSource {
            vtable: &DROP_SOURCE_VTABLE,
            refcount: std::cell::Cell::new(1),
        })
    }

    /// 取 COM 接口指针（供 `DoDragDrop` 使用）。
    ///
    /// 返回的指针由 Box 拥有；调用方不得单独释放，须整体 drop Box。
    pub fn as_interface_ptr(&mut self) -> *mut std::ffi::c_void {
        self as *mut SimpleDropSource as *mut std::ffi::c_void
    }
}

impl Default for SimpleDropSource {
    fn default() -> Self {
        Self::new_owned()
    }
}

impl SimpleDropSource {
    /// 非 Box 构造（供栈上/嵌入使用；调用方负责生命周期）。
    fn new_owned() -> Self {
        SimpleDropSource {
            vtable: &DROP_SOURCE_VTABLE,
            refcount: std::cell::Cell::new(1),
        }
    }
}

/// `QueryInterface`：只认 IDropSource 与 IUnknown（同一指针）。
unsafe extern "system" fn ds_query_interface(
    this: *mut SimpleDropSource,
    iid: *const [u8; 16],
    out: *mut *mut std::ffi::c_void,
) -> i32 {
    // SAFETY: this/iid/out 由 COM 调用方保证有效。
    unsafe {
        if iid.is_null() || out.is_null() {
            return -2147024809; // E_POINTER
        }
        let iid = &*iid;
        // IUnknown 的 IID 是全零；IDropSource 的 IID 见 IID_IDROPSOURCE。
        let is_unknown = iid == &[0u8; 16];
        let is_drop_source = iid == &IID_IDROPSOURCE;
        if is_unknown || is_drop_source {
            *out = this as *mut std::ffi::c_void;
            ds_add_ref(this);
            0 // S_OK
        } else {
            *out = std::ptr::null_mut();
            -2147467262 // E_NOINTERFACE
        }
    }
}

/// `AddRef`。
unsafe extern "system" fn ds_add_ref(this: *mut SimpleDropSource) -> u32 {
    // SAFETY: this 由 COM 调用方保证有效。
    unsafe {
        let this = &*this;
        let n = this.refcount.get() + 1;
        this.refcount.set(n);
        n
    }
}

/// `Release`：引用归零时不销毁（Box 拥有对象，Drop 统一处理）。
unsafe extern "system" fn ds_release(this: *mut SimpleDropSource) -> u32 {
    // SAFETY: this 由 COM 调用方保证有效。
    unsafe {
        let this = &*this;
        let n = this.refcount.get().saturating_sub(1);
        this.refcount.set(n);
        n
    }
}

/// `QueryContinueDrag`：Esc → 取消；左键松开 → 放下；否则继续。
unsafe extern "system" fn ds_query_continue_drag(
    this: *mut SimpleDropSource,
    escape_pressed: i32,
    key_state: u32,
) -> i32 {
    let _ = this;
    if escape_pressed != 0 {
        return DRAGDROP_S_CANCEL;
    }
    // MK_LBUTTON = 0x0001；位清零表示已松开。
    if key_state & 0x0001 == 0 {
        return DRAGDROP_S_DROP;
    }
    S_OK
}

/// `GiveFeedback`：返回 S_FALSE 让系统使用默认拖放光标。
unsafe extern "system" fn ds_give_feedback(_this: *mut SimpleDropSource, _effect: u32) -> i32 {
    S_FALSE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_files_rejects_null_handle() {
        assert!(DropFiles::from_raw(0).is_err());
    }

    #[test]
    fn drag_outcome_mapping() {
        assert_eq!(DragOutcome::from_effect(DROPEFFECT_COPY_VALUE), DragOutcome::Copy);
        assert_eq!(DragOutcome::from_effect(DROPEFFECT_MOVE_VALUE), DragOutcome::Move);
        assert_eq!(DragOutcome::from_effect(0), DragOutcome::None);
        assert_eq!(DragOutcome::from_effect(0x10), DragOutcome::Cancelled);
    }

    #[test]
    fn effect_helpers() {
        assert!(allows_copy(DROPEFFECT_COPY_VALUE));
        assert!(!allows_copy(DROPEFFECT_MOVE_VALUE));
        assert!(allows_move(DROPEFFECT_MOVE_VALUE));
        assert!(!allows_move(DROPEFFECT_COPY_VALUE));
    }

    #[test]
    fn begin_drag_with_empty_list_is_none() {
        let r = begin_drag(HostWindow::from_raw_hwnd(0), &[], false);
        assert_eq!(r.expect("空列表不应触达 COM"), DragOutcome::None);
    }

    #[test]
    fn begin_drag_rejects_nul_path() {
        let r = begin_drag(
            HostWindow::from_raw_hwnd(0),
            &[PathBuf::from("C:\u{0}bad")],
            false,
        );
        assert!(r.is_err());
    }

    #[test]
    fn drop_target_helpers() {
        let t = DropTarget::new(10, 20, 0x30);
        assert_eq!(t.screen_x, 10);
        assert_eq!(t.screen_y, 20);
        assert_eq!(t.hwnd_raw, 0x30);
    }

    #[test]
    fn constants_match_win32() {
        assert_eq!(DRAGDROP_S_DROP, 0x00040100);
        assert_eq!(DRAGDROP_S_CANCEL, 0x00040101);
        assert_eq!(DROPEFFECT_COPY_VALUE, 1);
        assert_eq!(DROPEFFECT_MOVE_VALUE, 2);
    }
}
