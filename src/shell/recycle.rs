//! 回收站撤销删除（P1.1，FEATURE_GAP_AUDIT #18/#19；设计见
//! DEV_SPEC_UNDO_DELETE.md）。
//!
//! 机制：删除（FOF_ALLOWUNDO）前记录 `DeleteBatch { t0, 原路径列表 }`；
//! 撤销时在原路径所在盘的 `$Recycle.Bin\<SID>\` 下枚举 `$I` 元数据文件，
//! 按「原路径（大小写不敏感）+ 删除时刻 ≥ t0−SLACK」定位对应 `$R` 数据文件，
//! `MoveFileW` 改名回原路径。同盘改名即还原（时间戳/属性自动保留）。
//!
//! `$I` 格式（取证界公开稳定格式；本机 Win10/11 为 v2）：
//! - v1：[0..8) u64 版本=1 | [8..16) u64 大小 | [16..24) u64 删除时刻
//!   （FILETIME，UTC，100ns 单位）| [24..544) UTF-16LE 原完整路径
//!   （260 wchar，NUL 补齐）；
//! - v2：头部同 v1 至 24 字节，[24..28) u32 名长（字节，含 NUL——实测口径；
//!   解析按「读剩余全部、首个 NUL 截断」防御「字符数」口径），
//!   [28..) UTF-16LE 原完整路径。
//! - `$R` 与 `$I` 同名、I→R（`$I89YT3C.txt` ↔ `$R89YT3C.txt`）。
//!
//! 边界：盘无回收站/UNC 路径/找不到条目 → 逐路径失败原因上报，不中断其余；
//! 目标已存在 → `名 (n).扩展名` 唯一名，不覆盖（Explorer 同场景弹询问，
//! 这里自动改名 *待复核*）。

use std::path::{Path, PathBuf};

use windows::Win32::Storage::FileSystem::MoveFileW;

use super::{as_pcwstr, to_wide};
use crate::core::{QError, QResult};

/// 删除时刻匹配放宽量：1 秒（FILETIME 100ns 单位 = 1e7）。
/// t0 采于删除调用前，shell 打戳在删除时；放宽仅防时钟抖动。
const TIME_SLACK_100NS: u64 = 10_000_000;

/// 撤销栈深度上限（超出丢最旧；原版多级行为未确证 *待复核*）。
pub const UNDO_STACK_LIMIT: usize = 32;

/// 一次删除操作（一个批次）。t0 = 删除前采集的 FILETIME（100ns，UTC）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteBatch {
    pub t0_filetime: u64,
    pub paths: Vec<PathBuf>,
    /// 发起删除的窗格（撤销成功后重灌用）。
    pub pane: usize,
}

/// 一条回收站条目（一个 `$I` + 对应 `$R` 的解析结果）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecycleEntry {
    /// `$I` 元数据文件路径。
    pub i_path: PathBuf,
    /// `$R` 数据文件路径（同目录，I→R）。
    pub r_path: PathBuf,
    /// `$I` 内记录的原完整路径。
    pub original: String,
    /// 原大小（字节）。
    pub size: u64,
    /// 删除时刻（FILETIME，100ns，UTC）。
    pub deleted_filetime: u64,
}

/// 一批还原的结果：成功数 + 逐路径失败原因。
#[derive(Clone, Debug, Default)]
pub struct RestoreReport {
    pub restored: usize,
    pub failed: Vec<(PathBuf, String)>,
}

/// 当前时刻的 FILETIME（100ns，UTC）。std 时钟在 Windows 即系统 UTC 时钟，
/// 与 shell 删除打戳同源；精度（100ns 级）对本功能绰绰有余。
pub fn now_filetime() -> u64 {
    system_time_to_filetime(std::time::SystemTime::now())
}

/// `SystemTime` → FILETIME（100ns，UTC）。纯函数（单测已知值）。
pub fn system_time_to_filetime(t: std::time::SystemTime) -> u64 {
    // 1601→1970 秒差 11_644_473_600 × 1e7（100ns/秒）= 1.16444736e17。
    // （曾错写 100 倍致时间窗过滤全部真实条目——单测自我印证教训。）
    const EPOCH_DELTA_100NS: u64 = 116_444_736_000_000_000;
    match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => EPOCH_DELTA_100NS
            .saturating_add(d.as_secs().saturating_mul(10_000_000))
            .saturating_add((d.subsec_nanos() / 100) as u64),
        Err(_) => 0,
    }
}

/// 小端读 u64（不足返回 None）。
fn le_u64(b: &[u8], off: usize) -> Option<u64> {
    if b.len() < off + 8 {
        return None;
    }
    Some(u64::from_le_bytes(b[off..off + 8].try_into().unwrap()))
}

/// UTF-16LE 字节串 → String，首个 NUL 截断（非法序列以替换符降级，不失败）。
fn utf16le_until_nul(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// 解析一个 `$I` 元数据文件内容（纯函数）。
/// 返回 (原完整路径, 原大小, 删除时刻 FILETIME)。版本不认/截断 → None。
pub fn parse_i_file(bytes: &[u8]) -> Option<(String, u64, u64)> {
    let version = le_u64(bytes, 0)?;
    let size = le_u64(bytes, 8)?;
    let deleted = le_u64(bytes, 16)?;
    let name_region: &[u8] = match version {
        1 => {
            // v1：名固定 260 wchar（520 字节）区，NUL 补齐。
            if bytes.len() < 24 + 520 {
                return None;
            }
            &bytes[24..24 + 520]
        }
        2 => {
            // v2：名长 u32 在 24；字节口径含 NUL，字符口径不含——统一读
            // 剩余全部再截 NUL，两种口径都得到正确名。
            if bytes.len() < 28 {
                return None;
            }
            &bytes[28..]
        }
        _ => return None,
    };
    let original = utf16le_until_nul(name_region);
    if original.is_empty() {
        return None;
    }
    Some((original, size, deleted))
}

/// `$R` 路径：同目录同名、首段 `$I` → `$R`。
pub fn r_path_of(i_path: &Path) -> PathBuf {
    let name = i_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let renamed = if let Some(stripped) = name.strip_prefix("$I") {
        format!("$R{stripped}")
    } else {
        name.to_string()
    };
    i_path.with_file_name(renamed)
}

/// 路径的盘锚（`C:\a\b` → `C:\`；UNC/Verbatim/相对路径 → None——
/// UNC 删除不进回收站，无从还原）。
pub fn drive_anchor_of(p: &Path) -> Option<PathBuf> {
    use std::path::{Component, Prefix};
    match p.components().next() {
        Some(Component::Prefix(pc)) => match pc.kind() {
            Prefix::Disk(letter) => Some(PathBuf::from(format!("{}:\\", letter as char))),
            _ => None,
        },
        _ => None,
    }
}

/// 目标已占用时的唯一名：`名 (n).扩展名` 从 1 递增；无扩展名回落 `名 (n)`。
/// 不占用时原样返回。
pub fn unique_restore_name(target: &Path) -> PathBuf {
    if !target.exists() {
        return target.to_path_buf();
    }
    let parent = target.parent().unwrap_or(Path::new("."));
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_string();
    let ext = target.extension().and_then(|e| e.to_str()).map(|e| e.to_string());
    for n in 1..=999u32 {
        let name = match &ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let cand = parent.join(name);
        if !cand.exists() {
            return cand;
        }
    }
    target.to_path_buf()
}

/// 判断候选是否匹配 (原路径, t0)：大小写不敏感 + 时刻窗。
fn entry_matches(e: &RecycleEntry, path: &Path, t0: u64) -> bool {
    e.original
        .eq_ignore_ascii_case(&path.to_string_lossy())
        && e.deleted_filetime + TIME_SLACK_100NS >= t0
}

/// 从候选取删除时刻最大的匹配项（同路径多次删除只还原最新）。
pub fn pick_restore_entry<'a>(
    path: &Path,
    t0: u64,
    candidates: &'a [RecycleEntry],
) -> Option<&'a RecycleEntry> {
    candidates
        .iter()
        .filter(|e| entry_matches(e, path, t0))
        .max_by_key(|e| e.deleted_filetime)
}

/// 枚举一个盘的回收站条目：`<anchor>\$Recycle.Bin\<*\>\$I*`。
/// SID 子目录读失败（他人回收站）跳过；只收删除时刻 ≥ t0−SLACK 的条目。
pub fn scan_drive_bin(anchor: &Path, t0: u64) -> Vec<RecycleEntry> {
    let bin_root = anchor.join("$Recycle.Bin");
    let floor = t0.saturating_sub(TIME_SLACK_100NS);
    let mut out = Vec::new();
    let Ok(sids) = std::fs::read_dir(&bin_root) else {
        return out;
    };
    for sid in sids.flatten() {
        if !sid.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(files) = std::fs::read_dir(sid.path()) else {
            continue;
        };
        for f in files.flatten() {
            let name = f.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("$I") {
                continue;
            }
            let Ok(bytes) = std::fs::read(f.path()) else {
                continue;
            };
            let Some((original, size, deleted)) = parse_i_file(&bytes) else {
                continue;
            };
            if deleted < floor {
                continue;
            }
            out.push(RecycleEntry {
                r_path: r_path_of(&f.path()),
                i_path: f.path(),
                original,
                size,
                deleted_filetime: deleted,
            });
        }
    }
    out
}

/// 还原一个删除批次（Ctrl+Z 主体）。逐路径独立，失败不中断其余。
pub fn restore_batch(batch: &DeleteBatch) -> RestoreReport {
    let mut report = RestoreReport::default();
    let mut cache: std::collections::HashMap<PathBuf, Vec<RecycleEntry>> =
        std::collections::HashMap::new();
    for path in &batch.paths {
        let Some(anchor) = drive_anchor_of(path) else {
            report
                .failed
                .push((path.clone(), "非盘符/相对路径，无回收站".into()));
            continue;
        };
        let entries = cache.entry(anchor.clone()).or_insert_with(|| {
            // SAFETY: anchor 由路径盘符构造，合法。
            scan_drive_bin(&anchor, batch.t0_filetime)
        });
        let Some(entry) = pick_restore_entry(path, batch.t0_filetime, entries) else {
            report
                .failed
                .push((path.clone(), "回收站未找到匹配条目".into()));
            continue;
        };
        let r = entry.r_path.clone();
        if !r.exists() {
            report
                .failed
                .push((path.clone(), format!("$R 数据文件缺失：{}", r.display())));
            continue;
        }
        let target = unique_restore_name(path);
        let target_w = to_wide(&target.to_string_lossy());
        let r_w = to_wide(&r.to_string_lossy());
        // SAFETY: 两侧宽串存活至调用返回；MoveFileW 同盘=改名（元数据保留）。
        let ok = unsafe { MoveFileW(as_pcwstr(&r_w), as_pcwstr(&target_w)) };
        if ok.is_err() {
            report.failed.push((
                path.clone(),
                format!("MoveFileW 失败：{}", super::last_error_code()),
            ));
            continue;
        }
        report.restored += 1;
    }
    report
}

/// 便捷封装：把 `restore_batch` 失败原因转 QError（整批全失败时返回 Err）。
pub fn restore_batch_checked(batch: &DeleteBatch) -> QResult<RestoreReport> {
    let r = restore_batch(batch);
    if r.restored == 0 && !r.failed.is_empty() {
        let msg = r
            .failed
            .iter()
            .map(|(p, why)| format!("{}: {why}", p.display()))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(QError::Io { code: 0, msg });
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_i_v1_固定520字节名区() {
        let mut b = vec![0u8; 24 + 520];
        b[0..8].copy_from_slice(&1u64.to_le_bytes());
        b[8..16].copy_from_slice(&12345u64.to_le_bytes());
        b[16..24].copy_from_slice(&0x01D9_ABCD_1234u64.to_le_bytes());
        let name: Vec<u16> = "C:\\foo\\bar.txt".encode_utf16().collect();
        for (i, u) in name.iter().enumerate() {
            b[24 + i * 2..26 + i * 2].copy_from_slice(&u.to_le_bytes());
        }
        let (original, size, deleted) = parse_i_file(&b).expect("v1 应可解析");
        assert_eq!(original, "C:\\foo\\bar.txt");
        assert_eq!(size, 12345);
        assert_eq!(deleted, 0x01D9_ABCD_1234);
    }

    #[test]
    fn parse_i_v2_名长含nul两种口径都取对名() {
        let name: Vec<u16> = "C:\\测 试.txt".encode_utf16().collect();
        // 口径 A：名长 = 字节数（含 NUL）= (chars+1)*2。
        let mut a = vec![0u8; 28];
        a[0..8].copy_from_slice(&2u64.to_le_bytes());
        a[24..28].copy_from_slice(&((name.len() as u32 + 1) * 2).to_le_bytes());
        for u in &name {
            a.extend_from_slice(&u.to_le_bytes());
        }
        a.extend_from_slice(&0u16.to_le_bytes());
        // 口径 B：名长 = 字符数（不含 NUL）。
        let mut b = a.clone();
        b[24..28].copy_from_slice(&(name.len() as u32).to_le_bytes());
        for (label, data) in [("字节口径", a), ("字符口径", b)] {
            let (original, _, _) = parse_i_file(&data).unwrap_or_else(|| panic!("{label} 解析失败"));
            assert_eq!(original, "C:\\测 试.txt", "{label}");
        }
    }

    #[test]
    fn parse_i_截断与坏版本返回none() {
        assert_eq!(parse_i_file(&[]), None);
        assert_eq!(parse_i_file(&vec![0u8; 23]), None, "头部不足 24");
        let mut v1 = vec![0u8; 24 + 520];
        v1[0..8].copy_from_slice(&1u64.to_le_bytes());
        assert_eq!(parse_i_file(&v1[..24 + 519]), None, "v1 名区差 1 字节");
        let mut bad = vec![0u8; 24 + 520];
        bad[0..8].copy_from_slice(&7u64.to_le_bytes());
        assert_eq!(parse_i_file(&bad), None, "未知版本");
        // v2 空名 → None。
        let mut v2 = vec![0u8; 28];
        v2[0..8].copy_from_slice(&2u64.to_le_bytes());
        assert_eq!(parse_i_file(&v2), None);
    }

    #[test]
    fn r路径同名i换r() {
        assert_eq!(
            r_path_of(Path::new(r"C:\$Recycle.Bin\S-1-5-21-x\$I89YT3C.txt")),
            PathBuf::from(r"C:\$Recycle.Bin\S-1-5-21-x\$R89YT3C.txt")
        );
        // 无扩展名（目录条目）同样成立。
        assert_eq!(
            r_path_of(Path::new(r"C:\$Recycle.Bin\S-1-5-21-x\$IAB12CD")),
            PathBuf::from(r"C:\$Recycle.Bin\S-1-5-21-x\$RAB12CD")
        );
        // 非 $I 前缀原样（防御，不产生于真实扫描）。
        assert_eq!(
            r_path_of(Path::new(r"C:\x\y.txt")),
            PathBuf::from(r"C:\x\y.txt")
        );
    }

    #[test]
    fn 盘锚与相对路径() {
        assert_eq!(
            drive_anchor_of(Path::new(r"C:\a\b")),
            Some(PathBuf::from(r"C:\"))
        );
        assert_eq!(drive_anchor_of(Path::new(r"C:\")), Some(PathBuf::from(r"C:\")));
        assert_eq!(drive_anchor_of(Path::new(r"a\b")), None, "相对路径无锚");
        assert_eq!(drive_anchor_of(Path::new("")), None);
        // UNC 不进回收站（删除即永久），锚=None。
        assert_eq!(drive_anchor_of(Path::new(r"\\srv\share\x")), None);
    }

    #[test]
    fn pick_取最大时刻并拒旧时间窗() {
        let path = Path::new(r"C:\t\foo.txt");
        let mk = |t: u64| RecycleEntry {
            i_path: PathBuf::new(),
            r_path: PathBuf::new(),
            original: r"C:\t\foo.txt".into(),
            size: 1,
            deleted_filetime: t,
        };
        let cands = vec![mk(1_000), mk(9_000), mk(5_000)];
        // t0=9500：两条在窗内（≥8.5e6 窗下限全部通过——这里用小值验证
        // 「取最大」逻辑，SLACK 语义由下一条锁）。
        let pick = pick_restore_entry(path, 9_500, &cands).map(|e| e.deleted_filetime);
        assert_eq!(pick, Some(9_000));
        // 大小写不敏感。
        let upper = RecycleEntry {
            original: r"C:\T\FOO.TXT".into(),
            ..mk(9_000)
        };
        assert!(pick_restore_entry(path, 9_500, &[upper]).is_some());
        // 时刻早于 t0−SLACK 的旧条目被拒（同路径先删后还原再删场景）。
        assert!(
            pick_restore_entry(path, 9_000 + TIME_SLACK_100NS + 1, &[mk(9_000)]).is_none(),
            "早于窗下限的条目不得还原"
        );
        // 路径不同不匹配。
        let other = RecycleEntry {
            original: r"C:\t\bar.txt".into(),
            ..mk(9_000)
        };
        assert!(pick_restore_entry(path, 9_500, &[other]).is_none());
    }

    #[test]
    fn unique还原名_占用递增与无扩展名() {
        let dir = std::env::temp_dir().join(format!("qdir-rs-uniq-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录失败");
        let t = dir.join("x.txt");
        std::fs::write(&t, b"a").unwrap();
        assert_eq!(unique_restore_name(&t), dir.join("x (1).txt"), "占用→(1)");
        std::fs::write(dir.join("x (1).txt"), b"a").unwrap();
        assert_eq!(unique_restore_name(&t), dir.join("x (2).txt"), "(1) 占用→(2)");
        // 未占用原样。
        let free = dir.join("free.txt");
        assert_eq!(unique_restore_name(&free), free);
        // 无扩展名。
        let ne = dir.join("y");
        std::fs::write(&ne, b"a").unwrap();
        assert_eq!(unique_restore_name(&ne), dir.join("y (1)"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 文件时间换算已知值() {
        use std::time::{Duration, UNIX_EPOCH};
        // 1601→1970：0 时刻 = 11_644_473_600s × 1e7 = 116_444_736_000_000_000。
        // 独立核对：真实 $I 的 ft≈1.34e17 → 减此常数 ≈ 17.9e8 s ≈ 2026 年，吻合。
        assert_eq!(
            system_time_to_filetime(UNIX_EPOCH),
            116_444_736_000_000_000
        );
        assert_eq!(
            system_time_to_filetime(UNIX_EPOCH + Duration::from_secs(1)),
            116_444_736_000_000_000 + 10_000_000
        );
        // now() 落在合理带宽（1970 后、2034 前）——锁数量级，
        // 100× 类错误（1.16e19）必然越界，又不绑死当前年份。
        let now = now_filetime();
        assert!(
            (116_444_736_000_000_000..200_000_000_000_000_000).contains(&now),
            "now_filetime 异常：{now}"
        );
    }
}
