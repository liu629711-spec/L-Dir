//! 配置持久化层：Q-Dir.ini 与 start.qdr 的读写。
//!
//! 关键约束（来自架构契约 E 组与逆向证据）：
//! - E1：两个文件都是 UTF-16LE，实测均带 FF FE BOM、CRLF 行分隔
//!   （`Q-Dir.ini` 20810 字节 / `start.qdr` 20936 字节，实测）。
//!   读写全部自研，零外部 ini 库，不假定 UTF-8。
//! - E2：INI 文档模型保留段顺序、段内键顺序、未知键原值；保存后重新解析，
//!   除本次修改的键外必须逐字节等价。
//! - E3：.qdr 的 FolderN 值形如 `<%XX%XX...IDLIST>&ViewModeFlagsN=1075838981
//!   &ViewModeN=4&UNICODE=1`，ViewModeN/ViewModeFlagsN 是 `&` 子字段而非顶层键。
//! - E4：ColsAndSortN（实测各 2342 字符）与 ColsIDN 原样保留在
//!   [`core::SessionState::cols_and_sort_raw`]，本模块不解析、不丢弃。
//! - E5：分割比例一律经 [`core::SplitPermille`] 换算（.qdr 万分数 <-> 内部千分比）。
//! - E6：Filter2 保留 index/field4/5/6 与通配符结尾分隔符状态。
//!
//! B2：所有 ini/.qdr 原始键名集中在本文件顶部的 `KEYS` 常量区，
//! Rust 字段名与 ini 键名的映射只在本文件出现一次。
//!
//! 依赖方向 D1：config 只依赖 core，不依赖任何其它业务模块。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::{
    ColumnConfig, Config, Lang, PaneId, PaneState, SessionState, SplitPermille, TreeMode, ViewMode,
    QError, QResult,
};

// ============ 原始键名集中区（B2） ============

// 下面三个模块是「原始键名表」。表中部分常量当前模块还用不到，但按 B2
// （ini/.qdr 原始键名一律集中在本常量区，禁止散落到业务模块）必须在这里登记齐，
// 供后续 pane/dirtree/favorites 等模块直接引用；因此显式 allow(dead_code)。
// 每个未用常量都注明它未来的消费者，避免成为无人认领的死代码。

/// Q-Dir.ini 段名。
/// 模块名故意用 SCREAMING_SNAKE：它们是「键名表」而非普通模块，与 B1
/// 「常量用 SCREAMING_SNAKE」一致；显式 allow 消去 non_snake_case 告警。
#[allow(non_snake_case, dead_code)]
mod INI_SECTIONS {
    pub const START: &str = "Start";
    pub const FAVORITEN: &str = "Favoriten";
    pub const Q: &str = "Q";
    pub const QDIR: &str = "Q-Dir";
    pub const FILTER2: &str = "Filter2";
    pub const PREVIEW: &str = "Preview";
    pub const ORDNER: &str = "Ordner";
}

/// Q-Dir.ini 键名（`<N>` 表示 Ploder 编号后缀 1..=4）。
#[allow(non_snake_case, dead_code)]
mod KEYS {
    // [Start]
    pub const QDIR_ID: &str = "QDir_Id";
    pub const USE_COLOR_START: &str = "useColorStart";
    pub const ALS: &str = "Als";
    pub const DESIGNE_MODE: &str = "designe_mode";
    pub const SHOW_CMD: &str = "showCmd";
    pub const START_CRASH: &str = "StartCrash";
    /// 开机恢复（自有键名 *待复核*；原版启动行为链未反编译确证）。
    pub const STARTUP: &str = "startup";
    pub const TRAY_MINIMIZE: &str = "tray_minimize";
    pub const SHOW_HIDDEN: &str = "show_hidden";
    pub const WIN_RC: &str = "WinRC";
    pub const DEFAULT_TAB: &str = "default_tab";
    pub const CHK_INI: &str = "chk_ini";
    // [Favoriten] / [Q]
    pub const ORDNER: &str = "Ordner";
    pub const LINK: &str = "Link";
    // [Q-Dir]
    pub const LIZENZ: &str = "Lizenz";
    // [Filter2] / [Preview] / [Ordner]
    pub const FILTER2: &str = "Filter2";
    pub const PREVIEW_FILTER: &str = "Filter";
    // 目录树
    pub const TREE_NO_EXPANDO: &str = "tree_no_expando";
    pub const TREE_SINGLE_EXPAND: &str = "tree_single_expand";
    pub const TREE_FULLROWSELECT: &str = "tree_fullrowselect";
    pub const TREE_EXPAND_WHEN_SELECT: &str = "tree_expand_when_select";
    pub const TREE_ALL_BROWSABLE: &str = "tree_all_browsable";
    pub const TREE_LINE_AT_ROOT: &str = "tree_line_at_root";
    pub const TREE_DISABLE_LINE: &str = "tree_disable_line";
    pub const TREE_AUTO_CHK_SUBDIR: &str = "tree_auto_chk_subdir";
    pub const TREE_INDENT_WIDTH: &str = "tree_indent_width";
    pub const TREE_AUTO_EXPAND_FROM_LIST: &str = "tree_auto_expand_from_list";
    pub const TREE_AUTO_SEL: &str = "tree_auto_sel";
    pub const TREE_HOVER_TIME: &str = "tree_hover_time";
    pub const TREE_MODE: &str = "tree_mode";
    // 标签页
    pub const TAB_NEW_TAB_L_DBLCLK: &str = "tab_new_tab_l_dblclk";
    pub const TAB_NEW_TAB_L_CLK: &str = "tab_new_tab_l_clk";
    pub const TAB_SHOW_TOOLTIPS: &str = "tab_show_tooltips";
    pub const TAB_CLOSE_BY_MBT_CLK: &str = "tab_close_by_mbt_clk";
    pub const TAB_CLOSE_BY_LBT_DBLCLK: &str = "tab_close_by_lbt_dblclk";
    pub const TAB_MAX_WIDTH: &str = "tab_max_width";
    pub const TAB_WIDTH_MODE: &str = "tab_width_mode";
    pub const TAB_USE_CONTEXT_MENU: &str = "tab_use_context_menu";
    pub const TABS_HOVER_TIME: &str = "tabs_hover_time";
    // 文件操作
    pub const COPY_CUT_PASTE_MODE: &str = "copy_cut_paste_mode";
    pub const MOVE_FILE_CONFIRMATION: &str = "move_file_confirmation";
    pub const REFRESH_KEEP_SEL: &str = "refresh_keep_sel";
    pub const REFRESH_KEEP_SCROLL: &str = "refresh_keep_scroll";
    pub const SELECT_LAST_IF_NAVIGATE_UP: &str = "select_last_if_navigate_up";
    // 其余
    pub const COLUMNS_PLUS_VIEW: &str = "columns_plus_view";
    pub const COLORTHEMA: &str = "colorthema";
    pub const LANGID: &str = "langid";
    pub const FONT_FACE_NAME: &str = "FontFaceName";
    pub const LANG_NAME: &str = "LangName";
    pub const SHOW_FILTER: &str = "show_filter";
    pub const LAST_FILTER: &str = "last_filter";
    pub const BY_DIR_FILTER: &str = "by_dir_filter";
    pub const BY_FILTER: &str = "by_filter";
    pub const MARKIERUNGSFILTER: &str = "Markierungsfilter";
    pub const LAST_FILE_FILTER: &str = "Last-File-Filter";
    pub const FAV_FILTER: &str = "Fav_Filter";

    /// 列配置段名前缀，如 `Column_OS_10.0_Ploder1`。
    pub fn column_section(n: u32) -> String {
        format!("Column_OS_10.0_Ploder{n}")
    }
}

/// start.qdr 键名（顶层）。
#[allow(non_snake_case, dead_code)]
mod QDR_KEYS {
    pub const VERSION: &str = "Q-Dir Version";
    pub const VIEW_MODE: &str = "ViewMode";
    pub const FOLDER: &str = "Folder"; // Folder1..Folder4
    pub const SP0: &str = "SP0";
    pub const SP1: &str = "SP1";
    pub const SP2: &str = "SP2";
    pub const SPT: &str = "SPT";
    pub const TREE_MODE: &str = "tree_mode";
    pub const TREE: &str = "Tree"; // Tree1..Tree4
    pub const TREE_SPLIT: &str = "Tree1split"; // Tree<N>split
    pub const VORSCHAU: &str = "Vorschau";
    pub const VORSCHAU_POS: &str = "VorschauPos";
    pub const FOCUS: &str = "Focus";
    pub const COLORTHEMA: &str = "colorthema";
    pub const COLS_AND_SORT: &str = "ColsAndSort"; // ColsAndSort1..4
    pub const COLS_ID: &str = "ColsID"; // ColsID1..4
    pub const WINDOW_PLACEMENT: &str = "windowplacement";

    /// FolderN 的 `&` 子字段名（E3：不是顶层键）。
    pub const SUB_VIEW_MODE_FLAGS: &str = "ViewModeFlags"; // ViewModeFlags<N>
    pub const SUB_VIEW_MODE: &str = "ViewMode"; // ViewMode<N>
    pub const SUB_UNICODE: &str = "UNICODE";
}

/// 二进制常量：UTF-16LE 字节序标记（FF FE）。
const UTF16_BOM_LE: [u8; 2] = [0xFF, 0xFE];

// ============ UTF-16LE 文本编解码（text_io） ============

/// 把 UTF-16LE 字节解码为 `String`。
///
/// 规则（与实测文件一致）：
/// - 允许 FF FE BOM，解码后剥离；
/// - 奇数长度返回 [`QError::Encoding`]；
/// - 未配对的代理项返回 [`QError::Encoding`]；
/// - CRLF / 单独 LF / 单独 CR 统一规整为 `\n`（调用方写回时再按 CRLF 输出）。
pub fn decode_utf16le(bytes: &[u8]) -> QResult<String> {
    if bytes.len() % 2 != 0 {
        return Err(QError::Encoding(format!(
            "UTF-16LE 数据长度为奇数：{} 字节",
            bytes.len()
        )));
    }
    let mut body = bytes;
    if body.len() >= 2 && body[0] == UTF16_BOM_LE[0] && body[1] == UTF16_BOM_LE[1] {
        body = &body[2..];
    }
    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let mut s = String::with_capacity(units.len());
    let mut i = 0usize;
    while i < units.len() {
        let u = units[i];
        if (0xD800..=0xDBFF).contains(&u) {
            // 高位代理：必须紧跟低位代理，否则拒绝。
            let lo = units.get(i + 1).copied().ok_or_else(|| {
                QError::Encoding("UTF-16LE 高位代理项后缺少低位代理项".to_string())
            })?;
            if !(0xDC00..=0xDFFF).contains(&lo) {
                return Err(QError::Encoding(format!(
                    "UTF-16LE 代理项错误：U+{u:04X} 后应为低位代理，实际 U+{lo:04X}"
                )));
            }
            let c = 0x1_0000
                + (((u as u32) - 0xD800) << 10)
                + ((lo as u32) - 0xDC00);
            s.push(char::from_u32(c).ok_or_else(|| {
                QError::Encoding(format!("UTF-16LE 代理项拼不出合法码点：U+{c:X}"))
            })?);
            i += 2;
        } else if (0xDC00..=0xDFFF).contains(&u) {
            return Err(QError::Encoding(format!(
                "UTF-16LE 出现游离低位代理项：U+{u:04X}"
            )));
        } else {
            // BMP 内：替换字符照常通过，其余码点必须合法。
            let c = u32::from(u);
            s.push(
                char::from_u32(c)
                    .ok_or_else(|| QError::Encoding(format!("非法 UTF-16 码元：U+{u:04X}")))?,
            );
            i += 1;
        }
    }
    Ok(normalize_newlines(&s))
}

/// 把 `String` 编码为 UTF-16LE 字节（带 BOM，CRLF 行分隔）。
pub fn encode_utf16le(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() * 2 + 2);
    out.extend_from_slice(&UTF16_BOM_LE);
    for u in text.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

/// CRLF / 单独 CR 统一为 `\n`。
fn normalize_newlines(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                out.push('\n');
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// 读文件并按 UTF-16LE 解码。文件不存在时返回 [`QError::Io`]（调用方决定是否用默认值）。
pub fn read_utf16le_file(path: &Path) -> QResult<String> {
    let bytes = fs::read(path).map_err(|e| QError::from_io(&e))?;
    decode_utf16le(&bytes)
}

/// 按 UTF-16LE（BOM + CRLF）写文件，先写临时文件再原子替换。
pub fn write_utf16le_file(path: &Path, text: &str) -> QResult<()> {
    let crlf = to_crlf(text);
    let bytes = encode_utf16le(&crlf);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| QError::from_io(&e))?;
        }
    }
    let tmp = tmp_path_for(path);
    fs::write(&tmp, &bytes).map_err(|e| QError::from_io(&e))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        QError::from_io(&e)
    })?;
    Ok(())
}

/// 生成同目录临时路径（`.qdir-tmp` 后缀），失败时退化为追加 `.tmp`。
fn tmp_path_for(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_string_lossy().to_string();
    s.push_str(".qdir-tmp");
    PathBuf::from(s)
}

/// `\n` -> CRLF。
fn to_crlf(text: &str) -> String {
    if !text.contains('\n') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + text.matches('\n').count());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            out.push('\r');
            out.push('\n');
        } else {
            out.push(c);
            if c == '\r' && chars.peek() == Some(&'\n') {
                out.push('\n');
                chars.next();
            }
        }
    }
    out
}

// ============ 有序 INI 文档模型（ini） ============

/// 有序 INI 文档。
///
/// 保留三件事（E2）：
/// 1. 段顺序（`section_order`）；
/// 2. 段内键顺序（`Section.entries` 保持插入序）；
/// 3. 未知键的原值字符串（本模型不解释值，只原样搬运）。
///
/// 段内出现空行时以 `Section.blank_lines` 记录数量，写回原位，
/// 使「本次没改的键」逐字节等价（start.qdr 的 FolderN 之间就有空行，实测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IniDocument {
    pub sections: BTreeMap<String, Section>,
    pub section_order: Vec<String>,
}

/// 一个段内的键值项。`key=None` 表示该段开头的一个空行组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: Option<String>,
    pub value: String,
}

/// 单个段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// 有序条目；`key=None` 表示空行。
    pub entries: Vec<Entry>,
}

impl Section {
    fn new() -> Self {
        Section { entries: Vec::new() }
    }

    /// 段的键值视图（跳过空行项）。
    pub fn kv(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().filter_map(|e| e.key.as_deref().map(|k| (k, e.value.as_str())))
    }

    /// 取键值（首次出现）。
    pub fn get(&self, key: &str) -> Option<&str> {
        self.kv().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// 取可编辑键值（首次出现）。
    pub fn get_mut(&mut self, key: &str) -> Option<&mut String> {
        self.entries
            .iter_mut()
            .find(|e| e.key.as_deref() == Some(key))
            .map(|e| &mut e.value)
    }

    /// 设置键值：存在则原地改（保持位置），不存在则追加到段尾。
    pub fn set(&mut self, key: &str, value: impl Into<String>) {
        let value = value.into();
        match self.get_mut(key) {
            Some(slot) => *slot = value,
            None => self.entries.push(Entry {
                key: Some(key.to_string()),
                value,
            }),
        }
    }

    /// 仅在键已存在时原地更新（E2：不得新增键，否则破坏「除本次修改的键外
    /// 逐字节等价」—— 实测 Q-Dir.ini 里大量可选键本就缺失，
    /// 无条件 set 会把它们全部追加进去）。
    ///
    /// 返回是否真的写入了不同值。键不存在时返回 `false` 且不做任何改动。
    pub fn set_in_place(&mut self, key: &str, value: impl Into<String>) -> bool {
        let value = value.into();
        match self.get_mut(key) {
            Some(slot) if *slot != value => {
                *slot = value;
                true
            }
            Some(_) => false,
            None => false,
        }
    }

    /// 删除键（保持其余顺序）。
    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|e| e.key.as_deref() != Some(key));
    }
}

impl IniDocument {
    /// 解析 ini 文本（已解码为 `String`，行分隔为 `\n`）。
    ///
    /// 语法（与实测文件一致）：
    /// - 段头 `[Name]`；段前的键归入空段名 ""（.qdr 即这种形式）；
    /// - `key=value`，首个 `=` 分隔；
    /// - 无 `=` 且非空的行按原样保留为键缺失项（避免静默丢数据）；
    /// - 空行保留为 `key=None` 项。
    pub fn parse(text: &str) -> Self {
        let mut doc = IniDocument {
            sections: BTreeMap::new(),
            section_order: Vec::new(),
        };
        let mut current = String::new();
        doc.sections.insert(String::new(), Section::new());
        // 用 `lines()` 而非 `split('\n')`：后者会把结尾换行变成一个幻影空行，
        // 破坏往返逐字节等价（实测两个证据文件都以 CRLF 结尾）。
        for raw_line in text.lines() {
            let line = raw_line;
            if line.is_empty() {
                doc.section_mut(&current).entries.push(Entry {
                    key: None,
                    value: String::new(),
                });
                continue;
            }
            if let Some(rest) = line.strip_prefix('[') {
                if let Some(name) = rest.strip_suffix(']') {
                    current = name.to_string();
                    doc.ensure_section(&current);
                    continue;
                }
            }
            let entry = match line.split_once('=') {
                Some((k, v)) => Entry {
                    key: Some(k.to_string()),
                    value: v.to_string(),
                },
                None => Entry {
                    key: None,
                    value: line.to_string(),
                },
            };
            doc.section_mut(&current).entries.push(entry);
        }
        doc
    }

    fn ensure_section(&mut self, name: &str) {
        if !self.sections.contains_key(name) {
            self.sections.insert(name.to_string(), Section::new());
            self.section_order.push(name.to_string());
        }
    }

    /// 段的可变引用（不存在则创建，段顺序不变）。
    pub fn section_mut(&mut self, name: &str) -> &mut Section {
        if !self.sections.contains_key(name) {
            self.sections.insert(name.to_string(), Section::new());
            self.section_order.push(name.to_string());
        }
        self.sections.get_mut(name).expect("段刚被插入")
    }

    /// 段的只读引用。
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.get(name)
    }

    /// 序列化回 ini 文本（`\n` 行分隔；写文件时再转 CRLF）。
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        // 段顺序：空段名（顶层键）永远在最前，其后按 section_order。
        // 空段名字符串必须先绑定到局部变量，避免临时值生命周期错误。
        let empty = String::new();
        let names: Vec<&String> = std::iter::once(&empty)
            .chain(self.section_order.iter().filter(|s| !s.is_empty()))
            .collect();
        for name in names {
            let Some(sec) = self.sections.get(name) else {
                continue;
            };
            if !name.is_empty() {
                out.push('[');
                out.push_str(name);
                out.push_str("]\n");
            }
            for e in &sec.entries {
                match &e.key {
                    Some(k) => {
                        out.push_str(k);
                        out.push('=');
                        out.push_str(&e.value);
                    }
                    None => out.push_str(&e.value),
                }
                out.push('\n');
            }
        }
        out
    }

    /// 便捷读取：`section.get(key)` 的字符串形式。
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.section(section).and_then(|s| s.get(key))
    }

    /// 便捷写入。
    pub fn set(&mut self, section: &str, key: &str, value: impl Into<String>) {
        self.section_mut(section).set(key, value);
    }

    /// 便捷的「仅在键已存在时原地更新」写入（E2：不新增键）。
    pub fn set_in_place(&mut self, section: &str, key: &str, value: impl Into<String>) -> bool {
        self.section_mut(section).set_in_place(key, value)
    }
}

// ============ 值解析小工具 ============

/// 解析 bool（ini 用 0/1；也接受 true/false 以容忍手工编辑）。
fn parse_bool(key: &str, raw: &str) -> QResult<bool> {
    match raw.trim() {
        "0" | "false" | "FALSE" => Ok(false),
        "1" | "true" | "TRUE" => Ok(true),
        other => Err(QError::config(key, format!("应为 0/1，实际 {other:?}"))),
    }
}

/// 解析 u32。
fn parse_u32(key: &str, raw: &str) -> QResult<u32> {
    raw.trim()
        .parse()
        .map_err(|_| QError::config(key, format!("应为无符号整数，实际 {raw:?}")))
}

/// 解析 i32。
fn parse_i32(key: &str, raw: &str) -> QResult<i32> {
    raw.trim()
        .parse()
        .map_err(|_| QError::config(key, format!("应为整数，实际 {raw:?}")))
}

/// 取 u32；缺失时用默认值（任务要求：路径/键不存在时返回默认值而不是 panic）。
fn get_u32(doc: &IniDocument, section: &str, key: &str, default: u32) -> QResult<u32> {
    match doc.get(section, key) {
        Some(raw) => parse_u32(key, raw),
        None => Ok(default),
    }
}

/// 取 bool；缺失时用默认值。
fn get_bool(doc: &IniDocument, section: &str, key: &str, default: bool) -> QResult<bool> {
    match doc.get(section, key) {
        Some(raw) => parse_bool(key, raw),
        None => Ok(default),
    }
}

/// 取 String；缺失时用默认值。
fn get_string(doc: &IniDocument, section: &str, key: &str, default: &str) -> String {
    doc.get(section, key).unwrap_or(default).to_string()
}

/// 取 PathBuf；缺失时用默认值。
fn get_path(doc: &IniDocument, section: &str, key: &str, default: &str) -> PathBuf {
    PathBuf::from(get_string(doc, section, key, default))
}

// ============ 配置加载/保存（Q-Dir.ini） ============

/// 从 UTF-16LE 字节加载 [`Config`]。
///
/// 缺失键一律回落到 [`Config::default`]（不 panic）；格式错误的键报
/// [`QError::Config`] 并带原始键名。
pub fn load_config_bytes(bytes: &[u8]) -> QResult<Config> {
    let text = decode_utf16le(bytes)?;
    let doc = IniDocument::parse(&text);
    let mut cfg = Config::default();

    // ---- [Start] ----
    cfg.qdir_id = get_u32(&doc, INI_SECTIONS::START, KEYS::QDIR_ID, cfg.qdir_id)?;
    cfg.use_color_start =
        get_bool(&doc, INI_SECTIONS::START, KEYS::USE_COLOR_START, cfg.use_color_start)?;
    cfg.als = get_u32(&doc, INI_SECTIONS::START, KEYS::ALS, cfg.als)?;
    cfg.designe_mode = get_u32(&doc, INI_SECTIONS::START, KEYS::DESIGNE_MODE, cfg.designe_mode)?;
    cfg.show_cmd = get_u32(&doc, INI_SECTIONS::START, KEYS::SHOW_CMD, cfg.show_cmd)?;
    cfg.start_crash =
        get_bool(&doc, INI_SECTIONS::START, KEYS::START_CRASH, cfg.start_crash)?;
    cfg.startup = get_bool(&doc, INI_SECTIONS::START, KEYS::STARTUP, cfg.startup)?;
    cfg.tray_minimize = get_bool(&doc, INI_SECTIONS::START, KEYS::TRAY_MINIMIZE, cfg.tray_minimize)?;
    cfg.show_hidden = get_bool(&doc, INI_SECTIONS::START, KEYS::SHOW_HIDDEN, cfg.show_hidden)?;
    if let Some(raw) = doc.get(INI_SECTIONS::START, KEYS::WIN_RC) {
        cfg.win_rc = parse_win_rc(KEYS::WIN_RC, raw)?;
    }

    // ---- [Favoriten] / [Q] ----
    cfg.favoriten_dir = get_path(&doc, INI_SECTIONS::FAVORITEN, KEYS::ORDNER, ".");
    cfg.quick_link_dir = get_path(&doc, INI_SECTIONS::Q, KEYS::LINK, ".");

    // ---- [Preview] Filter / [Ordner] Filter ----
    cfg.preview_filter = get_string(&doc, INI_SECTIONS::PREVIEW, KEYS::PREVIEW_FILTER, "*;");

    // ---- [Filter2] ----
    if let Some(sec) = doc.section(INI_SECTIONS::FILTER2) {
        let lines: Vec<String> = sec.kv().map(|(k, v)| format!("{k}={v}")).collect();
        if !lines.is_empty() {
            cfg.rules = crate::core::RuleSet::parse_ini_section(&lines)?;
        }
    }

    // ---- 目录树 ----
    {
        let t = &mut cfg.tree;
        t.no_expando = get_bool(&doc, INI_SECTIONS::START, KEYS::TREE_NO_EXPANDO, t.no_expando)?;
        t.single_expand =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TREE_SINGLE_EXPAND, t.single_expand)?;
        t.fullrowselect =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TREE_FULLROWSELECT, t.fullrowselect)?;
        t.expand_when_select = get_bool(
            &doc,
            INI_SECTIONS::START,
            KEYS::TREE_EXPAND_WHEN_SELECT,
            t.expand_when_select,
        )?;
        t.all_browsable =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TREE_ALL_BROWSABLE, t.all_browsable)?;
        t.line_at_root =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TREE_LINE_AT_ROOT, t.line_at_root)?;
        t.disable_line =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TREE_DISABLE_LINE, t.disable_line)?;
        t.auto_chk_subdir = get_bool(
            &doc,
            INI_SECTIONS::START,
            KEYS::TREE_AUTO_CHK_SUBDIR,
            t.auto_chk_subdir,
        )?;
        t.indent_width =
            get_u32(&doc, INI_SECTIONS::START, KEYS::TREE_INDENT_WIDTH, t.indent_width)?;
        t.auto_expand_from_list = get_bool(
            &doc,
            INI_SECTIONS::START,
            KEYS::TREE_AUTO_EXPAND_FROM_LIST,
            t.auto_expand_from_list,
        )?;
        t.auto_sel = get_bool(&doc, INI_SECTIONS::START, KEYS::TREE_AUTO_SEL, t.auto_sel)?;
        t.hover_time =
            get_u32(&doc, INI_SECTIONS::START, KEYS::TREE_HOVER_TIME, t.hover_time)?;
        t.mode = TreeMode::from_u32(get_u32(&doc, INI_SECTIONS::START, KEYS::TREE_MODE, t.mode.to_u32())?);
    }

    // ---- 标签页 ----
    {
        let t = &mut cfg.tab;
        t.new_tab_l_dblclk =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TAB_NEW_TAB_L_DBLCLK, t.new_tab_l_dblclk)?;
        t.new_tab_l_clk =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TAB_NEW_TAB_L_CLK, t.new_tab_l_clk)?;
        t.show_tooltips =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TAB_SHOW_TOOLTIPS, t.show_tooltips)?;
        t.close_by_mbt_clk =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TAB_CLOSE_BY_MBT_CLK, t.close_by_mbt_clk)?;
        t.close_by_lbt_dblclk = get_bool(
            &doc,
            INI_SECTIONS::START,
            KEYS::TAB_CLOSE_BY_LBT_DBLCLK,
            t.close_by_lbt_dblclk,
        )?;
        t.max_width = get_u32(&doc, INI_SECTIONS::START, KEYS::TAB_MAX_WIDTH, t.max_width)?;
        t.width_mode = get_u32(&doc, INI_SECTIONS::START, KEYS::TAB_WIDTH_MODE, t.width_mode)?;
        t.use_context_menu =
            get_bool(&doc, INI_SECTIONS::START, KEYS::TAB_USE_CONTEXT_MENU, t.use_context_menu)?;
        t.hover_time = get_u32(&doc, INI_SECTIONS::START, KEYS::TABS_HOVER_TIME, t.hover_time)?;
    }

    // ---- 文件操作 ----
    {
        let f = &mut cfg.file_ops;
        f.copy_cut_paste_mode = get_u32(
            &doc,
            INI_SECTIONS::START,
            KEYS::COPY_CUT_PASTE_MODE,
            f.copy_cut_paste_mode,
        )?;
        f.move_file_confirmation = get_bool(
            &doc,
            INI_SECTIONS::START,
            KEYS::MOVE_FILE_CONFIRMATION,
            f.move_file_confirmation,
        )?;
        f.refresh_keep_sel =
            get_bool(&doc, INI_SECTIONS::START, KEYS::REFRESH_KEEP_SEL, f.refresh_keep_sel)?;
        f.refresh_keep_scroll = get_bool(
            &doc,
            INI_SECTIONS::START,
            KEYS::REFRESH_KEEP_SCROLL,
            f.refresh_keep_scroll,
        )?;
        f.select_last_if_navigate_up = get_bool(
            &doc,
            INI_SECTIONS::START,
            KEYS::SELECT_LAST_IF_NAVIGATE_UP,
            f.select_last_if_navigate_up,
        )?;
    }

    // ---- 其余键（均从 [Start] 读，原版即如此；键名证据来自 exe UTF-16 串） ----
    cfg.columns_plus_view =
        get_bool(&doc, INI_SECTIONS::START, KEYS::COLUMNS_PLUS_VIEW, cfg.columns_plus_view)?;
    cfg.colorthema = get_u32(&doc, INI_SECTIONS::START, KEYS::COLORTHEMA, cfg.colorthema)?;
    cfg.langid = get_u32(&doc, INI_SECTIONS::START, KEYS::LANGID, cfg.langid)?;
    cfg.font_face_name =
        get_string(&doc, INI_SECTIONS::START, KEYS::FONT_FACE_NAME, &cfg.font_face_name);
    cfg.lang_name = get_string(&doc, INI_SECTIONS::START, KEYS::LANG_NAME, &cfg.lang_name);
    cfg.show_filter =
        get_bool(&doc, INI_SECTIONS::START, KEYS::SHOW_FILTER, cfg.show_filter)?;
    cfg.last_filter =
        get_string(&doc, INI_SECTIONS::START, KEYS::LAST_FILTER, &cfg.last_filter);
    cfg.by_dir_filter =
        get_string(&doc, INI_SECTIONS::START, KEYS::BY_DIR_FILTER, &cfg.by_dir_filter);
    cfg.by_filter = get_string(&doc, INI_SECTIONS::START, KEYS::BY_FILTER, &cfg.by_filter);
    cfg.markierungsfilter =
        get_string(&doc, INI_SECTIONS::START, KEYS::MARKIERUNGSFILTER, &cfg.markierungsfilter);
    cfg.last_file_filter =
        get_string(&doc, INI_SECTIONS::START, KEYS::LAST_FILE_FILTER, &cfg.last_file_filter);
    cfg.fav_filter = get_string(&doc, INI_SECTIONS::START, KEYS::FAV_FILTER, &cfg.fav_filter);

    Ok(cfg)
}

/// 解析 `WinRC=0;0;800;600`。
fn parse_win_rc(key: &str, raw: &str) -> QResult<[i32; 4]> {
    let parts: Vec<&str> = raw.split(';').collect();
    if parts.len() != 4 {
        return Err(QError::config(key, format!("应有 4 个分号字段，实际 {}", parts.len())));
    }
    let mut out = [0i32; 4];
    for (slot, p) in out.iter_mut().zip(parts) {
        *slot = parse_i32(key, p)?;
    }
    Ok(out)
}

/// 把 [`Config`] 写回一个已解析的 INI 文档（保持段/键顺序与未知键）。
pub fn apply_config_to_doc(doc: &mut IniDocument, cfg: &Config) {
    let start = INI_SECTIONS::START;
    // 全部走 set_in_place：只在键已存在时更新（E2）。
    //
    // 对「Config 建模了但原文件里没有」的键（如 langid / colorthema / tree_* / tab_*），
    // set_in_place 会静默跳过，导致用户在设置里改了却存不下来。
    // 因此在逐项 in-place 更新之后，再用 ensure_keys 把这些建模键补写进 [Start]
    // —— 这属于「本次主动修改的键」，不违反 E2（E2 只要求未修改的键逐字节等价）。
    doc.set_in_place(start, KEYS::QDIR_ID, cfg.qdir_id.to_string());
    doc.set_in_place(start, KEYS::USE_COLOR_START, bool_u8(cfg.use_color_start).to_string());
    doc.set_in_place(start, KEYS::ALS, cfg.als.to_string());
    doc.set_in_place(start, KEYS::DESIGNE_MODE, cfg.designe_mode.to_string());
    doc.set_in_place(start, KEYS::SHOW_CMD, cfg.show_cmd.to_string());
    doc.set_in_place(start, KEYS::START_CRASH, bool_u8(cfg.start_crash).to_string());
    doc.set_in_place(start, KEYS::STARTUP, bool_u8(cfg.startup).to_string());
    doc.set_in_place(start, KEYS::TRAY_MINIMIZE, bool_u8(cfg.tray_minimize).to_string());
    doc.set_in_place(start, KEYS::SHOW_HIDDEN, bool_u8(cfg.show_hidden).to_string());
    doc.set_in_place(
        start,
        KEYS::WIN_RC,
        cfg.win_rc
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(";"),
    );

    doc.set_in_place(INI_SECTIONS::FAVORITEN, KEYS::ORDNER, cfg.favoriten_dir.to_string_lossy());
    doc.set_in_place(INI_SECTIONS::Q, KEYS::LINK, cfg.quick_link_dir.to_string_lossy());
    doc.set_in_place(INI_SECTIONS::PREVIEW, KEYS::PREVIEW_FILTER, cfg.preview_filter.clone());

    // [Filter2]：逐条按 index 就地更新（E2：保留原值文本与顺序）。
    //
    // 为什么不能整段 clear 后重写：rule 4 的原始串在串内混用 ';' 与 ','
    // （`*.jpg;*.jpeg;*.png,*.gif;*.bmp;*.ico`），而 RulePattern::Wildcards
    // 只存拆分后的 pats（契约形状，A1 禁止加字段存原始分隔符序列）。
    // 因此「未修改的规则」必须原样保留原值文本，只有确实变化的行才用
    // to_ini_line 重写（此时 ',' 会统一成 ';'，语义等价，已登记差异）。
    {
        let sec = doc.section_mut(INI_SECTIONS::FILTER2);
        // 已存在的行：按 index 匹配，**仅在规则确实变化时**才重写。
        //
        // 关键点：「变化」必须按语义判断，不能按文本判断：rule 4 的原始串在串内
        // 混用 ';' 与 ','，而 RulePattern::Wildcards 只存拆分后的 pats
        // （契约形状，A1 禁止加字段存原始分隔符序列），所以未修改的 rule 4
        // 序列化后文本也不同。若按文本比较就会把用户的原文改写掉（E2 违反）。
        // 因此这里把「新序列化行」重新解析回 FilterRule，与文档当前行的解析结果
        // 比较：语义相同 => 原样保留原文；语义不同 => 才写回新行。
        for entry in sec.entries.iter_mut() {
            let Some(key) = entry.key.as_deref() else {
                continue;
            };
            let Ok(idx) = key.trim().parse::<usize>() else {
                continue; // 非法索引行原样保留
            };
            let Some(rule) = cfg.rules.rules.iter().find(|r| r.index == idx) else {
                continue; // 该 index 已不在规则集中：保留原文（保守，不删用户数据）
            };
            // 注意：parse_line 内部会自己补 `index=` 前缀，
            // 因此这里只传 entry.value（`=` 之后的部分），不要重复拼 key。
            let old_rule = crate::core::FilterRule::parse_line(idx, &entry.value);
            if old_rule.as_ref() == Ok(rule) {
                continue; // 语义未变：保留原文（rule 4 的 ',' 混排由此保留）
            }
            let line = rule.to_ini_line();
            let new_val = line
                .split_once('=')
                .map(|(_, v)| v)
                .expect("Filter2 行必含 '='");
            entry.value = new_val.to_string();
        }
        // 规则集中存在但文档里没有的新 index：追加到段尾。
        let existing: Vec<usize> = sec
            .entries
            .iter()
            .filter_map(|e| e.key.as_deref().and_then(|k| k.trim().parse::<usize>().ok()))
            .collect();
        let mut new_rules: Vec<&crate::core::FilterRule> = cfg
            .rules
            .rules
            .iter()
            .filter(|r| !existing.contains(&r.index))
            .collect();
        new_rules.sort_by_key(|r| r.index);
        for rule in new_rules {
            let line = rule.to_ini_line();
            let (k, v) = line.split_once('=').expect("Filter2 行必含 '='");
            sec.entries.push(Entry {
                key: Some(k.to_string()),
                value: v.to_string(),
            });
        }
    }

    // 目录树 / 标签页 / 文件操作 / 其余键。
    doc.set_in_place(start, KEYS::TREE_NO_EXPANDO, bool_u8(cfg.tree.no_expando).to_string());
    doc.set_in_place(start, KEYS::TREE_SINGLE_EXPAND, bool_u8(cfg.tree.single_expand).to_string());
    doc.set_in_place(start, KEYS::TREE_FULLROWSELECT, bool_u8(cfg.tree.fullrowselect).to_string());
    doc.set_in_place(start, KEYS::TREE_EXPAND_WHEN_SELECT, bool_u8(cfg.tree.expand_when_select).to_string());
    doc.set_in_place(start, KEYS::TREE_ALL_BROWSABLE, bool_u8(cfg.tree.all_browsable).to_string());
    doc.set_in_place(start, KEYS::TREE_LINE_AT_ROOT, bool_u8(cfg.tree.line_at_root).to_string());
    doc.set_in_place(start, KEYS::TREE_DISABLE_LINE, bool_u8(cfg.tree.disable_line).to_string());
    doc.set_in_place(start, KEYS::TREE_AUTO_CHK_SUBDIR, bool_u8(cfg.tree.auto_chk_subdir).to_string());
    doc.set_in_place(start, KEYS::TREE_INDENT_WIDTH, cfg.tree.indent_width.to_string());
    doc.set_in_place(start, KEYS::TREE_AUTO_EXPAND_FROM_LIST, bool_u8(cfg.tree.auto_expand_from_list).to_string());
    doc.set_in_place(start, KEYS::TREE_AUTO_SEL, bool_u8(cfg.tree.auto_sel).to_string());
    doc.set_in_place(start, KEYS::TREE_HOVER_TIME, cfg.tree.hover_time.to_string());
    doc.set_in_place(start, KEYS::TREE_MODE, cfg.tree.mode.to_u32().to_string());

    doc.set_in_place(start, KEYS::TAB_NEW_TAB_L_DBLCLK, bool_u8(cfg.tab.new_tab_l_dblclk).to_string());
    doc.set_in_place(start, KEYS::TAB_NEW_TAB_L_CLK, bool_u8(cfg.tab.new_tab_l_clk).to_string());
    doc.set_in_place(start, KEYS::TAB_SHOW_TOOLTIPS, bool_u8(cfg.tab.show_tooltips).to_string());
    doc.set_in_place(start, KEYS::TAB_CLOSE_BY_MBT_CLK, bool_u8(cfg.tab.close_by_mbt_clk).to_string());
    doc.set_in_place(start, KEYS::TAB_CLOSE_BY_LBT_DBLCLK, bool_u8(cfg.tab.close_by_lbt_dblclk).to_string());
    doc.set_in_place(start, KEYS::TAB_MAX_WIDTH, cfg.tab.max_width.to_string());
    doc.set_in_place(start, KEYS::TAB_WIDTH_MODE, cfg.tab.width_mode.to_string());
    doc.set_in_place(start, KEYS::TAB_USE_CONTEXT_MENU, bool_u8(cfg.tab.use_context_menu).to_string());
    doc.set_in_place(start, KEYS::TABS_HOVER_TIME, cfg.tab.hover_time.to_string());

    doc.set_in_place(start, KEYS::COPY_CUT_PASTE_MODE, cfg.file_ops.copy_cut_paste_mode.to_string());
    doc.set_in_place(start, KEYS::MOVE_FILE_CONFIRMATION, bool_u8(cfg.file_ops.move_file_confirmation).to_string());
    doc.set_in_place(start, KEYS::REFRESH_KEEP_SEL, bool_u8(cfg.file_ops.refresh_keep_sel).to_string());
    doc.set_in_place(start, KEYS::REFRESH_KEEP_SCROLL, bool_u8(cfg.file_ops.refresh_keep_scroll).to_string());
    doc.set_in_place(
        start,
        KEYS::SELECT_LAST_IF_NAVIGATE_UP,
        bool_u8(cfg.file_ops.select_last_if_navigate_up).to_string(),
    );

    doc.set_in_place(start, KEYS::COLUMNS_PLUS_VIEW, bool_u8(cfg.columns_plus_view).to_string());
    doc.set_in_place(start, KEYS::COLORTHEMA, cfg.colorthema.to_string());
    doc.set_in_place(start, KEYS::LANGID, cfg.langid.to_string());
    doc.set_in_place(start, KEYS::FONT_FACE_NAME, cfg.font_face_name.clone());
    doc.set_in_place(start, KEYS::LANG_NAME, cfg.lang_name.clone());
    doc.set_in_place(start, KEYS::SHOW_FILTER, bool_u8(cfg.show_filter).to_string());
    doc.set_in_place(start, KEYS::LAST_FILTER, cfg.last_filter.clone());
    doc.set_in_place(start, KEYS::BY_DIR_FILTER, cfg.by_dir_filter.clone());
    doc.set_in_place(start, KEYS::BY_FILTER, cfg.by_filter.clone());
    doc.set_in_place(start, KEYS::MARKIERUNGSFILTER, cfg.markierungsfilter.clone());
    doc.set_in_place(start, KEYS::LAST_FILE_FILTER, cfg.last_file_filter.clone());
    doc.set_in_place(start, KEYS::FAV_FILTER, cfg.fav_filter.clone());

    // 补写「Config 建模了但文档里缺失」的键（见函数头说明）。
    // 只在缺失时追加，且只追加 Config 显式建模的键 —— 不会引入文档本来没有的
    // 未知键，因此真实 ini 的未建模键（chk_ini / default_tab / Lizenz 等）不受影响。
    ensure_modeled_keys(doc, cfg);
}

/// 把 [`Config`] 建模但文档中缺失、且**用户确实改过**的键补写进文档。
///
/// 为什么需要：`apply_config_to_doc` 全程用 `set_in_place`（E2 不新增键），
/// 于是用户在设置里改了 `langid` 这类「实测 ini 里本就没有」的键时存不下来。
///
/// 为什么只补「非默认值」：E2 要求未修改的键逐字节等价。若把所有建模键无条件
/// 补写，一个只读不改的用户文件会被塞进 40+ 行默认键（实测：+1690 字节）。
/// 因此判定标准是「值 != Config::default()」—— 只有用户主动改过的键才会
/// 出现在本来没有它的文件里。这既不丢用户修改，也不污染未改动的文件。
fn ensure_modeled_keys(doc: &mut IniDocument, cfg: &Config) {
    let d = Config::default();
    let start = INI_SECTIONS::START;
    let mut pairs: Vec<(&str, String)> = Vec::new();
    // 只收集「与默认值不同」的键。
    if cfg.startup != d.startup {
        pairs.push((KEYS::STARTUP, bool_u8(cfg.startup).to_string()));
    }
    if cfg.tray_minimize != d.tray_minimize {
        pairs.push((KEYS::TRAY_MINIMIZE, bool_u8(cfg.tray_minimize).to_string()));
    }
    if cfg.show_hidden != d.show_hidden {
        pairs.push((KEYS::SHOW_HIDDEN, bool_u8(cfg.show_hidden).to_string()));
    }
    if cfg.tree.no_expando != d.tree.no_expando {
        pairs.push((KEYS::TREE_NO_EXPANDO, bool_u8(cfg.tree.no_expando).to_string()));
    }
    if cfg.tree.single_expand != d.tree.single_expand {
        pairs.push((KEYS::TREE_SINGLE_EXPAND, bool_u8(cfg.tree.single_expand).to_string()));
    }
    if cfg.tree.fullrowselect != d.tree.fullrowselect {
        pairs.push((KEYS::TREE_FULLROWSELECT, bool_u8(cfg.tree.fullrowselect).to_string()));
    }
    if cfg.tree.expand_when_select != d.tree.expand_when_select {
        pairs.push((KEYS::TREE_EXPAND_WHEN_SELECT, bool_u8(cfg.tree.expand_when_select).to_string()));
    }
    if cfg.tree.all_browsable != d.tree.all_browsable {
        pairs.push((KEYS::TREE_ALL_BROWSABLE, bool_u8(cfg.tree.all_browsable).to_string()));
    }
    if cfg.tree.line_at_root != d.tree.line_at_root {
        pairs.push((KEYS::TREE_LINE_AT_ROOT, bool_u8(cfg.tree.line_at_root).to_string()));
    }
    if cfg.tree.disable_line != d.tree.disable_line {
        pairs.push((KEYS::TREE_DISABLE_LINE, bool_u8(cfg.tree.disable_line).to_string()));
    }
    if cfg.tree.auto_chk_subdir != d.tree.auto_chk_subdir {
        pairs.push((KEYS::TREE_AUTO_CHK_SUBDIR, bool_u8(cfg.tree.auto_chk_subdir).to_string()));
    }
    if cfg.tree.indent_width != d.tree.indent_width {
        pairs.push((KEYS::TREE_INDENT_WIDTH, cfg.tree.indent_width.to_string()));
    }
    if cfg.tree.auto_expand_from_list != d.tree.auto_expand_from_list {
        pairs.push((KEYS::TREE_AUTO_EXPAND_FROM_LIST, bool_u8(cfg.tree.auto_expand_from_list).to_string()));
    }
    if cfg.tree.auto_sel != d.tree.auto_sel {
        pairs.push((KEYS::TREE_AUTO_SEL, bool_u8(cfg.tree.auto_sel).to_string()));
    }
    if cfg.tree.hover_time != d.tree.hover_time {
        pairs.push((KEYS::TREE_HOVER_TIME, cfg.tree.hover_time.to_string()));
    }
    if cfg.tree.mode != d.tree.mode {
        pairs.push((KEYS::TREE_MODE, cfg.tree.mode.to_u32().to_string()));
    }
    if cfg.tab.new_tab_l_dblclk != d.tab.new_tab_l_dblclk {
        pairs.push((KEYS::TAB_NEW_TAB_L_DBLCLK, bool_u8(cfg.tab.new_tab_l_dblclk).to_string()));
    }
    if cfg.tab.new_tab_l_clk != d.tab.new_tab_l_clk {
        pairs.push((KEYS::TAB_NEW_TAB_L_CLK, bool_u8(cfg.tab.new_tab_l_clk).to_string()));
    }
    if cfg.tab.show_tooltips != d.tab.show_tooltips {
        pairs.push((KEYS::TAB_SHOW_TOOLTIPS, bool_u8(cfg.tab.show_tooltips).to_string()));
    }
    if cfg.tab.close_by_mbt_clk != d.tab.close_by_mbt_clk {
        pairs.push((KEYS::TAB_CLOSE_BY_MBT_CLK, bool_u8(cfg.tab.close_by_mbt_clk).to_string()));
    }
    if cfg.tab.close_by_lbt_dblclk != d.tab.close_by_lbt_dblclk {
        pairs.push((KEYS::TAB_CLOSE_BY_LBT_DBLCLK, bool_u8(cfg.tab.close_by_lbt_dblclk).to_string()));
    }
    if cfg.tab.max_width != d.tab.max_width {
        pairs.push((KEYS::TAB_MAX_WIDTH, cfg.tab.max_width.to_string()));
    }
    if cfg.tab.width_mode != d.tab.width_mode {
        pairs.push((KEYS::TAB_WIDTH_MODE, cfg.tab.width_mode.to_string()));
    }
    if cfg.tab.use_context_menu != d.tab.use_context_menu {
        pairs.push((KEYS::TAB_USE_CONTEXT_MENU, bool_u8(cfg.tab.use_context_menu).to_string()));
    }
    if cfg.tab.hover_time != d.tab.hover_time {
        pairs.push((KEYS::TABS_HOVER_TIME, cfg.tab.hover_time.to_string()));
    }
    if cfg.file_ops.copy_cut_paste_mode != d.file_ops.copy_cut_paste_mode {
        pairs.push((KEYS::COPY_CUT_PASTE_MODE, cfg.file_ops.copy_cut_paste_mode.to_string()));
    }
    if cfg.file_ops.move_file_confirmation != d.file_ops.move_file_confirmation {
        pairs.push((KEYS::MOVE_FILE_CONFIRMATION, bool_u8(cfg.file_ops.move_file_confirmation).to_string()));
    }
    if cfg.file_ops.refresh_keep_sel != d.file_ops.refresh_keep_sel {
        pairs.push((KEYS::REFRESH_KEEP_SEL, bool_u8(cfg.file_ops.refresh_keep_sel).to_string()));
    }
    if cfg.file_ops.refresh_keep_scroll != d.file_ops.refresh_keep_scroll {
        pairs.push((KEYS::REFRESH_KEEP_SCROLL, bool_u8(cfg.file_ops.refresh_keep_scroll).to_string()));
    }
    if cfg.file_ops.select_last_if_navigate_up != d.file_ops.select_last_if_navigate_up {
        pairs.push((
            KEYS::SELECT_LAST_IF_NAVIGATE_UP,
            bool_u8(cfg.file_ops.select_last_if_navigate_up).to_string(),
        ));
    }
    if cfg.columns_plus_view != d.columns_plus_view {
        pairs.push((KEYS::COLUMNS_PLUS_VIEW, bool_u8(cfg.columns_plus_view).to_string()));
    }
    if cfg.colorthema != d.colorthema {
        pairs.push((KEYS::COLORTHEMA, cfg.colorthema.to_string()));
    }
    if cfg.langid != d.langid {
        pairs.push((KEYS::LANGID, cfg.langid.to_string()));
    }
    if cfg.font_face_name != d.font_face_name {
        pairs.push((KEYS::FONT_FACE_NAME, cfg.font_face_name.clone()));
    }
    if cfg.lang_name != d.lang_name {
        pairs.push((KEYS::LANG_NAME, cfg.lang_name.clone()));
    }
    if cfg.show_filter != d.show_filter {
        pairs.push((KEYS::SHOW_FILTER, bool_u8(cfg.show_filter).to_string()));
    }
    if cfg.last_filter != d.last_filter {
        pairs.push((KEYS::LAST_FILTER, cfg.last_filter.clone()));
    }
    if cfg.by_dir_filter != d.by_dir_filter {
        pairs.push((KEYS::BY_DIR_FILTER, cfg.by_dir_filter.clone()));
    }
    if cfg.by_filter != d.by_filter {
        pairs.push((KEYS::BY_FILTER, cfg.by_filter.clone()));
    }
    if cfg.markierungsfilter != d.markierungsfilter {
        pairs.push((KEYS::MARKIERUNGSFILTER, cfg.markierungsfilter.clone()));
    }
    if cfg.last_file_filter != d.last_file_filter {
        pairs.push((KEYS::LAST_FILE_FILTER, cfg.last_file_filter.clone()));
    }
    if cfg.fav_filter != d.fav_filter {
        pairs.push((KEYS::FAV_FILTER, cfg.fav_filter.clone()));
    }

    {
        let sec = doc.section_mut(start);
        for (key, value) in pairs {
            if sec.get(key).is_none() {
                sec.set(key, value);
            }
        }
    }
    if cfg.favoriten_dir != d.favoriten_dir
        && doc.section_mut(INI_SECTIONS::FAVORITEN).get(KEYS::ORDNER).is_none()
    {
        doc.set(INI_SECTIONS::FAVORITEN, KEYS::ORDNER, cfg.favoriten_dir.to_string_lossy());
    }
    if cfg.quick_link_dir != d.quick_link_dir
        && doc.section_mut(INI_SECTIONS::Q).get(KEYS::LINK).is_none()
    {
        doc.set(INI_SECTIONS::Q, KEYS::LINK, cfg.quick_link_dir.to_string_lossy());
    }
    if cfg.preview_filter != d.preview_filter
        && doc.section_mut(INI_SECTIONS::PREVIEW).get(KEYS::PREVIEW_FILTER).is_none()
    {
        doc.set(INI_SECTIONS::PREVIEW, KEYS::PREVIEW_FILTER, cfg.preview_filter.clone());
    }
}

/// bool -> ini 的 0/1。
fn bool_u8(b: bool) -> u8 {
    if b {
        1
    } else {
        0
    }
}

// ============ 会话序列化（.qdr） ============

/// FolderN 的拆解结果（E3）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QdrFolder {
    /// IDLIST 十六进制串（`%XX%XX...`，原样保留）。
    pub idlist: String,
    /// `&ViewModeFlagsN=` 子字段值。
    pub view_mode_flags: Option<u32>,
    /// `&ViewModeN=` 子字段值（注意不是顶层键）。
    pub view_mode: Option<u32>,
    /// `&UNICODE=` 子字段值。
    pub unicode: Option<u32>,
}

impl QdrFolder {
    /// 解析 `FolderN` 值。
    pub fn parse(raw: &str) -> QResult<Self> {
        let mut f = QdrFolder {
            idlist: String::new(),
            ..QdrFolder::default()
        };
        // 第一个 '&' 之前是 IDLIST；其后是子字段。
        let (idlist, rest) = match raw.split_once('&') {
            Some((a, b)) => (a, Some(b)),
            None => (raw, None),
        };
        f.idlist = idlist.to_string();
        if let Some(rest) = rest {
            for part in rest.split('&') {
                let (k, v) = part.split_once('=').unwrap_or((part, ""));
                // 去掉结尾数字后缀后匹配子字段名：ViewModeFlags1 -> ViewModeFlags，
                // ViewMode1 -> ViewMode（E3：这些不是顶层键）。
                let base = k.trim_end_matches(|c: char| c.is_ascii_digit());
                match base {
                    QDR_KEYS::SUB_VIEW_MODE_FLAGS => {
                        f.view_mode_flags = Some(parse_u32(k, v)?);
                    }
                    QDR_KEYS::SUB_VIEW_MODE => {
                        f.view_mode = Some(parse_u32(k, v)?);
                    }
                    QDR_KEYS::SUB_UNICODE => f.unicode = Some(parse_u32(k, v)?),
                    _ => {} // 未知子字段不保留文本（原版未见，差异已登记）
                }
            }
        }
        Ok(f)
    }

    /// 序列化回 FolderN 值。IDLIST 原样输出，子字段按原顺序重排为
    /// `ViewModeFlagsN / ViewModeN / UNICODE`（与实测文件一致）。
    pub fn to_ini_value(&self, n: u32) -> String {
        let mut s = self.idlist.clone();
        if let Some(v) = self.view_mode_flags {
            s.push('&');
            s.push_str(QDR_KEYS::SUB_VIEW_MODE_FLAGS);
            s.push_str(&n.to_string());
            s.push('=');
            s.push_str(&v.to_string());
        }
        if let Some(v) = self.view_mode {
            s.push('&');
            s.push_str(QDR_KEYS::SUB_VIEW_MODE);
            s.push_str(&n.to_string());
            s.push('=');
            s.push_str(&v.to_string());
        }
        if let Some(v) = self.unicode {
            s.push('&');
            s.push_str(QDR_KEYS::SUB_UNICODE);
            s.push('=');
            s.push_str(&v.to_string());
        }
        s
    }
}

/// 从 UTF-16LE 字节加载 [`SessionState`]，并附带四个 `FolderN` 的 IDLIST 原文
/// （`%XX%XX...` 百分号串；空串=该窗格无 IDLIST）。
///
/// **路径解码不在此处**：core/config 无 shell 依赖（契约 A1），IDLIST 字节 ↔
/// 路径的换算由调用方经 `shell::idlist` 完成（解码后覆盖对应标签页 path）。
/// 历史上 load 侧直接丢弃 IDLIST（差异已登记），导致 .qdr 打开/重启恢复/
/// 布局槽载入一律丢失窗格路径——本函数把原文交还调用方补齐该链路。
pub fn load_session_bytes_with_idlists(bytes: &[u8]) -> QResult<(SessionState, [String; 4])> {
    let text = decode_utf16le(bytes)?;
    let doc = IniDocument::parse(&text);
    let mut st = SessionState::default();

    st.version = get_string(&doc, "", QDR_KEYS::VERSION, &st.version);
    st.view_mode = get_u32(&doc, "", QDR_KEYS::VIEW_MODE, st.view_mode)?;
    st.tree_mode = TreeMode::from_u32(get_u32(&doc, "", QDR_KEYS::TREE_MODE, st.tree_mode.to_u32())?);
    st.preview_visible = get_bool(&doc, "", QDR_KEYS::VORSCHAU, st.preview_visible)?;
    st.preview_pos = SplitPermille::from_qdr_ten_thousandths(get_u32(
        &doc,
        "",
        QDR_KEYS::VORSCHAU_POS,
        st.preview_pos.to_qdr_ten_thousandths(),
    )?);
    st.colorthema = get_u32(&doc, "", QDR_KEYS::COLORTHEMA, st.colorthema)?;

    // Focus=1..4；越界报 Session 错误（QError::Session 的用途）。
    if let Some(raw) = doc.get("", QDR_KEYS::FOCUS) {
        let n = parse_u32(QDR_KEYS::FOCUS, raw)?;
        st.focus = PaneId::from_number(n)
            .ok_or_else(|| QError::Session(format!("Focus={n} 不是 1..=4")))?;
    }

    // 水平分割 SP0/SP1/SP2 与预览 SPT（E5：万分数 -> 千分比）。
    for (i, key) in [QDR_KEYS::SP0, QDR_KEYS::SP1, QDR_KEYS::SP2].iter().enumerate() {
        let v = get_u32(&doc, "", key, st.splits_h[i].to_qdr_ten_thousandths())?;
        st.splits_h[i] = SplitPermille::from_qdr_ten_thousandths(v);
    }
    st.split_preview = SplitPermille::from_qdr_ten_thousandths(get_u32(
        &doc,
        "",
        QDR_KEYS::SPT,
        st.split_preview.to_qdr_ten_thousandths(),
    )?);

    let mut idlists: [String; 4] = Default::default();

    for pane_id in PaneId::ALL {
        let n = pane_id.number();
        let mut pane = PaneState::new(pane_id, st.pane(pane_id).path());

        // TreeN / TreeNsplit。
        let tree_key = format!("{}{}", QDR_KEYS::TREE, n);
        pane.tree_visible = get_bool(&doc, "", &tree_key, pane.tree_visible)?;
        let tree_split_key = format!("Tree{n}split");
        let split = get_u32(&doc, "", &tree_split_key, pane.tree_split * 10)?;
        pane.tree_split = split / 10;

        // FolderN：拆出 ViewModeN / ViewModeFlagsN（E3）；IDLIST 原文交还调用方
        // （core 无 shell 依赖，不在此解码成路径——由 shell::idlist 换算）。
        let folder_key = format!("{}{}", QDR_KEYS::FOLDER, n);
        if let Some(raw) = doc.get("", &folder_key) {
            let folder = QdrFolder::parse(raw)?;
            idlists[(n - 1) as usize] = folder.idlist;
            if let Some(vm) = folder.view_mode {
                if let Some(tab) = pane.current_tab_mut() {
                    tab.view_mode = ViewMode::from_u32(vm);
                }
            }
            if let Some(flags) = folder.view_mode_flags {
                pane.columns.folder_flags = flags;
            }
        }

        // ColsAndSortN / ColsIDN 原样保留（E4）。
        st.cols_and_sort_raw[(n - 1) as usize] =
            get_string(&doc, "", &format!("{}{}", QDR_KEYS::COLS_AND_SORT, n), "");
        let cols_id = get_string(&doc, "", &format!("{}{}", QDR_KEYS::COLS_ID, n), "");
        if !cols_id.is_empty() {
            // ColsID 是列配置标识串（实测 `Spatlen_::{20D04FE0-...}`），
            // 不解析，只作为 group_by 之外的元数据存不进去 core；
            // 因此仅在不为空时保留在 ColumnConfig 里没有字段可放 —— 丢弃会违反
            // E4 的“不丢弃”要求，故原样写回 doc（见 apply_session_to_doc）。
            // 这里什么都不做：cols_and_sort_raw 已覆盖 E4 的核心要求。
        }

        *st.pane_mut(pane_id) = pane;
    }

    Ok((st, idlists))
}

/// 从 UTF-16LE 字节加载 [`SessionState`]（丢弃 IDLIST 原文的便捷包装；
/// 需要恢复窗格路径的调用方用 [`load_session_bytes_with_idlists`]）。
pub fn load_session_bytes(bytes: &[u8]) -> QResult<SessionState> {
    Ok(load_session_bytes_with_idlists(bytes)?.0)
}

/// 把 [`SessionState`] 写回一个已解析的 .qdr 文档（顶层键形式）。
///
/// 未建模的顶层键（`windowplacement`、未知未来键）由 doc 原样保留。
/// 不带路径编码的便捷包装（保留既有调用方/测试行为）。
pub fn apply_session_to_doc(doc: &mut IniDocument, st: &SessionState) {
    apply_session_to_doc_with(doc, st, None);
}

/// [`apply_session_to_doc`] 的完整版：`encode_idlist` 非 None 时，对
/// **原文档缺失 FolderN 或 IDLIST 为空**的窗格，用当前标签页路径编码注入
/// （否则新文件会写出空路径的会话——布局槽/重启恢复曾因此丢失全部窗格路径）。
pub fn apply_session_to_doc_with(
    doc: &mut IniDocument,
    st: &SessionState,
    encode_idlist: Option<&dyn Fn(&Path) -> Option<String>>,
) {
    doc.set("", QDR_KEYS::VERSION, st.version.clone());
    doc.set("", QDR_KEYS::VIEW_MODE, st.view_mode.to_string());
    // 万分数精度：SplitPermille 只存千分比（契约类型，不可改），
    // 因此 5003 -> 500 -> 5000 会有 3/10000 的取整损失。
    // E2 要求「除本次修改的键外逐字节等价」，故：
    //   - 原文本经 from_qdr_ten_thousandths 换算后与内部值相同 => 原样保留原文；
    //   - 否则才写回换算值（此时用户确实改过分割位置，取整损失是量纲固有上限）。
    for (i, key) in [QDR_KEYS::SP0, QDR_KEYS::SP1, QDR_KEYS::SP2].iter().enumerate() {
        let want = st.splits_h[i];
        let old_raw = doc.get("", key).map(|s| s.to_string());
        let old_val = old_raw
            .as_deref()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .map(SplitPermille::from_qdr_ten_thousandths);
        if old_val == Some(want) {
            continue; // 原文换算后一致，保留原值（如 5003）
        }
        doc.set("", key, want.to_qdr_ten_thousandths().to_string());
    }
    doc.set("", QDR_KEYS::SPT, st.split_preview.to_qdr_ten_thousandths().to_string());
    doc.set("", QDR_KEYS::TREE_MODE, st.tree_mode.to_u32().to_string());
    doc.set("", QDR_KEYS::VORSCHAU, bool_u8(st.preview_visible).to_string());
    doc.set("", QDR_KEYS::VORSCHAU_POS, st.preview_pos.to_qdr_ten_thousandths().to_string());
    doc.set("", QDR_KEYS::FOCUS, st.focus.number().to_string());
    doc.set("", QDR_KEYS::COLORTHEMA, st.colorthema.to_string());

    for pane_id in PaneId::ALL {
        let n = pane_id.number();
        let pane = st.pane(pane_id);
        doc.set("", &format!("{}{}", QDR_KEYS::TREE, n), bool_u8(pane.tree_visible).to_string());
        doc.set("", &format!("Tree{n}split"), (pane.tree_split * 10).to_string());

        // FolderN：保留原 IDLIST 与子字段，只在标签页显式覆盖时改 ViewModeN。
        // 原文档缺失/IDLIST 为空且提供了编码器时，用当前标签页路径编码注入
        // （新文件此前会写出空路径会话——路径全丢）。
        let folder_key = format!("{}{}", QDR_KEYS::FOLDER, n);
        let mut folder = doc
            .get("", &folder_key)
            .and_then(|raw| QdrFolder::parse(raw).ok())
            .unwrap_or_else(|| QdrFolder {
                idlist: String::new(),
                ..QdrFolder::default()
            });
        if folder.idlist.is_empty() {
            if let Some(enc) = encode_idlist {
                if let Some(tab) = pane.current_tab() {
                    if !tab.path.as_os_str().is_empty() {
                        if let Some(hex) = enc(&tab.path) {
                            folder.idlist = hex;
                        }
                    }
                }
            }
        }
        // 注意：不能写 `pane.view_mode()`（生效值）—— 那会把「未显式设置」
        // 固化成默认值，破坏往返。只有标签页显式覆盖时才写子字段；
        // 否则保留原文件的原始值（实测 ViewMode1=4 超出 0..=3 枚举范围，
        // 正是靠这条保留才不丢）。
        if let Some(vm) = pane.current_tab().and_then(|t| t.view_mode) {
            folder.view_mode = Some(vm.to_u32());
        }
        if pane.columns.folder_flags != 0 {
            folder.view_mode_flags = Some(pane.columns.folder_flags);
        }
        doc.set("", &folder_key, folder.to_ini_value(n));

        // ColsAndSortN 原样回写（E4）。
        let raw = &st.cols_and_sort_raw[(n - 1) as usize];
        doc.set("", &format!("{}{}", QDR_KEYS::COLS_AND_SORT, n), raw.clone());
    }
}

// ============ 便捷入口（路径级） ============

/// 加载 Q-Dir.ini；文件不存在时返回 [`Config::default`]（不 panic）。
pub fn load_config(path: &Path) -> QResult<Config> {
    match fs::read(path) {
        Ok(bytes) => load_config_bytes(&bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(QError::from_io(&e)),
    }
}

/// 保存 Q-Dir.ini（UTF-16LE）。
///
/// 若目标文件已存在，先解析原文档再套用 [`Config`]，以保留未知键与顺序（E2）；
/// 不存在则按默认段骨架新建。
pub fn save_config(path: &Path, cfg: &Config) -> QResult<()> {
    let mut doc = match fs::read(path) {
        Ok(bytes) => {
            let text = decode_utf16le(&bytes)?;
            IniDocument::parse(&text)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => default_ini_skeleton(),
        Err(e) => return Err(QError::from_io(&e)),
    };
    apply_config_to_doc(&mut doc, cfg);
    write_utf16le_file(path, &doc.to_text())
}

/// 加载 start.qdr；文件不存在时返回 [`SessionState::default`]（不 panic）。
pub fn load_session(path: &Path) -> QResult<SessionState> {
    Ok(load_session_with_idlists(path)?.0)
}

/// [`load_session`] 的完整版：附带四个 `FolderN` 的 IDLIST 原文
/// （调用方经 `shell::idlist::idlist_hex_to_path` 解码后覆盖窗格路径）。
/// 文件不存在时返回默认会话 + 四个空 IDLIST。
pub fn load_session_with_idlists(path: &Path) -> QResult<(SessionState, [String; 4])> {
    match fs::read(path) {
        Ok(bytes) => load_session_bytes_with_idlists(&bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok((SessionState::default(), Default::default()))
        }
        Err(e) => Err(QError::from_io(&e)),
    }
}

/// 保存 start.qdr（UTF-16LE），保留未建模键。
pub fn save_session(path: &Path, st: &SessionState) -> QResult<()> {
    save_session_with(path, st, None)
}

/// 保存 start.qdr，并给「新文件/空 IDLIST」的 `FolderN` 注入路径编码。
///
/// `encode_idlist`：`路径 -> %XX%XX...`（调用方经 `shell::idlist::path_to_idlist_hex`
/// 注入；core/config 无 shell 依赖）。已有 IDLIST 的原文件不受影响（E2 保留原文）。
pub fn save_session_with(
    path: &Path,
    st: &SessionState,
    encode_idlist: Option<&dyn Fn(&Path) -> Option<String>>,
) -> QResult<()> {
    let mut doc = match fs::read(path) {
        Ok(bytes) => {
            let text = decode_utf16le(&bytes)?;
            IniDocument::parse(&text)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            IniDocument::parse("")
        }
        Err(e) => return Err(QError::from_io(&e)),
    };
    apply_session_to_doc_with(&mut doc, st, encode_idlist);
    write_utf16le_file(path, &doc.to_text())
}

/// 新建 ini 时的默认段骨架（顺序与实测 Q-Dir.ini 一致）。
fn default_ini_skeleton() -> IniDocument {
    let mut doc = IniDocument::parse("");
    for s in [
        INI_SECTIONS::START,
        INI_SECTIONS::FAVORITEN,
        INI_SECTIONS::Q,
        INI_SECTIONS::QDIR,
        INI_SECTIONS::FILTER2,
        INI_SECTIONS::PREVIEW,
        INI_SECTIONS::ORDNER,
    ] {
        doc.section_mut(s);
    }
    doc
}

// ============ 列配置与视图状态串 ============

/// 从 ColumnConfig 生成视图状态串（薄封装，供 pane 层调用）。
pub fn view_state_string(cfg: &ColumnConfig) -> String {
    cfg.to_view_state_string()
}

/// 从视图状态串解析 ColumnConfig。
pub fn parse_view_state(s: &str) -> QResult<ColumnConfig> {
    ColumnConfig::parse_view_state_string(s)
}

/// langid -> [`Lang`]；未知 langid 回落德语（1031 是资源原始语言）。
pub fn lang_from_langid(id: u32) -> Lang {
    Lang::from_langid(id).unwrap_or(Lang::De)
}

/// 由 [`Config`] 推导界面语言（LangName 优先于 langid）。
pub fn config_lang(cfg: &Config) -> Lang {
    let name = cfg.lang_name.trim();
    if !name.is_empty() {
        // LangName 形如 "Deutsch (Deutschland)" / "English (United States)"；
        // 只取首个空格前的语言名做协商，其余交给 i18n::langneg。
        let head = name.split_whitespace().next().unwrap_or("");
        match head {
            "Deutsch" => return Lang::De,
            "English" => return Lang::En,
            "中文" | "Chinese" => return Lang::ZhCn,
            _ => {}
        }
    }
    lang_from_langid(cfg.langid)
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    /// 实测 Q-Dir.ini 的 [Filter2] 段 12 行（来自 D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir.ini）。
    fn evidence_filter2_lines() -> Vec<String> {
        vec![
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
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    #[test]
    fn utf16le_roundtrip_and_bom() {
        let text = "[Start]\r\nQDir_Id=1\r\nAls=5\r\n";
        let bytes = encode_utf16le(text);
        assert_eq!(&bytes[..2], &UTF16_BOM_LE);
        let back = decode_utf16le(&bytes).expect("往返解码");
        assert_eq!(back, text.replace("\r\n", "\n"));
        // 无 BOM 也接受（手写 ini 常见）。
        let no_bom = &bytes[2..];
        assert_eq!(decode_utf16le(no_bom).expect("无 BOM 解码"), back);
    }

    #[test]
    fn utf16le_odd_length_and_surrogates_rejected() {
        let err = decode_utf16le(&[0x41, 0x00, 0x42]).unwrap_err();
        assert!(matches!(err, QError::Encoding(_)));
        // 游离高位代理。
        let err = decode_utf16le(&[0x00, 0xD8]).unwrap_err();
        assert!(matches!(err, QError::Encoding(_)));
        // 合法代理对（U+1F600）。
        let ok = decode_utf16le(&[0x3D, 0xD8, 0x00, 0xDE]).expect("代理对应可解码");
        assert_eq!(ok, "\u{1F600}");
    }

    #[test]
    fn ini_document_preserves_order_and_unknown_keys() {
        let text = "[Start]\nB=2\nA=1\n\n[Other]\nX=keep\n";
        let doc = IniDocument::parse(text);
        assert_eq!(doc.section_order, vec!["Start".to_string(), "Other".to_string()]);
        assert_eq!(doc.get("Start", "A"), Some("1"));
        assert_eq!(doc.get("Start", "B"), Some("2"));
        assert_eq!(doc.get("Other", "X"), Some("keep"));
        // 键顺序保持：B 在 A 前。
        let kv: Vec<(&str, &str)> = doc.section("Start").expect("Start").kv().collect();
        assert_eq!(kv, vec![("B", "2"), ("A", "1")]);
        // 空行保留（段内 3 项：B、A 与一个空行项）。
        assert_eq!(doc.section("Start").expect("Start").entries.len(), 3);
        let out = doc.to_text();
        assert_eq!(out, text);
    }

    #[test]
    fn ini_set_keeps_position_unknown_keys_intact() {
        let mut doc = IniDocument::parse("[Start]\nB=2\nA=1\nZ=future\n");
        doc.set("Start", "A", "9");
        let out = doc.to_text();
        assert_eq!(out, "[Start]\nB=2\nA=9\nZ=future\n");
        // 新键追加到段尾。
        doc.set("Start", "NEW", "1");
        assert_eq!(doc.get("Start", "NEW"), Some("1"));
        assert_eq!(doc.to_text(), "[Start]\nB=2\nA=9\nZ=future\nNEW=1\n");
    }

    #[test]
    fn filter2_section_roundtrip_through_ini_document() {
        // E6 + F：解析 12 行证据 -> 回写 -> 再解析必须一致（rule 4 分隔符已登记差异）。
        let mut doc = IniDocument::parse("[Filter2]\n0=#Hidden=1=-1=1=1=-1=0\n");
        for line in &evidence_filter2_lines()[1..] {
            let (k, v) = line.split_once('=').expect("有 '='");
            doc.set("Filter2", k, v);
        }
        let sec = doc.section("Filter2").expect("Filter2");
        let lines: Vec<String> = sec.kv().map(|(k, v)| format!("{k}={v}")).collect();
        assert_eq!(lines.len(), 12);
        let rs = crate::core::RuleSet::parse_ini_section(&lines).expect("可解析");
        let back = rs.to_ini_lines();
        assert_eq!(back.len(), 12);
        assert_eq!(back[0], "0=#Hidden=1=-1=1=1=-1=0");
        assert_eq!(back[1], "1=*.zip;*.rar;*.gz;*.7z;=1=00AA00=-1=1=-1=0");
        assert_eq!(back[4], "4=*.jpg;*.jpeg;*.png;*.gif;*.bmp;*.ico=1=BB00BB=-1=1=-1=0");
        // 注：rule 4 原始串在串内混用 ','，回写统一为 ';'（语义等价，已登记差异）。
        // 重新解析回写结果，语义必须不变。
        let rs2 = crate::core::RuleSet::parse_ini_section(&back).expect("可再解析");
        assert_eq!(rs, rs2);
    }

    #[test]
    fn qdr_folder_split_and_rebuild() {
        // E3：实测 FolderN 值形态。
        let raw = "%14%00%1F%50%E0%4F%D0%20%EA%3A%69%10%A2%D8%08%00%2B%30%30%9D%00%00&ViewModeFlags1=1075838981&ViewMode1=4&UNICODE=1";
        let f = QdrFolder::parse(raw).expect("可解析");
        assert_eq!(f.view_mode_flags, Some(1075838981));
        assert_eq!(f.view_mode, Some(4));
        assert_eq!(f.unicode, Some(1));
        assert!(f.idlist.starts_with("%14%00"));
        // 重序列化必须与原始串一致（E3：不破坏 FolderN 结构）。
        assert_eq!(f.to_ini_value(1), raw);
    }

    #[test]
    fn qdr_folder_view_mode_is_subfield_not_top_level() {
        // 顶层键 "ViewMode" 与 FolderN 的 "ViewMode1" 必须互不干扰（E3）。
        let text = "ViewMode=0\nFolder1=abc&ViewModeFlags1=1075838981&ViewMode1=3&UNICODE=1\n";
        let st = load_session_bytes(&encode_utf16le(text)).expect("可加载");
        assert_eq!(st.view_mode, 0, "顶层 ViewMode 保持 0");
        assert_eq!(
            st.pane(PaneId::P1).view_mode(),
            ViewMode::Detail,
            "Folder1 的 ViewMode1=3 是子字段，决定窗格视图"
        );
        assert_eq!(
            st.pane(PaneId::P1).columns.folder_flags,
            1075838981,
            "ViewModeFlags1 同为子字段"
        );
    }

    #[test]
    fn qdr_loads_split_and_focus_from_evidence() {
        // start.qdr 实测键值。
        let text = "SP0=5000\nSP1=5003\nSP2=5003\nSPT=2000\nFocus=1\nVorschauPos=7500\ntree_mode=0\nTree1split=2500\n";
        let st = load_session_bytes(&encode_utf16le(text)).expect("可加载");
        assert_eq!(st.splits_h[0], SplitPermille(500));
        assert_eq!(st.splits_h[1], SplitPermille(500), "5003/10 截断为 500");
        assert_eq!(st.split_preview, SplitPermille(200));
        assert_eq!(st.preview_pos, SplitPermille(750));
        assert_eq!(st.focus, PaneId::P1);
        assert_eq!(st.pane(PaneId::P1).tree_split, 250);
        assert_eq!(st.tree_mode, TreeMode::Off);
    }

    #[test]
    fn qdr_vorschau_roundtrip() {
        // 预览开关 Vorschau=1 与位置 VorschauPos 往返（save 走文件，load 走字节）。
        let mut st = SessionState::default();
        st.preview_visible = true;
        st.preview_pos = SplitPermille(300);
        let dir = std::env::temp_dir().join("qdir-rs-vorschau-test");
        std::fs::create_dir_all(&dir).expect("tmp dir");
        let path = dir.join("pv.qdr");
        save_session(&path, &st).expect("存");
        let bytes = std::fs::read(&path).expect("读");
        let back = load_session_bytes(&bytes).expect("载");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(back.preview_visible, "Vorschau=1 往返");
        assert_eq!(back.preview_pos.value(), 300);
    }

    #[test]
    fn qdr_focus_out_of_range_is_session_error() {
        let text = "Focus=9\n";
        let err = load_session_bytes(&encode_utf16le(text)).unwrap_err();
        assert!(matches!(err, QError::Session(_)), "Focus 越界应为 Session 错误");
    }

    #[test]
    fn config_missing_file_yields_defaults_not_panic() {
        // 任务要求：路径不存在时返回默认值而不是 panic。
        let cfg = load_config(Path::new("Z:\\definitely-not-here\\Q-Dir.ini")).expect("默认值");
        assert_eq!(cfg.qdir_id, 1);
        assert_eq!(cfg.rules.rules.len(), 12);
        let st = load_session(Path::new("Z:\\definitely-not-here\\start.qdr")).expect("默认值");
        assert_eq!(st.version, "12.71");
        assert_eq!(st.focus, PaneId::P1);
    }

    #[test]
    fn config_load_from_evidence_bytes() {
        // 用真实 ini 字节做最小投影断言（完整往返见 tests/config_tests.rs）。
        let text = "[Start]\nQDir_Id=1\nuseColorStart=1\nAls=5\ndesigne_mode=1\nshowCmd=3\nStartCrash=0\nWinRC=0;0;800;600\n[Favoriten]\nOrdner=.\\Favoriten\\\n[Q]\nLink=.\\Favoriten\\Quick-Link\\\n[Filter2]\n";
        let mut text = text.to_string();
        for l in &evidence_filter2_lines() {
            text.push_str(l);
            text.push('\n');
        }
        text.push_str("[Preview]\nFilter=*;\n[Ordner]\nFilter=\n");
        let bytes = encode_utf16le(&text);
        let cfg = load_config_bytes(&bytes).expect("可加载");
        assert_eq!(cfg.qdir_id, 1);
        assert_eq!(cfg.als, 5);
        assert!(cfg.use_color_start);
        assert!(!cfg.start_crash);
        assert_eq!(cfg.win_rc, [0, 0, 800, 600]);
        assert_eq!(cfg.favoriten_dir, PathBuf::from(".\\Favoriten\\"));
        assert_eq!(cfg.preview_filter, "*;");
        assert_eq!(cfg.rules.rules.len(), 12);
    }

    #[test]
    fn config_bad_key_reports_key_name() {
        // C3：配置错误必须带上原始键名。
        let bytes = encode_utf16le("[Start]\nAls=not-a-number\n");
        let err = load_config_bytes(&bytes).unwrap_err();
        match err {
            QError::Config { key, .. } => assert_eq!(key, "Als"),
            other => panic!("应为 Config 错误，实际 {other:?}"),
        }
    }

    #[test]
    fn config_lang_and_view_state_helpers() {
        let mut cfg = Config::default();
        assert_eq!(config_lang(&cfg), Lang::De);
        cfg.langid = 2052;
        assert_eq!(config_lang(&cfg), Lang::ZhCn);
        cfg.lang_name = "English (United States)".to_string();
        assert_eq!(config_lang(&cfg), Lang::En);
        cfg.lang_name = "Deutsch (Deutschland)".to_string();
        assert_eq!(config_lang(&cfg), Lang::De);
        assert_eq!(lang_from_langid(1041), Lang::De, "未知 langid 回落德语");

        let c = ColumnConfig::default();
        let s = view_state_string(&c);
        assert!(s.starts_with("ViewMode,3;"));
        let back = parse_view_state(&s).expect("可解析");
        assert_eq!(back.view_mode, c.view_mode);
    }
    #[test]
    fn show_hidden_key_roundtrip_and_ensure() {
        // 批次C/C1：show_hidden 键解析→写回往返；缺键时只有改过才补写。
        let mut doc = IniDocument::parse("[Start]
Als=5
");
        let mut cfg = Config::default();
        assert!(cfg.show_hidden, "默认 true=保持现行行为");
        cfg.show_hidden = false;
        apply_config_to_doc(&mut doc, &cfg);
        let sec = doc.section("Start").unwrap();
        assert_eq!(sec.get("show_hidden"), Some("0"), "改过的 show_hidden 补写");
        let text = doc.to_text();
        let back = load_config_bytes(&encode_utf16le(&text)).unwrap();
        assert!(!back.show_hidden);
        // 显式写 1 → 解析为 true。
        let mut doc2 = IniDocument::parse("[Start]
show_hidden=1
");
        let cfg2 = load_config_bytes(&encode_utf16le(&doc2.to_text())).unwrap();
        assert!(cfg2.show_hidden);
        // show_hidden=true（默认）不补写。
        let mut doc3 = IniDocument::parse("[Start]
Als=5
");
        apply_config_to_doc(&mut doc3, &Config::default());
        assert!(doc3.section("Start").unwrap().get("show_hidden").is_none());
    }

    #[test]
    fn startup_key_roundtrip_and_ensure() {
        // startup 键：解析→写回往返；缺键时只有改过才补写（ensure 路径）。
        let mut doc = IniDocument::parse("[Start]
Als=5
");
        let mut cfg = Config::default();
        cfg.startup = true;
        apply_config_to_doc(&mut doc, &cfg);
        let sec = doc.section("Start").unwrap();
        assert_eq!(sec.get("startup"), Some("1"), "改过的 startup 补写");
        // 从该文档解析回来。
        let text = doc.to_text();
        let back = load_config_bytes(&encode_utf16le(&text)).unwrap();
        assert!(back.startup);
        // startup=false（默认）不补写。
        let mut doc2 = IniDocument::parse("[Start]
Als=5
");
        let cfg2 = Config::default();
        apply_config_to_doc(&mut doc2, &cfg2);
        assert!(doc2.section("Start").unwrap().get("startup").is_none());
    }

    #[test]
    fn ensure_modeled_keys_skips_defaults_and_appends_changes() {
        // 原文件没有 tree_* 键：未改动的默认值不得补写；改过的必须补写。
        let mut doc = IniDocument::parse("[Start]
Als=5
");
        let mut cfg = Config::default();
        apply_config_to_doc(&mut doc, &cfg);
        let sec = doc.section("Start").unwrap();
        assert!(sec.get("tree_indent_width").is_none(), "默认值不得补写");
        // 改 indent_width（默认 20 -> 24）后补写。
        cfg.tree.indent_width = 24;
        cfg.tree.no_expando = true;
        apply_config_to_doc(&mut doc, &cfg);
        let sec = doc.section("Start").unwrap();
        assert_eq!(sec.get("tree_indent_width"), Some("24"));
        assert_eq!(sec.get("tree_no_expando"), Some("1"));
        // 补写不破坏既有键序：Als 仍在，且没有未知键。
        let keys: Vec<&str> = sec.entries.iter().filter_map(|e| e.key.as_deref()).collect();
        assert!(keys.contains(&"Als"), "原键保留");
    }
}
