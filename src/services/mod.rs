//! 服务层门面：导出、收藏夹与多语言。
//!
//! 本模块只编排三个纯逻辑子系统，自身不含业务：
//! - [`export`]：列表导出（TXT / CSV / HTML / XLS / Tabgetrennt）；
//! - [`favorites`]：Favoriten 与 Quick-Link 管理；
//! - [`i18n`]：langid / LangName 驱动的三套字符串表。
//!
//! 依赖方向 D1：本模块只依赖 `crate::core`，不依赖 config / shell / app，
//! 因此全部逻辑可在非 Windows 平台单测（G1）。Windows 专有能力
//! （创建真正的 .lnk、托盘、剪贴板）属于 shell 模块，不在这里出现。

pub mod export;
pub mod favorites;
pub mod i18n;
pub mod pixdiff;

pub use export::{ExportColumn, ExportOptions, export_bytes, render_items, write_export};
pub use favorites::{FavoriteKind, FavoriteStore, FavoriteWithTarget};
pub use i18n::{STRINGS, Translator};
pub use pixdiff::{DiffReport, DiffThreshold, RgbaImage, diff, overlay_image};
