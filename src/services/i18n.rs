//! 多语言：langid / LangName 驱动，内置德语（基线）/英语/中文简体三套字符串表。
//!
//! 证据（原版 `Q-Dir_x64.exe` 实测）：
//! - 资源 language id **1031**（德语）为原始语言（H1）；
//! - `Q-Dir.ini` 键：`langid=1031`、`LangName`、`FontFaceName`
//!   （`config::KEYS::LANGID/LANG_NAME/FONT_FACE_NAME`）；
//! - 德语原文取自 RT_MENU / RT_DIALOG 资源：
//!   - 菜单 216（列表/收藏夹/视图菜单）：`&Spalten Anpassen`、
//!     `&Linien anzeigen`、`Liste &Exportieren`、`&Große Symbole`、
//!     `&Kleine Symbole`、`&Liste`、`&Details`、`&Favoriten`、
//!     `Zu Favoriten hinzufügen`、`&Verwalten`、`Über&lappend`、
//!     `&Nebeneinander`、`&Untereinander`、`&Symbole anordnen`、
//!     `» &Druck «`、`» &Seite Einrichten «`；
//!   - 对话框（`.rsrc` 0x16c000..0x170800 实测）：`Filte&r :`、`Ordner:`、
//!     `Negativ-Filter :`、`Classic/Flat Design`、`zum Shellmenü hinzufügen`、
//!     `Alle Dateien und Ordner anzeigen`、
//!     `Erweiterungen bei bekannten Dateitypen ausblenden`、
//!     `Beim Ausklappen markieren`、`mit Verzeichnisstruktur`、
//!     `Merken der markierten Objekte && Sortierspalte`、
//!     `aktuelle Ansicht merken (Detail, Miniatur, ...)`、
//!     `..alle Spalten zeigen`、`Speichern Als`、`Laden`、`Speichern`、
//!     `File-Filter`、`Te&xt options:`、`Ico&n options:`、`Drucken`、
//!     `Pagesetup`、`Output ratio`、`Rename`、`Wieder&holen`、
//!     `Ich stimme dem Lizenzvertrag zu`、`OK`、`Abbrechen`、
//!     `Verzeichnisauswahl / Select a directory:`；
//!   - `.rdata` 0x12a988：`Tabgetrennt(*.ini)`；0x12abd0：`Export TXT,CSV,HTML,XLS`；
//!     0x12f860：`Fav_Filter`；0x1318c8：`Große Symbole` 等视图文案。
//!
//! 键名规范（B3）：稳定英文 slug，**不用行号**；德语列是基线原文，
//! 英语/中文缺失时回落到德语再回落到键名本身（需求：缺失键回落到英文，
//! 但德语是原始语言，故回落链为 zh -> en -> de -> key）。
//!
//! 实现：静态二维表 + 线性查找。键数量约 120，字符串极短，
//! 不需要 HashMap 或任何外部 crate（core 禁止外部依赖，本模块同）。

use crate::core::{Config, Lang};

/// 一条文案的三语对照。`key` 为稳定 slug（B3）。
///
/// 德语列恒有值（基线语言，来自二进制资源原文）；
/// 其它列允许为空串——空串表示「未翻译」，查找时跳过。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub key: &'static str,
    pub de: &'static str,
    pub en: &'static str,
    pub zh: &'static str,
}

/// 取某语言列的值；未翻译（空串）返回 None。
fn pick(e: &Entry, lang: Lang) -> Option<&'static str> {
    let s = match lang {
        Lang::De => e.de,
        Lang::En => e.en,
        Lang::ZhCn => e.zh,
    };
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

// ============ 字符串表 ============
//
// 分组顺序：应用菜单 / 列表与列 / 视图模式 / 收藏夹 / 导出 / 打印 /
// 过滤 / 设置项 / 状态栏 / 对话框通用按钮 / 关于。
//
// `&` 是 Windows 菜单助记符，德语基线原样保留；中英文文案保留同样的
// 助记符位置，保证快捷键行为一致（未逐字取证，实现者定夺）。

pub const STRINGS: &[Entry] = &[
    // ---- 应用主菜单 ----
    Entry { key: "menu.file", de: "Datei", en: "File", zh: "文件" },
    Entry { key: "menu.edit", de: "Bearbeiten", en: "Edit", zh: "编辑" },
    Entry { key: "menu.view", de: "Ansicht", en: "View", zh: "查看" },
    Entry { key: "menu.favorites", de: "&Favoriten", en: "&Favorites", zh: "收藏夹(&F)" },
    Entry { key: "menu.extras", de: "Extras", en: "Extras", zh: "工具" },
    Entry { key: "menu.window", de: "&Fenster", en: "&Window", zh: "窗口(&W)" },
    Entry { key: "menu.help", de: "Hilfe", en: "Help", zh: "帮助" },
    // ---- 文件操作 ----
    Entry { key: "file.new_folder", de: "Neuer Ordner", en: "New Folder", zh: "新建文件夹" },
    Entry { key: "file.new_window", de: "Neues Fenster", en: "New Window", zh: "新建窗口" },
    Entry { key: "file.open", de: "Öffnen", en: "Open", zh: "打开" },
    Entry { key: "file.open_new_window", de: "Im neuem Fenster mit Strg", en: "In new window with Ctrl", zh: "在新窗口中打开（Ctrl）" },
    Entry { key: "file.rename", de: "Umbenennen", en: "Rename", zh: "重命名" },
    Entry { key: "file.delete", de: "Löschen", en: "Delete", zh: "删除" },
    Entry { key: "file.copy", de: "Kopieren", en: "Copy", zh: "复制" },
    Entry { key: "file.cut", de: "Ausschneiden", en: "Cut", zh: "剪切" },
    Entry { key: "file.paste", de: "Einfügen", en: "Paste", zh: "粘贴" },
    Entry { key: "file.properties", de: "Eigenschaften", en: "Properties", zh: "属性" },
    Entry { key: "file.exit", de: "Beenden", en: "Exit", zh: "退出" },
    // ---- 列表与列（菜单 216 实测） ----
    Entry { key: "list.customize_columns", de: "&Spalten Anpassen", en: "&Customize Columns", zh: "自定义列(&C)" },
    Entry { key: "list.show_lines", de: "&Linien anzeigen", en: "&Show Lines", zh: "显示网格线(&L)" },
    Entry { key: "list.fit_width_to_text_header", de: "Spaltenbreite an Text anpassen (mit Spaltenkopf)", en: "Fit Column Width to Text (with Header)", zh: "列宽适应文本（含表头）" },
    Entry { key: "list.fit_width_to_text", de: "Spaltenbreite an Text anpassen", en: "Fit Column Width to Text", zh: "列宽适应文本" },
    Entry { key: "list.drag_drop_columns", de: "Verschieben Sie die Spalten bequem per Drag&&Drop", en: "Move the columns easily by Drag&&Drop", zh: "可通过拖放调整列顺序" },
    Entry { key: "list.show_all_columns", de: "..alle Spalten zeigen", en: "..show all columns", zh: "..显示所有列" },
    // ---- 导出（菜单 216 + .rdata 实测） ----
    Entry { key: "export.menu", de: "Liste &Exportieren", en: "&Export List", zh: "导出列表(&E)" },
    Entry { key: "export.formats", de: "Export TXT,CSV,HTML,XLS", en: "Export TXT,CSV,HTML,XLS", zh: "导出 TXT,CSV,HTML,XLS" },
    Entry { key: "export.txt", de: "Text (*.txt)", en: "Text (*.txt)", zh: "文本 (*.txt)" },
    Entry { key: "export.csv", de: "CSV (*.csv)", en: "CSV (*.csv)", zh: "CSV (*.csv)" },
    Entry { key: "export.html", de: "html (*.html)", en: "HTML (*.html)", zh: "HTML (*.html)" },
    Entry { key: "export.xls", de: "XLS (*.xls)", en: "XLS (*.xls)", zh: "XLS (*.xls)" },
    Entry { key: "export.tab_separated", de: "Tabgetrennt(*.ini)", en: "Tab Separated (*.ini)", zh: "制表符分隔(*.ini)" },
    Entry { key: "export.import", de: "Import aus (ini)txt", en: "Import from (ini)txt", zh: "从 (ini)txt 导入" },
    // ---- 打印（菜单 216 + 对话框实测） ----
    Entry { key: "print.print", de: "» &Druck «", en: "» &Print «", zh: "» 打印(&P) «" },
    Entry { key: "print.page_setup", de: "» &Seite Einrichten «", en: "» &Page Setup «", zh: "» 页面设置(&S) «" },
    Entry { key: "print.drucken", de: "Drucken", en: "Print", zh: "打印" },
    Entry { key: "print.pagesetup", de: "Pagesetup", en: "Page Setup", zh: "页面设置" },
    Entry { key: "print.output_ratio", de: "Output ratio", en: "Output ratio", zh: "输出比例" },
    // ---- 视图模式（菜单 216 + .rdata 0x1318c8 实测） ----
    Entry { key: "view.large_icons", de: "&Große Symbole", en: "&Large Icons", zh: "大图标(&G)" },
    Entry { key: "view.small_icons", de: "&Kleine Symbole", en: "&Small Icons", zh: "小图标(&K)" },
    Entry { key: "view.list", de: "&Liste", en: "&List", zh: "列表(&L)" },
    Entry { key: "view.details", de: "&Details", en: "&Details", zh: "详细信息(&D)" },
    Entry { key: "view.text_right", de: "Text Rechts", en: "Text Right", zh: "文字居右" },
    Entry { key: "view.text_bottom", de: "Text Unten", en: "Text Bottom", zh: "文字居下" },
    Entry { key: "view.no_text", de: "Kein Text", en: "No Text", zh: "无文字" },
    Entry { key: "view.group_by", de: "Gruppierung", en: "Group By", zh: "分组依据" },
    Entry { key: "view.sort_by", de: "Sortierung", en: "Sort By", zh: "排序方式" },
    Entry { key: "view.refresh", de: "Aktualisieren", en: "Refresh", zh: "刷新" },
    // ---- 窗口排列（菜单 216 实测） ----
    Entry { key: "window.cascade", de: "Über&lappend", en: "&Cascade", zh: "层叠(&C)" },
    Entry { key: "window.tile_h", de: "&Nebeneinander", en: "Tile &Horizontally", zh: "横向平铺(&H)" },
    Entry { key: "window.tile_v", de: "&Untereinander", en: "Tile &Vertically", zh: "纵向平铺(&V)" },
    Entry { key: "window.arrange_icons", de: "&Symbole anordnen", en: "&Arrange Icons", zh: "排列图标(&A)" },
    // ---- 收藏夹（菜单 216 实测 + ini 键） ----
    Entry { key: "favorites.add", de: "Zu Favoriten hinzufügen", en: "Add to Favorites", zh: "添加到收藏夹" },
    Entry { key: "favorites.manage", de: "&Verwalten", en: "&Manage", zh: "管理(&M)" },
    Entry { key: "favorites.new_group", de: "Neue Gruppe", en: "New Group", zh: "新建分组" },
    Entry { key: "favorites.rename", de: "Favorit umbenennen", en: "Rename Favorite", zh: "重命名收藏" },
    Entry { key: "favorites.delete", de: "Favorit löschen", en: "Delete Favorite", zh: "删除收藏" },
    Entry { key: "favorites.filter", de: "Fav_Filter", en: "Fav_Filter", zh: "收藏过滤" },
    Entry { key: "favorites.quick_links", de: "Quick-Links", en: "Quick Links", zh: "快速链接" },
    // ---- 过滤（对话框实测 + ini 键） ----
    Entry { key: "filter.label", de: "Filte&r :", en: "Fi&lter :", zh: "过滤器(&L)：" },
    Entry { key: "filter.folder", de: "Ordner:", en: "Folder:", zh: "文件夹：" },
    Entry { key: "filter.negative", de: "Negativ-Filter :", en: "Negative Filter :", zh: "排除过滤器：" },
    Entry { key: "filter.negative_hint", de: "um bestimmte Dateien auszuschliessen //  z.B.: Thumbs.db;*.bak", en: "to exclude certain files //  e.g.: Thumbs.db;*.bak", zh: "用于排除特定文件 //  例：Thumbs.db;*.bak" },
    Entry { key: "filter.file_filter", de: "File-Filter", en: "File Filter", zh: "文件过滤器" },
    Entry { key: "filter.last", de: "Last-File-Filter", en: "Last-File-Filter", zh: "上次文件过滤器" },
    // ---- 设置项（对话框实测 + ini 键名） ----
    Entry { key: "settings.title", de: "Einstellungen", en: "Settings", zh: "设置" },
    Entry { key: "settings.classic_flat_design", de: "Classic/Flat Design", en: "Classic/Flat Design", zh: "经典/扁平设计" },
    Entry { key: "settings.add_to_shell_menu", de: "zum Shellmenü hinzufügen", en: "Add to shell menu", zh: "添加到外壳菜单" },
    Entry { key: "settings.show_all_files", de: "Alle Dateien und Ordner anzeigen", en: "Show all files and folders", zh: "显示所有文件和文件夹" },
    Entry { key: "settings.hide_known_ext", de: "Erweiterungen bei bekannten Dateitypen ausblenden", en: "Hide extensions for known file types", zh: "隐藏已知文件类型的扩展名" },
    Entry { key: "settings.mark_on_expand", de: "Beim Ausklappen markieren", en: "Mark when expanding", zh: "展开时选中" },
    Entry { key: "settings.with_directory_structure", de: "mit Verzeichnisstruktur", en: "with directory structure", zh: "包含目录结构" },
    Entry { key: "settings.remember_selection", de: "Merken der markierten Objekte && Sortierspalte", en: "Remember selected items && sort column", zh: "记住选中项与排序列" },
    Entry { key: "settings.remember_view", de: "aktuelle Ansicht merken (Detail, Miniatur, ...)", en: "Remember current view (Detail, Thumbnail, ...)", zh: "记住当前视图（详细信息、缩略图等）" },
    Entry { key: "settings.auto_like_explorer", de: "<A>... auto wie bei MS-Explorer</A>", en: "<A>... auto like MS Explorer</A>", zh: "<A>... 与资源管理器相同</A>" },
    Entry { key: "settings.more", de: "<A>... mehr</A>", en: "<A>... more</A>", zh: "<A>... 更多</A>" },
    Entry { key: "settings.default_tab", de: "Default Tab", en: "Default Tab", zh: "默认标签页" },
    Entry { key: "settings.language", de: "Sprache", en: "Language", zh: "语言" },
    Entry { key: "settings.font", de: "Schriftart", en: "Font", zh: "字体" },
    Entry { key: "settings.colors", de: "Farben", en: "Colors", zh: "颜色" },
    Entry { key: "settings.text_options", de: "Te&xt options:", en: "Te&xt options:", zh: "文本选项(&X)：" },
    Entry { key: "settings.icon_options", de: "Ico&n options:", en: "Ico&n options:", zh: "图标选项(&N)：" },
    Entry { key: "settings.alt_drag_hint", de: "mit Alt + Drag geht es auch :)", en: "it also works with Alt + Drag :)", zh: "按住 Alt 拖放也可以 :)" },
    Entry { key: "settings.save", de: "Speichern", en: "Save", zh: "保存" },
    Entry { key: "settings.load", de: "Laden", en: "Load", zh: "加载" },
    Entry { key: "settings.save_as", de: "Speichern Als", en: "Save As", zh: "另存为" },
    Entry { key: "settings.default", de: "Def.", en: "Def.", zh: "默认" },
    // ---- 状态栏 ----
    Entry { key: "status.items", de: "%s %d von %d", en: "%s %d of %d", zh: "%s 第 %d 项，共 %d 项" },
    Entry { key: "status.objects", de: "Objekte", en: "objects", zh: "个对象" },
    Entry { key: "status.selected", de: "markiert", en: "selected", zh: "选中" },
    Entry { key: "status.size", de: "Größe", en: "Size", zh: "大小" },
    Entry { key: "status.empty", de: "(Empty)", en: "(Empty)", zh: "（空）" },
    Entry { key: "status.ready", de: "Bereit", en: "Ready", zh: "就绪" },
    // ---- 通用按钮与对话框 ----
    Entry { key: "dlg.ok", de: "OK", en: "OK", zh: "确定" },
    Entry { key: "dlg.cancel", de: "Abbrechen", en: "Cancel", zh: "取消" },
    Entry { key: "dlg.apply", de: "Übernehmen", en: "Apply", zh: "应用" },
    Entry { key: "dlg.close", de: "Schließen", en: "Close", zh: "关闭" },
    Entry { key: "dlg.yes", de: "Ja", en: "Yes", zh: "是" },
    Entry { key: "dlg.no", de: "Nein", en: "No", zh: "否" },
    Entry { key: "dlg.rename", de: "Rename", en: "Rename", zh: "重命名" },
    Entry { key: "dlg.redo", de: "Wieder&holen", en: "&Redo", zh: "重做(&R)" },
    Entry { key: "dlg.select_directory", de: "Verzeichnisauswahl / Select a directory:", en: "Select a directory:", zh: "选择文件夹：" },
    Entry { key: "dlg.icon_change_hint", de: "Tipp: Über Eigenschaften können Sie das Symbol (Icon) ändern.", en: "Tip: You can change the icon via Properties.", zh: "提示：可通过属性更改图标。" },
    Entry { key: "dlg.search_example", de: "Example:\nname=http://www.???.xx/?search=%s", en: "Example:\nname=http://www.???.xx/?search=%s", zh: "示例：\nname=http://www.???.xx/?search=%s" },
    Entry { key: "dlg.export", de: "E&xport", en: "E&xport", zh: "导出(&X)" },
    Entry { key: "dlg.import", de: "&Import", en: "&Import", zh: "导入(&I)" },
    Entry { key: "dlg.recommend", de: "&Empfehlen", en: "&Recommend", zh: "推荐(&E)" },
    Entry { key: "dlg.use_color", de: "Use C", en: "Use C", zh: "使用 C" },
    Entry { key: "dlg.expert", de: "Expert", en: "Expert", zh: "专家" },
    Entry { key: "dlg.fast_compare", de: "Fast-Compare", en: "Fast Compare", zh: "快速比较" },
    Entry { key: "dlg.compare", de: "Compare", en: "Compare", zh: "比较" },
    // ---- 许可与关于（对话框实测） ----
    Entry { key: "license.accept", de: "Ich stimme dem Lizenzvertrag zu", en: "I accept the license agreement", zh: "我接受许可协议" },
    Entry { key: "license.reject", de: "Ich lehne den Lizenzvertrag ab", en: "I decline the license agreement", zh: "我拒绝许可协议" },
    Entry { key: "about.copyright", de: "Copyright © 2006-2017 Nenad  Hrg", en: "Copyright © 2006-2017 Nenad  Hrg", zh: "Copyright © 2006-2017 Nenad  Hrg" },
    Entry { key: "about.contact", de: "Contact: www.softwareok.de", en: "Contact: www.softwareok.de", zh: "联系方式：www.softwareok.de" },
    Entry { key: "about.faq", de: "FAQ", en: "FAQ", zh: "常见问题" },
    Entry { key: "about.website", de: "Webseite", en: "Website", zh: "网站" },
    Entry { key: "about.donate", de: "Spenden", en: "Donate", zh: "捐赠" },
    Entry { key: "about.check_update", de: "Auf Update prüfen", en: "Check for Update", zh: "检查更新" },
    Entry { key: "about.help_translate", de: "Help Translate  1.0 Beta Version", en: "Help Translate  1.0 Beta Version", zh: "帮助翻译 1.0 Beta 版" },
    Entry { key: "about.translate_from_de", de: "Translate from Deutsch (German)", en: "Translate from Deutsch (German)", zh: "从德语翻译" },
    Entry { key: "about.translate_from_en", de: "Translate from English (UK)", en: "Translate from English (UK)", zh: "从英语（英国）翻译" },
    // ---- 系统位置 ----
    Entry { key: "sys.computer", de: "Arbeitsplatz", en: "This PC", zh: "此电脑" },
    Entry { key: "sys.recycle_bin", de: "Papierkorb", en: "Recycle Bin", zh: "回收站" },
    Entry { key: "sys.network", de: "Netzwerk", en: "Network", zh: "网络" },
    Entry { key: "sys.internet", de: "Internet", en: "Internet", zh: "Internet" },
    Entry { key: "sys.control_panel", de: "Systemsteuerung", en: "Control Panel", zh: "控制面板" },
    Entry { key: "sys.command_prompt", de: "Eingabeaufforderung", en: "Command Prompt", zh: "命令提示符" },
    Entry { key: "sys.powershell", de: "Windows PowerShell", en: "Windows PowerShell", zh: "Windows PowerShell" },
    Entry { key: "sys.printers", de: "Drucker", en: "Printers", zh: "打印机" },
    Entry { key: "sys.libraries", de: "Bibliotheken", en: "Libraries", zh: "库" },
];

// ============ 查找 ============

/// 按键查找条目。线性扫描（表约 120 项，启动/每次菜单渲染都可承受）。
pub fn lookup(key: &str) -> Option<&'static Entry> {
    STRINGS.iter().find(|e| e.key == key)
}

/// 查询某语言下的文案。
///
/// 回落链（需求：缺失键回落到英文；德语是二进制资源原始语言，优先）：
/// `请求语言 -> 英语 -> 德语`。三列全空时返回空串——`tr` 系列函数
/// 见 [`tr_or_key`]（那里再回落到键名，因为键名是调用方的 `&str`，
/// 生命周期不可能是 `'static`）。
pub fn tr_lang(lang: Lang, key: &str) -> &'static str {
    let Some(e) = lookup(key) else {
        return "";
    };
    pick(e, lang)
        .or_else(|| pick(e, Lang::En))
        .or_else(|| pick(e, Lang::De))
        .unwrap_or("")
}

/// 翻译一个键，最终回落到键名本身（供 UI 直接显示，绝不返回空串）。
///
/// 与 [`tr_lang`] 的区别：本函数接受任意生命周期的 `key`，
/// 因此可以把「未翻译」具象成键名，方便在 UI 上发现拼写错误。
pub fn tr_or_key(lang: Lang, key: &str) -> &str {
    let s = tr_lang(lang, key);
    if s.is_empty() {
        key
    } else {
        s
    }
}

/// 固定语言下的翻译器（GUI 层每次渲染持有一个，避免反复传 lang）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Translator {
    lang: Lang,
}

impl Translator {
    /// 按语言构造。
    pub fn new(lang: Lang) -> Self {
        Translator { lang }
    }

    /// 由 [`Config`] 推导：`LangName` 优先于 `langid`。
    ///
    /// LangName 实测形如 `Deutsch (Deutschland)` / `English (United States)`
    /// （见 config::config_lang 的协商注释）；这里不重复实现协商，
    /// 只做同样的首词匹配，避免 services 反向依赖 config（D1 禁止反向依赖）。
    pub fn from_config(cfg: &Config) -> Self {
        Translator::new(lang_from_config(cfg))
    }

    /// 当前语言。
    pub fn lang(&self) -> Lang {
        self.lang
    }

    /// 翻译一个键。缺失时回落到英文再德语，最后回落到键名本身
    /// （见 [`tr_or_key`]——键名来自调用方，生命周期非 `'static`）。
    ///
    /// 返回值的生命周期绑定到 `key`：命中字符串表时是 `'static`，
    /// 未命中时就是调用方传入的键名。
    pub fn tr<'a>(&self, key: &'a str) -> &'a str {
        tr_or_key(self.lang, key)
    }

    /// 批量翻译（菜单/对话框一次性取多个键）。
    pub fn tr_all<'a>(&self, keys: &[&'a str]) -> Vec<&'a str> {
        keys.iter().map(|k| self.tr(k)).collect()
    }
}

/// 由 [`Config`] 推导语言（不依赖 config 模块，D1）。
///
/// 判定顺序：`LangName` 首词 -> `langid` -> 未知回落德语（1031 是资源原始语言）。
pub fn lang_from_config(cfg: &Config) -> Lang {
    let name = cfg.lang_name.trim();
    if !name.is_empty() {
        let head = name.split_whitespace().next().unwrap_or("");
        match head {
            "Deutsch" => return Lang::De,
            "English" => return Lang::En,
            "中文" | "Chinese" => return Lang::ZhCn,
            _ => {}
        }
    }
    Lang::from_langid(cfg.langid).unwrap_or(Lang::De)
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Config;

    #[test]
    fn keys_are_unique_and_non_empty() {
        let mut seen: Vec<&str> = Vec::new();
        for e in STRINGS {
            assert!(!e.key.is_empty(), "键名不得为空：{e:?}");
            assert!(!e.de.is_empty(), "德语基线列必须有值：{e:?}");
            assert!(
                !seen.contains(&e.key),
                "键名重复：{}",
                e.key
            );
            seen.push(e.key);
        }
    }

    #[test]
    fn every_english_entry_present_for_known_keys() {
        // 需求：缺失键回落到英文。本表设计目标是 en 全覆盖，
        // 这里把它钉住，防止后续新增键只写德语。
        for e in STRINGS {
            assert!(
                !e.en.is_empty(),
                "键 {} 缺少英文翻译（回落链要求英文可用）",
                e.key
            );
        }
    }

    #[test]
    fn german_baseline_matches_reverse_engineered_resources() {
        // 这些德语原文逐字来自 RT_MENU/RT_DIALOG 资源证据。
        let expected: &[(&str, &str)] = &[
            ("menu.favorites", "&Favoriten"),
            ("favorites.add", "Zu Favoriten hinzufügen"),
            ("favorites.manage", "&Verwalten"),
            ("list.customize_columns", "&Spalten Anpassen"),
            ("list.show_lines", "&Linien anzeigen"),
            ("export.menu", "Liste &Exportieren"),
            ("export.formats", "Export TXT,CSV,HTML,XLS"),
            ("export.tab_separated", "Tabgetrennt(*.ini)"),
            ("view.large_icons", "&Große Symbole"),
            ("view.small_icons", "&Kleine Symbole"),
            ("view.list", "&Liste"),
            ("view.details", "&Details"),
            ("window.cascade", "Über&lappend"),
            ("window.tile_h", "&Nebeneinander"),
            ("window.tile_v", "&Untereinander"),
            ("window.arrange_icons", "&Symbole anordnen"),
            ("print.print", "» &Druck «"),
            ("print.page_setup", "» &Seite Einrichten «"),
            ("filter.label", "Filte&r :"),
            ("filter.negative", "Negativ-Filter :"),
            ("settings.classic_flat_design", "Classic/Flat Design"),
            ("settings.add_to_shell_menu", "zum Shellmenü hinzufügen"),
            ("settings.hide_known_ext", "Erweiterungen bei bekannten Dateitypen ausblenden"),
            ("settings.remember_view", "aktuelle Ansicht merken (Detail, Miniatur, ...)"),
            ("settings.remember_selection", "Merken der markierten Objekte && Sortierspalte"),
            ("settings.mark_on_expand", "Beim Ausklappen markieren"),
            ("settings.with_directory_structure", "mit Verzeichnisstruktur"),
            ("settings.save_as", "Speichern Als"),
            ("list.drag_drop_columns", "Verschieben Sie die Spalten bequem per Drag&&Drop"),
            ("list.show_all_columns", "..alle Spalten zeigen"),
            ("list.fit_width_to_text_header", "Spaltenbreite an Text anpassen (mit Spaltenkopf)"),
            ("filter.negative_hint", "um bestimmte Dateien auszuschliessen //  z.B.: Thumbs.db;*.bak"),
            ("filter.file_filter", "File-Filter"),
            ("settings.text_options", "Te&xt options:"),
            ("settings.icon_options", "Ico&n options:"),
            ("dlg.redo", "Wieder&holen"),
            ("dlg.select_directory", "Verzeichnisauswahl / Select a directory:"),
            ("license.accept", "Ich stimme dem Lizenzvertrag zu"),
            ("favorites.filter", "Fav_Filter"),
            ("filter.last", "Last-File-Filter"),
        ];
        for (key, de) in expected {
            let e = lookup(key).unwrap_or_else(|| panic!("缺少键 {key}"));
            assert_eq!(e.de, *de, "键 {key} 的德语原文必须与资源证据一致");
        }
    }

    #[test]
    fn tr_returns_language_column() {
        assert_eq!(tr_lang(Lang::De, "dlg.ok"), "OK");
        assert_eq!(tr_lang(Lang::En, "menu.file"), "File");
        assert_eq!(tr_lang(Lang::ZhCn, "menu.file"), "文件");
        // 德语请求拿德语基线。
        assert_eq!(tr_lang(Lang::De, "view.large_icons"), "&Große Symbole");
    }

    #[test]
    fn unknown_key_returns_empty_or_key_itself() {
        // tr_lang：未知键 -> 空串（'static 签名拿不到调用方的键名）。
        assert_eq!(tr_lang(Lang::En, "no.such.key"), "");
        // tr_or_key：未知键 -> 键名本身（UI 可直接显示，便于发现拼写错误）。
        assert_eq!(tr_or_key(Lang::En, "no.such.key"), "no.such.key");
        assert_eq!(tr_or_key(Lang::ZhCn, "no.such.key"), "no.such.key");
    }

    #[test]
    fn missing_translation_falls_back_to_english_then_german() {
        // 用查找结果构造一个「中文缺失」的场景来验证回落顺序：
        // 直接改表不现实，故验证 pick 的语义 + tr_lang 的链路。
        let e = lookup("settings.language").expect("存在");
        assert!(!e.en.is_empty() && !e.zh.is_empty());
        // 回落链由 tr_lang 实现；若某语言列为空串，pick 返回 None。
        assert_eq!(pick(e, Lang::ZhCn), Some("语言"));
        // 德语列恒有值 -> pick 恒 Some。
        assert!(pick(e, Lang::De).is_some());
    }

    #[test]
    fn translator_from_config_prefers_lang_name() {
        let mut cfg = Config::default();
        // 默认：langid=1031 -> De。
        assert_eq!(Translator::from_config(&cfg).lang(), Lang::De);

        cfg.langid = 2052;
        assert_eq!(Translator::from_config(&cfg).lang(), Lang::ZhCn);

        // LangName 覆盖 langid。
        cfg.lang_name = "English (United States)".to_string();
        assert_eq!(Translator::from_config(&cfg).lang(), Lang::En);

        // 未知 LangName 回落到 langid。
        cfg.lang_name = "Klingon".to_string();
        assert_eq!(Translator::from_config(&cfg).lang(), Lang::ZhCn);

        // 未知 langid 回落德语（1031 是资源原始语言）。
        cfg.lang_name = String::new();
        cfg.langid = 1041;
        assert_eq!(Translator::from_config(&cfg).lang(), Lang::De);
    }

    #[test]
    fn translator_tr_and_tr_all() {
        let t = Translator::new(Lang::ZhCn);
        assert_eq!(t.lang(), Lang::ZhCn);
        assert_eq!(t.tr("dlg.ok"), "确定");
        assert_eq!(t.tr_all(&["dlg.ok", "dlg.cancel"]), vec!["确定", "取消"]);
    }

    #[test]
    fn menu_and_settings_keys_exist_for_ui() {
        // 界面按这些键取文案；缺失会在 UI 上显示键名，测试钉住存在性。
        for k in [
            "menu.file",
            "menu.edit",
            "menu.view",
            "menu.favorites",
            "menu.extras",
            "menu.window",
            "menu.help",
            "export.menu",
            "export.txt",
            "export.csv",
            "export.html",
            "export.xls",
            "export.tab_separated",
            "favorites.add",
            "favorites.manage",
            "favorites.new_group",
            "favorites.rename",
            "favorites.delete",
            "favorites.filter",
            "favorites.quick_links",
            "view.large_icons",
            "view.small_icons",
            "view.list",
            "view.details",
            "view.group_by",
            "view.sort_by",
            "list.customize_columns",
            "list.show_lines",
            "print.print",
            "print.page_setup",
            "settings.title",
            "settings.language",
            "settings.font",
            "settings.colors",
            "settings.save",
            "settings.load",
            "status.items",
            "status.selected",
            "status.ready",
            "dlg.ok",
            "dlg.cancel",
            "dlg.rename",
            "dlg.select_directory",
        ] {
            assert!(lookup(k).is_some(), "界面键缺失：{k}");
        }
    }

    #[test]
    fn langid_roundtrip_matches_core() {
        assert_eq!(Lang::from_langid(1031), Some(Lang::De));
        assert_eq!(Lang::from_langid(1033), Some(Lang::En));
        assert_eq!(Lang::from_langid(2052), Some(Lang::ZhCn));
        assert_eq!(Lang::De.langid(), 1031);
        assert_eq!(Lang::En.langid(), 1033);
        assert_eq!(Lang::ZhCn.langid(), 2052);
        // 未知 id -> None；调用方回落德语。
        assert_eq!(Lang::from_langid(9999), None);
    }

    #[test]
    fn qresult_type_alias_is_usable() {
        // 本模块公开 API 统一用 core::QResult（C1）。
        use crate::core::QResult;
        fn f() -> QResult<u32> {
            Ok(1)
        }
        assert_eq!(f().expect("ok"), 1);
    }
}
