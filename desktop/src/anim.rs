//! 动效工具集——egui 无 CSS 动画，靠「手动时间插值 + 主动重绘」模拟。
//!
//! 本模块只提供**纯计算与绘制**函数，动画的起止时刻由 `BootTrackerApp` 持有，
//! 避免把可变状态藏进全局（即时模式下极易泄漏）。
//!
//! 对齐 web 端的设计规范：
//! - 缓动统一非线性（这里用 ease-out-expo：`1 - 2^(-10t)`）
//! - 页面入场 0.34s、位移 20px → 0
//! - 列表逐行阶梯 0.03s，上限 0.45s（长列表最后几行不必苦等）

use egui::{Color32, Context, Painter, Pos2, Stroke, Vec2};

/// 页面入场时长（秒）
pub const DUR_PAGE: f32 = 0.34;
/// 单行入场时长（秒）
pub const DUR_ROW: f32 = 0.26;
/// 逐行延迟步长（秒）
pub const ROW_STEP: f32 = 0.03;
/// 逐行延迟上限（秒）
pub const ROW_MAX_DELAY: f32 = 0.45;
/// 入场位移（px），从下方 20px 滑到 0
pub const RISE_OFFSET: f32 = 20.0;

/// 当前时间（秒，自程序启动起算）
pub fn now(ctx: &Context) -> f64 {
    ctx.input(|i| i.time)
}

/// ease-out-expo：起步快、收尾稳，与 web 的非线性缓动观感一致
pub fn ease_out(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        1.0 - 2f32.powf(-10.0 * t)
    }
}

/// 入场进度 0→1：`start` 为入场起始时刻，`delay` 为该元素的错峰延迟
pub fn progress(now: f64, start: f64, delay: f32, dur: f32) -> f32 {
    ease_out(((now - start) as f32 - delay) / dur)
}

/// 第 index 行的错峰延迟（带上限，避免长列表尾部等待过久）
pub fn row_delay(index: usize) -> f32 {
    (index as f32 * ROW_STEP).min(ROW_MAX_DELAY)
}

/// 一次完整入场（页面 + 最慢的一行）所需的总时长，用于判断是否还需继续重绘
pub fn total() -> f32 {
    DUR_PAGE + ROW_MAX_DELAY + DUR_ROW
}

/// 旋转指示器：12 段圆弧 + 透明度衰减。
///
/// egui 无法旋转字形（`Painter::text` 不支持变换），所以不用静态图标做加载态，
/// 直接画一个会转的圆弧环——这也是唯一需要持续重绘的动效。
pub fn spinner(painter: &Painter, center: Pos2, r: f32, t: f64, color: Color32) {
    const N: usize = 12;
    let base = (t * 1.6) as f32 * std::f32::consts::TAU;
    let seg = std::f32::consts::TAU / N as f32;
    for i in 0..N {
        let frac = i as f32 / N as f32;
        let a0 = base + frac * std::f32::consts::TAU;
        let a1 = a0 + seg * 0.62;
        let alpha = (40.0 + 205.0 * frac) as u8;
        let c = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);
        painter.line_segment(
            [center + Vec2::angled(a0) * r, center + Vec2::angled(a1) * r],
            Stroke::new(3.0_f32, c),
        );
    }
}
