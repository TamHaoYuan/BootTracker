//! 五个主页面的原生渲染（egui 即时模式）

use chrono::Utc;
use egui::{Align2, Color32, FontId, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

use crate::anim;
use crate::api::{Anomalies, AppSettings, BootSession, DailyStat, Overview, TrendDay, WeeklyStat};
use crate::app::{BootTrackerApp, RecordsView};
use crate::fmt;
use crate::theme::{Palette, R_CARD, R_PANEL};
use egui_phosphor::regular;

// ===================== 通用组件 =====================

/// 图标 + 文字混排。
///
/// Phosphor 图标与中文字体同属 `FontFamily::Proportional`，egui 会逐字回落：
/// 图标的 PUA 码位中文命不中 → 用 Phosphor 字形；中文命中 → 用 CJK 字形。
/// 所以一条字符串里可以直接混排，不必拆成两个 label。
fn ic(icon: &str, text: &str) -> RichText {
    RichText::new(format!("{icon}  {text}"))
}

/// 带图标的按钮（默认按钮字号 15，与图标视觉重量匹配）
fn icon_button(ui: &mut egui::Ui, icon: &str, text: &str) -> egui::Response {
    ui.button(ic(icon, text))
}

/// 空状态引导卡
///
/// 对齐 web `EmptyState.tsx`（Vercel 模式）：图标 + 标题 + 说明。
/// web 源码注释：「空状态占感知精致度 80%」——无数据时展示引导，而不是一行灰字。
pub fn empty_state(ui: &mut egui::Ui, p: &Palette, icon: &str, title: &str, desc: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(RichText::new(icon).size(36.0).color(p.text_disabled));
        ui.add_space(6.0);
        ui.label(
            RichText::new(title)
                .size(15.0)
                .strong()
                .color(p.text_secondary),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(desc).size(13.0).color(p.text_muted));
        ui.add_space(24.0);
    });
}

/// 加载态：与 `empty_state` 同构，但图标换成**会转的**圆弧指示器。
///
/// egui 无法旋转字形，静态的 `CIRCLE_NOTCH` 看着像卡住了；这里用 `anim::spinner`
/// 画一圈透明度递减的圆弧并持续重绘，读起来才像「正在加载」。
pub fn loading_state(ui: &mut egui::Ui, p: &Palette, title: &str, desc: &str) {
    let t = anim::now(ui.ctx());
    ui.ctx().request_repaint();
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
        anim::spinner(ui.painter(), rect.center(), 15.0, t, p.text_disabled);
        ui.add_space(6.0);
        ui.label(
            RichText::new(title)
                .size(15.0)
                .strong()
                .color(p.text_secondary),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(desc).size(13.0).color(p.text_muted));
        ui.add_space(24.0);
    });
}

/// 在矩形顶部画 1px 内高光，近似 web `inset 0 1px 0 var(--glass-light)`。
///
/// egui 没有 backdrop-filter，玻璃质感主要靠这根亮线 + 1px 描边撑起来。
fn inner_highlight(painter: &egui::Painter, rect: Rect, p: &Palette) {
    let inset = 8.0;
    painter.line_segment(
        [
            Pos2::new(rect.left() + inset, rect.top() + 1.0),
            Pos2::new(rect.right() - inset, rect.top() + 1.0),
        ],
        Stroke::new(1.0_f32, p.glass_light),
    );
}

// ===================== 图表绘制 =====================

fn bar_chart(ui: &mut egui::Ui, items: &[(String, f64)], color: Color32, p: &Palette) {
    if items.is_empty() {
        empty_state(
            ui,
            p,
            regular::CHART_BAR,
            "暂无数据",
            "积累几次开机后，这里会出现趋势",
        );
        return;
    }
    let avail = ui.available_width();
    let h = 200.0;
    // egui 0.29: allocate_space 返回 (Id, Rect)，顺序不要写反
    let (_, rect) = ui.allocate_space(Vec2::new(avail, h));
    let painter = ui.painter();
    let max = items
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let n = items.len() as f32;
    let gap = 6.0;
    let bw = (rect.width() - gap * (n - 1.0).max(0.0)) / n.max(1.0);
    for (i, (label, v)) in items.iter().enumerate() {
        let bh = ((v / max) as f32) * (rect.height() - 24.0);
        let x = rect.left() + i as f32 * (bw + gap);
        let bar = Rect::from_min_max(
            Pos2::new(x, rect.bottom() - 20.0 - bh),
            Pos2::new(x + bw, rect.bottom() - 20.0),
        );
        painter.rect_filled(bar, Rounding::same(2.0), color);
        painter.text(
            Pos2::new(x + bw / 2.0, rect.bottom() - 16.0),
            Align2::CENTER_BOTTOM,
            label,
            FontId::new(11.0, egui::FontFamily::Proportional),
            Color32::GRAY,
        );
    }
}

fn line_chart(ui: &mut egui::Ui, items: &[(String, f64)], color: Color32, p: &Palette) {
    if items.is_empty() {
        empty_state(
            ui,
            p,
            regular::CHART_LINE_UP,
            "暂无数据",
            "积累几次开机后，这里会出现趋势",
        );
        return;
    }
    let avail = ui.available_width();
    let h = 200.0;
    // egui 0.29: allocate_space 返回 (Id, Rect)
    let (_, rect) = ui.allocate_space(Vec2::new(avail, h));
    let painter = ui.painter();
    let max = items
        .iter()
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let n = items.len();
    let pts: Vec<Pos2> = items
        .iter()
        .enumerate()
        .map(|(i, (_, v))| {
            let x = if n > 1 {
                rect.left() + i as f32 / (n - 1) as f32 * rect.width()
            } else {
                rect.left()
            };
            let y = rect.bottom() - 20.0 - ((v / max) as f32) * (rect.height() - 24.0);
            Pos2::new(x, y)
        })
        .collect();
    // egui 0.29 已移除 Shape::line，改用逐段 line_segment + 数据点
    let stroke = Stroke::new(2.0_f32, color);
    for w in pts.windows(2) {
        painter.line_segment([w[0], w[1]], stroke);
    }
    for p in &pts {
        painter.circle_filled(*p, 2.5, color);
    }
}

// ===================== 仪表盘 =====================

pub fn dashboard(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    if app.loading && app.data.is_none() {
        loading_state(ui, &p, "正在加载数据", "首次启动需要一点时间");
        return;
    }
    ui.heading("总览");
    ui.separator();

    // KPI 卡片
    if let Some(o) = &app.overview {
        ui.horizontal_wrapped(|ui| {
            kpi_card(
                ui,
                &p,
                "累计开机",
                &o.total_boot.to_string(),
                None,
                false,
                false,
            );
            kpi_card(
                ui,
                &p,
                "累计关机",
                &o.total_shutdown.to_string(),
                None,
                false,
                false,
            );
            kpi_card(
                ui,
                &p,
                "累计时长",
                &fmt::fmt_duration(o.total_duration),
                None,
                false,
                false,
            );
            kpi_card(
                ui,
                &p,
                "平均时长",
                &fmt::fmt_duration(o.avg_duration),
                None,
                false,
                false,
            );
            // 有进行中会话时用 accent 色强调（对应 web KpiCard 的 accent prop）
            kpi_card(
                ui,
                &p,
                "进行中",
                &o.active_count.to_string(),
                None,
                false,
                o.active_count > 0,
            );
        });
    }

    ui.add_space(10.0);
    ui.heading("近 14 天开机时长");
    let trend_items: Vec<(String, f64)> = app
        .trend
        .as_ref()
        .map(|t| {
            let mut days: Vec<&String> = t.data.keys().collect();
            days.sort();
            days.into_iter()
                .rev()
                .take(14)
                .rev()
                .map(|d| {
                    let v = t
                        .data
                        .get(d)
                        .map(|x| x.total_duration as f64)
                        .unwrap_or(0.0);
                    (d[5..].to_string(), v)
                })
                .collect()
        })
        .unwrap_or_default();
    bar_chart(ui, &trend_items, p.accent, &p);

    ui.add_space(10.0);
    ui.heading("本次会话");
    let now = Utc::now();
    let active = app.data.as_ref().and_then(|d| {
        d.sessions
            .iter()
            .filter(|s| s.shutdown_time.is_none())
            .max_by_key(|s| s.boot_time.clone())
    });
    match active {
        Some(s) => {
            if let Some(bt) = fmt::parse_utc(&s.boot_time) {
                let dur = (now - bt).num_milliseconds();
                let since = format!("自 {}", fmt::fmt_datetime(&s.boot_time));
                // hero 主卡：对应 web 的「今日开机」卡（accent 底 + 40px 大数值）
                kpi_card(
                    ui,
                    &p,
                    "本次会话",
                    &fmt::fmt_duration(dur),
                    Some((since.as_str(), p.text_muted)),
                    true,
                    true,
                );
            }
        }
        None => {
            empty_state(
                ui,
                &p,
                regular::CLOCK,
                "当前无进行中的会话",
                "下次开机后会自动开始计时",
            );
        }
    }

    ui.add_space(10.0);
    ui.heading("最近记录");
    if let Some(o) = &app.overview {
        let count = o.recent_sessions.len();
        for (i, s) in o.recent_sessions.iter().enumerate() {
            // 逐行错峰入场（对齐 web 表格的阶梯动画）
            let t = app.row_t(ui.ctx(), i);
            ui.scope(|ui| {
                ui.set_opacity(t);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(regular::CLOCK).color(p.text_muted));
                    ui.label(fmt::fmt_datetime(&s.boot_time));
                    ui.label(match &s.shutdown_time {
                        Some(t) => fmt::fmt_datetime(t),
                        None => "进行中".to_string(),
                    });
                    ui.label(
                        RichText::new(match s.duration {
                            Some(d) => fmt::fmt_duration(d),
                            None => "-".to_string(),
                        })
                        .font(FontId::new(14.0, egui::FontFamily::Monospace)),
                    );
                });
            });
        }
        if count == 0 {
            empty_state(
                ui,
                &p,
                regular::NOTE,
                "暂无记录",
                "开机后会自动记录一条会话",
            );
        }
    }
}

/// KPI 卡（对齐 web `KpiCard.tsx`）
///
/// 公式：标签（12px muted / hero 13px）+ 大数值（26px/700，hero 40px）+ 可选趋势行。
/// 趋势用「箭头 + 语义色」双编码（色盲可用），对应 web 的 `ArrowUp/Down/Minus` + trend-* 类。
/// hero 变体对应 web 的「今日开机」主卡：accent 低透明度底 + accent 描边。
fn kpi_card(
    ui: &mut egui::Ui,
    p: &Palette,
    title: &str,
    value: &str,
    trend: Option<(&str, Color32)>,
    hero: bool,
    accent: bool,
) -> egui::Response {
    // web hero 卡是 135° 渐变，egui 无原生渐变 → 用 accent 低透明度实色近似。
    // 注意 mica 等主题的 accent 接近纯白：hero 底色若仍用 46α 会变成一块白板，
    // 白字压白底完全读不清——按 accent 亮度自适应降低叠加浓度。
    let accent_lum =
        0.299 * p.accent.r() as f32 + 0.587 * p.accent.g() as f32 + 0.114 * p.accent.b() as f32;
    let hero_alpha = if accent_lum > 180.0 { 26 } else { 46 };
    let (fill, stroke_color) = if hero {
        (
            Color32::from_rgba_premultiplied(p.accent.r(), p.accent.g(), p.accent.b(), hero_alpha),
            Color32::from_rgba_premultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 90),
        )
    } else {
        (p.bg_card, p.border_light)
    };
    let title_size = if hero { 13.0 } else { 12.0 };
    let value_size = if hero { 40.0 } else { 26.0 };

    // 用 layout_no_wrap 精确测量文本（纯计算、无 Ui 副作用）。
    // 之前用 sizing_pass 探针在 wrapped 布局下量出了过小的宽度，导致卡片内
    // 文字逐字换行并溢出卡片、压到下面的标题——正是「文字错位」的来源。
    let title_gal = ui.painter().layout_no_wrap(
        title.to_owned(),
        FontId::new(title_size, egui::FontFamily::Proportional),
        Color32::WHITE,
    );
    // 数值用等宽族（tabular 数字），避免实时计时时数字宽度跳动
    let value_gal = ui.painter().layout_no_wrap(
        value.to_owned(),
        FontId::new(value_size, egui::FontFamily::Monospace),
        Color32::WHITE,
    );
    let trend_gal = trend.map(|(t, _)| {
        ui.painter().layout_no_wrap(
            t.to_owned(),
            FontId::new(12.0, egui::FontFamily::Proportional),
            Color32::WHITE,
        )
    });

    let trend_w = trend_gal.as_ref().map(|g| g.size().x).unwrap_or(0.0);
    let inner_w = title_gal.size().x.max(value_gal.size().x).max(trend_w);
    let inner_h = title_gal.size().y
        + 2.0
        + value_gal.size().y
        + trend_gal.as_ref().map(|g| 2.0 + g.size().y).unwrap_or(0.0);
    // 纵向布局（hero 独占一行）撑满可用宽度；横向排列按内容收缩
    let w = if ui.layout().is_vertical() {
        ui.available_width()
    } else {
        inner_w + 36.0
    };
    let size = Vec2::new(w, inner_h + 32.0);

    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let hovered = resp.hovered();
    let rect = rect.translate(Vec2::new(0.0, if hovered { -3.0 } else { 0.0 }));
    let painter = ui.painter().clone();
    painter.rect_filled(rect, Rounding::same(R_CARD), fill);
    if hovered {
        painter.rect_filled(
            rect,
            Rounding::same(R_CARD),
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 22),
        );
    }
    painter.rect_stroke(
        rect,
        Rounding::same(R_CARD),
        Stroke::new(1.0_f32, if hovered { p.accent } else { stroke_color }),
    );
    inner_highlight(&painter, rect, p);

    // 内容用 painter 按测量结果直接定位，与卡片矩形严格一致，绝无换行溢出
    let x = rect.left() + 18.0;
    let mut y = rect.top() + 16.0;
    painter.text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        title,
        FontId::new(title_size, egui::FontFamily::Proportional),
        if hero { p.text_secondary } else { p.text_muted },
    );
    y += title_gal.size().y + 2.0;
    painter.text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        value,
        FontId::new(value_size, egui::FontFamily::Monospace),
        if accent { p.accent } else { p.text_primary },
    );
    if let (Some((t, c)), Some(g)) = (trend, trend_gal) {
        y += value_gal.size().y + 2.0;
        painter.text(
            Pos2::new(x, y),
            Align2::LEFT_TOP,
            t,
            FontId::new(12.0, egui::FontFamily::Proportional),
            c,
        );
        let _ = g;
    }
    resp
}

// ===================== 记录 =====================

pub fn records(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    ui.heading("开机记录");
    ui.separator();

    // 过滤与操作
    ui.horizontal_wrapped(|ui| {
        ui.label(ic(regular::MAGNIFYING_GLASS, "搜索"));
        ui.text_edit_singleline(&mut app.filter_text);
        ui.label(ic(regular::CALENDAR, "起"));
        ui.text_edit_singleline(&mut app.filter_from);
        ui.label(ic(regular::CALENDAR, "止"));
        ui.text_edit_singleline(&mut app.filter_to);
        ui.separator();
        ui.selectable_value(
            &mut app.records_view,
            RecordsView::Table,
            ic(regular::TABLE, "表格"),
        );
        ui.selectable_value(
            &mut app.records_view,
            RecordsView::Timeline,
            ic(regular::CLOCK_COUNTER_CLOCKWISE, "时间轴"),
        );
        ui.separator();
        if icon_button(ui, regular::ARROWS_CLOCKWISE, "刷新").clicked() {
            app.refresh_data();
        }
        if icon_button(ui, regular::UPLOAD_SIMPLE, "导入").clicked() {
            app.import_json();
        }
        let owned: Vec<BootSession> = app.filtered_sessions();
        if icon_button(ui, regular::FILE_CSV, "导出CSV").clicked() {
            app.export_csv(&owned);
        }
        if icon_button(ui, regular::FILE_XLS, "导出XLSX").clicked() {
            app.export_xlsx(&owned);
        }
        if icon_button(ui, regular::ARROWS_MERGE, "合并选中").clicked() {
            app.request_merge();
        }
    });

    ui.separator();
    let sessions = app.filtered_sessions();
    ui.label(format!("共 {} 条", sessions.len()));

    if sessions.is_empty() {
        let p = app.palette();
        empty_state(
            ui,
            &p,
            regular::NOTE,
            "暂无开机记录",
            "积累几次开机后，这里会出现完整的时间表",
        );
        return;
    }

    if app.records_view == RecordsView::Table {
        egui::ScrollArea::vertical().show(ui, |ui| {
            records_table(ui, app, &sessions);
        });
    } else {
        // 时间轴：按日期分组
        let muted = app.palette().text_muted;
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut last_date = String::new();
            for (i, s) in sessions.iter().enumerate() {
                let date = fmt::fmt_date(&s.boot_time);
                if date != last_date {
                    last_date = date.clone();
                    ui.add_space(6.0);
                    ui.label(RichText::new(date).strong());
                }
                // 逐行错峰入场
                let t = app.row_t(ui.ctx(), i);
                ui.scope(|ui| {
                    ui.set_opacity(t);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(regular::POWER).color(muted));
                        ui.label(fmt::fmt_datetime(&s.boot_time));
                        ui.label(match &s.shutdown_time {
                            Some(t) => fmt::fmt_datetime(t),
                            None => "进行中".to_string(),
                        });
                        ui.label(
                            RichText::new(match s.duration {
                                Some(d) => fmt::fmt_duration(d),
                                None => "-".to_string(),
                            })
                            .font(FontId::new(14.0, egui::FontFamily::Monospace)),
                        );
                        if icon_button(ui, regular::TRASH, "删除").clicked() {
                            app.request_delete_session(s.id.clone());
                        }
                    });
                });
            }
        });
    }
}

/// 表格单元格：固定宽高 + 垂直居中，保证各行列对齐。
///
/// 两个关键点：
/// 1. `set_min_size` 占满整格——`allocate_new_ui` 回收到父光标的只有 `min_rect`
///    （内容实际宽度），不撑满的话列宽会缩成文字宽，整表挤成一团；
/// 2. 强制 `Truncate`——ID 之类超长文本会溢出单元格压到下一列，必须截断。
fn cell(ui: &mut egui::Ui, w: f32, h: f32, content: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(w, h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(Vec2::new(w, h));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            content(ui);
        },
    );
}

/// 记录表格
///
/// 不用 `egui::Grid`：Grid 拿不到行矩形，做不了**容器 16px 圆角、表头、整行 hover
/// 底色**这三件事。这里按窗口宽度按比例分配列宽手工布局，换来完整观感：
/// 容器圆角 + 内高光 → 表头（12px muted）→ 分隔线 → 逐行（hover 高亮 / 斑马纹 / 错峰入场）。
fn records_table(ui: &mut egui::Ui, app: &mut BootTrackerApp, sessions: &[BootSession]) {
    const ROW_H: f32 = 34.0;
    const HEAD_H: f32 = 32.0;
    const PAD: f32 = 12.0;
    /// 勾选框列：必须 ≥ theme 的 `spacing.interact_size.x`(48)，否则复选框会挤进 ID 列
    const SEL_W: f32 = 52.0;
    /// 操作列：容纳「图标 删除」按钮（文字宽 + 两侧 button_padding）
    const ACT_W: f32 = 112.0;
    /// 中间 4 列（ID / 开机 / 关机 / 时长）按剩余宽度等比分配
    const MID_RATIO: [f32; 4] = [0.20, 0.28, 0.28, 0.24];

    let p = app.palette();
    let avail = ui.available_width();
    let body_w = (avail - PAD * 2.0).max(SEL_W + ACT_W + 200.0);
    let rest = body_w - SEL_W - ACT_W;
    let widths: Vec<f32> = std::iter::once(SEL_W)
        .chain(MID_RATIO.iter().map(|r| rest * r))
        .chain(std::iter::once(ACT_W))
        .collect();
    let total_w = PAD * 2.0 + SEL_W + rest + ACT_W;
    // 表头与数据行必须共用同一个内容矩形，否则列会左右错开
    let content_w = total_w - PAD * 2.0;
    let total_h = PAD * 2.0 + HEAD_H + sessions.len() as f32 * ROW_H;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, total_h), egui::Sense::hover());
    let painter = ui.painter().clone();
    painter.rect_filled(rect, Rounding::same(R_PANEL), p.bg_card);
    painter.rect_stroke(
        rect,
        Rounding::same(R_PANEL),
        Stroke::new(1.0_f32, p.border_light),
    );
    inner_highlight(&painter, rect, &p);

    let mut y = rect.top() + PAD;
    // ---- 表头 ----
    let head_rect = Rect::from_min_size(
        Pos2::new(rect.left() + PAD, y),
        Vec2::new(content_w, HEAD_H),
    );
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .id_salt("records_head")
            .max_rect(head_rect),
        |ui| {
            ui.horizontal(|ui| {
                // 列宽已算进 cell，去掉单元格之间的自动间距，否则整行会被撑出容器
                ui.spacing_mut().item_spacing.x = 0.0;
                for (i, h) in ["", "ID", "开机时间", "关机时间", "时长", "操作"]
                    .iter()
                    .enumerate()
                {
                    cell(ui, widths[i], HEAD_H, |ui| {
                        ui.label(RichText::new(*h).size(12.0).strong().color(p.text_muted));
                    });
                }
            });
        },
    );
    y += HEAD_H;
    painter.line_segment(
        [
            Pos2::new(rect.left() + PAD, y),
            Pos2::new(rect.right() - PAD, y),
        ],
        Stroke::new(1.0_f32, p.border_light),
    );

    // ---- 数据行 ----
    for (i, s) in sessions.iter().enumerate() {
        let row_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 1.0, y),
            Vec2::new(rect.width() - 2.0, ROW_H),
        );
        let hovered = ui
            .interact(
                row_rect,
                egui::Id::new(("records_row", s.id.as_str())),
                egui::Sense::hover(),
            )
            .hovered();
        if hovered {
            // hover：极淡 accent 底（对齐 web 的 rgba(accent,0.06)）
            painter.rect_filled(
                row_rect,
                Rounding::same(8.0),
                Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 24),
            );
        } else if i % 2 == 1 {
            // 斑马纹：比 Grid::striped 更淡，避免抢走 hover 的注意力
            painter.rect_filled(
                row_rect,
                Rounding::same(8.0),
                Color32::from_rgba_unmultiplied(
                    p.text_primary.r(),
                    p.text_primary.g(),
                    p.text_primary.b(),
                    8,
                ),
            );
        }

        // 逐行错峰入场。注意：内容矩形与表头同源（left+PAD / 宽 content_w），
        // hover 底色才用铺满容器的 row_rect —— 两者不能混用，否则列会错位。
        let t = app.row_t(ui.ctx(), i);
        let content_rect =
            Rect::from_min_size(Pos2::new(rect.left() + PAD, y), Vec2::new(content_w, ROW_H));
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .id_salt(("records_row_ui", s.id.as_str()))
                .max_rect(content_rect),
            |ui| {
                ui.set_opacity(t);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    cell(ui, widths[0], ROW_H, |ui| {
                        let mut checked = app.selected.contains(&s.id);
                        if ui.checkbox(&mut checked, "").changed() {
                            if checked {
                                app.selected.insert(s.id.clone());
                            } else {
                                app.selected.remove(&s.id);
                            }
                        }
                    });
                    cell(ui, widths[1], ROW_H, |ui| {
                        ui.label(&s.id);
                    });
                    cell(ui, widths[2], ROW_H, |ui| {
                        ui.label(fmt::fmt_datetime(&s.boot_time));
                    });
                    cell(ui, widths[3], ROW_H, |ui| {
                        ui.label(match &s.shutdown_time {
                            Some(t) => fmt::fmt_datetime(t),
                            None => "进行中".to_string(),
                        });
                    });
                    cell(ui, widths[4], ROW_H, |ui| {
                        // 时长用等宽族，数字宽度稳定，列不会左右抖
                        ui.label(
                            RichText::new(match s.duration {
                                Some(d) => fmt::fmt_duration(d),
                                None => "-".to_string(),
                            })
                            .font(FontId::new(14.0, egui::FontFamily::Monospace)),
                        );
                    });
                    cell(ui, widths[5], ROW_H, |ui| {
                        if icon_button(ui, regular::TRASH, "删除").clicked() {
                            app.request_delete_session(s.id.clone());
                        }
                    });
                });
            },
        );
        y += ROW_H;
    }
}

// ===================== 图表 =====================

pub fn charts(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    ui.heading("图表分析");
    ui.separator();

    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.chart_mode, 0, ic(regular::CHART_BAR, "柱状图"));
        ui.selectable_value(&mut app.chart_mode, 1, ic(regular::CHART_LINE_UP, "折线图"));
    });
    let p = app.palette();
    let color = p.accent;

    ui.add_space(8.0);
    ui.heading("每日开机时长");
    let daily_items: Vec<(String, f64)> = app
        .daily
        .as_ref()
        .map(|m| {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            keys.into_iter()
                .rev()
                .take(14)
                .rev()
                .map(|k| {
                    let v = m
                        .get(k)
                        .map(|d: &DailyStat| d.total_duration as f64)
                        .unwrap_or(0.0);
                    (k[5..].to_string(), v)
                })
                .collect()
        })
        .unwrap_or_default();
    if app.chart_mode == 0 {
        bar_chart(ui, &daily_items, color, &p);
    } else {
        line_chart(ui, &daily_items, color, &p);
    }

    ui.add_space(8.0);
    ui.heading("每周开机次数");
    let weekly_items: Vec<(String, f64)> = app
        .weekly
        .as_ref()
        .map(|m| {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            keys.into_iter()
                .map(|k| {
                    let v = m
                        .get(k)
                        .map(|w: &WeeklyStat| w.boot_count as f64)
                        .unwrap_or(0.0);
                    (k.clone(), v)
                })
                .collect()
        })
        .unwrap_or_default();
    bar_chart(ui, &weekly_items, p.success, &p);

    ui.add_space(8.0);
    ui.heading("异常检测");
    if let Some(a) = &app.anomalies {
        ui.label(format!(
            "平均时长 {}，阈值 高 {} / 低 {}",
            fmt::fmt_duration(a.avg_duration),
            fmt::fmt_duration(a.threshold_high),
            fmt::fmt_duration(a.threshold_low)
        ));
        ui.collapsing("异常偏长", |ui| {
            for s in &a.longest {
                ui.label(format!(
                    "{}  {}",
                    fmt::fmt_datetime(&s.boot_time),
                    fmt::fmt_duration(s.duration.unwrap_or(0))
                ));
            }
        });
        ui.collapsing("异常偏短", |ui| {
            for s in &a.shortest {
                ui.label(format!(
                    "{}  {}",
                    fmt::fmt_datetime(&s.boot_time),
                    fmt::fmt_duration(s.duration.unwrap_or(0))
                ));
            }
        });
    } else if app.loading {
        loading_state(ui, &p, "正在分析异常", "需要足够的记录才能计算阈值");
    } else {
        empty_state(
            ui,
            &p,
            regular::SEAL_CHECK,
            "暂无异常",
            "记录足够后会自动计算阈值",
        );
    }
}

// ===================== 设置 =====================

pub fn settings(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    ui.heading("设置");
    ui.separator();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.settings_tab, 0, ic(regular::PALETTE, "外观"));
        ui.selectable_value(&mut app.settings_tab, 1, ic(regular::GEAR_SIX, "通用"));
        ui.selectable_value(&mut app.settings_tab, 2, ic(regular::PLUG, "网络"));
        ui.selectable_value(&mut app.settings_tab, 3, ic(regular::INFO, "关于"));
    });
    ui.separator();

    match app.settings_tab {
        0 => settings_appearance(ui, app),
        1 => settings_general(ui, app),
        2 => settings_network(ui, app),
        _ => settings_about(ui, app),
    }
}

fn settings_appearance(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let mode = app
        .settings
        .as_ref()
        .map(|s| s.app_mode.clone())
        .unwrap_or_else(|| "dark".into());
    ui.horizontal(|ui| {
        ui.label(ic(regular::PALETTE, "主题"));
        if ui
            .selectable_label(mode == "dark", ic(regular::MOON, "暗色"))
            .clicked()
            && mode != "dark"
        {
            app.set_theme("dark");
            if let Some(s) = app.settings.as_mut() {
                s.app_mode = "dark".into();
            }
        }
        if ui
            .selectable_label(mode == "light", ic(regular::SUN, "亮色"))
            .clicked()
            && mode != "light"
        {
            app.set_theme("light");
            if let Some(s) = app.settings.as_mut() {
                s.app_mode = "light".into();
            }
        }
    });
    let chart_type = app
        .settings
        .as_ref()
        .map(|s| s.default_chart_type.clone())
        .unwrap_or_else(|| "bar".into());
    ui.horizontal(|ui| {
        ui.label(ic(regular::CHART_LINE_UP, "默认图表类型"));
        if ui
            .selectable_label(chart_type == "bar", ic(regular::CHART_BAR, "柱状"))
            .clicked()
            && chart_type != "bar"
        {
            app.save_setting("defaultChartType", serde_json::json!("bar"), "图表类型");
        }
        if ui
            .selectable_label(chart_type == "line", ic(regular::CHART_LINE_UP, "折线"))
            .clicked()
            && chart_type != "line"
        {
            app.save_setting("defaultChartType", serde_json::json!("line"), "图表类型");
        }
    });
}

fn settings_general(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    // 取一份**拥有所有权**的快照，避免 &app.settings 的不可变借用
    // 跨越后续 app.settings.as_mut() 的可变借用
    let s = match app.settings.clone() {
        Some(s) => s,
        None => {
            // 首帧/请求中才转圈；否则说明拿不到设置，给静态提示（避免无限重绘）
            if app.loading {
                loading_state(ui, &app.palette(), "正在加载设置", "请稍候…");
            } else {
                empty_state(
                    ui,
                    &app.palette(),
                    regular::WARNING,
                    "未能加载设置",
                    "请确认后端服务正在运行",
                );
            }
            return;
        }
    };
    let auto_start = s.auto_start;
    let mut v = auto_start;
    if ui.checkbox(&mut v, "开机自启").changed() {
        app.save_setting("autoStart", serde_json::json!(v), "开机自启");
        if let Some(st) = app.settings.as_mut() {
            st.auto_start = v;
        }
    }
    let auto_backup = s.auto_backup;
    let mut v = auto_backup;
    if ui.checkbox(&mut v, "自动备份").changed() {
        app.save_setting("autoBackup", serde_json::json!(v), "自动备份");
        if let Some(st) = app.settings.as_mut() {
            st.auto_backup = v;
        }
    }
    let auto_close = s.auto_close_idle;
    let mut v = auto_close;
    if ui.checkbox(&mut v, "空闲自动关闭会话").changed() {
        app.save_setting("autoCloseIdle", serde_json::json!(v), "空闲关闭");
        if let Some(st) = app.settings.as_mut() {
            st.auto_close_idle = v;
        }
    }
    let idle_min = s.idle_close_minutes;
    let mut v = idle_min;
    if ui
        .add(egui::Slider::new(&mut v, 1..=1440).text("空闲分钟"))
        .changed()
    {
        app.save_setting("idleCloseMinutes", serde_json::json!(v), "空闲分钟");
        if let Some(st) = app.settings.as_mut() {
            st.idle_close_minutes = v;
        }
    }
    let backup_count = s.backup_count;
    let mut v = backup_count;
    if ui
        .add(egui::Slider::new(&mut v, 1..=100).text("保留备份数"))
        .changed()
    {
        app.save_setting("backupCount", serde_json::json!(v), "备份数量");
        if let Some(st) = app.settings.as_mut() {
            st.backup_count = v;
        }
    }
    let time_format = s.time_format.clone();
    ui.horizontal(|ui| {
        ui.label("时间格式");
        if ui
            .selectable_label(time_format == "24h", "24小时")
            .clicked()
            && time_format != "24h"
        {
            app.save_setting("timeFormat", serde_json::json!("24h"), "时间格式");
        }
        if ui
            .selectable_label(time_format == "12h", "12小时")
            .clicked()
            && time_format != "12h"
        {
            app.save_setting("timeFormat", serde_json::json!("12h"), "时间格式");
        }
    });
    let widget = s.widget_enabled;
    let mut v = widget;
    if ui.checkbox(&mut v, "启用桌面小组件").changed() {
        app.save_setting("widgetEnabled", serde_json::json!(v), "桌面组件");
        if let Some(st) = app.settings.as_mut() {
            st.widget_enabled = v;
        }
    }
}

fn settings_network(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    // 同样取所有权快照，避免借用冲突
    let s = match app.settings.clone() {
        Some(s) => s,
        None => {
            if app.loading {
                loading_state(ui, &app.palette(), "正在加载设置", "请稍候…");
            } else {
                empty_state(
                    ui,
                    &app.palette(),
                    regular::WARNING,
                    "未能加载设置",
                    "请确认后端服务正在运行",
                );
            }
            return;
        }
    };
    let mut lan = s.lan_access;
    if ui.checkbox(&mut lan, "局域网访问（重启生效）").changed() {
        app.save_setting("lanAccess", serde_json::json!(lan), "局域网访问");
        if let Some(st) = app.settings.as_mut() {
            st.lan_access = lan;
        }
    }
    let tunnel = s.tunnel_enabled;
    let mut v = tunnel;
    if ui.checkbox(&mut v, "启用 Cloudflare 隧道").changed() {
        app.save_setting("tunnelEnabled", serde_json::json!(v), "隧道");
        if let Some(st) = app.settings.as_mut() {
            st.tunnel_enabled = v;
        }
    }
    let mut tok_changed = false;
    let mut dom_changed = false;
    if let Some(st) = app.settings.as_mut() {
        if ui.text_edit_singleline(&mut st.tunnel_token).lost_focus() {
            tok_changed = true;
        }
        if ui.text_edit_singleline(&mut st.custom_domain).lost_focus() {
            dom_changed = true;
        }
    }
    if tok_changed {
        let t = app
            .settings
            .as_ref()
            .map(|s| s.tunnel_token.clone())
            .unwrap_or_default();
        app.save_setting("tunnelToken", serde_json::json!(t), "隧道Token");
    }
    if dom_changed {
        let d = app
            .settings
            .as_ref()
            .map(|s| s.custom_domain.clone())
            .unwrap_or_default();
        app.save_setting("customDomain", serde_json::json!(d), "自定义域名");
    }
    ui.separator();
    ui.label(ic(
        regular::CLOUD_ARROW_UP,
        &format!(
            "隧道地址：{}",
            app.tunnel_url.clone().unwrap_or_else(|| "未启用".into())
        ),
    ));
    if icon_button(ui, regular::DOWNLOAD_SIMPLE, "检查更新").clicked() {
        app.go(
            |c| c.post_json("/api/check-update", &serde_json::json!({})),
            |r| crate::api::AppMsg::Action(r.map(|_| ()), "检查更新".into()),
        );
    }
}

fn settings_about(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    if let Some(v) = &app.version {
        ui.label(ic(
            regular::SEAL_CHECK,
            &format!("当前版本：v{}", v.version),
        ));
        ui.label(ic(
            regular::ARROW_UP_RIGHT,
            &format!("更新地址：{}", v.update_url),
        ));
    } else if app.loading {
        loading_state(ui, &p, "正在获取版本", "请稍候…");
    } else {
        empty_state(
            ui,
            &p,
            regular::INFO,
            "暂无版本信息",
            "请确认后端服务正在运行",
        );
    }
    if let Some(o) = &app.overview {
        ui.separator();
        ui.label(format!(
            "首次记录：{}  最近记录：{}",
            o.first_date.clone().unwrap_or_default(),
            o.last_date.clone().unwrap_or_default()
        ));
    }
    ui.separator();
    ui.label("纯原生桌面端（egui/eframe），无 WebView / 无 HTML。");
}

// ===================== 管理 =====================

pub fn admin(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    ui.heading("管理");
    ui.separator();

    ui.collapsing("回收站", |ui| {
        if icon_button(ui, regular::ARROWS_CLOCKWISE, "刷新回收站").clicked() {
            app.load_trash();
        }
        if icon_button(ui, regular::TRASH_SIMPLE, "清空回收站").clicked() {
            app.request_clear_trash();
        }
        ui.separator();
        // 先克隆列表，避免循环中的不可变借用与闭包内的可变借用冲突
        let trash = app.trash.clone();
        for (i, s) in trash.iter().enumerate() {
            let t = app.row_t(ui.ctx(), i);
            ui.scope(|ui| {
                ui.set_opacity(t);
                ui.horizontal(|ui| {
                    ui.label(fmt::fmt_datetime(&s.boot_time));
                    ui.label(
                        RichText::new(match s.duration {
                            Some(d) => fmt::fmt_duration(d),
                            None => "-".to_string(),
                        })
                        .font(FontId::new(14.0, egui::FontFamily::Monospace)),
                    );
                    if icon_button(ui, regular::CLOCK_COUNTER_CLOCKWISE, "恢复").clicked() {
                        app.request_restore_trash(s.id.clone());
                    }
                    if ui
                        .button(ic(regular::TRASH, "永久删除").color(p.danger))
                        .clicked()
                    {
                        app.request_delete_trash(s.id.clone());
                    }
                });
            });
        }
        if trash.is_empty() {
            empty_state(
                ui,
                &p,
                regular::TRASH,
                "回收站为空",
                "删除的记录会在这里保留一段时间",
            );
        }
    });

    ui.collapsing("备份", |ui| {
        if icon_button(ui, regular::ARROWS_CLOCKWISE, "刷新备份").clicked() {
            app.load_backups();
        }
        if icon_button(ui, regular::BROOM, "清理旧备份").clicked() {
            app.request_clean_backups();
        }
        ui.separator();
        // 同样先克隆，避免借用冲突
        let backups = app.backups.clone();
        for (i, b) in backups.iter().enumerate() {
            let t = app.row_t(ui.ctx(), i);
            ui.scope(|ui| {
                ui.set_opacity(t);
                ui.horizontal(|ui| {
                    ui.label(ic(regular::FLOPPY_DISK, b));
                    if icon_button(ui, regular::CLOCK_COUNTER_CLOCKWISE, "恢复").clicked() {
                        app.request_restore_backup(b.clone());
                    }
                    if icon_button(ui, regular::TRASH, "删除").clicked() {
                        app.request_delete_backup(b.clone());
                    }
                });
            });
        }
        if backups.is_empty() {
            empty_state(
                ui,
                &p,
                regular::FLOPPY_DISK,
                "暂无备份",
                "开启自动备份后会自动生成",
            );
        }
    });

    ui.collapsing("版本", |ui| {
        ui.horizontal(|ui| {
            if icon_button(ui, regular::ARROW_UP, "升级 补丁").clicked() {
                app.go(
                    |c| c.post_json("/api/version/bump", &serde_json::json!({"type":"patch"})),
                    |r| crate::api::AppMsg::Action(r.map(|_| ()), "升级版本".into()),
                );
            }
            if icon_button(ui, regular::ARROW_UP_RIGHT, "升级 次要").clicked() {
                app.go(
                    |c| c.post_json("/api/version/bump", &serde_json::json!({"type":"minor"})),
                    |r| crate::api::AppMsg::Action(r.map(|_| ()), "升级版本".into()),
                );
            }
            if icon_button(ui, regular::ROCKET, "升级 主要").clicked() {
                app.go(
                    |c| c.post_json("/api/version/bump", &serde_json::json!({"type":"major"})),
                    |r| crate::api::AppMsg::Action(r.map(|_| ()), "升级版本".into()),
                );
            }
        });
    });

    ui.separator();
    if ui
        .button(ic(regular::WARNING_OCTAGON, "清空全部数据").color(p.danger))
        .clicked()
    {
        app.request_clear_data();
    }
}

// 抑制未使用告警（保留类型导入以备扩展）
#[allow(dead_code)]
fn _use_types(_: &AppSettings, _: &TrendDay, _: &Anomalies, _: &Overview) {}
