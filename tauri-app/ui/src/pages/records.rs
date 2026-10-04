//! 开机记录页：只读视图（表格 / 时间轴）+ 导出 CSV/XLSX + 日期筛选

use crate::api::BootSession;
use crate::components::*;
use crate::fmt;
use crate::store::{self, RecordsView};
use crate::table::{Column, DataTable};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

/* ================= 导出工具 ================= */

fn escape_csv_field(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

pub fn export_csv(sessions: &[BootSession]) {
    let mut lines = vec!["开机时间,关机时间,时长(ms),时长字符串".to_string()];
    for s in sessions {
        let boot = fmt::fmt_full_time(Some(&s.boot_time));
        let shut = s
            .shutdown_time
            .as_ref()
            .map(|t| fmt::fmt_full_time(Some(t)))
            .unwrap_or_default();
        let dur_ms = s.duration.map(|d| d.to_string()).unwrap_or_default();
        let dur_str = s.duration.map(fmt::fmt_duration).unwrap_or_default();
        lines.push(
            [boot, shut, dur_ms, dur_str]
                .iter()
                .map(|f| escape_csv_field(f))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    let mut csv = String::from("\u{feff}");
    csv.push_str(&lines.join("\r\n"));
    fmt::download_blob(
        csv.into_bytes(),
        "text/csv;charset=utf-8",
        &format!("boot-records-{}.csv", fmt::today_filename_stamp()),
    );
}

fn export_xlsx(sessions: &[BootSession]) {
    if sessions.is_empty() {
        return;
    }
    let mut workbook = rust_xlsxwriter::Workbook::new();

    // Sheet 1: 开机记录
    let sheet1 = workbook.add_worksheet().set_name("开机记录").unwrap();
    let headers = ["序号", "开机时间", "关机时间", "会话时长", "状态"];
    for (i, h) in headers.iter().enumerate() {
        sheet1.write_string(0, i as u16, *h).ok();
    }
    sheet1.set_column_width(0, 6).ok();
    sheet1.set_column_width(1, 20).ok();
    sheet1.set_column_width(2, 20).ok();
    sheet1.set_column_width(3, 14).ok();
    sheet1.set_column_width(4, 10).ok();
    for (i, s) in sessions.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet1.write_number(row, 0, (i + 1) as f64).ok();
        sheet1
            .write_string(row, 1, &fmt::fmt_full_time(Some(&s.boot_time)))
            .ok();
        sheet1
            .write_string(
                row,
                2,
                &match &s.shutdown_time {
                    Some(t) => fmt::fmt_full_time(Some(t)),
                    None => "未关机".to_string(),
                },
            )
            .ok();
        sheet1
            .write_string(
                row,
                3,
                &s.duration.map(fmt::fmt_duration).unwrap_or_else(|| "—".into()),
            )
            .ok();
        sheet1
            .write_string(
                row,
                4,
                if s.shutdown_time.is_some() { "已关机" } else { "进行中" },
            )
            .ok();
    }

    // Sheet 2: 统计
    let sheet2 = workbook.add_worksheet().set_name("统计").unwrap();
    sheet2.write_string(0, 0, "统计项").ok();
    sheet2.write_string(0, 1, "数值").ok();
    sheet2.set_column_width(0, 16).ok();
    sheet2.set_column_width(1, 20).ok();
    let total = sessions.len();
    let closed = sessions.iter().filter(|s| s.shutdown_time.is_some()).count();
    sheet2.write_string(1, 0, "导出时间").ok();
    sheet2.write_string(1, 1, &fmt::fmt_full_time(Some(&fmt::now_iso()))).ok();
    sheet2.write_string(2, 0, "总开机次数").ok();
    sheet2.write_number(2, 1, total as f64).ok();
    sheet2.write_string(3, 0, "总关机次数").ok();
    sheet2.write_number(3, 1, closed as f64).ok();
    let with_dur: Vec<f64> = sessions.iter().filter_map(|s| s.duration).collect();
    if !with_dur.is_empty() {
        let avg = with_dur.iter().sum::<f64>() / with_dur.len() as f64;
        sheet2.write_string(4, 0, "平均会话时长").ok();
        sheet2.write_string(4, 1, &fmt::fmt_duration(avg)).ok();
    }

    match workbook.save_to_buffer() {
        Ok(buf) => fmt::download_blob(
            buf,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            &format!("开机记录_{}.xlsx", fmt::today_filename_stamp()),
        ),
        Err(e) => crate::toast::toast_error(&format!("导出 XLSX 失败：{e}")),
    }
}

/* ================= 页面 ================= */

#[component]
pub fn Records() -> impl IntoView {
    let data = store::data_store().data;
    let loading = store::data_store().loading;
    let view_mode = store::records_view();

    // 筛选（本地日期，YYYY-MM-DD）
    let date_from = RwSignal::new(String::new());
    let date_to = RwSignal::new(String::new());

    let filtered = Memo::new(move |_| {
        let from = date_from.get();
        let to = date_to.get();
        let mut list: Vec<BootSession> = data
            .get()
            .sessions
            .into_iter()
            .filter(|s| {
                let d = fmt::iso_to_local_date(&s.boot_time);
                if !from.is_empty() && d < from {
                    return false;
                }
                if !to.is_empty() && d > to {
                    return false;
                }
                true
            })
            .collect();
        list.sort_by(|a, b| {
            let at = fmt::parse_iso_ms(&a.boot_time).unwrap_or(0.0);
            let bt = fmt::parse_iso_ms(&b.boot_time).unwrap_or(0.0);
            bt.partial_cmp(&at).unwrap_or(std::cmp::Ordering::Equal)
        });
        list
    });

    // 时间轴分组（最近 100 条）
    let timeline_groups = Memo::new(move |_| {
        let now = fmt::now_ms();
        let today = fmt::get_local_today();
        let mut order: Vec<String> = Vec::new();
        let mut map: std::collections::HashMap<String, Vec<BootSession>> =
            std::collections::HashMap::new();
        for s in filtered.get().into_iter().take(100) {
            let d = fmt::iso_to_local_date(&s.boot_time);
            if !map.contains_key(&d) {
                order.push(d.clone());
            }
            map.entry(d).or_default().push(s);
        }
        let _ = order.sort();
        let _ = order.reverse();
        order
            .into_iter()
            .map(|date| {
                let items = map.remove(&date).unwrap_or_default();
                let total: f64 = items
                    .iter()
                    .map(|s| fmt::session_duration(s, now))
                    .sum();
                let label = if date == today {
                    "今天".to_string()
                } else {
                    date.clone()
                };
                (date, label, total, items)
            })
            .collect::<Vec<_>>()
    });

    let columns: Vec<Column<BootSession>> = vec![
        Column::indexed(),
        Column::new("开机时间", "mono", |s: &BootSession| {
            view! { <span class="tnum mono">{fmt::fmt_full_time(Some(&s.boot_time))}</span> }
        }),
        Column::new("关机时间", "mono", |s: &BootSession| {
            view! {
                <span class="tnum mono">
                    {match &s.shutdown_time {
                        Some(t) => fmt::fmt_full_time(Some(t)),
                        None => "—".to_string(),
                    }}
                </span>
            }
        }),
        Column::new("会话时长", "num", |s: &BootSession| {
            if s.shutdown_time.is_some() {
                view! { <span class="tnum mono">{fmt::fmt_duration(fmt::session_duration(s, fmt::now_ms()))}</span> }.into_any()
            } else {
                let b = s.boot_time.clone();
                view! { <LiveDuration boot_time=b /> }.into_any()
            }
        }),
        Column::new("状态", "", |s: &BootSession| {
            if s.shutdown_time.is_some() {
                view! { <StatusPill tone=PillTone::Success>"已关机"</StatusPill> }.into_any()
            } else {
                view! { <StatusPill tone=PillTone::Accent>"进行中"</StatusPill> }.into_any()
            }
        }),
    ];
    let columns = Arc::new(columns);

    let empty_fn: Arc<dyn Fn() -> AnyView + Send + Sync> = Arc::new(|| {
        view! {
            <EmptyState
                title="等待第一次开机记录…"
                description="记录来自应用启动时自动采集；也可以手动补录历史数据。"
            />
        }
        .into_any()
    });

    // 视图模式：字符串信号 ↔ RecordsView（持久化在 uiStore）
    let view_str = RwSignal::new(
        if view_mode.get_untracked() == RecordsView::Timeline { "timeline" } else { "table" }.to_string(),
    );
    Effect::new(move |prev: Option<String>| {
        let v = view_str.get();
        if let Some(p) = prev {
            if p != v {
                view_mode.set(if v == "timeline" { RecordsView::Timeline } else { RecordsView::Table });
            }
        }
        v
    });

    let has_filter = move || !date_from.get().is_empty() || !date_to.get().is_empty();

    view! {
        <div>
            <div class="toolbar">
                <h4 class="page-title">"开机记录"</h4>
                <div class="toolbar-group">
                    <Segmented
                        value=view_str
                        options=vec![
                            SegmentOpt::new("table", "表格"),
                            SegmentOpt::new("timeline", "时间轴"),
                        ]
                    />
                    <button
                        class="btn"
                        on:click=move |_| {
                            let list = filtered.get_untracked();
                            if list.is_empty() {
                                crate::toast::toast_warning("没有可导出的记录");
                                return;
                            }
                            export_csv(&list);
                        }
                    >
                        "导出 CSV"
                    </button>
                    <button
                        class="btn"
                        disabled=move || filtered.get().is_empty()
                        on:click=move |_| {
                            let list = filtered.get_untracked();
                            if list.is_empty() {
                                return;
                            }
                            export_xlsx(&list);
                        }
                    >
                        "导出 XLSX"
                    </button>
                </div>
            </div>

            // 筛选栏
            <div class="toolbar">
                <div class="toolbar-group">
                    <input
                        class="datetime-input"
                        type="date"
                        prop:value=move || date_from.get()
                        on:input=move |ev| date_from.set(event_target_value(&ev))
                    />
                    <span class="hint-text">"至"</span>
                    <input
                        class="datetime-input"
                        type="date"
                        prop:value=move || date_to.get()
                        on:input=move |ev| date_to.set(event_target_value(&ev))
                    />
                    <button class="btn" disabled=move || !has_filter() on:click=move |_| {
                        date_from.set(String::new());
                        date_to.set(String::new());
                    }>"清除筛选"</button>
                    <button class="btn" disabled=loading.get() on:click=move |_| {
                        spawn_local(store::refresh_data())
                    }>"刷新"</button>
                </div>
                <span class="hint-text">"只读视图，管理操作请前往「管理」"</span>
            </div>

            <div class="card">
                {move || {
                    if view_mode.get() == RecordsView::Table {
                        view! {
                            <DataTable
                                rows=filtered
                                columns=(*columns).clone()
                                row_key=Arc::new(|s: &BootSession| s.id.clone())
                                loading=loading
                                empty=Some(empty_fn.clone())
                            />
                        }
                            .into_any()
                    } else {
                        let groups = timeline_groups.get();
                        if groups.is_empty() {
                            return view! {
                                <EmptyState
                                    title="等待第一次开机记录…"
                                    description="记录来自应用启动时自动采集；时间轴会按天展示开机事件流。"
                                />
                            }
                            .into_any();
                        }
                        view! {
                            <div>
                                {groups
                                    .into_iter()
                                    .map(|(_date, label, total, items)| {
                                        view! {
                                            <div>
                                                <div class="tl-day-head">
                                                    <span class="tl-day-date">{label}</span>
                                                    <span class="tl-day-sum">
                                                        {format!("{} 次 · 共 {}", items.len(), fmt::fmt_duration(total))}
                                                    </span>
                                                </div>
                                                <div class="tl-list">
                                                    {items
                                                        .into_iter()
                                                        .map(|s| {
                                                            let active = s.shutdown_time.is_none();
                                                            let boot = s.boot_time.clone();
                                                            let shut = s.shutdown_time.clone();
                                                            let item_class = if active { "tl-item tl-active" } else { "tl-item" };
                                                            let dur = s
                                                                .duration
                                                                .unwrap_or_else(|| {
                                                                    match (fmt::parse_iso_ms(&s.boot_time), s.shutdown_time.as_ref().and_then(|t| fmt::parse_iso_ms(t))) {
                                                                        (Some(b), Some(sh)) => sh - b,
                                                                        _ => 0.0,
                                                                    }
                                                                });
                                                            view! {
                                                                <div class=item_class>
                                                                    <span class="tl-dot"></span>
                                                                    <span class="tl-time">
                                                                        {fmt::fmt_clock(&boot)}
                                                                        " → "
                                                                        {match &shut {
                                                                            Some(t) => fmt::fmt_clock(t),
                                                                            None => "现在".to_string(),
                                                                        }}
                                                                    </span>
                                                                    <span class="tl-meta">
                                                                        {if active {
                                                                            view! {
                                                                                <span class="tl-dur">
                                                                                    <LiveDuration boot_time=Some(boot.clone()) />
                                                                                </span>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! { <span class="tl-dur">{fmt::fmt_duration(dur)}</span> }.into_any()
                                                                        }}
                                                                        {if active {
                                                                            view! { <StatusPill tone=PillTone::Accent>"进行中"</StatusPill> }.into_any()
                                                                        } else {
                                                                            view! { <StatusPill tone=PillTone::Success>"已关机"</StatusPill> }.into_any()
                                                                        }}
                                                                    </span>
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                                {move || {
                                    if filtered.get().len() > 100 {
                                        view! {
                                            <div class="hint-text" style="text-align:center;padding:12px 0 4px">
                                                "时间轴仅展示最近 100 条，更早记录请切换到表格视图"
                                            </div>
                                        }
                                        .into_any()
                                    } else {
                                        ().into_any()
                                    }
                                }}
                            </div>
                        }
                        .into_any()
                    }
                }}
            </div>
        </div>
    }
}
