//! core 类型回归测试：Filter2 实测 12 条规则、分割比例换算、视图状态串往返。
//!
//! 说明：`src/core.rs` 内部已有 `#[cfg(test)] mod tests` 覆盖同样内容，
//! 本文件作为集成测试再从 crate 公开 API 侧验证一遍（G1：全部无需 GUI /
//! 无需 Windows API，可在任何平台运行）。

#[test]
fn filter2_evidence_twelve_rules_parse_and_roundtrip() {
    // 来自 D:\Q-Dir_Portable_x64\Q-Dir\Q-Dir.ini 的 [Filter2] 段实测 12 行。
    let lines: Vec<String> = [
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
    .collect();

    let rs = qdir_rs::core::RuleSet::parse_ini_section(&lines).expect("12 行证据必须可解析");
    assert_eq!(rs.rules.len(), 12);

    // 逐行回写：除 rule 4（串内混用 ','，已登记差异）外必须逐字节一致。
    for rule in &rs.rules {
        let line = rule.to_ini_line();
        if rule.index == 4 {
            assert_eq!(
                line, "4=*.jpg;*.jpeg;*.png;*.gif;*.bmp;*.ico=1=BB00BB=-1=1=-1=0",
                "rule 4 回写统一为 ';'（语义等价）"
            );
            continue;
        }
        assert_eq!(line, lines[rule.index], "rule {} 回写必须逐字节一致", rule.index);
    }
}

#[test]
fn first_matching_rule_decides_color_even_when_default() {
    use qdir_rs::core::*;
    let lines: Vec<String> = [
        "0=#Hidden=1=-1=1=1=-1=0".to_string(),
        "3=#DIR=1=008800=-1=1=-1=0".to_string(),
        "9=*=1=0000BB=-1=1=-1=0".to_string(),
        "10=#BG=1=-1=-1=1=-1=0".to_string(),
        "11=#BG-A=1=-1=-1=1=-1=0".to_string(),
    ]
    .to_vec();
    let rs = RuleSet::parse_ini_section(&lines).expect("可解析");

    // F1：隐藏项命中 rule 0（颜色 -1）即停，不得继续落到 #DIR / 兜底。
    let hidden = FileItem {
        path: std::path::PathBuf::from(r"C:\x\.secret"),
        name: ".secret".into(),
        is_dir: false,
        is_hidden: true,
        size: 1,
        modified: 0,
        icon_index: None,
    };
    assert_eq!(rs.color_for(&hidden), None);

    let dir = FileItem {
        path: std::path::PathBuf::from(r"C:\x\docs"),
        name: "docs".into(),
        is_dir: true,
        is_hidden: false,
        size: 0,
        modified: 0,
        icon_index: None,
    };
    assert_eq!(rs.color_for(&dir), Some(Rgb(0x00, 0x88, 0x00)));
}

#[test]
fn split_permille_conversions_match_qdr_scale() {
    use qdir_rs::core::SplitPermille;
    // start.qdr 实测：SP0=5000 SP1=5003 SP2=5003 SPT=2000 VorschauPos=7500。
    assert_eq!(SplitPermille::from_qdr_ten_thousandths(5000).value(), 500);
    assert_eq!(SplitPermille::from_qdr_ten_thousandths(5003).value(), 500);
    assert_eq!(SplitPermille::from_qdr_ten_thousandths(2000).value(), 200);
    assert_eq!(SplitPermille::from_qdr_ten_thousandths(7500).value(), 750);
    assert_eq!(SplitPermille::from_qdr_ten_thousandths(2500).value(), 250);
    // 截断到 0..=1000。
    assert_eq!(SplitPermille::from_qdr_ten_thousandths(20000).value(), 1000);
    assert_eq!(SplitPermille::from_qdr_ten_thousandths(5000).to_qdr_ten_thousandths(), 5000);
    assert_eq!(SplitPermille(500).ratio(), 0.5);
}

#[test]
fn view_state_string_roundtrip() {
    use qdir_rs::core::{ColumnConfig, ViewMode};
    let cfg = ColumnConfig {
        view_mode: ViewMode::Detail,
        image_size: 32,
        folder_flags: 1075838981,
        group_by: "Ext".into(),
        group: true,
        ..ColumnConfig::default()
    };
    let s = cfg.to_view_state_string();
    assert_eq!(s, "ViewMode,3;ImageSize,32;FolderFlags,1075838981;GroupBy,Ext;Group,1;");
    let back = ColumnConfig::parse_view_state_string(&s).expect("可解析");
    assert_eq!(back, cfg);
    // 未知键忽略（前向兼容）。
    let fwd = ColumnConfig::parse_view_state_string("ViewMode,2;FutureKey,9;")
        .expect("未知键不得报错");
    assert_eq!(fwd.view_mode, ViewMode::List);
}

#[test]
fn pane_and_tab_state_matches_qdr_defaults() {
    use qdir_rs::core::*;
    // start.qdr 实测：TreeN=0 / TreeNsplit=2500 -> 250 千分比。
    let pane = PaneState::new(PaneId::P1, std::path::PathBuf::from(r"C:\Users"));
    assert!(!pane.tree_visible);
    assert_eq!(pane.tree_split, 250);

    // 导航历史：前进/后退栈语义。
    let mut tab = TabState::new(std::path::PathBuf::from(r"C:\a"));
    tab.navigate(std::path::PathBuf::from(r"C:\b"));
    tab.back().expect("后退");
    assert_eq!(tab.path, std::path::PathBuf::from(r"C:\a"));
    assert!(tab.forward().is_ok());
    tab.navigate(std::path::PathBuf::from(r"C:\c"));
    assert!(!tab.can_forward(), "新导航必须截断前进栈");

    // 最后一个标签页不可关闭。
    let mut single = PaneState::new(PaneId::P2, std::path::PathBuf::from(r"C:\x"));
    assert!(single.close_tab(0).is_err());
    single.add_tab(std::path::PathBuf::from(r"C:\y"));
    single.close_tab(0).expect("关闭第一个");
    assert_eq!(single.tabs.len(), 1);
}
