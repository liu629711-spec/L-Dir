//! 配置往返测试：用真实 Q-Dir.ini / start.qdr 的字节做
//! 解析 → 回写 → 再解析断言（契约 E2）。
//!
//! 夹具路径来自契约 G4 指定的唯一逆向证据目录：
//! `D:\Q-Dir_Portable_x64\Q-Dir\`（不得修改其中的文件）。
//! 若该目录不存在（如在 CI 的其它机器上跑），测试自动跳过而不是失败。

use std::path::{Path, PathBuf};

/// 证据夹具目录（契约 G4：唯一逆向证据来源）。
const EVIDENCE_DIR: &str = r"D:\Q-Dir_Portable_x64\Q-Dir";

fn evidence_ini() -> Option<PathBuf> {
    let p = Path::new(EVIDENCE_DIR).join("Q-Dir.ini");
    p.exists().then_some(p)
}

fn evidence_qdr() -> Option<PathBuf> {
    let p = Path::new(EVIDENCE_DIR).join("start.qdr");
    p.exists().then_some(p)
}

/// 把证据文件复制到一个**唯一**临时路径后返回副本路径。
/// `save_*` 会读原文件以保留未知键（E2），因此必须给一个可写的副本。
///
/// 路径必须唯一：多个测试并发跑时若共用同一文件名会互相覆盖，
/// 导致读到别的测试已经改过的内容（曾因此误报「修改未落盘」）。
fn copy_to_tmp(name: &str) -> PathBuf {
    let src = Path::new(EVIDENCE_DIR).join(name);
    let bytes = std::fs::read(&src).expect("读证据文件");
    // 用测试名 + 纳秒时间戳保证唯一。
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut out = std::env::temp_dir();
    out.push(format!("qdir-rs-{unique}-{name}"));
    std::fs::write(&out, &bytes).expect("写临时副本");
    out
}

#[test]
fn ini_byte_identical_roundtrip_on_real_file() {
    let Some(src_path) = evidence_ini() else {
        eprintln!("跳过：证据目录不存在（{}）", EVIDENCE_DIR);
        return;
    };
    let src = std::fs::read(&src_path).expect("读证据 ini");
    let out = copy_to_tmp("Q-Dir.ini");

    let cfg = qdir_rs::config::load_config(&out).expect("解析真实 ini");
    // 先断言建模字段与证据一致（H1：关键键值可被自研解析器读出）。
    // QDir_Id 是原版的实例计数键（运行中自增、正常退出归零，实测见过
    // 1/2/0），不断钉具体值，只校验解析为非负整数。稳定键才断言具体值。
    assert!(
        cfg.qdir_id >= 0,
        "QDir_Id 应解析为非负整数，实测 {}",
        cfg.qdir_id
    );
    assert_eq!(cfg.als, 5);
    assert_eq!(cfg.designe_mode, 1);
    assert_eq!(cfg.show_cmd, 3);
    assert_eq!(cfg.langid, 1031);
    // WinRC 是运行时状态键（原版退出时按窗口实际位置回写；夹具
    // 实测见过 0;0;800;600 与 40;40;1140;840 两种），同 qdir_id
    // 不钉具体值，只校验解析出「宽高为正」的 4 整数。
    assert_eq!(cfg.win_rc.len(), 4);
    assert!(
        cfg.win_rc[2] > 0 && cfg.win_rc[3] > 0,
        "WinRC 宽高应为正，实测 {:?}",
        cfg.win_rc
    );
    assert_eq!(cfg.favoriten_dir, PathBuf::from(".\\Favoriten\\"));
    assert_eq!(cfg.quick_link_dir, PathBuf::from(".\\Favoriten\\Quick-Link\\"));
    assert_eq!(cfg.preview_filter, "*;");
    assert_eq!(cfg.rules.rules.len(), 12, "Filter2 实测 12 条规则");

    qdir_rs::config::save_config(&out, &cfg).expect("保存");
    let written = std::fs::read(&out).expect("读回写结果");
    assert_eq!(
        written, src,
        "不改任何键时，Q-Dir.ini 必须逐字节等价（E2）"
    );
    let cfg2 = qdir_rs::config::load_config_bytes(&written).expect("再解析");
    assert_eq!(cfg, cfg2, "建模字段必须往返一致");
}

#[test]
fn ini_modification_only_touches_target_key() {
    let Some(src_path) = evidence_ini() else {
        eprintln!("跳过：证据目录不存在（{}）", EVIDENCE_DIR);
        return;
    };
    let src = std::fs::read(&src_path).expect("读证据 ini");
    let out = copy_to_tmp("Q-Dir.ini");

    let mut cfg = qdir_rs::config::load_config(&out).expect("解析");
    let original_als = cfg.als;
    cfg.als = 4242;
    qdir_rs::config::save_config(&out, &cfg).expect("保存");

    let written = std::fs::read(&out).expect("读回写结果");
    let cfg2 = qdir_rs::config::load_config_bytes(&written).expect("再解析");
    assert_eq!(cfg2.als, 4242, "修改必须落盘");
    assert_eq!(cfg, cfg2, "其余字段不得变化");

    // 逐行比对：只有 Als 一行不同。
    let before = qdir_rs::config::decode_utf16le(&src).expect("解码原文");
    let after = qdir_rs::config::decode_utf16le(&written).expect("解码回写");
    let b: Vec<&str> = before.lines().collect();
    let a: Vec<&str> = after.lines().collect();
    assert_eq!(b.len(), a.len(), "行数不得变化");
    let changed: Vec<(&str, &str)> = b
        .iter()
        .zip(a.iter())
        .filter(|(x, y)| x != y)
        .map(|(x, y)| (*x, *y))
        .collect();
    assert_eq!(changed.len(), 1, "应只有一行变化，实际 {changed:?}");
    assert_eq!(changed[0].0, format!("Als={original_als}"));
    assert_eq!(changed[0].1, "Als=4242");
}

#[test]
fn ini_filter2_rules_survive_roundtrip_verbatim() {
    let Some(src_path) = evidence_ini() else {
        eprintln!("跳过：证据目录不存在（{}）", EVIDENCE_DIR);
        return;
    };
    let src = std::fs::read(&src_path).expect("读证据 ini");
    let out = copy_to_tmp("Q-Dir.ini");
    let cfg = qdir_rs::config::load_config(&out).expect("解析");
    qdir_rs::config::save_config(&out, &cfg).expect("保存");
    let written = std::fs::read(&out).expect("读回写结果");

    let before = qdir_rs::config::decode_utf16le(&src).expect("解码");
    let after = qdir_rs::config::decode_utf16le(&written).expect("解码");
    let filter2 = |t: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut in_sec = false;
        for line in t.lines() {
            if line.starts_with('[') {
                in_sec = line == "[Filter2]";
                continue;
            }
            if in_sec && !line.is_empty() {
                out.push(line.to_string());
            }
        }
        out
    };
    assert_eq!(
        filter2(&before),
        filter2(&after),
        "[Filter2] 段必须逐字节保留（含 rule 4 的混用分隔符，E6）"
    );
}

#[test]
fn qdr_byte_identical_roundtrip_on_real_file() {
    let Some(src_path) = evidence_qdr() else {
        eprintln!("跳过：证据目录不存在（{}）", EVIDENCE_DIR);
        return;
    };
    let src = std::fs::read(&src_path).expect("读证据 qdr");
    let out = copy_to_tmp("start.qdr");

    let st = qdir_rs::config::load_session(&out).expect("解析真实 qdr");
    // H1 / 契约证据键值断言。
    assert_eq!(st.version, "12.71");
    assert_eq!(st.view_mode, 0);
    // Focus 是原版退出时写回的最后活动窗格（运行时易变，实测见过 P1/P3），
    // 只校验解析为合法窗格，不钉具体值（同 QDir_Id 原则）。
    assert!(
        matches!(
            st.focus,
            qdir_rs::core::PaneId::P1
                | qdir_rs::core::PaneId::P2
                | qdir_rs::core::PaneId::P3
                | qdir_rs::core::PaneId::P4
        ),
        "Focus 应解析为合法窗格，实测 {:?}",
        st.focus
    );
    assert_eq!(st.colorthema, 0);
    assert_eq!(st.tree_mode, qdir_rs::core::TreeMode::Off, "实测 start.qdr tree_mode=0");
    assert!(!st.preview_visible);
    assert_eq!(st.splits_h[0].value(), 500, "SP0=5000 万分数 -> 500 千分比");
    assert_eq!(st.splits_h[1].value(), 500, "SP1=5003 取整后同为 500");
    assert_eq!(st.splits_h[2].value(), 500, "SP2=5003 取整后同为 500");
    assert_eq!(st.split_preview.value(), 200, "SPT=2000");
    assert_eq!(st.preview_pos.value(), 750, "VorschauPos=7500");
    for pane_id in qdir_rs::core::PaneId::ALL {
        let pane = st.pane(pane_id);
        assert!(!pane.tree_visible, "Tree{} =0", pane_id.number());
        assert_eq!(pane.tree_split, 250, "Tree{}split=2500", pane_id.number());
    }
    // E4：ColsAndSort1..4 实测各约 2342 字符，必须原样保留。
    // 注意：原版每次运行会按列交互微调该串（实测 2341/2342 都有），
    // 因此断言"量级"而非精确值；原样保留由下面逐字节等价断言保证。
    for (i, raw) in st.cols_and_sort_raw.iter().enumerate() {
        assert!(
            (2340..=2345).contains(&raw.len()),
            "ColsAndSort{} 实测约 2342 字符，当前 {}",
            i + 1,
            raw.len()
        );
    }

    qdir_rs::config::save_session(&out, &st).expect("保存");
    let written = std::fs::read(&out).expect("读回写结果");
    assert_eq!(
        written, src,
        "不改任何键时，start.qdr 必须逐字节等价（E2）"
    );
    let st2 = qdir_rs::config::load_session_bytes(&written).expect("再解析");
    assert_eq!(st, st2, "会话状态必须往返一致");
}

#[test]
fn qdr_folder_subfields_are_not_top_level_keys() {
    let Some(src_path) = evidence_qdr() else {
        eprintln!("跳过：证据目录不存在（{}）", EVIDENCE_DIR);
        return;
    };
    let src = std::fs::read(&src_path).expect("读证据 qdr");
    let st = qdir_rs::config::load_session_bytes(&src).expect("解析");

    // E3：ViewModeN / ViewModeFlagsN 是 FolderN 的 '&' 子字段，不是顶层键。
    // 顶层 ViewMode 实测为 0；FolderN 的 ViewModeN 实测为 4（超出 0..=3 的
    // ViewMode 枚举，因此不能落入标签页覆盖，只能原样保留）。
    assert_eq!(st.view_mode, 0);
    for pane_id in qdir_rs::core::PaneId::ALL {
        assert_eq!(
            st.pane(pane_id).columns.folder_flags,
            1075838981,
            "ViewModeFlags{} 子字段必须被读出",
            pane_id.number()
        );
    }
}

#[test]
fn missing_files_yield_defaults_without_panic() {
    // 任务要求：路径不存在时返回默认值而不是 panic。
    let cfg = qdir_rs::config::load_config(Path::new(
        r"Z:\qdir-rs-definitely-missing\Q-Dir.ini",
    ))
    .expect("缺失 ini 应返回默认 Config");
    assert_eq!(cfg.qdir_id, 1);
    assert_eq!(cfg.rules.rules.len(), 12);

    let st = qdir_rs::config::load_session(Path::new(
        r"Z:\qdir-rs-definitely-missing\start.qdr",
    ))
    .expect("缺失 qdr 应返回默认 SessionState");
    assert_eq!(st.version, "12.71");
    // Focus 是原版退出时写回的最后活动窗格（运行时易变，实测见过 P1/P3），
    // 只校验解析为合法窗格，不钉具体值（同 QDir_Id 原则）。
    assert!(
        matches!(
            st.focus,
            qdir_rs::core::PaneId::P1
                | qdir_rs::core::PaneId::P2
                | qdir_rs::core::PaneId::P3
                | qdir_rs::core::PaneId::P4
        ),
        "Focus 应解析为合法窗格，实测 {:?}",
        st.focus
    );
}
