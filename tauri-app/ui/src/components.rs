//! 基础组件：StatusPill / EmptyState / KpiCard / TrendBars / LiveDuration / 表单控件

use crate::fmt;
use crate::store;
use leptos::prelude::*;

/* ================= StatusPill ================= */

#[derive(Clone, Copy, PartialEq)]
pub enum PillTone {
    Success,
    Accent,
    Muted,
    Danger,
}

#[component]
pub fn StatusPill(tone: PillTone, children: Children) -> impl IntoView {
    let class = match tone {
        PillTone::Success => "status-pill pill-success",
        PillTone::Accent => "status-pill pill-accent",
        PillTone::Muted => "status-pill pill-muted",
        PillTone::Danger => "status-pill pill-danger",
    };
    view! {
        <span class=class>
            <span class="pill-dot"></span>
            {children()}
        </span>
    }
}

/* ================= EmptyState ================= */

#[component]
pub fn EmptyState(
    #[prop(into)] title: String,
    #[prop(optional, into)] description: String,
    #[prop(optional)] mut children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="empty-state">
            <div class="empty-icon">
                <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="22 12 16 12 14 15 10 15 8 12 2 12"></polyline>
                    <path d="M5.45 5.11L2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"></path>
                </svg>
            </div>
            <div class="empty-title">{title}</div>
            {(!description.is_empty()).then(|| view! { <div class="empty-desc">{description}</div> })}
            {children.map(|c| view! { <div class="empty-action">{c()}</div> })}
        </div>
    }
}

/* ================= KpiCard ================= */

#[derive(Clone, Copy, PartialEq)]
pub enum TrendDir {
    Up,
    Down,
    Flat,
}

#[derive(Clone)]
pub struct KpiTrend {
    pub dir: TrendDir,
    pub text: String,
}

#[component]
pub fn KpiCard(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    trend: Option<KpiTrend>,
    #[prop(default = false)] hero: bool,
    #[prop(default = false)] accent: bool,
    #[prop(default = false)] mono: bool,
    #[prop(default = false)] loading: bool,
    #[prop(optional)] mut children: Option<Children>,
) -> impl IntoView {
    let card_class = if hero {
        "card today-boot-card kpi-hero"
    } else {
        "card"
    };
    view! {
        <div class=card_class>
            {move || {
                if loading {
                    view! {
                        <div class="skeleton-line w50"></div>
                        <div class="skeleton-line w70"></div>
                    }
                        .into_any()
                } else {
                    let trend_v = trend.clone();
                    let label_c = label.clone();
                    let value_c = value.clone();
                    let mono_c = mono;
                    let accent_c = accent;
                    let hero_c = hero;
                    view! {
                        <div class="kpi-inner">
                            <span class=move || format!("kpi-label{}", if hero_c { " hero" } else { "" })>
                                {label_c}
                            </span>
                            <div class="kpi-value-row">
                                <span class=move || {
                                    let mut cls = String::from("kpi-value tnum");
                                    if hero_c {
                                        cls.push_str(" hero");
                                    }
                                    if accent_c {
                                        cls.push_str(" accent");
                                    }
                                    if mono_c {
                                        cls.push_str(" mono");
                                    }
                                    cls
                                }>{value_c}</span>
                                {children.take().map(|c| c())}
                            </div>
                            {trend_v.map(|t| {
                                let (cls, arrow) = match t.dir {
                                    TrendDir::Up => ("kpi-trend trend-up", "↑"),
                                    TrendDir::Down => ("kpi-trend trend-down", "↓"),
                                    TrendDir::Flat => ("kpi-trend trend-flat", "→"),
                                };
                                view! {
                                    <span class=cls>
                                        {arrow}
                                        " "
                                        {t.text}
                                    </span>
                                }
                            })}
                        </div>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

/* ================= TrendBars（近 14 日开机趋势） ================= */

#[derive(Clone, PartialEq)]
pub struct TrendPoint {
    pub date: String,
    pub count: i64,
}

#[component]
pub fn TrendBars(data: Vec<TrendPoint>, #[prop(default = 130)] height: u32) -> impl IntoView {
    let max = data.iter().map(|d| d.count).max().unwrap_or(1).max(1);
    let last_idx = data.len().saturating_sub(1);
    let mid_idx = last_idx / 2;
    let first_label = data.first().map(|d| d.date[5..].to_string()).unwrap_or_default();
    let mid_label = data
        .get(mid_idx)
        .map(|d| d.date[5..].to_string())
        .unwrap_or_default();
    view! {
        <div>
            <div class="trend-bars" style=move || format!("height:{height}px")>
                {data
                    .iter()
                    .enumerate()
                    .map(|(i, d)| {
                        let pct = (d.count as f64 / max as f64) * 100.0;
                        let is_today = i == last_idx;
                        let h = if d.count > 0 {
                            format!("{}%", pct.max(4.0))
                        } else {
                            "2px".to_string()
                        };
                        let bg = if is_today {
                            "var(--accent)".to_string()
                        } else {
                            "rgba(var(--accent-rgb), 0.28)".to_string()
                        };
                        let title = format!("{}：开机 {} 次", d.date, d.count);
                        view! {
                            <div class="trend-col" title=title>
                                <div class="trend-bar" style=move || format!("height:{h};background:{bg}")></div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="trend-scale tnum">
                <span>{first_label}</span>
                <span>{mid_label}</span>
                <span class="today-label">"今天"</span>
            </div>
        </div>
    }
}

/* ================= LiveDuration ================= */

#[component]
pub fn LiveDuration(
    #[prop(into)] boot_time: Option<String>,
    #[prop(default = false)] hero: bool,
) -> impl IntoView {
    let now = store::now_signal();
    let text = move || {
        now.get();
        boot_time
            .as_ref()
            .and_then(|b| fmt::parse_iso_ms(b))
            .map(|ms| fmt::live_duration(ms, fmt::now_ms()))
            .unwrap_or_else(|| "—".to_string())
    };
    view! {
        <span class=move || format!("tnum live-dur{}", if hero { " hero" } else { "" })>
            {text}
        </span>
    }
}

/* ================= 表单控件 ================= */

/// 开关
#[component]
pub fn Toggle(
    #[prop(into)] checked: Signal<bool>,
    #[prop(into)] on_change: Callback<bool>,
    #[prop(default = false)] disabled: bool,
) -> impl IntoView {
    view! {
        <div
            class=move || if checked.get() { "switch on" } else { "switch" }
            style:cursor=move || if disabled { "not-allowed" } else { "pointer" }
            on:click=move |_| {
                if !disabled {
                    on_change.run(!checked.get());
                }
            }
        ></div>
    }
}

/// 分段控制器
#[derive(Clone)]
pub struct SegmentOpt {
    pub value: String,
    pub label: String,
}

impl SegmentOpt {
    pub fn new(value: &str, label: &str) -> Self {
        Self {
            value: value.to_string(),
            label: label.to_string(),
        }
    }
}

#[component]
pub fn Segmented(
    value: RwSignal<String>,
    options: Vec<SegmentOpt>,
) -> impl IntoView {
    view! {
        <div class="segmented">
            {options
                .into_iter()
                .map(|opt| {
                    let v = opt.value.clone();
                    let v_class = v.clone();
                    view! {
                        <span
                            class=move || if value.get() == v_class { "segmented-item active" } else { "segmented-item" }
                            on:click=move |_| value.set(v.clone())
                        >
                            {opt.label}
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// 单选组
#[component]
pub fn RadioGroup(
    value: RwSignal<String>,
    options: Vec<SegmentOpt>,
) -> impl IntoView {
    view! {
        <div class="radio-group">
            {options
                .into_iter()
                .map(|opt| {
                    let v = opt.value.clone();
                    let v_class = v.clone();
                    view! {
                        <span
                            class=move || if value.get() == v_class { "radio checked" } else { "radio" }
                            on:click=move |_| value.set(v.clone())
                        >
                            <span class="radio-dot"></span>
                            {opt.label}
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// 文本输入
#[component]
pub fn TextInput(
    #[prop(into)] value: RwSignal<String>,
    #[prop(optional, into)] placeholder: String,
    #[prop(default = false)] password: bool,
) -> impl IntoView {
    view! {
        <input
            class="text-input"
            type=move || if password { "password" } else { "text" }
            prop:value=move || value.get()
            placeholder=placeholder
            on:input=move |ev| {
                let t = event_target_value(&ev);
                value.set(t);
            }
        />
    }
}

/// 多行文本
#[component]
pub fn TextArea(#[prop(into)] value: RwSignal<String>, #[prop(optional, into)] placeholder: String, #[prop(default = 2)] rows: usize) -> impl IntoView {
    view! {
        <textarea
            class="text-input text-area"
            rows=rows
            placeholder=placeholder
            prop:value=move || value.get()
            on:input=move |ev| {
                let t = event_target_value(&ev);
                value.set(t);
            }
        ></textarea>
    }
}

/// 数字输入
#[component]
pub fn NumberInput(
    #[prop(into)] value: RwSignal<i64>,
    min: i64,
    max: i64,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <input
            class="number-input"
            type="number"
            min=min
            max=max
            disabled=move || disabled.get()
            prop:value=move || value.get().to_string()
            on:input=move |ev| {
                if let Ok(n) = event_target_value(&ev).parse::<i64>() {
                    value.set(n);
                }
            }
        />
    }
}

/// 下拉选择
#[component]
pub fn SelectInput(#[prop(into)] value: RwSignal<String>, options: Vec<SegmentOpt>) -> impl IntoView {
    view! {
        <select
            class="select-input"
            on:change=move |ev| {
                value.set(event_target_value(&ev));
            }
        >
            {options
                .into_iter()
                .map(|opt| {
                    let v = opt.value.clone();
                    let sel = value.get_untracked() == v;
                    view! {
                        <option value=v.clone() selected=sel>
                            {opt.label}
                        </option>
                    }
                })
                .collect_view()}
        </select>
    }
}

/// datetime-local 输入（本地时间）
#[component]
pub fn DateTimeInput(#[prop(into)] value: RwSignal<String>) -> impl IntoView {
    view! {
        <input
            class="datetime-input"
            type="datetime-local"
            prop:value=move || value.get()
            on:input=move |ev| {
                let t = event_target_value(&ev);
                value.set(t);
            }
        />
    }
}
