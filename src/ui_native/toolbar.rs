//! 工具栏图标管线：原版 PNG 资源 → 32bpp ImageList → ToolbarWindow32。
//!
//! 规格来自 REA 反编译 FUN_00429bc4（0x429bc4）：按文件名加载 PNG
//! （z6/v6/upF/new/view/update/go/viewtree，48x48），再按命令 id 逐个挂到
//! 主导航工具栏；dd/dd_r/dd_w（24x48）为下拉箭头。
//! PNG 资源名是 ASCII 按 UTF-16LE 存储的，提取时显示为乱码宽串——
//! 已规整到 assets/pngs/ 小写文件名。
//!
//! id↔图 映射（0xA76C/6D/71/72/75/70/73 与 0xB379，FUN_00429ac0 第四参数=位）：
//! 位0=z6(42860) 位1=v6(42861) 位2=upF(42865) 位3=new(42866) 位4=view(42869)
//! 位5=update(42864) 位6=go(45945,SplitButton) 位7=viewtree(42867)。

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::core::{QError, QResult};

/// 主导航按钮表：(png 文件, 命令 id, 是否 SplitButton)。
/// 顺序 = FUN_00429bc4 数组序（原版工具栏从左到右）。
pub const NAV_BUTTONS: &[(&str, i32, bool)] = &[
    ("z6.png", 0xA76C, false),
    ("v6.png", 0xA76D, false),
    ("upf.png", 0xA771, false),
    ("new.png", 0xA772, false),
    ("view.png", 0xA775, false),
    ("update.png", 0xA770, false),
    ("go.png", 0xB379, true), // +0x40=1：带下拉箭头（dd.png）
    ("viewtree.png", 0xA773, false),
];

/// 内嵌 PNG（assets/pngs 小写名 → 原版资源）。
pub fn png_bytes(name: &str) -> Option<&'static [u8]> {
    let table: &[(&str, &[u8])] = &[
        ("z6.png", include_bytes!("../../assets/pngs/z6.png")),
        ("v6.png", include_bytes!("../../assets/pngs/v6.png")),
        ("upf.png", include_bytes!("../../assets/pngs/upf.png")),
        ("new.png", include_bytes!("../../assets/pngs/new.png")),
        ("view.png", include_bytes!("../../assets/pngs/view.png")),
        ("update.png", include_bytes!("../../assets/pngs/update.png")),
        ("go.png", include_bytes!("../../assets/pngs/go.png")),
        ("viewtree.png", include_bytes!("../../assets/pngs/viewtree.png")),
        ("plus.png", include_bytes!("../../assets/pngs/plus.png")),
        ("dd.png", include_bytes!("../../assets/pngs/dd.png")),
        ("dd_r.png", include_bytes!("../../assets/pngs/dd_r.png")),
        ("dd_w.png", include_bytes!("../../assets/pngs/dd_w.png")),
    ];
    let key = name.to_ascii_lowercase();
    table.iter().find(|(k, _)| *k == key).map(|(_, b)| *b)
}

/// PNG 字节 → (w, h, BGRA 自上而下)。
pub fn png_to_bgra(bytes: &[u8]) -> QResult<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| QError::Config { key: "png".into(), msg: format!("解码失败: {e}") })?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let mut bgra = vec![0u8; (w as usize) * (h as usize) * 4];
    for (i, px) in img.pixels().enumerate() {
        let o = i * 4;
        bgra[o] = px[2];
        bgra[o + 1] = px[1];
        bgra[o + 2] = px[0];
        bgra[o + 3] = px[3];
    }
    Ok((w, h, bgra))
}

/// PNG → 32bpp 带 alpha 掩码的 ImageList。
///
/// 全部 PNG 按各自原尺寸直接加入（原版 ImageList 尺寸取最大项）；
/// 尺寸不一致时 ImageList 会居中绘制，无需预缩放。
pub unsafe fn imagelist_from_pngs(names: &[&str]) -> QResult<HIMAGELIST> {
    let mut pngs: Vec<Vec<u8>> = Vec::new();
    let (mut max_w, mut max_h) = (16u32, 16u32);
    for n in names {
        let bytes = png_bytes(n).ok_or_else(|| QError::Config { key: "png".into(), msg: format!("资源缺失: {n}") })?;
        let (w, h, bgra) = png_to_bgra(bytes)?;
        max_w = max_w.max(w);
        max_h = max_h.max(h);
        pngs.push(bgra);
        // 尺寸与元信息一并传递：用 (w,h) 前缀存进 Vec 不便，改为重解一次。
        let _ = (w, h);
    }
    let himl = unsafe {
        ImageList_Create(
            max_w as i32,
            max_h as i32,
            ILC_COLOR32 | ILC_MASK,
            pngs.len() as i32,
            0,
        )
    };
    if himl.is_invalid() {
        return Err(QError::Shell("ImageList_Create 失败".into()));
    }
    for (n, bgra) in names.iter().zip(pngs.iter()) {
        let bytes = png_bytes(n).unwrap();
        let (w, h, _) = png_to_bgra(bytes)?;
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbm = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) }
            .map_err(|e| QError::Shell(format!("CreateDIBSection: {e}")))?;
        unsafe {
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
            let _ = ImageList_Add(himl, hbm, None);
            let _ = DeleteObject(HGDIOBJ(hbm.0));
        }
    }
    Ok(himl)
}

/// 建原版主导航工具栏（48x48 图标 + 8 个布局切换按钮，go 带下拉箭头）。
pub unsafe fn build_nav_toolbar(
    parent: HWND,
    hinstance: HINSTANCE,
    id: i32,
) -> QResult<(HWND, HIMAGELIST)> {
    let names: Vec<&str> = NAV_BUTTONS.iter().map(|(n, _, _)| *n).collect();
    let himl = unsafe { imagelist_from_pngs(&names) }?;

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
        .map(|(i, (_png, cid, split))| TBBUTTON {
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
