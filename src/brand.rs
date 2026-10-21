//! 品牌常量（产品身份的唯一来源，PRODUCT_SPEC R1：不引用原版标识）。
//!
//! 所有散落的 "Q-Dir"/"12.71"/ATL 类名字面量都收口到这里。
//! 配置文件的**读取**可以兼容原版格式（R2 导入特性），但**写出**用
//! 本模块的自有文件名。

/// 产品名（窗口标题、关于框、托盘 tooltip）。
pub const PRODUCT_NAME: &str = "L-Dir";

/// 产品版本（自有版本线，与原版无关）。
pub const PRODUCT_VERSION: &str = "0.1.0";

/// 窗口类名前缀（自有命名；原版为 ATL: 运行时串，不得复用——R1）。
pub const WINDOW_CLASS: &str = "LDir.MainFrame";

/// 每窗格内容容器类名（自有）。
pub const PANE_CONTENT_CLASS: &str = "LDir.PaneContent";

/// 窗格类名（自有）。
pub const PANE_CLASS: &str = "LDir.Pane";

/// 默认写出的配置文件名（相对 exe 目录）。
pub const INI_FILE: &str = "L-Dir.ini";

/// 默认写出的会话文件名（相对 exe 目录）。
pub const SESSION_FILE: &str = "L.qdr";

/// 原版兼容文件名（导入/回退读取用，见 R2；仅当自有文件不存在时才读）。
pub const LEGACY_INI_FILE: &str = "Q-Dir.ini";
pub const LEGACY_SESSION_FILE: &str = "start.qdr";

/// 状态保存对话框过滤描述（自有文案）。
pub const STATE_FILTER_DESC: &str = "L-Dir 状态(*.qdr)";

/// 关于框一行简介（自有文案）。
pub const TAGLINE: &str = "四格文件管理器";
