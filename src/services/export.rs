//! 列表导出：TXT / CSV / HTML / XLS / Tabgetrennt(*.ini) 五种格式。
//!
//! 证据（原版 `Q-Dir_x64.exe`，`.rdata` 0x12a900..0x12ac00 实测）：
//! - 菜单项 `Export TXT,CSV,HTML,XLS`（菜单 216 的子项 `Liste &Exportieren`）；
//! - 文件对话框过滤器：`Text (*.txt)` / `CSV (*.csv)` / `html (*.html)` /
//!   `XLS (*.xls)` / `DOC (*.doc)` / `Tabgetrennt(*.ini)`；
//! - 扩展名常量：`.txt` `.csv` `.html` `.xls` `.doc` `.ini`；
//! - HTML 导出模板：`<TABLE Border=0>` + `<TR class="b_gelb">` + `bg_grau`
//!   交替行（实测字节，见本文件 HTML_TEMPLATE 附近注释）；
//! - 模板变量 `{Q-DIR-FILE-PATH}`（实测位于 0x131bc8，原版用于导出内容里
//!   展开当前文件路径）。
//!
//! 实现约定（未逐字节取证，实现者定夺）：
//! - XLS 采用**最简 SpreadsheetML**（XML 表格）而不是真实 BIFF，符合架构契约
//!   H2(c)：原版菜单名义提供 XLS，真实格式未从二进制确认，故登记差异。
//!   该 XML 用 Excel 打开可正确显示，且零额外依赖。
//! - 所有文本格式统一 UTF-8 **带 BOM**：Excel 双击打开 CSV/TXT 时按 UTF-8
//!   解码，否则中文/德语变音会乱码。
//! - 列可配置：调用方传入 [`ExportColumn`] 列表（顺序即输出顺序）。

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::core::{FileItem, QError, QResult};

// ============ 列定义 ============

/// 一列的数据来源。键名与 [`crate::core::ColumnDef::key`] 保持一致
/// （Name/Ext/Size/Modified/Attr），使窗格列配置可直接映射为导出列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExportColumn {
    Name,
    Ext,
    Size,
    Modified,
    Attr,
    Path,
}

impl ExportColumn {
    /// 全部可用列（界面列选择器的顺序）。
    pub const ALL: [ExportColumn; 6] = [
        ExportColumn::Name,
        ExportColumn::Ext,
        ExportColumn::Size,
        ExportColumn::Modified,
        ExportColumn::Attr,
        ExportColumn::Path,
    ];

    /// ini / 序列化用的稳定键名（不随显示语言变化）。
    pub fn key(self) -> &'static str {
        match self {
            ExportColumn::Name => "Name",
            ExportColumn::Ext => "Ext",
            ExportColumn::Size => "Size",
            ExportColumn::Modified => "Modified",
            ExportColumn::Attr => "Attr",
            ExportColumn::Path => "Path",
        }
    }

    /// 从键名解析；未知键返回 None（前向兼容，调用方可忽略）。
    pub fn from_key(k: &str) -> Option<Self> {
        Some(match k {
            "Name" => ExportColumn::Name,
            "Ext" => ExportColumn::Ext,
            "Size" => ExportColumn::Size,
            "Modified" => ExportColumn::Modified,
            "Attr" => ExportColumn::Attr,
            "Path" => ExportColumn::Path,
            _ => return None,
        })
    }

    /// 列在界面上的本地化标题（德语原文，B3 基线文案）。
    pub fn label_de(self) -> &'static str {
        match self {
            ExportColumn::Name => "Name",
            ExportColumn::Ext => "Erweiterung",
            ExportColumn::Size => "Größe",
            ExportColumn::Modified => "Geändert",
            ExportColumn::Attr => "Attribute",
            ExportColumn::Path => "Pfad",
        }
    }
}

// ============ 导出选项 ============

/// 导出参数。`columns` 为空时回落到 Name/Size/Modified 三列
/// （不视为错误：界面未配置过列时也要能导出）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportOptions {
    pub columns: Vec<ExportColumn>,
    /// 是否写 UTF-8 BOM（默认 true，Excel 兼容）。
    pub utf8_bom: bool,
    /// HTML/XLS 导出的文档标题。
    pub title: String,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            columns: vec![
                ExportColumn::Name,
                ExportColumn::Size,
                ExportColumn::Modified,
            ],
            utf8_bom: true,
            title: "Q-Dir Listexport".to_string(),
        }
    }
}

impl ExportOptions {
    /// 生效列：空配置回落默认三列。
    fn effective_columns(&self) -> &[ExportColumn] {
        if self.columns.is_empty() {
            &[
                ExportColumn::Name,
                ExportColumn::Size,
                ExportColumn::Modified,
            ]
        } else {
            &self.columns
        }
    }

    /// 列键名序列化（供配置持久化，格式 `Name;Size;Modified`）。
    pub fn columns_to_string(&self) -> String {
        self.effective_columns()
            .iter()
            .map(|c| c.key())
            .collect::<Vec<_>>()
            .join(";")
    }

    /// 解析列键名序列化；未知键跳过，全部未知时回落默认。
    pub fn columns_from_string(s: &str) -> Vec<ExportColumn> {
        let cols: Vec<ExportColumn> = s
            .split(';')
            .filter_map(ExportColumn::from_key)
            .collect();
        if cols.is_empty() {
            ExportOptions::default().columns
        } else {
            cols
        }
    }
}

// ============ 单元格取值 ============

/// 取一列在某个条目上的纯文本值（不含任何转义）。
fn cell_text(col: ExportColumn, item: &FileItem) -> String {
    match col {
        ExportColumn::Name => item.name.clone(),
        ExportColumn::Ext => item.ext_lower(),
        ExportColumn::Size => {
            if item.is_dir {
                // 目录不显示字节数（资源管理器惯例）。
                String::new()
            } else {
                item.size.to_string()
            }
        }
        ExportColumn::Modified => format_filetime(item.modified),
        ExportColumn::Attr => {
            let mut s = String::new();
            if item.is_dir {
                s.push('D');
            }
            if item.is_hidden {
                s.push('H');
            }
            s
        }
        ExportColumn::Path => item.path.to_string_lossy().to_string(),
    }
}

/// FILETIME（100ns 单位，1601 纪元）-> `YYYY-MM-DD HH:MM:SS`。
///
/// 自写历法换算而不引 `time` crate：本模块必须平台无关可单测（G1），
/// 且 core 禁止外部依赖。未跨时区——原版列表显示的就是本地时间，
/// 而 FILETIME 本身就是 UTC，这里按 UTC 输出（差异已登记）。
fn format_filetime(ft: u64) -> String {
    if ft == 0 {
        return String::new();
    }
    // 1601-01-01 -> 1970-01-01 的 100ns 单位数。
    const EPOCH_DIFF: u64 = 116_444_736_000_000_000;
    let total_100ns = ft.saturating_sub(EPOCH_DIFF);
    let secs = total_100ns / 10_000_000;
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// Howard Hinnant 的 days<->civil 日期算法（公有领域），
/// 把「1970 起的天数」转为 (年, 月, 日)。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ============ 转义 ============

/// CSV 字段转义：含分隔符/引号/换行时加双引号，内嵌引号翻倍。
/// 分隔符固定 `;`（德语区 Excel 的区域设置默认，实测原版 ini 本身
/// 也用 `;` 作列表分隔符，如 `WinRC=0;0;800;600`）。
fn csv_escape(s: &str) -> String {
    if s.contains(';') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// HTML 文本转义（属性与文本节点共用）。
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// XML 文本转义（SpreadsheetML 与 HTML 的必需集合一致）。
fn xml_escape(s: &str) -> String {
    html_escape(s)
}

// ============ 渲染 ============

/// 把条目列表渲染为指定格式的完整文本（尚未加 BOM）。
///
/// 纯函数：不碰文件系统、不依赖平台，可直接单测。
pub fn render_items(
    items: &[FileItem],
    format: crate::core::ExportFormat,
    opts: &ExportOptions,
) -> String {
    match format {
        crate::core::ExportFormat::Txt => render_txt(items, opts),
        crate::core::ExportFormat::Csv => render_csv(items, opts),
        crate::core::ExportFormat::Html => render_html(items, opts),
        crate::core::ExportFormat::Xls => render_xls(items, opts),
        crate::core::ExportFormat::TabSeparatedIni => render_tab_ini(items, opts),
    }
}

/// TXT：列宽对齐的纯文本表。
fn render_txt(items: &[FileItem], opts: &ExportOptions) -> String {
    let cols = opts.effective_columns();
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|it| cols.iter().map(|c| cell_text(*c, it)).collect())
        .collect();
    // 列宽 = max(表头, 所有单元格) 的字符数（按 char 计，中文不会错位）。
    let mut widths: Vec<usize> = cols.iter().map(|c| c.key().chars().count()).collect();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    let mut out = String::new();
    // 表头
    let header: Vec<String> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| pad(c.key(), widths[i]))
        .collect();
    let _ = writeln!(out, "{}", header.join("\t"));
    for row in &rows {
        let line: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| pad(cell, widths[i]))
            .collect();
        let _ = writeln!(out, "{}", line.join("\t"));
    }
    out
}

/// 右填充到指定字符数（按 char，不按字节）。
fn pad(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        let mut out = String::with_capacity(s.len() + (width - len));
        out.push_str(s);
        for _ in 0..(width - len) {
            out.push(' ');
        }
        out
    }
}

/// CSV：`;` 分隔，首行表头。
fn render_csv(items: &[FileItem], opts: &ExportOptions) -> String {
    let cols = opts.effective_columns();
    let mut out = String::new();
    let header: Vec<String> = cols.iter().map(|c| csv_escape(c.key())).collect();
    let _ = writeln!(out, "{}", header.join(";"));
    for it in items {
        let line: Vec<String> = cols
            .iter()
            .map(|c| csv_escape(&cell_text(*c, it)))
            .collect();
        let _ = writeln!(out, "{}", line.join(";"));
    }
    out
}

/// HTML：`<TABLE Border=0>` + 交替行类名（对齐原版模板的 class 体系）。
///
/// 原版实测模板片段（`.rdata` 0x12aab8 起）：
/// `<TABLE Border=0><TR class="b_gelb">` 与 `bg_grau`，
/// 表体单元格形如 `<td ...><b><br></td>`。重写保留 class 名与表格结构，
/// 具体 CSS 由调用方（app 层）注入——本模块只产出 body 片段与最小骨架。
fn render_html(items: &[FileItem], opts: &ExportOptions) -> String {
    let cols = opts.effective_columns();
    let mut out = String::new();
    let _ = writeln!(out, "<!DOCTYPE html>");
    let _ = writeln!(out, "<html><head>");
    let _ = writeln!(
        out,
        "<meta charset=\"utf-8\"><title>{}</title>",
        html_escape(&opts.title)
    );
    let _ = writeln!(out, "<style>");
    let _ = writeln!(
        out,
        "html,body {{ background-color:#FFFFFF; color:#000000; height:100%; margin:0; padding:0; }}"
    );
    let _ = writeln!(out, "table {{ border-collapse:collapse; }}");
    let _ = writeln!(out, "td {{ padding:2px 8px; white-space:nowrap; }}");
    let _ = writeln!(out, ".b_gelb {{ background-color:#FFFFCC; font-weight:bold; }}");
    let _ = writeln!(out, ".bg_grau {{ background-color:#EEEEEE; }}");
    let _ = writeln!(out, "</style></head><body>");
    let _ = writeln!(out, "<TABLE Border=0>");
    // 表头行：原版用 b_gelb。
    let _ = write!(out, "<TR class=\"b_gelb\">");
    for c in cols {
        let _ = write!(out, "<td><b>{}</b></td>", html_escape(c.key()));
    }
    let _ = writeln!(out, "</TR>");
    for (i, it) in items.iter().enumerate() {
        // 交替行：偶数 bg_grau（与原版 class 命名一致，奇偶取向定夺）。
        let cls = if i % 2 == 0 { " class=\"bg_grau\"" } else { "" };
        let _ = write!(out, "<TR{cls}>");
        for c in cols {
            let _ = write!(out, "<td>{}</td>", html_escape(&cell_text(*c, it)));
        }
        let _ = writeln!(out, "</TR>");
    }
    let _ = writeln!(out, "</TABLE>");
    let _ = writeln!(out, "</body></html>");
    out
}

/// XLS：最简 SpreadsheetML 2003（XML 表格）。
///
/// 真实 BIFF 未从二进制确认（契约 H2(c)），故采用 SpreadsheetML：
/// Excel 2003+ 可直接打开，WPS/LibreOffice 亦支持，且无需任何依赖。
/// 数字列输出为 `<Cell>` 的 `ss:Type="Number"`，保证求和/排序可用。
fn render_xls(items: &[FileItem], opts: &ExportOptions) -> String {
    let cols = opts.effective_columns();
    let mut out = String::new();
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(
        out,
        "<?mso-application progid=\"Excel.Sheet\"?>"
    );
    let _ = writeln!(
        out,
        "<Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\""
    );
    let _ = writeln!(
        out,
        " xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\">"
    );
    let _ = writeln!(out, "<Worksheet ss:Name=\"{}\">", xml_escape("Liste"));
    let _ = writeln!(out, "<Table>");
    // 表头
    let _ = write!(out, "<Row>");
    for c in cols {
        let _ = write!(
            out,
            "<Cell><Data ss:Type=\"String\">{}</Data></Cell>",
            xml_escape(c.key())
        );
    }
    let _ = writeln!(out, "</Row>");
    for it in items {
        let _ = write!(out, "<Row>");
        for c in cols {
            let text = cell_text(*c, it);
            // 仅 Size 是纯数字；目录/未知留空串。
            let is_num = matches!(c, ExportColumn::Size) && !it.is_dir && !text.is_empty();
            if is_num {
                let _ = write!(out, "<Cell ss:StyleID=\"s_num\"><Data ss:Type=\"Number\">{text}</Data></Cell>");
            } else {
                let _ = write!(
                    out,
                    "<Cell><Data ss:Type=\"String\">{}</Data></Cell>",
                    xml_escape(&text)
                );
            }
        }
        let _ = writeln!(out, "</Row>");
    }
    let _ = writeln!(out, "</Table>");
    let _ = writeln!(out, "</Worksheet>");
    let _ = writeln!(out, "</Workbook>");
    out
}

/// Tabgetrennt(*.ini)：Tab 分隔的 ini 风格文本。
///
/// 原版实测：菜单/过滤器串 `Tabgetrennt(*.ini)` / `*.ini`
/// （`.rdata` 0x12a988）。格式定夺：首行 `[Liste]` 段头 +
/// `Index<TAB>Name<TAB>...` 每行一条，保持 ini 的段结构以便重新导入。
fn render_tab_ini(items: &[FileItem], opts: &ExportOptions) -> String {
    let cols = opts.effective_columns();
    let mut out = String::new();
    let _ = writeln!(out, "[Liste]");
    let header: Vec<&str> = std::iter::once("Index")
        .chain(cols.iter().map(|c| c.key()))
        .collect();
    let _ = writeln!(out, "{}", header.join("\t"));
    for (i, it) in items.iter().enumerate() {
        let mut line: Vec<String> = vec![(i + 1).to_string()];
        for c in cols {
            line.push(cell_text(*c, it));
        }
        let _ = writeln!(out, "{}", line.join("\t"));
    }
    out
}

// ============ 模板变量 ============

/// 展开导出内容里的模板变量。
///
/// 实测原版变量：`{Q-DIR-FILE-PATH}`（`.rdata` 0x131bc8）。
/// 逐份导出（每文件一个输出）时由调用方对每个路径各展开一次。
pub fn expand_template(template: &str, file_path: &str) -> String {
    template.replace("{Q-DIR-FILE-PATH}", file_path)
}

// ============ 写出 ============

/// 渲染并写文件（UTF-8，可选 BOM）。
///
/// 父目录不存在时创建；写失败返回 [`QError::Io`] 并保留 OS 错误码（C3）。
pub fn write_export(
    path: &Path,
    items: &[FileItem],
    format: crate::core::ExportFormat,
    opts: &ExportOptions,
) -> QResult<()> {
    let text = render_items(items, format, opts);
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if opts.utf8_bom {
        bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    bytes.extend_from_slice(text.as_bytes());
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| QError::from_io(&e))?;
        }
    }
    fs::write(path, &bytes).map_err(|e| QError::from_io(&e))
}

/// 便捷入口：导出到内存字节（含 BOM 决策），供剪贴板/拖放复用。
pub fn export_bytes(
    items: &[FileItem],
    format: crate::core::ExportFormat,
    opts: &ExportOptions,
) -> Vec<u8> {
    let text = render_items(items, format, opts);
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if opts.utf8_bom {
        bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ExportFormat, FileItem};
    use std::path::PathBuf;

    fn item(name: &str, is_dir: bool, size: u64, modified: u64) -> FileItem {
        FileItem {
            path: PathBuf::from(format!("C:\\data\\{name}")),
            name: name.to_string(),
            is_dir,
            is_hidden: false,
            size,
            modified,
            icon_index: None,
        }
    }

    fn sample() -> Vec<FileItem> {
        vec![
            item("docs", true, 0, 0),
            item("a.zip", false, 1024, 132_000_000_000_000_000),
            // 文件名里故意放分隔符/引号，验证转义。
            item("we;ird\".txt", false, 7, 116_444_736_000_000_000),
        ]
    }

    fn opts(cols: Vec<ExportColumn>) -> ExportOptions {
        ExportOptions {
            columns: cols,
            utf8_bom: true,
            title: "T".to_string(),
        }
    }

    #[test]
    fn txt_columns_aligned_and_tab_separated() {
        let out = render_txt(&sample(), &opts(vec![ExportColumn::Name, ExportColumn::Size]));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4);
        // 表头按列内容对齐：最长名字是 "we;ird\".txt"（12 字符）。
        assert_eq!(lines[0], "Name       \tSize");
        // 目录 Size 为空字符串（仍按 Size 列宽补齐）。
        assert_eq!(lines[1], "docs       \t    ");
        assert_eq!(lines[2], "a.zip      \t1024");
        assert_eq!(lines[3], "we;ird\".txt\t7   ");
    }

    #[test]
    fn csv_escapes_separator_and_quotes() {
        let out = render_csv(&sample(), &opts(vec![ExportColumn::Name]));
        assert_eq!(out.lines().next(), Some("Name"));
        // 含 ';' 与 '"'：整字段加引号且引号翻倍。
        assert_eq!(out.lines().nth(3), Some("\"we;ird\"\".txt\""));
        // 普通字段不加引号。
        assert_eq!(out.lines().nth(1), Some("docs"));
    }

    #[test]
    fn html_structure_and_escaping() {
        let out = render_html(&sample(), &opts(vec![ExportColumn::Name, ExportColumn::Size]));
        assert!(out.starts_with("<!DOCTYPE html>"));
        assert!(out.contains("<TABLE Border=0>"));
        assert!(out.contains("<TR class=\"b_gelb\">"));
        // 交替行 class 出现（3 行 -> 至少一个 bg_grau）。
        assert!(out.contains("class=\"bg_grau\""));
        assert!(out.ends_with("</body></html>\n"));
        // 特殊字符必须转义：'"' 变成 &quot;，不允许出现在单元格文本里。
        assert!(out.contains("we;ird&quot;.txt"));
        assert!(!out.contains("<td>we;ird\""));
    }

    #[test]
    fn xls_is_spreadsheetml_with_number_cells() {
        let out = render_xls(&sample(), &opts(vec![ExportColumn::Name, ExportColumn::Size]));
        assert!(out.contains("<?mso-application progid=\"Excel.Sheet\"?>"));
        assert!(out.contains("urn:schemas-microsoft-com:office:spreadsheet"));
        assert!(out.contains("ss:Type=\"Number\">1024"));
        // 目录不出数字单元格。
        assert!(out.contains("<Cell><Data ss:Type=\"String\">docs</Data></Cell>"));
        // 转义生效。
        assert!(out.contains("we;ird&quot;.txt"));
    }

    #[test]
    fn tab_separated_ini_has_section_and_index() {
        let out = render_tab_ini(&sample(), &opts(vec![ExportColumn::Name]));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "[Liste]");
        assert_eq!(lines[1], "Index\tName");
        assert_eq!(lines[2], "1\tdocs");
        assert_eq!(lines[4], "3\twe;ird\".txt");
    }

    #[test]
    fn bom_prepend_and_write_roundtrip() {
        let dir = std::env::temp_dir().join("qdir-rs-export-test");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("out.csv");
        write_export(&path, &sample(), ExportFormat::Csv, &opts(vec![ExportColumn::Name]))
            .expect("写入临时文件");
        let bytes = fs::read(&path).expect("读回");
        assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF], "必须带 UTF-8 BOM");
        let text = String::from_utf8(bytes[3..].to_vec()).expect("BOM 后是合法 UTF-8");
        assert!(text.starts_with("Name\r\n") || text.starts_with("Name\n"));

        // 关闭 BOM 时不写三字节头。
        let path2 = dir.join("nobom.txt");
        let mut o = opts(vec![ExportColumn::Name]);
        o.utf8_bom = false;
        write_export(&path2, &sample(), ExportFormat::Txt, &o).expect("写入");
        let b2 = fs::read(&path2).expect("读回");
        assert_ne!(&b2[..3], &[0xEF, 0xBB, 0xBF]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_options_when_columns_empty() {
        // 空列配置必须回落而不是 panic 或空表。
        let out = render_csv(&sample(), &opts(Vec::new()));
        assert_eq!(out.lines().next(), Some("Name;Size;Modified"));
    }

    #[test]
    fn filetime_formatting() {
        assert_eq!(format_filetime(0), "");
        // 1970-01-01 00:00:00（unix 纪元）。
        assert_eq!(
            format_filetime(116_444_736_000_000_000),
            "1970-01-01 00:00:00"
        );
        // 2024-01-02 00:24:05 UTC
        let ft = (1_704_155_045u64) * 10_000_000 + 116_444_736_000_000_000;
        assert_eq!(format_filetime(ft), "2024-01-02 00:24:05");
        // 任意小值（含 1）不得 panic：saturating_sub 已保证。
        let _ = format_filetime(1);
    }

    #[test]
    fn column_key_roundtrip() {
        for c in ExportColumn::ALL {
            assert_eq!(ExportColumn::from_key(c.key()), Some(c));
        }
        assert_eq!(ExportColumn::from_key("Nope"), None);
        let s = ExportOptions::columns_to_string(&opts(vec![
            ExportColumn::Name,
            ExportColumn::Path,
        ]));
        assert_eq!(s, "Name;Path");
        assert_eq!(
            ExportOptions::columns_from_string("Name;Bogus;Size"),
            vec![ExportColumn::Name, ExportColumn::Size]
        );
        // 全部未知 -> 默认列。
        assert_eq!(ExportOptions::columns_from_string("X;Y").len(), 3);
    }

    #[test]
    fn template_variable_expansion() {
        assert_eq!(
            expand_template("path={Q-DIR-FILE-PATH}", "C:\\a\\b.txt"),
            "path=C:\\a\\b.txt"
        );
        // 无变量时原样返回。
        assert_eq!(expand_template("plain", "x"), "plain");
    }

    #[test]
    fn render_dispatch_matches_each_format_ext() {
        // 与 core::ExportFormat::ext 对齐（CSV->csv 等）。
        let o = ExportOptions::default();
        assert!(render_items(&sample(), ExportFormat::Txt, &o).contains("Name"));
        assert!(render_items(&sample(), ExportFormat::Csv, &o).contains("Name"));
        assert!(render_items(&sample(), ExportFormat::Html, &o).contains("<TABLE"));
        assert!(render_items(&sample(), ExportFormat::Xls, &o).contains("<Workbook"));
        assert!(render_items(&sample(), ExportFormat::TabSeparatedIni, &o).contains("[Liste]"));
        assert_eq!(ExportFormat::TabSeparatedIni.ext(), "ini");
    }
}
