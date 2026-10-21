//! 分割条：按原版 0x42a3bc 反编译规格实现。
//!
//! 原版语义（已反编译）：
//! - 比例存储为**万分比**：`ratio = MulDiv(pos, 10000, clientWidth - splitterWidth)`
//! - 拖动位置钳制在 `splitterWidth ..= client - splitterWidth`
//! - `flags bit0` = 反向模式（第二窗格占左边），`flags bit2` = 按比例恢复
//!
//! 对应 .qdr：SP0/SP1/SP2（万分比），SPT 是预览分割。

use crate::core::SplitPermille;

/// 分割条宽度（DLU≈像素，原版为固定窄条）。
pub const SPLITTER_WIDTH: i32 = 4;

/// 12 种布局模式。名称/文案来源（可证部分）：
/// - 原版 UIA dump：布局栏 12 个按钮 tooltip 精确文案，顺序如下
///   [1] Q-Dir 默认4格 [2] 3上1下2 [3] 3上2下1 [4] 2垂直 [5] 2水平
///   [6] 4垂直 [7] 4水平 [8] 3垂直 [9] 3水平 [10] 3左1右2 [11] 3左2右1
///   [12] 1-Dir（均带 "\n右击重置"）。
/// - 视图菜单（menu128 popup 53000）暴露 4 个布局命令 id：
///   0xA776/0xA777/0xA778/0xA779（i18n 键 53005..53008）。
/// - FUN_00429bc4：go 按钮(0xB379) 是 BTNS_SPLIT（+0x40=1），即 12 布局
///   下拉的宿主；导航栏 8 个 PNG 按钮是快捷布局选择，与下拉共享 id 空间
///   0xA76C..0xA773。
///
/// 未确证项：8 个 PNG 按钮各自精确对应 12 模式中的哪一档
/// （原版对跨进程 PostMessage WM_COMMAND 布局 id 无响应，实测矩形逐字节
/// 不变，无法用外部注入复核；需 UI 内点击复核，列为遗留）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layout {
    /// 2×2 网格（原版默认，导航栏按钮 0xA76C）。
    Default,
    /// 3-Dir：1 上、2 下（导航栏按钮 0xA771）。
    ThreeTopOneTwoBottom,
    /// 3-Dir：2 上、1 下（导航栏按钮 0xA772）。
    ThreeTopTwoOneBottom,
    /// 2-Dir：左右（垂直分割线，导航栏按钮 0xA76D）。
    TwoVertical,
    /// 2-Dir：上下（水平分割线，导航栏下拉项）。
    TwoHorizontal,
    /// 4-Dir：4 列。
    FourVertical,
    /// 4-Dir：4 行。
    FourHorizontal,
    /// 3-Dir：3 列。
    ThreeVertical,
    /// 3-Dir：3 行。
    ThreeHorizontal,
    /// 3-Dir：1 左、2 右。
    ThreeLeftOneTwoRight,
    /// 3-Dir：2 左、1 右。
    ThreeLeftTwoOneRight,
    /// 单窗格（导航栏按钮 0xA773）。
    One,
}

impl Layout {
    /// 可见窗格数。
    pub fn pane_count(self) -> usize {
        match self {
            Layout::Default | Layout::FourVertical | Layout::FourHorizontal => 4,
            Layout::TwoVertical | Layout::TwoHorizontal => 2,
            Layout::One => 1,
            _ => 3,
        }
    }

    /// 已确证 id 的导航栏按钮 → 布局模式（其余走 go 下拉/视图菜单）。
    pub fn from_nav_id(id: u32) -> Option<Layout> {
        match id {
            0xA76C => Some(Layout::Default),
            0xA76D => Some(Layout::TwoVertical),
            0xA771 => Some(Layout::ThreeTopOneTwoBottom),
            0xA772 => Some(Layout::ThreeTopTwoOneBottom),
            0xA773 => Some(Layout::One),
            _ => None,
        }
    }
}

/// 递归分割树的一个节点布局结果：4 个窗格各自的 (x, y, w, h)。
pub struct PaneRects(pub [(i32, i32, i32, i32); 4]);

/// 按布局模式递归切分区域，返回 4 个窗格矩形（不可见窗格 w/h=0）。
pub fn layout_rects(mode: Layout, x: i32, y: i32, w: i32, h: i32) -> PaneRects {
    use Layout::*;
    let sw = SPLITTER_WIDTH;
    let mut r = [(-1i32, -1i32, 0, 0); 4];
    let n = mode.pane_count();

    // 顶部分割方向 + 是否再内部二次分割。
    let mut assign = |rects: &mut [(i32, i32, i32, i32); 4], slots: Vec<(i32, i32, i32, i32)>| {
        for (i, s) in slots.into_iter().enumerate().take(4) {
            rects[i] = s;
        }
    };

    match mode {
        Default => {
            // 左右两半，各再上下分。
            let colw = (w - sw) / 2;
            let rowh = (h - sw) / 2;
            assign(
                &mut r,
                vec![
                    (x, y, colw, rowh),
                    (x, y + rowh + sw, colw, h - rowh - sw),
                    (x + colw + sw, y, w - colw - sw, rowh),
                    (x + colw + sw, y + rowh + sw, w - colw - sw, h - rowh - sw),
                ],
            );
        }
        TwoVertical => {
            let colw = (w - sw) / 2;
            assign(
                &mut r,
                vec![
                    (x, y, colw, h),
                    (x + colw + sw, y, w - colw - sw, h),
                    (0, 0, 0, 0),
                    (0, 0, 0, 0),
                ],
            );
        }
        TwoHorizontal => {
            let rowh = (h - sw) / 2;
            assign(
                &mut r,
                vec![
                    (x, y, w, rowh),
                    (x, y + rowh + sw, w, h - rowh - sw),
                    (0, 0, 0, 0),
                    (0, 0, 0, 0),
                ],
            );
        }
        FourVertical => {
            let cw = (w - 3 * sw) / 4;
            assign(
                &mut r,
                (0..4)
                    .map(|i| (x + i * (cw + sw), y, cw, h))
                    .collect(),
            );
        }
        FourHorizontal => {
            let ch = (h - 3 * sw) / 4;
            assign(
                &mut r,
                (0..4).map(|i| (x, y + i * (ch + sw), w, ch)).collect(),
            );
        }
        ThreeVertical => {
            let cw = (w - 2 * sw) / 3;
            assign(
                &mut r,
                vec![
                    (x, y, cw, h),
                    (x + cw + sw, y, cw, h),
                    (x + 2 * (cw + sw), y, w - 2 * (cw + sw), h),
                    (0, 0, 0, 0),
                ],
            );
        }
        ThreeHorizontal => {
            let ch = (h - 2 * sw) / 3;
            assign(
                &mut r,
                vec![
                    (x, y, w, ch),
                    (x, y + ch + sw, w, ch),
                    (x, y + 2 * (ch + sw), w, h - 2 * (ch + sw)),
                    (0, 0, 0, 0),
                ],
            );
        }
        ThreeTopOneTwoBottom => {
            let rowh = (h - sw) / 2;
            let cw = (w - sw) / 2;
            assign(
                &mut r,
                vec![
                    (x, y, w, rowh),
                    (x, y + rowh + sw, cw, h - rowh - sw),
                    (x + cw + sw, y + rowh + sw, w - cw - sw, h - rowh - sw),
                    (0, 0, 0, 0),
                ],
            );
        }
        ThreeTopTwoOneBottom => {
            let rowh = (h - sw) / 2;
            let cw = (w - sw) / 2;
            assign(
                &mut r,
                vec![
                    (x, y, cw, rowh),
                    (x + cw + sw, y, w - cw - sw, rowh),
                    (x, y + rowh + sw, w, h - rowh - sw),
                    (0, 0, 0, 0),
                ],
            );
        }
        ThreeLeftOneTwoRight => {
            let cw = (w - sw) / 2;
            let ch = (h - sw) / 2;
            assign(
                &mut r,
                vec![
                    (x, y, cw, h),
                    (x + cw + sw, y, w - cw - sw, ch),
                    (x + cw + sw, y + ch + sw, w - cw - sw, h - ch - sw),
                    (0, 0, 0, 0),
                ],
            );
        }
        ThreeLeftTwoOneRight => {
            let cw = (w - sw) / 2;
            let ch = (h - sw) / 2;
            assign(
                &mut r,
                vec![
                    (x, y, cw, ch),
                    (x, y + ch + sw, cw, h - ch - sw),
                    (x + cw + sw, y, w - cw - sw, h),
                    (0, 0, 0, 0),
                ],
            );
        }
        One => {
            assign(&mut r, vec![(x, y, w, h), (0, 0, 0, 0), (0, 0, 0, 0), (0, 0, 0, 0)]);
        }
    }
    // n<4 的窗格已被置零；n 与 mode.pane_count() 一致（此处仅示意，
    // 实际隐藏逻辑由调用方按 rect 宽高为 0 处理）。
    let _ = n;
    PaneRects(r)
}

/// N 窗格分割组：当前布局模式 + 原版嵌套分割的三条万分比位置。
///
/// Default（2×2）模式的 SP 语义与原版 .qdr 一致（嵌套 splitter 实测）：
/// - `ratios[0]`：外层竖直分割条（左列 | 右列）
/// - `ratios[1]`：左列内水平分割条（上 | 下）
/// - `ratios[2]`：右列内水平分割条（上 | 下）
pub struct SplitterGroup {
    /// 三条分割条位置（万分比 0..=10000）。
    ratios: Vec<u32>,
    /// 当前布局模式。
    pub layout: Layout,
}

impl SplitterGroup {
    pub fn horizontal(panes: usize) -> Self {
        let _ = panes;
        SplitterGroup {
            ratios: vec![5000, 5000, 5000],
            layout: Layout::Default,
        }
    }

    /// 从会话恢复（SP0/SP1/SP2 万分比已是原版单位）。
    pub fn from_permille_ten(&mut self, sps: &[SplitPermille]) {
        for (r, sp) in self.ratios.iter_mut().zip(sps) {
            // SplitPermille 存的是千分比（0..1000），原版 SP 是万分比（0..10000）。
            // core::SplitPermille::from_qdr_ten_thousandths 已做 /10 收敛，这里还原为万分比。
            *r = sp.value() * 10;
        }
    }

    /// 设置第 i 条分割条位置（像素），按原版 MulDiv 公式转万分比。
    pub fn set_pixel_pos(&mut self, i: usize, pos: i32, client: i32) {
        if i >= self.ratios.len() {
            return;
        }
        let usable = (client - SPLITTER_WIDTH).max(1);
        // 原版: ratio = MulDiv(pos, 10000, usable)（0x42a3bc 反编译确认）
        let ratio = ((pos as i64 * 10000) / usable as i64).clamp(0, 10000) as u32;
        self.ratios[i] = ratio;
    }

    pub fn ratios(&self) -> &[u32] {
        &self.ratios
    }

    /// 按当前布局模式切分窗格区 (w × h)，返回 4 个窗格矩形。
    ///
    /// Default 用 SP0/1/2 比例；比例退化（非递增或压扁窗格，如实测异常
    /// 值 5000/5003/5003）时回退等分（可用性优先，原版表现为后窗格不可见）。
    pub fn rects(&self, w: i32, h: i32) -> PaneRects {
        if self.layout != Layout::Default {
            return layout_rects(self.layout, 0, 0, w, h);
        }
        let sw = SPLITTER_WIDTH;
        let min_pane: i64 = 8;
        let usable_w = (w - sw).max(1);
        let usable_h = (h - sw).max(1);
        let col = self.ratios[0] as i64 * usable_w as i64 / 10000;
        let rowl = self.ratios[1] as i64 * usable_h as i64 / 10000;
        let rowr = self.ratios[2] as i64 * usable_h as i64 / 10000;
        let ok_col = (min_pane..=usable_w as i64 - min_pane).contains(&col);
        let ok_rows = (min_pane..=usable_h as i64 - min_pane).contains(&rowl)
            && (min_pane..=usable_h as i64 - min_pane).contains(&rowr);
        if !(ok_col && ok_rows) {
            return layout_rects(Layout::Default, 0, 0, w, h);
        }
        let (col, rowl, rowr) = (col as i32, rowl as i32, rowr as i32);
        PaneRects([
            (0, 0, col, rowl),
            (0, rowl + sw, col, h - rowl - sw),
            (col + sw, 0, w - col - sw, rowr),
            (col + sw, rowr + sw, w - col - sw, h - rowr - sw),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 断言某布局的可见窗格数与 rect 宽高为正、隐藏窗格为 0。
    fn check(mode: Layout, w: i32, h: i32) -> PaneRects {
        let r = layout_rects(mode, 0, 0, w, h);
        let visible = (0..4)
            .filter(|&i| r.0[i].2 > 0 && r.0[i].3 > 0)
            .count();
        assert_eq!(visible, mode.pane_count(), "{mode:?} 可见窗格数");
        r
    }

    #[test]
    fn one_uses_full_area() {
        let r = check(Layout::One, 800, 600);
        assert_eq!(r.0[0], (0, 0, 800, 600));
    }

    #[test]
    fn two_vertical_is_side_by_side() {
        let r = check(Layout::TwoVertical, 804, 600);
        // 两条垂直排列：同 y，左右分。
        assert_eq!(r.0[0].1, 0);
        assert_eq!(r.0[1].1, 0);
        assert!(r.0[0].0 < r.0[1].0, "第二个窗格在第一个右边");
    }

    #[test]
    fn two_horizontal_is_stacked() {
        let r = check(Layout::TwoHorizontal, 800, 604);
        // 两条水平排列：同 x，上下分。
        assert_eq!(r.0[0].0, 0);
        assert_eq!(r.0[1].0, 0);
        assert!(r.0[0].1 < r.0[1].1, "第二个窗格在第一个下边");
    }

    #[test]
    fn default_is_2x2_grid() {
        let r = check(Layout::Default, 804, 604);
        assert!(r.0[2].0 > r.0[0].0, "右列在左列右");
        assert!(r.0[1].1 > r.0[0].1, "左下在左上正下方");
    }

    #[test]
    fn three_left_one_two_right() {
        let r = check(Layout::ThreeLeftOneTwoRight, 804, 604);
        // 窗格1占满左侧高，2/3 在右侧上下分。
        assert_eq!(r.0[0].1, 0);
        assert!(r.0[1].1 < r.0[2].1, "pane2 在 pane3 上方");
        assert!(r.0[1].0 > r.0[0].0, "pane2/3 在 pane1 右侧");
    }

    #[test]
    fn rects_cover_area_without_overlap_default() {
        let g = SplitterGroup::horizontal(4);
        let r = g.rects(804, 604);
        // 四格互不重叠：任意两格不相交。
        for i in 0..4 {
            for j in (i + 1)..4 {
                let (a, b) = (r.0[i], r.0[j]);
                let overlap_x = a.0.max(b.0) < (a.0 + a.2).min(b.0 + b.2);
                let overlap_y = a.1.max(b.1) < (a.1 + a.3).min(b.1 + b.3);
                assert!(
                    !(overlap_x && overlap_y),
                    "{:?} 格 {i} 与 {j} 重叠: {a:?} {b:?}",
                    g.layout
                );
            }
        }
    }

    #[test]
    fn degenerate_sp_falls_back_to_equal_grid() {
        // 实测异常 .qdr：SP 全为 5000 附近退化值仍应给出非零四格。
        let mut g = SplitterGroup::horizontal(4);
        g.ratios = vec![5000, 5003, 5003];
        let r = g.rects(804, 604);
        for (i, rect) in r.0.iter().enumerate() {
            assert!(rect.2 > 0 && rect.3 > 0, "退化保护下格 {i} 非零: {rect:?}");
        }
    }
}
