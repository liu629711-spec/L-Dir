//! 选择性粘贴规划（P2 audit #22，menu_texts:43「选择性粘贴 Ctrl+Shift+V」）。
//!
//! 原版对话框语义未反编译确证 *待复核*；按菜单文案实现为：粘贴前勾选
//! 剪贴板条目 + 选同名冲突策略。三策略映射（FileOpOptions → FOF_*）：
//! - Ask：不预过滤、不加重命名/静默旗标 → SHFileOperation 弹系统冲突确认
//! - Rename：FOF_RENAMEONCOLLISION（自动改名）
//! - Skip：粘贴前把目标目录已存在同名项的条目剔除（纯函数可测）

use crate::shell::fileops::FileOpOptions;
use std::path::{Path, PathBuf};

/// 同名冲突策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteStrategy {
    /// 同名时询问（系统 SHFileOperation 确认框）。
    Ask,
    /// 自动重命名。
    Rename,
    /// 跳过同名项。
    Skip,
}

/// 目标目录中已存在的文件名集合（大小写不敏感，NTFS 语义）。
pub fn dest_names(dest: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dest) {
        for ent in rd.flatten() {
            out.push(ent.file_name().to_string_lossy().to_lowercase());
        }
    }
    out
}

/// 粘贴规划：勾选过滤 + 策略 → (实际粘贴路径, 操作选项)。
/// `checked` 长度不足时按 true 处理（对话框全勾默认）。
/// 返回 Err 仅当过滤后为空（无可粘贴项）。
pub fn plan_paste_special(
    paths: &[PathBuf],
    checked: &[bool],
    strategy: PasteStrategy,
    dest_names: &[String],
    cut: bool,
) -> QResult2<(Vec<PathBuf>, FileOpOptions)> {
    let selected: Vec<PathBuf> = paths
        .iter()
        .enumerate()
        .filter(|(i, _)| checked.get(*i).copied().unwrap_or(true))
        .map(|(_, p)| p.clone())
        .collect();
    if selected.is_empty() {
        return Err(PasteErr::Empty);
    }
    let selected = if strategy == PasteStrategy::Skip {
        selected
            .into_iter()
            .filter(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                !dest_names.iter().any(|d| *d == name)
            })
            .collect::<Vec<_>>()
    } else {
        selected
    };
    if selected.is_empty() {
        return Err(PasteErr::AllSkipped);
    }
    let opts = FileOpOptions {
        allow_undo: cut,
        silent: false,
        create_missing_dirs: true,
        rename_on_collision: strategy == PasteStrategy::Rename,
        move_confirmation: false,
    };
    Ok((selected, opts))
}

/// 本模块轻量错误（不进 QError 族——空选/全跳过属正常交互分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteErr {
    /// 一个都没勾。
    Empty,
    /// 策略 Skip 把全部条目剔掉了。
    AllSkipped,
}

pub type QResult2<T> = Result<T, PasteErr>;

#[cfg(test)]
mod tests {
    use super::*;

    fn p(dir: &Path, name: &str) -> PathBuf {
        dir.join(name)
    }

    #[test]
    fn 勾选过滤() {
        let d = Path::new("D:\\不存在\\假目录");
        let paths = vec![p(d, "a"), p(d, "b"), p(d, "c")];
        let (sel, _) = plan_paste_special(&paths, &[true, false, true], PasteStrategy::Ask, &[], false)
            .expect("可规划");
        assert_eq!(sel, vec![p(d, "a"), p(d, "c")]);
        // 长度不足按全勾。
        let (sel2, _) =
            plan_paste_special(&paths, &[false], PasteStrategy::Ask, &[], false).expect("可规划");
        assert_eq!(sel2.len(), 2, "checked[0]=false，其余按 true");
    }

    #[test]
    fn 全不勾拒绝() {
        let paths = vec![p(Path::new("D:\\"), "a")];
        assert_eq!(
            plan_paste_special(&paths, &[false, false], PasteStrategy::Ask, &[], false),
            Err(PasteErr::Empty)
        );
    }

    #[test]
    fn 策略映射到操作选项() {
        let paths = vec![p(Path::new("D:\\"), "a")];
        let (_, ask) =
            plan_paste_special(&paths, &[], PasteStrategy::Ask, &[], false).unwrap();
        assert!(!ask.rename_on_collision);
        let (_, ren) =
            plan_paste_special(&paths, &[], PasteStrategy::Rename, &[], false).unwrap();
        assert!(ren.rename_on_collision, "Rename→FOF_RENAMEONCOLLISION");
        let (_, cut_opts) =
            plan_paste_special(&paths, &[], PasteStrategy::Ask, &[], true).unwrap();
        assert!(cut_opts.allow_undo, "cut 语义→allow_undo（移动）");
    }

    #[test]
    fn 跳过同名剔除条目() {
        let dest = std::env::temp_dir().join(format!("qdir-rs-ps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("同名.txt"), "old").unwrap();
        let names = dest_names(&dest);
        let paths = vec![p(&dest, "同名.txt"), p(&dest, "新的.txt")];
        let (sel, _) =
            plan_paste_special(&paths, &[], PasteStrategy::Skip, &names, false).expect("可规划");
        assert_eq!(sel, vec![p(&dest, "新的.txt")], "同名项应被剔除");
        // 大小写不敏感：dst 的 A.TXT 剔掉同名 a.txt → 全跳过。
        std::fs::write(dest.join("A.TXT"), "x").unwrap();
        let names2 = dest_names(&dest);
        let paths2 = vec![p(&dest, "a.txt")];
        assert_eq!(
            plan_paste_special(&paths2, &[], PasteStrategy::Skip, &names2, false),
            Err(PasteErr::AllSkipped),
            "大小写不敏感匹配应全跳过"
        );
        // Ask/Rename 不剔除。
        let (sel3, _) =
            plan_paste_special(&paths2, &[], PasteStrategy::Ask, &names2, false).unwrap();
        assert_eq!(sel3.len(), 1);
        let _ = std::fs::remove_dir_all(&dest);
    }
}
