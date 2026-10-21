//! 收藏夹与快速链接：管理 `Favoriten` 与 `Favoriten\Quick-Link` 目录。
//!
//! 证据（原版 `Q-Dir_x64.exe` 与 `Q-Dir.ini` 实测）：
//! - ini `[Favoriten] Ordner=.\Favoriten\`、`[Q] Link=.\Favoriten\Quick-Link\`
//!   （`.rdata` 0x1269c8 / 0x1269f8 / 0x126a20 亦有同名字符串常量）；
//! - 配置键 `Fav_Filter`（0x12f860）——收藏夹菜单的过滤串；
//! - 菜单 216 德语原文：`&Favoriten` / `Zu Favoriten hinzufügen` /
//!   `&Verwalten` / `Im neuem Fenster mit Strg`；
//! - `.rdata` 0x12f7f0 的 `Quick-Links` 与 0x12c1a8 的 `\Favorites\`。
//!
//! 实现边界（重要）：
//! - 本模块**不创建真正的 Windows `.lnk` 文件**：那需要 IShellLinkW COM
//!   调用，按 D3 全部 Windows API 必须集中在 `src/shell/`。本模块只做
//!   目录枚举、名称/分组/过滤等纯逻辑，并把「创建链接」留给 shell 层。
//! - 目录枚举用 `std::fs`，因此平台无关、可单测（G1）。
//! - 分组：原版 Favoriten 下的**子目录**即分组（未逐字节取证，
//!   实现者定夺并在此登记）；组内项通过路径前缀归属。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::{Config, Favorite, QError, QResult};

/// 收藏夹条目种类。由文件系统形态判定：目录 / `.lnk` / `.url` / 其它文件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FavoriteKind {
    /// 子目录（可作为分组容器）。
    Dir,
    /// Windows 快捷方式 `.lnk`。
    Lnk,
    /// 互联网快捷方式 `.url`。
    Url,
    /// 其它文件（仍显示，但不可导航）。
    File,
}

impl FavoriteKind {
    /// 是否可导航（目录或链接）。
    pub fn is_navigable(self) -> bool {
        !matches!(self, FavoriteKind::File)
    }
}

/// 带种类与归属信息的收藏夹条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteWithTarget {
    pub favorite: Favorite,
    pub kind: FavoriteKind,
    /// 所属分组名（顶层项为 `None`）。子目录形式的分组即组名。
    pub group: Option<String>,
}

impl FavoriteWithTarget {
    /// 显示名（小写比较前的原始名）。
    pub fn name(&self) -> &str {
        &self.favorite.name
    }
}

/// 收藏夹存储：两个根目录的枚举 + 名称/分组/过滤纯逻辑。
///
/// 不持有任何 Windows 句柄；`load` 只读文件系统，失败项跳过而不是报错
/// （收藏夹是辅助 UI，个别坏项不应阻塞整个菜单——任务要求：
/// 路径/键不存在时返回默认值而不是 panic）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteStore {
    /// Favoriten 根目录（`Config::favoriten_dir`）。
    pub favoriten_dir: PathBuf,
    /// Quick-Link 根目录（`Config::quick_link_dir`）。
    pub quick_link_dir: PathBuf,
}

impl FavoriteStore {
    /// 由配置构造。
    pub fn from_config(cfg: &Config) -> Self {
        FavoriteStore {
            favoriten_dir: cfg.favoriten_dir.clone(),
            quick_link_dir: cfg.quick_link_dir.clone(),
        }
    }

    /// 直接由两个目录构造（测试/调用方自备路径）。
    pub fn new(favoriten_dir: PathBuf, quick_link_dir: PathBuf) -> Self {
        FavoriteStore {
            favoriten_dir,
            quick_link_dir,
        }
    }

    /// 枚举 Favoriten 根下的项（不含 Quick-Link 目录自身及其内容）。
    ///
    /// 目录不存在时返回空列表（首次运行/便携版未创建目录的常态）。
    ///
    /// Quick-Link 目录虽在 Favoriten 之下，但语义上是独立分组，
    /// 不作为普通项重复出现（原版菜单分开两组，实现者定夺）；
    /// 因此整个 Quick-Link 子树都从本列表剔除。
    pub fn load_favoriten(&self) -> Vec<FavoriteWithTarget> {
        let mut out = self.scan_dir(&self.favoriten_dir, false);
        let ql = &self.quick_link_dir;
        out.retain(|f| !f.favorite.path.starts_with(ql));
        out
    }

    /// 枚举 Quick-Link 根下的项（`quick_link=true`）。
    pub fn load_quick_links(&self) -> Vec<FavoriteWithTarget> {
        self.scan_dir(&self.quick_link_dir, true)
    }

    /// 全部条目（Favoriten 项在前，Quick-Link 项在后）。
    pub fn load_all(&self) -> Vec<FavoriteWithTarget> {
        let mut out = self.load_favoriten();
        out.extend(self.load_quick_links());
        out
    }

    /// 按 [`crate::core::Config::fav_filter`] 过滤（大小写不敏感子串匹配，
    /// 空过滤器不过滤）。过滤作用于**显示名**。
    pub fn filter(items: &[FavoriteWithTarget], filter: &str) -> Vec<FavoriteWithTarget> {
        let needle = filter.trim();
        if needle.is_empty() {
            return items.to_vec();
        }
        let n = needle.to_lowercase();
        items
            .iter()
            .filter(|f| f.favorite.name.to_lowercase().contains(&n))
            .cloned()
            .collect()
    }

    /// 全部分组名（Favoriten 下的子目录），排序稳定。
    ///
    /// 分组判定：一级子目录且不是 Quick-Link 根。
    pub fn groups(&self) -> Vec<String> {
        let mut set: BTreeSet<String> = BTreeSet::new();
        for f in self.load_favoriten() {
            if let Some(g) = f.group {
                set.insert(g);
            }
        }
        set.into_iter().collect()
    }

    /// 新增收藏（复制文件/目录到 Favoriten 根下）。
    ///
    /// 目标已存在时返回 [`QError::InvalidPath`]（不覆盖用户数据）。
    /// `group` 为 `Some(名)` 时放入 `Favoriten\<组>\` 子目录（组不存在则创建）。
    pub fn add(&self, source: &Path, name: &str, group: Option<&str>) -> QResult<Favorite> {
        let name = sanitize_name(name)?;
        let target_dir = match group {
            Some(g) if !g.trim().is_empty() => {
                let g = sanitize_name(g)?;
                self.favoriten_dir.join(g)
            }
            _ => self.favoriten_dir.clone(),
        };
        let target = target_dir.join(&name);
        if target.exists() {
            return Err(QError::InvalidPath(format!(
                "收藏目标已存在：{}",
                target.display()
            )));
        }
        fs::create_dir_all(&target_dir).map_err(|e| QError::from_io(&e))?;
        let meta = fs::metadata(source).map_err(|e| QError::from_io(&e))?;
        if meta.is_dir() {
            copy_dir_recursive(source, &target)?;
        } else {
            fs::copy(source, &target).map_err(|e| QError::from_io(&e))?;
        }
        Ok(Favorite {
            name,
            path: target,
            quick_link: false,
        })
    }

    /// 删除收藏项（文件或目录均可；不递归确认，由 UI 层弹确认框——C4）。
    pub fn remove(&self, favorite: &Favorite) -> QResult<()> {
        if !favorite.path.exists() {
            // 幂等：目标本就不在时视为成功（用户可能已手工删除）。
            return Ok(());
        }
        if favorite.path.is_dir() {
            fs::remove_dir_all(&favorite.path).map_err(|e| QError::from_io(&e))
        } else {
            fs::remove_file(&favorite.path).map_err(|e| QError::from_io(&e))
        }
    }

    /// 改名（同目录内重命名）。新名非法或已存在时报错。
    pub fn rename(&self, favorite: &Favorite, new_name: &str) -> QResult<Favorite> {
        let new_name = sanitize_name(new_name)?;
        let parent = favorite
            .path
            .parent()
            .ok_or_else(|| QError::InvalidPath("收藏项没有父目录".to_string()))?;
        let target = parent.join(&new_name);
        if target.exists() {
            return Err(QError::InvalidPath(format!(
                "改名目标已存在：{}",
                target.display()
            )));
        }
        fs::rename(&favorite.path, &target).map_err(|e| QError::from_io(&e))?;
        Ok(Favorite {
            name: new_name,
            path: target,
            quick_link: favorite.quick_link,
        })
    }

    /// 移动到分组（在 Favoriten 树内移动，改归属组）。
    pub fn move_to_group(&self, favorite: &Favorite, group: Option<&str>) -> QResult<Favorite> {
        let target_dir = match group {
            Some(g) if !g.trim().is_empty() => self.favoriten_dir.join(sanitize_name(g)?),
            _ => self.favoriten_dir.clone(),
        };
        let target = target_dir.join(&favorite.name);
        if target.exists() {
            return Err(QError::InvalidPath(format!(
                "分组内已存在同名项：{}",
                target.display()
            )));
        }
        fs::create_dir_all(&target_dir).map_err(|e| QError::from_io(&e))?;
        fs::rename(&favorite.path, &target).map_err(|e| QError::from_io(&e))?;
        Ok(Favorite {
            name: favorite.name.clone(),
            path: target,
            quick_link: favorite.quick_link,
        })
    }

    /// 新建分组（在 Favoriten 根下建目录）。
    pub fn create_group(&self, name: &str) -> QResult<PathBuf> {
        let name = sanitize_name(name)?;
        let dir = self.favoriten_dir.join(name);
        if dir.exists() {
            return Err(QError::InvalidPath(format!(
                "分组已存在：{}",
                dir.display()
            )));
        }
        fs::create_dir_all(&dir).map_err(|e| QError::from_io(&e))?;
        Ok(dir)
    }

    // ---- 内部 ----

    /// 枚举一个目录（含子目录），产出条目。`quick_link` 标记写进
    /// [`Favorite::quick_link`]。
    ///
    /// 递归进入子目录：分组内的项也要出现在菜单里（原版收藏夹菜单
    /// 展平显示组内项，实现者定夺并在此登记）。目录自身既作为条目
    /// 出现（可作为分组导航目标），其子项也各自成为条目。
    fn scan_dir(&self, dir: &Path, quick_link: bool) -> Vec<FavoriteWithTarget> {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            // 目录不存在/不可读：收藏夹是辅助 UI，返回空而不是报错。
            Err(_) => return Vec::new(),
        };
        let mut out: Vec<FavoriteWithTarget> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
            else {
                continue;
            };
            let kind = kind_of(&path);
            // 分组归属：Favoriten 树内、位于某个一级子目录之下即为该组成员。
            let group = if quick_link {
                None
            } else {
                self.group_of(&path)
            };
            out.push(FavoriteWithTarget {
                favorite: Favorite {
                    name,
                    path: path.clone(),
                    quick_link,
                },
                kind,
                group,
            });
            // 递归子目录（分组内容）。递归深度由文件系统自然限制，
            // 不额外设上限：收藏夹树是用户自建的，通常只有一两层。
            if kind == FavoriteKind::Dir {
                out.extend(self.scan_dir(&path, quick_link));
            }
        }
        // 目录在前，其余按名称排序，保证菜单顺序稳定可复现。
        out.sort_by(|a, b| {
            let ka = if a.kind == FavoriteKind::Dir { 0 } else { 1 };
            let kb = if b.kind == FavoriteKind::Dir { 0 } else { 1 };
            ka.cmp(&kb)
                .then_with(|| a.favorite.name.to_lowercase().cmp(&b.favorite.name.to_lowercase()))
        });
        out
    }

    /// 判定路径属于哪个一级分组（不在任何一级子目录下 -> None）。
    fn group_of(&self, path: &Path) -> Option<String> {
        let parent = path.parent()?;
        if parent == self.favoriten_dir {
            return None; // 顶层项
        }
        // 逐级向上找到 favoriten_dir 的下一级。
        let mut cur = parent.to_path_buf();
        loop {
            let p = cur.parent()?;
            if p == self.favoriten_dir {
                return cur
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string());
            }
            if !cur.starts_with(&self.favoriten_dir) {
                return None;
            }
            cur = p.to_path_buf();
        }
    }
}

/// 由路径形态判定种类。
fn kind_of(path: &Path) -> FavoriteKind {
    if path.is_dir() {
        return FavoriteKind::Dir;
    }
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
    {
        Some("lnk") => FavoriteKind::Lnk,
        Some("url") => FavoriteKind::Url,
        _ => FavoriteKind::File,
    }
}

/// 名称清洗：拒绝空名、路径分隔符与 Windows 非法字符。
///
/// 收藏项名会成为文件系统名字，必须挡住 `..`、`\`、`/`、`:`、`*`、`?`、
/// `"`、`<`、`>`、`|` 与控制字符（C1：不用 panic 表达用户输入错误）。
fn sanitize_name(name: &str) -> QResult<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(QError::InvalidPath("名称为空".to_string()));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(QError::InvalidPath(format!(
            "名称非法（保留字）：{trimmed}"
        )));
    }
    let bad = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
    for c in trimmed.chars() {
        if bad.contains(&c) || c.is_control() {
            return Err(QError::InvalidPath(format!(
                "名称含非法字符 {c:?}：{trimmed}"
            )));
        }
    }
    Ok(trimmed.to_string())
}

/// 递归复制目录（仅文件与子目录；符号链接按普通文件复制内容）。
fn copy_dir_recursive(src: &Path, dst: &Path) -> QResult<()> {
    fs::create_dir_all(dst).map_err(|e| QError::from_io(&e))?;
    let entries = fs::read_dir(src).map_err(|e| QError::from_io(&e))?;
    for entry in entries.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ft = entry.file_type().map_err(|e| QError::from_io(&e))?;
        if ft.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| QError::from_io(&e))?;
        }
    }
    Ok(())
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    /// 每个测试一个独立临时根（按测试名 + 一个单调计数区分），
    /// 避免 cargo test 默认的多线程并行互相踩目录。
    fn setup(test_name: &str) -> (PathBuf, FavoriteStore) {
        static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "qdir-rs-fav-{}-{}-{}",
            test_name,
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&root);
        let fav = root.join("Favoriten");
        let ql = fav.join("Quick-Link");
        fs::create_dir_all(&fav).expect("建 Favoriten");
        fs::create_dir_all(&ql).expect("建 Quick-Link");
        (root, FavoriteStore::new(fav, ql))
    }

    #[test]
    fn scan_empty_dirs_yields_nothing() {
        let (_root, store) = setup("scan_empty");
        assert!(store.load_favoriten().is_empty());
        assert!(store.load_quick_links().is_empty());
        assert!(store.load_all().is_empty());
        assert!(store.groups().is_empty());
    }

    #[test]
    fn missing_dir_is_not_an_error() {
        // 目录不存在 -> 空列表（任务要求：不 panic）。
        let store = FavoriteStore::new(
            PathBuf::from("Z:\\definitely-not-here\\Favoriten"),
            PathBuf::from("Z:\\definitely-not-here\\Quick-Link"),
        );
        assert!(store.load_favoriten().is_empty());
    }

    #[test]
    fn scan_classifies_kinds_and_groups() {
        let (root, store) = setup("classify");
        let fav = root.join("Favoriten");
        // 顶层：一个目录（分组）、一个 .lnk、一个 .url、一个 .txt
        fs::create_dir_all(fav.join("Arbeit")).expect("分组目录");
        fs::write(fav.join("arbeit.lnk"), b"x").expect("lnk");
        fs::write(fav.join("home.url"), b"x").expect("url");
        fs::write(fav.join("note.txt"), b"x").expect("txt");
        // 组内项
        fs::write(fav.join("Arbeit").join("projekt.lnk"), b"x").expect("组内 lnk");

        let items = store.load_favoriten();
        // Quick-Link 目录自身不出现；4 个顶层项 + 1 个组内项 = 5。
        assert_eq!(items.len(), 5, "{items:?}");

        let arbeit = items.iter().find(|f| f.name() == "Arbeit").expect("Arbeit");
        assert_eq!(arbeit.kind, FavoriteKind::Dir);
        assert_eq!(arbeit.group, None, "分组自身是顶层");

        let lnk = items.iter().find(|f| f.name() == "arbeit.lnk").expect("lnk");
        assert_eq!(lnk.kind, FavoriteKind::Lnk);
        assert!(lnk.kind.is_navigable());

        let url = items.iter().find(|f| f.name() == "home.url").expect("url");
        assert_eq!(url.kind, FavoriteKind::Url);

        let txt = items.iter().find(|f| f.name() == "note.txt").expect("txt");
        assert_eq!(txt.kind, FavoriteKind::File);
        assert!(!txt.kind.is_navigable());

        let inner = items
            .iter()
            .find(|f| f.name() == "projekt.lnk")
            .expect("组内项");
        assert_eq!(inner.group.as_deref(), Some("Arbeit"));

        assert_eq!(store.groups(), vec!["Arbeit".to_string()]);
    }

    #[test]
    fn quick_links_marked_separately() {
        let (root, store) = setup("quicklinks");
        let ql = root.join("Favoriten").join("Quick-Link");
        fs::write(ql.join("editor.lnk"), b"x").expect("lnk");
        let qls = store.load_quick_links();
        assert_eq!(qls.len(), 1);
        assert!(qls[0].favorite.quick_link);
        assert_eq!(qls[0].kind, FavoriteKind::Lnk);
        // Favoriten 列表不含 Quick-Link 目录自身，也不含其内容。
        assert!(store.load_favoriten().is_empty());
    }

    #[test]
    fn dirs_sort_before_files_then_by_name() {
        let (root, store) = setup("sort");
        let fav = root.join("Favoriten");
        fs::write(fav.join("b.lnk"), b"x").expect("b");
        fs::write(fav.join("a.lnk"), b"x").expect("a");
        fs::create_dir_all(fav.join("zdir")).expect("zdir");
        let names: Vec<String> = store
            .load_favoriten()
            .iter()
            .map(|f| f.favorite.name.clone())
            .collect();
        assert_eq!(names, vec!["zdir", "a.lnk", "b.lnk"]);
    }

    #[test]
    fn fav_filter_matches_name_case_insensitively() {
        let (root, store) = setup("favfilter");
        let fav = root.join("Favoriten");
        fs::write(fav.join("Home.lnk"), b"x").expect("1");
        fs::write(fav.join("work.lnk"), b"x").expect("2");
        let all = store.load_favoriten();
        assert_eq!(FavoriteStore::filter(&all, "home").len(), 1);
        assert_eq!(FavoriteStore::filter(&all, "  ").len(), 2, "空白不过滤");
        assert_eq!(FavoriteStore::filter(&all, "nomatch").len(), 0);
    }

    #[test]
    fn add_rename_remove_roundtrip() {
        let (root, store) = setup("roundtrip");
        let src = root.join("src");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.txt"), b"hello").expect("file");

        let fav = store.add(&src, "Mein Ordner", None).expect("新增");
        assert_eq!(fav.name, "Mein Ordner");
        assert!(fav.path.join("a.txt").exists(), "目录内容必须复制");

        // 重复新增同名 -> 报错而不是覆盖。
        assert!(store.add(&src, "Mein Ordner", None).is_err());

        // 改名
        let renamed = store.rename(&fav, "Umbenannt").expect("改名");
        assert_eq!(renamed.name, "Umbenannt");
        assert!(renamed.path.join("a.txt").exists());
        assert!(!fav.path.exists(), "旧名应已不在");

        // 删除
        store.remove(&renamed).expect("删除");
        assert!(!renamed.path.exists());

        // 删除不存在的项：幂等成功。
        store.remove(&renamed).expect("幂等删除");
    }

    #[test]
    fn add_into_group_creates_it() {
        let (root, store) = setup("addgroup");
        let src = root.join("f.txt");
        fs::write(&src, b"x").expect("f");
        let fav = store.add(&src, "doc.lnk", Some("Arbeit")).expect("新增到组");
        assert_eq!(
            fav.path,
            store.favoriten_dir.join("Arbeit").join("doc.lnk")
        );
        let items = store.load_favoriten();
        let entry = items.iter().find(|f| f.name() == "doc.lnk").expect("entry");
        assert_eq!(entry.group.as_deref(), Some("Arbeit"));
    }

    #[test]
    fn move_to_group_and_create_group() {
        let (root, store) = setup("movegroup");
        let src = root.join("f.txt");
        fs::write(&src, b"x").expect("f");
        let fav = store.add(&src, "doc.lnk", None).expect("新增");

        let moved = store
            .move_to_group(&fav, Some("Projekte"))
            .expect("移动");
        assert_eq!(moved.path, store.favoriten_dir.join("Projekte").join("doc.lnk"));

        // 移回顶层
        let back = store.move_to_group(&moved, None).expect("移回");
        assert_eq!(back.path, store.favoriten_dir.join("doc.lnk"));

        let g = store.create_group("Neu").expect("建组");
        assert!(g.is_dir());
        assert!(store.create_group("Neu").is_err(), "重复建组报错");
    }

    #[test]
    fn illegal_names_rejected() {
        let (root, store) = setup("illegal");
        let src = root.join("f.txt");
        fs::write(&src, b"x").expect("f");
        for bad in ["", "   ", "..", "a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b"] {
            assert!(
                store.add(&src, bad, None).is_err(),
                "名称 {bad:?} 必须被拒绝"
            );
        }
        // 合法名可用
        assert!(store.add(&src, "ok.lnk", None).is_ok());
    }

    #[test]
    fn store_from_config_uses_configured_dirs() {
        let cfg = Config::default();
        let store = FavoriteStore::from_config(&cfg);
        assert_eq!(store.favoriten_dir, cfg.favoriten_dir);
        assert_eq!(store.quick_link_dir, cfg.quick_link_dir);
    }
}
