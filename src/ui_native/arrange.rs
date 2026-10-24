//! 窗口排列（P2.9）：层叠 / 横向平铺 / 纵向平铺 / 排列图标。
//!
//! 原版证据：`services/i18n.rs`「窗口排列（菜单 216 实测）」四词条
//! （Überlappend/Nebeneinander/Untereinander/Symbole anordnen）。原版入口
//! 位置未反编译确证 *待复核*，本实现挂在「活动窗口」菜单尾部。
//!
//! 层叠/平铺对**同类名的可见顶层窗**生效（含本进程与「重复窗口」生成的
//! 其他实例——EnumWindows 按类名 `LDir.MainFrame` 收集，跨进程 SetWindowPos
//! 合法）。平铺语义按 Windows 惯例：横向平铺（Nebeneinander）= 左右列，
//! 纵向平铺（Untereinander）= 上下行。最小化窗口不参与。
//!
//! 布局计算为纯函数 [`arrange_rects`]（单测覆盖），UI 侧只做收集与应用。

use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowRect, IsIconic, IsWindowVisible, SetWindowPos,
    SWP_NOZORDER, SET_WINDOW_POS_FLAGS, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    SystemParametersInfoW,
};

use crate::brand;

/// 排列方式（层叠 / 横向平铺 / 纵向平铺；排列图标走 LVM_ARRANGE 不在此）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrangeKind {
    /// 层叠（Cascade）：统一 3/4 尺寸 + 24px 阶梯偏移。
    Cascade,
    /// 横向平铺（Nebeneinander）：左右列，等宽。
    TileHorizontal,
    /// 纵向平铺（Untereinander）：上下行，等高。
    TileVertical,
}

/// 工作区矩形（x, y, 宽, 高）。
pub type Rect4 = (i32, i32, i32, i32);

/// 层叠阶梯偏移（像素；Windows 层叠惯例 24px）。
const CASCADE_STEP: i32 = 24;
/// 平铺单元最小边（像素；低于此值不再均分，钳到最小）。
const TILE_MIN: i32 = 120;

/// 计算排列布局（纯函数，单测锁定）。
///
/// `n`=参与窗口数（n=0 返回空）。返回与窗口收集序一一对应的目标矩形。
/// - Cascade：每窗 = 工作区 3/4 尺寸，位置 (x+24i, y+24i)（i 不回绕，
///   窗多时自然越界由系统钳制）；
/// - TileHorizontal：左右列，每窗宽 = w/n（低于 TILE_MIN 钳 TILE_MIN）；
/// - TileVertical：上下行，每窗高 = h/n（同上钳制）。
pub fn arrange_rects(kind: ArrangeKind, work: Rect4, n: usize) -> Vec<Rect4> {
    if n == 0 {
        return Vec::new();
    }
    let (x, y, w, h) = work;
    let n_i = n as i32;
    match kind {
        ArrangeKind::Cascade => {
            let cw = (w * 3 / 4).max(TILE_MIN);
            let ch = (h * 3 / 4).max(TILE_MIN);
            (0..n_i)
                .map(|i| (x + CASCADE_STEP * i, y + CASCADE_STEP * i, cw, ch))
                .collect()
        }
        ArrangeKind::TileHorizontal => {
            let cw = (w / n_i).max(TILE_MIN);
            (0..n_i)
                .map(|i| (x + cw * i, y, cw, h))
                .collect()
        }
        ArrangeKind::TileVertical => {
            let ch = (h / n_i).max(TILE_MIN);
            (0..n_i)
                .map(|i| (x, y + ch * i, w, ch))
                .collect()
        }
    }
}

/// 主显示器工作区（不含任务栏）。
pub unsafe fn work_area() -> Rect4 {
    let mut wa = RECT::default();
    // SAFETY: 出参合法；SPI 无其他前提。
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut wa as *mut RECT as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    (wa.left, wa.top, wa.right - wa.left, wa.bottom - wa.top)
}

/// 收集参与排列的顶层窗：类名 = 本产品主窗类、可见、未最小化。
/// 排除本调用后由调用方决定是否保留本窗（默认保留，排第一）。
unsafe fn collect_windows() -> Vec<HWND> {
    struct Ctx {
        out: Vec<HWND>,
    }
    unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> windows::core::BOOL {
        let ctx = unsafe { &mut *(lp.0 as *mut Ctx) };
        // SAFETY: lp 指向栈上 Ctx，回调期间存活。
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
                return true.into();
            }
            let mut buf = [0u16; 64];
            let n = GetClassNameW(hwnd, &mut buf);
            if n > 0 && String::from_utf16_lossy(&buf[..n as usize]) == brand::WINDOW_CLASS {
                ctx.out.push(hwnd);
            }
        }
        true.into()
    }
    let mut ctx = Ctx { out: Vec::new() };
    // SAFETY: 回调与参数如上。
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut ctx as *mut Ctx as isize));
    }
    ctx.out
}

/// 对收集到的窗口应用排列；返回参与窗口数（0=无窗可排）。
///
/// # Safety
/// 仅操作本产品类名的顶层窗（SetWindowPos/GetWindowRect）。
pub unsafe fn arrange_windows(kind: ArrangeKind) -> usize {
    // SAFETY: 调用方在 UI 线程；内部均为窗口查询。
    let wins = unsafe { collect_windows() };
    if wins.is_empty() {
        return 0;
    }
    let n = wins.len();
    let work = unsafe { work_area() };
    let rects = arrange_rects(kind, work, n);
    for (h, (x, y, cw, ch)) in wins.iter().zip(rects.iter()) {
        // SAFETY: h 为有效顶层窗。
        unsafe {
            let _ = SetWindowPos(
                *h,
                None,
                *x,
                *y,
                *cw,
                *ch,
                SET_WINDOW_POS_FLAGS(SWP_NOZORDER.0),
            );
        }
    }
    n
}

/// 主窗当前矩形（钩子断言用）。
pub unsafe fn window_rect(h: HWND) -> Rect4 {
    // SAFETY: 出参合法。
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(h, &mut r);
    }
    (r.left, r.top, r.right - r.left, r.bottom - r.top)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 平铺等分与求和还原() {
        let work = (0, 0, 1200, 800);
        // 横向平铺：左右列等宽，求和 = 全宽。
        let hs = arrange_rects(ArrangeKind::TileHorizontal, work, 3);
        assert_eq!(hs.len(), 3);
        assert_eq!(hs.iter().map(|r| r.2).sum::<i32>(), 1200);
        assert_eq!(hs[0], (0, 0, 400, 800));
        assert_eq!(hs[2], (800, 0, 400, 800));
        // 纵向平铺：上下行等高。
        let vs = arrange_rects(ArrangeKind::TileVertical, work, 2);
        assert_eq!(vs[0], (0, 0, 1200, 400));
        assert_eq!(vs[1], (0, 400, 1200, 400));
    }

    #[test]
    fn 层叠阶梯与尺寸() {
        let rects = arrange_rects(ArrangeKind::Cascade, (0, 0, 1200, 800), 3);
        assert_eq!(rects[0], (0, 0, 900, 600), "3/4 尺寸");
        assert_eq!(rects[1], (24, 24, 900, 600), "阶梯 +24");
        assert_eq!(rects[2], (48, 48, 900, 600));
    }

    #[test]
    fn 最小边钳制与空集() {
        // 极窄工作区：平铺钳到 TILE_MIN（120）。
        let r = arrange_rects(ArrangeKind::TileHorizontal, (0, 0, 100, 100), 2);
        assert_eq!(r[0].2, TILE_MIN, "钳最小宽");
        assert!(arrange_rects(ArrangeKind::Cascade, (0, 0, 800, 600), 0).is_empty());
    }

    #[test]
    fn 单窗即满铺或四分之三() {
        let one_h = arrange_rects(ArrangeKind::TileHorizontal, (0, 0, 800, 600), 1);
        assert_eq!(one_h, vec![(0, 0, 800, 600)], "单窗横向平铺=满铺");
        let one_c = arrange_rects(ArrangeKind::Cascade, (0, 0, 800, 600), 1);
        assert_eq!(one_c, vec![(0, 0, 600, 450)], "单窗层叠=3/4");
    }
}
