//! 探测原版菜单模板字节能否直接交给 LoadMenuIndirectW（不经过自定义转换）。
//!
//! 与对话框走的路线一致：原字节原样复用。若 Windows 要求 DWORD 对齐，
//! 再退回到"对齐后原字节"。

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::LoadMenuIndirectW;

fn read_w(v: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([v[o], v[o + 1]])
}

fn read_str(v: &[u8], mut o: usize) -> (String, usize) {
    let mut s = Vec::new();
    while o + 1 < v.len() && !(v[o] == 0 && v[o + 1] == 0) {
        s.push(read_w(v, o));
        o += 2;
    }
    (String::from_utf16_lossy(&s), o + 2)
}

/// 按原模板结构走一遍，逐条目 DWORD 对齐后重发（保留原始 opt 位）。
fn rebuild_aligned(src: &[u8]) -> Vec<u16> {
    fn walk(src: &[u8], o: &mut usize, out: &mut Vec<u16>) {
        loop {
            if *o + 2 > src.len() {
                break;
            }
            // 每个条目起始处按 4 字节（2 个 u16）对齐。
            if out.len() % 2 != 0 {
                out.push(0);
            }
            let opt = read_w(src, *o);
            *o += 2;
            let is_popup = opt & 0x0010 != 0;
            let mut id = 0u16;
            if !is_popup {
                if *o + 2 > src.len() {
                    break;
                }
                id = read_w(src, *o);
                *o += 2;
            }
            let (text, no) = read_str(src, *o);
            *o = no;
            out.push(opt);
            if !is_popup {
                out.push(id);
            }
            out.extend(text.encode_utf16());
            out.push(0);
            if is_popup {
                walk(src, o, out);
            }
            if opt & 0x0080 != 0 {
                break;
            }
        }
    }
    let mut out = vec![0u16, 0]; // 头：ver=0, offset=0
    let mut o = 4usize;
    walk(src, &mut o, &mut out);
    out
}

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/extracted/4/128.bin");
    let bytes = std::fs::read(path).expect("读取菜单模板失败");
    println!("[probe] 模板 {} 字节", bytes.len());

    unsafe {
        println!("[probe] A. 原字节直接加载…");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let h = LoadMenuIndirectW(bytes.as_ptr() as *const core::ffi::c_void);
        match h {
            Ok(h) => println!("[probe] A. 成功 -> {h:?}"),
            Err(e) => println!("[probe] A. 失败 -> {e}"),
        }

        println!("[probe] B. 对齐后原字节加载…");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let aligned = rebuild_aligned(&bytes);
        println!("[probe] B. 重建 {} 个 u16", aligned.len());
        let h2 = LoadMenuIndirectW(aligned.as_ptr() as *const core::ffi::c_void);
        match h2 {
            Ok(h) => println!("[probe] B. 成功 -> {h:?}"),
            Err(e) => println!("[probe] B. 失败 -> {e}"),
        }
    }
    println!("[probe] 全部完成");
}
