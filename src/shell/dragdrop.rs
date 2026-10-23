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
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Ole::CF_HDROP;
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
pub const DROPEFFECT_COPY_VALUE: DropEffect = 1;
/// `DROPEFFECT_MOVE`（移动/剪切）。
pub const DROPEFFECT_MOVE_VALUE: DropEffect = 2;
/// `MK_CONTROL`（拖放按键决议用）。
const MK_CONTROL_VALUE: u32 = 0x0008;
/// `MK_SHIFT`（拖放按键决议用）。
const MK_SHIFT_VALUE: u32 = 0x0004;

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
/// `cut` 对应 `copy_cut_paste_mode = 1`（剪切语义）。薄包装：单值效果
/// （cut→MOVE/MOVE，否则 COPY/COPY）；双许可 + 按键 preferred 见
/// [`begin_drag_ex`]。
pub fn begin_drag(hwnd: HostWindow, paths: &[PathBuf], cut: bool) -> QResult<DragOutcome> {
    let e = if cut { DROPEFFECT_MOVE_VALUE } else { DROPEFFECT_COPY_VALUE };
    begin_drag_ex(hwnd, paths, e, e)
}

/// 拖出（批次B/B2）：`allowed`=dwokeffects（可给 COPY|MOVE 双许可），
/// `preferred`=Preferred DropEffect（目标按此决议光标与默认动作）。
///
/// 在 CIDLData 之外额外 `SetData` 一份 CF_HDROP，让本进程 IDropTarget
/// 与任何 CF_HDROP 阅读器（Explorer/剪贴板链）都能直接取到路径——
/// 原 CIDLData 只提供 CFSTR_SHELLIDLIST。
pub fn begin_drag_ex(
    hwnd: HostWindow,
    paths: &[PathBuf],
    allowed: DropEffect,
    preferred: DropEffect,
) -> QResult<DragOutcome> {
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
    if let Err(e) = set_preferred_drop_effect(&data, preferred) {
        for q in &pidls {
            // SAFETY: 归本函数所有。
            unsafe { free_pidl(*q) };
        }
        return Err(e);
    }

    // 批次B/B2：附加 CF_HDROP（DROPFILES：20 字节头 + 双 NUL 宽串数组），
    // 让 CF_HDROP 阅读器（含本进程 PaneDropTarget）直接取路径。
    if let Err(e) = set_cf_hdrop(&data, paths) {
        for q in &pidls {
            // SAFETY: 归本函数所有。
            unsafe { free_pidl(*q) };
        }
        return Err(e);
    }

    // SimpleDropSource 是手写 COM 对象；Box 拥有它，接口指针在拖放期间有效。
    let mut source = SimpleDropSource::new();
    let source_ptr = source.as_interface_ptr();
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
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

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

    // SAFETY: CFSTR_PREFERREDDROPEFFECT 是静态宽串常量（在 UI::Shell 模块）。
    let cf = unsafe { RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT) };
    if cf == 0 {
        // SAFETY: hglobal 归本函数所有。
        let _ = unsafe { GlobalFree(Some(hglobal)) };
        return Err(io_error("RegisterClipboardFormatW"));
    }
    data_object_set_hglobal(data, cf, hglobal)
}

/// 往 IDataObject 里塞一份 CF_HDROP（批次B/B2）：DROPFILES 20 字节头
/// （pFiles=20 / fWide=1）+ 各路径宽串 + 双 NUL 结尾。句柄用
/// GMEM_MOVEABLE|GMEM_ZEROINIT（SetData 失败时由本函数释放）。
fn set_cf_hdrop(data: &IDataObject, paths: &[PathBuf]) -> QResult<()> {
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
    };

    let mut wides: Vec<u16> = Vec::new();
    for p in paths {
        let w = path_to_wide(p)?;
        wides.extend_from_slice(&w);
    }
    wides.push(0); // 列表结束的额外 NUL

    // DROPFILES { pFiles: u32, pt: POINT, fNC: BOOL, fWide: BOOL } = 20 字节。
    let header_size = 20usize;
    let total = header_size + wides.len() * 2;
    // SAFETY: GMEM_MOVEABLE|GMEM_ZEROINIT 合法。
    let hglobal = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, total) }
        .map_err(|e| shell_error("GlobalAlloc", e))?;
    {
        // SAFETY: hglobal 刚分配；lock/unlock 配对。
        let ptr = unsafe { GlobalLock(hglobal) } as *mut u8;
        if ptr.is_null() {
            // SAFETY: hglobal 归本函数所有。
            let _ = unsafe { GlobalFree(Some(hglobal)) };
            return Err(io_error("GlobalLock"));
        }
        // SAFETY: 区域大小由 GlobalAlloc 保证为 total 字节。
        unsafe {
            let p_files = ptr as *mut u32;
            p_files.write(header_size as u32);
            let f_wide = ptr.add(16) as *mut u32;
            f_wide.write(1);
            let data = ptr.add(header_size) as *mut u16;
            std::ptr::copy_nonoverlapping(wides.as_ptr(), data, wides.len());
        }
        // SAFETY: 与上面的 GlobalLock 配对。
        let _ = unsafe { GlobalUnlock(hglobal) };
    }
    data_object_set_hglobal(data, CF_HDROP.0 as u32, hglobal)
}

/// `IDataObject::SetData` 的 HGLOBAL 变体（vtable 槽 7；fRelease=TRUE）。
/// 失败时句柄由本函数释放（SetData 未接管）。`cf` 为 RegisterClipboardFormatW
/// 返回的 u32 格式号。
fn data_object_set_hglobal(data: &IDataObject, cf: u32, hglobal: windows::Win32::Foundation::HGLOBAL) -> QResult<()> {
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::Com::TYMED_HGLOBAL;

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
            &hglobal as *const windows::Win32::Foundation::HGLOBAL as *const u64,
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
        // SetData 失败且 fRelease=true 时 shell 已接管 medium；但按 MSDN，
        // 失败时调用方仍应自行释放（fRelease 语义是「成功时」）。保守：释放。
        // SAFETY: hglobal 归本函数所有。
        let _ = unsafe { GlobalFree(Some(hglobal)) };
        return Err(shell_error(
            "IDataObject::SetData",
            windows::core::Error::from_hresult(windows::core::HRESULT(hr)),
        ));
    }
    Ok(())
}

/// `IDataObject::GetData`（vtable 槽 3）：取指定 clipboard 格式的 HGLOBAL。
///
/// `cf` 为 RegisterClipboardFormatW 返回的 u32 格式号。返回的 HGLOBAL 由
/// 数据对象/medium 拥有；读完后按 punkForRelease 为空的情况由调用方
/// `GlobalFree`（TYMED_HGLOBAL 语义）。
fn data_object_get_hglobal(data: &IDataObject, cf: u32) -> Option<windows::Win32::Foundation::HGLOBAL> {
    use windows::Win32::System::Com::TYMED_HGLOBAL;

    let fmt = FORMATETC {
        cfFormat: cf as u16,
        ptd: std::ptr::null_mut(),
        dwAspect: 1,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let mut medium = [0u64; 3];
    type GetDataFn = unsafe extern "system" fn(
        *mut std::ffi::c_void,
        *const FORMATETC,
        *mut std::ffi::c_void,
    ) -> i32;
    // SAFETY: data 为有效 IDataObject；medium 为出参。
    let hr = unsafe {
        let vtable = *(data as *const IDataObject as *const *const *const std::ffi::c_void);
        let get_data: GetDataFn = std::mem::transmute(*vtable.add(3));
        get_data(
            data as *const IDataObject as *mut std::ffi::c_void,
            &fmt,
            medium.as_mut_ptr() as *mut std::ffi::c_void,
        )
    };
    if hr < 0 {
        return None;
    }
    // medium[1] = union 首 8 字节 = hGlobal（punkForRelease=medium[2]=null）。
    if medium[2] != 0 {
        return None; // 非常规 medium，不处理
    }
    Some(windows::Win32::Foundation::HGLOBAL(medium[1] as *mut std::ffi::c_void))
}

/// 从数据对象读 CF_HDROP 文件列表（GetData → DROPFILES 解析）。
/// 读取后释放 medium（GlobalFree；pUnkForRelease 为空）。
pub fn paths_from_data_object(data: &IDataObject) -> QResult<Vec<PathBuf>> {
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    let Some(hglobal) = data_object_get_hglobal(data, CF_HDROP.0 as u32) else {
        return Err(QError::Shell("数据对象中没有 CF_HDROP".to_string()));
    };
    // SAFETY: hglobal 由数据对象分配，只读。
    let ptr = unsafe { GlobalLock(hglobal) } as *const u8;
    if ptr.is_null() {
        // SAFETY: hglobal 归本函数所有（medium 未设 pUnkForRelease）。
        let _ = unsafe { GlobalFree(Some(hglobal)) };
        return Err(io_error("GlobalLock"));
    }
    // SAFETY: ptr 指向 DROPFILES 结构。
    let p_files = unsafe { *(ptr as *const u32) } as usize;
    let f_wide = unsafe { *(ptr.add(16) as *const u32) } != 0;
    let base = unsafe { ptr.add(p_files) };
    let paths = if f_wide {
        read_double_nul_wide(base as *const u16)
    } else {
        read_double_nul_narrow(base)
    };
    // SAFETY: 与 GlobalLock 配对。
    let _ = unsafe { GlobalUnlock(hglobal) };
    // SAFETY: medium 未设 pUnkForRelease，调用方负责释放。
    let _ = unsafe { GlobalFree(Some(hglobal)) };
    Ok(paths)
}

/// 读双 NUL 结尾的宽串列表（DROPFILES 数据区）。
fn read_double_nul_wide(base: *const u16) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut cur = base;
    loop {
        // SAFETY: 剪贴板内存以双 NUL 结束；逐个读到 NUL。
        let len = unsafe { wide_strlen(cur) };
        if len == 0 {
            break;
        }
        // SAFETY: len 由 strlen 保证在缓冲内。
        let slice = unsafe { std::slice::from_raw_parts(cur, len) };
        out.push(PathBuf::from(String::from_utf16_lossy(slice)));
        // SAFETY: 前进到下一个串（跳过 NUL）。
        cur = unsafe { cur.add(len + 1) };
    }
    out
}

/// 读双 NUL 结尾的窄串列表。
fn read_double_nul_narrow(base: *const u8) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut cur = base;
    loop {
        // SAFETY: 同上（窄串版本）。
        let len = unsafe { narrow_strlen(cur) };
        if len == 0 {
            break;
        }
        // SAFETY: len 保证在缓冲内。
        let slice = unsafe { std::slice::from_raw_parts(cur, len) };
        out.push(PathBuf::from(String::from_utf8_lossy(slice).to_string()));
        // SAFETY: 前进到下一个串（跳过 NUL）。
        cur = unsafe { cur.add(len + 1) };
    }
    out
}

/// 宽串长度（到 NUL 为止）。
///
/// # Safety
/// `s` 必须指向以 NUL 结尾的宽串。
unsafe fn wide_strlen(s: *const u16) -> usize {
    let mut i = 0usize;
    // SAFETY: 调用方保证终止条件可达。
    while unsafe { *s.add(i) } != 0 {
        i += 1;
    }
    i
}

/// 窄串长度（到 NUL 为止）。
///
/// # Safety
/// `s` 必须指向以 NUL 结尾的窄串。
unsafe fn narrow_strlen(s: *const u8) -> usize {
    let mut i = 0usize;
    // SAFETY: 调用方保证终止条件可达。
    while unsafe { *s.add(i) } != 0 {
        i += 1;
    }
    i
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

// ============ 批次B：拖放落点（IDropTarget 手写 COM） ============

/// 拖放落点投递项：COM 回调（Drop）只入队，主线程消息循环消费——
/// 绝不在 COM 回调里直接操作 UI（规格 B3 关键设计）。
#[derive(Clone, Debug)]
pub struct DroppedPaths {
    /// 目标窗格索引（注册时绑定）。
    pub pane_index: usize,
    /// 拖入的文件列表（CF_HDROP 解析）。
    pub paths: Vec<PathBuf>,
    /// 决议效果（DROPEFFECT_COPY/MOVE）。
    pub effect: u32,
    /// 落点屏幕坐标。
    pub screen_pt: (i32, i32),
}

thread_local! {
    /// 拖放投递队列（UI 线程生产消费；线程局部避免锁）。
    static DROP_QUEUE: std::cell::RefCell<Vec<DroppedPaths>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// 入队一条拖放投递（COM Drop 回调调用）。
pub fn push_dropped(d: DroppedPaths) {
    DROP_QUEUE.with(|q| q.borrow_mut().push(d));
}

/// 取走全部待处理投递（主线程 WM_APP+2 处理调用）。
pub fn take_dropped() -> Vec<DroppedPaths> {
    DROP_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))
}

/// 按键 → 拖放效果决议（纯函数）：
/// Ctrl→COPY；Shift→MOVE；默认取 preferred（数据对象带的
/// CFSTR_PREFERREDDROPEFFECT；无则 MOVE——Q-Dir 窗格间默认移动）。
/// 结果与 `allowed` 取交（allowed=0 视为不限制）。
pub fn resolve_drop_effect(allowed: u32, key_state: u32, preferred: Option<u32>) -> u32 {
    let want = if key_state & MK_CONTROL_VALUE != 0 {
        DROPEFFECT_COPY_VALUE
    } else if key_state & MK_SHIFT_VALUE != 0 {
        DROPEFFECT_MOVE_VALUE
    } else {
        preferred.unwrap_or(DROPEFFECT_MOVE_VALUE)
    };
    if allowed == 0 {
        want
    } else {
        want & allowed
    }
}

/// 拖放落点拒绝理由（纯函数；供上层在执行前预检）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropReject {
    /// 拖入列表为空。
    Empty,
    /// 目标目录为空（窗格未导航）。
    NoDest,
    /// 源与目标同目录。
    SameDir,
    /// 目录拖入自身/自身子目录（自拖）。
    SelfDrop,
}

/// 同目录比较（Windows 卷大小写无关；忽略尾部分隔符）。
fn same_dir_ci(a: &std::path::Path, b: &std::path::Path) -> bool {
    let sa = a
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();
    let sb = b
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();
    !sa.is_empty() && sa == sb
}

/// 拖放落点拒绝判定（纯函数）：空列表/目标为空/同目录/自拖（目录拖进
/// 自身或其子目录）→ Some(reason)；否则 None（可执行）。
pub fn drop_reject_reason(paths: &[PathBuf], dest: &std::path::Path) -> Option<DropReject> {
    if paths.is_empty() {
        return Some(DropReject::Empty);
    }
    if dest.as_os_str().is_empty() {
        return Some(DropReject::NoDest);
    }
    for p in paths {
        // 目录拖到自身。
        if p == dest {
            return Some(DropReject::SelfDrop);
        }
        // 源所在目录 == 目标目录。
        if let Some(src_dir) = p.parent() {
            if same_dir_ci(src_dir, dest) {
                return Some(DropReject::SameDir);
            }
        }
        // 目录拖进自身子目录（父→子循环）。
        if dest.starts_with(p) && p != dest {
            return Some(DropReject::SelfDrop);
        }
    }
    None
}

// ---------- PaneDropTarget：手写 IDropTarget vtable ----------
//
// 布局（x64）：
//   [0] QueryInterface  [1] AddRef  [2] Release
//   [3] DragEnter  [4] DragOver  [5] DragLeave  [6] Drop
//
// 线程模型：DoDragDrop 模态在本 UI 线程运行（自拖与 Explorer 拖入的
// IDropTarget 回调都发生在 UI 线程）。DragEnter/DragOver 只写
// *pdwEffect；Drop 只入队 + PostMessage(WM_APP+2) 唤醒主线程。

/// IDropTarget 的 IID：{00000122-0000-0000-C000-000000000046}。
const IID_IDROPTARGET: [u8; 16] = [
    0x22, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46,
];

/// 拖放自定义消息（WM_APP+2；主框架消费队列）。
const WM_APP_DROP_MSG: u32 = 0x8000 + 2;

/// 窗格 OLE 落点：绑定 pane_index + 主窗句柄；缓存 DragEnter 读到的
/// CF_HDROP 路径与 Preferred DropEffect 供 DragOver 复用。
///
/// `#[repr(C)]` 是 COM ABI 硬要求：vtable 指针必须在对象首 8 字节
/// （默认 Rust repr 字段可重排 → COM 调用方段错误）。
#[repr(C)]
pub struct PaneDropTarget {
    /// 指向 vtable 的指针（COM 对象首字段约定；不得移动对象）。
    vtable: *const DropTargetVtbl,
    /// 引用计数（COM 约定从 1 开始）。
    refcount: std::cell::Cell<u32>,
    /// 目标窗格索引。
    pane_index: usize,
    /// 主窗句柄（Drop 后 PostMessage 唤醒）。
    main_hwnd: isize,
    /// DragEnter 缓存的拖入路径（None=当前数据无 CF_HDROP）。
    cached_paths: std::cell::RefCell<Option<Vec<PathBuf>>>,
    /// DragEnter 缓存的 Preferred DropEffect（None=无）。
    preferred: std::cell::Cell<Option<u32>>,
}

/// IDropTarget 的 vtable（`#[repr(C)]`，方法顺序即 COM ABI 顺序）。
#[repr(C)]
struct DropTargetVtbl {
    query_interface: unsafe extern "system" fn(*mut PaneDropTarget, *const [u8; 16], *mut *mut std::ffi::c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut PaneDropTarget) -> u32,
    release: unsafe extern "system" fn(*mut PaneDropTarget) -> u32,
    drag_enter: unsafe extern "system" fn(*mut PaneDropTarget, *mut std::ffi::c_void, u32, windows::Win32::Foundation::POINT, *mut u32) -> i32,
    drag_over: unsafe extern "system" fn(*mut PaneDropTarget, u32, windows::Win32::Foundation::POINT, *mut u32) -> i32,
    drag_leave: unsafe extern "system" fn(*mut PaneDropTarget) -> i32,
    drop: unsafe extern "system" fn(*mut PaneDropTarget, *mut std::ffi::c_void, u32, windows::Win32::Foundation::POINT, *mut u32) -> i32,
}

/// 静态 vtable 实例（所有 PaneDropTarget 共享）。
static DROP_TARGET_VTABLE: DropTargetVtbl = DropTargetVtbl {
    query_interface: dt_query_interface,
    add_ref: dt_add_ref,
    release: dt_release,
    drag_enter: dt_drag_enter,
    drag_over: dt_drag_over,
    drag_leave: dt_drag_leave,
    drop: dt_drop,
};

impl PaneDropTarget {
    /// 构造（引用计数初值 1，符合 COM 创建语义）。
    pub fn new(pane_index: usize, main_hwnd: isize) -> Box<Self> {
        Box::new(PaneDropTarget {
            vtable: &DROP_TARGET_VTABLE,
            refcount: std::cell::Cell::new(1),
            pane_index,
            main_hwnd,
            cached_paths: std::cell::RefCell::new(None),
            preferred: std::cell::Cell::new(None),
        })
    }

    /// 取 COM 接口指针（供 `RegisterDragDrop` 使用）。
    ///
    /// 对象必须已置于稳定地址（调用方先存入 frame 再取指针注册）。
    pub fn as_interface_ptr(&self) -> *mut std::ffi::c_void {
        self as *const PaneDropTarget as *mut PaneDropTarget as *mut std::ffi::c_void
    }
}

/// `QueryInterface`：只认 IDropTarget 与 IUnknown（同一指针）。
unsafe extern "system" fn dt_query_interface(
    this: *mut PaneDropTarget,
    iid: *const [u8; 16],
    out: *mut *mut std::ffi::c_void,
) -> i32 {
    // SAFETY: this/iid/out 由 COM 调用方保证有效。
    unsafe {
        if iid.is_null() || out.is_null() {
            return -2147024809; // E_POINTER
        }
        let iid = &*iid;
        let is_unknown = iid == &[0u8; 16];
        let is_drop_target = iid == &IID_IDROPTARGET;
        if is_unknown || is_drop_target {
            *out = this as *mut std::ffi::c_void;
            dt_add_ref(this);
            0 // S_OK
        } else {
            *out = std::ptr::null_mut();
            -2147467262 // E_NOINTERFACE
        }
    }
}

/// `AddRef`。
unsafe extern "system" fn dt_add_ref(this: *mut PaneDropTarget) -> u32 {
    // SAFETY: this 由 COM 调用方保证有效。
    unsafe {
        let this = &*this;
        let n = this.refcount.get() + 1;
        this.refcount.set(n);
        n
    }
}

/// `Release`：引用归零时不销毁（Box 拥有对象，由 frame 统一释放）。
unsafe extern "system" fn dt_release(this: *mut PaneDropTarget) -> u32 {
    // SAFETY: this 由 COM 调用方保证有效。
    unsafe {
        let this = &*this;
        let n = this.refcount.get().saturating_sub(1);
        this.refcount.set(n);
        n
    }
}

/// 从数据对象读 Preferred DropEffect（CFSTR_PREFERREDDROPEFFECT DWORD）。
unsafe fn read_preferred_from_data(data: &IDataObject) -> Option<u32> {
    use windows::Win32::Foundation::GlobalFree;
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    // SAFETY: CFSTR_PREFERREDDROPEFFECT 是静态宽串常量。
    let cf = unsafe { RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT) };
    if cf == 0 {
        return None;
    }
    let hglobal = data_object_get_hglobal(data, cf)?;
    // SAFETY: hglobal 由数据对象分配，只读。
    let ptr = unsafe { GlobalLock(hglobal) } as *const u32;
    let effect = if ptr.is_null() {
        None
    } else {
        // SAFETY: 4 字节由格式语义保证。
        Some(unsafe { ptr.read() })
    };
    // SAFETY: 配对。
    let _ = unsafe { GlobalUnlock(hglobal) };
    // SAFETY: medium 未设 pUnkForRelease，调用方释放。
    let _ = unsafe { GlobalFree(Some(hglobal)) };
    effect
}

/// 效果决议的公共逻辑：读/用缓存 preferred + 按键 → *pdwEffect。
/// 无 CF_HDROP 数据（cached_paths 空）→ 0（拒绝）。
unsafe fn dt_update_effect(this: &PaneDropTarget, key_state: u32, out: *mut u32) -> i32 {
    let has_files = this.cached_paths.borrow().is_some();
    let resolved = if has_files {
        resolve_drop_effect(
            DROPEFFECT_COPY_VALUE | DROPEFFECT_MOVE_VALUE,
            key_state,
            this.preferred.get(),
        )
    } else {
        0
    };
    // SAFETY: out 由 COM 调用方保证可写。
    unsafe { *out = resolved };
    0 // S_OK
}

/// `DragEnter`：读 CF_HDROP 路径与 Preferred DropEffect 缓存，写效果。
unsafe extern "system" fn dt_drag_enter(
    this: *mut PaneDropTarget,
    data_obj: *mut std::ffi::c_void,
    key_state: u32,
    _pt: windows::Win32::Foundation::POINT,
    effect: *mut u32,
) -> i32 {
    // SAFETY: this/data_obj/effect 由 COM 调用方保证有效。
    let this = unsafe { &*this };
    if data_obj.is_null() {
        this.cached_paths.borrow_mut().take();
        this.preferred.set(None);
        return unsafe { dt_update_effect(this, key_state, effect) };
    }
    let data = unsafe { &*(data_obj as *const IDataObject) };
    // CF_HDROP 路径（读一次缓存；Drop 复用）。
    let paths = paths_from_data_object(data).ok();
    this.cached_paths.borrow_mut().take();
    if paths.as_ref().map(|p| !p.is_empty()).unwrap_or(false) {
        *this.cached_paths.borrow_mut() = paths;
    }
    // Preferred DropEffect（自拖 begin_drag_ex 有写；Explorer 也有）。
    this.preferred.set(unsafe { read_preferred_from_data(data) });
    unsafe { dt_update_effect(this, key_state, effect) }
}

/// `DragOver`：按键可能变化，按缓存重算效果（不重读数据对象）。
unsafe extern "system" fn dt_drag_over(
    this: *mut PaneDropTarget,
    key_state: u32,
    _pt: windows::Win32::Foundation::POINT,
    effect: *mut u32,
) -> i32 {
    // SAFETY: this/effect 由 COM 调用方保证有效。
    let this = unsafe { &*this };
    unsafe { dt_update_effect(this, key_state, effect) }
}

/// `DragLeave`：清缓存（无高亮可清，规格允许先只清队列/缓存）。
unsafe extern "system" fn dt_drag_leave(this: *mut PaneDropTarget) -> i32 {
    // SAFETY: this 由 COM 调用方保证有效。
    let this = unsafe { &*this };
    this.cached_paths.borrow_mut().take();
    this.preferred.set(None);
    0 // S_OK
}

/// `Drop`：取缓存路径 → 入队 + PostMessage(WM_APP+2) 唤醒主线程。
/// 绝不在此执行文件操作/UI（主线程消费）。
unsafe extern "system" fn dt_drop(
    this: *mut PaneDropTarget,
    data_obj: *mut std::ffi::c_void,
    key_state: u32,
    pt: windows::Win32::Foundation::POINT,
    effect: *mut u32,
) -> i32 {
    // SAFETY: this/data_obj/effect 由 COM 调用方保证有效。
    let this = unsafe { &*this };
    let mut paths = this.cached_paths.borrow_mut().take();
    if paths.is_none() && !data_obj.is_null() {
        let data = unsafe { &*(data_obj as *const IDataObject) };
        paths = paths_from_data_object(data).ok();
    }
    let resolved = unsafe { dt_update_effect(this, key_state, effect) };
    let Some(paths) = paths else {
        return 0; // 无文件数据：效果已置 0
    };
    if paths.is_empty() {
        return 0;
    }
    push_dropped(DroppedPaths {
        pane_index: this.pane_index,
        paths,
        effect: unsafe { *effect },
        screen_pt: (pt.x, pt.y),
    });
    // 唤醒主线程消息循环（同窗口 PostMessage 指针语义安全）。
    if this.main_hwnd != 0 {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                Some(windows::Win32::Foundation::HWND(this.main_hwnd as *mut core::ffi::c_void)),
                WM_APP_DROP_MSG,
                windows::Win32::Foundation::WPARAM(0),
                windows::Win32::Foundation::LPARAM(0),
            );
        }
    }
    resolved
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
///
/// `#[repr(C)]` 是 COM ABI 硬要求：vtable 指针必须在对象首 8 字节。
/// （默认 Rust repr 允许字段重排，COM 调用方会调错地址段错误。）
#[repr(C)]
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
    use std::path::Path;

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
    fn begin_drag_ex_empty_list_is_none() {
        // 双许可变体同样：空列表不触达 COM。
        let r = begin_drag_ex(
            HostWindow::from_raw_hwnd(0),
            &[],
            DROPEFFECT_COPY_VALUE | DROPEFFECT_MOVE_VALUE,
            DROPEFFECT_MOVE_VALUE,
        );
        assert_eq!(r.expect("空列表不应触达 COM"), DragOutcome::None);
    }

    #[test]
    fn begin_drag_ex_rejects_nul_path() {
        let r = begin_drag_ex(
            HostWindow::from_raw_hwnd(0),
            &[PathBuf::from("C:\u{0}bad")],
            DROPEFFECT_COPY_VALUE | DROPEFFECT_MOVE_VALUE,
            DROPEFFECT_COPY_VALUE,
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

    // ---------- 批次B：effect 决议 / 落点拒绝（纯函数） ----------

    #[test]
    fn effect_resolution_by_keys() {
        // Ctrl→COPY；Shift→MOVE；默认 preferred（有则从）。
        assert_eq!(resolve_drop_effect(0, MK_CONTROL_VALUE, None), DROPEFFECT_COPY_VALUE);
        assert_eq!(resolve_drop_effect(0, MK_SHIFT_VALUE, None), DROPEFFECT_MOVE_VALUE);
        assert_eq!(resolve_drop_effect(0, 0, Some(DROPEFFECT_COPY_VALUE)), DROPEFFECT_COPY_VALUE);
        assert_eq!(resolve_drop_effect(0, 0, Some(DROPEFFECT_MOVE_VALUE)), DROPEFFECT_MOVE_VALUE);
        // 无 preferred 默认 MOVE（Q-Dir 窗格间默认移动；*待实测复核*）。
        assert_eq!(resolve_drop_effect(0, 0, None), DROPEFFECT_MOVE_VALUE);
    }

    #[test]
    fn effect_resolution_intersects_allowed() {
        // 结果必须落在 allowed 内；不在 → 0（拒绝）。
        assert_eq!(
            resolve_drop_effect(DROPEFFECT_COPY_VALUE, 0, Some(DROPEFFECT_MOVE_VALUE)),
            0
        );
        assert_eq!(
            resolve_drop_effect(DROPEFFECT_COPY_VALUE | DROPEFFECT_MOVE_VALUE, MK_CONTROL_VALUE, None),
            DROPEFFECT_COPY_VALUE
        );
    }

    #[test]
    fn drop_reject_empty_and_no_dest() {
        assert_eq!(drop_reject_reason(&[], Path::new(r"C:\a")), Some(DropReject::Empty));
        assert_eq!(
            drop_reject_reason(&[PathBuf::from(r"C:\a\f.txt")], Path::new("")),
            Some(DropReject::NoDest)
        );
    }

    #[test]
    fn drop_reject_same_dir() {
        // 源就在目标目录里 → 同目录拒绝。
        assert_eq!(
            drop_reject_reason(&[PathBuf::from(r"C:\a\f.txt")], Path::new(r"C:\a")),
            Some(DropReject::SameDir)
        );
        // 大小写/尾斜杠不敏感（Windows 卷语义）。
        assert_eq!(
            drop_reject_reason(&[PathBuf::from(r"C:\a\f.txt")], Path::new(r"c:\A\")),
            Some(DropReject::SameDir)
        );
    }

    #[test]
    fn drop_reject_self_drop() {
        // 目录拖到自身。
        assert_eq!(
            drop_reject_reason(&[PathBuf::from(r"C:\a\b")], Path::new(r"C:\a\b")),
            Some(DropReject::SelfDrop)
        );
        // 目录拖进自身子目录。
        assert_eq!(
            drop_reject_reason(&[PathBuf::from(r"C:\a\b")], Path::new(r"C:\a\b\c")),
            Some(DropReject::SelfDrop)
        );
    }

    #[test]
    fn drop_accepts_cross_dir() {
        assert_eq!(
            drop_reject_reason(&[PathBuf::from(r"C:\a\f.txt")], Path::new(r"C:\b")),
            None
        );
        assert_eq!(
            drop_reject_reason(&[PathBuf::from(r"C:\a\b")], Path::new(r"C:\c")),
            None
        );
    }

    #[test]
    fn drop_queue_roundtrip() {
        // 队列 push/take 往返（线程局部，无 FFI）。
        assert!(take_dropped().is_empty() || true); // 可能有其他测试残留，不假设
        push_dropped(DroppedPaths {
            pane_index: 1,
            paths: vec![PathBuf::from(r"C:\a\f.txt")],
            effect: DROPEFFECT_MOVE_VALUE,
            screen_pt: (5, 6),
        });
        let got = take_dropped();
        assert!(got.iter().any(|d| d.pane_index == 1 && d.screen_pt == (5, 6)));
    }

    #[test]
    fn pane_drop_target_refcount_and_qi() {
        // PaneDropTarget 的 COM 语义：QI 认 IDropTarget/IUnknown，AddRef/Release 计数。
        let mut t = PaneDropTarget::new(2, 0x1234);
        let ptr = t.as_interface_ptr();
        assert!(!ptr.is_null());
        unsafe {
            assert_eq!(dt_add_ref(t.as_mut()), 2);
            assert_eq!(dt_add_ref(t.as_mut()), 3);
            assert_eq!(dt_release(t.as_mut()), 2);
            let mut out: *mut std::ffi::c_void = std::ptr::null_mut();
            // IUnknown（全零 IID）。
            let iid_unknown = [0u8; 16];
            assert_eq!(dt_query_interface(t.as_mut(), &iid_unknown, &mut out), 0);
            assert_eq!(out, ptr);
            // IDropTarget IID。
            assert_eq!(dt_query_interface(t.as_mut(), &IID_IDROPTARGET, &mut out), 0);
            assert_eq!(out, ptr);
            // 陌生 IID → E_NOINTERFACE。
            let iid_other = [0x99u8; 16];
            assert_eq!(dt_query_interface(t.as_mut(), &iid_other, &mut out), -2147467262);
        }
    }
}
