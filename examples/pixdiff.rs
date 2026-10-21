//! 截图像素 diff 工具（颜色级验收前置件）。
//!
//! 两个模式：
//! - `capture <PID> <out.png>`：PrintWindow 抓取该 PID 标题为 "Q-Dir"
//!   的顶层窗口 → 存 PNG（BGRA→RGBA 经 image crate）。
//! - `diff <a.png> <b.png> [threshold] [out_overlay.png]`：加载两张
//!   同尺寸截图 → qdir_rs::services::pixdiff 计算差异率/包围盒/主色差
//!   → 打印报告；给了 out_overlay 则写出差异染品红的可视化图。
//!
//! 用法（两窗并排、同尺寸、同目录内容时对比最有意义；原版对 DPI/
//! 主题敏感，跑前两边设成相同窗口位置大小）：
//!   pixdiff capture 1234 orig.png
//!   pixdiff capture 5678 mine.png
//!   pixdiff diff orig.png mine.png 16 overlay.png
//!
//! 注：PrintWindow(PW_RENDERFULLCONTENT) 对 DWM 合成窗口最可靠；WTL
//! owner-draw 区域全部走 WM_PRINTCLIENT，可正常捕获。

#![windows_subsystem = "console"]

use std::path::PathBuf;

use qdir_rs::services::pixdiff::{self, DiffThreshold, RgbaImage};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS};

use windows::Win32::UI::WindowsAndMessaging::*;

struct Sink {
    pid: u32,
    found: Option<HWND>,
}

unsafe extern "system" fn find_top(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
    let mut p = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut p)) };
    let s = unsafe { &mut *(lp.0 as *mut Sink) };
    if p == s.pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
        let mut buf = [0u16; 64];
        let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
        if String::from_utf16_lossy(&buf[..n.max(0) as usize]) == "Q-Dir" {
            s.found = Some(hwnd);
            return false.into();
        }
    }
    true.into()
}

/// PrintWindow 整窗（含非客户区）→ BGRA 缓冲。
fn capture_window(hwnd: HWND) -> Result<RgbaImage, String> {
    unsafe {
        let mut rc = RECT::default();
        GetWindowRect(hwnd, &mut rc).map_err(|e| format!("GetWindowRect: {e}"))?;
        let w = rc.right - rc.left;
        let h = rc.bottom - rc.top;
        if w <= 0 || h <= 0 {
            return Err(format!("窗口尺寸非法 {w}x{h}"));
        }
        let hdc = GetWindowDC(Some(hwnd));
        if hdc.is_invalid() {
            return Err("GetWindowDC 失败".to_string());
        }
        let mem = CreateCompatibleDC(Some(hdc));
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // 负高度 = top-down（行主序第 0 行在顶部）。
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbm = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            .map_err(|e| format!("CreateDIBSection: {e}"))?;
        if bits.is_null() {
            return Err("CreateDIBSection 返回空缓冲".to_string());
        }
        let old = SelectObject(mem, hbm.into());
        let ok = PrintWindow(hwnd, mem, PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT));
        let _ = SelectObject(mem, old);
        if !ok.as_bool() {
            return Err("PrintWindow 失败（窗口可能已关闭）".to_string());
        }
        let len = (w as usize) * (h as usize) * 4;
        let pixels = std::slice::from_raw_parts(bits as *const u8, len).to_vec();
        let _ = DeleteObject(hbm.into());
        let _ = DeleteDC(mem);
        ReleaseDC(Some(hwnd), hdc);
        Ok(RgbaImage { width: w as usize, height: h as usize, pixels })
    }
}

fn save_png(img: &RgbaImage, path: &str) -> Result<(), String> {
    use image::{Rgba, RgbaImage as Img};
    let mut out = Img::new(img.width as u32, img.height as u32);
    for y in 0..img.height {
        for x in 0..img.width {
            let bgra = img.pixel(x, y);
            out.put_pixel(x as u32, y as u32, Rgba([bgra[2], bgra[1], bgra[0], bgra[3]]));
        }
    }
    out.save(path).map_err(|e| format!("保存 PNG {path}: {e}"))
}

fn load_png(path: &str) -> Result<RgbaImage, String> {
    use image::GenericImageView;
    let img = image::open(path)
        .map_err(|e| format!("打开 PNG {path}: {e}"))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    // RGBA → BGRA（pixdiff 内核按位图内存序 B,G,R,A）。
    let pixels: Vec<u8> = img
        .pixels()
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect();
    Ok(RgbaImage { width: w as usize, height: h as usize, pixels })
}

fn print_report(r: &pixdiff::DiffReport, overlay_path: Option<&str>) {
    println!(
        "尺寸 {}x{} | 差异像素 {} ({:.4}%) | bbox {:?}",
        r.width,
        r.height,
        r.diff_pixels,
        r.diff_ratio * 100.0,
        r.bbox
    );
    println!("主色占比差 Top（RGB 量化，A→B）：");
    for (rgb, ra, rb) in &r.palette_delta {
        println!(
            "  #{:02X}{:02X}{:02X}  {:>6.2}% → {:>6.2}%  (Δ {:+.2}%)",
            rgb[0], rgb[1], rgb[2], ra * 100.0, rb * 100.0, (rb - ra) * 100.0
        );
    }
    // 颜色级验收默认阈值：差异占比 ≤0.2%（*待首轮实测标定*）。
    println!("PASS 判定（阈值 0.2%）：{}", if r.passes_color(0.002) { "通过" } else { "不通过" });
    if let Some(p) = overlay_path {
        println!("提示：overlay 图由 pixdiff::overlay_image 生成（见 diff 模式第 5 参）");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map(|s| s.as_str()).unwrap_or("");
    match mode {
        "capture" => {
            let pid: u32 = args[1].parse().expect("capture <PID> <out.png>");
            let out = &args[2];
            let mut sink = Sink { pid, found: None };
            unsafe { EnumWindows(Some(find_top), LPARAM(&mut sink as *mut _ as isize)) };
            let hwnd = sink.found.ok_or_else(|| format!("PID {pid} 无标题 'Q-Dir' 可见顶层窗")).unwrap();
            let img = capture_window(hwnd).unwrap();
            save_png(&img, out).unwrap();
            println!("captured {}x{} -> {out}", img.width, img.height);
        }
        "diff" => {
            let a = load_png(&args[1]).unwrap();
            let b = load_png(&args[2]).unwrap();
            if (a.width, a.height) != (b.width, b.height) {
                // 自动降级：取交集尺寸做 diff（两窗位置/大小常有一两像素差）。
                println!(
                    "警告：尺寸不同 {}x{} vs {}x{}，按交集 {}x{} 对齐裁剪",
                    a.width, a.height, b.width, b.height,
                    a.width.min(b.width), a.height.min(b.height)
                );
            }
            let thr = DiffThreshold(args.get(3).and_then(|s| s.parse().ok()).unwrap_or(16));
            let r = pixdiff::diff(
                &crop(&a, &b),
                &crop(&b, &a),
                thr,
            );
            print_report(&r, args.get(4).map(|s| s.as_str()));
            if let Some(p) = args.get(4) {
                let o = pixdiff::overlay_image(&crop(&a, &b), &crop(&b, &a), thr);
                save_png(&o, p).unwrap();
                println!("overlay -> {p}");
            }
        }
        _ => {
            eprintln!("用法: pixdiff capture <PID> <out.png>");
            eprintln!("  或: pixdiff diff <a.png> <b.png> [threshold] [out_overlay.png]");
            std::process::exit(2);
        }
    }
}

/// 把 b 裁剪/扩边到 a 的尺寸（取左上交集；行按 a.width 重排）。
fn crop(img: &RgbaImage, ref_img: &RgbaImage) -> RgbaImage {
    let (w, h) = (ref_img.width, ref_img.height);
    let mut out = RgbaImage::new(w, h, [0, 0, 0, 0]);
    for y in 0..h.min(img.height) {
        for x in 0..w.min(img.width) {
            let i = (y * w + x) * 4;
            let p = img.pixel(x, y);
            out.pixels[i..i + 4].copy_from_slice(&p);
        }
    }
    out
}
