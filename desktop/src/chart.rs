//! 图表绘制（柱状 / 折线）
//!
//! 从 `pages.rs` 拆出来单独立模块：加入 Y 轴刻度、网格线、X 标签抽稀与 hover
//! tooltip 后，这块代码量已经不小。
//!
//! 设计取舍（对齐 web Recharts 的观感）：
//! - 左侧固定 52px 轴区画 4 条 `border_light` 网格线 + 刻度值（`fmt_duration_short`）
//! - 标签多于 7 个时隔一显示，避免 14 天挤在窄窗里叠字
//! - hover 命中最近一列：抬高该列亮度 + 气泡显示「日期 + 完整时长」
//! - 空数据 / 全 0 走 `empty_state`，不画一排 0 高柱子

use egui::{Align2, Color32, FontId, Pos2, Rect, Rounding, Stroke, Vec2};

use crate::anim;
use crate::components;
use crate::fmt;
use crate::theme::{self, Palette};
use egui_phosphor::regular;

const CHART_H: f32 = 210.0;
/// 左侧轴区宽度（刻度值 + 单位）
const AXIS_W: f32 = 58.0;
/// 底部标签区高度
const LABEL_H: f32 = 22.0;
/// Y 轴刻度条数
const TICKS: usize = 4;

/// 一组图表数据：标签 + 数值 + 该值是否表示时长（决定刻度文案格式）
pub struct ChartData {
    pub items: Vec<(String, f64)>,
    pub as_duration: bool,
}

fn fmt_value(v: f64, as_duration: bool) -> String {
    if as_duration {
        fmt::fmt_duration_short(v as i64)
    } else {
        format!("{}", v.round() as i64)
    }
}

/// 空数据判定：无数据，或全部为 0（画出来只有一条底线，不如给空状态）
fn is_empty(data: &ChartData) -> bool {
    data.items.is_empty() || data.items.iter().all(|(_, v)| *v <= 0.0)
}

/// 计算绘图区（去掉轴区与标签区）
fn plot_rect(rect: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(rect.left() + AXIS_W, rect.top() + 8.0),
        Pos2::new(rect.right() - 8.0, rect.bottom() - LABEL_H),
    )
}

/// 画 Y 轴网格线与刻度
fn draw_axis(painter: &egui::Painter, plot: Rect, max: f64, p: &Palette, as_duration: bool) {
    let grid = theme::overlay(p.bg_page, p.text_primary, 0.08);
    for i in 0..=TICKS {
        let f = i as f32 / TICKS as f32;
        let y = plot.bottom() - f * plot.height();
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(1.0_f32, grid),
        );
        painter.text(
            Pos2::new(plot.left() - 8.0, y),
            Align2::RIGHT_CENTER,
            fmt_value(max * f as f64, as_duration),
            FontId::new(11.0, egui::FontFamily::Proportional),
            p.text_muted,
        );
    }
    painter.text(
        Pos2::new(plot.left() - AXIS_W + 6.0, plot.top() - 4.0),
        Align2::LEFT_TOP,
        if as_duration { crate::t!("时长") } else { crate::t!("次数") },
        FontId::new(11.0, egui::FontFamily::Proportional),
        p.text_disabled,
    );
}

/// X 轴标签抽稀：超过 7 个时隔一显示（首尾必显示）
fn show_label(i: usize, n: usize) -> bool {
    if n <= 7 {
        return true;
    }
    i == 0 || i == n - 1 || i.is_multiple_of(2)
}

/// hover 气泡：显示完整标签 + 完整数值
fn tooltip(
    ui: &mut egui::Ui,
    p: &Palette,
    anchor: Pos2,
    label: &str,
    value: f64,
    as_duration: bool,
    color: Color32,
) {
    // 走 `tf!` 而不是 `format!`：英文表里这两个模板分别是
    // `{label}   {}` / `{label}   {} times`，占位符按出现顺序替换。
    let text = if as_duration {
        crate::tf!("{label}   {}", label, fmt::fmt_duration(value as i64))
    } else {
        crate::tf!("{label}   {} 次", label, value.round() as i64)
    };
    egui::Area::new(egui::Id::new("chart_tip"))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor + Vec2::new(0.0, -34.0))
        .interactable(false)
        .show(ui.ctx(), |ui| {
            egui::Frame::none()
                .fill(p.bg_card)
                .stroke(Stroke::new(1.0_f32, color))
                .rounding(Rounding::same(theme::R_INPUT))
                .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(text)
                            .size(12.0)
                            .color(p.text_primary),
                    );
                });
        });
}

/// 命中列索引（按指针 x 落在哪一列/哪个区间）
fn hit_index(ui: &egui::Ui, plot: Rect, n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let pos = ui.ctx().pointer_hover_pos()?;
    if !plot.expand2(Vec2::new(6.0, 0.0)).contains(pos) {
        return None;
    }
    let step = plot.width() / n as f32;
    let idx = ((pos.x - plot.left()) / step).floor();
    if idx < 0.0 {
        Some(0)
    } else if idx as usize >= n {
        Some(n - 1)
    } else {
        Some(idx as usize)
    }
}

/// 柱状图
pub fn bar_chart(ui: &mut egui::Ui, data: &ChartData, color: Color32, p: &Palette) {
    if is_empty(data) {
        components::empty_state(
            ui,
            p,
            regular::CHART_BAR,
            crate::t!("暂无数据"),
            crate::t!("积累几次开机后，这里会出现趋势"),
        );
        return;
    }
    let avail = ui.available_width();
    let (_, rect) = ui.allocate_space(Vec2::new(avail, CHART_H));
    let plot = plot_rect(rect);
    let max = data
        .items
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let n = data.items.len();
    let hovered = hit_index(ui, plot, n);
    let painter = ui.painter().clone();
    draw_axis(&painter, plot, max, p, data.as_duration);

    let gap = 6.0;
    let bw = ((plot.width() - gap * (n as f32 - 1.0).max(0.0)) / n as f32).max(2.0);
    for (i, (label, v)) in data.items.iter().enumerate() {
        let bh = ((v / max) as f32) * plot.height();
        let x = plot.left() + i as f32 * (bw + gap);
        let is_hover = hovered == Some(i);
        let c = if is_hover {
            color
        } else {
            theme::overlay(p.bg_page, color, 0.72)
        };
        let bar = Rect::from_min_max(
            Pos2::new(x, plot.bottom() - bh),
            Pos2::new(x + bw, plot.bottom()),
        );
        painter.rect_filled(bar, Rounding::same(3.0), c);
        if show_label(i, n) || is_hover {
            painter.text(
                Pos2::new(x + bw / 2.0, plot.bottom() + 4.0),
                Align2::CENTER_TOP,
                label,
                FontId::new(11.0, egui::FontFamily::Proportional),
                if is_hover { p.text_primary } else { p.text_muted },
            );
        }
        if is_hover {
            tooltip(ui, p, Pos2::new(x + bw / 2.0, plot.top()), label, *v, data.as_duration, color);
        }
    }
}

/// 折线图
pub fn line_chart(ui: &mut egui::Ui, data: &ChartData, color: Color32, p: &Palette) {
    if is_empty(data) {
        components::empty_state(
            ui,
            p,
            regular::CHART_LINE_UP,
            crate::t!("暂无数据"),
            crate::t!("积累几次开机后，这里会出现趋势"),
        );
        return;
    }
    let avail = ui.available_width();
    let (_, rect) = ui.allocate_space(Vec2::new(avail, CHART_H));
    let plot = plot_rect(rect);
    let max = data
        .items
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let n = data.items.len();
    let hovered = hit_index(ui, plot, n);
    let painter = ui.painter().clone();
    draw_axis(&painter, plot, max, p, data.as_duration);

    let pts: Vec<Pos2> = data
        .items
        .iter()
        .enumerate()
        .map(|(i, (_, v))| {
            let x = if n > 1 {
                plot.left() + i as f32 / (n - 1) as f32 * plot.width()
            } else {
                plot.center().x
            };
            let y = plot.bottom() - ((v / max) as f32) * plot.height();
            Pos2::new(x, y)
        })
        .collect();
    // egui 0.29 已移除 Shape::line，改用逐段 line_segment
    let stroke = Stroke::new(2.0_f32, color);
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
    for (i, (label, v)) in data.items.iter().enumerate() {
        let is_hover = hovered == Some(i);
        let pt = pts[i];
        if is_hover {
            painter.circle_filled(pt, 5.5, theme::accent_tint(color, 60));
        }
        painter.circle_filled(pt, if is_hover { 3.5 } else { 2.5 }, color);
        if show_label(i, n) || is_hover {
            painter.text(
                Pos2::new(pt.x, plot.bottom() + 4.0),
                Align2::CENTER_TOP,
                label,
                FontId::new(11.0, egui::FontFamily::Proportional),
                if is_hover { p.text_primary } else { p.text_muted },
            );
        }
        if is_hover {
            tooltip(ui, p, Pos2::new(pt.x, pt.y), label, *v, data.as_duration, color);
        }
    }
}

/// 滚动到图表时的入场进度（供页面做淡入）
pub fn enter_alpha(ctx: &egui::Context, anim_start: f64) -> f32 {
    anim::progress(anim::now(ctx), anim_start, 0.0, anim::DUR_PAGE)
}
