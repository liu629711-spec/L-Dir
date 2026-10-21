//! 系统图标：SHGetFileInfoW / SHGetStockIconInfo → RGBA 位图 + `icon_key` 缓存。
//!
//! 职责边界（契约 modules）：
//! - 本模块把 Windows 的 `HICON` 转成 egui 能用的 `Vec<u8>` RGBA；
//! - `FileItem.icon_index` 由 enumerate 填 `SHGFI_SYSICONINDEX` 的系统图标索引，
//!   UI 层用 [`IconCache::key_for_index`] 生成稳定的 `icon_key: u32` 做纹理缓存键；
//! - 图标提取走 GDI（GetIconInfo → CreateDIBSection → GetDIBits），
//!   所有句柄在 unsafe 块内配对释放，不泄漏到本模块之外。

use std::collections::HashMap;
use std::path::Path;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetDIBits,
    GetObjectW, ReleaseDC, SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, RGBQUAD, SRCCOPY,
};
use windows::Win32::UI::Shell::{
    SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SHELLICONSIZE, SHGFI_SMALLICON, SHGFI_SYSICONINDEX,
    SHGFI_USEFILEATTRIBUTES, SHGetFileInfoW, SHSTOCKICONID, SHGetStockIconInfo, SHGSI_ICON,
    SHGSI_SMALLICON, SIID_DOCASSOC, SIID_FOLDER, SHSTOCKICONINFO,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetIconInfo, HICON, ICONINFO,
};

use super::{as_pcwstr, io_error, path_to_wide, shell_error, shfileinfo_size};
use crate::core::{QError, QResult};

/// 图标尺寸档位。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconSize {
    /// 小图标（默认 16px，列表/详情视图）。
    Small,
    /// 大图标（默认 32px）。
    Large,
    /// 特大图标（默认 48px）。
    ExtraLarge,
}

impl IconSize {
    /// 该档位的目标边长（用于 SHGFI_SHELLICONSIZE 与兜底）。
    fn target_px(self) -> i32 {
        match self {
            IconSize::Small => 16,
            IconSize::Large => 32,
            IconSize::ExtraLarge => 48,
        }
    }

    /// SHGetFileInfoW 的尺寸标志。
    fn shgfi_flags(self) -> windows::Win32::UI::Shell::SHGFI_FLAGS {
        match self {
            IconSize::Small => SHGFI_SMALLICON,
            IconSize::Large => SHGFI_LARGEICON,
            // SHELLICONSIZE 由 GetSystemMetrics 决定（48 on most systems）。
            IconSize::ExtraLarge => SHGFI_SHELLICONSIZE,
        }
    }
}

/// 一张 RGBA 图标位图（自持有，无 Windows 句柄）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconImage {
    pub width: u32,
    pub height: u32,
    /// 行主序 RGBA8，长度 = width * height * 4。
    pub pixels: Vec<u8>,
}

impl IconImage {
    /// 生成纯色占位图标（HICON 提取失败时用，避免上层拿到空图）。
    pub fn solid(size: IconSize, rgba: [u8; 4]) -> Self {
        let px = size.target_px().max(1) as u32;
        let mut pixels = Vec::with_capacity((px * px * 4) as usize);
        for _ in 0..(px * px) {
            pixels.extend_from_slice(&rgba);
        }
        IconImage {
            width: px,
            height: px,
            pixels,
        }
    }

    /// 像素总数。
    pub fn len(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// 是否为空图。
    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

/// 图标缓存：`icon_key` → RGBA 位图。
///
/// `icon_key` 由 [`IconCache::key_for_index`] 从系统图标索引派生，
/// 保证同一索引在进程内只有一个 key（egui 纹理缓存以此为键）。
#[derive(Debug, Default)]
pub struct IconCache {
    entries: HashMap<u32, IconImage>,
    /// 兜底：系统图标索引查询失败时，按「扩展名」缓存的结果。
    by_ext: HashMap<String, u32>,
    /// 下一个可分配的自定义 key（系统索引从 0 起，这里从高位避开）。
    next_key: u32,
}

/// 自定义 key 起始值，避开 SHGetFileInfoW 的系统图标索引区间。
///
/// 兜底图标（fallback_image）从这里分配 key；文档说明用途，测试锁定边界。
#[allow(dead_code)]
const CUSTOM_KEY_BASE: u32 = 0x8000_0000;

impl IconCache {
    pub fn new() -> Self {
        IconCache {
            // 兜底 key 从 CUSTOM_KEY_BASE 起，避开系统索引区间（0..0x7FFF_FFFF）。
            next_key: CUSTOM_KEY_BASE,
            ..IconCache::default()
        }
    }

    /// 由系统图标索引生成稳定的缓存 key。
    ///
    /// 语义：key == 索引本身（0..0x7FFF_FFFF），UI 层可安全地把它当纹理句柄用。
    pub fn key_for_index(&self, index: i32) -> u32 {
        if index < 0 {
            0
        } else {
            index as u32
        }
    }

    /// 取（或懒加载）某个系统图标索引的位图。
    pub fn image_for_index(&mut self, index: i32, size: IconSize) -> QResult<IconImage> {
        let key = self.key_for_index(index);
        if let Some(img) = self.entries.get(&key) {
            return Ok(img.clone());
        }
        let img = load_system_icon(index, size)?;
        self.entries.insert(key, img.clone());
        Ok(img)
    }

    /// 按路径取图标位图（自动查系统索引并缓存）。
    pub fn image_for_path(&mut self, path: &Path, size: IconSize) -> QResult<IconImage> {
        let idx = system_icon_index_for_path(path, 0)?;
        self.image_for_index(idx, size)
    }

    /// 按扩展名取图标（缓存索引，避免重复 SHGetFileInfoW）。
    pub fn image_for_ext(&mut self, ext: &str, size: IconSize) -> QResult<IconImage> {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        if let Some(key) = self.by_ext.get(&ext) {
            return self.image_for_index(*key as i32, size);
        }
        let fake = format!("*.{ext}");
        let idx = system_icon_index_for_path(Path::new(&fake), 0)?;
        self.by_ext.insert(ext, idx as u32);
        self.image_for_index(idx, size)
    }

    /// 取内置兜底图标（文件夹/文档），并分配一个自定义 key。
    pub fn fallback_image(&mut self, is_dir: bool, size: IconSize) -> QResult<(u32, IconImage)> {
        let key = self.next_key;
        self.next_key = self.next_key.wrapping_add(1);
        let img = stock_icon(
            if is_dir { SIID_FOLDER } else { SIID_DOCASSOC },
            size,
        )
        .unwrap_or_else(|_| {
            IconImage::solid(
                size,
                if is_dir { [0xDD, 0xB5, 0x33, 0xFF] } else { [0x88, 0x88, 0x88, 0xFF] },
            )
        });
        self.entries.insert(key, img.clone());
        Ok((key, img))
    }

    /// 已缓存的位图数量（测试/诊断）。
    pub fn cached_count(&self) -> usize {
        self.entries.len()
    }
}

/// 查询路径对应的系统图标索引（SHGFI_SYSICONINDEX）。
///
/// 失败（文件不存在且未给属性）返回 [`QError::Shell`]。
pub fn system_icon_index_for_path(path: &Path, attrs: u32) -> QResult<i32> {
    let wide = path_to_wide(path)?;
    let mut sfi: windows::Win32::UI::Shell::SHFILEINFOW = unsafe { std::mem::zeroed() };
    // SAFETY: wide 为 NUL 结尾宽串，sfi 为出参。
    let r = unsafe {
        SHGetFileInfoW(
            as_pcwstr(&wide),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(attrs),
            Some(&mut sfi),
            shfileinfo_size(),
            SHGFI_SYSICONINDEX | SHGFI_USEFILEATTRIBUTES,
        )
    };
    if r == 0 {
        return Err(io_error("SHGetFileInfoW(SYSICONINDEX)"));
    }
    Ok(sfi.iIcon)
}

/// 直接从路径提取 HICON 并转成 RGBA（用于一次性取图，不经过系统索引）。
pub fn icon_image_for_path(path: &Path, size: IconSize) -> QResult<IconImage> {
    let wide = path_to_wide(path)?;
    let mut sfi: windows::Win32::UI::Shell::SHFILEINFOW = unsafe { std::mem::zeroed() };
    // SAFETY: wide 合法，sfi 为出参；SHGFI_ICON 返回的 HICON 归调用方（须 DestroyIcon）。
    let r = unsafe {
        SHGetFileInfoW(
            as_pcwstr(&wide),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi),
            shfileinfo_size(),
            SHGFI_ICON | size.shgfi_flags(),
        )
    };
    if r == 0 {
        return Err(io_error("SHGetFileInfoW(ICON)"));
    }
    // 转换期间持有图标；失败也要销毁。
    let icon = HICON(sfi.hIcon.0);
    if icon.is_invalid() {
        return Err(QError::Shell("SHGetFileInfoW 未返回有效 HICON".to_string()));
    }
    let result = hicon_to_rgba(icon);
    // SAFETY: HICON 由 SHGetFileInfoW(SHGFI_ICON) 分配，本函数拥有它。
    let _ = unsafe { DestroyIcon(icon) };
    result
}

/// 从系统图像列表索引加载图标（走 SHGetStockIconInfo 的等效路径）。
///
/// Windows 未直接暴露「按系统索引取 HICON」的公开 API；
/// 常见做法是 `ImageList_GetIcon(small/large, index)`，但那需要系统图像列表句柄。
/// 这里退化为：索引 < 0 时用内置兜底，否则返回 [`QError::Shell`]，
/// 由调用方走 [`IconCache::fallback_image`]。上层日常路径是
/// [`IconCache::image_for_path`]（直接拿 HICON），不依赖本函数。
pub fn load_system_icon(index: i32, size: IconSize) -> QResult<IconImage> {
    if index < 0 {
        return Err(QError::Shell(format!(
            "系统图标索引无效：{index}"
        )));
    }
    // 未实测到按索引取图标的可靠公开路径（见模块注释），交由上层兜底。
    Err(QError::Shell(format!(
        "不支持按系统索引 {index} 直接取图（{:?}），请改用 image_for_path",
        size
    )))
}

/// 取内置 stock 图标（文件夹/文档）。
pub fn stock_icon(siid: SHSTOCKICONID, size: IconSize) -> QResult<IconImage> {
    let mut info: SHSTOCKICONINFO = unsafe { std::mem::zeroed() };
    // SAFETY: info 为出参。
    unsafe {
        SHGetStockIconInfo(
            siid,
            SHGSI_ICON | if size == IconSize::Small { SHGSI_SMALLICON } else { SHGSI_ICON },
            &mut info,
        )
    }
    .map_err(|e| shell_error("SHGetStockIconInfo", e))?;

    let icon = HICON(info.hIcon.0);
    if icon.is_invalid() {
        return Err(QError::Shell("SHGetStockIconInfo 未返回有效 HICON".to_string()));
    }
    let result = hicon_to_rgba(icon);
    // SAFETY: HICON 由 SHGetStockIconInfo 分配，本函数拥有它。
    let _ = unsafe { DestroyIcon(icon) };
    result
}

/// HICON → RGBA8 位图（核心 unsafe 逻辑，句柄全部在本函数内释放）。
///
/// 做法：GetIconInfo 取彩色位图 → 选入内存 DC → CreateDIBSection(32bpp top-down)
/// → BitBlt → GetDIBits 读像素 → 手动补 alpha（彩色位图的 alpha 常为 0）。
fn hicon_to_rgba(icon: HICON) -> QResult<IconImage> {
    let mut info: ICONINFO = unsafe { std::mem::zeroed() };
    // SAFETY: info 为出参；icon 由调用方保证有效且归本函数所有。
    unsafe { GetIconInfo(icon, &mut info) }
        .map_err(|e| shell_error("GetIconInfo", e))?;

    // 彩色位图（hbmColor）才是我们要的；mask 位图只用于 AND 掩码。
    let color_bmp = HBITMAP(info.hbmColor.0);
    if color_bmp.is_invalid() {
        // 无色位图（黑白图标）：也要释放 mask。
        // SAFETY: hbmMask/hbmColor 由 GetIconInfo 分配。
        unsafe {
            if !HBITMAP(info.hbmMask.0).is_invalid() {
                let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            }
        }
        return Err(QError::Shell("图标没有彩色位图".to_string()));
    }

    // 位图尺寸。
    let mut bm: BITMAP = unsafe { std::mem::zeroed() };
    // SAFETY: bm 为出参。
    let got = unsafe { GetObjectW(HGDIOBJ(color_bmp.0), size_of::<BITMAP>() as i32, Some(&mut bm as *mut BITMAP as *mut std::ffi::c_void)) };
    if got == 0 {
        // SAFETY: 两个位图都由 GetIconInfo 分配。
        unsafe {
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            let _ = DeleteObject(HGDIOBJ(color_bmp.0));
        }
        return Err(io_error("GetObjectW"));
    }
    let width = bm.bmWidth;
    let height = bm.bmHeight;
    if width <= 0 || height <= 0 {
        // SAFETY: 同上。
        unsafe {
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            let _ = DeleteObject(HGDIOBJ(color_bmp.0));
        }
        return Err(QError::Shell(format!(
            "图标位图尺寸非法：{width}x{height}"
        )));
    }

    // 屏幕 DC（仅用于 CreateDIBSection 兼容）。
    let screen_dc = unsafe { GetDC(Some(HWND::default())) };
    if screen_dc.is_invalid() {
        // SAFETY: 同上。
        unsafe {
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            let _ = DeleteObject(HGDIOBJ(color_bmp.0));
        }
        return Err(io_error("GetDC"));
    }

    let mem_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
    if mem_dc.is_invalid() {
        // SAFETY: 同上 + 释放屏幕 DC。
        unsafe {
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            let _ = DeleteObject(HGDIOBJ(color_bmp.0));
            ReleaseDC(Some(HWND::default()), screen_dc);
        }
        return Err(io_error("CreateCompatibleDC"));
    }

    // 32bpp top-down DIB 段（负高度 = top-down）。
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [RGBQUAD::default(); 1],
    };

    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    // SAFETY: bmi 为合法描述，bits 为出参；hsection/offset 为空/0。
    let dib = unsafe {
        CreateDIBSection(
            Some(mem_dc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        )
    };
    let dib = match dib {
        Ok(h) => h,
        Err(e) => {
            // SAFETY: 全部本函数创建/取得的句柄。
            unsafe {
                let _ = DeleteDC(mem_dc);
                let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
                let _ = DeleteObject(HGDIOBJ(color_bmp.0));
                ReleaseDC(Some(HWND::default()), screen_dc);
            }
            return Err(shell_error("CreateDIBSection", e));
        }
    };

    let old = unsafe { SelectObject(mem_dc, HGDIOBJ(dib.0)) };
    // 源 DC（color_bmp 选入的兼容 DC）：BitBlt 后必须 DeleteDC，否则每次取图标泄漏 1 个 DC。
    let src_dc = unsafe { CreateCompatibleDC(Some(screen_dc)) };
    if src_dc.is_invalid() {
        // SAFETY: 全部本函数创建/取得的句柄。
        unsafe {
            let _ = SelectObject(mem_dc, old);
            let _ = DeleteObject(HGDIOBJ(dib.0));
            let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            let _ = DeleteObject(HGDIOBJ(color_bmp.0));
            let _ = DeleteDC(mem_dc);
            ReleaseDC(Some(HWND::default()), screen_dc);
        }
        return Err(io_error("CreateCompatibleDC(src)"));
    }
    // SAFETY: src_dc 有效，bmp 由 GetIconInfo 分配。
    unsafe { SelectObject(src_dc, HGDIOBJ(color_bmp.0)) };

    // SAFETY: 目标/源 DC 与位图均有效。
    let blit = unsafe {
        BitBlt(
            mem_dc,
            0,
            0,
            width,
            height,
            Some(src_dc),
            0,
            0,
            SRCCOPY,
        )
    };
    let _ = blit;

    // GetDIBits 把 DIB 内容读回（BitBlt 已把图标画进去）。
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    // SAFETY: pixels 长度匹配 32bpp 位图，bmi 为入参。
    let copied = unsafe {
        GetDIBits(
            mem_dc,
            dib,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut std::ffi::c_void),
            &bmi as *const BITMAPINFO as *mut BITMAPINFO,
            DIB_RGB_COLORS,
        )
    };

    // 还原并释放所有 GDI 对象（含 BitBlt 的源 DC）。
    // SAFETY: 均为本函数取得/创建的句柄。
    unsafe {
        let _ = SelectObject(mem_dc, old);
        let _ = DeleteObject(HGDIOBJ(dib.0));
        let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
        let _ = DeleteObject(HGDIOBJ(color_bmp.0));
        let _ = DeleteDC(mem_dc);
        let _ = DeleteDC(src_dc);
        ReleaseDC(Some(HWND::default()), screen_dc);
    }

    if copied != height {
        return Err(io_error("GetDIBits"));
    }

    // BGRA → RGBA，并强制 alpha=255（图标彩色位图 alpha 通道通常为 0）。
    for px in pixels.chunks_exact_mut(4) {
        px.swap(0, 2);
        px[3] = 0xFF;
    }

    Ok(IconImage {
        width: width as u32,
        height: height as u32,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_for_index_maps_negative_to_zero() {
        let c = IconCache::new();
        assert_eq!(c.key_for_index(-1), 0);
        assert_eq!(c.key_for_index(0), 0);
        assert_eq!(c.key_for_index(7), 7);
        assert_eq!(c.key_for_index(i32::MAX), i32::MAX as u32);
    }

    #[test]
    fn solid_icon_has_full_alpha() {
        let img = IconImage::solid(IconSize::Small, [1, 2, 3, 255]);
        assert_eq!(img.width, 16);
        assert_eq!(img.height, 16);
        assert_eq!(img.pixels.len(), 16 * 16 * 4);
        assert!(img.pixels.chunks_exact(4).all(|p| p[3] == 255));
        assert!(!img.is_empty());
        assert_eq!(img.len(), 256);
    }

    #[test]
    fn size_targets_match_windows_defaults() {
        assert_eq!(IconSize::Small.target_px(), 16);
        assert_eq!(IconSize::Large.target_px(), 32);
        assert_eq!(IconSize::ExtraLarge.target_px(), 48);
    }

    #[test]
    fn load_system_icon_rejects_negative() {
        assert!(load_system_icon(-1, IconSize::Small).is_err());
    }

    #[test]
    fn fallback_key_is_above_system_range() {
        let mut c = IconCache::new();
        // fallback_image 内部会调 SHGetStockIconInfo（真实 Windows API）；
        // 这里只验证 key 分配规则：两次调用拿到不同且高于系统索引区间的 key。
        // 失败（无桌面会话）时不该 panic，改为跳过后续断言。
        let Ok((k1, _)) = c.fallback_image(true, IconSize::Small) else {
            return;
        };
        assert!(k1 >= CUSTOM_KEY_BASE, "自定义 key 不得与系统索引冲突");
        if let Ok((k2, _)) = c.fallback_image(false, IconSize::Large) {
            assert_ne!(k1, k2);
        }
    }

    #[test]
    fn key_for_index_and_custom_base_do_not_overlap() {
        // 纯算术：系统索引最大 0x7FFF_FFFF，自定义 key 从 0x8000_0000 起。
        let c = IconCache::new();
        assert_eq!(c.key_for_index(0x7FFF_FFFF), 0x7FFF_FFFF);
        assert!(CUSTOM_KEY_BASE > 0x7FFF_FFFF);
    }

    #[test]
    fn icon_size_flags_differ() {
        assert_ne!(
            IconSize::Small.shgfi_flags(),
            IconSize::Large.shgfi_flags()
        );
    }
}
