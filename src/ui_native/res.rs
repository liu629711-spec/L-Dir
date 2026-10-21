//! 原版资源嵌入：Q-Dir_x64.exe 提取的对话框/菜单模板原样字节。
//!
//! 1:1 原则：这些字节就是原版编译产物里的模板，运行时用
//! `CreateDialogIndirectParamW` / `LoadMenuIndirectW` 直接消费，
//! 布局/字体/控件 ID 与原版零差异（数据同源，非仿制）。

/// 对话框模板（原 RT_DIALOG，57 个）。键 = 资源 ID。
pub static DIALOGS: &[(&str, &[u8])] = &[
    ("100", include_bytes!("../../assets/extracted/5/100.bin")),
    ("101", include_bytes!("../../assets/extracted/5/101.bin")),
    ("102", include_bytes!("../../assets/extracted/5/102.bin")),
    ("103", include_bytes!("../../assets/extracted/5/103.bin")),
    ("113", include_bytes!("../../assets/extracted/5/113.bin")),
    ("132", include_bytes!("../../assets/extracted/5/132.bin")),
    ("133", include_bytes!("../../assets/extracted/5/133.bin")),
    ("134", include_bytes!("../../assets/extracted/5/134.bin")),
    ("135", include_bytes!("../../assets/extracted/5/135.bin")),
    ("201", include_bytes!("../../assets/extracted/5/201.bin")),
    ("202", include_bytes!("../../assets/extracted/5/202.bin")),
    ("203", include_bytes!("../../assets/extracted/5/203.bin")),
    ("206", include_bytes!("../../assets/extracted/5/206.bin")),
    ("208", include_bytes!("../../assets/extracted/5/208.bin")),
    ("212", include_bytes!("../../assets/extracted/5/212.bin")),
    ("214", include_bytes!("../../assets/extracted/5/214.bin")),
    ("218", include_bytes!("../../assets/extracted/5/218.bin")),
    ("220", include_bytes!("../../assets/extracted/5/220.bin")),
    ("221", include_bytes!("../../assets/extracted/5/221.bin")),
    ("223", include_bytes!("../../assets/extracted/5/223.bin")),
    ("228", include_bytes!("../../assets/extracted/5/228.bin")),
    ("230", include_bytes!("../../assets/extracted/5/230.bin")),
    ("246", include_bytes!("../../assets/extracted/5/246.bin")),
    ("253", include_bytes!("../../assets/extracted/5/253.bin")),
    ("275", include_bytes!("../../assets/extracted/5/275.bin")),
    ("279", include_bytes!("../../assets/extracted/5/279.bin")),
    ("280", include_bytes!("../../assets/extracted/5/280.bin")),
    ("298", include_bytes!("../../assets/extracted/5/298.bin")),
    ("302", include_bytes!("../../assets/extracted/5/302.bin")),
    ("304", include_bytes!("../../assets/extracted/5/304.bin")),
    ("310", include_bytes!("../../assets/extracted/5/310.bin")),
    ("313", include_bytes!("../../assets/extracted/5/313.bin")),
    ("322", include_bytes!("../../assets/extracted/5/322.bin")),
    ("324", include_bytes!("../../assets/extracted/5/324.bin")),
    ("331", include_bytes!("../../assets/extracted/5/331.bin")),
    ("333", include_bytes!("../../assets/extracted/5/333.bin")),
    ("335", include_bytes!("../../assets/extracted/5/335.bin")),
    ("339", include_bytes!("../../assets/extracted/5/339.bin")),
    ("340", include_bytes!("../../assets/extracted/5/340.bin")),
    ("342", include_bytes!("../../assets/extracted/5/342.bin")),
    ("343", include_bytes!("../../assets/extracted/5/343.bin")),
    ("347", include_bytes!("../../assets/extracted/5/347.bin")),
    ("349", include_bytes!("../../assets/extracted/5/349.bin")),
    ("361", include_bytes!("../../assets/extracted/5/361.bin")),
    ("370", include_bytes!("../../assets/extracted/5/370.bin")),
    ("414", include_bytes!("../../assets/extracted/5/414.bin")),
    ("415", include_bytes!("../../assets/extracted/5/415.bin")),
    ("416", include_bytes!("../../assets/extracted/5/416.bin")),
    ("417", include_bytes!("../../assets/extracted/5/417.bin")),
    ("418", include_bytes!("../../assets/extracted/5/418.bin")),
    ("419", include_bytes!("../../assets/extracted/5/419.bin")),
    ("432", include_bytes!("../../assets/extracted/5/432.bin")),
    ("462", include_bytes!("../../assets/extracted/5/462.bin")),
    ("469", include_bytes!("../../assets/extracted/5/469.bin")),
    ("470", include_bytes!("../../assets/extracted/5/470.bin")),
    ("472", include_bytes!("../../assets/extracted/5/472.bin")),
    ("473", include_bytes!("../../assets/extracted/5/473.bin")),
];

/// 菜单模板（原 RT_MENU，10 个）。键 = 资源 ID。
pub static MENUS: &[(&str, &[u8])] = &[
    ("128", include_bytes!("../../assets/extracted/4/128.bin")),
    ("216", include_bytes!("../../assets/extracted/4/216.bin")),
    ("217", include_bytes!("../../assets/extracted/4/217.bin")),
    ("303", include_bytes!("../../assets/extracted/4/303.bin")),
    ("305", include_bytes!("../../assets/extracted/4/305.bin")),
    ("307", include_bytes!("../../assets/extracted/4/307.bin")),
    ("319", include_bytes!("../../assets/extracted/4/319.bin")),
    ("358", include_bytes!("../../assets/extracted/4/358.bin")),
    ("430", include_bytes!("../../assets/extracted/4/430.bin")),
    ("4161", include_bytes!("../../assets/extracted/4/4161.bin")),
];

pub fn dialog_template(id: &str) -> Option<&'static [u8]> {
    DIALOGS.iter().find(|(k, _)| *k == id).map(|(_, b)| *b)
}

pub fn menu_template(id: &str) -> Option<&'static [u8]> {
    MENUS.iter().find(|(k, _)| *k == id).map(|(_, b)| *b)
}
