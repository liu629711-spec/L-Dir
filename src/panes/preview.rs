//! 预览面板：按 `[Preview] Filter` 通配符决定能否预览，并给出三类预览数据。
//!
//! 纯状态/数据层（D2：不画界面）：
//! - [`PreviewKind`] 由扩展名 + [`core::Config::preview_filter`] 共同决定；
//! - 图片给 **RGBA 位图**（已按上限缩放过），GUI 层直接贴纹理；
//! - 文本给**前 N 行**（不把整个文件读进内存）；
//! - 二进制只给**大小 + 前若干字节的十六进制**，不解释内容。
//!
//! 面板显隐由会话状态 [`core::SessionState::preview_visible`]（.qdr 的
//! `Vorschau`）与分割位置 `VorschauPos` 决定，本模块只回答「这个文件能预览出
//! 什么」，不回答「面板开不开」。
//!
//! 图片解码走 `image` crate（架构契约已列入依赖清单，features 覆盖
//! png/jpeg/gif/bmp/ico/webp）。解码失败不 panic，统一转 [`QError`]。

use std::fs::File;
use std::io::Read;
use std::path::Path;

use image::GenericImageView;
use crate::core::{wildcard_match, QError, QResult};

/// 单张预览图的最大边长（像素）。超过则先降采样，避免 GUI 纹理过大。
/// 原版 Q-Dir 的预览是按面板尺寸拉伸的，此处取一个够用的固定上限（约定值）。
pub const MAX_IMAGE_DIM: u32 = 512;

/// 预览文本的最大行数（文本给「前 N 行」）。
pub const DEFAULT_MAX_TEXT_LINES: usize = 200;

/// 文本预览最多读入的字节数。超过即截断（`truncated=true`），
/// 防止把几百 MB 的日志整个读进内存。
pub const MAX_TEXT_BYTES: u64 = 1024 * 1024;

/// 二进制预览展示的前导字节数。
pub const BINARY_PREVIEW_BYTES: usize = 256;

/// 超过该大小的文件一律不预览（图片解码前的总闸）。
pub const MAX_PREVIEW_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// 解码后像素数的绝对上限（防恶意/畸形图片声明巨大尺寸）。
const MAX_DECODED_PIXELS: u64 = 64 * 1024 * 1024;

/// 预览能力分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewKind {
    /// 不支持预览（Filter 未命中或类型未知）。
    None,
    /// 图片：解码为 RGBA 位图。
    Image,
    /// 文本：给前 N 行。
    Text,
    /// 二进制：只给大小与前导字节。
    Binary,
}

impl PreviewKind {
    /// 是否可预览。
    pub fn is_previewable(self) -> bool {
        !matches!(self, PreviewKind::None)
    }
}

/// 图片预览数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePreview {
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）。
    pub height: u32,
    /// RGBA8 像素，长度 = `width * height * 4`，行优先。
    pub rgba: Vec<u8>,
}

/// 文本预览数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPreview {
    /// 前 N 行（不含行分隔符）。
    pub lines: Vec<String>,
    /// 是否还有未显示的内容（行数或字节数超限）。
    pub truncated: bool,
    /// 文件总字节数。
    pub total_bytes: u64,
}

/// 二进制预览数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryPreview {
    /// 文件总字节数。
    pub total_bytes: u64,
    /// 前 [`BINARY_PREVIEW_BYTES`] 字节（文件更短则全给）。
    pub head: Vec<u8>,
}

/// 一个文件的完整预览结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewData {
    Image(ImagePreview),
    Text(TextPreview),
    Binary(BinaryPreview),
}

impl PreviewData {
    /// 预览能力分类。
    pub fn kind(&self) -> PreviewKind {
        match self {
            PreviewData::Image(_) => PreviewKind::Image,
            PreviewData::Text(_) => PreviewKind::Text,
            PreviewData::Binary(_) => PreviewKind::Binary,
        }
    }
}

/// 图片扩展名（小写，不含点）。与契约依赖的 image features 对齐。
const IMAGE_EXTS: [&str; 9] = ["png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "tif", "tiff"];

/// 文本扩展名（小写，不含点）。未取证的原版清单，按常见文本类型约定。
const TEXT_EXTS: [&str; 22] = [
    "txt", "log", "ini", "inf", "cfg", "conf", "csv", "tsv", "md", "json", "xml", "html", "htm",
    "css", "js", "ts", "rs", "c", "h", "cpp", "hpp", "bat",
];

/// 小写扩展名（不含点）。
fn ext_lower(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// 判断文件名是否命中 `[Preview] Filter` 通配符列表。
///
/// 分隔符同时接受 `;` 与 `,`（与 Filter2 的 F2 约定一致，容忍手写文件混用）；
/// 空模式段忽略（实测默认值 `*;` 结尾就有一个空段）。
/// 过滤器为空串时**任何文件都不可预览**（实测 `[Ordner] Filter=` 即此形态）。
pub fn matches_filter(filter: &str, name: &str) -> bool {
    if filter.trim().is_empty() {
        return false;
    }
    let lower = name.to_lowercase();
    filter
        .split([';', ','])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .any(|p| wildcard_match(p, &lower))
}

/// 该文件的预览分类（未考虑文件大小）。
pub fn kind_for(path: &Path, filter: &str) -> PreviewKind {
    let name = match path.file_name() {
        Some(n) => n.to_string_lossy().to_string(),
        None => return PreviewKind::None,
    };
    if !matches_filter(filter, &name) {
        return PreviewKind::None;
    }
    let ext = ext_lower(path);
    if IMAGE_EXTS.contains(&ext.as_str()) {
        PreviewKind::Image
    } else if TEXT_EXTS.contains(&ext.as_str()) {
        PreviewKind::Text
    } else {
        // Filter 放行但类型未知：按二进制给大小 + 前导字节，不猜内容。
        PreviewKind::Binary
    }
}

/// 判断文件是否可预览（含大小闸）。
pub fn previewable(path: &Path, filter: &str) -> bool {
    if kind_for(path, filter) == PreviewKind::None {
        return false;
    }
    match file_size(path) {
        Ok(sz) => sz <= MAX_PREVIEW_FILE_BYTES,
        Err(_) => false, // 读不到大小（不存在/无权限）按不可预览处理
    }
}

/// 文件大小；失败转 [`QError`]。
fn file_size(path: &Path) -> QResult<u64> {
    let meta = std::fs::metadata(path).map_err(|e| QError::from_io(&e))?;
    Ok(meta.len())
}

/// 生成预览数据。
///
/// `max_lines` 传 `None` 用 [`DEFAULT_MAX_TEXT_LINES`]。
/// 不可预览（Filter 未命中）返回 [`QError::Config`] 并带上 `Filter` 键名，
/// 让上层能提示「该类型不在预览过滤器里」。
pub fn build_preview(path: &Path, filter: &str, max_lines: Option<usize>) -> QResult<PreviewData> {
    let kind = kind_for(path, filter);
    if kind == PreviewKind::None {
        return Err(QError::config(
            "Filter",
            format!("{} 不在 [Preview] Filter 里", path.display()),
        ));
    }
    let size = file_size(path)?;
    if size > MAX_PREVIEW_FILE_BYTES {
        return Err(QError::config(
            "Filter",
            format!("{} 大小 {size} 超过预览上限 {MAX_PREVIEW_FILE_BYTES}", path.display()),
        ));
    }
    match kind {
        PreviewKind::None => unreachable!("上面已判定"),
        PreviewKind::Image => Ok(PreviewData::Image(decode_image(path)?)),
        PreviewKind::Text => Ok(PreviewData::Text(read_text(path, max_lines)?)),
        PreviewKind::Binary => Ok(PreviewData::Binary(read_binary(path)?)),
    }
}

/// 解码图片并降采样到 [`MAX_IMAGE_DIM`] 以内，输出 RGBA8。
fn decode_image(path: &Path) -> QResult<ImagePreview> {
    let reader = image::ImageReader::open(path).map_err(|e| open_err(path, &e))?;
    // 按扩展名猜格式；猜不出时 image 会按内容嗅探，失败再报错。
    let img = reader
        .with_guessed_format()
        .map_err(|e| open_err(path, &e))?
        .decode()
        .map_err(|e| decode_err(path, &e))?;
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_DECODED_PIXELS {
        // 尺寸非法（0 边或超过绝对像素上限，防畸形/恶意图片）。
        return Err(bad_dim_err(path));
    }
    let out = if w.max(h) > MAX_IMAGE_DIM {
        let scale = MAX_IMAGE_DIM as f32 / w.max(h) as f32;
        let nw = ((w as f32 * scale).round() as u32).max(1);
        let nh = ((h as f32 * scale).round() as u32).max(1);
        image::imageops::resize(&img.to_rgba8(), nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img.to_rgba8()
    };
    Ok(ImagePreview {
        width: out.width(),
        height: out.height(),
        rgba: out.into_raw(),
    })
}

/// 图片解码错误统一转 [`QError::Io`]（code=0 表示非 OS 错误，与 `from_io` 的
/// `unwrap_or(0)` 语义一致）。
fn decode_err(path: &Path, e: &image::ImageError) -> QError {
    QError::Io {
        code: 0,
        msg: format!("预览图片解码失败 {}：{e}", path.display()),
    }
}

/// 打开图片文件失败（IO 层）转 [`QError`]，保留 OS 错误码（C3）。
fn open_err(path: &Path, e: &std::io::Error) -> QError {
    QError::Io {
        code: e.raw_os_error().unwrap_or(0) as u32,
        msg: format!("打开预览图片失败 {}：{e}", path.display()),
    }
}

/// 尺寸非法（0 边或超过绝对像素上限）的构造错误。
fn bad_dim_err(path: &Path) -> QError {
    decode_err(
        path,
        &image::ImageError::Parameter(image::error::ParameterError::from_kind(
            image::error::ParameterErrorKind::DimensionMismatch,
        )),
    )
}

/// 读文本前 N 行。
fn read_text(path: &Path, max_lines: Option<usize>) -> QResult<TextPreview> {
    let limit = max_lines.unwrap_or(DEFAULT_MAX_TEXT_LINES).max(1);
    let total_bytes = file_size(path)?;
    let mut f = File::open(path).map_err(|e| QError::from_io(&e))?;
    // 只读前 MAX_TEXT_BYTES 字节，超限即截断。
    let mut buf = Vec::new();
    f.by_ref()
        .take(MAX_TEXT_BYTES)
        .read_to_end(&mut buf)
        .map_err(|e| QError::from_io(&e))?;
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<String> = Vec::new();
    let mut truncated = total_bytes > MAX_TEXT_BYTES;
    for line in text.lines() {
        if lines.len() >= limit {
            truncated = true;
            break;
        }
        lines.push(line.to_string());
    }
    Ok(TextPreview {
        lines,
        truncated,
        total_bytes,
    })
}

/// 读二进制前导字节。
fn read_binary(path: &Path) -> QResult<BinaryPreview> {
    let total_bytes = file_size(path)?;
    let mut buf = vec![0u8; BINARY_PREVIEW_BYTES];
    let mut f = File::open(path).map_err(|e| QError::from_io(&e))?;
    let n = f.read(&mut buf).map_err(|e| QError::from_io(&e))?;
    buf.truncate(n);
    Ok(BinaryPreview { total_bytes, head: buf })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("qdir-rs-preview-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("建临时目录");
        p
    }

    /// 最小 1x1 红色 PNG（image crate 自己编码，避免手写字节）。
    fn write_tiny_png(dir: &Path) -> std::path::PathBuf {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([0xFF, 0x00, 0x00, 0xFF]));
        let p = dir.join("tiny.png");
        img.save(&p).expect("写 PNG");
        p
    }

    #[test]
    fn filter_matching_semicolon_and_comma_and_trailing_sep() {
        // 实测默认值 `*;`：结尾有一个空段，必须忽略。
        assert!(matches_filter("*;", "anything.bin"));
        assert!(matches_filter("*.txt;", "a.txt"));
        assert!(!matches_filter("*.txt;", "a.bin"));
        // 分隔符混用（与 Filter2 的 F2 约定一致）。
        assert!(matches_filter("*.jpg,*.png", "a.png"));
        assert!(matches_filter("*.jpg,*.png", "a.jpg"));
        assert!(!matches_filter("*.jpg,*.png", "a.gif"));
        // 大小写不敏感。
        assert!(matches_filter("*.TXT", "a.txt"));
        // 空过滤器：任何文件都不可预览（实测 [Ordner] Filter= 形态）。
        assert!(!matches_filter("", "a.txt"));
        assert!(!matches_filter("  ", "a.txt"));
    }

    #[test]
    fn kind_classification_by_extension() {
        let f = |n: &str| Path::new(n).to_path_buf();
        assert_eq!(kind_for(&f("a.png"), "*;"), PreviewKind::Image);
        assert_eq!(kind_for(&f("a.JPG"), "*;"), PreviewKind::Image);
        assert_eq!(kind_for(&f("a.txt"), "*;"), PreviewKind::Text);
        assert_eq!(kind_for(&f("a.log"), "*;"), PreviewKind::Text);
        assert_eq!(kind_for(&f("a.zip"), "*;"), PreviewKind::Binary);
        // Filter 不放行 -> None（优先于扩展名）。
        assert_eq!(kind_for(&f("a.txt"), "*.png;"), PreviewKind::None);
        assert!(!PreviewKind::None.is_previewable());
        assert!(PreviewKind::Image.is_previewable());
    }

    #[test]
    fn text_preview_returns_first_n_lines_and_truncates() {
        let dir = tmp_dir("text");
        let p = dir.join("many.txt");
        std::fs::write(&p, (0..500).map(|i| format!("line{i}\n")).collect::<Vec<_>>().join(""))
            .expect("写文本");
        let d = build_preview(&p, "*;", Some(10)).expect("文本预览");
        match d {
            PreviewData::Text(t) => {
                assert_eq!(t.lines.len(), 10);
                assert_eq!(t.lines[0], "line0");
                assert_eq!(t.lines[9], "line9");
                assert!(t.truncated, "还有 490 行没显示");
                assert_eq!(t.total_bytes, std::fs::metadata(&p).expect("meta").len());
            }
            other => panic!("应为文本，实际 {other:?}"),
        }
    }

    #[test]
    fn text_preview_small_file_not_truncated() {
        let dir = tmp_dir("text-small");
        let p = dir.join("few.txt");
        std::fs::write(&p, "a\nb\nc").expect("写文本");
        match build_preview(&p, "*;", None).expect("文本预览") {
            PreviewData::Text(t) => {
                assert_eq!(t.lines, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
                assert!(!t.truncated);
            }
            other => panic!("应为文本，实际 {other:?}"),
        }
    }

    #[test]
    fn image_preview_returns_rgba() {
        let dir = tmp_dir("img");
        let p = write_tiny_png(&dir);
        match build_preview(&p, "*;", None).expect("图片预览") {
            PreviewData::Image(i) => {
                assert_eq!(i.width, 1);
                assert_eq!(i.height, 1);
                assert_eq!(i.rgba.len(), 4, "1x1 RGBA = 4 字节");
                assert_eq!(i.rgba, vec![0xFF, 0x00, 0x00, 0xFF]);
            }
            other => panic!("应为图片，实际 {other:?}"),
        }
    }

    #[test]
    fn image_preview_downscales_large_image() {
        let dir = tmp_dir("img-big");
        let p = dir.join("big.png");
        // 600x600 超过 MAX_IMAGE_DIM=512，必须降采样。
        let img = image::RgbaImage::new(600, 600);
        img.save(&p).expect("写 PNG");
        match build_preview(&p, "*;", None).expect("图片预览") {
            PreviewData::Image(i) => {
                assert_eq!(i.width, 512);
                assert_eq!(i.height, 512);
                assert_eq!(i.rgba.len(), 512 * 512 * 4);
            }
            other => panic!("应为图片，实际 {other:?}"),
        }
    }

    #[test]
    fn binary_preview_gives_size_and_head() {
        let dir = tmp_dir("bin");
        let p = dir.join("blob.zip");
        let data: Vec<u8> = (0..1000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&p, &data).expect("写二进制");
        match build_preview(&p, "*;", None).expect("二进制预览") {
            PreviewData::Binary(b) => {
                assert_eq!(b.total_bytes, 1000);
                assert_eq!(b.head.len(), BINARY_PREVIEW_BYTES);
                assert_eq!(b.head, data[..BINARY_PREVIEW_BYTES]);
            }
            other => panic!("应为二进制，实际 {other:?}"),
        }
    }

    #[test]
    fn unlisted_file_reports_config_error_with_key_name() {
        let dir = tmp_dir("blocked");
        let p = dir.join("a.txt");
        std::fs::write(&p, "x").expect("写文本");
        let err = build_preview(&p, "*.png;", None).unwrap_err();
        match err {
            QError::Config { key, .. } => assert_eq!(key, "Filter", "C3：必须带上原始键名"),
            other => panic!("应为 Config 错误，实际 {other:?}"),
        }
    }

    #[test]
    fn missing_file_is_not_previewable() {
        let p = Path::new(r"Z:\definitely-not-here\a.txt");
        assert!(!previewable(p, "*;"));
        assert!(build_preview(p, "*;", None).is_err());
    }

    #[test]
    fn corrupt_image_reports_error_not_panic() {
        let dir = tmp_dir("corrupt");
        let p = dir.join("bad.png");
        std::fs::write(&p, b"not a png at all").expect("写坏文件");
        let err = build_preview(&p, "*;", None).unwrap_err();
        assert!(matches!(err, QError::Io { .. }), "解码失败应转 QError，实际 {err:?}");
    }
}
