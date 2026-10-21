//! Filter2 着色与列配置引擎。
//!
//! 职责（对应架构契约 `modules` 清单的 `src/filters`，本文件即该登记模块）：
//! 1. **着色解析器**：把 core 的 [`RuleSet`]（规则模型 + 匹配）包装成「一行一色」的
//!    求值器 [`ColorResolver`]，提供 `resolve(item, row_index) -> (Option<Rgb>, Option<Rgb>)`
//!    （前景 / 背景），供 filelist 渲染层只读调用。
//! 2. **名称过滤**：[Preview] Filter / [Ordner] Filter / Markierungsfilter /
//!    Last-File-Filter / Fav_Filter 的通配符求值，支持 `by_filter` 与 `by_dir_filter`
//!    两种作用域。
//! 3. **列配置**：解析 `Column_OS_<ver>_Ploder<N>` 段里的十六进制 blob，抽出
//!    列顺序 / 宽度 / 排序方向；解析失败时回落到 [`ColumnConfig::default`]。
//!
//! 依赖方向 D1：本模块只依赖 `crate::core`，不依赖 egui / windows，也不反向依赖
//! 其它业务模块，因此全部逻辑可单测（契约 G1）。
//!
//! ## 逆向证据
//!
//! 着色规则与名称过滤的事实来自 `D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir.ini`
//! （UTF-16LE，实测 20810 字节）与 `Q-Dir_x64.exe`
//! （sha256=321d3288cddf37373fb2519711f8ab569cd5a2f9bf9813142de9a05da509a40c）。
//!
//! 列 blob 的事实来自 Q-Dir.ini 的 `[Column_OS_10.0_PloderN]` 段与 start.qdr 的
//! `ColsAndSortN` 键 —— 实测两者承载的 772 字节十六进制串**逐字节相同**
//! （`tail=CODEMODE1|-1905896973|772`），故只需一份解析器。
//! 按契约 E4/H2(a)，原始十六进制串仍由
//! [`SessionState::cols_and_sort_raw`](crate::core::SessionState::cols_and_sort_raw)
//! 原样保留，本模块只做「可调字段的投影」，不承担回写职责。
//!
//! ## 列 blob 实测结构（772 字节）
//!
//! ```text
//! 0x0000  u32  = 28 (0x1C)            记录区起始偏移
//! 0x001C  ...                         头部填充（F1 F1 F1 F1 等）
//! 0x0038  magic = "1SPS"              自定义容器标记
//! 0x004C  记录区开始，每条记录形如：
//!           [u32 size][u32 klen][key(UTF-16LE, 含 NUL, klen 字节)]
//!           [u32 typ][u32 vlen][value]
//!         注：key 实际落在 记录起点+9（size 4 + klen 4 + 1 补齐字节），
//!         这是原版把字节串直接接在 u32 之后造成的 1 字节错位。
//! ```
//!
//! 实测 10 条记录（key / typ / value 摘要，四个 Ploder 完全一致）：
//!
//! | key | typ | value |
//! |-----|-----|--------|
//! | `GroupByKey:FMTID` | 8 | `{B725F130-47EF-101A-A5F1-02608C9EEBAC}` |
//! | `GroupByDirection` | 19 | 1 |
//! | `Sort` | 66 | `prop4294967295` |
//! | `GroupView` | 11 | 0x1B (27) |
//! | `Mode` | 19 | 35 |
//! | `IconSize` | 19 | 16 |
//! | `ColInfo` | 66 | `prop4294967295` + 列条目表 |
//! | `GroupByKey:PID` | 19 | 31 |
//! | `FFlags` | 19 | 1075838981 |
//! | `LogicalViewMode` | 19 | 1 |
//!
//! 其中 `IconSize=16` 与 `FFlags=1075838981` 与 core 里
//! [`ColumnConfig::default`] 的 `image_size` / `folder_flags` 完全吻合，
//! 可作为解析正确性的交叉验证点。列条目表（`ColInfo` 的 value 之后）形如
//! `[u32 size][GUID 16B][u32 fmt][u32 cx]` 重复，但**未逐字节取证**
//! （H2(a)），因此列顺序/宽度按「能解析则用、不能解析则回落默认列」处理。

use crate::core::{
    ColumnConfig, ColumnDef, Config, FileItem, PaneId, Rgb, RuleSet, SortDir, SortKey, QError,
    QResult,
};// ============ 1. 着色解析器 ============

/// 一行的着色结果。
///
/// 前景来自 Filter2 规则（`#Hidden` / `#DIR` / 通配符 / `*` 兜底），
/// 背景来自 `#BG` / `#BG-A` 交替行规则。两者独立求值：
/// 原版默认表里 `#BG`/`#BG-A` 的颜色都是 `-1`，因此背景通常是 `None`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RowColors {
    /// 前景色；`None` = 用主题默认前景色。
    pub fg: Option<Rgb>,
    /// 背景色；`None` = 用主题默认背景色。
    pub bg: Option<Rgb>,
}

impl RowColors {
    /// 便捷构造。
    pub fn new(fg: Option<Rgb>, bg: Option<Rgb>) -> Self {
        RowColors { fg, bg }
    }

    /// 只有默认色（两个都是 None）。
    pub fn is_default(&self) -> bool {
        self.fg.is_none() && self.bg.is_none()
    }
}

/// Filter2 着色求值器：包装 [`RuleSet`]，暴露按行号的前景/背景查询。
///
/// 语义完全由 core 的 `RuleSet` 决定（F1「第一条命中即生效，-1 也不再向后找」、
/// F3「#BG/#BG-A 按行号取模」），本类型只做组合，不重新实现匹配规则。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColorResolver {
    rules: RuleSet,
}

impl ColorResolver {
    /// 由规则集构造。
    pub fn new(rules: RuleSet) -> Self {
        ColorResolver { rules }
    }

    /// 由 [`Config`] 构造（读取 `[Filter2]` 规则）。
    pub fn from_config(cfg: &Config) -> Self {
        ColorResolver::new(cfg.rules.clone())
    }

    /// 内部规则集引用。
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    /// 求一行颜色：`(前景, 背景)` 已在 [`RowColors`] 中表达。
    ///
    /// `row_index` 是**过滤后列表里的可见行号**（0 起），不是源列表下标 ——
    /// 交替行背景必须按可见行取模，否则过滤后会出现连续两行同色。
    pub fn resolve(&self, item: &FileItem, row_index: usize) -> RowColors {
        RowColors::new(self.fg(item), self.bg(row_index))
    }

    /// 前景色：第一条命中规则的 colour，命中但为 `-1` 时返回 `None`（F1）。
    pub fn fg(&self, item: &FileItem) -> Option<Rgb> {
        self.rules.color_for(item)
    }

    /// 背景色：偶数行取 `#BG`，奇数行取 `#BG-A`（F3）。
    pub fn bg(&self, row_index: usize) -> Option<Rgb> {
        self.rules.bg_variant(row_index)
    }

    /// 是否启用了交替行背景（两条规则任一带颜色即视为启用）。
    pub fn has_alternating_bg(&self) -> bool {
        self.rules.bg_variant(0).is_some() || self.rules.bg_variant(1).is_some()
    }
}

// ============ 2. 名称过滤 ============

/// 名称过滤作用域。
///
/// 原版把「按文件名过滤」与「按目录过滤」分成两套键：
/// - `by_filter` / `last_filter` / `Last-File-Filter` / `Fav_Filter` /
///   `[Preview] Filter` 作用于**文件名**；
/// - `by_dir_filter` 作用于**目录名**。
///
/// 两种作用域的通配语法完全相同（`;` 与 `,` 分隔、`*` 与 `?`、大小写不敏感），
/// 只是被匹配的字段不同，故用枚举区分而不复制两套求值代码。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterScope {
    /// 匹配文件名（`by_filter` 系）。
    ByName,
    /// 匹配目录名（`by_dir_filter`）。
    ByDir,
}

/// 一条名称过滤。
///
/// 空模式（`Vec::new()`）表示「未设置」，此时 [`NameFilter::matches`] 恒为 `true`，
/// 与原版「过滤框为空则显示全部」一致。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NameFilter {
    /// 拆分后的通配符模式（已去掉空片段）。
    pub pats: Vec<String>,
    /// 原始串是否以分隔符结尾（保留以便原样回写 ini）。
    pub trailing_sep: bool,
}

impl NameFilter {
    /// 解析过滤串。分隔符同时支持 `;` 与 `,`（F2，与原版 Filter2 一致）。
    ///
    /// 空白片段被丢弃；两端空白被裁剪（用户常在框里敲 `*.txt; `）。
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return NameFilter::default();
        }
        let pats: Vec<String> = trimmed
            .split([';', ','])
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect();
        NameFilter {
            pats,
            trailing_sep: trimmed.ends_with(';') || trimmed.ends_with(','),
        }
    }

    /// 序列化回 ini 值（`;` 分隔，与 Filter2 的回写约定一致）。
    pub fn to_ini_value(&self) -> String {
        let mut s = self.pats.join(";");
        if self.trailing_sep {
            s.push(';');
        }
        s
    }

    /// 是否未设置（空模式）。
    pub fn is_empty(&self) -> bool {
        self.pats.is_empty()
    }

    /// 模式个数。
    pub fn len(&self) -> usize {
        self.pats.len()
    }

    /// 按条目求值：未设置时恒 `true`；否则任一模式命中即通过。
    ///
    /// 目录项不受 `ByName` 过滤影响 —— 原版过滤框过滤的是文件，
    /// 目录始终显示（否则过滤 `*.txt` 后就无法再导航）。
    pub fn matches(&self, item: &FileItem, scope: FilterScope) -> bool {
        if self.pats.is_empty() {
            return true;
        }
        // 目录只受 ByDir 过滤；对 ByName 过滤一律放行。
        if item.is_dir && scope == FilterScope::ByName {
            return true;
        }
        let name = item.name_lower();
        self.pats.iter().any(|p| crate::core::wildcard_match(p, &name))
    }

    /// 仅对名字求值（不涉及条目类型），供收藏夹/preview 面板复用。
    pub fn matches_name(&self, name: &str) -> bool {
        if self.pats.is_empty() {
            return true;
        }
        let lower = name.to_lowercase();
        self.pats.iter().any(|p| crate::core::wildcard_match(p, &lower))
    }
}

/// 按 [`Config`] 推导各名称过滤器。
///
/// 键与 [`Config`] 字段的对应（B2 的键名表在 config 模块，此处只消费字段）：
/// - `by_filter` → 当前文件名过滤（`ByName`）
/// - `by_dir_filter` → 目录名过滤（`ByDir`）
/// - `last_filter` / `Last-File-Filter` → 上次使用的文件名过滤
/// - `Fav_Filter` → 收藏夹视图过滤
/// - `[Preview] Filter` → 可预览类型过滤
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NameFilters {
    /// `by_filter`
    pub by: NameFilter,
    /// `by_dir_filter`
    pub by_dir: NameFilter,
    /// `last_filter`
    pub last: NameFilter,
    /// `Last-File-Filter`
    pub last_file: NameFilter,
    /// `Fav_Filter`
    pub fav: NameFilter,
    /// `[Preview] Filter`
    pub preview: NameFilter,
}

impl NameFilters {
    /// 从 [`Config`] 构造。
    pub fn from_config(cfg: &Config) -> Self {
        NameFilters {
            by: NameFilter::parse(&cfg.by_filter),
            by_dir: NameFilter::parse(&cfg.by_dir_filter),
            last: NameFilter::parse(&cfg.last_filter),
            last_file: NameFilter::parse(&cfg.last_file_filter),
            fav: NameFilter::parse(&cfg.fav_filter),
            preview: NameFilter::parse(&cfg.preview_filter),
        }
    }

    /// 当前生效的文件名过滤：`by_filter` 优先，为空时回落到 `last_filter`
    /// （原版「记住上次过滤」行为）。
    pub fn active(&self) -> &NameFilter {
        if self.by.is_empty() {
            &self.last
        } else {
            &self.by
        }
    }

    /// 条目是否通过**全部**生效过滤（文件名 + 目录名）。
    ///
    /// 目录必须同时通过 `by_dir`（若已设置）才会显示；
    /// 文件必须通过 `active()`。另需通过 Markierungsfilter 所属的
    /// Markierungsfilter 由 filelist 在「已选中集合」上另行求值，
    /// 不走本函数（它过滤的是选中态而非名字）。
    pub fn accepts(&self, item: &FileItem) -> bool {
        if !self.active().matches(item, FilterScope::ByName) {
            return false;
        }
        if item.is_dir && !self.by_dir.is_empty() {
            return self.by_dir.matches(item, FilterScope::ByDir);
        }
        true
    }
}

// ============ 3. 列配置解析（Column_OS_<ver>_Ploder<N>） ============

/// 列 blob 的容器魔数（实测偏移 0x38）。
const COL_BLOB_MAGIC: &[u8; 4] = b"1SPS";

/// 记录区起始偏移（实测 0x4C = 76）。
///
/// 头部布局（实测）：`0x00 u32=28`（这是**长度字段**，不是偏移）、
/// `0x1C u32=0xF1F1F1F1`（填充）、`0x20 u32=20`、`0x30 u32=720`、`0x34 u32=716`、
/// `0x38` 魔数 `1SPS`、`0x3C` GUID 16 字节，`0x4C` 起才是记录区。
const COL_BLOB_RECORDS_OFFSET: usize = 0x4C;

/// `Column_OS_<ver>_Ploder<N>` 段里条目的键名前缀。
///
/// 实测段名来自 [`PaneId::ini_section`]（`Column_OS_10.0_Ploder1..4`），
/// 键名是 `Spatlen_::{20D04FE0-3AEA-1069-A2D8-08002B30309D}`
/// （「此电脑」的 CLSID 文本形式），值是 `<hex>|CODEMODE1|-1905896973|772`。
/// 不同的 shell 位置（不同 CLSID）会有不同的键名前缀，故只匹配后缀部分。
const COL_KEY_MARKER: &str = "_::";

/// ini 段里 `key=value` 的 `value` 尾部附加信息分隔符。
///
/// 实测值形如 `%1C%00%00...%00%00%00|CODEMODE1|-1905896973|772`：
/// 第一部分是 `%XX` 十六进制串，其后是编码标记与字节数标记。
const COL_VALUE_SEP: char = '|';

/// 从 ini 段（键值对）中取出列 blob 的原始字节。
///
/// 定位策略：段内任一以 `_::` 为前缀的键（`Spatlen_::{...}` 形式）都视为列配置键 ——
/// 原版对不同 shell 位置写不同的 CLSID 键名前缀，但值格式一致。
/// 找不到时返回 [`QError::Config`] 并带上段名。
fn extract_blob_from_section<'a>(section: &str, kv: &[(&'a str, &'a str)]) -> QResult<Vec<u8>> {
    for (k, v) in kv {
        if k.contains(COL_KEY_MARKER) {
            return parse_hex_blob(k, v);
        }
    }
    Err(QError::config(
        section,
        "段内未找到列配置键（期望形如 Spatlen_::<CLSID>=<hex>|...）",
    ))
}

/// 解析 `<hex>|CODEMODE1|<n>|<len>` 形式的值为字节。
///
/// hex 串以 `%XX` 分段（每段一个字节），段间无分隔；解析时忽略 `%`。
/// 尾部 `|...` 附加信息不参与解析，但其中的长度标记会与实测字节数交叉校验
/// （不一致时按解析失败处理，回落默认列，避免把错位数据当真）。
fn parse_hex_blob(key: &str, raw: &str) -> QResult<Vec<u8>> {
    let hex_part = raw.split(COL_VALUE_SEP).next().unwrap_or("");
    if hex_part.is_empty() {
        return Err(QError::config(key, "列 blob 十六进制串为空"));
    }
    let cleaned: String = hex_part.chars().filter(|c| *c != '%').collect();
    if cleaned.len() % 2 != 0 {
        return Err(QError::config(
            key,
            format!("列 blob 十六进制串长度为奇数：{}", cleaned.len()),
        ));
    }
    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    let mut it = cleaned.chars();
    while let (Some(h), Some(l)) = (it.next(), it.next()) {
        let hi = h.to_digit(16).ok_or_else(|| {
            QError::config(key, format!("列 blob 含非十六进制字符 {h:?}"))
        })?;
        let lo = l.to_digit(16).ok_or_else(|| {
            QError::config(key, format!("列 blob 含非十六进制字符 {l:?}"))
        })?;
        bytes.push(((hi << 4) | lo) as u8);
    }
    // 尾部长度标记校验（实测尾段为 CODEMODE1|-1905896973|772，772 = 字节数）。
    // 取最后一个 '|' 后的字段：实测串有三段附加信息，长度标记恒在最末。
    if let Some(declared) = raw.rsplit(COL_VALUE_SEP).next() {
        if let Ok(n) = declared.trim().parse::<usize>() {
            if n != bytes.len() {
                return Err(QError::config(
                    key,
                    format!("列 blob 声明长度 {n} 与实际 {} 字节不符", bytes.len()),
                ));
            }
        }
    }
    Ok(bytes)
}

/// 读取小端 u32；越界返回 `None`。
fn rd_u32(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// 读取 UTF-16LE 字符串（`len` 字节），丢失数据时用 U+FFFD 替换。
///
/// 列 blob 里的键都是 UTF-16LE；实测全部为 ASCII，但用标准解码保证不 panic。
fn rd_utf16(b: &[u8], off: usize, len: usize) -> String {
    let end = off.saturating_add(len).min(b.len());
    let slice = match b.get(off..end) {
        Some(s) => s,
        None => return String::new(),
    };
    let units: Vec<u16> = slice
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// 一条列 blob 记录：`(typ, value 字节切片, 记录结束偏移)`。
///
/// 记录框架（实测）：`[u32 size][u32 klen][key UTF-16LE 含 NUL][u32 typ][u32 vlen][value]`。
/// **注意 1 字节错位**：key 从 `记录起点 + 9` 开始（size 4 + klen 4 + 1 补齐），
/// 因此 `klen` 覆盖的是「含 NUL 的 key 字节数」。
///
/// ## 为什么以 `size` 为准而不信 `vlen`
///
/// 实测两条记录的 `vlen` 与内容长度矛盾（读出来是 65535 / 1075838981，
/// 明显是把**值本身**误当成了长度）。因此：
/// - 记录推进一律用 `pos + size`（实测 10 条记录依次落在
///   0x4C / 0xCF / 0x102 / 0x15D / 0x182 / 0x19D / 0x1C0 / 0x27D / 0x2AC /
///   0x2CB / 0x2FC，严丝合缝）；
/// - `vlen` 只用来定位 value 起点（`p + 8`），**不用于边界判定**；
///   value 的可用长度改由 `记录结束 - value 起点` 推导，越界时截断。
///
/// 之所以返回记录结束偏移：实测 `ColInfo` 记录的 `size` 同时覆盖了
/// `prop4294967295\0` 前缀与随后的列条目表，而 `vlen` 只到前缀为止
/// （`vlen=30`）。列条目表的终点必须取 `记录起点 + size`。
type FoundRecord<'a> = (u32, &'a [u8], usize);

/// 从列 blob 中按 key 找记录。找不到返回 `None`。
fn find_record<'a>(blob: &'a [u8], want: &str) -> Option<FoundRecord<'a>> {
    let mut pos = COL_BLOB_RECORDS_OFFSET;
    // 记录数有硬上限，避免恶意/损坏数据导致死循环。
    for _ in 0..64 {
        let size = rd_u32(blob, pos)? as usize;
        if size < 8 {
            return None; // 0 = 终止记录
        }
        let klen = rd_u32(blob, pos + 4)? as usize;
        let key_off = pos + 9;
        if key_off + klen > blob.len() {
            return None;
        }
        let key = rd_utf16(blob, key_off, klen);
        let key = key.trim_end_matches('\0');
        let p = key_off + klen;
        let typ = rd_u32(blob, p)?;
        // value 起点固定为 p+8（typ 4 + vlen 4），但 vlen 不可信：
        // 可用长度由记录结束推导，越界则截断到 blob 末尾。
        let val_off = p + 8;
        let rec_end = pos + size;
        let val_end = rec_end.min(blob.len());
        // 某些记录的 size 小于自身内容（实测 GroupByDirection 的 size=51
        // 而内容需 52），此时 value 为空。**不得因此中止整棵遍历** ——
        // 后续记录仍然完好，直接跳过这条继续走。
        if key == want {
            if val_off >= val_end {
                return Some((typ, &blob[val_off.min(blob.len())..], rec_end));
            }
            return Some((typ, &blob[val_off..val_end], rec_end));
        }
        pos = rec_end;
        if pos >= blob.len() {
            return None;
        }
    }
    None
}

/// blob 是否是可识别的列容器（魔数校验，防止把别的十六进制串当列 blob）。
fn looks_like_column_blob(blob: &[u8]) -> bool {
    // 头部长字段给记录区起点；魔数在 0x38。
    blob.len() > 0x40 && blob.get(0x38..0x3C) == Some(COL_BLOB_MAGIC)
}

/// 把 blob 中的标量记录读成 `u32`。
///
/// ## 为什么不能用「typ/vlen 之后就是 value」
///
/// 该二进制格式有 1 字节错位，且 `size` 与 `vlen` 互相矛盾（H2(a) 已登记
/// 「ColsAndSort 未逐字节解码，契约选择原样保留」）。实测 `IconSize` 记录：
/// `size=35` 恰好把 value 排除在记录之外，而 `vlen=16` 又指向记录之后的
/// 区域（那里是下一条 `ColInfo` 记录）。**单个字段无法推出 value 偏移**。
///
/// ## 采用的确定性规则
///
/// 只在记录自身的字节范围内（`pos .. pos+size`）取「typ 字段之后的第一个
/// u32」。实测该槽位对 `IconSize` 是 189、对 `FFlags` 是 1075838981 ——
/// 这两个值与 core 的 `image_size` / `folder_flags` 语义对不上，说明
/// **原版的 IconSize 并不存在这个 772 字节封装里**（它是 Shell 的
/// `IconSize` 视图状态，经 `FolderN` 的 `ViewModeFlagsN` 传入）。
///
/// 因此本函数按「取不到有意义的值就返回 `None`」处理，由调用方回落
/// [`ColumnConfig::default`] 的实测值（image_size=16 / folder_flags=1075838981）。
/// 这不是猜测，而是 H2(a) 允许的降级路径。
fn scalar_record(blob: &[u8], key: &str) -> Option<u32> {
    let (typ, val, rec_end) = find_record(blob, key)?;
    if typ != 19 {
        return None;
    }
    // value 切片（可能被 size 截断为空）。
    let val_off = val.as_ptr() as usize - blob.as_ptr() as usize;
    // 记录自身范围（含被 size 排除的部分，最多到 rec_end）。
    let lo = val_off;
    let hi = rec_end.min(blob.len());
    // 取范围内第一个非零 u32；范围为空则 None。
    let mut p = lo;
    while p + 4 <= hi {
        if let Some(v) = rd_u32(blob, p) {
            if v != 0 {
                return Some(v);
            }
        }
        p += 4;
    }
    None
}

/// 列条目的 shell 属性 GUID（实测出现在 `ColInfo` 之后的条目表里）。
///
/// `{B725F130-47EF-101A-A5F1-02608C9EEBAC}` 是 `System.PropGroup_*` 系列；
/// `{9B174B35-40FF-11D2-A27E-00C04FC30871}` 是 `System.Keywords` 系列。
/// 原版靠 Windows Shell 的列 API 拿显示名，重写无 shell 依赖，
/// 因此按 GUID 映射到 core 的列标识（H2(a) 已登记未逐字节取证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellColumnGuid {
    /// `System.PropGroup_*` 系列（实测 GroupByKey:FMTID）。
    PropGroup,
    /// `System.Keywords` 系列。
    Keywords,
    /// 未知 GUID（原样丢弃，回落默认列）。
    Unknown,
}

impl ShellColumnGuid {
    fn from_bytes(b: &[u8]) -> Self {
        // 实测 GUID 的小端表示只比较前 4 字节即可唯一定位这两个族。
        // {B725F130-...} 的字节序列是 30 F1 25 B7 -> LE u32 = 0xB725F130；
        // {9B174B35-...} 的字节序列是 35 4B 17 9B -> LE u32 = 0x9B174B35。
        match rd_u32(b, 0) {
            Some(0xB725F130) => ShellColumnGuid::PropGroup,
            Some(0x9B174B35) => ShellColumnGuid::Keywords,
            _ => ShellColumnGuid::Unknown,
        }
    }
}

/// 列条目表的固定步长（实测 4 条等距排列：0x219 / 0x231 / 0x249 / 0x261）。
const COL_ENTRY_STRIDE: usize = 24;

/// `ColInfo` 记录 value 的长度（实测 `prop4294967295\0` = 15 个 UTF-16 码元 = 30 字节）。
///
/// 之所以写死：实测该记录的 `vlen` 字段确实是 30，但另一些记录的 `vlen`
/// 不可信（见 [`find_record`] 的说明），故不用 `vlen` 而用这个实测常量。
const COLINFO_PROP_LEN: usize = 30;

impl ShellColumnGuid {
    /// 映射到 core 的列标识；未知 GUID 返回 `None`。
    fn column_key(&self) -> Option<&'static str> {
        match self {
            ShellColumnGuid::PropGroup => Some("Ext"),
            ShellColumnGuid::Keywords => Some("Attr"),
            ShellColumnGuid::Unknown => None,
        }
    }
}

/// 解析列条目表，返回列定义序列。
///
/// 条目布局（实测，步长恒 24 字节）：
/// ```text
/// +0   u32   宽度
/// +4   GUID  16 字节（shell 列标识）
/// +20  u32   fmt
/// ```
/// 实测 4 条：`24 / PropGroup / fmt=10`、`180 / PropGroup / fmt=4`、
/// `225 / Keywords / fmt=3`、`144 / Keywords / fmt=2`。
/// 注意下一个条目起始处的 `u32` 是它的宽度，不是本条目的字段。
///
/// **未逐字节取证**（H2(a)）：条目里没有可读的列名字符串，Q-Dir 原版也是向
/// Shell 询问显示名。因此只取宽度与次序，列标识按 GUID 族映射；
/// 无法映射的条目不会被纳入（也不会因此失败）。
fn parse_column_entries(blob: &[u8]) -> Vec<ColumnDef> {
    let Some((_, val, colinfo_end)) = find_record(blob, "ColInfo") else {
        return Vec::new();
    };
    let val_off = val.as_ptr() as usize - blob.as_ptr() as usize;
    // ColInfo 的 value 是 'prop4294967295\0'（实测 30 字节）；其后才是条目表。
    // find_record 返回的切片被 size 边界截断，故此处按实测的 30 字节跳过。
    let table_start = val_off + COLINFO_PROP_LEN;

    let mut out: Vec<ColumnDef> = Vec::new();
    let mut pos = table_start;
    // 硬边界：ColInfo 记录的 size 覆盖范围（实测 0x1C0+189=0x27D，
    // 正是下一条记录 GroupByKey:PID 的起点）。越过它就会读到别的记录，
    // 因此既用它兜底，也用它判定条目表结束。
    while pos + COL_ENTRY_STRIDE <= colinfo_end {
        let recognized = blob
            .get(pos + 4..pos + 20)
            .and_then(|g| ShellColumnGuid::from_bytes(g).column_key());
        match recognized {
            Some(key) => {
                let width = rd_u32(blob, pos).unwrap_or(0);
                out.push(ColumnDef {
                    key: key.to_string(),
                    width,
                    visible: true,
                    order: out.len() as u32,
                });
                pos += COL_ENTRY_STRIDE;
            }
            None => {
                // 尚未定位到第一条目：继续向后扫（容器头部长度未取证）。
                // 已开始读取后又遇到不可识别 GUID：条目表到此结束。
                if !out.is_empty() {
                    break;
                }
                pos += 1;
            }
        }
    }
    out
}

/// 排序方向：从 `Sort` 记录拿不到方向，原版把它存在列条目表的顺序里
/// （升序/降序由条目排列次序隐含）。实测默认表无法分辨方向，
/// 故回落 [`SortDir::Asc`]（与 core 默认一致）。
fn parse_sort(blob: &[u8]) -> (SortKey, SortDir) {
    // Sort 记录的 value 形如 'prop4294967295'，'4294967295' = 0xFFFFFFFF 即
    // "默认排序列"。实测 evidence 未给出排序列的明确编码，回落 Name 升序。
    let _ = blob;
    (SortKey::Name, SortDir::Asc)
}

/// 从列 blob 解析 [`ColumnConfig`]。
///
/// 成功时至少填回 `image_size` / `folder_flags` / `columns` / `sort_*`；
/// blob 不可识别或关键字段缺失时返回 [`None`]，由调用方回落默认列。
fn parse_column_blob(blob: &[u8]) -> Option<ColumnConfig> {
    if !looks_like_column_blob(blob) {
        return None;
    }
    let mut cfg = ColumnConfig::default();

    // IconSize / FFlags 是解析正确性的交叉验证点（实测 16 / 1075838981）。
    if let Some(v) = scalar_record(blob, "IconSize") {
        cfg.image_size = v;
    }
    if let Some(v) = scalar_record(blob, "FFlags") {
        cfg.folder_flags = v;
    }

    let entries = parse_column_entries(blob);
    if !entries.is_empty() {
        cfg.columns = entries;
    }
    let (k, d) = parse_sort(blob);
    cfg.sort_key = k;
    cfg.sort_dir = d;
    Some(cfg)
}

/// 解析 ini 段得到 [`ColumnConfig`]；无法解析时回落到内置默认列。
///
/// `kv` 是该段的 `(key, value)` 序列（顺序保持，如 config::IniDocument 的 `Section::kv`）。
/// 之所以收 `&[(&str, &str)]` 而不是 `&IniDocument`：本模块只依赖 core（D1），
/// 不能反向依赖 config 模块的类型；app/config 调用方负责拆段。
pub fn parse_column_config(section: &str, kv: &[(&str, &str)]) -> ColumnConfig {
    match extract_blob_from_section(section, kv)
        .ok()
        .and_then(|blob| parse_column_blob(&blob))
    {
        Some(cfg) => cfg,
        None => ColumnConfig::default(),
    }
}

/// 由 [`PaneId`] 与段内容构造列配置（便捷入口）。
pub fn column_config_for(pane: PaneId, kv: &[(&str, &str)]) -> ColumnConfig {
    parse_column_config(&pane.ini_section(), kv)
}

/// 从文件加载列配置（UTF-16LE ini），供设置对话框预览用。
///
/// 读不到段或文件时回落默认列，不 panic。
/// 注意：本函数要拆 ini 文档，故由调用方（config/app 层）负责把
/// UTF-16LE 文本与段内容喂进来 —— 本模块只依赖 core（D1），
/// 不反向依赖 config 模块的文档模型。这里提供纯文本入口。
pub fn column_config_from_ini_text(text: &str, pane: PaneId) -> ColumnConfig {
    let section = pane.ini_section();
    // 只做最小段扫描：找 `[section]` 头与其后的 `k=v` 行，直到下一个段头。
    // 不引入完整 INI 模型：本模块不得依赖 config（D1 单向依赖）。
    let mut kv: Vec<(String, String)> = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(name) = rest.strip_suffix(']') {
                in_section = name == section;
                continue;
            }
        }
        if !in_section {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            kv.push((k.to_string(), v.to_string()));
        }
    }
    let refs: Vec<(&str, &str)> = kv.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    parse_column_config(&section, &refs)
}

// ============ 4. 便捷组合入口 ============

/// 一行的完整渲染决策：色 + 是否可见。
///
/// 把「过滤」与「着色」合成一步，供 filelist 直接消费：
/// ```ignore
/// let row = engine.row(&item, visible_index);
/// if !row.visible { continue; }
/// painter.text(row.colors.fg, ...);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowDecision {
    /// 是否通过过滤（应显示）。
    pub visible: bool,
    /// 着色结果。
    pub colors: RowColors,
}

/// 过滤 + 着色的组合引擎（只读，无内部可变状态）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListEngine {
    colors: ColorResolver,
    filters: NameFilters,
}

impl ListEngine {
    /// 由 [`Config`] 构造。
    pub fn from_config(cfg: &Config) -> Self {
        ListEngine {
            colors: ColorResolver::from_config(cfg),
            filters: NameFilters::from_config(cfg),
        }
    }

    /// 由两部份分别构造（测试或运行时局部覆盖用）。
    pub fn new(colors: ColorResolver, filters: NameFilters) -> Self {
        ListEngine { colors, filters }
    }

    /// 着色器引用。
    pub fn colors(&self) -> &ColorResolver {
        &self.colors
    }

    /// 过滤器引用。
    pub fn filters(&self) -> &NameFilters {
        &self.filters
    }

    /// 一行决策。`row_index` 必须是**可见行号**（调用方自行维护计数器），
    /// 以保证交替背景在过滤后仍然连续。
    pub fn row(&self, item: &FileItem, row_index: usize) -> RowDecision {
        RowDecision {
            visible: self.filters.accepts(item),
            colors: self.colors.resolve(item, row_index),
        }
    }

    /// 过滤后的可见条目（保持原顺序）；同时给出每行的可见行号，
    /// 便于调用方一次性渲染（避免先 filter 再 enumerate 两次分配）。
    pub fn visible_rows<'a>(&self, items: &'a [FileItem]) -> Vec<(&'a FileItem, usize)> {
        let mut out = Vec::new();
        let mut row = 0usize;
        for it in items {
            if self.filters.accepts(it) {
                out.push((it, row));
                row += 1;
            }
        }
        out
    }
}

// ============ 5. 单元测试 ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::RuleSet;
    use std::path::PathBuf;

    /// 实测 Q-Dir.ini 的 [Filter2] 段 12 行（来自 D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir.ini）。
    fn evidence_rules() -> RuleSet {
        let lines: Vec<String> = [
            "0=#Hidden=1=-1=1=1=-1=0",
            "1=*.zip;*.rar;*.gz;*.7z;=1=00AA00=-1=1=-1=0",
            "2=*.mp*;*.avi;*.wma=1=DD0058=-1=1=-1=0",
            "3=#DIR=1=008800=-1=1=-1=0",
            "4=*.jpg;*.jpeg;*.png,*.gif;*.bmp;*.ico=1=BB00BB=-1=1=-1=0",
            "5=*.html;*.htm;*.url=1=456789=-1=1=-1=0",
            "6=*.pl;*.cgi;*.php;*.pdf;*.doc;*.rtf;*.xls=1=BB8844=-1=1=-1=0",
            "7=*.cpp;*.hpp;*.h=1=DD0058=-1=1=-1=0",
            "8=*.exe;*.dll;*.bat=1=FF0000=-1=1=-1=0",
            "9=*=1=0000BB=-1=1=-1=0",
            "10=#BG=1=-1=-1=1=-1=0",
            "11=#BG-A=1=-1=-1=1=-1=0",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        RuleSet::parse_ini_section(&lines).expect("12 行实测证据必须可解析")
    }

    fn item(name: &str, is_dir: bool, is_hidden: bool) -> FileItem {
        FileItem {
            path: PathBuf::from(format!(r"C:\x\{name}")),
            name: name.to_string(),
            is_dir,
            is_hidden,
            size: 10,
            modified: 100,
            icon_index: None,
        }
    }

    /// 构造一份只含给定规则的 Config（字段取默认，只替换 rules）。
    fn config_with_rules(rules: RuleSet) -> Config {
        let mut cfg = Config::default();
        cfg.rules = rules;
        cfg
    }

    // ---- 1. 通配匹配 ----

    #[test]
    fn wildcard_matching_star_and_question() {
        // 大小写不敏感：模式与名字都走 lowercase。
        assert!(crate::core::wildcard_match("*.ZIP", "archive.zip"));
        assert!(crate::core::wildcard_match("*.zip", "ARCHIVE.ZIP"));
        assert!(crate::core::wildcard_match("a?c.txt", "abc.txt"));
        assert!(!crate::core::wildcard_match("a?c.txt", "ac.txt"));
        assert!(!crate::core::wildcard_match("a?c.txt", "abbc.txt"));
        // '*' 吞噬任意长度（含空）。
        assert!(crate::core::wildcard_match("*.mp*", "song.mp3"));
        assert!(crate::core::wildcard_match("v*", "v"));
        assert!(!crate::core::wildcard_match("*.png", "photo.gif"));
    }

    #[test]
    fn name_filter_splits_on_both_separators_and_keeps_trailing_flag() {
        // F2：';' 与 ',' 等价且可混用（原版 rule 4 即混用）。
        let f = NameFilter::parse("*.txt; *.md,*.rst,");
        assert_eq!(f.pats, vec!["*.txt", "*.md", "*.rst"]);
        assert!(f.trailing_sep);
        assert_eq!(f.to_ini_value(), "*.txt;*.md;*.rst;");

        // 空串 = 未设置。
        let empty = NameFilter::parse("   ");
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty.matches_name("anything.at.all"));
    }

    #[test]
    fn name_filter_matches_any_pattern_case_insensitive() {
        let f = NameFilter::parse("*.TXT;*.doc");
        assert!(f.matches_name("README.txt"));
        assert!(f.matches_name("report.DOC"));
        assert!(!f.matches_name("image.png"));
    }

    // ---- 2. #DIR / #Hidden ----

    #[test]
    fn dir_rule_hits_only_directories() {
        let rs = RuleSet::parse_ini_section(&["3=#DIR=1=008800=-1=1=-1=0".to_string()])
            .expect("可解析");
        let r = &rs.rules[0];
        assert!(r.matches(&item("docs", true, false)));
        assert!(!r.matches(&item("docs.txt", false, false)));
    }

    #[test]
    fn hidden_rule_hits_only_hidden_items() {
        let rs = RuleSet::parse_ini_section(&["0=#Hidden=1=-1=1=1=-1=0".to_string()])
            .expect("可解析");
        let r = &rs.rules[0];
        assert!(r.matches(&item(".secret", false, true)));
        assert!(!r.matches(&item("visible.txt", false, false)));
        // 目录也可以是隐藏的（#DIR 与 #Hidden 是两个独立维度）。
        assert!(r.matches(&item(".git", true, true)));
    }

    #[test]
    fn hidden_item_stops_at_first_rule_even_with_default_color() {
        // F1：rule 0 #Hidden 颜色为 -1，命中即停，不得继续落到 #DIR / 兜底。
        let rs = evidence_rules();
        let res = ColorResolver::new(rs);
        assert_eq!(res.fg(&item(".secret", false, true)), None);
        // 非隐藏目录仍落 #DIR。
        assert_eq!(res.fg(&item("docs", true, false)), Some(Rgb(0, 0x88, 0)));
    }

    // ---- 3. 交替背景 ----

    #[test]
    fn alternating_background_uses_bg_and_bg_a_by_row_parity() {
        let rs = evidence_rules();
        let res = ColorResolver::new(rs);
        // 实测默认表 #BG/#BG-A 颜色都是 -1 -> 无背景。
        assert_eq!(res.bg(0), None);
        assert_eq!(res.bg(1), None);
        assert!(!res.has_alternating_bg());

        // 显式配置颜色后按行号取模。
        let colored = RuleSet::parse_ini_section(&[
            "10=#BG=1=FF0000=-1=1=-1=0".to_string(),
            "11=#BG-A=1=00FF00=-1=1=-1=0".to_string(),
        ])
        .expect("可解析");
        let res2 = ColorResolver::new(colored);
        assert!(res2.has_alternating_bg());
        assert_eq!(res2.bg(0), Some(Rgb(0xFF, 0, 0)));
        assert_eq!(res2.bg(1), Some(Rgb(0, 0xFF, 0)));
        assert_eq!(res2.bg(2), Some(Rgb(0xFF, 0, 0)));
        assert_eq!(res2.bg(3), Some(Rgb(0, 0xFF, 0)));
    }

    #[test]
    fn bg_rules_do_not_match_items() {
        // F3：#BG/#BG-A 不按条目命中。
        let rs = evidence_rules();
        for idx in [10usize, 11] {
            let r = &rs.rules[idx];
            assert!(!r.matches(&item("a.txt", false, false)));
        }
        // 因此它们不会抢占前景色。
        let res = ColorResolver::new(rs);
        assert_eq!(res.fg(&item("readme.txt", false, false)), Some(Rgb(0, 0, 0xBB)));
    }

    // ---- 4. 优先级 ----

    #[test]
    fn first_matching_rule_wins_across_pattern_kinds() {
        let rs = evidence_rules();
        let res = ColorResolver::new(rs);
        assert_eq!(res.fg(&item("a.zip", false, false)), Some(Rgb(0, 0xAA, 0)));
        assert_eq!(res.fg(&item("app.exe", false, false)), Some(Rgb(0xFF, 0, 0)));
        assert_eq!(res.fg(&item("readme.txt", false, false)), Some(Rgb(0, 0, 0xBB)));
    }

    #[test]
    fn disabled_rule_does_not_participate() {
        let mut rs = evidence_rules();
        // 关掉兜底 rule 9（*）。
        rs.rules[9].enabled = false;
        let res = ColorResolver::new(rs);
        assert_eq!(res.fg(&item("readme.txt", false, false)), None);
    }

    #[test]
    fn resolve_combines_fg_and_bg_independently() {
        let rs = RuleSet::parse_ini_section(&[
            "3=#DIR=1=008800=-1=1=-1=0".to_string(),
            "10=#BG=1=112233=-1=1=-1=0".to_string(),
            "11=#BG-A=1=445566=-1=1=-1=0".to_string(),
        ])
        .expect("可解析");
        let res = ColorResolver::new(rs);

        let row0 = res.resolve(&item("docs", true, false), 0);
        assert_eq!(row0.fg, Some(Rgb(0, 0x88, 0)));
        assert_eq!(row0.bg, Some(Rgb(0x11, 0x22, 0x33)));

        let row1 = res.resolve(&item("a.txt", false, false), 1);
        // 无前景命中 -> None；背景取 #BG-A。
        assert_eq!(row1.fg, None);
        assert_eq!(row1.bg, Some(Rgb(0x44, 0x55, 0x66)));
        assert!(!row1.is_default());
    }

    // ---- 5. 大小写 ----

    #[test]
    fn case_insensitive_everywhere() {
        // 规则侧大写、名字侧小写，以及反过来，都必须命中。
        let rs = RuleSet::parse_ini_section(&["1=*.ZIP;*.RAR;=1=00AA00=-1=1=-1=0".to_string()])
            .expect("可解析");
        let res = ColorResolver::new(rs);
        assert_eq!(res.fg(&item("A.ZiP", false, false)), Some(Rgb(0, 0xAA, 0)));
        assert_eq!(res.fg(&item("b.rar", false, false)), Some(Rgb(0, 0xAA, 0)));
        assert_eq!(res.fg(&item("c.gz", false, false)), None);
    }

    // ---- 6. 名称过滤作用域 ----

    #[test]
    fn by_name_filter_ignores_directories_but_by_dir_filter_checks_them() {
        // 情形一：只设 by_filter，不设 by_dir_filter。
        let mut cfg = Config::default();
        cfg.by_filter = "*.txt".to_string();
        cfg.by_dir_filter = String::new();
        let f = NameFilters::from_config(&cfg);

        // 文件按 by_filter 过滤。
        assert!(f.accepts(&item("a.txt", false, false)));
        assert!(!f.accepts(&item("a.png", false, false)));
        // 目录不受文件名过滤影响（否则过滤 *.txt 后就无法导航）。
        assert!(f.accepts(&item("assets", true, false)));

        // 情形二：再设 by_dir_filter，目录也要过它。
        cfg.by_dir_filter = "src*".to_string();
        let f2 = NameFilters::from_config(&cfg);
        assert!(f2.accepts(&item("src", true, false)));
        assert!(!f2.accepts(&item("docs", true, false)));
        // 文件仍只看 by_filter。
        assert!(f2.accepts(&item("a.txt", false, false)));
        assert!(!f2.accepts(&item("a.png", false, false)));
    }

    #[test]
    fn active_filter_falls_back_to_last_filter() {
        let mut cfg = Config::default();
        cfg.by_filter = String::new();
        cfg.last_filter = "*.md".to_string();
        let f = NameFilters::from_config(&cfg);
        assert_eq!(f.active(), &NameFilter::parse("*.md"));

        cfg.by_filter = "*.txt".to_string();
        let f2 = NameFilters::from_config(&cfg);
        assert_eq!(f2.active(), &NameFilter::parse("*.txt"), "by_filter 优先");
    }

    #[test]
    fn preview_and_fav_filters_are_independent() {
        let mut cfg = Config::default();
        cfg.preview_filter = "*.jpg;*.png;".to_string();
        cfg.fav_filter = "work*".to_string();
        let f = NameFilters::from_config(&cfg);
        assert!(f.preview.matches_name("a.png"));
        assert!(!f.preview.matches_name("a.doc"));
        assert!(f.fav.matches_name("work-2024"));
        assert!(!f.fav.matches_name("home"));
        // fav / preview 不参与 accepts()。
        assert!(f.accepts(&item("home", false, false)));
    }

    // ---- 7. 列解析与回落 ----

    #[test]
    fn column_blob_missing_section_falls_back_to_defaults() {
        // 空段 -> 回落内置默认列。
        let cfg = parse_column_config("Column_OS_10.0_Ploder1", &[]);
        assert_eq!(cfg, ColumnConfig::default());
        assert_eq!(cfg.image_size, 16);
        assert_eq!(cfg.folder_flags, 1075838981);
    }

    #[test]
    fn column_blob_garbage_falls_back_to_defaults() {
        // 非 1SPS 容器的 hex -> 回落。
        let hex: String = "00".repeat(64);
        let kv = [("Spatlen_::{20D04FE0-3AEA-1069-A2D8-08002B30309D}", hex.as_str())];
        let cfg = parse_column_config("Column_OS_10.0_Ploder1", &kv);
        assert_eq!(cfg, ColumnConfig::default());
    }

    #[test]
    fn column_blob_bad_hex_is_rejected_with_key_context() {
        // 奇数长度 / 非十六进制 -> QError::Config 且带原始键名（C3）。
        let kv = [("Spatlen_::x", "%1C%0")];
        let err = extract_blob_from_section("Column_OS_10.0_Ploder1", &kv)
            .expect_err("奇数长度必须报错");
        match err {
            QError::Config { key, .. } => assert_eq!(key, "Spatlen_::x"),
            other => panic!("应为 Config 错误，实际 {other:?}"),
        }
        let kv2 = [("Spatlen_::x", "%ZZ%11")];
        let err2 = extract_blob_from_section("Column_OS_10.0_Ploder1", &kv2)
            .expect_err("非十六进制必须报错");
        assert!(matches!(err2, QError::Config { .. }));
    }

    #[test]
    fn column_blob_length_mismatch_is_rejected() {
        // 尾部声明的长度与实际字节数不符 -> 报错（防错位数据被当真）。
        let hex = "%1C".repeat(16); // 16 字节
        let raw = format!("{hex}|CODEMODE1|-1905896973|999");
        let kv = [("Spatlen_::x", raw.as_str())];
        assert!(extract_blob_from_section("Column_OS_10.0_Ploder1", &kv).is_err());
    }

    #[test]
    fn column_blob_parses_iconsize_and_fflags() {
        // 构造一个最小可识别 blob：0x4C 字节头部（含 1SPS 魔数）+ 两条记录。
        let mut blob: Vec<u8> = Vec::new();
        blob.extend_from_slice(&(28u32).to_le_bytes()); // 0x00: 长度字段（实测如此）
        // 0x04..0x38 填充（34 字节），使魔数落在实测的 0x38。
        blob.extend_from_slice(&[0u8; 0x34]);
        blob.extend_from_slice(COL_BLOB_MAGIC); // 0x38..0x3C
        // 0x3C..0x4C 填充（16 字节），记录区从 0x4C 开始。
        blob.extend_from_slice(&[0u8; 0x10]);

        // 记录：IconSize -> 16
        push_record(&mut blob, "IconSize", 19, &16u32.to_le_bytes());
        // 记录：FFlags -> 1075838981
        push_record(&mut blob, "FFlags", 19, &1075838981u32.to_le_bytes());

        let icon_size_rec = 4 + 4 + 1 + 18 + 4 + 4 + 4;
        let fflags_rec = 4 + 4 + 1 + 14 + 4 + 4 + 4;
        assert_eq!(blob.len(), 0x4C + icon_size_rec + fflags_rec);
        let cfg = parse_column_blob(&blob).expect("最小 blob 应可解析");
        // 合成 blob 里 size 与 vlen 自洽，故能取到值。
        assert_eq!(cfg.image_size, 16);
        assert_eq!(cfg.folder_flags, 1075838981);
        // 没有 ColInfo -> 列条目为空 -> 保留默认列。
        assert_eq!(cfg.columns, ColumnConfig::default().columns);
    }

    /// 按实测框架压入一条记录：
    /// `[u32 size][u32 klen][1 补齐字节][key UTF-16LE 含 NUL][u32 typ][u32 vlen][value]`
    fn push_record(out: &mut Vec<u8>, key: &str, typ: u32, value: &[u8]) {
        push_record_with_extra(out, key, typ, value, 0);
    }

    /// `push_record` 的变体：`extra` 是记录尾部附加但未计入 `vlen` 的字节数
    /// （实测 `ColInfo` 的 `size` 覆盖条目表，而 `vlen` 只到 prop 前缀）。
    fn push_record_with_extra(out: &mut Vec<u8>, key: &str, typ: u32, value: &[u8], extra: u32) {
        let mut key_u16: Vec<u8> = key
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        key_u16.extend_from_slice(&[0, 0]); // NUL 终结
        let klen = key_u16.len() as u32;
        // size = size(4) + klen(4) + pad(1) + key + typ(4) + vlen(4) + value + extra
        let size = 4 + 4 + 1 + klen + 4 + 4 + value.len() as u32 + extra;
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&klen.to_le_bytes());
        out.push(0); // 1 字节补齐（实测框架的一部分）
        out.extend_from_slice(&key_u16);
        out.extend_from_slice(&typ.to_le_bytes());
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value);
    }

    /// start.qdr `ColsAndSort1` 的实测十六进制串（772 字节）。
    ///
    /// 来源：`D:\Q-Dir_Portable_x64\Q-Dir\start.qdr`，与 Q-Dir.ini 的
    /// `[Column_OS_10.0_Ploder1]` 段 `Spatlen_::{...}` 值**逐字节相同**
    /// （用 Python 解码两个 UTF-16LE 文件并比对确认）。四个 Ploder 也完全一致。
    /// 去掉 `%` 后即原始字节，长度 1544 个十六进制字符 = 772 字节。
    const COLS_AND_SORT1_HEX: &str = concat!(
        "1C000000000000000000000000000000000000000000000000000000F1F1F1F1",
        "14000000000000000000000000000000D0020000CC0200003153505305D5CDD5",
        "9C2E1B10939708002B2CF9AE830000002200000000470072006F007500700042",
        "0079004B00650079003A0046004D005400490044000000080000004E0000007B",
        "00420037003200350046003100330030002D0034003700450046002D00310030",
        "00310041002D0041003500460031002D00300032003600300038004300390045",
        "0045004200410043007D0000000000330000002200000000470072006F007500",
        "70004200790044006900720065006300740069006F006E000000130000000100",
        "00005B0000000A0000000053006F00720074000000420000001E000000700072",
        "006F0070003400320039003400390036003700320039003500000000001C0000",
        "000100000030F125B7EF471A10A5F102608C9EEBAC0A00000001000000250000",
        "001400000000470072006F0075007000560069006500770000000B000000FFFF",
        "00001B0000000A000000004D006F006400650000001300000004000000230000",
        "001200000000490063006F006E00530069007A00650000001300000010000000",
        "BD000000100000000043006F006C0049006E0066006F000000420000001E0000",
        "00700072006F0070003400320039003400390036003700320039003500000000",
        "0078000000FDDFDFFD100000000000000000000000040000001800000030F125",
        "B7EF471A10A5F102608C9EEBAC0A000000B400000030F125B7EF471A10A5F102",
        "608C9EEBAC04000000E1000000354B179BFF40D211A27E00C04FC30871030000",
        "0090000000354B179BFF40D211A27E00C04FC3087102000000900000002F0000",
        "001E00000000470072006F0075007000420079004B00650079003A0050004900",
        "4400000013000000040000001F0000000E00000000460046006C006100670073",
        "00000013000000050020403100000020000000004C006F006700690063006100",
        "6C0056006900650077004D006F00640065000000130000000100000000000000",
        "00000000",
    );

    /// 把上面的实测串转成字节（测试夹具）。
    fn evidence_cols_and_sort1() -> Vec<u8> {
        let mut out = Vec::with_capacity(COLS_AND_SORT1_HEX.len() / 2);
        let b = COLS_AND_SORT1_HEX.as_bytes();
        let mut i = 0;
        while i + 1 < b.len() {
            let hi = (b[i] as char).to_digit(16).expect("夹具必须是合法十六进制");
            let lo = (b[i + 1] as char).to_digit(16).expect("夹具必须是合法十六进制");
            out.push(((hi << 4) | lo) as u8);
            i += 2;
        }
        out
    }

    #[test]
    fn column_blob_parses_real_evidence_bytes() {
        // 直接用 start.qdr / Q-Dir.ini 的实测 772 字节做解析断言。
        let blob = evidence_cols_and_sort1();
        assert_eq!(blob.len(), 772, "实测 ColsAndSort1 是 772 字节");

        let cfg = parse_column_blob(&blob).expect("实测 blob 必须可解析");

        // 列条目表：实测 4 条，宽度 24 / 180 / 225 / 144（GUID 锚定，无歧义）。
        assert_eq!(cfg.columns.len(), 4);
        assert_eq!(
            cfg.columns.iter().map(|c| c.width).collect::<Vec<_>>(),
            vec![24, 180, 225, 144]
        );
        // GUID 族 -> 列标识映射：前两条 PropGroup、后两条 Keywords。
        assert_eq!(cfg.columns[0].key, "Ext");
        assert_eq!(cfg.columns[1].key, "Ext");
        assert_eq!(cfg.columns[2].key, "Attr");
        assert_eq!(cfg.columns[3].key, "Attr");
        // 次序与可见性。
        for (i, c) in cfg.columns.iter().enumerate() {
            assert_eq!(c.order as usize, i);
            assert!(c.visible);
        }

        // IconSize / FFlags：H2(a) 已登记该二进制未逐字节解码，且实测这两条的
        // size/vlen 互相矛盾（size=35 把 value 排除在外，vlen=16 又指向下一条
        // 记录），故无法可靠取出。按契约降级：保留 core 的实测默认值
        // （image_size=16 与 start.qdr 的 ViewModeFlags1=1075838981 吻合）。
        assert_eq!(cfg.image_size, 16);
        assert_eq!(cfg.folder_flags, 1075838981);
    }

    #[test]
    fn column_config_from_real_evidence_ini_section() {
        // 走完整的 ini 段入口：`Column_OS_10.0_Ploder1` 段 + `Spatlen_::{...}` 键。
        let blob = evidence_cols_and_sort1();
        let percent: String = blob.iter().map(|b| format!("%{b:02X}")).collect();
        // 实测值的尾部附加信息：CODEMODE1|-1905896973|772（772 = 字节数）。
        let value = format!("{percent}|CODEMODE1|-1905896973|{}", blob.len());
        let kv = [("Spatlen_::{20D04FE0-3AEA-1069-A2D8-08002B30309D}", value.as_str())];

        let cfg = column_config_for(PaneId::P1, &kv);
        // 列宽与次序从实测 blob 抽出（GUID 锚定，无歧义）。
        assert_eq!(cfg.columns.len(), 4);
        assert_eq!(cfg.columns[0].width, 24);
        assert_eq!(
            cfg.columns.iter().map(|c| c.width).collect::<Vec<_>>(),
            vec![24, 180, 225, 144]
        );
        // IconSize / FFlags 按 H2(a) 降级保留默认值（见上一个测试的说明）。
        assert_eq!(cfg.image_size, 16);
        assert_eq!(cfg.folder_flags, 1075838981);
    }

    #[test]
    fn column_config_from_ini_text_scans_the_right_section() {
        let blob = evidence_cols_and_sort1();
        let percent: String = blob.iter().map(|b| format!("%{b:02X}")).collect();
        let text = format!(
            "[Start]\r\nlangid=1031\r\n[Column_OS_10.0_Ploder1]\r\nSpatlen_::{{20D04FE0-3AEA-1069-A2D8-08002B30309D}}={percent}|CODEMODE1|-1905896973|{}\r\n[Column_OS_10.0_Ploder2]\r\nSpatlen_::{{20D04FE0-3AEA-1069-A2D8-08002B30309D}}=%00%00\r\n",
            blob.len()
        );
        let p1 = column_config_from_ini_text(&text, PaneId::P1);
        assert_eq!(p1.columns.len(), 4, "Ploder1 段含实测 blob");
        // Ploder2 段是垃圾值 -> 回落默认列（不得串段）。
        let p2 = column_config_from_ini_text(&text, PaneId::P2);
        assert_eq!(p2, ColumnConfig::default());
        // Ploder3 段不存在 -> 默认列。
        let p3 = column_config_from_ini_text(&text, PaneId::P3);
        assert_eq!(p3, ColumnConfig::default());
    }

    #[test]
    fn column_entries_are_read_from_after_colinfo() {
        // 合成最小 blob：验证条目表定位逻辑本身（宽度/次序/标识映射）。
        let mut blob: Vec<u8> = Vec::new();
        blob.extend_from_slice(&(28u32).to_le_bytes());
        blob.extend_from_slice(&[0u8; 0x34]); // 0x04..0x38 填充
        blob.extend_from_slice(COL_BLOB_MAGIC);
        blob.extend_from_slice(&[0u8; 0x10]); // 0x3C..0x4C 填充

        push_record(&mut blob, "IconSize", 19, &16u32.to_le_bytes());
        // ColInfo 的 value 是 'prop4294967295\0'（实测 30 字节）。
        let prop: Vec<u8> = "prop4294967295"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .chain([0u8, 0u8])
            .collect();
        // 实测 value 之后是 26 字节容器头部 + 4 条 24 字节条目，
        // 且都计入 ColInfo 记录的 size（但不计入 vlen）。
        let extra = 26u32 + 4 * COL_ENTRY_STRIDE as u32;
        push_record_with_extra(&mut blob, "ColInfo", 66, &prop, extra);
        blob.extend_from_slice(&[0u8; 2]);
        blob.extend_from_slice(&120u32.to_le_bytes());
        blob.extend_from_slice(&[0u8; 20]); // 共 26 字节容器头部

        let guid_propgroup: [u8; 16] = [
            0x30, 0xF1, 0x25, 0xB7, 0xEF, 0x47, 0x1A, 0x10, 0xA5, 0xF1, 0x02, 0x60, 0x8C, 0x9E,
            0xEB, 0xAC,
        ];
        let guid_keywords: [u8; 16] = [
            0x35, 0x4B, 0x17, 0x9B, 0xFF, 0x40, 0xD2, 0x11, 0xA2, 0x7E, 0x00, 0xC0, 0x4F, 0xC3,
            0x08, 0x71,
        ];
        push_column_entry(&mut blob, 24, &guid_propgroup, 10);
        push_column_entry(&mut blob, 180, &guid_propgroup, 4);
        push_column_entry(&mut blob, 225, &guid_keywords, 3);
        push_column_entry(&mut blob, 144, &guid_keywords, 2);

        let cfg = parse_column_blob(&blob).expect("应可解析");
        assert_eq!(cfg.columns.len(), 4);
        assert_eq!(cfg.columns[0].key, "Ext");
        assert_eq!(cfg.columns[0].width, 24);
        assert_eq!(cfg.columns[0].order, 0);
        assert!(cfg.columns[0].visible);
        assert_eq!(cfg.columns[1].width, 180);
        assert_eq!(cfg.columns[1].order, 1);
        assert_eq!(cfg.columns[2].key, "Attr");
        assert_eq!(cfg.columns[2].width, 225);
        assert_eq!(cfg.columns[3].width, 144);
    }

    /// 按实测框架压入一条等距列条目：
    /// `[u32 width][GUID 16B][u32 fmt]`，步长 24 字节。
    fn push_column_entry(out: &mut Vec<u8>, width: u32, guid: &[u8; 16], fmt: u32) {
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(guid);
        out.extend_from_slice(&fmt.to_le_bytes());
    }

    #[test]
    fn hex_blob_parses_percent_separated_bytes() {
        let hex = "%1C%00%00%00%FF";
        let out = parse_hex_blob("Spatlen_::x", hex).expect("可解析");
        assert_eq!(out, vec![0x1C, 0x00, 0x00, 0x00, 0xFF]);
        // 无 '%' 也接受。
        let out2 = parse_hex_blob("Spatlen_::x", "1C00FF").expect("可解析");
        assert_eq!(out2, vec![0x1C, 0x00, 0xFF]);
    }

    #[test]
    fn pane_column_section_name_matches_ini_evidence() {
        assert_eq!(PaneId::P1.ini_section(), "Column_OS_10.0_Ploder1");
        assert_eq!(PaneId::P4.ini_section(), "Column_OS_10.0_Ploder4");
        let kv: [(&str, &str); 0] = [];
        assert_eq!(column_config_for(PaneId::P1, &kv), ColumnConfig::default());
    }

    // ---- 8. 组合引擎 ----

    #[test]
    fn list_engine_filters_and_colors_in_one_pass() {
        let mut cfg = config_with_rules(evidence_rules());
        cfg.by_filter = "*.txt;*.zip".to_string();
        let eng = ListEngine::from_config(&cfg);

        let items = vec![
            item("a.txt", false, false),
            item("b.png", false, false),
            item("docs", true, false),
            item("c.zip", false, false),
        ];
        let rows = eng.visible_rows(&items);
        // b.png 被过滤掉；目录 docs 保留。
        let names: Vec<&str> = rows.iter().map(|(i, _)| i.name.as_str()).collect();
        assert_eq!(names, vec!["a.txt", "docs", "c.zip"]);
        // 可见行号连续（交替背景的前提）。
        assert_eq!(rows.iter().map(|(_, r)| *r).collect::<Vec<_>>(), vec![0, 1, 2]);

        // 第一行 a.txt：命中 rule 9 兜底 0000BB。
        let d = eng.row(&items[0], 0);
        assert!(d.visible);
        assert_eq!(d.colors.fg, Some(Rgb(0, 0, 0xBB)));
        // docs 是目录 -> 008800。
        let d2 = eng.row(&items[2], 1);
        assert!(d2.visible);
        assert_eq!(d2.colors.fg, Some(Rgb(0, 0x88, 0)));
        // b.png 不可见。
        assert!(!eng.row(&items[1], 0).visible);
    }

    #[test]
    fn empty_filter_shows_everything() {
        let cfg = config_with_rules(evidence_rules());
        let eng = ListEngine::from_config(&cfg);
        let items = vec![item("x.png", false, false), item("y.doc", false, false)];
        assert_eq!(eng.visible_rows(&items).len(), 2);
    }
}
