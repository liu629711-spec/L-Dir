//! 数据大小统计（P2 audit #45，menu_texts:102「数据大小信息 [F9]」）。
//!
//! 原版 F9 语义未反编译确证 *待复核*；按菜单文案实现为：对选中项
//! （无选中=当前目录）递归统计文件数/目录数/字节数并展示。符号链接
//! 不跟随（symlink_metadata），读不到的项跳过（权限目录常见）。

/// 递归统计结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirStats {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
}

impl DirStats {
    /// 逐项相加（纯函数）。
    pub fn add(&mut self, o: DirStats) {
        self.files += o.files;
        self.dirs += o.dirs;
        self.bytes += o.bytes;
    }
}

/// 单路径递归统计：文件=1 文件+其字节；目录=1 目录+递归。
/// 深度上限 64 防环（NTFS 连接点），越过按空统计。
pub fn dir_stats(path: &std::path::Path) -> DirStats {
    dir_stats_depth(path, 0)
}

fn dir_stats_depth(path: &std::path::Path, depth: u32) -> DirStats {
    if depth > 64 {
        return DirStats::default();
    }
    // 符号链接不跟随。
    let Ok(md) = std::fs::symlink_metadata(path) else {
        return DirStats::default();
    };
    if md.is_dir() {
        // 查询根自身不计目录数（只数子孙目录），F9 展示口径。
        let mut st = DirStats { dirs: if depth == 0 { 0 } else { 1 }, ..Default::default() };
        let Ok(rd) = std::fs::read_dir(path) else {
            return st;
        };
        for ent in rd.flatten() {
            st.add(dir_stats_depth(&ent.path(), depth + 1));
        }
        st
    } else {
        DirStats { files: 1, bytes: md.len(), ..Default::default() }
    }
}

/// 多路径聚合（选中项列表；纯函数入口走 dir_stats）。
pub fn stats_of_paths(paths: &[std::path::PathBuf]) -> DirStats {
    let mut st = DirStats::default();
    for p in paths {
        st.add(dir_stats(p));
    }
    st
}

/// 千分位字节展示（1 234 567 字节；KB/MB 换算避免界面零长串）。
pub fn fmt_bytes(n: u64) -> String {
    if n >= 1024 * 1024 * 1024 {
        format!("{:.2} GB ({} 字节)", n as f64 / (1024.0 * 1024.0 * 1024.0), n)
    } else if n >= 1024 * 1024 {
        format!("{:.2} MB ({} 字节)", n as f64 / (1024.0 * 1024.0), n)
    } else if n >= 1024 {
        format!("{:.2} KB ({} 字节)", n as f64 / 1024.0, n)
    } else {
        format!("{n} 字节")
    }
}

/// F9 信息文本（选中项逐行 + 合计；无选中时单行目录统计）。
/// 纯函数（输入=路径与名列表），UI 只负责 MessageBox 展示。
pub fn info_text(items: &[(String, std::path::PathBuf)], total_header: &str) -> String {
    if items.is_empty() {
        return format!("{total_header}：空");
    }
    let mut out = String::new();
    let mut total = DirStats::default();
    for (name, path) in items {
        let st = dir_stats(path);
        out.push_str(&format!(
            "{name}：{} 文件 / {} 目录 / {}\n",
            st.files, st.dirs, fmt_bytes(st.bytes)
        ));
        total.add(st);
    }
    out.push_str(&format!(
        "\n{total_header}：{} 文件 / {} 目录 / {}",
        total.files,
        total.dirs,
        fmt_bytes(total.bytes)
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 单文件统计() {
        let dir = std::env::temp_dir().join(format!("qdir-rs-si-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.txt");
        std::fs::write(&f, "12345").unwrap();
        let st = dir_stats(&f);
        assert_eq!(st, DirStats { files: 1, dirs: 0, bytes: 5 });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 目录递归统计() {
        let base = std::env::temp_dir().join(format!("qdir-rs-si2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let sub = base.join("子");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(base.join("a"), "12").unwrap();          // 2B 文件
        std::fs::write(sub.join("b"), "34567").unwrap();        // 5B 文件
        std::fs::create_dir_all(sub.join("深")).unwrap();       // 目录
        let st = dir_stats(&base);
        assert_eq!(st.files, 2);
        assert_eq!(st.dirs, 2); // 子 + 深
        assert_eq!(st.bytes, 7);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn 不存在路径空统计() {
        assert_eq!(dir_stats(std::path::Path::new("Z:\\no\\such\\path")), DirStats::default());
    }

    #[test]
    fn 字节格式化() {
        assert_eq!(fmt_bytes(0), "0 字节");
        assert_eq!(fmt_bytes(1023), "1023 字节");
        assert!(fmt_bytes(1024).starts_with("1.00 KB"));
        assert!(fmt_bytes(5 * 1024 * 1024).starts_with("5.00 MB"));
        assert!(fmt_bytes(2 * 1024 * 1024 * 1024).starts_with("2.00 GB"));
    }

    #[test]
    fn 信息文本含合计() {
        let dir = std::env::temp_dir().join(format!("qdir-rs-si3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("x.log");
        std::fs::write(&f, "abcd").unwrap();
        let text = info_text(&[("x.log".into(), f)], "合计");
        assert!(text.contains("x.log：1 文件"), "逐行缺失：{text}");
        assert!(text.ends_with("4 字节"), "合计缺失：{text}");
        assert_eq!(info_text(&[], "合计"), "合计：空");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
