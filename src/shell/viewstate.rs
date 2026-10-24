//! 原版 ini `Column_OS_10.0_PloderN` 段 `Spatlen_*` 视图状态 blob 解码
//! （P2.6，2026-10-24 取证，参照 D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir.ini）。
//!
//! 取证结论（字节实测）：
//! 1. ini 值是 **%XX 百分号转义的字节串**（`%1C%00…`），`%` 后两个十六进制
//!    位一个字节；解码后 Ploder1 为 **798 字节**。
//! 2. 798 = 772 净载荷 + 26 字节 ASCII 尾标 `|CODEMODE1|-1905896973|772`：
//!    `|CODEMODE<版本>|<i32 校验值>|<净载荷字节数>`。校验算法未确证
//!    *待复核*——导入只按声明长度切载荷，不校验。`Spatlen_329` 同结构
//!    （797=772+25，校验值不同），两键按同格式处理。
//! 3. 净载荷是 **shell 视图状态属性袋**（"1SPS" 魔数 + FMTID
//!    d5cdd505-2e9c-101b-9397-08002b2cf9ae）。条目**不做 4 字节对齐**
//!    （逐键紧排，各键起点奇偶不一——「错位」系打包方式而非损坏），
//!    因此解析策略是**锚定键名**而非游标步进：
//!    找到 UTF-16LE 键名 + NUL 后，紧跟 u32 VARIANT 类型，标量类型
//!    （VT_I4=3 / VT_UI4=19 / VT_BOOL=11）取随后 u32（BOOL 取低 16 位）。
//!    六个标量键全部按此规则读通，其中 FFlags/IconSize 与既有 core
//!    默认值 1075838981/16 完全吻合（交叉验证）。
//!
//! R1：本模块不内嵌任何原版字节——测试用**按取证版式自造**的载荷。

use crate::core::{ColumnViewState, QError, QResult};

/// 百分号转义解码（`%XX` → 字节；非转义字符取其 Unicode 码点低字节，
/// 实测值只含 ASCII 与 `%`）。
pub fn unescape_pct(v: &str) -> Vec<u8> {
    let b = v.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 3 + 4);
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(x) = u8::from_str_radix(&v[i + 1..i + 3], 16) {
                out.push(x);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// 切掉 `|CODEMODE…|…|NNN` 尾标，返回 (净载荷, 声明长度)。
/// 无尾标时原样返回、声明长度 None（前向兼容）。
pub fn split_codemode(raw: &[u8]) -> (&[u8], Option<u32>) {
    if let Some(pos) = raw.windows(11).rposition(|w| w == b"|CODEMODE1|") {
        let payload = &raw[..pos];
        let trailer = &raw[pos + 11..];
        // 尾标形如 `<i32>|<十进制长度>`：最后一段是长度。
        let declared = trailer
            .rsplit(|c| *c == b'|')
            .next()
            .and_then(|s| std::str::from_utf8(s).ok())
            .and_then(|s| s.trim_end_matches('\0').parse::<u32>().ok());
        (payload, declared)
    } else {
        (raw, None)
    }
}

/// 属性袋标量键扫描：锚定 UTF-16LE 键名 + NUL，读 u32 vt，
/// 标量类型取随后 u32。返回 (键名, 整数值)（VT_BOOL 的整数即 0/-1）。
/// 非标量键（BSTR/STREAM/CLSID 等复合键）跳过不产出。
pub fn extract_scalar_props(payload: &[u8]) -> Vec<(String, i64)> {
    // VT 常量（wtypes.h）。
    const VT_I4: u32 = 3;
    const VT_UI4: u32 = 19;
    const VT_BOOL: u32 = 11;
    let mut out = Vec::new();
    // 逐字节找候选键名：连续可打印 UTF-16LE 双元组 + NUL 终结。
    let mut i = 0usize;
    while i + 1 < payload.len() {
        // 尝试在 i 处读一个「可打印串 + NUL」，随后必须紧跟合法 vt。
        if let Some((name, after_name)) = read_name_at(payload, i) {
            if after_name + 8 <= payload.len() {
                let vt = u32::from_le_bytes([
                    payload[after_name],
                    payload[after_name + 1],
                    payload[after_name + 2],
                    payload[after_name + 3],
                ]);
                if matches!(vt, VT_I4 | VT_UI4 | VT_BOOL) {
                    let v = u32::from_le_bytes([
                        payload[after_name + 4],
                        payload[after_name + 5],
                        payload[after_name + 6],
                        payload[after_name + 7],
                    ]);
                    // VT_BOOL 是 16 位 VARIANT_BOOL + 2 字节填充（实测
                    // GroupView=ff ff 00 00），取低 16 位有符号值。
                    let val: i64 = if vt == VT_BOOL {
                        (v as u16) as i16 as i64
                    } else {
                        v as i32 as i64
                    };
                    // 只收「像属性名」的键（1..64 字符、ASCII 可打印），
                    // 防止二进制偶然成串。
                    let printable = !name.is_empty()
                        && name.len() <= 64
                        && name.bytes().all(|c| (0x20..0x7f).contains(&c));
                    if printable {
                        out.push((name, val));
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// 在 `payload[i..]` 处读 `UTF-16LE 可打印串 + NUL`（键名形态），
/// 返回 (ASCII 键名, 名后偏移)。串内出现非 ASCII 双元组即失败。
fn read_name_at(payload: &[u8], start: usize) -> Option<(String, usize)> {
    let mut chars = Vec::new();
    let mut j = start;
    while j + 1 < payload.len() {
        let c = u16::from_le_bytes([payload[j], payload[j + 1]]);
        if c == 0 {
            return if chars.is_empty() {
                None
            } else {
                Some((String::from_utf8(chars).ok()?, j + 2))
            };
        }
        if !(0x20..0x7f).contains(&c) {
            return None;
        }
        chars.push(c as u8);
        j += 2;
    }
    None
}

/// 六个已确证标量键 → [`ColumnViewState`]（缺键回落 Default；
/// `GroupByKey:PID` 等复合键不进结构，留档在 core 文档）。
pub fn column_view_state_from_props(props: &[(String, i64)]) -> ColumnViewState {
    let mut st = ColumnViewState::default();
    for (k, v) in props {
        match k.as_str() {
            "IconSize" => st.icon_size = *v as u32,
            "FFlags" => st.folder_flags = *v as u32,
            "LogicalViewMode" => st.logical_view_mode = *v as u32,
            "GroupView" => st.group = *v != 0,
            "GroupByDirection" => st.group_by_dir = *v as u32,
            _ => {}
        }
    }
    st
}

/// 一条 `Spatlen_*` ini 值 → 解码视图状态（全链）。
pub fn column_view_state_from_blob(value: &str) -> QResult<ColumnViewState> {
    let raw = unescape_pct(value);
    let (payload, _declared) = split_codemode(&raw);
    if payload.len() < 0x40 {
        return Err(QError::config(
            "Spatlen",
            format!("载荷过短：{} 字节", payload.len()),
        ));
    }
    let props = extract_scalar_props(payload);
    Ok(column_view_state_from_props(&props))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按取证版式自造一条属性袋载荷（结构=1SPS 头 + 紧排标量键，
    /// **非 4 字节对齐**：故意用奇数长度键名复刻实测的奇偶错位）。
    fn synth_payload() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(&0x1c_u32.to_le_bytes()); // 头长
        p.extend_from_slice(&[0u8; 24]);
        p.extend_from_slice(&[0xf1, 0xf1, 0xf1, 0xf1]);
        p.extend_from_slice(&0x14_u32.to_le_bytes());
        p.extend_from_slice(&[0u8; 8]);
        p.extend_from_slice(b"1SPS"); // 魔数
        p.extend_from_slice(&[0x05, 0xd5, 0xcd, 0xd5]); // FMTID 尾 4 字节（自造截断）
        // 键1（9 字符，名+vt+value 起点保持偶偏移）：
        let put_key = |p: &mut Vec<u8>, name: &str, vt: u32, v: u32| {
            for u in name.encode_utf16() {
                p.extend_from_slice(&u.to_le_bytes());
            }
            p.extend_from_slice(&0u16.to_le_bytes());
            p.extend_from_slice(&vt.to_le_bytes());
            p.extend_from_slice(&v.to_le_bytes());
        };
        put_key(&mut p, "IconSize", 19, 16);
        put_key(&mut p, "FFlags", 19, 0x4020_0005);
        put_key(&mut p, "GroupView", 11, 0x0000_ffff); // VARIANT_BOOL true（ff ff 00 00 低16位）
        put_key(&mut p, "LogicalViewMode", 19, 1);
        put_key(&mut p, "GroupByDirection", 19, 1);
        // 复合键（BSTR）：只应被跳过。
        let bstr = "{B725F130-47EF-101A-A5F1-02608C9EEBAC}";
        for u in "GroupByKey:FMTID".encode_utf16() {
            p.extend_from_slice(&u.to_le_bytes());
        }
        p.extend_from_slice(&0u16.to_le_bytes());
        p.extend_from_slice(&8_u32.to_le_bytes());
        p.extend_from_slice(&(bstr.len() as u32 * 2).to_le_bytes());
        for u in bstr.encode_utf16() {
            p.extend_from_slice(&u.to_le_bytes());
        }
        p
    }

    #[test]
    fn 百分号转义解码() {
        assert_eq!(unescape_pct("%1C%00%41"), vec![0x1c, 0x00, 0x41]);
        assert_eq!(unescape_pct("AB"), b"AB".to_vec());
        assert_eq!(unescape_pct("%zz"), b"%zz".to_vec()); // 非法转义原样
        assert_eq!(unescape_pct("%1"), b"%1".to_vec()); // 截断转义原样
    }

    #[test]
    fn 尾标切分与声明长度() {
        let mut raw = synth_payload();
        raw.extend_from_slice(b"|CODEMODE1|-1905896973|772");
        let (payload, declared) = split_codemode(&raw);
        assert_eq!(declared, Some(772));
        assert_eq!(payload.len(), raw.len() - 26);
        // 无尾标：原样。
        let bare = synth_payload();
        let (p2, d2) = split_codemode(&bare);
        assert_eq!(d2, None);
        assert_eq!(p2.len(), bare.len());
    }

    #[test]
    fn 标量键提取六键() {
        let props = extract_scalar_props(&synth_payload());
        let get = |k: &str| props.iter().find(|(n, _)| n == k).map(|(_, v)| *v);
        assert_eq!(get("IconSize"), Some(16));
        assert_eq!(get("FFlags"), Some(0x4020_0005));
        assert_eq!(get("GroupView"), Some(-1), "VARIANT_BOOL true = -1");
        assert_eq!(get("LogicalViewMode"), Some(1));
        assert_eq!(get("GroupByDirection"), Some(1));
        assert!(get("GroupByKey:FMTID").is_none(), "BSTR 复合键不产出");
    }

    #[test]
    fn 全链_转义到结构体() {
        let mut raw = synth_payload();
        raw.extend_from_slice(b"|CODEMODE1|-563719693|772");
        let mut escaped = String::new();
        for b in &raw {
            escaped.push_str(&format!("%{b:02X}"));
        }
        let st = column_view_state_from_blob(&escaped).expect("可解析");
        assert_eq!(st.icon_size, 16);
        assert_eq!(st.folder_flags, 0x4020_0005);
        assert!(st.group);
        assert_eq!(st.group_by_dir, 1);
        assert_eq!(st.logical_view_mode, 1);
        // 与 core 交叉验证：取证值与默认值一致（除 group 外）。
        let d = ColumnViewState::default();
        assert_eq!(st.icon_size, d.icon_size);
        assert_eq!(st.folder_flags, d.folder_flags);
    }

    #[test]
    fn 短载荷拒绝() {
        let r = column_view_state_from_blob("%01%02%03");
        assert!(r.is_err());
    }

    #[test]
    fn 缺键回落默认() {
        let mut p = Vec::new();
        for u in "IconSize".encode_utf16() {
            p.extend_from_slice(&u.to_le_bytes());
        }
        p.extend_from_slice(&0u16.to_le_bytes());
        p.extend_from_slice(&19_u32.to_le_bytes());
        p.extend_from_slice(&32_u32.to_le_bytes());
        let st = column_view_state_from_props(&extract_scalar_props(&p));
        assert_eq!(st.icon_size, 32);
        assert_eq!(st.folder_flags, ColumnViewState::default().folder_flags);
    }
}
