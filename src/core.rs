//! Q-Dir 重写工程 —— 核心类型层（地基）。
//!
//! 本文件只依赖 `std`，不引用 egui / windows 等任何外部 crate，
//! 因此所有类型都可在非 Windows 平台构造与单元测试（契约 G1）。
//!
//! 逆向事实来源：REA 对 `D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir_x64.exe`
//! （sha256=321d3288cddf37373fb2519711f8ab569cd5a2f9bf9813142de9a05da509a40c）的证据，
//! 以及同目录下 Q-Dir.ini / start.qdr 两个 UTF-16LE 夹具的字节实测。
//!
//! 契约：字段与方法签名由架构契约 `coreTypes` 固定（A1），实现者不得改动签名；
//! 需要新字段时只能提 constraints 变更。为实现任务要求的「默认排序比较 /
//! FileItem 按 SortKey 排序」，本文件在契约之外仅新增两个 `pub` 辅助项
//! （`SortKey::compare` 与自由函数 `sort_items`），已在回复中登记。

use std::cmp::Ordering;
use std::fmt;
use std::path::PathBuf;

// ============ 1. 错误类型 ============

/// 全应用唯一错误类型。所有跨模块边界传递的错误都必须是它。
/// Io/Os 携带原始 OS 错误码（Windows GetLastError），供托盘/日志/兜底路径展示；
/// core.rs 因此不需要引用 windows crate。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QError {
    /// 文件系统 / IO 失败。code 为原始 OS 错误码。
    Io { code: u32, msg: String },
    /// 路径非法（空、含 NUL、相对路径却要求绝对路径等）。
    InvalidPath(String),
    /// UTF-16LE 编解码失败（奇数长度或代理项错误）。
    Encoding(String),
    /// 配置键缺失或格式错误。key 为 ini/.qdr 中的原始键名。
    Config { key: String, msg: String },
    /// 着色规则解析失败（Filter2 段）。
    Rule { index: usize, msg: String },
    /// 会话状态越界，例如 Focus 指向不存在的窗格。
    Session(String),
    /// Windows Shell 目标不存在（枚举失败、PIDL 无效等）。
    Shell(String),
    /// 用户取消（例如“移动文件确认”对话框选了否）。
    Cancelled,
}

impl QError {
    /// 从 std::io::Error 转换，保留 OS 错误码。
    pub fn from_io(e: &std::io::Error) -> Self {
        QError::Io {
            code: e.raw_os_error().unwrap_or(0) as u32,
            msg: e.to_string(),
        }
    }

    /// 构造带原始键名的配置错误（C3：错误必须带上下文）。
    pub fn config(key: &str, msg: impl Into<String>) -> Self {
        QError::Config {
            key: key.to_string(),
            msg: msg.into(),
        }
    }
}

impl fmt::Display for QError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QError::Io { code, msg } => write!(f, "IO 错误（OS 代码 {code}）：{msg}"),
            QError::InvalidPath(p) => write!(f, "路径非法：{p}"),
            QError::Encoding(m) => write!(f, "UTF-16LE 编解码失败：{m}"),
            QError::Config { key, msg } => write!(f, "配置键 {key} 错误：{msg}"),
            QError::Rule { index, msg } => write!(f, "Filter2 第 {index} 条规则错误：{msg}"),
            QError::Session(m) => write!(f, "会话状态错误：{m}"),
            QError::Shell(m) => write!(f, "Shell 错误：{m}"),
            QError::Cancelled => write!(f, "用户取消"),
        }
    }
}

impl std::error::Error for QError {}

/// 全应用统一结果别名。
pub type QResult<T> = Result<T, QError>;

// ============ 2. 颜色 ============

/// RRGGBB 颜色。Filter2 里颜色是 6 位十六进制串，-1 表示默认色（用 None）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// 解析 6 位十六进制 RRGGBB，大小写不敏感；失败返回 None。
    /// 为容错额外接受一个可选的前导 `#`（不影响 ini 往返，to_hex 不输出 `#`）。
    pub fn parse_hex(s: &str) -> Option<Self> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Rgb(r, g, b))
    }

    /// 输出 6 位大写十六进制，与 ini 格式一致。
    pub fn to_hex(&self) -> String {
        format!("{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }
}

// ============ 3. 文件条目 ============

/// 文件/目录条目。列表、树、预览、导出、剪贴板共用的最小单元。
/// 不持有任何 Windows 句柄或 PIDL，可在非 Windows 平台构造和测试。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    /// 完整路径，已规范化，分隔符为 `\`。
    pub path: PathBuf,
    /// 显示名（含扩展名）。
    pub name: String,
    /// 是否目录。
    pub is_dir: bool,
    /// 是否隐藏（FILE_ATTRIBUTE_HIDDEN/SYSTEM），#Hidden 规则消费此位。
    pub is_hidden: bool,
    /// 字节数；目录为 0。
    pub size: u64,
    /// 最后修改时间（FILETIME，100ns 单位）。0 = 未知。
    pub modified: u64,
    /// SHGetFileInfoW 得到的图标索引；None 用内置兜底图标。
    pub icon_index: Option<i32>,
}

impl FileItem {
    /// 小写扩展名（不含点），Filter2 通配符匹配使用。
    pub fn ext_lower(&self) -> String {
        self.path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }

    /// 小写文件名（不含路径）。
    pub fn name_lower(&self) -> String {
        self.name.to_lowercase()
    }
}

// ============ 4. 窗格标识 ============

/// 四窗格标识，对应 Ploder1..4 / Folder1..4 / Tree1..4 / ColsAndSort1..4。
/// 固定 4 个，不动态增长。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PaneId {
    P1,
    P2,
    P3,
    P4,
}

impl PaneId {
    pub const ALL: [PaneId; 4] = [PaneId::P1, PaneId::P2, PaneId::P3, PaneId::P4];

    /// 1..=4，与 ini 键名后缀一致。
    pub fn number(&self) -> u32 {
        match self {
            PaneId::P1 => 1,
            PaneId::P2 => 2,
            PaneId::P3 => 3,
            PaneId::P4 => 4,
        }
    }

    /// 从 1..=4 构造，越界返回 None。
    pub fn from_number(n: u32) -> Option<Self> {
        match n {
            1 => Some(PaneId::P1),
            2 => Some(PaneId::P2),
            3 => Some(PaneId::P3),
            4 => Some(PaneId::P4),
            _ => None,
        }
    }

    /// ini 段名，如 `Column_OS_10.0_Ploder1`。
    /// 段名中的 `10.0` 来自实测 ini 的 `Column_OS_10.0_PloderN`（Windows 10 平台标记）。
    pub fn ini_section(&self) -> String {
        format!("Column_OS_10.0_Ploder{}", self.number())
    }
}

impl fmt::Display for PaneId {
    /// 输出 Ploder1..Ploder4（.qdr 的 FolderN / TreeN 键名前缀）。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ploder{}", self.number())
    }
}

// ============ 5. 视图模式与排序 ============

/// 四种视图模式，对应菜单 216 德语原文 &Große Symbole / &Kleine Symbole / &Liste / &Details。
/// 序号与 ViewMode=%d 状态串一致：0=大图标 1=小图标 2=列表 3=详情。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    LargeIcon,
    SmallIcon,
    List,
    Detail,
}

impl ViewMode {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(ViewMode::LargeIcon),
            1 => Some(ViewMode::SmallIcon),
            2 => Some(ViewMode::List),
            3 => Some(ViewMode::Detail),
            _ => None,
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            ViewMode::LargeIcon => 0,
            ViewMode::SmallIcon => 1,
            ViewMode::List => 2,
            ViewMode::Detail => 3,
        }
    }
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::LargeIcon
    } // start.qdr 实测 ViewMode=0
}

/// 排序列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortKey {
    Name,
    Ext,
    Size,
    Modified,
    Type,
}

impl SortKey {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(SortKey::Name),
            1 => Some(SortKey::Ext),
            2 => Some(SortKey::Size),
            3 => Some(SortKey::Modified),
            4 => Some(SortKey::Type),
            _ => None,
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            SortKey::Name => 0,
            SortKey::Ext => 1,
            SortKey::Size => 2,
            SortKey::Modified => 3,
            SortKey::Type => 4,
        }
    }

    /// 两个条目的升序比较（契约未规定，实现者补充；任务要求「默认排序比较」）。
    ///
    /// 约定（未从二进制取证，实现者定夺并在此登记）：
    /// - 全部键均按**升序**比较，方向由 [`SortDir`] 决定；
    /// - `Type` 列没有独立数据源，退化为「扩展名 -> 名称」；
    /// - 同级比较最终回落到名称的大小写敏感比较，保证排序稳定可复现。
    pub fn compare(&self, a: &FileItem, b: &FileItem) -> Ordering {
        let primary = match self {
            SortKey::Name => a.name_lower().cmp(&b.name_lower()),
            SortKey::Ext => a
                .ext_lower()
                .cmp(&b.ext_lower())
                .then_with(|| a.name_lower().cmp(&b.name_lower())),
            SortKey::Size => a.size.cmp(&b.size),
            SortKey::Modified => a.modified.cmp(&b.modified),
            SortKey::Type => a
                .ext_lower()
                .cmp(&b.ext_lower())
                .then_with(|| a.name_lower().cmp(&b.name_lower())),
        };
        primary.then_with(|| a.name.cmp(&b.name))
    }
}

/// 排序方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(SortDir::Asc),
            1 => Some(SortDir::Desc),
            _ => None,
        }
    }

    pub fn to_u32(self) -> u32 {
        match self {
            SortDir::Asc => 0,
            SortDir::Desc => 1,
        }
    }
}

impl Default for SortDir {
    fn default() -> Self {
        SortDir::Asc
    }
}

/// 按排序键对条目排序（契约未规定，实现者补充；任务要求「FileItem 按 SortKey 排序」）。
///
/// 目录恒排在文件之前（资源管理器惯例），且不受排序方向影响；
/// 方向只作用于键值本身的升降序。
pub fn sort_items(items: &mut [FileItem], key: SortKey, dir: SortDir) {
    items.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let ord = key.compare(a, b);
        match dir {
            SortDir::Asc => ord,
            SortDir::Desc => ord.reverse(),
        }
    });
}

// ============ 6. 着色规则（Filter2 段） ============

/// Filter2 单条规则的模式部分。原版 ini 实测样例：
///   0=#Hidden=1=-1=1=1=-1=0
///   3=#DIR=1=008800=-1=1=-1=0
///   9=*=1=0000BB=-1=1=-1=0          （兜底）
///   10=#BG=1=-1=-1=1=-1=0
///   11=#BG-A=1=-1=-1=1=-1=0
///   1=*.zip;*.rar;*.gz;*.7z;=1=00AA00=-1=1=-1=0
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulePattern {
    /// `#Hidden`：命中隐藏/系统文件。
    Hidden,
    /// `#DIR`：命中目录。
    Dir,
    /// `#BG`：交替行背景（第一组）。
    BgAlt,
    /// `#BG-A`：交替行背景（第二组）。
    BgAlt2,
    /// 通配符列表。注意原版 rule 4 混用了逗号
    /// `*.jpg;*.jpeg;*.png,*.gif;*.bmp;*.ico`，故分隔符须同时支持 `;` 与 `,`。
    /// `trailing_sep` 记录原始串是否以分隔符结尾（rule 1/2 以 `;` 结尾；
    /// 实测 rule 4 以 `*.ico` 结尾，故为 false——以真实字节为准），
    /// 以便 to_ini_line 原样回写。
    ///
    /// 已登记差异：`pats` 只保留拆分后的模式，不保留原始分隔符序列。
    /// 因此 rule 4 回写时混排的 `,` 会统一成 `;`（语义不变，F2 保证两种分隔符
    /// 等价）。要逐字节还原需在 RulePattern 加字段存原始分隔符序列，
    /// 但那会改动契约类型形状（A1 禁止），故按契约 E6 只保证 trailing_sep。
    Wildcards {
        pats: Vec<String>,
        trailing_sep: bool,
    },
    /// `*`：兜底，匹配一切。
    Any,
}

impl RulePattern {
    /// ini 字段 [1] 的文本形式。
    fn as_ini(&self) -> String {
        match self {
            RulePattern::Hidden => "#Hidden".to_string(),
            RulePattern::Dir => "#DIR".to_string(),
            RulePattern::BgAlt => "#BG".to_string(),
            RulePattern::BgAlt2 => "#BG-A".to_string(),
            RulePattern::Any => "*".to_string(),
            RulePattern::Wildcards { pats, trailing_sep } => {
                let mut s = pats.join(";");
                if *trailing_sep {
                    s.push(';');
                }
                s
            }
        }
    }

    /// 从 ini 字段 [1] 解析模式；无法识别时按通配符列表处理。
    fn parse_ini(s: &str) -> Self {
        match s {
            "#Hidden" => RulePattern::Hidden,
            "#DIR" => RulePattern::Dir,
            "#BG" => RulePattern::BgAlt,
            "#BG-A" => RulePattern::BgAlt2,
            "*" => RulePattern::Any,
            other => {
                // F2：分隔符同时支持 ';' 与 ','（原版 rule 4 混用）。
                let pats: Vec<String> = other
                    .split([';', ','])
                    .filter(|p| !p.is_empty())
                    .map(|p| p.to_string())
                    .collect();
                let trailing_sep = other.ends_with(';') || other.ends_with(',');
                RulePattern::Wildcards { pats, trailing_sep }
            }
        }
    }
}

/// 一条完整 Filter2 规则。ini 行 8 个字段：`idx=模式=启用=颜色=f4=f5=f6=0`。
/// 实测样例 `1=*.zip;*.rar;*.gz;*.7z;=1=00AA00=-1=1=-1=0`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRule {
    /// ini 行首索引，保留原值以便原样回写。
    pub index: usize,
    pub pattern: RulePattern,
    /// 是否启用。
    pub enabled: bool,
    /// 前景色；None = 默认色（ini 中的 -1）。
    pub color: Option<Rgb>,
    /// ini 字段 [4]，保留原值。
    pub field4: i32,
    /// ini 字段 [5]，保留原值。
    pub field5: i32,
    /// ini 字段 [6]，保留原值。
    pub field6: i32,
}

impl FilterRule {
    /// 解析单行 ini（不含换行），行首索引由调用方给定。
    ///
    /// 供 config 层做「规则是否变化」的语义比较：原文件里 rule 4 的串内
    /// 混用 `;` 与 `,`，序列化后文本必然不同，只有比较解析结果才能判断
    /// 规则本身是否被改过（E2「除本次修改的键外逐字节等价」）。
    ///
    /// 字段数不足/非法时返回 [`QError::Rule`]，与 `RuleSet::parse_ini_section`
    /// 的错误类型一致。
    pub fn parse_line(index: usize, line: &str) -> QResult<Self> {
        RuleSet::parse_ini_section(&[format!("{index}={line}")])
            .map(|rs| {
                // parse_ini_section 对非空输入必产出一条规则（上面已保证非空）。
                let mut it = rs.rules.into_iter();
                let first = it.next();
                first.ok_or_else(|| QError::Rule {
                    index,
                    msg: "单行解析未产出规则".to_string(),
                })
            })
            .unwrap_or_else(|e| Err(e))
    }

    /// 判断条目是否命中。#BG/#BG-A 不按条目命中（按行号取模），恒为 false。
    pub fn matches(&self, item: &FileItem) -> bool {
        match &self.pattern {
            RulePattern::Hidden => item.is_hidden,
            RulePattern::Dir => item.is_dir,
            // F3：#BG/#BG-A 走 RuleSet::bg_variant，不参与条目命中。
            RulePattern::BgAlt | RulePattern::BgAlt2 => false,
            RulePattern::Any => true,
            RulePattern::Wildcards { pats, .. } => {
                let name = item.name_lower();
                pats.iter().any(|p| wildcard_match(p, &name))
            }
        }
    }

    /// 序列化回 ini 行（不含换行）。
    pub fn to_ini_line(&self) -> String {
        format!(
            "{}={}={}={}={}={}={}=0",
            self.index,
            self.pattern.as_ini(),
            if self.enabled { 1 } else { 0 },
            match self.color {
                Some(c) => c.to_hex(),
                None => "-1".to_string(),
            },
            self.field4,
            self.field5,
            self.field6,
        )
    }
}

/// Filter2 规则集合，“第一条命中即生效”。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleSet {
    pub rules: Vec<FilterRule>,
}

impl RuleSet {
    pub fn new() -> Self {
        RuleSet { rules: Vec::new() }
    }

    /// 按索引排序（回写 ini 前调用）。
    pub fn sort(&mut self) {
        self.rules.sort_by_key(|r| r.index);
    }

    /// 第一条命中（enabled 且 matches）的规则。
    /// 与 `color_for` 的区别：保留「无命中」与「命中但默认色」的差异（F1）。
    fn first_match(&self, item: &FileItem) -> Option<&FilterRule> {
        self.rules
            .iter()
            .find(|r| r.enabled && r.matches(item))
    }

    /// 条目前景色。语义：**第一条命中（enabled 且 matches）的规则即决定颜色**，
    /// 即使该规则颜色为 -1 也不再向后找。必须区分“无命中”与“命中但默认色”。
    pub fn color_for(&self, item: &FileItem) -> Option<Rgb> {
        // F1：命中即停。-1（None）不再继续匹配，否则隐藏文件会错落到 #DIR 规则。
        self.first_match(item).and_then(|r| r.color)
    }

    /// 交替行背景：偶数行取 #BG 规则色，奇数行取 #BG-A；未启用或 -1 返回 None。
    pub fn bg_variant(&self, row: usize) -> Option<Rgb> {
        let want = if row % 2 == 0 {
            RulePattern::BgAlt
        } else {
            RulePattern::BgAlt2
        };
        self.rules
            .iter()
            .find(|r| r.enabled && r.pattern == want)
            .and_then(|r| r.color)
    }

    /// 解析 ini 的 [Filter2] 段内容（行序列，不含段头）。
    pub fn parse_ini_section(lines: &[String]) -> QResult<Self> {
        let mut rules = Vec::new();
        for (lineno, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let fields: Vec<&str> = trimmed.split('=').collect();
            let bad_index = fields
                .first()
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(lineno);
            if fields.len() != 8 {
                return Err(QError::Rule {
                    index: bad_index,
                    msg: format!("应有 8 个 '=' 分隔字段，实际 {}", fields.len()),
                });
            }
            let index: usize = fields[0].trim().parse().map_err(|_| QError::Rule {
                index: bad_index,
                msg: format!("行首索引不是整数：{:?}", fields[0]),
            })?;
            let pattern = RulePattern::parse_ini(fields[1]);
            let enabled = match fields[2] {
                "1" => true,
                "0" => false,
                other => {
                    return Err(QError::Rule {
                        index,
                        msg: format!("启用位只能是 0/1，实际 {other:?}"),
                    });
                }
            };
            let color = if fields[3] == "-1" {
                None
            } else {
                Some(Rgb::parse_hex(fields[3]).ok_or_else(|| QError::Rule {
                    index,
                    msg: format!("颜色不是 6 位十六进制 RRGGBB：{:?}", fields[3]),
                })?)
            };
            let mut nums = [0i32; 3];
            for (slot, raw) in nums.iter_mut().zip(fields[4..7].iter()) {
                *slot = raw.trim().parse().map_err(|_| QError::Rule {
                    index,
                    msg: format!("保留字段不是整数：{raw:?}"),
                })?;
            }
            // 第 8 字段实测恒为 0，不建模；仅校验其为整数以免静默丢数据。
            fields[7].trim().parse::<i32>().map_err(|_| QError::Rule {
                index,
                msg: format!("末字段不是整数：{:?}", fields[7]),
            })?;
            rules.push(FilterRule {
                index,
                pattern,
                enabled,
                color,
                field4: nums[0],
                field5: nums[1],
                field6: nums[2],
            });
        }
        Ok(RuleSet { rules })
    }

    /// 序列化整个段（按 index 排序后逐行输出，不含段头）。
    pub fn to_ini_lines(&self) -> Vec<String> {
        let mut sorted = self.clone();
        sorted.sort();
        sorted.rules.iter().map(|r| r.to_ini_line()).collect()
    }
}

/// 通配符匹配（大小写不敏感，`*` 与 `?`）。自写，因 core 禁止外部依赖。
pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    // 经典回溯匹配：'*' 吞噬任意长度，'?' 匹配单个字符。
    let (mut pi, mut ni) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Q-Dir.ini [Filter2] 的 12 行实测证据（原版默认规则表）。
/// 直接喂给 `RuleSet::parse_ini_section`，保证默认值与用户文件逐字节一致。
const FILTER2_EVIDENCE_LINES: [&str; 12] = [
    "0=#Hidden=1=-1=1=1=-1=0",
    "1=*.zip;*.rar;*.gz;*.7z;=1=00AA00=-1=1=-1=0",
    "2=*.mp*;*.avi;*.wma;=1=DD0058=-1=1=-1=0",
    "3=#DIR=1=008800=-1=1=-1=0",
    "4=*.jpg;*.jpeg;*.png,*.gif;*.bmp;*.ico=1=BB00BB=-1=1=-1=0",
    "5=*.html;*.htm;*.url=1=456789=-1=1=-1=0",
    "6=*.pl;*.cgi;*.php;*.pdf;*.doc;*.rtf;*.xls=1=BB8844=-1=1=-1=0",
    "7=*.cpp;*.hpp;*.h=1=DD0058=-1=1=-1=0",
    "8=*.exe;*.dll;*.bat=1=FF0000=-1=1=-1=0",
    "9=*=1=0000BB=-1=1=-1=0",
    "10=#BG=1=-1=-1=1=-1=0",
    "11=#BG-A=1=-1=-1=1=-1=0",
];

/// 由实测证据构造默认规则集。
/// 静态表恒可解析（单测 `config_defaults_match_ini_evidence` 锁定）。
fn default_rules() -> RuleSet {
    let lines: Vec<String> = FILTER2_EVIDENCE_LINES
        .iter()
        .map(|s| s.to_string())
        .collect();
    // 不变式：FILTER2_EVIDENCE_LINES 是原版 ini 的逐字节拷贝，必为合法 8 字段行。
    RuleSet::parse_ini_section(&lines).expect("Filter2 证据规则表必须是合法 ini 行")
}

// ============ 7. 标签页 ============

/// 单个标签页。tab_new_tab_l_dblclk / tab_new_tab_l_clk / tab_close_by_mbt_clk 等键作用于它。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabState {
    /// 标签页标题（通常为目录名）。
    pub title: String,
    /// 所在目录。
    pub path: PathBuf,
    /// 导航历史（前进/后退栈）。
    pub history: Vec<PathBuf>,
    /// 当前在 history 中的下标。
    pub history_pos: usize,
    /// 该标签页的视图模式（可覆盖窗格默认）。
    pub view_mode: Option<ViewMode>,
    pub sort_key: SortKey,
    pub sort_dir: SortDir,
    /// 过滤串（by_filter / last_filter）。
    pub filter: String,
}

impl TabState {
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        TabState {
            title,
            path: path.clone(),
            history: vec![path],
            history_pos: 0,
            view_mode: None,
            sort_key: SortKey::Name,
            sort_dir: SortDir::Asc,
            filter: String::new(),
        }
    }

    pub fn can_back(&self) -> bool {
        self.history_pos > 0
    }

    pub fn can_forward(&self) -> bool {
        self.history_pos + 1 < self.history.len()
    }

    /// 后退一步；已到栈底返回 QError::Session。
    pub fn back(&mut self) -> QResult<()> {
        if !self.can_back() {
            return Err(QError::Session(format!(
                "标签页 {:?} 已到历史栈底，无法后退",
                self.title
            )));
        }
        self.history_pos -= 1;
        self.path = self.history[self.history_pos].clone();
        Ok(())
    }

    /// 前进一步；已到栈顶返回 QError::Session。
    pub fn forward(&mut self) -> QResult<()> {
        if !self.can_forward() {
            return Err(QError::Session(format!(
                "标签页 {:?} 已到历史栈顶，无法前进",
                self.title
            )));
        }
        self.history_pos += 1;
        self.path = self.history[self.history_pos].clone();
        Ok(())
    }

    /// 导航到新目录：截断前进栈，压入新路径，刷新标题。
    pub fn navigate(&mut self, path: PathBuf) {
        self.history.truncate(self.history_pos + 1);
        self.history.push(path.clone());
        self.history_pos = self.history.len() - 1;
        self.path = path;
        self.title = self
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string());
    }
}

// ============ 8. 窗格状态 ============

/// 单个窗格（Ploder1..4）的完整可序列化状态。
/// start.qdr 键对应：Folder1 -> tabs[tabs_pos].path；ViewMode1 -> view_mode；
/// Tree1/Tree1split -> 目录树开关与分割；ColsAndSort1 -> columns。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneState {
    pub id: PaneId,
    /// 标签页列表，至少 1 个。
    pub tabs: Vec<TabState>,
    /// 当前标签页下标。
    pub tabs_pos: usize,
    /// 是否显示目录树（Tree1..4）。
    pub tree_visible: bool,
    /// 目录树与列表的分割位置，千分比 0..=1000（Tree1split=2500 万分数）。
    pub tree_split: u32,
    /// 列宽/列显隐/列顺序配置（ColsAndSort 的纯 Rust 投影）。
    pub columns: ColumnConfig,
}

impl PaneState {
    pub fn new(id: PaneId, path: PathBuf) -> Self {
        PaneState {
            id,
            tabs: vec![TabState::new(path)],
            tabs_pos: 0,
            tree_visible: false,
            // Tree1split=2500（万分数）-> 250 千分比，四个窗格实测一致。
            tree_split: 250,
            columns: ColumnConfig::default(),
        }
    }

    pub fn current_tab(&self) -> Option<&TabState> {
        self.tabs.get(self.tabs_pos)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut TabState> {
        self.tabs.get_mut(self.tabs_pos)
    }

    /// 当前目录。
    /// 不变式：tabs 永不为空（new/add_tab/close_tab 共同维护），空列表时返回空路径。
    pub fn path(&self) -> PathBuf {
        self.current_tab()
            .map(|t| t.path.clone())
            .unwrap_or_default()
    }

    /// 生效视图模式：标签页覆盖 > 默认 LargeIcon。
    pub fn view_mode(&self) -> ViewMode {
        self.current_tab()
            .and_then(|t| t.view_mode)
            .unwrap_or_default()
    }

    /// 新建标签页并切过去。
    pub fn add_tab(&mut self, path: PathBuf) {
        self.tabs.push(TabState::new(path));
        self.tabs_pos = self.tabs.len() - 1;
    }

    /// 关闭标签页；最后一个不可关闭（原版行为）。
    pub fn close_tab(&mut self, idx: usize) -> QResult<()> {
        if self.tabs.len() <= 1 {
            return Err(QError::Session(
                "最后一个标签页不可关闭（原版行为）".to_string(),
            ));
        }
        if idx >= self.tabs.len() {
            return Err(QError::Session(format!(
                "标签页下标越界：{idx}（共 {} 个）",
                self.tabs.len()
            )));
        }
        self.tabs.remove(idx);
        if self.tabs_pos >= self.tabs.len() {
            self.tabs_pos = self.tabs.len() - 1;
        }
        Ok(())
    }
}

// ============ 9. 列配置 ============

/// 详情视图一列。原版 ColsAndSort 是十六进制封装，其中可见
/// GroupByKey:FMTID / GroupByDirection / Sort / ColInfo / IconSize /
/// GroupView / LogicalViewMode / GroupByKey:PID / FFlags 等子串。
/// 重写不逐字节复刻该二进制格式，用等价结构保存并在保存时重新封装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    /// 列标识，如 "Name"/"Ext"/"Size"/"Modified"/"Attr"。
    pub key: String,
    /// 显示宽度（像素）。
    pub width: u32,
    pub visible: bool,
    /// 在该视图中的次序。
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnConfig {
    pub columns: Vec<ColumnDef>,
    pub sort_key: SortKey,
    pub sort_dir: SortDir,
    pub view_mode: ViewMode,
    /// ImageSize（图标尺寸）。
    pub image_size: u32,
    /// FolderFlags（原 ViewModeFlags1=1075838981 的投影）。
    pub folder_flags: u32,
    /// GroupBy 字符串，可为空。
    pub group_by: String,
    /// Group 开关。
    pub group: bool,
}

impl Default for ColumnConfig {
    // 默认 Name/Ext/Size/Modified/Attr，Detail，image_size=16。
    // image_size=16 与 folder_flags=1075838981 来自 start.qdr / Q-Dir.ini 实测值。
    fn default() -> Self {
        ColumnConfig {
            columns: vec![
                ColumnDef { key: "Name".into(), width: 200, visible: true, order: 0 },
                ColumnDef { key: "Ext".into(), width: 60, visible: true, order: 1 },
                ColumnDef { key: "Size".into(), width: 80, visible: true, order: 2 },
                ColumnDef { key: "Modified".into(), width: 140, visible: true, order: 3 },
                ColumnDef { key: "Attr".into(), width: 60, visible: true, order: 4 },
            ],
            sort_key: SortKey::Name,
            sort_dir: SortDir::Asc,
            view_mode: ViewMode::Detail,
            image_size: 16,
            folder_flags: 1075838981,
            group_by: String::new(),
            group: false,
        }
    }
}

impl ColumnConfig {
    /// 生成视图状态串：`ViewMode,%d;ImageSize,%d;FolderFlags,%d;GroupBy,%s;Group,%d;`
    pub fn to_view_state_string(&self) -> String {
        format!(
            "ViewMode,{};ImageSize,{};FolderFlags,{};GroupBy,{};Group,{};",
            self.view_mode.to_u32(),
            self.image_size,
            self.folder_flags,
            self.group_by,
            if self.group { 1 } else { 0 },
        )
    }

    /// 解析上面的状态串；未知键忽略以保证前向兼容。
    pub fn parse_view_state_string(s: &str) -> QResult<Self> {
        let mut cfg = ColumnConfig::default();
        for chunk in s.split(';') {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }
            let (k, v) = match chunk.split_once(',') {
                Some(kv) => kv,
                None => continue, // 无 ',' 的片段忽略（前向兼容）
            };
            match k {
                "ViewMode" => {
                    let n: u32 = v.parse().map_err(|_| {
                        QError::config("ViewMode", format!("不是整数：{v:?}"))
                    })?;
                    cfg.view_mode = ViewMode::from_u32(n)
                        .ok_or_else(|| QError::config("ViewMode", format!("越界：{n}")))?;
                }
                "ImageSize" => {
                    cfg.image_size = v
                        .parse()
                        .map_err(|_| QError::config("ImageSize", format!("不是整数：{v:?}")))?;
                }
                "FolderFlags" => {
                    cfg.folder_flags = v
                        .parse()
                        .map_err(|_| QError::config("FolderFlags", format!("不是整数：{v:?}")))?;
                }
                "GroupBy" => cfg.group_by = v.to_string(),
                "Group" => {
                    let n: u32 = v
                        .parse()
                        .map_err(|_| QError::config("Group", format!("不是整数：{v:?}")))?;
                    cfg.group = n != 0;
                }
                _ => {} // 未知键忽略（前向兼容）
            }
        }
        Ok(cfg)
    }
}

// ============ 10. 分割条 ============

/// 千分比分割位置。原版 start.qdr 实测 SP0=5000 SP1=5003 SP2=5003 SPT=2000。
/// 这些值量纲是**万分之一**（5000=50%，SPT=2000=20% 预览高度），
/// 且 5003 之和不为 10000 说明有取整累积。内部统一换算为 0..=1000 千分比存储。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SplitPermille(pub u32);

impl SplitPermille {
    pub const MAX: u32 = 1000;

    /// 从 .qdr 万分数构造（value/10），截断到 0..=1000。
    pub fn from_qdr_ten_thousandths(v: u32) -> Self {
        SplitPermille::from_permille(v / 10)
    }

    /// 输出回 .qdr 万分数。
    pub fn to_qdr_ten_thousandths(self) -> u32 {
        self.0.saturating_mul(10)
    }

    /// 从千分比直接构造。
    pub fn from_permille(v: u32) -> Self {
        SplitPermille(v.min(Self::MAX))
    }

    pub fn value(self) -> u32 {
        self.0
    }

    /// 0.0..=1.0，供 egui 布局直接使用。
    pub fn ratio(self) -> f32 {
        self.0 as f32 / Self::MAX as f32
    }
}

// ============ 11. 会话状态 ============

/// 默认窗格路径。
/// 原版 start.qdr 的 Folder1..4 是「此电脑」的 PIDL
/// （CLSID {20D04FE0-3AEA-1069-A2D8-08002B30309D}），core 无 shell 依赖、
/// 无法把该 PIDL 解码成文件系统路径，故按契约回退到 USERPROFILE（差异已登记）。
fn default_pane_path() -> PathBuf {
    match std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from("."),
    }
}

/// 目录树显示模式（原版 `tree_mode` 键，取值 0/1/2 由 `FUN_0046dc10` 反编译确证，
/// 见 NATIVE_SPEC 附录 A.3）：关 / 双列（树与列表并排）/ 覆盖（树浮在列表上方）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeMode {
    #[default]
    Off,
    Split,
    Overlay,
}

impl TreeMode {
    pub fn to_u32(self) -> u32 {
        match self {
            TreeMode::Off => 0,
            TreeMode::Split => 1,
            TreeMode::Overlay => 2,
        }
    }

    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => TreeMode::Split,
            2 => TreeMode::Overlay,
            _ => TreeMode::Off,
        }
    }

    pub fn is_active(self) -> bool {
        !matches!(self, TreeMode::Off)
    }

    /// viewtree 按钮（0xA773）确证语义：开↔关切换（原 0x806c/0x806b）。
    pub fn toggled(self) -> Self {
        if self.is_active() { TreeMode::Off } else { TreeMode::Split }
    }

    /// 54006 系菜单确证语义：+1 循环（原 0x806a）。
    pub fn cycled(self) -> Self {
        match self {
            TreeMode::Off => TreeMode::Split,
            TreeMode::Split => TreeMode::Overlay,
            TreeMode::Overlay => TreeMode::Off,
        }
    }
}

/// 整个窗口/会话的可序列化状态，对应 start.qdr。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    /// 版本串，实测 "12.71"。
    pub version: String,
    /// 全局 ViewMode（顶层键，实测 0）。
    pub view_mode: u32,
    /// 四个窗格。
    pub panes: [PaneState; 4],
    /// 水平分割条位置（SP0/SP1/SP2），千分比。
    pub splits_h: [SplitPermille; 3],
    /// 预览面板分割位置（SPT），千分比。
    pub split_preview: SplitPermille,
    /// 目录树显示模式（原版 `tree_mode` 键，FUN_0046dc10 确证 0/1/2）。
    pub tree_mode: TreeMode,
    /// 预览面板是否显示（Vorschau）。
    pub preview_visible: bool,
    /// 预览面板分割位置（VorschauPos=7500 万分数）。
    pub preview_pos: SplitPermille,
    /// 当前活动窗格（Focus=1）。
    pub focus: PaneId,
    /// 配色主题（colorthema=0）。
    pub colorthema: u32,
    /// 各窗格 ColsAndSort 原始十六进制串，保留以便原样回写。
    pub cols_and_sort_raw: [String; 4],
}

impl Default for SessionState {
    // 与 start.qdr 实测一致：version=12.71, ViewMode=0, SP0=5000/SP1=5003/SP2=5003
    // -> 500/500/500 千分比，SPT=2000 -> 200，Tree*=0，Tree*split=2500 -> 250，
    // Vorschau=0，VorschauPos=7500 -> 750，Focus=1，colorthema=0。
    fn default() -> Self {
        SessionState {
            version: "12.71".to_string(),
            view_mode: 0,
            panes: [
                PaneState::new(PaneId::P1, default_pane_path()),
                PaneState::new(PaneId::P2, default_pane_path()),
                PaneState::new(PaneId::P3, default_pane_path()),
                PaneState::new(PaneId::P4, default_pane_path()),
            ],
            splits_h: [
                SplitPermille(500),
                SplitPermille(500),
                SplitPermille(500),
            ],
            split_preview: SplitPermille(200),
            tree_mode: TreeMode::default(),
            preview_visible: false,
            preview_pos: SplitPermille(750),
            focus: PaneId::P1,
            colorthema: 0,
            cols_and_sort_raw: Default::default(),
        }
    }
}

impl SessionState {
    pub fn pane(&self, id: PaneId) -> &PaneState {
        &self.panes[(id.number() - 1) as usize]
    }

    pub fn pane_mut(&mut self, id: PaneId) -> &mut PaneState {
        &mut self.panes[(id.number() - 1) as usize]
    }

    /// 活动窗格引用。
    pub fn focused(&self) -> &PaneState {
        self.pane(self.focus)
    }

    pub fn focused_mut(&mut self) -> &mut PaneState {
        let focus = self.focus;
        self.pane_mut(focus)
    }
}

// ============ 12. 配置 ============

/// 目录树配置键。字段名与 ini 键一一对应（证据列出，全部需在设置里可改）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeConfig {
    pub no_expando: bool,          // tree_no_expando
    pub single_expand: bool,       // tree_single_expand
    pub fullrowselect: bool,       // tree_fullrowselect
    pub expand_when_select: bool,  // tree_expand_when_select
    pub all_browsable: bool,       // tree_all_browsable
    pub line_at_root: bool,        // tree_line_at_root
    pub disable_line: bool,        // tree_disable_line
    pub auto_chk_subdir: bool,     // tree_auto_chk_subdir
    pub indent_width: u32,         // tree_indent_width
    pub auto_expand_from_list: bool, // tree_auto_expand_from_list
    pub auto_sel: bool,            // tree_auto_sel
    pub hover_time: u32,           // tree_hover_time（毫秒）
    pub mode: TreeMode,            // tree_mode
}

impl Default for TreeConfig {
    // 这些键在实测 Q-Dir.ini 中缺失，取原版行为等价默认值；
    // 未取证的数值项（indent_width / hover_time）为约定值，已登记。
    fn default() -> Self {
        TreeConfig {
            no_expando: false,
            single_expand: false,
            fullrowselect: false,
            expand_when_select: false,
            all_browsable: false,
            line_at_root: true,
            disable_line: false,
            auto_chk_subdir: false,
            indent_width: 16,
            auto_expand_from_list: false,
            auto_sel: false,
            hover_time: 500,
            mode: TreeMode::default(),
        }
    }
}

/// 标签页行为配置键。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabConfig {
    pub new_tab_l_dblclk: bool,    // tab_new_tab_l_dblclk
    pub new_tab_l_clk: bool,       // tab_new_tab_l_clk
    pub show_tooltips: bool,       // tab_show_tooltips
    pub close_by_mbt_clk: bool,    // tab_close_by_mbt_clk
    pub close_by_lbt_dblclk: bool, // tab_close_by_lbt_dblclk
    pub max_width: u32,            // tab_max_width
    pub width_mode: u32,           // tab_width_mode：0=固定 1=自适应
    pub use_context_menu: bool,    // tab_use_context_menu
    pub hover_time: u32,           // tabs_hover_time（毫秒）
}

impl Default for TabConfig {
    // 同样未在实测 ini 中出现，取原版等价默认值（已登记为约定值）。
    fn default() -> Self {
        TabConfig {
            new_tab_l_dblclk: true,
            new_tab_l_clk: false,
            show_tooltips: true,
            close_by_mbt_clk: true,
            close_by_lbt_dblclk: false,
            max_width: 0,
            width_mode: 0,
            use_context_menu: true,
            hover_time: 500,
        }
    }
}

/// 文件操作与刷新行为配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOpsConfig {
    /// copy_cut_paste_mode：0=复制 1=剪切。
    pub copy_cut_paste_mode: u32,
    /// move_file_confirmation：移动前确认。
    pub move_file_confirmation: bool,
    /// refresh_keep_sel：刷新后保持选择。
    pub refresh_keep_sel: bool,
    /// refresh_keep_scroll：刷新后保持滚动位置。
    pub refresh_keep_scroll: bool,
    /// select_last_if_navigate_up：向上导航时选中上次项。
    pub select_last_if_navigate_up: bool,
}

impl Default for FileOpsConfig {
    fn default() -> Self {
        FileOpsConfig {
            copy_cut_paste_mode: 0,
            move_file_confirmation: true,
            refresh_keep_sel: true,
            refresh_keep_scroll: true,
            select_last_if_navigate_up: false,
        }
    }
}

/// Q-Dir.ini 的完整内存投影。原始 ini 的其它键（如 WinRC）由
/// config::ini::IniDocument 以有序键值表原样保留，保存时整体写回。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    // [Start]
    pub qdir_id: u32,              // QDir_Id
    pub use_color_start: bool,     // useColorStart
    pub als: u32,                  // Als
    pub designe_mode: u32,         // designe_mode
    pub show_cmd: u32,             // showCmd
    pub start_crash: bool,         // StartCrash
    pub win_rc: [i32; 4],          // WinRC=0;0;800;600
    // [Favoriten] / [Q]
    pub favoriten_dir: PathBuf,    // Ordner=.\Favoriten\
    pub quick_link_dir: PathBuf,   // Link=.\Favoriten\Quick-Link\
    // [Preview]
    pub preview_filter: String,    // Filter=*;
    // [Filter2]
    pub rules: RuleSet,
    // 目录树 / 标签页 / 文件操作
    pub tree: TreeConfig,
    pub tab: TabConfig,
    pub file_ops: FileOpsConfig,
    // 其余配置键（全部需在设置里可改）
    pub columns_plus_view: bool,   // columns_plus_view
    pub colorthema: u32,           // colorthema
    pub langid: u32,               // langid
    pub font_face_name: String,    // FontFaceName
    pub lang_name: String,         // LangName
    pub show_filter: bool,         // show_filter
    pub last_filter: String,       // last_filter
    pub by_dir_filter: String,     // by_dir_filter
    pub by_filter: String,         // by_filter
    pub markierungsfilter: String, // Markierungsfilter
    pub last_file_filter: String,  // Last-File-Filter
    pub fav_filter: String,        // Fav_Filter
}

impl Default for Config {
    // 与实测 ini 一致：qdir_id=1, als=5, show_cmd=3, langid=1031。
    fn default() -> Self {
        Config {
            qdir_id: 1,
            use_color_start: true,
            als: 5,
            designe_mode: 1,
            show_cmd: 3,
            start_crash: false,
            win_rc: [0, 0, 800, 600],
            favoriten_dir: PathBuf::from(".\\Favoriten\\"),
            quick_link_dir: PathBuf::from(".\\Favoriten\\Quick-Link\\"),
            preview_filter: "*;".to_string(),
            rules: default_rules(),
            tree: TreeConfig::default(),
            tab: TabConfig::default(),
            file_ops: FileOpsConfig::default(),
            columns_plus_view: false,
            colorthema: 0,
            langid: 1031,
            font_face_name: String::new(),
            lang_name: String::new(),
            show_filter: false,
            last_filter: String::new(),
            by_dir_filter: String::new(),
            by_filter: String::new(),
            markierungsfilter: String::new(),
            last_file_filter: String::new(),
            fav_filter: String::new(),
        }
    }
}

// ============ 13. 收藏夹 ============

/// 收藏夹条目（Favoriten 目录与 Quick-Link 目录下的项）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Favorite {
    pub name: String,
    pub path: PathBuf,
    /// 是否 Quick-Link（.lnk 语义）。
    pub quick_link: bool,
}

// ============ 14. 导出 ============

/// 导出格式，对应菜单 “Liste &Exportieren” 的
/// Export TXT,CSV,HTML,XLS 与 Tabgetrennt(*.ini)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExportFormat {
    Txt,
    Csv,
    Html,
    Xls,
    TabSeparatedIni,
}

impl ExportFormat {
    pub fn ext(self) -> &'static str {
        match self {
            ExportFormat::Txt => "txt",
            ExportFormat::Csv => "csv",
            ExportFormat::Html => "html",
            ExportFormat::Xls => "xls",
            ExportFormat::TabSeparatedIni => "ini",
        }
    }
}

// ============ 15. 语言 ============

/// 支持的语言。资源 language id 1031（德语）为原始语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    De,
    En,
    ZhCn,
}

impl Lang {
    pub fn from_langid(id: u32) -> Option<Self> {
        match id {
            1031 => Some(Lang::De),
            1033 => Some(Lang::En),
            2052 => Some(Lang::ZhCn),
            _ => None,
        }
    }

    pub fn langid(self) -> u32 {
        match self {
            Lang::De => 1031,
            Lang::En => 1033,
            Lang::ZhCn => 2052,
        }
    }
}

// ============ 16. 测试（全部无需 GUI、无需 Windows API） ============
#[cfg(test)]
mod tests {
    use super::*;

    fn evidence_lines() -> Vec<String> {
        FILTER2_EVIDENCE_LINES
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn item(name: &str, is_dir: bool, is_hidden: bool) -> FileItem {
        FileItem {
            path: PathBuf::from(format!("C:\\x\\{name}")),
            name: name.to_string(),
            is_dir,
            is_hidden,
            size: 10,
            modified: 100,
            icon_index: None,
        }
    }

    #[test]
    fn rgb_roundtrip() {
        assert_eq!(Rgb::parse_hex("00AA00"), Some(Rgb(0x00, 0xAA, 0x00)));
        assert_eq!(Rgb::parse_hex("aabbcc"), Some(Rgb(0xAA, 0xBB, 0xCC)));
        assert_eq!(Rgb::parse_hex("#FF0000"), Some(Rgb(0xFF, 0, 0)));
        assert_eq!(Rgb::parse_hex(""), None);
        assert_eq!(Rgb::parse_hex("zzzzzz"), None);
        assert_eq!(Rgb::parse_hex("12345"), None);
        let c = Rgb(0x0A, 0xB0, 0xCD);
        assert_eq!(c.to_hex(), "0AB0CD");
        assert_eq!(Rgb::parse_hex(&c.to_hex()), Some(c));
    }

    #[test]
    fn pane_id_numbering_matches_ini_keys() {
        assert_eq!(PaneId::P1.number(), 1);
        assert_eq!(PaneId::P4.number(), 4);
        assert_eq!(PaneId::from_number(1), Some(PaneId::P1));
        assert_eq!(PaneId::from_number(4), Some(PaneId::P4));
        assert_eq!(PaneId::from_number(0), None);
        assert_eq!(PaneId::from_number(5), None);
        assert_eq!(PaneId::P1.ini_section(), "Column_OS_10.0_Ploder1");
        assert_eq!(PaneId::P4.ini_section(), "Column_OS_10.0_Ploder4");
        assert_eq!(PaneId::P1.to_string(), "Ploder1");
        assert_eq!(PaneId::ALL.len(), 4);
    }

    #[test]
    fn view_mode_numbers_match_menu_order() {
        assert_eq!(ViewMode::LargeIcon.to_u32(), 0);
        assert_eq!(ViewMode::SmallIcon.to_u32(), 1);
        assert_eq!(ViewMode::List.to_u32(), 2);
        assert_eq!(ViewMode::Detail.to_u32(), 3);
        for v in 0..=3u32 {
            assert_eq!(ViewMode::from_u32(v).map(|m| m.to_u32()), Some(v));
        }
        assert_eq!(ViewMode::from_u32(4), None);
        assert_eq!(ViewMode::default(), ViewMode::LargeIcon);
    }

    #[test]
    fn sort_key_and_dir_numbers() {
        assert_eq!(SortKey::from_u32(0), Some(SortKey::Name));
        assert_eq!(SortKey::from_u32(4), Some(SortKey::Type));
        assert_eq!(SortKey::from_u32(5), None);
        assert_eq!(SortDir::from_u32(0), Some(SortDir::Asc));
        assert_eq!(SortDir::from_u32(1), Some(SortDir::Desc));
        assert_eq!(SortDir::from_u32(2), None);
        assert_eq!(SortDir::default(), SortDir::Asc);
    }

    #[test]
    fn wildcard_basics() {
        assert!(wildcard_match("*.zip", "a.ZIP"));
        assert!(wildcard_match("*.mp*", "song.mp3"));
        assert!(wildcard_match("a?c", "abc"));
        assert!(!wildcard_match("a?c", "ac"));
        assert!(!wildcard_match("*.zip", "a.rar"));
        assert!(wildcard_match("*", "anything"));
        // 单模式内不含 ';'：'*' 不跨越 ';' 字符本身。
        assert!(!wildcard_match("*.jpg;*.png", "x.png"));
        assert!(wildcard_match("*.jpg;*.png", "x.jpg;*.png"));
        assert!(wildcard_match("*.png", "x.png"));
        assert!(!wildcard_match("*.png", "x.gif"));
        // 单模式内不含 ';'：'*' 不跨越 ';' 字符本身（'*' 匹配任意字符但不含 ';'）。
        assert!(!wildcard_match("*.cpp;*.hpp;*.h", "v1.h"));
        assert!(wildcard_match("*.h", "v1.h"));
        assert!(wildcard_match("*.cpp;*.hpp;*.h", "v1.cpp;*.hpp;*.h"));
    }

    #[test]
    fn rule_parse_matches_reverse_engineered_ini() {
        let rs = RuleSet::parse_ini_section(&evidence_lines()).expect("12 行证据必须可解析");
        assert_eq!(rs.rules.len(), 12);
        let r0 = &rs.rules[0];
        assert_eq!(r0.index, 0);
        assert_eq!(r0.pattern, RulePattern::Hidden);
        assert!(r0.enabled);
        assert_eq!(r0.color, None);
        assert_eq!((r0.field4, r0.field5, r0.field6), (1, 1, -1));

        let r1 = &rs.rules[1];
        assert_eq!(
            r1.pattern,
            RulePattern::Wildcards {
                pats: vec![
                    "*.zip".to_string(),
                    "*.rar".to_string(),
                    "*.gz".to_string(),
                    "*.7z".to_string()
                ],
                trailing_sep: true,
            }
        );
        assert_eq!(r1.color, Some(Rgb(0x00, 0xAA, 0x00)));

        let r3 = &rs.rules[3];
        assert_eq!(r3.pattern, RulePattern::Dir);
        assert_eq!(r3.color, Some(Rgb(0x00, 0x88, 0x00)));

        let r4 = &rs.rules[4];
        match &r4.pattern {
            // F2：rule 4 混用 ';' 与 ','，必须拆成 6 个模式。
            RulePattern::Wildcards { pats, trailing_sep } => {
                assert_eq!(pats.len(), 6);
                assert_eq!(pats[2], "*.png");
                assert_eq!(pats[3], "*.gif");
                assert_eq!(*trailing_sep, false);
            }
            other => panic!("rule 4 应为通配符列表，实际 {other:?}"),
        }

        let r9 = &rs.rules[9];
        assert_eq!(r9.pattern, RulePattern::Any);
        assert_eq!(r9.color, Some(Rgb(0x00, 0x00, 0xBB)));

        assert_eq!(rs.rules[10].pattern, RulePattern::BgAlt);
        assert_eq!(rs.rules[11].pattern, RulePattern::BgAlt2);
    }

    #[test]
    fn rule_roundtrip_through_ini_line() {
        let rs = RuleSet::parse_ini_section(&evidence_lines()).expect("可解析");
        for rule in &rs.rules {
            let line = rule.to_ini_line();
            if rule.index == 4 {
                // 已登记差异：rule 4 原始串在「一个串内」混用 ';' 与 ','，
                // 而 RulePattern::Wildcards 只存拆分后的 pats（契约形状，A1 不可加字段）。
                // 回写统一为 ';'：语义等价（F2 两种分隔符等价），
                // 但该行不逐字节一致。其余 11 行必须逐字节一致。
                assert_eq!(
                    line, "4=*.jpg;*.jpeg;*.png;*.gif;*.bmp;*.ico=1=BB00BB=-1=1=-1=0",
                    "rule 4 应统一为 ';' 分隔"
                );
                let back = RuleSet::parse_ini_section(&[line]).expect("可再解析");
                assert_eq!(back.rules[0], *rule, "rule 4 必须语义往返");
                continue;
            }
            assert_eq!(
                line,
                FILTER2_EVIDENCE_LINES[rule.index],
                "rule {} 回写必须逐字节一致",
                rule.index
            );
        }
    }

    #[test]
    fn color_for_first_hit_wins_even_when_default_color() {
        // F1：第一条命中即生效，-1 也不得继续向后匹配。
        let rs = RuleSet::parse_ini_section(&evidence_lines()).expect("可解析");

        let hidden_dir = item("secret", true, true);
        assert_eq!(rs.color_for(&hidden_dir), None, "隐藏目录命中 rule 0（-1）即停");

        let dir = item("docs", true, false);
        assert_eq!(rs.color_for(&dir), Some(Rgb(0x00, 0x88, 0x00)));

        let zip = item("a.zip", false, false);
        assert_eq!(rs.color_for(&zip), Some(Rgb(0x00, 0xAA, 0x00)));

        let other = item("readme.txt", false, false);
        assert_eq!(rs.color_for(&other), Some(Rgb(0x00, 0x00, 0xBB)));

        let exe = item("app.exe", false, false);
        assert_eq!(rs.color_for(&exe), Some(Rgb(0xFF, 0x00, 0x00)));

        // 关闭的规则不参与命中。
        let mut disabled = rs.clone();
        disabled.rules[9].enabled = false;
        assert_eq!(disabled.color_for(&other), None);
    }

    #[test]
    fn bg_variant_alternates_rows() {
        let rs = RuleSet::parse_ini_section(&[
            "10=#BG=1=FF0000=-1=1=-1=0".to_string(),
            "11=#BG-A=1=00FF00=-1=1=-1=0".to_string(),
        ])
        .expect("可解析");
        assert_eq!(rs.bg_variant(0), Some(Rgb(0xFF, 0x00, 0x00)));
        assert_eq!(rs.bg_variant(1), Some(Rgb(0x00, 0xFF, 0x00)));
        assert_eq!(rs.bg_variant(2), Some(Rgb(0xFF, 0x00, 0x00)));
        assert_eq!(rs.bg_variant(3), Some(Rgb(0x00, 0xFF, 0x00)));

        // 实测默认表里 #BG/#BG-A 颜色都是 -1 -> None。
        let evidence = RuleSet::parse_ini_section(&evidence_lines()).expect("可解析");
        assert_eq!(evidence.bg_variant(0), None);
        assert_eq!(evidence.bg_variant(1), None);

        // #BG/#BG-A 不按条目命中（F3）。
        assert!(!evidence.rules[10].matches(&item("x.txt", false, false)));
    }

    #[test]
    fn tab_history_back_forward() {
        let mut tab = TabState::new(PathBuf::from("C:\\a"));
        assert!(!tab.can_back());
        assert!(!tab.can_forward());
        tab.navigate(PathBuf::from("C:\\b"));
        tab.navigate(PathBuf::from("C:\\c"));
        assert_eq!(tab.path, PathBuf::from("C:\\c"));
        assert_eq!(tab.title, "c");
        assert!(tab.can_back());
        assert!(!tab.can_forward());
        tab.back().expect("后退到 b");
        assert_eq!(tab.path, PathBuf::from("C:\\b"));
        assert!(tab.can_forward());
        tab.forward().expect("前进到 c");
        assert_eq!(tab.path, PathBuf::from("C:\\c"));
        tab.back().expect("后退到 b");
        tab.back().expect("后退到 a");
        assert_eq!(tab.path, PathBuf::from("C:\\a"));
        assert!(tab.back().is_err(), "栈底再后退必须报 Session");
        tab.forward().expect("前进到 b");
        tab.navigate(PathBuf::from("C:\\d"));
        assert!(!tab.can_forward(), "新导航必须截断前进栈");
        assert_eq!(tab.history.len(), 3);
    }

    #[test]
    fn pane_tab_close_rules() {
        let mut pane = PaneState::new(PaneId::P1, PathBuf::from("C:\\a"));
        assert!(pane.close_tab(0).is_err(), "最后一个标签页不可关闭");
        pane.add_tab(PathBuf::from("C:\\b"));
        assert_eq!(pane.tabs.len(), 2);
        assert_eq!(pane.tabs_pos, 1);
        assert_eq!(pane.path(), PathBuf::from("C:\\b"));
        pane.close_tab(0).expect("关闭第一个");
        assert_eq!(pane.tabs.len(), 1);
        assert_eq!(pane.tabs_pos, 0);
        assert_eq!(pane.path(), PathBuf::from("C:\\b"));
        assert!(pane.close_tab(5).is_err(), "下标越界必须报错");
        pane.add_tab(PathBuf::from("C:\\c"));
        pane.add_tab(PathBuf::from("C:\\d"));
        pane.close_tab(1).expect("关闭中间");
        assert_eq!(pane.tabs_pos, 1);
        assert_eq!(pane.path(), PathBuf::from("C:\\d"));
        assert!(pane.current_tab().is_some());
        assert!(pane.current_tab_mut().is_some());
    }

    #[test]
    fn pane_view_mode_override_and_tree_default() {
        let mut pane = PaneState::new(PaneId::P2, PathBuf::from("C:\\a"));
        assert_eq!(pane.view_mode(), ViewMode::LargeIcon);
        pane.current_tab_mut().expect("有标签页").view_mode = Some(ViewMode::Detail);
        assert_eq!(pane.view_mode(), ViewMode::Detail);
    }

    #[test]
    fn tree_mode_roundtrip_and_switches() {
        assert_eq!(TreeMode::from_u32(0), TreeMode::Off);
        assert_eq!(TreeMode::from_u32(1), TreeMode::Split);
        assert_eq!(TreeMode::from_u32(2), TreeMode::Overlay);
        assert_eq!(TreeMode::from_u32(7), TreeMode::Off, "野值=关");
        assert_eq!(TreeMode::Overlay.to_u32(), 2);
        // 按钮确证语义（附录 A.3）：toggled 开↔关；cycled +1 回绕。
        assert_eq!(TreeMode::Off.toggled(), TreeMode::Split);
        assert_eq!(TreeMode::Overlay.toggled(), TreeMode::Off);
        assert_eq!(TreeMode::Overlay.cycled(), TreeMode::Off);
    }

    #[test]
    fn pane_state_tree_split_default() {
        let pane = PaneState::new(PaneId::P1, PathBuf::from("C:\\a"));
        assert_eq!(pane.tree_split, 250, "Tree1split=2500 万分数 -> 250 千分比");
        assert!(!pane.tree_visible, "Tree1=0");
    }

    #[test]
    fn split_conversion_qdr_scale() {
        assert_eq!(SplitPermille::from_qdr_ten_thousandths(5000).value(), 500);
        assert_eq!(SplitPermille::from_qdr_ten_thousandths(5003).value(), 500);
        assert_eq!(SplitPermille::from_qdr_ten_thousandths(2000).value(), 200);
        assert_eq!(SplitPermille::from_qdr_ten_thousandths(7500).value(), 750);
        assert_eq!(SplitPermille::from_qdr_ten_thousandths(2500).value(), 250);
        assert_eq!(SplitPermille::from_qdr_ten_thousandths(20000).value(), 1000, "必须截断到 MAX");
        assert_eq!(SplitPermille::from_qdr_ten_thousandths(5000).to_qdr_ten_thousandths(), 5000);
        assert_eq!(SplitPermille(500).ratio(), 0.5);
        assert_eq!(SplitPermille(200).ratio(), 0.2);
        assert_eq!(SplitPermille::MAX, 1000);
        assert_eq!(SplitPermille::default(), SplitPermille(0));
    }

    #[test]
    fn view_state_string_roundtrip() {
        let cfg = ColumnConfig {
            view_mode: ViewMode::Detail,
            image_size: 32,
            folder_flags: 1075838981,
            group_by: "Ext".to_string(),
            group: true,
            ..ColumnConfig::default()
        };
        let s = cfg.to_view_state_string();
        assert_eq!(s, "ViewMode,3;ImageSize,32;FolderFlags,1075838981;GroupBy,Ext;Group,1;");
        let back = ColumnConfig::parse_view_state_string(&s).expect("可解析");
        assert_eq!(back, cfg);
        // 未知键忽略（前向兼容）。
        let back2 = ColumnConfig::parse_view_state_string("ViewMode,2;Future,9;")
            .expect("未知键不得报错");
        assert_eq!(back2.view_mode, ViewMode::List);
        assert!(ColumnConfig::parse_view_state_string("ViewMode,9").is_err());
        assert_eq!(ColumnConfig::default().image_size, 16);
    }

    #[test]
    fn session_defaults_match_qdr_evidence() {
        let mut s = SessionState::default();
        assert_eq!(s.version, "12.71");
        assert_eq!(s.view_mode, 0);
        assert_eq!(s.focus, PaneId::P1);
        assert_eq!(s.splits_h, [SplitPermille(500), SplitPermille(500), SplitPermille(500)]);
        assert_eq!(s.split_preview, SplitPermille(200));
        assert_eq!(s.preview_pos, SplitPermille(750));
        assert_eq!(s.tree_mode, TreeMode::Off);
        assert!(!s.preview_visible);
        assert_eq!(s.colorthema, 0);
        assert_eq!(s.panes.len(), 4);
        assert_eq!(s.pane(PaneId::P1).id, PaneId::P1);
        assert_eq!(s.focused().id, PaneId::P1);
        s.focused_mut().add_tab(PathBuf::from("C:\\tmp"));
        assert_eq!(s.panes[0].tabs.len(), 2);
    }

    #[test]
    fn config_defaults_match_ini_evidence() {
        let c = Config::default();
        assert_eq!(c.qdir_id, 1);
        assert_eq!(c.als, 5);
        assert_eq!(c.designe_mode, 1);
        assert_eq!(c.show_cmd, 3);
        assert_eq!(c.langid, 1031);
        assert!(c.use_color_start);
        assert!(!c.start_crash);
        assert_eq!(c.win_rc, [0, 0, 800, 600]);
        assert_eq!(c.favoriten_dir, PathBuf::from(".\\Favoriten\\"));
        assert_eq!(c.quick_link_dir, PathBuf::from(".\\Favoriten\\Quick-Link\\"));
        assert_eq!(c.preview_filter, "*;");
        assert_eq!(c.rules.rules.len(), 12);
        assert_eq!(c.colorthema, 0);
        assert_eq!(Lang::from_langid(1031), Some(Lang::De));
        assert_eq!(Lang::from_langid(2052), Some(Lang::ZhCn));
        assert_eq!(Lang::from_langid(1033), Some(Lang::En));
        assert_eq!(Lang::from_langid(1041), None);
        assert_eq!(Lang::De.langid(), 1031);
    }

    #[test]
    fn export_format_extensions() {
        assert_eq!(ExportFormat::Txt.ext(), "txt");
        assert_eq!(ExportFormat::Csv.ext(), "csv");
        assert_eq!(ExportFormat::Html.ext(), "html");
        assert_eq!(ExportFormat::Xls.ext(), "xls");
        assert_eq!(ExportFormat::TabSeparatedIni.ext(), "ini");
    }

    #[test]
    fn sort_items_dirs_first_then_key() {
        let mut items = vec![
            item("b.zip", false, false),
            item("adir", true, false),
            item("a.zip", false, false),
            item("zdir", true, false),
        ];
        sort_items(&mut items, SortKey::Name, SortDir::Asc);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["adir", "zdir", "a.zip", "b.zip"]);
        sort_items(&mut items, SortKey::Name, SortDir::Desc);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["zdir", "adir", "b.zip", "a.zip"]);
    }

    #[test]
    fn file_item_ext_and_name_lower() {
        let f = item("README.TXT", false, false);
        assert_eq!(f.ext_lower(), "txt");
        assert_eq!(f.name_lower(), "readme.txt");
        let d = item("NoExt", true, false);
        assert_eq!(d.ext_lower(), "");
    }

    #[test]
    fn qerror_display_and_from_io() {
        let e = QError::config("Als", "不是整数");
        assert!(e.to_string().contains("Als"));
        let io = std::io::Error::from_raw_os_error(5);
        let q = QError::from_io(&io);
        match q {
            QError::Io { code, .. } => assert_eq!(code, 5),
            other => panic!("应为 Io 变体，实际 {other:?}"),
        }
        assert_eq!(QError::Cancelled.to_string(), "用户取消");
    }
}
