//! 首页 Dashboard：KPI 卡条 + 近 14 日趋势 + 本次会话 + 最近会话

use crate::api::BootSession;
use crate::components::*;
use crate::fmt;
use crate::store;
use crate::table::{Column, DataTable};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
struct Kpi {
    today_boots: i64,
    yesterday_boots: i64,
    today_usage: f64,
    yesterday_usage: f64,
    avg7: f64,
    prev_avg7: f64,
    trend14: Vec<TrendPoint>,
    trend14_total: i64,
}

fn compute_kpi(sessions: &[BootSession]) -> Kpi {
    let now = fmt::now_ms();
    let today = fmt::get_local_today();
    let yesterday = fmt::shift_date(&today, -1);

    let mut by_day: std::collections::HashMap<String, Vec<&BootSession>> =
        std::collections::HashMap::new();
    for s in sessions {
        by_day
            .entry(fmt::iso_to_local_date(&s.boot_time))
            .or_default()
            .push(s);
    }

    let sum_of = |day: &str| -> f64 {
        by_day
            .get(day)
            .map(|list| list.iter().map(|s| fmt::session_duration(s, now)).sum())
            .unwrap_or(0.0)
    };

    let mut last7 = 0i64;
    let mut prev7 = 0i64;
    for i in 0..7 {
        last7 += by_day.get(&fmt::shift_date(&today, -i)).map(|l| l.len()).unwrap_or(0) as i64;
        prev7 += by_day
            .get(&fmt::shift_date(&today, -7 - i))
            .map(|l| l.len())
            .unwrap_or(0) as i64;
    }

    let mut trend14 = Vec::new();
    let mut trend14_total = 0i64;
    for i in (0..14).rev() {
        let d = fmt::shift_date(&today, -i);
        let count = by_day.get(&d).map(|l| l.len()).unwrap_or(0) as i64;
        trend14_total += count;
        trend14.push(TrendPoint { date: d, count });
    }

    Kpi {
        today_boots: by_day.get(&today).map(|l| l.len()).unwrap_or(0) as i64,
        yesterday_boots: by_day.get(&yesterday).map(|l| l.len()).unwrap_or(0) as i64,
        today_usage: sum_of(&today),
        yesterday_usage: sum_of(&yesterday),
        avg7: last7 as f64 / 7.0,
        prev_avg7: prev7 as f64 / 7.0,
        trend14,
        trend14_total,
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let data = store::data_store().data;
    let loading = store::data_store().loading;

    let kpi = Memo::new(move |_| compute_kpi(&data.get().sessions));
    let recent = Memo::new(move |_| {
        let mut list = data.get().sessions;
        list.sort_by(|a, b| {
            let at = fmt::parse_iso_ms(&a.boot_time).unwrap_or(0.0);
            let bt = fmt::parse_iso_ms(&b.boot_time).unwrap_or(0.0);
            bt.partial_cmp(&at).unwrap_or(std::cmp::Ordering::Equal)
        });
        list.truncate(5);
        list
    });

    let active_count = Memo::new(move |_| {
        data.get()
            .sessions
            .iter()
            .filter(|s| s.shutdown_time.is_none())
            .count() as i64
    });
    let active_session = Memo::new(move |_| {
        data.get()
            .sessions
            .iter()
            .find(|s| s.shutdown_time.is_none())
            .cloned()
    });

    let handle_shutdown = move |_| {
        let Some(active) = active_session.get_untracked() else {
            return;
        };
        spawn_local(async move {
            match crate::api::DataApi::update_session(
                &active.id,
                crate::api::SessionPatch {
                    boot_time: None,
                    shutdown_time: Some(fmt::now_iso()),
                },
            )
            .await
            {
                Ok(()) => {
                    crate::toast::toast_success("已记录关机");
                    store::refresh_data().await;
                }
                Err(e) => crate::toast::toast_error(&format!("记录关机失败：{e}")),
            }
        });
    };

    let columns: Vec<Column<BootSession>> = vec![
        Column::new("开机时间", "mono", |s: &BootSession| {
            view! { <span class="tnum mono">{fmt::fmt_full_time(Some(&s.boot_time))}</span> }
        }),
        Column::new("关机时间", "mono", |s: &BootSession| {
            view! {
                <span class="tnum mono" style="--c:1">
                    {match &s.shutdown_time {
                        Some(t) => fmt::fmt_full_time(Some(t)),
                        None => "—".to_string(),
                    }}
                </span>
            }
        }),
        Column::new("时长", "num", |s: &BootSession| {
            if s.shutdown_time.is_some() {
                view! {
                    <span class="tnum mono">
                        {fmt::fmt_duration(fmt::session_duration(s, fmt::now_ms()))}
                    </span>
                }
                .into_any()
            } else {
                let b = s.boot_time.clone();
                view! { <LiveDuration boot_time=b /> }.into_any()
            }
        }),
        Column::new("状态", "", |s: &BootSession| {
            if s.shutdown_time.is_some() {
                view! { <StatusPill tone=PillTone::Muted>"已结束"</StatusPill> }.into_any()
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
                description="记录来自应用启动时自动采集；也可以到「管理」页面手动补录历史数据。"
            />
        }
        .into_any()
    });

    let boot_diff = move || kpi.get().today_boots - kpi.get().yesterday_boots;

    view! {
        <div>
            // ===== KPI 卡条 =====
            <div class="row kpi-row">
                <div class="col col-lg">
                    {move || {
                        let k = kpi.get();
                        let (dir, text) = if boot_diff() > 0 {
                            (TrendDir::Up, format!("较昨日 +{} 次", boot_diff()))
                        } else if boot_diff() < 0 {
                            (TrendDir::Down, format!("较昨日 {} 次", boot_diff()))
                        } else {
                            (TrendDir::Flat, "与昨日持平".to_string())
                        };
                        view! {
                            <KpiCard
                                hero=true
                                accent=true
                                label="今日开机"
                                value=k.today_boots.to_string()
                                trend=Some(KpiTrend { dir, text })
                            />
                        }
                    }}
                </div>
                <div class="col col-md">
                    {move || {
                        let k = kpi.get();
                        view! {
                            <KpiCard
                                mono=true
                                label="今日使用时长"
                                value=fmt::fmt_duration(k.today_usage)
                                trend=Some(KpiTrend {
                                    dir: TrendDir::Flat,
                                    text: format!("昨日 {}", fmt::fmt_duration(k.yesterday_usage)),
                                })
                            />
                        }
                    }}
                </div>
                <div class="col col-sm">
                    {move || {
                        let k = kpi.get();
                        let (dir, text) = if k.avg7 > k.prev_avg7 {
                            (TrendDir::Up, format!("前 7 日 {:.1}", k.prev_avg7))
                        } else if k.avg7 < k.prev_avg7 {
                            (TrendDir::Down, format!("前 7 日 {:.1}", k.prev_avg7))
                        } else {
                            (TrendDir::Flat, format!("前 7 日 {:.1}", k.prev_avg7))
                        };
                        view! {
                            <KpiCard
                                label="7 日日均开机"
                                value=format!("{:.1}", k.avg7)
                                trend=Some(KpiTrend { dir, text })
                            />
                        }
                    }}
                </div>
                <div class="col col-sm">
                    {move || {
                        let d = data.get();
                        view! {
                            <KpiCard
                                label="累计记录"
                                value=(d.boot_count.max(d.sessions.len() as i64)).to_string()
                                trend=None
                            />
                        }
                    }}
                </div>
                <div class="col col-md">
                    {move || {
                        let c = active_count.get();
                        view! {
                            <KpiCard label="进行中会话" value=c.to_string() trend=None>
                                {if c > 0 {
                                    view! { <StatusPill tone=PillTone::Accent>"使用中"</StatusPill> }.into_any()
                                } else {
                                    view! { <StatusPill tone=PillTone::Muted>"无"</StatusPill> }.into_any()
                                }}
                            </KpiCard>
                        }
                    }}
                </div>
            </div>

            // ===== 趋势 + 本次会话 =====
            <div class="row">
                <div class="col col-lg">
                    <div class="card">
                        <div class="card-head">
                            <span class="card-title">
                                <span class="title-icon">
                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
                                    </svg>
                                </span>
                                "近 14 日开机趋势"
                            </span>
                            <span class="card-extra tnum">
                                {move || format!("共 {} 次", kpi.get().trend14_total)}
                            </span>
                        </div>
                        {move || {
                            let k = kpi.get();
                            if k.trend14_total == 0 && !loading.get() {
                                view! {
                                    <EmptyState
                                        title="近 14 日没有开机记录"
                                        description="应用会在每次启动时自动记录开机时间，数据积累后这里会展示趋势。"
                                    />
                                }
                                    .into_any()
                            } else {
                                view! { <TrendBars data=k.trend14.clone() height=130 /> }.into_any()
                            }
                        }}
                    </div>
                </div>
                <div class="col">
                    <div class="card full-height">
                        <div class="card-head">
                            <span class="card-title">"本次会话"</span>
                        </div>
                        {move || {
                            let active = active_session.get();
                            view! {
                                <div class="kv-row">
                                    <span class="kv-label">"本次开机时间"</span>
                                    <span class="kv-value tnum mono">
                                        {fmt::fmt_full_time(active.as_ref().map(|a| a.boot_time.as_str()))}
                                    </span>
                                </div>
                                <div class="kv-row">
                                    <span class="kv-label">"当前状态"</span>
                                    {if active.is_some() {
                                        view! { <StatusPill tone=PillTone::Accent>"进行中"</StatusPill> }.into_any()
                                    } else {
                                        view! { <StatusPill tone=PillTone::Muted>"已结束"</StatusPill> }.into_any()
                                    }}
                                </div>
                                <div class="kv-row">
                                    <span class="kv-label">"已运行时长"</span>
                                    <LiveDuration
                                        hero=true
                                        boot_time=active.as_ref().map(|a| a.boot_time.clone())
                                    />
                                </div>
                                <button
                                    class="btn primary danger block large"
                                    disabled=active.is_none()
                                    on:click=handle_shutdown
                                >
                                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M18.36 6.64a9 9 0 1 1-12.73 0"></path>
                                        <line x1="12" y1="2" x2="12" y2="12"></line>
                                    </svg>
                                    "记录关机"
                                </button>
                            }
                        }}
                    </div>
                </div>
            </div>

            // ===== 最近会话 =====
            <div class="card" style="margin-top:16px">
                <div class="card-head">
                    <span class="card-title">
                        "最近会话"
                        <span class="card-extra">"（最近 5 条）"</span>
                    </span>
                    <button
                        class="btn small"
                        disabled=loading.get()
                        on:click=move |_| spawn_local(store::refresh_data())
                    >
                        {move || if loading.get() { view! { <span class="spin"></span> }.into_any() } else { "刷新".into_any() }}
                    </button>
                </div>
                <DataTable
                    rows=recent
                    columns=(*columns).clone()
                    row_key=Arc::new(|s: &BootSession| s.id.clone())
                    loading=loading
                    empty=Some(empty_fn)
                />
            </div>
        </div>
    }
}
