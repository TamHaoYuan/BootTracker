//! 五个主页面的原生渲染（egui 即时模式）
//!
//! 卡片 / 区块 / 列表行 / KPI 卡 / 空状态等公共部件统一放在 `components`，
//! 图表在 `chart`；本模块只负责「每个页面放什么内容」。

use chrono::{DateTime, Utc};
use egui::{Color32, FontId, RichText, Vec2};

use crate::api::{BootSession, DailyStat, Overview, TrendResponse, WeeklyStat};
use crate::app::{BootTrackerApp, SortKey};
use crate::chart::{self, ChartData};
use crate::components::{
    card, cell, empty_state, ic, icon_button, kpi_card, list_row, loading_state, section, status_pill,
};
use crate::fmt;
use crate::theme;
use egui_phosphor::regular;

/// 趋势行文案：涨/跌 + 语义色（web KpiCard 的 trend 用箭头 + 颜色双编码）。
/// `as_duration` 为真时把数值当毫秒格式化成「x时y分」，否则按整数计数。
fn trend_delta(delta: f64, as_duration: bool) -> Option<(String, Color32)> {
    if delta.abs() < 1e-6 {
        return None;
    }
    let up = delta > 0.0;
    let arrow = if up {
        regular::ARROW_UP
    } else {
        regular::ARROW_DOWN
    };
    let value = if as_duration {
        fmt::fmt_duration(delta.abs() as i64)
    } else {
        format!("{:.0}", delta.abs())
    };
    let color = if up {
        // 上升用成功色，下降用弱化色（时长本身无好坏，只是变化提示）
        Color32::from_rgb(16, 185, 129)
    } else {
        Color32::from_rgb(148, 163, 184)
    };
    Some((format!("{arrow}  较上周 {value}"), color))
}

/// 近 7 天与上一个 7 天的总量差值（有数据才返回趋势）
fn week_over_week(trend: Option<&TrendResponse>, as_duration: bool) -> Option<(String, Color32)> {
    let t = trend?;
    let mut days: Vec<(&String, f64)> = t
        .data
        .iter()
        .map(|(k, v)| {
            let v = if as_duration {
                v.total_duration as f64
            } else {
                v.boot_count as f64
            };
            (k, v)
        })
        .collect();
    if days.len() < 4 {
        return None;
    }
    days.sort_by(|a, b| a.0.cmp(b.0));
    let n = days.len();
    let recent = &days[n.saturating_sub(7)..];
    let prev_end = n.saturating_sub(7);
    let prev_start = prev_end.saturating_sub(7);
    let prev = &days[prev_start..prev_end];
    if prev.is_empty() {
        return None;
    }
    let avg = |xs: &[(&String, f64)]| xs.iter().map(|(_, v)| *v).sum::<f64>() / xs.len() as f64;
    trend_delta(avg(recent) - avg(prev), as_duration)
}

// ===================== 仪表盘 =====================

pub fn dashboard(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    if app.loading && app.data.is_none() {
        loading_state(ui, &p, crate::t!("正在加载数据"), crate::t!("首次启动需要一点时间"));
        return;
    }
    let hour12 = app.hour12();

    // ---- KPI ----
    // 趋势取「近 7 天均值 vs 上一个 7 天均值」，跨周对比才有意义；无 trend 数据时不显示趋势行
    let volume_trend = week_over_week(app.trend.as_ref(), false);
    let duration_trend = week_over_week(app.trend.as_ref(), true);
    if let Some(o) = &app.overview {
        ui.horizontal_wrapped(|ui| {
            kpi_card(
                ui,
                &p,
                crate::t!("累计开机"),
                &o.total_boot.to_string(),
                volume_trend
                    .as_ref()
                    .map(|(t, c)| (t.as_str(), *c)),
                false,
                false,
            );
            kpi_card(
                ui,
                &p,
                crate::t!("累计关机"),
                &o.total_shutdown.to_string(),
                None,
                false,
                false,
            );
            kpi_card(
                ui,
                &p,
                crate::t!("累计时长"),
                &fmt::fmt_duration(o.total_duration),
                None,
                false,
                false,
            );
            kpi_card(
                ui,
                &p,
                crate::t!("平均时长"),
                &fmt::fmt_duration(o.avg_duration),
                duration_trend
                    .as_ref()
                    .map(|(t, c)| (t.as_str(), *c)),
                false,
                false,
            );
            // 有进行中会话时用 accent 色强调（对应 web KpiCard 的 accent prop）
            kpi_card(
                ui,
                &p,
                crate::t!("进行中"),
                &o.active_count.to_string(),
                None,
                false,
                o.active_count > 0,
            );
        });
    }

    // ---- 本次会话（hero）----
    ui.add_space(14.0);
    let now = Utc::now();
    let active: Option<BootSession> = app.data.as_ref().and_then(|d| {
        d.sessions
            .iter()
            .filter(|s| s.shutdown_time.is_none())
            .max_by_key(|s| s.boot_time.clone())
            .cloned()
    });
    match active {
        Some(s) => {
            if let Some(bt) = fmt::parse_utc(&s.boot_time) {
                let dur = (now - bt).num_milliseconds();
                let since = format!("自 {}", fmt::fmt_datetime(&s.boot_time, hour12));
                kpi_card(
                    ui,
                    &p,
                    crate::t!("本次会话"),
                    &fmt::fmt_duration(dur),
                    Some((&since, p.text_secondary)),
                    true,
                    true,
                );
            }
        }
        None => {
            card(ui, &p, |ui| {
                empty_state(
                    ui,
                    &p,
                    regular::CLOCK,
                    crate::t!("当前无进行中的会话"),
                    crate::t!("下次开机后会自动开始计时"),
                );
            });
        }
    }

    // ---- 近 14 天开机时长 ----
    ui.add_space(14.0);
    let items = trend_items(app.trend.as_ref(), 14);
    section(
        ui,
        &p,
        regular::CHART_BAR,
        crate::t!("近 14 天开机时长"),
        |_| {},
        |ui| {
            chart::bar_chart(
                ui,
                &ChartData {
                    items,
                    as_duration: true,
                },
                p.accent,
                &p,
            )
        },
    );

    // ---- 最近记录（最多 5 条，跳转记录页看全部）----
    ui.add_space(14.0);
    let recent: Vec<BootSession> = app
        .overview
        .as_ref()
        .map(|o| o.recent_sessions.iter().take(5).cloned().collect())
        .unwrap_or_default();
    let total = app
        .overview
        .as_ref()
        .map(|o| o.recent_sessions.len())
        .unwrap_or(0);
    let mut goto_records = false;
    section(
        ui,
        &p,
        regular::CLOCK,
        crate::t!("最近记录"),
        |ui| {
            if ui
                .button(ic(regular::ARROW_RIGHT, crate::t!("查看全部")))
                .on_hover_text(crate::t!("打开记录页"))
                .clicked()
            {
                goto_records = true;
            }
        },
        |ui| {
            if recent.is_empty() {
                empty_state(ui, &p, regular::NOTE, crate::t!("暂无记录"), crate::t!("开机后会自动记录一条会话"));
                return;
            }
            for (i, s) in recent.iter().enumerate() {
                let t = app.row_t(ui.ctx(), i);
                ui.scope(|ui| {
                    ui.set_opacity(t);
                    list_row(ui, &p, i % 2 == 1, |ui| {
                        ui.label(RichText::new(regular::POWER).color(p.text_muted));
                        ui.add_space(4.0);
                        ui.label(fmt::fmt_datetime(&s.boot_time, hour12));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(match s.duration {
                                    Some(d) => fmt::fmt_duration(d),
                                    None => crate::t!("进行中").into(),
                                })
                                .font(FontId::new(14.0, egui::FontFamily::Monospace))
                                .color(p.text_secondary),
                            );
                            ui.label(match &s.shutdown_time {
                                Some(x) => fmt::fmt_time(x, hour12),
                                None => "—".into(),
                            });
                        });
                    });
                });
            }
            if total > recent.len() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("仅显示最近 {} 条，共 {total} 条", recent.len()))
                        .size(12.0)
                        .color(p.text_muted),
                );
            }
        },
    );
    if goto_records {
        app.page = crate::app::Page::Records;
    }
}

/// 趋势 → 图表数据（按日期升序取最后 n 天，标签取 MM-DD）
fn trend_items(trend: Option<&TrendResponse>, n: usize) -> Vec<(String, f64)> {
    trend
        .map(|t| {
            let mut days: Vec<&String> = t.data.keys().collect();
            days.sort();
            days.into_iter()
                .rev()
                .take(n)
                .rev()
                .map(|d| {
                    let v = t.data.get(d).map(|x| x.total_duration as f64).unwrap_or(0.0);
                    (short_day(d), v)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// "2026-10-01" → "10-01"（越界时原样返回，绝不 panic）
fn short_day(d: &str) -> String {
    if d.len() >= 10 {
        d[5..].to_string()
    } else {
        d.to_string()
    }
}

// ===================== 记录 =====================

pub fn records(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    let hour12 = app.hour12();

    // ---- 工具条 ----
    card(ui, &p, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(ic(regular::MAGNIFYING_GLASS, crate::t!("搜索")));
            ui.add(
                egui::TextEdit::singleline(&mut app.filter_text)
                    .id(egui::Id::new("records_search"))
                    .hint_text(crate::t!("ID 关键字"))
                    .desired_width(140.0),
            );
            ui.add_space(6.0);
            ui.label(ic(regular::CALENDAR, crate::t!("起")));
            ui.add(
                egui::TextEdit::singleline(&mut app.filter_from)
                    .hint_text("YYYY-MM-DD")
                    .desired_width(110.0),
            );
            ui.label("止");
            ui.add(
                egui::TextEdit::singleline(&mut app.filter_to)
                    .hint_text("YYYY-MM-DD")
                    .desired_width(110.0),
            );
            if (!app.filter_text.is_empty()
                || !app.filter_from.is_empty()
                || !app.filter_to.is_empty())
                && icon_button(ui, regular::X_CIRCLE, crate::t!("清除")).clicked()
            {
                app.filter_text.clear();
                app.filter_from.clear();
                app.filter_to.clear();
            }
            ui.separator();
            ui.selectable_value(
                &mut app.records_view,
                crate::app::RecordsView::Table,
                ic(regular::TABLE, crate::t!("表格")),
            );
            ui.selectable_value(
                &mut app.records_view,
                crate::app::RecordsView::Timeline,
                ic(regular::CLOCK_COUNTER_CLOCKWISE, crate::t!("时间轴")),
            );
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            if icon_button(ui, regular::ARROWS_CLOCKWISE, crate::t!("刷新")).clicked() {
                app.refresh_data();
            }
            if icon_button(ui, regular::UPLOAD_SIMPLE, crate::t!("导入")).clicked() {
                app.import_json();
            }
            let owned: Vec<BootSession> = app.filtered_sessions();
            if icon_button(ui, regular::FILE_CSV, "CSV").on_hover_text(crate::t!("导出为 CSV")).clicked() {
                app.export_csv(&owned);
            }
            if icon_button(ui, regular::FILE_XLS, "XLSX")
                .on_hover_text(crate::t!("导出为 Excel"))
                .clicked()
            {
                app.export_xlsx(&owned);
            }
            if icon_button(ui, regular::ARROWS_MERGE, crate::t!("合并"))
                .on_hover_text(crate::t!("合并选中的记录（至少 2 条）"))
                .clicked()
            {
                app.request_merge();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        crate::tf!("共 {} 条", owned.len()),
                        if app.selected.is_empty() {
                            String::new()
                        } else {
                            crate::tf!("· 已选 {} 条", app.selected.len())
                        }
                    ))
                    .size(12.0)
                    .color(p.text_muted),
                );
            });
        });
    });

    ui.add_space(10.0);
    let sessions = app.filtered_sessions();
    if sessions.is_empty() {
        card(ui, &p, |ui| {
            empty_state(
                ui,
                &p,
                regular::NOTE,
                crate::t!("没有匹配的记录"),
                crate::t!("调整搜索关键字或日期范围后再试"),
            );
        });
        return;
    }

    if app.records_view == crate::app::RecordsView::Table {
        records_table(ui, app, &sessions, hour12);
    } else {
        timeline(ui, app, &sessions, hour12);
    }
}

/// 表头：可点击排序（再点一次切换升降序）
fn head_cell(ui: &mut egui::Ui, p: &theme::Palette, app: &mut BootTrackerApp, key: SortKey, text: &str) {
    let active = app.sort_key == key;
    let arrow = if active {
        if app.sort_asc {
            regular::ARROW_UP
        } else {
            regular::ARROW_DOWN
        }
    } else {
        ""
    };
    let label = RichText::new(format!("{text} {arrow}"))
        .size(12.0)
        .strong()
        .color(if active { p.accent } else { p.text_muted });
    if ui
        .add(egui::Label::new(label).sense(egui::Sense::click()))
        .on_hover_text(crate::t!("点击切换排序"))
        .clicked()
    {
        if app.sort_key == key {
            app.sort_asc = !app.sort_asc;
        } else {
            app.sort_key = key;
            app.sort_asc = false;
        }
    }
}

/// 记录表格
///
/// 不用 `egui::Grid`：Grid 拿不到行矩形，做不了**容器 16px 圆角、表头、整行 hover
/// 底色**这三件事。这里按窗口宽度按比例分配列宽手工布局。
fn records_table(ui: &mut egui::Ui, app: &mut BootTrackerApp, sessions: &[BootSession], hour12: bool) {
    const ROW_H: f32 = 36.0;
    const HEAD_H: f32 = 34.0;
    const PAD: f32 = 12.0;
    /// 勾选框列：必须 ≥ theme 的 `spacing.interact_size.x`(48)，否则复选框会挤进 ID 列
    const SEL_W: f32 = 52.0;
    /// 操作列：单个图标按钮 + 内边距
    const ACT_W: f32 = 56.0;
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
    let content_w = total_w - PAD * 2.0;
    let total_h = PAD * 2.0 + HEAD_H + sessions.len() as f32 * ROW_H;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, total_h), egui::Sense::hover());
    let painter = ui.painter().clone();
    painter.rect_filled(rect, egui::Rounding::same(theme::R_PANEL), p.bg_card);
    painter.rect_stroke(
        rect,
        egui::Rounding::same(theme::R_PANEL),
        egui::Stroke::new(1.0_f32, p.border_light),
    );
    crate::components::inner_highlight(&painter, rect, &p);

    let mut y = rect.top() + PAD;
    // ---- 表头 ----
    let head_rect = egui::Rect::from_min_size(
        egui::Pos2::new(rect.left() + PAD, y),
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
                let heads = ["", "ID", crate::t!("开机时间"), crate::t!("关机时间"), crate::t!("时长"), crate::t!("操作")];
                for (i, h) in heads.iter().enumerate() {
                    cell(ui, widths[i], HEAD_H, |ui| {
                        match i {
                            2 => head_cell(ui, &p, app, SortKey::Boot, h),
                            3 => head_cell(ui, &p, app, SortKey::Shutdown, h),
                            4 => head_cell(ui, &p, app, SortKey::Duration, h),
                            _ => {
                                ui.label(RichText::new(*h).size(12.0).strong().color(p.text_muted));
                            }
                        }
                    });
                }
            });
        },
    );
    y += HEAD_H;
    painter.line_segment(
        [
            egui::Pos2::new(rect.left() + PAD, y),
            egui::Pos2::new(rect.right() - PAD, y),
        ],
        egui::Stroke::new(1.0_f32, p.border_light),
    );

    // ---- 数据行 ----
    for (i, s) in sessions.iter().enumerate() {
        let row_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.left() + 1.0, y),
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
            painter.rect_filled(
                row_rect,
                egui::Rounding::same(8.0),
                theme::accent_tint(p.accent, 24),
            );
        } else if i % 2 == 1 {
            painter.rect_filled(
                row_rect,
                egui::Rounding::same(8.0),
                theme::overlay(p.bg_card, p.text_primary, 0.03),
            );
        }

        // 逐行错峰入场。内容矩形与表头同源（left+PAD / 宽 content_w），
        // hover 底色才用铺满容器的 row_rect —— 两者不能混用，否则列会错位。
        let t = app.row_t(ui.ctx(), i);
        let content_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.left() + PAD, y),
            Vec2::new(content_w, ROW_H),
        );
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
                        ui.label(RichText::new(&s.id).color(p.text_secondary));
                    });
                    cell(ui, widths[2], ROW_H, |ui| {
                        ui.label(fmt::fmt_datetime(&s.boot_time, hour12));
                    });
                    cell(ui, widths[3], ROW_H, |ui| {
                        ui.label(match &s.shutdown_time {
                            Some(t) => fmt::fmt_datetime(t, hour12),
                            None => crate::t!("进行中").to_string(),
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
                        if ui
                            .button(RichText::new(regular::TRASH).color(p.danger))
                            .on_hover_text(crate::t!("移入回收站"))
                            .clicked()
                        {
                            app.request_delete_session(s.id.clone());
                        }
                    });
                });
            },
        );
        y += ROW_H;
    }
}

/// 时间轴视图：按日期分组
fn timeline(ui: &mut egui::Ui, app: &mut BootTrackerApp, sessions: &[BootSession], hour12: bool) {
    let p = app.palette();
    card(ui, &p, |ui| {
        egui::ScrollArea::vertical()
            .max_height(520.0)
            .show(ui, |ui| {
                let mut last_date = String::new();
                for (i, s) in sessions.iter().enumerate() {
                    let date = fmt::fmt_date(&s.boot_time);
                    if date != last_date {
                        last_date = date.clone();
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!("{}  {}", regular::CALENDAR_BLANK, date))
                                .size(13.0)
                                .strong()
                                .color(p.accent),
                        );
                        ui.add_space(2.0);
                    }
                    // 逐行错峰入场
                    let t = app.row_t(ui.ctx(), i);
                    ui.scope(|ui| {
                        ui.set_opacity(t);
                        list_row(ui, &p, i % 2 == 1, |ui| {
                            ui.label(RichText::new(regular::POWER).color(p.text_muted));
                            ui.add_space(4.0);
                            ui.label(fmt::fmt_datetime(&s.boot_time, hour12));
                            ui.label(RichText::new("→").color(p.text_muted));
                            ui.label(match &s.shutdown_time {
                                Some(x) => fmt::fmt_datetime(x, hour12),
                                None => crate::t!("进行中").into(),
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(RichText::new(regular::TRASH).color(p.danger))
                                    .on_hover_text(crate::t!("移入回收站"))
                                    .clicked()
                                {
                                    app.request_delete_session(s.id.clone());
                                }
                                ui.label(
                                    RichText::new(match s.duration {
                                        Some(d) => fmt::fmt_duration(d),
                                        None => "-".into(),
                                    })
                                    .font(FontId::new(14.0, egui::FontFamily::Monospace))
                                    .color(p.text_secondary),
                                );
                            });
                        });
                    });
                }
            });
    });
}

// ===================== 图表 =====================

pub fn charts(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    ui.horizontal(|ui| {
        ui.label(ic(regular::CHART_LINE_UP, crate::t!("图表分析")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.selectable_value(&mut app.chart_mode, 1, ic(regular::CHART_LINE_UP, crate::t!("折线图")));
            ui.selectable_value(&mut app.chart_mode, 0, ic(regular::CHART_BAR, crate::t!("柱状图")));
        });
    });
    ui.add_space(8.0);

    let color = p.accent;
    let mode = app.chart_mode;
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
                    (short_day(k), v)
                })
                .collect()
        })
        .unwrap_or_default();
    section(
        ui,
        &p,
        regular::CALENDAR_CHECK,
        crate::t!("每日开机时长"),
        |_| {},
        |ui| {
            let data = ChartData {
                items: daily_items,
                as_duration: true,
            };
            if mode == 0 {
                chart::bar_chart(ui, &data, color, &p);
            } else {
                chart::line_chart(ui, &data, color, &p);
            }
        },
    );

    ui.add_space(12.0);
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
                    (k[5..].to_string(), v)
                })
                .collect()
        })
        .unwrap_or_default();
    section(
        ui,
        &p,
        regular::CALENDAR,
        crate::t!("每周开机次数"),
        |_| {},
        |ui| {
            chart::bar_chart(
                ui,
                &ChartData {
                    items: weekly_items,
                    as_duration: false,
                },
                p.success,
                &p,
            )
        },
    );

    ui.add_space(12.0);
    let (avg, hi, lo) = app
        .anomalies
        .as_ref()
        .map(|a| (a.avg_duration, a.threshold_high, a.threshold_low))
        .unwrap_or((0, 0, 0));
    section(
        ui,
        &p,
        regular::SEAL_WARNING,
        crate::t!("异常检测"),
        |_| {},
        |ui| match &app.anomalies {
            Some(a) => {
                ui.label(
                    RichText::new(crate::tf!(
                        "平均时长 {}    阈值 高 {} / 低 {}",
                        fmt::fmt_duration(avg),
                        fmt::fmt_duration(hi),
                        fmt::fmt_duration(lo)
                    ))
                    .size(13.0)
                    .color(p.text_secondary),
                );
                ui.add_space(6.0);
                anomaly_group(ui, &p, crate::t!("异常偏长"), regular::TREND_UP, &a.longest, hour12_of(app));
                ui.add_space(6.0);
                anomaly_group(ui, &p, crate::t!("异常偏短"), regular::TREND_DOWN, &a.shortest, hour12_of(app));
            }
            None if app.loading => loading_state(ui, &p, crate::t!("正在分析异常"), crate::t!("需要足够的记录才能计算阈值")),
            None => empty_state(
                ui,
                &p,
                regular::SEAL_CHECK,
                crate::t!("暂无异常"),
                crate::t!("记录足够后会自动计算阈值"),
            ),
        },
    );
}

fn hour12_of(app: &BootTrackerApp) -> bool {
    app.hour12()
}

/// 异常分组：标题带条数，0 条时直接给空态（不再是展开展开一片空白）
fn anomaly_group(
    ui: &mut egui::Ui,
    p: &theme::Palette,
    title: &str,
    icon: &str,
    list: &[BootSession],
    hour12: bool,
) {
    let head = if list.is_empty() {
        format!("{title}（无）")
    } else {
        format!("{title}（{} 条）", list.len())
    };
    egui::CollapsingHeader::new(RichText::new(format!("{icon}  {head}")).size(14.0))
        .default_open(!list.is_empty())
        .show(ui, |ui| {
            if list.is_empty() {
                ui.label(
                    RichText::new(crate::t!("没有超过阈值的记录"))
                        .size(13.0)
                        .color(p.text_muted),
                );
                return;
            }
            for (i, s) in list.iter().enumerate() {
                list_row(ui, p, i % 2 == 1, |ui| {
                    ui.label(fmt::fmt_datetime(&s.boot_time, hour12));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(fmt::fmt_duration(s.duration.unwrap_or(0)))
                                .font(FontId::new(14.0, egui::FontFamily::Monospace))
                                .color(p.text_secondary),
                        );
                    });
                });
            }
        });
}

// ===================== 设置 =====================

pub fn settings(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    ui.horizontal(|ui| {
        ui.label(ic(regular::GEAR, crate::t!("设置")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.selectable_value(&mut app.settings_tab, 3, ic(regular::INFO, crate::t!("关于")));
            ui.selectable_value(&mut app.settings_tab, 2, ic(regular::PLUG, crate::t!("网络")));
            ui.selectable_value(&mut app.settings_tab, 1, ic(regular::GEAR_SIX, crate::t!("通用")));
            ui.selectable_value(&mut app.settings_tab, 0, ic(regular::PALETTE, crate::t!("外观")));
        });
    });
    ui.add_space(10.0);
    let _ = p;

    match app.settings_tab {
        0 => settings_appearance(ui, app),
        1 => settings_general(ui, app),
        2 => settings_network(ui, app),
        _ => settings_about(ui, app),
    }
}

fn settings_appearance(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    let mode = app
        .settings
        .as_ref()
        .map(|s| s.app_mode.clone())
        .unwrap_or_else(|| "dark".into());
    let cur_theme = app
        .settings
        .as_ref()
        .map(|s| s.app_theme.clone())
        .unwrap_or_default();
    let chart_type = app
        .settings
        .as_ref()
        .map(|s| s.default_chart_type.clone())
        .unwrap_or_else(|| "bar".into());

    section(
        ui,
        &p,
        regular::MOON,
        crate::t!("外观模式"),
        |_| {},
        |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(mode == "dark", ic(regular::MOON, crate::t!("暗色")))
                    .clicked()
                    && mode != "dark"
                {
                    app.set_theme("dark");
                    if let Some(s) = app.settings.as_mut() {
                        s.app_mode = "dark".into();
                    }
                }
                if ui
                    .selectable_label(mode == "light", ic(regular::SUN, crate::t!("亮色")))
                    .clicked()
                    && mode != "light"
                {
                    app.set_theme("light");
                    if let Some(s) = app.settings.as_mut() {
                        s.app_mode = "light".into();
                    }
                }
            });
        },
    );

    ui.add_space(12.0);
    // 主题色：8 个色块（对齐 web 的 data-theme 列表），选中态加 accent 描边
    let mut picked: Option<&'static str> = None;
    section(
        ui,
        &p,
        regular::PALETTE,
        crate::t!("主题色"),
        |_| {},
        |ui| {
            ui.horizontal_wrapped(|ui| {
                for (key, name) in theme::themes() {
                    let selected = cur_theme == key || (key.is_empty() && cur_theme.is_empty());
                    let swatch = theme::theme_swatch(key);
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(72.0, 54.0), egui::Sense::click());
                    let painter = ui.painter();
                    let body = egui::Rect::from_min_max(
                        egui::Pos2::new(rect.left(), rect.top() + 4.0),
                        egui::Pos2::new(rect.right(), rect.bottom()),
                    );
                    painter.rect_filled(
                        body,
                        egui::Rounding::same(theme::R_CARD),
                        if selected {
                            theme::accent_tint(swatch, 40)
                        } else {
                            p.bg_page
                        },
                    );
                    painter.rect_stroke(
                        body,
                        egui::Rounding::same(theme::R_CARD),
                        egui::Stroke::new(
                            if selected { 2.0_f32 } else { 1.0_f32 },
                            if selected { swatch } else { p.border_light },
                        ),
                    );
                    painter.rect_filled(
                        egui::Rect::from_center_size(
                            egui::Pos2::new(body.center().x, body.top() + 16.0),
                            Vec2::splat(14.0),
                        ),
                        egui::Rounding::same(4.0),
                        swatch,
                    );
                    painter.text(
                        egui::Pos2::new(body.center().x, body.bottom() - 6.0),
                        egui::Align2::CENTER_BOTTOM,
                        name,
                        FontId::new(12.0, egui::FontFamily::Proportional),
                        if selected { swatch } else { p.text_secondary },
                    );
                    if resp.clicked() && !selected {
                        picked = Some(key);
                    }
                }
            });
        },
    );
    if let Some(key) = picked {
        app.save_setting("appTheme", serde_json::json!(key), crate::t!("主题色"));
        if let Some(s) = app.settings.as_mut() {
            s.app_theme = key.to_string();
        }
        // 立即重绘外壳（不等 15s 轮询）
        app.applied_theme = None;
    }

    ui.add_space(12.0);
    section(
        ui,
        &p,
        regular::MAGNIFYING_GLASS,
        crate::t!("界面缩放"),
        |_| {},
        |ui| {
            let mut scale = app.settings.as_ref().map(|s| s.ui_scale).unwrap_or(1.0);
            let resp = ui.add(
                egui::Slider::new(&mut scale, 0.5..=2.0)
                    .step_by(0.05)
                    .fixed_decimals(2)
                    .suffix("×")
                    .text(crate::t!("整体缩放")),
            );
            if resp.changed() {
                // 本地立即生效（不等 PUT 返回），后端持久化到 settings.uiScale
                app.set_ui_scale(scale);
            }
            if resp.drag_stopped() {
                app.set_ui_scale(scale);
            }
            ui.horizontal(|ui| {
                if ui.button("100%").clicked() {
                    app.set_ui_scale(1.0);
                }
                if ui.button("125%").clicked() {
                    app.set_ui_scale(1.25);
                }
                if ui.button("150%").clicked() {
                    app.set_ui_scale(1.5);
                }
                if ui.button(crate::t!("重置")).clicked() {
                    app.set_ui_scale(1.0);
                }
            });
            ui.label(
                RichText::new(crate::t!("快捷键：Ctrl + 滚轮 / Ctrl 加号减号 缩放，Ctrl + 0 复位。"))
                    .size(12.0)
                    .color(p.text_muted),
            );
        },
    );

    ui.add_space(12.0);
    section(
        ui,
        &p,
        regular::CHART_LINE_UP,
        crate::t!("默认图表类型"),
        |_| {},
        |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(chart_type == "bar", ic(regular::CHART_BAR, crate::t!("柱状")))
                    .clicked()
                    && chart_type != "bar"
                {
                    app.save_setting("defaultChartType", serde_json::json!("bar"), crate::t!("图表类型"));
                    if let Some(s) = app.settings.as_mut() {
                        s.default_chart_type = "bar".into();
                    }
                    app.chart_mode = 0;
                }
                if ui
                    .selectable_label(chart_type == "line", ic(regular::CHART_LINE_UP, crate::t!("折线")))
                    .clicked()
                    && chart_type != "line"
                {
                    app.save_setting("defaultChartType", serde_json::json!("line"), crate::t!("图表类型"));
                    if let Some(s) = app.settings.as_mut() {
                        s.default_chart_type = "line".into();
                    }
                    app.chart_mode = 1;
                }
            });
        },
    );
}

fn settings_general(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    // 取一份拥有所有权的快照，避免 &app.settings 的不可变借用跨越后续 as_mut()
    let s = match app.settings.clone() {
        Some(s) => s,
        None => {
            if app.loading {
                loading_state(ui, &p, crate::t!("正在加载设置"), crate::t!("请稍候…"));
            } else {
                empty_state(ui, &p, regular::WARNING, crate::t!("未能加载设置"), crate::t!("请确认后端服务正在运行"));
            }
            return;
        }
    };

    section(
        ui,
        &p,
        regular::POWER,
        crate::t!("启动与备份"),
        |_| {},
        |ui| {
            let mut v = s.auto_start;
            if ui.checkbox(&mut v, crate::t!("开机自启")).changed() {
                app.save_setting("autoStart", serde_json::json!(v), crate::t!("开机自启"));
                if let Some(st) = app.settings.as_mut() {
                    st.auto_start = v;
                }
            }
            let mut v = s.auto_backup;
            if ui.checkbox(&mut v, crate::t!("自动备份")).changed() {
                app.save_setting("autoBackup", serde_json::json!(v), crate::t!("自动备份"));
                if let Some(st) = app.settings.as_mut() {
                    st.auto_backup = v;
                }
            }
            let mut v = s.backup_count;
            if ui
                .add(egui::Slider::new(&mut v, 1..=100).text(crate::t!("保留备份数")))
                .changed()
            {
                app.save_setting("backupCount", serde_json::json!(v), crate::t!("备份数量"));
                if let Some(st) = app.settings.as_mut() {
                    st.backup_count = v;
                }
            }
            let mut v = s.auto_close_idle;
            if ui.checkbox(&mut v, crate::t!("空闲自动关闭会话")).changed() {
                app.save_setting("autoCloseIdle", serde_json::json!(v), crate::t!("空闲关闭"));
                if let Some(st) = app.settings.as_mut() {
                    st.auto_close_idle = v;
                }
            }
            let mut v = s.idle_close_minutes;
            if ui
                .add(egui::Slider::new(&mut v, 1..=1440).text(crate::t!("空闲分钟")))
                .changed()
            {
                app.save_setting("idleCloseMinutes", serde_json::json!(v), crate::t!("空闲分钟"));
                if let Some(st) = app.settings.as_mut() {
                    st.idle_close_minutes = v;
                }
            }
        },
    );

    ui.add_space(12.0);
    section(
        ui,
        &p,
        regular::CLOCK,
        crate::t!("显示"),
        |_| {},
        |ui| {
            let tf = s.time_format.clone();
            ui.horizontal(|ui| {
                ui.label("时间格式");
                if ui.selectable_label(tf == "24h", crate::t!("24 小时")).clicked() && tf != "24h" {
                    app.save_setting("timeFormat", serde_json::json!("24h"), crate::t!("时间格式"));
                    if let Some(st) = app.settings.as_mut() {
                        st.time_format = "24h".into();
                    }
                }
                if ui.selectable_label(tf == "12h", crate::t!("12 小时")).clicked() && tf != "12h" {
                    app.save_setting("timeFormat", serde_json::json!("12h"), crate::t!("时间格式"));
                    if let Some(st) = app.settings.as_mut() {
                        st.time_format = "12h".into();
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(crate::t!("示例：2026-10-06 21:43:56 / 2026-10-06 09:43:56 下午"))
                    .size(12.0)
                    .color(p.text_muted),
            );
        },
    );

    ui.add_space(12.0);
    section(
        ui,
        &p,
        regular::TRANSLATE,
        crate::t!("界面语言"),
        |_| {},
        |ui| {
            let cur = if s.language.is_empty() {
                crate::i18n::lang().code().to_string()
            } else {
                s.language.clone()
            };
            ui.horizontal(|ui| {
                for lang in [crate::i18n::Lang::ZhCn, crate::i18n::Lang::EnUs] {
                    if ui
                        .selectable_label(cur == lang.code(), lang.native_name())
                        .clicked()
                        && cur != lang.code()
                    {
                        app.save_setting("language", serde_json::json!(lang.code()), crate::t!("界面语言"));
                        if let Some(st) = app.settings.as_mut() {
                            st.language = lang.code().to_string();
                        }
                        // 立即生效：不必等下一次 15s 轮询
                        crate::i18n::set_lang(lang);
                        app.applied_lang = Some(lang);
                    }
                }
            });
            ui.label(
                RichText::new(crate::t!("切换后界面文案立即更新；托盘菜单需要重启应用才能跟随。"))
                    .size(12.0)
                    .color(p.text_muted),
            );
        },
    );

    ui.add_space(12.0);
    section(
        ui,
        &p,
        regular::SQUARES_FOUR,
        crate::t!("桌面小组件"),
        |_| {},
        |ui| {
            let mut v = s.widget_enabled;
            if ui.checkbox(&mut v, crate::t!("启用桌面小组件")).changed() {
                app.save_setting("widgetEnabled", serde_json::json!(v), crate::t!("桌面组件"));
                if let Some(st) = app.settings.as_mut() {
                    st.widget_enabled = v;
                }
            }
            ui.label(
                RichText::new(crate::t!("浮窗每 30 秒同步一次记录，可在顶栏一键开关。"))
                    .size(12.0)
                    .color(p.text_muted),
            );
        },
    );
}

fn settings_network(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    let s = match app.settings.clone() {
        Some(s) => s,
        None => {
            if app.loading {
                loading_state(ui, &p, crate::t!("正在加载设置"), crate::t!("请稍候…"));
            } else {
                empty_state(ui, &p, regular::WARNING, crate::t!("未能加载设置"), crate::t!("请确认后端服务正在运行"));
            }
            return;
        }
    };

    section(
        ui,
        &p,
        regular::PLUG,
        crate::t!("本地访问"),
        |_| {},
        |ui| {
            let mut lan = s.lan_access;
            if ui.checkbox(&mut lan, crate::t!("局域网访问（重启后端生效）")).changed() {
                app.save_setting("lanAccess", serde_json::json!(lan), crate::t!("局域网访问"));
                if let Some(st) = app.settings.as_mut() {
                    st.lan_access = lan;
                }
            }
            ui.label(
                RichText::new(format!(
                    "本机地址 http://127.0.0.1:18792{}",
                    if s.lan_access {
                        crate::t!("    局域网内其他设备可用「本机 IP:18792」访问")
                    } else {
                        crate::t!("（仅本机可访问）")
                    }
                ))
                .size(12.0)
                .color(p.text_muted),
            );
        },
    );

    ui.add_space(12.0);
    let _ = &s;
    settings_tunnel(ui, app, &p);
}
/// 隧道错误 key → 可读文案。
///
/// 后端只回稳定 key（`cloudflared_missing`、`download_failed`…），本地化在这里做，
/// 这样界面能准确说出「没装 cloudflared」而不是笼统的「启动失败」。`_detail` 是
/// 后端原样透传的诊断信息（退出码 / cloudflared 日志尾巴），拼在末尾便于排查。
fn tunnel_error_text(key: &str, detail: &str) -> String {
    let base: String = match key {
        "" => String::new(),
        "cloudflared_missing" => crate::t!("未找到 cloudflared，请点击「下载 cloudflared」（自动下载到程序目录）").into(),
        "download_failed" => crate::t!("cloudflared 下载失败，请检查网络后重试").into(),
        "download_in_progress" => crate::t!("cloudflared 正在下载中").into(),
        "spawn_failed" => crate::t!("无法启动 cloudflared 进程（可能被杀毒软件拦截）").into(),
        "exited" => crate::t!("cloudflared 启动后立即退出，请查看下方诊断信息").into(),
        "url_timeout" => crate::t!("等待隧道地址超时（45 秒），请检查网络或 Cloudflare 状态").into(),
        "not_running" => crate::t!("隧道未在运行").into(),
        "not_found" => crate::t!("未找到已下载的 cloudflared").into(),
        "remove_failed" => crate::t!("删除 cloudflared 失败").into(),
        other => format!("cloudflared: {other}"),
    };
    if detail.is_empty() {
        base
    } else {
        format!("{base}\n{detail}")
    }
}

/// 「一键临时隧道」区块。
///
/// 与旧版的区别：旧版只有一个「启用 Cloudflare 隧道」复选框 + 一行 URL，没装
/// cloudflared 时就一直显示「未启用」，用户完全不知道原因。现在把四个状态
/// （未安装 / 下载中 / 运行中 / 已停止）和错误原因都直接摆在界面上。
fn settings_tunnel(ui: &mut egui::Ui, app: &mut BootTrackerApp, p: &crate::theme::Palette) {
    let status = app.tunnel_status.clone();
    let installed = status.as_ref().map(|x| x.installed).unwrap_or(false);
    let running = status.as_ref().map(|x| x.running).unwrap_or(false);
    let url = status.as_ref().map(|x| x.url.clone()).unwrap_or_default();
    let downloading = status
        .as_ref()
        .map(|x| x.download.status == "downloading")
        .unwrap_or(false);
    let dl_bytes = status.as_ref().map(|x| x.download.bytes).unwrap_or(0);
    let version = status
        .as_ref()
        .map(|x| x.binary_version.clone())
        .unwrap_or_default();
    let err_key = status.as_ref().map(|x| x.error.clone()).unwrap_or_default();
    let err_detail = status
        .as_ref()
        .map(|x| x.error_detail.clone())
        .unwrap_or_default();

    section(
        ui,
        p,
        regular::CLOUD_ARROW_UP,
        crate::t!("一键临时隧道"),
        |_| {},
        |ui| {
            // ---- 状态行：胶囊 + 版本 ----
            ui.horizontal(|ui| {
                if !installed {
                    if downloading {
                        status_pill(
                            ui,
                            p,
                            regular::DOWNLOAD_SIMPLE,
                            crate::t!("正在下载 cloudflared…"),
                            p.accent,
                        );
                    } else {
                        status_pill(ui, p, regular::WARNING_OCTAGON, crate::t!("未安装 cloudflared"), p.danger);
                    }
                } else if running {
                    let label = if url.is_empty() {
                        crate::t!("正在建立隧道…").to_string()
                    } else {
                        crate::t!("隧道已开启").to_string()
                    };
                    status_pill(ui, p, regular::LINK, &label, p.success);
                } else {
                    status_pill(ui, p, regular::CIRCLE, crate::t!("隧道已停止"), p.text_muted);
                }
                if !version.is_empty() {
                    ui.label(RichText::new(&version).size(12.0).color(p.text_muted));
                }
            });

            // ---- 未安装：给出下载入口 ----
            if !installed {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(crate::t!(
                        "临时隧道需要 Cloudflare 官方的 cloudflared 程序。点击下方按钮会自动下载到程序目录（约 60 MB），无需手动安装。"
                    ))
                    .size(12.0)
                    .color(p.text_muted),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let btn = if downloading {
                        // 注意：`tf!` 的 `{name:spec}` 只实现了 `{:0N}`（补零），
                        // `{:.1}` 会退化成默认 Display，所以这里先自己格式化成
                        // 字符串再插值，别指望模板里能写精度。
                        let mb = format!("{:.1}", dl_bytes as f64 / 1_048_576.0);
                        ui.add_enabled(
                            false,
                            egui::Button::new(crate::tf!(
                                "{}  正在下载 {} MB…",
                                regular::DOWNLOAD_SIMPLE,
                                mb
                            )),
                        )
                    } else {
                        icon_button(ui, regular::DOWNLOAD_SIMPLE, crate::t!("下载 cloudflared"))
                    };
                    if btn.clicked() {
                        app.download_cloudflared();
                    }
                });
            } else {
                // ---- 已安装：连接 / 断开 ----
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if running {
                        if icon_button(ui, regular::PLUGS, crate::t!("断开隧道")).clicked() {
                            app.stop_tunnel();
                            if let Some(st) = app.settings.as_mut() {
                                st.tunnel_enabled = false;
                            }
                        }
                    } else if icon_button(ui, regular::PLUGS_CONNECTED, crate::t!("开启隧道")).clicked() {
                        app.start_tunnel();
                        if let Some(st) = app.settings.as_mut() {
                            st.tunnel_enabled = true;
                        }
                    }
                    if icon_button(ui, regular::ARROWS_CLOCKWISE, crate::t!("刷新状态")).clicked() {
                        app.load_tunnel_status();
                    }
                });
            }

            // ---- 运行中：显示地址 + 复制 ----
            if running && !url.is_empty() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&url)
                            .size(13.0)
                            .monospace()
                            .color(p.accent),
                    );
                    if icon_button(ui, regular::COPY, crate::t!("复制")).clicked() {
                        ui.ctx().copy_text(url.clone());
                        let msg = crate::t!("隧道地址已复制").to_string();
                        app.notify_kind(&msg, crate::app::ToastKind::Ok);
                    }
                });
                ui.label(
                    RichText::new(crate::t!(
                        "这个地址是临时隧道，关机或断线后会失效；同一时间只对一个应用有效。"
                    ))
                    .size(12.0)
                    .color(p.text_muted),
                );
            }

            // ---- 错误原因：永远显示，绝不再静默 ----
            if !err_key.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(tunnel_error_text(&err_key, &err_detail))
                        .size(12.0)
                        .color(p.danger),
                );
            }

            // ---- 固定隧道的可选配置（token / 自定义域名）----
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(
                RichText::new(crate::t!("高级：使用自己的 Cloudflare 隧道（可选）"))
                    .size(13.0)
                    .strong()
                    .color(p.text_secondary),
            );
            ui.label(
                RichText::new(crate::t!(
                    "留空则使用上面的临时隧道。填入 Tunnel Token 后可用固定域名长期访问。"
                ))
                .size(12.0)
                .color(p.text_muted),
            );
            // 实测踩过的坑：这里紧接两个 `TextEdit` 时，说明文字会和
            // 「自定义域名（可选）」hint 叠在一起（说明文字占位后再叠一行）。
            // 固定留出一行高度，让 hint 落在下一行。
            ui.add_space(22.0);
            let mut tok_changed = false;
            let mut dom_changed = false;
            if let Some(st) = app.settings.as_mut() {
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut st.custom_domain)
                            .desired_width(360.0)
                            .hint_text(crate::t!("自定义域名（可选）")),
                    )
                    .lost_focus()
                {
                    dom_changed = true;
                }
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut st.tunnel_token)
                            .desired_width(360.0)
                            .hint_text("eyJhIjoi..."),
                    )
                    .lost_focus()
                {
                    tok_changed = true;
                }
            }
            if tok_changed {
                let t = app
                    .settings
                    .as_ref()
                    .map(|s| s.tunnel_token.clone())
                    .unwrap_or_default();
                app.save_setting("tunnelToken", serde_json::json!(t), crate::t!("隧道Token"));
            }
            if dom_changed {
                let d = app
                    .settings
                    .as_ref()
                    .map(|s| s.custom_domain.clone())
                    .unwrap_or_default();
                app.save_setting("customDomain", serde_json::json!(d), crate::t!("自定义域名"));
            }
        },
    );
}

fn settings_about(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    if let Some(v) = &app.version {
        section(
            ui,
            &p,
            regular::INFO,
            crate::t!("版本信息"),
            |_| {},
            |ui| {
                ui.label(ic(regular::SEAL_CHECK, &format!("当前版本：v{}", v.version)));
                ui.label(ic(
                    regular::ARROW_UP_RIGHT,
                    &format!("更新地址：{}", v.update_url),
                ));
                ui.add_space(6.0);
                if icon_button(ui, regular::DOWNLOAD_SIMPLE, crate::t!("检查更新")).clicked() {
                    app.go(
                        |c| c.post_json("/api/check-update", &serde_json::json!({})),
                        |r| crate::api::AppMsg::Action(r.map(|_| ()), crate::t!("检查更新").into()),
                    );
                }
            },
        );
    } else if app.loading {
        loading_state(ui, &p, crate::t!("正在获取版本"), crate::t!("请稍候…"));
    } else {
        empty_state(ui, &p, regular::INFO, crate::t!("暂无版本信息"), crate::t!("请确认后端服务正在运行"));
    }

    ui.add_space(12.0);
    let (first, last, total) = app
        .overview
        .as_ref()
        .map(|o| {
            (
                o.first_date.clone().unwrap_or_default(),
                o.last_date.clone().unwrap_or_default(),
                o.total_boot,
            )
        })
        .unwrap_or_default();
    section(
        ui,
        &p,
        regular::DATABASE,
        crate::t!("数据概况"),
        |_| {},
        |ui| {
            ui.label(format!("累计记录：{total} 条"));
            ui.label(format!("首次记录：{first}"));
            ui.label(format!("最近记录：{last}"));
        },
    );

    ui.add_space(12.0);
    ui.label(
        RichText::new(crate::t!("纯原生桌面端（egui/eframe），无 WebView / 无 HTML。"))
            .size(12.0)
            .color(p.text_muted),
    );
}

// ===================== 管理 =====================

pub fn admin(ui: &mut egui::Ui, app: &mut BootTrackerApp) {
    let p = app.palette();
    let hour12 = app.hour12();
    ui.horizontal(|ui| {
        ui.label(ic(regular::SHIELD, crate::t!("数据管理")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(crate::tf!(
                    "回收站 {} 条 · 备份 {} 个",
                    app.trash.len(),
                    app.backups.len()
                ))
                .size(12.0)
                .color(p.text_muted),
            );
        });
    });
    ui.add_space(10.0);

    // 回收站（默认展开，操作按钮在卡片头右侧）
    let trash = app.trash.clone();
    let mut ask_clear_trash = false;
    let mut ask_refresh_trash = false;
    section(
        ui,
        &p,
        regular::TRASH,
        &format!("回收站（{}）", trash.len()),
        |ui| {
            if ui
                .button(ic(regular::ARROWS_CLOCKWISE, crate::t!("刷新")))
                .clicked()
            {
                ask_refresh_trash = true;
            }
            if ui.button(ic(regular::TRASH_SIMPLE, crate::t!("清空"))).clicked() {
                ask_clear_trash = true;
            }
        },
        |ui| {
            if trash.is_empty() {
                empty_state(ui, &p, regular::TRASH, crate::t!("回收站为空"), crate::t!("删除的记录会在这里保留一段时间"));
                return;
            }
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                for (i, s) in trash.iter().enumerate() {
                    let t = app.row_t(ui.ctx(), i);
                    ui.scope(|ui| {
                        ui.set_opacity(t);
                        list_row(ui, &p, i % 2 == 1, |ui| {
                            ui.label(fmt::fmt_datetime(&s.boot_time, hour12));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(RichText::new(regular::TRASH).color(p.danger))
                                    .on_hover_text(crate::t!("永久删除（不可恢复）"))
                                    .clicked()
                                {
                                    app.request_delete_trash(s.id.clone());
                                }
                                if ui
                                    .button(ic(regular::CLOCK_COUNTER_CLOCKWISE, crate::t!("恢复")))
                                    .clicked()
                                {
                                    app.request_restore_trash(s.id.clone());
                                }
                                ui.label(
                                    RichText::new(match s.duration {
                                        Some(d) => fmt::fmt_duration(d),
                                        None => "-".into(),
                                    })
                                    .font(FontId::new(14.0, egui::FontFamily::Monospace))
                                    .color(p.text_secondary),
                                );
                            });
                        });
                    });
                }
            });
        },
    );
    if ask_refresh_trash {
        app.load_trash();
    }
    if ask_clear_trash {
        app.request_clear_trash();
    }

    ui.add_space(12.0);
    let backups = app.backups.clone();
    let mut ask_clean = false;
    let mut ask_refresh_backups = false;
    section(
        ui,
        &p,
        regular::FLOPPY_DISK,
        &format!("备份（{}）", backups.len()),
        |ui| {
            if ui.button(ic(regular::ARROWS_CLOCKWISE, crate::t!("刷新"))).clicked() {
                ask_refresh_backups = true;
            }
            if ui.button(ic(regular::BROOM, crate::t!("清理旧备份"))).clicked() {
                ask_clean = true;
            }
        },
        |ui| {
            if backups.is_empty() {
                empty_state(ui, &p, regular::FLOPPY_DISK, crate::t!("暂无备份"), crate::t!("开启自动备份后会自动生成"));
                return;
            }
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                for (i, b) in backups.iter().enumerate() {
                    let t = app.row_t(ui.ctx(), i);
                    ui.scope(|ui| {
                        ui.set_opacity(t);
                        list_row(ui, &p, i % 2 == 1, |ui| {
                            ui.label(ic(regular::FLOPPY_DISK, b));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui
                                    .button(RichText::new(regular::TRASH).color(p.danger))
                                    .on_hover_text(crate::t!("删除该备份文件"))
                                    .clicked()
                                {
                                    app.request_delete_backup(b.clone());
                                }
                                if ui
                                    .button(ic(regular::CLOCK_COUNTER_CLOCKWISE, crate::t!("恢复")))
                                    .on_hover_text(crate::t!("恢复该备份（覆盖当前数据）"))
                                    .clicked()
                                {
                                    app.request_restore_backup(b.clone());
                                }
                            });
                        });
                    });
                }
            });
        },
    );
    if ask_refresh_backups {
        app.load_backups();
    }
    if ask_clean {
        app.request_clean_backups();
    }

    ui.add_space(12.0);
    section(
        ui,
        &p,
        regular::ARROW_UP,
        crate::t!("版本号维护"),
        |_| {},
        |ui| {
            ui.label(
                RichText::new(crate::t!("用于开发/发版时递增 version.json 中的版本号。"))
                    .size(12.0)
                    .color(p.text_muted),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                for (label, icon, kind, tip) in [
                    (crate::t!("补丁"), regular::ARROW_UP, "patch", "1.0.0 → 1.0.1"),
                    (crate::t!("次要"), regular::ARROW_UP_RIGHT, "minor", "1.0.0 → 1.1.0"),
                    (crate::t!("主要"), regular::ROCKET, "major", "1.0.0 → 2.0.0"),
                ] {
                    if ui
                        .button(ic(icon, &format!("升级 {label}")))
                        .on_hover_text(tip)
                        .clicked()
                    {
                        app.go(
                            move |c| c.post_json("/api/version/bump", &serde_json::json!({"type":kind})),
                            |r| crate::api::AppMsg::Action(r.map(|_| ()), crate::t!("升级版本").into()),
                        );
                    }
                }
            });
        },
    );

    ui.add_space(16.0);
    card(ui, &p, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(regular::WARNING_OCTAGON)
                    .size(18.0)
                    .color(p.danger),
            );
            ui.label(
                RichText::new(crate::t!("危险操作"))
                    .size(15.0)
                    .strong()
                    .color(p.danger),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(ic(regular::WARNING_OCTAGON, crate::t!("清空全部数据")).color(p.danger))
                    .clicked()
                {
                    app.request_clear_data();
                }
            });
        });
        ui.label(
            RichText::new(crate::t!("清空后所有会话记录将被删除（备份文件保留）。"))
                .size(12.0)
                .color(p.text_muted),
        );
    });
}

// 抑制未使用告警（保留类型导入以备扩展）
#[allow(dead_code)]
fn _use_types(_: &Overview) {}

/// 供 `chart` 之外的页面共享：把 `DateTime<Utc>` 转成小时制文案（预留）
#[allow(dead_code)]
fn fmt_dt(dt: DateTime<Utc>, hour12: bool) -> String {
    fmt::fmt_datetime(&dt.to_rfc3339(), hour12)
}
