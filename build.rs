//! 嵌入 comctl32 v6 manifest：ReBar 波段 / ImageList 32bpp / 视觉样式均依赖 v6。
//! new_manifest 默认带 Microsoft.Windows.Common-Controls 6.0.0.0 依赖。

fn main() {
    let m = embed_manifest::new_manifest("QDirRs.Main");
    embed_manifest::embed_manifest(m).expect("嵌入 comctl32 v6 manifest 失败");
}
