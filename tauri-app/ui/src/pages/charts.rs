//! 图表分析页：柱状/折线/饼图 + 热度图（SVG，视觉与网页端一致）

use crate::api::{TrendStat, WeeklyStat};
use crate::fmt;
use crate::store;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone)]
struct TrendRow {
    date: String,
    stat: TrendStat,
}

#[derive(Clone)]
struct WeeklyRow {
    week_key: String,
    stat: WeeklyStat,
}

const W: f64 = 1000.0;
const H: f64 = 360.0;
const PAD_L: f64 = 70.0;
const PAD_R: f64 = 30.0;
const PAD_T: f64 = 30.0;
const PAD_B: f64 = 50.0;

const PIE_COLORS: [&str; 6] = [
    "#8b5cf6", "#3b82f6", "#10b981", "#f97316", "#ef4444", "#d946ef",
];

fn ms_to_hours(ms: f64) -> f64 {
    ms / 3600000.0
}

fn fmt_hours_axis(hours: f64) -> String {
    if hours == 0.0 {
        "0h".to_string()
    } else if hours < 1.0 {
        format!("{}m", (hours * 60.0).round() as i64)
    } else {
        let h = hours.floor() as i64;
        let m = ((hours - h as f64) * 60.0).round() as i64;
        if m > 0 {
            format!("{h}h{m}m")
        } else {
            format!("{h}h")
        }
    }
}

fn pad2(n: i64) -> String {
    format!("{n:02}")
}

#[component]
pub fn Charts() -> impl IntoView {
    let settings = store::settings_store();
    let mode = crate::theme::mode();
    let theme = crate::theme::theme();

    let days = RwSignal::new(30i64);
    let chart_type = RwSignal::new("line".to_string());
    let trend_loading = RwSignal::new(false);
    let weekly_loading = RwSignal::new(false);
    let trend_rows = RwSignal::new(Vec::<TrendRow>::new());
    let weekly_rows = RwSignal::new(Vec::<WeeklyRow>::new());
    let chart_initialized = RwSignal::new(false);

    // 默认图表类型跟随设置（首次拉到 settings 时初始化）
    Effect::new(move |_| {
        if chart_initialized.get() {
            return;
        }
        if let Some(s) = settings.get() {
            chart_initialized.set(true);
            if s.default_chart_type == "bar" || s.default_chart_type == "line" {
                chart_type.set(s.default_chart_type.clone());
            }
        }
    });

    // 加载趋势（days 变化时；首次也会执行）
    Effect::new(move |_| {
        let d = days.get();
        load_trend(d, trend_loading, trend_rows);
    });

    // 饼图 → 加载周统计
    Effect::new(move |_| {
        if chart_type.get() == "pie" {
            let wl = weekly_loading;
            let wr = weekly_rows;
            spawn_local(async move {
                wl.set(true);
                match crate::api::StatsApi::weekly().await {
                    Ok(data) => {
                        let mut keys: Vec<String> = data.keys().cloned().collect();
                        keys.sort();
                        let rows: Vec<WeeklyRow> = keys
                            .into_iter()
                            .filter_map(|k| data.get(&k).map(|s| WeeklyRow { week_key: k, stat: s.clone() }))
                            .collect();
                        let n = rows.len();
                        wr.set(rows.into_iter().skip(n.saturating_sub(12)).collect());
                    }
                    Err(e) => crate::toast::toast_error(&format!("加载周统计失败：{e}")),
                }
                wl.set(false);
            });
        }
    });

    let has_any_data = Memo::new(move |_| !trend_rows.get().is_empty());
    let today = fmt::get_local_today();

    view! {
        <div>
            <div class="toolbar">
                <div>
                    <h4 class="page-title">"数据分析"</h4>
                    <div class="hint-text" style="margin-top:4px">
                        {move || {
                            let start = fmt::shift_date(&today, -(days.get() - 1));
                            format!("数据范围：{} 天（{} ~ {}）", days.get(), start, today)
                        }}
                    </div>
                </div>
                <div class="toolbar-group">
                    <span style="color:var(--text-secondary)">"最近"</span>
                    <select
                        class="select-input"
                        style="width:100px"
                        on:change=move |ev| {
                            if let Ok(n) = event_target_value(&ev).parse::<i64>() {
                                days.set(n);
                            }
                        }
                    >
                        <option value="30" selected=days.get_untracked() == 30>"30 天"</option>
                        <option value="60" selected=days.get_untracked() == 60>"60 天"</option>
                        <option value="90" selected=days.get_untracked() == 90>"90 天"</option>
                    </select>
                </div>
            </div>

            <Show
                when=move || has_any_data.get() || trend_loading.get()
                fallback=|| view! {
                    <div class="card">
                        <crate::components::EmptyState
                            title="暂无开机会话数据"
                            description="记录第一次开机后再来查看分析图表吧"
                        />
                    </div>
                }
            >
                <div class="card" style="margin-bottom:16px">
                    <div class="card-head">
                        <span class="card-title">"开机时长趋势"</span>
                        <div class="segmented">
                            {[
                                ("bar", "柱状图"),
                                ("line", "折线图"),
                                ("pie", "饼图"),
                            ]
                                .iter()
                                .map(|(v, label)| {
                                    let v = v.to_string();
                                    let v1 = v.clone();
                                    let v2 = v.clone();
                                    view! {
                                        <span
                                            class=move || if chart_type.get() == v1 { "segmented-item active" } else { "segmented-item" }
                                            on:click=move |_| chart_type.set(v2.clone())
                                        >
                                            {*label}
                                        </span>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                    {move || {
                        let rows = trend_rows.get();
                        let loading = trend_loading.get() || (chart_type.get() == "pie" && weekly_loading.get());
                        if loading {
                            return view! { <div class="chart-loading"><div class="spin large"></div></div> }.into_any();
                        }
                        match chart_type.get().as_str() {
                            "bar" => render_bar(&rows).into_any(),
                            "line" => render_line(&rows).into_any(),
                            "pie" => render_pie(weekly_rows.get()),
                            _ => ().into_any(),
                        }
                    }}
                </div>

                {move || {
                    let rows = trend_rows.get();
                    if rows.is_empty() {
                        return ().into_any();
                    }
                    view! { <div class="card" style="margin-bottom:16px">{render_top10(&rows)}</div> }.into_any()
                }}

                <div class="card">
                    <div class="card-head">
                        <span class="card-title">
                            "开机热度图"
                            <span class="card-extra">"（颜色越深表示开机次数越多）"</span>
                        </span>
                    </div>
                    {move || {
                        if trend_loading.get() {
                            return view! { <div class="chart-loading"><div class="spin large"></div></div> }.into_any();
                        }
                        render_heatmap(&trend_rows.get(), mode.get(), theme.get()).into_any()
                    }}
                </div>
            </Show>
        </div>
    }
}

fn load_trend(days: i64, loading: RwSignal<bool>, rows: RwSignal<Vec<TrendRow>>) {
    spawn_local(async move {
        loading.set(true);
        match crate::api::StatsApi::trend(days).await {
            Ok(resp) => {
                let mut keys: Vec<String> = resp.data.keys().cloned().collect();
                keys.sort();
                rows.set(keys
                    .into_iter()
                    .filter_map(|k| resp.data.get(&k).map(|s| TrendRow { date: k, stat: s.clone() }))
                    .collect());
            }
            Err(e) => crate::toast::toast_error(&format!("加载趋势数据失败：{e}")),
        }
        loading.set(false);
    });
}

/* ================= 主图渲染 ================= */

fn y_ticks_svg(max_val: f64) -> impl IntoView {
    let inner_h = H - PAD_T - PAD_B;
    (0..=5).map(move |i| {
        let y = PAD_T + (inner_h / 5.0) * i as f64;
        let val = max_val * (1.0 - i as f64 / 5.0);
        let dash = if i == 5 { "" } else { "4 4" };
        view! {
            <g>
                <line x1=PAD_L x2=W - PAD_R y1=y y2=y
                    style="stroke:rgba(var(--accent-rgb), 0.12)" stroke-width="1" stroke-dasharray=dash />
                <text x=PAD_L - 8.0 y=y + 4.0 text-anchor="end" font-size="11"
                    style="fill:var(--text-muted)">
                    {fmt_hours_axis(val)}
                </text>
            </g>
        }
    }).collect_view()
}

fn x_labels_svg<F: Fn(usize) -> (f64, String)>(count: usize, f: F) -> impl IntoView {
    let step = (count as f64 / 10.0).ceil().max(1.0) as usize;
    (0..count)
        .filter(|i| count <= 15 || i % step == 0)
        .map(|i| {
            let (x, label) = f(i);
            let y = H - PAD_B + 18.0;
            let rot = format!("rotate(-30 {x} {y})");
            view! {
                <text x=x y=y text-anchor="middle" font-size="10"
                    style="fill:var(--text-muted)" transform=rot>
                    {label}
                </text>
            }
        })
        .collect_view()
}

fn render_bar(rows: &[TrendRow]) -> impl IntoView + use<'_> {
    let inner_w = W - PAD_L - PAD_R;
    let inner_h = H - PAD_T - PAD_B;
    let values: Vec<f64> = rows.iter().map(|r| ms_to_hours(r.stat.total_duration)).collect();
    let max_val = values.iter().cloned().fold(1.0f64, f64::max);
    let n = rows.len() as f64;
    let slot = inner_w / n;
    let bar_w = (slot * 0.65).max(2.0);

    let bars = rows.iter().enumerate().map(|(i, row)| {
        let val = ms_to_hours(row.stat.total_duration);
        let bh = (val / max_val) * inner_h;
        let x = PAD_L + slot * i as f64 + (slot - bar_w) / 2.0;
        let y = PAD_T + inner_h - bh;
        let title = format!("{}\n总时长：{}", row.date, fmt::fmt_duration(row.stat.total_duration));
        view! {
            <rect x=x y=y width=bar_w height=bh rx="3" opacity="0.85"
                style="fill:var(--accent)" title=title />
        }
    }).collect_view();

    let slot_c = slot;
    let labels = x_labels_svg(rows.len(), move |i| {
        let x = PAD_L + slot_c * i as f64 + slot_c / 2.0;
        (x, rows[i].date[5..].to_string())
    });

    view! {
        <svg viewBox="0 0 1000 360" style="width:100%;height:auto">
            {y_ticks_svg(max_val)}
            {bars}
            {labels}
        </svg>
    }
}

fn render_line(rows: &[TrendRow]) -> impl IntoView + use<'_> {
    let inner_w = W - PAD_L - PAD_R;
    let inner_h = H - PAD_T - PAD_B;
    let values: Vec<f64> = rows.iter().map(|r| ms_to_hours(r.stat.total_duration)).collect();
    let max_val = values.iter().cloned().fold(1.0f64, f64::max);
    let n = rows.len();
    let step_x = if n > 1 { inner_w / (n - 1) as f64 } else { 0.0 };

    let pts: Vec<(f64, f64)> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let val = ms_to_hours(r.stat.total_duration);
            let x = PAD_L + step_x * i as f64;
            let y = PAD_T + inner_h - (val / max_val) * inner_h;
            (x, y)
        })
        .collect();

    let area_path = if !pts.is_empty() {
        let mut d = format!("M {PAD_L},{}", PAD_T + inner_h);
        for (x, y) in &pts {
            d.push_str(&format!(" L {x},{y}"));
        }
        d.push_str(&format!(" L {},{} Z", PAD_L + if n > 1 { inner_w } else { 0.0 }, PAD_T + inner_h));
        d
    } else {
        String::new()
    };

    let polyline = pts
        .iter()
        .map(|(x, y)| format!("{x},{y}"))
        .collect::<Vec<_>>()
        .join(" ");

    let dots = rows.iter().enumerate().map(|(i, row)| {
        let (x, y) = pts[i];
        let title = format!("{}\n总时长：{}", row.date, fmt::fmt_duration(row.stat.total_duration));
        view! {
            <circle cx=x cy=y r="4" style="fill:var(--accent);stroke:var(--bg-card)" stroke-width="2" title=title />
        }
    }).collect_view();

    let step_x_c = step_x;
    let labels = x_labels_svg(n, move |i| (PAD_L + step_x_c * i as f64, rows[i].date[5..].to_string()));

    view! {
        <svg viewBox="0 0 1000 360" style="width:100%;height:auto">
            {y_ticks_svg(max_val)}
            {(!area_path.is_empty()).then(|| view! {
                <path d=area_path style="fill:var(--accent)" opacity="0.12" />
            })}
            {(!pts.is_empty()).then(|| view! {
                <polyline points=polyline style="stroke:var(--accent)" stroke-width="2.5"
                    fill="none" stroke-linejoin="round" stroke-linecap="round" />
            })}
            {dots}
            {labels}
        </svg>
    }
}

fn render_pie(rows: Vec<WeeklyRow>) -> AnyView {
    if rows.is_empty() {
        return ().into_any();
    }
    let cx = W / 2.0;
    let cy = H / 2.0;
    let radius = ((W - PAD_L - PAD_R).min(H - PAD_T - PAD_B)) / 2.0 - 10.0;
    let total: i64 = rows.iter().map(|r| r.stat.boot_count).sum();
    if total == 0 {
        return ().into_any();
    }

    let mut cumulative = 0i64;
    let mut segments = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let value = row.stat.boot_count;
        let start_a = (cumulative as f64 / total as f64) * std::f64::consts::TAU;
        cumulative += value;
        let end_a = (cumulative as f64 / total as f64) * std::f64::consts::TAU;
        let large_arc = if end_a - start_a > std::f64::consts::PI { 1 } else { 0 };
        let x1 = cx + radius * start_a.sin();
        let y1 = cy - radius * start_a.cos();
        let x2 = cx + radius * end_a.sin();
        let y2 = cy - radius * end_a.cos();
        let path = if value == total {
            format!(
                "M {a},{cy} A {r},{r} 0 1 1 {b},{cy} A {r},{r} 0 1 1 {a},{cy} Z",
                a = cx - radius,
                b = cx + radius,
                r = radius,
            )
        } else {
            format!("M {cx},{cy} L {x1},{y1} A {r},{r} 0 {large_arc} 1 {x2},{y2} Z", r = radius)
        };
        let mid_a = (start_a + end_a) / 2.0;
        let label_r = radius * 0.65;
        let label_x = cx + label_r * mid_a.sin();
        let label_y = cy - label_r * mid_a.cos();
        let pct = value as f64 / total as f64 * 100.0;
        segments.push((path, PIE_COLORS[i % PIE_COLORS.len()], row.week_key.clone(), value, pct, label_x, label_y));
    }

    let legend = segments
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let y = PAD_T as usize + i * 22 + 13;
            let color = s.1;
            let label = format!("{} · {}次 ({:.1}%)", s.2[5..].to_string(), s.3, s.4);
            view! {
                <g transform=move || format!("translate(20, {})", PAD_T as usize + i * 22)>
                    <rect x="0" y="2" width="14" height="14" rx="3" fill=color />
                    <text x="20" y=format!("{y}") font-size="11" style="fill:var(--text-secondary)">
                        {label}
                    </text>
                </g>
            }
        })
        .collect_view();

    let seg_views = segments
        .iter()
        .map(|s| {
            let (path, color, week, value, pct, lx, ly) = s.clone();
            let title = format!("{}\n开机次数：{}\n占比：{:.1}%", week, value, pct);
            let show_pct = pct >= 5.0;
            let pct_text = format!("{pct:.0}%");
            view! {
                <g>
                    <path d=path fill=color stroke="var(--bg-card)" stroke-width="2" opacity="0.9" title=title />
                    {show_pct.then(|| view! {
                        <text x=lx y=ly text-anchor="middle" dominant-baseline="middle"
                            font-size="11" fill="#fff" font-weight="600">
                            {pct_text}
                        </text>
                    })}
                </g>
            }
        })
        .collect_view();

    view! {
        <svg viewBox="0 0 1000 360" style="width:100%;height:auto">
            {seg_views}
            {legend}
        </svg>
    }
    .into_any()
}

/* ================= 近 10 天数据表 ================= */

fn render_top10(rows: &[TrendRow]) -> impl IntoView + use<'_> {
    let mut top: Vec<&TrendRow> = rows.iter().collect();
    top.sort_by(|a, b| b.date.cmp(&a.date));
    top.truncate(10);

    view! {
        <div class="card-head">
            <span class="card-title">"详细数据（最近 10 天）"</span>
        </div>
        <div class="table-wrap">
            <table class="tbl">
                <thead>
                    <tr>
                        <th>"日期"</th>
                        <th class="num">"开机次数"</th>
                        <th class="num">"关机次数"</th>
                        <th class="num">"总时长"</th>
                        <th class="num">"平均时长"</th>
                    </tr>
                </thead>
                <tbody>
                    {top
                        .iter()
                        .map(|r| {
                            view! {
                                <tr>
                                    <td class="mono">{r.date.clone()}</td>
                                    <td class="num">{r.stat.boot_count.to_string()}</td>
                                    <td class="num">{r.stat.shutdown_count.to_string()}</td>
                                    <td class="num">
                                        <span class="mono" style="color:var(--accent)">
                                            {fmt::fmt_duration(r.stat.total_duration)}
                                        </span>
                                    </td>
                                    <td class="num">
                                        <span class="mono">{fmt::fmt_duration(r.stat.avg_duration)}</span>
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

/* ================= 热度图 ================= */

fn render_heatmap(
    rows: &[TrendRow],
    mode: crate::theme::AppMode,
    theme: crate::theme::ThemeName,
) -> impl IntoView + use<'_> {
    if rows.is_empty() {
        return ().into_any();
    }

    let count_of = |date: &str| -> i64 {
        rows.iter().find(|r| r.date == date).map(|r| r.stat.boot_count).unwrap_or(0)
    };

    let start = fmt::week_start(&rows[0].date);
    let end = &rows[rows.len() - 1].date;
    let mut weeks: Vec<Vec<String>> = Vec::new();
    let mut week_starts: Vec<String> = Vec::new();
    {
        let mut z = fmt::parse_date_str(&start).unwrap_or(0);
        let end_z = fmt::parse_date_str(end).unwrap_or(0);
        let mut cur: Vec<String> = Vec::new();
        while z <= end_z {
            if cur.is_empty() {
                week_starts.push(fmt::date_str_from_days(z));
            }
            cur.push(fmt::date_str_from_days(z));
            if cur.len() == 7 {
                weeks.push(std::mem::take(&mut cur));
            }
            z += 1;
        }
        if !cur.is_empty() {
            weeks.push(cur);
        }
    }

    let cell = 18.0;
    let gap = 4.0;
    let left_pad = 80.0;
    let top_pad = 30.0;
    let bottom_pad = 50.0;
    let width = left_pad + 7.0 * (cell + gap) + 20.0;
    let height = top_pad + weeks.len() as f64 * (cell + gap) + bottom_pad;

    let is_mica = theme == crate::theme::ThemeName::Mica;
    let is_light = mode == crate::theme::AppMode::Light;
    let (heat_levels, heat_empty) = if is_mica {
        (
            vec![
                "rgba(148,163,184,0.25)",
                "rgba(148,163,184,0.4)",
                "rgba(148,163,184,0.6)",
                "rgba(148,163,184,0.8)",
            ],
            "rgba(255,255,255,0.03)",
        )
    } else if is_light {
        (
            vec![
                "rgba(var(--accent-rgb),0.12)",
                "rgba(var(--accent-rgb),0.25)",
                "rgba(var(--accent-rgb),0.45)",
                "rgba(var(--accent-rgb),0.7)",
            ],
            "rgba(0,0,0,0.04)",
        )
    } else {
        (
            vec![
                "rgba(var(--accent-rgb),0.5)",
                "rgba(var(--accent-rgb),0.65)",
                "rgba(var(--accent-rgb),0.8)",
                "rgba(var(--accent-rgb),0.95)",
            ],
            "rgba(30,41,59,0.5)",
        )
    };

    let heat_levels_c = heat_levels.clone();
    let get_heat = move |count: i64| -> &'static str {
        if count <= 0 {
            return heat_empty;
        }
        if count <= 2 {
            return heat_levels_c[0];
        }
        if count <= 4 {
            return heat_levels_c[1];
        }
        if count <= 6 {
            return heat_levels_c[2];
        }
        heat_levels_c[3]
    };

    // 月份标签：每周首日月份变化时标注
    let mut month_labels: Vec<(f64, String)> = Vec::new();
    let mut last_month: i64 = -1;
    for (wi, week) in weeks.iter().enumerate() {
        if let Some(d) = week.first() {
            let m = d[5..7].parse::<i64>().unwrap_or(-1);
            if m != last_month {
                month_labels.push((
                    left_pad,
                    format!("{}-{}", &d[0..4], pad2(m)),
                ));
                let _ = wi;
                last_month = m;
            }
        }
    }

    let weekday_labels = ["一", "二", "三", "四", "五", "六", "日"];

    view! {
        <svg viewBox=format!("0 0 {width} {height}") style="width:100%;height:auto;max-width:800px">
            {weekday_labels
                .iter()
                .enumerate()
                .map(|(i, lbl)| {
                    let y = top_pad + i as f64 * (cell + gap) + cell - 4.0;
                    view! {
                        <text x=left_pad - 6.0 y=y text-anchor="end" font-size="10" style="fill:var(--text-muted)">
                            {*lbl}
                        </text>
                    }
                })
                .collect_view()}
            {weeks
                .iter()
                .enumerate()
                .map(|(wi, week)| {
                    let wy = top_pad + wi as f64 * (cell + gap);
                    view! {
                        <g>
                            {(wi % 2 == 0).then(|| {
                                let label = week_starts[wi][5..].to_string();
                                view! {
                                    <text x=left_pad - 6.0 y=wy + cell - 4.0 text-anchor="end" font-size="9" style="fill:var(--text-muted)">
                                        {label}
                                    </text>
                                }
                            })}
                            {week
                                .iter()
                                .enumerate()
                                .map(|(di, date)| {
                                    let x = left_pad + di as f64 * (cell + gap);
                                    let count = count_of(date);
                                    let fill = get_heat(count);
                                    let title = format!("{}\n开机次数：{}", date, count);
                                    view! {
                                        <rect x=x y=wy width=cell height=cell rx="3"
                                            style=format!("fill:{fill}") title=title />
                                    }
                                })
                                .collect_view()}
                        </g>
                    }
                })
                .collect_view()}
            {month_labels
                .iter()
                .map(|(x, label)| {
                    let y = top_pad + weeks.len() as f64 * (cell + gap) + 20.0;
                    view! {
                        <text x=*x y=y font-size="10" style="fill:var(--text-muted)">
                            {label.clone()}
                        </text>
                    }
                })
                .collect_view()}
            // 图例
            <g transform=format!("translate({left_pad}, {})", top_pad + weeks.len() as f64 * (cell + gap) + 36.0)>
                <text x="0" y="10" font-size="10" style="fill:var(--text-muted)">"少"</text>
                {(0..5usize).map(|i| {
                    let x = 24.0 + i as f64 * (cell + gap);
                    let fill = if i == 0 { heat_empty } else { heat_levels[i - 1] };
                    view! {
                        <rect x=x y="0" width=cell height=cell rx="3" style=format!("fill:{fill}") />
                    }
                }).collect_view()}
                <text x=format!("{}", 24.0 + 5.0 * (cell + gap)) y="10" font-size="10" style="fill:var(--text-muted)">"多"</text>
            </g>
        </svg>
    }
    .into_any()
}
