//! 截图像素 diff：颜色级验收的计算内核。
//!
//! 用途：UIA 控制树 diff 只能证明"控件结构一致"，证明不了"颜色一致"
//! （colorthema 各档调色板、Filter2 行色的最终定案都卡在它上面）。本模块
//! 提供纯计算部分——两张 BGRA 位图（来自 `examples/pixdiff.rs` 的
//! PrintWindow 捕获）→ 差异率 / 差异包围盒 / 主色差异列表；渲染与
//! 捕获在 example 侧，保证内核可离线单元测试。

/// 一张 BGRA 位图（每像素 4 字节，行主序，无 padding）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: usize,
    pub height: usize,
    /// 字节序 B,G,R,A（与 32bpp BI_RGB 位图内存布局一致）。
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn new(width: usize, height: usize, fill: [u8; 4]) -> Self {
        let mut pixels = Vec::with_capacity(width * height * 4);
        for _ in 0..(width * height) {
            pixels.extend_from_slice(&fill);
        }
        Self { width, height, pixels }
    }

    #[inline]
    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.width + x) * 4;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2], self.pixels[i + 3]]
    }
}

/// 单通道色差阈值：小于等于该值视为相同（抗子像素渲染/抖动噪声）。
#[derive(Clone, Copy, Debug)]
pub struct DiffThreshold(pub u8);

impl Default for DiffThreshold {
    fn default() -> Self {
        // 16 = 每通道约 6% 容差；文字抗锯齿边缘通常落在 ±10 内。
        DiffThreshold(16)
    }
}

/// 一张图上量化后的主色统计（BGR 各截到 3bit 量化 = 32 色桶）。
fn color_buckets(img: &RgbaImage) -> Vec<([u8; 3], u64)> {
    use std::collections::HashMap;
    let mut m: HashMap<[u8; 3], u64> = HashMap::new();
    for px in img.pixels.chunks_exact(4) {
        // 量化：抹掉低位（4bit 量化 = 16 级/通道）。
        let key = [px[2] & 0xF0, px[1] & 0xF0, px[0] & 0xF0]; // RGB
        *m.entry(key).or_default() += 1;
    }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// 两图差异报告。
#[derive(Clone, Debug)]
pub struct DiffReport {
    pub width: usize,
    pub height: usize,
    /// 差异像素占比（0..=1）。
    pub diff_ratio: f64,
    pub diff_pixels: u64,
    /// 差异包围盒（min_x, min_y, max_x_excl, max_y_excl）；无差异为 None。
    pub bbox: Option<(usize, usize, usize, usize)>,
    /// 主色占比差（RGB 量化桶，按 |Δratio| 降序，前 8 条）。
    pub palette_delta: Vec<([u8; 3], f64, f64)>,
}

impl DiffReport {
    pub fn is_identical(&self) -> bool {
        self.diff_pixels == 0
    }
    /// 颜色级验收阈值：差异占比 ≤ 0.2% 视为一致（同布局下字体渲染
    /// 的固有噪声；*阈值待首轮实测标定*）。
    pub fn passes_color(&self, limit: f64) -> bool {
        self.diff_ratio <= limit
    }
}

/// 逐像素 diff：任一通道超过阈值即计差异。两图尺寸必须一致。
pub fn diff(img_a: &RgbaImage, img_b: &RgbaImage, thr: DiffThreshold) -> DiffReport {
    assert_eq!(
        (img_a.width, img_a.height),
        (img_b.width, img_b.height),
        "diff 要求同尺寸位图"
    );
    let (w, h) = (img_a.width, img_a.height);
    let mut diff_pixels = 0u64;
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            let a = img_a.pixel(x, y);
            let b = img_b.pixel(x, y);
            let delta = (a[0].abs_diff(b[0]))
                .max(a[1].abs_diff(b[1]))
                .max(a[2].abs_diff(b[2]));
            if delta > thr.0 {
                diff_pixels += 1;
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    let total = (w * h) as u64;
    let bbox = if diff_pixels == 0 {
        None
    } else {
        Some((x0, y0, x1, y1))
    };

    // 主色占比差：两侧各取前 24 桶并集，算 |Δratio| 排序取前 8。
    let mut delta_map: std::collections::HashMap<[u8; 3], (f64, f64)> = std::collections::HashMap::new();
    for (key, cnt) in color_buckets(img_a) {
        delta_map.entry(key).or_insert((0.0, 0.0)).0 = cnt as f64 / total as f64;
    }
    for (key, cnt) in color_buckets(img_b) {
        delta_map.entry(key).or_insert((0.0, 0.0)).1 = cnt as f64 / total as f64;
    }
    let mut palette_delta: Vec<([u8; 3], f64, f64)> = delta_map
        .into_iter()
        .map(|(k, (ra, rb))| (k, ra, rb))
        .collect();
    palette_delta.sort_by(|a, b| {
        (b.1 - b.2)
            .abs()
            .partial_cmp(&(a.1 - a.2).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    palette_delta.truncate(8);

    DiffReport {
        width: w,
        height: h,
        diff_ratio: diff_pixels as f64 / total as f64,
        diff_pixels,
        bbox,
        palette_delta,
    }
}

/// 生成差异可视化图：底图 = img_a，差异像素染成品红。
pub fn overlay_image(img_a: &RgbaImage, img_b: &RgbaImage, thr: DiffThreshold) -> RgbaImage {
    assert_eq!((img_a.width, img_a.height), (img_b.width, img_b.height));
    let mut out = img_a.clone();
    let (w, h) = (out.width, out.height);
    for y in 0..h {
        for x in 0..w {
            let a = img_a.pixel(x, y);
            let b = img_b.pixel(x, y);
            let delta = (a[0].abs_diff(b[0])).max(a[1].abs_diff(b[1])).max(a[2].abs_diff(b[2]));
            if delta > thr.0 {
                let i = (y * w + x) * 4;
                out.pixels[i] = 0xFF; // B
                out.pixels[i + 1] = 0x00; // G
                out.pixels[i + 2] = 0xFF; // R -> 品红
                out.pixels[i + 3] = 0xFF;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(w: usize, h: usize, c: [u8; 4]) -> RgbaImage {
        RgbaImage::new(w, h, c)
    }

    #[test]
    fn identical_images_report_zero_diff() {
        let a = img(64, 48, [10, 20, 30, 255]);
        let b = img(64, 48, [10, 20, 30, 255]);
        let r = diff(&a, &b, DiffThreshold::default());
        assert!(r.is_identical());
        assert_eq!(r.bbox, None);
        assert_eq!(r.diff_ratio, 0.0);
    }

    #[test]
    fn single_pixel_diff_has_bbox_and_ratio() {
        let a = img(100, 10, [0, 0, 0, 255]);
        let mut b = a.clone();
        b.pixels[(7 * 100 + 3) * 4 + 2] = 255; // (3,7) 变红
        let r = diff(&a, &b, DiffThreshold::default());
        assert_eq!(r.diff_pixels, 1);
        assert_eq!(r.bbox, Some((3, 7, 4, 8)));
        assert_eq!(r.diff_ratio, 1.0 / 1000.0);
    }

    #[test]
    fn threshold_suppresses_noise() {
        let a = img(16, 16, [100, 100, 100, 255]);
        let mut b = a.clone();
        for px in b.pixels.chunks_exact_mut(4) {
            px[0] = 100 + 8; // 每通道 8 的抖动
        }
        assert!(diff(&a, &b, DiffThreshold(16)).is_identical());
        assert_eq!(diff(&a, &b, DiffThreshold(4)).diff_pixels, 16 * 16);
    }

    #[test]
    fn palette_delta_surfaces_theme_color_change() {
        // 整幅底色从 #008800（绿）变 #BB8844：主色桶应出现在 delta 头部。
        let a = img(64, 64, [0x00, 0x88, 0x00, 255]);
        let b = img(64, 64, [0x44, 0x88, 0xBB, 255]);
        let r = diff(&a, &b, DiffThreshold::default());
        assert!(r.diff_pixels > 0);
        let top = &r.palette_delta[0];
        assert!((top.1 - top.2).abs() > 0.99, "头部主色占比差应接近 1.0：{top:?}");
    }

    #[test]
    fn overlay_marks_diff_pixels_magenta() {
        let a = img(4, 4, [0, 0, 0, 255]);
        let mut b = a.clone();
        b.pixels[(1 * 4 + 1) * 4 + 2] = 255; // (1,1)
        let o = overlay_image(&a, &b, DiffThreshold::default());
        assert_eq!(o.pixel(1, 1), [0xFF, 0x00, 0xFF, 0xFF]);
        assert_eq!(o.pixel(0, 0), [0, 0, 0, 255]);
    }

    #[test]
    #[should_panic(expected = "同尺寸")]
    fn mismatched_sizes_panic() {
        let _ = diff(&img(2, 2, [0, 0, 0, 0]), &img(3, 2, [0, 0, 0, 0]), DiffThreshold::default());
    }
}
