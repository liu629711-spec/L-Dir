//! 工具栏图标管线：**自绘** 32bpp 位图（PRODUCT_SPEC R1：不使用原版 PNG）。
//!
//! 8 按钮语义保留（NATIVE_SPEC 附录 A.4 确证）：z6/v6=布局索引 −1/+1、
//! upf=弹出布局列表、new=新窗、view/update/go/viewtree=转发/刷新/下拉/树切换。
//! 图形 = 几何 glyph（窗格方块+箭头/树形），品牌色自定（占位浅灰底）。
//!
//! 历史注记：原版 FUN_00429bc4 加载 48x48 PNG；转向后资源不再复用，
//! 但按钮顺序与命令 id 保持（工具栏是"布局体验"层）。

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::{QError, QResult};

/// 主导航按钮表：(glyph, 命令 id, 是否 SplitButton)。
/// 顺序 = 原版 FUN_00429bc4 数组序（布局体验层，保留）。
pub const NAV_BUTTONS: &[(&str, i32, bool)] = &[
    ("layout_prev", 0xA76C, false),
    ("layout_next", 0xA76D, false),
    ("layout_menu", 0xA771, false),
    ("new_window", 0xA772, false),
    ("view", 0xA775, false),
    ("refresh", 0xA770, false),
    ("go", 0xB379, true), // BTNS_DROPDOWN
    ("tree_toggle", 0xA773, false),
];

/// glyph 名 → 48x48 BGRA 像素（全不透明；ImageList 用负高度=自上而下）。
pub fn glyph_bits(name: &str) -> Vec<u8> {
    const S: usize = 48;

    struct Pen {
        px: [u8; S * S * 4],
    }
    impl Pen {
        fn new() -> Self {
            Pen { px: [0; S * S * 4] }
        }
        fn set(&mut self, x: i32, y: i32, rgba: [u8; 4]) {
            if (0..S as i32).contains(&x) && (0..S as i32).contains(&y) {
                let o = (y as usize) * S * 4 + (x as usize) * 4;
                self.px[o] = rgba[2];
                self.px[o + 1] = rgba[1];
                self.px[o + 2] = rgba[0];
                self.px[o + 3] = rgba[3];
            }
        }
        fn fill(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: [u8; 4]) {
            for y in y0..(y0 + h) {
                for x in x0..(x0 + w) {
                    self.set(x, y, c);
                }
            }
        }
        fn outline(&mut self, x0: i32, y0: i32, w: i32, h: i32, c: [u8; 4]) {
            for i in 0..w {
                self.set(x0 + i, y0, c);
                self.set(x0 + i, y0 + h - 1, c);
            }
            for j in 0..h {
                self.set(x0, y0 + j, c);
                self.set(x0 + w - 1, y0 + j, c);
            }
        }
    }

    let bg: [u8; 4] = [245, 245, 245, 255];
    let line: [u8; 4] = [70, 110, 170, 255];
    let accent: [u8; 4] = [210, 130, 40, 255];
    let mut pen = Pen::new();
    pen.fill(0, 0, 48, 48, bg);
    match name {
        "layout_prev" | "layout_next" => {
            pen.outline(12, 6, 10, 10, line);
            pen.outline(26, 6, 10, 10, line);
            pen.outline(12, 20, 10, 10, line);
            pen.outline(26, 20, 10, 10, line);
            let next = name == "layout_next";
            for i in 0..8i32 {
                let x = if next { 8 + i } else { 38 - i };
                let y = 44 - (if next { i } else { 7 - i });
                pen.set(x, y, accent);
                pen.set(x, y - 1, accent);
            }
        }
        "layout_menu" => {
            pen.fill(10, 12, 28, 4, line);
            pen.fill(10, 22, 28, 4, line);
            pen.fill(10, 32, 28, 4, line);
        }
        "new_window" => {
            pen.outline(8, 12, 22, 22, line);
            pen.fill(9, 13, 20, 20, bg);
            pen.outline(20, 18, 20, 20, accent);
        }
        "view" => {
            for x in 8..40i32 {
                let t = x - 24;
                let dy = (16 - t * t / 8).max(0);
                pen.set(x, 24 - dy / 2, line);
                pen.set(x, 24 + dy / 2, line);
            }
            pen.fill(22, 20, 4, 8, accent);
        }
        "refresh" => {
            for x in 10..38i32 {
                let t = (x - 24) as i64;
                let d = t * t / 16;
                pen.set(x, 10 + d as i32, line);
                pen.set(x, 38 - d as i32, line);
            }
            pen.fill(32, 6, 6, 6, accent);
        }
        "go" => {
            for i in 0..14i32 {
                for j in 0..(14 - i) {
                    pen.set(14 + i, 10 + j, line);
                }
            }
        }
        "tree_toggle" => {
            pen.fill(10, 8, 3, 32, line);
            pen.fill(13, 12, 16, 3, line);
            pen.fill(13, 24, 24, 3, line);
            pen.fill(13, 34, 8, 3, line);
            pen.fill(30, 8, 6, 6, accent);
        }
        _ => {
            for i in 0..48i32 {
                pen.set(i, i, line);
            }
        }
    }
    pen.px.to_vec()
}

/// glyph 名列表 → 48x48 ImageList。
pub unsafe fn imagelist_from_glyphs(names: &[&str]) -> QResult<HIMAGELIST> {
    let himl = unsafe { ImageList_Create(48, 48, ILC_COLOR32 | ILC_MASK, names.len() as i32, 0) };
    if himl.is_invalid() {
        return Err(QError::Shell("ImageList_Create 失败".into()));
    }
    for n in names {
        let bits = glyph_bits(n);
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: 48,
            biHeight: -48,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };
        let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbm = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut ptr, None, 0) }
            .map_err(|e| QError::Shell(format!("CreateDIBSection: {e}")))?;
        unsafe {
            std::ptr::copy_nonoverlapping(bits.as_ptr(), ptr as *mut u8, bits.len());
            let _ = ImageList_Add(himl, hbm, None);
            let _ = DeleteObject(HGDIOBJ(hbm.0));
        }
    }
    Ok(himl)
}

/// 建主导航工具栏（48x48 glyph + 8 按钮，go 带下拉箭头）。
pub unsafe fn build_nav_toolbar(
    parent: HWND,
    hinstance: HINSTANCE,
    id: i32,
) -> QResult<(HWND, HIMAGELIST)> {
    let names: Vec<&str> = NAV_BUTTONS.iter().map(|(n, _, _)| *n).collect();
    let himl = unsafe { imagelist_from_glyphs(&names) }?;

    let tb = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            TOOLBARCLASSNAMEW,
            windows::core::w!(""),
            WS_VISIBLE
                | WS_CHILD
                | WS_CLIPSIBLINGS
                | WINDOW_STYLE(
                    TBSTYLE_FLAT | TBSTYLE_TOOLTIPS | CCS_NODIVIDER as u32 | CCS_NOPARENTALIGN as u32,
                ),
            0,
            0,
            48 * NAV_BUTTONS.len() as i32 + 8,
            56,
            Some(parent),
            Some(HMENU(id as _)),
            Some(hinstance),
            None,
        )
    }
    .map_err(|e| QError::Shell(format!("工具栏创建失败: {e}")))?;

    unsafe {
        SendMessageW(
            tb,
            TB_BUTTONSTRUCTSIZE,
            Some(WPARAM(core::mem::size_of::<TBBUTTON>())),
            None,
        );
        SendMessageW(
            tb,
            TB_SETIMAGELIST,
            Some(WPARAM(0)),
            Some(LPARAM(himl.0 as isize)),
        );
    }

    let btns: Vec<TBBUTTON> = NAV_BUTTONS
        .iter()
        .enumerate()
        .map(|(i, (_glyph, cid, split))| TBBUTTON {
            iBitmap: i as i32,
            idCommand: *cid,
            fsState: TBSTATE_ENABLED as u8,
            fsStyle: if *split { BTNS_DROPDOWN as u8 } else { BTNS_BUTTON as u8 },
            ..Default::default()
        })
        .collect();
    unsafe {
        SendMessageW(
            tb,
            TB_ADDBUTTONS,
            Some(WPARAM(btns.len())),
            Some(LPARAM(btns.as_ptr() as isize)),
        );
    }
    Ok((tb, himl))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_尺寸与表一致() {
        for (n, _, _) in NAV_BUTTONS {
            let bits = glyph_bits(n);
            assert_eq!(bits.len(), 48 * 48 * 4, "glyph {n}");
            // 全不透明
            assert!(bits.iter().skip(3).step_by(4).all(|&a| a == 255), "glyph {n}");
        }
    }

    #[test]
    fn glyph_未知名走对角线占位() {
        let bits = glyph_bits("nope");
        assert_eq!(bits.len(), 48 * 48 * 4);
        // 对角线上有主色线（G=110 在 index o+1）
        let o = (10 * 48 + 10) * 4;
        assert_eq!(bits[o], 170); // B of line color
    }
}
