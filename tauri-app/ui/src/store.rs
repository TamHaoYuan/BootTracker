//! 全局状态：数据轮询 / 设置 / UI 偏好 / 可见性 / 路由 / 每秒 ticker

use crate::api;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use leptos::task::spawn_local;
use std::sync::OnceLock;

/* ================= 可见性单例 ================= */

static VISIBLE: OnceLock<RwSignal<bool>> = OnceLock::new();

pub fn visible() -> bool {
    VISIBLE.get().map(|s| s.get_untracked()).unwrap_or(true)
}

pub fn init_visibility() -> &'static RwSignal<bool> {
    VISIBLE.get_or_init(|| {
        let sig = RwSignal::new(true);
        let doc_vis = web_sys::window()
            .and_then(|w| w.document())
            .map(|d| d.visibility_state() == web_sys::VisibilityState::Visible)
            .unwrap_or(true);
        sig.set(doc_vis);

        // 1. document visibilitychange（浏览器标准信号）
        let s2 = sig;
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            let v = web_sys::window()
                .and_then(|w| w.document())
                .map(|d| d.visibility_state() == web_sys::VisibilityState::Visible)
                .unwrap_or(true);
            s2.set(v);
        });
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = doc.add_event_listener_with_callback(
                "visibilitychange",
                cb.as_ref().unchecked_ref(),
            );
        }
        cb.forget();

        // 2. Tauri 原生 window-visibility 事件（托盘隐藏时 WebView2 不保证触发标准信号）
        if crate::bridge::is_tauri() {
            let s3 = sig;
            crate::bridge::listen_window_visibility(move |v| s3.set(v));
        }
        sig
    })
}

/* ================= 每秒 ticker（时钟 / LiveDuration） ================= */

static NOW: OnceLock<RwSignal<f64>> = OnceLock::new();

pub fn now_signal() -> RwSignal<f64> {
    *NOW.get_or_init(|| {
        let sig = RwSignal::new(crate::fmt::now_ms());
        let s2 = sig;
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            s2.set(crate::fmt::now_ms());
        });
        if let Some(w) = web_sys::window() {
            let _ = w.set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 1000);
        }
        cb.forget();
        sig
    })
}

/* ================= 路由 ================= */

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Route {
    Dashboard,
    Records,
    Charts,
    Admin,
    Settings,
}

impl Route {
    pub fn title(&self) -> &'static str {
        match self {
            Route::Dashboard => "开机记录",
            Route::Records => "开机记录",
            Route::Charts => "图表",
            Route::Admin => "管理",
            Route::Settings => "设置",
        }
    }
    fn from_hash(h: &str) -> Self {
        match h {
            "/records" => Route::Records,
            "/charts" => Route::Charts,
            "/admin" => Route::Admin,
            "/settings" => Route::Settings,
            _ => Route::Dashboard,
        }
    }
    fn to_hash(&self) -> &'static str {
        match self {
            Route::Dashboard => "/dashboard",
            Route::Records => "/records",
            Route::Charts => "/charts",
            Route::Admin => "/admin",
            Route::Settings => "/settings",
        }
    }
}

static ROUTE: OnceLock<RwSignal<Route>> = OnceLock::new();

pub fn route() -> RwSignal<Route> {
    ROUTE.get().cloned().unwrap_or_else(|| RwSignal::new(Route::Dashboard))
}

pub fn navigate(r: Route) {
    if let Some(s) = ROUTE.get() {
        s.set(r);
    }
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_hash(Route::to_hash(&r));
    }
}

pub fn init_router() {
    ROUTE.get_or_init(|| {
        let init = web_sys::window()
            .and_then(|w| w.location().hash().ok())
            .map(|h| Route::from_hash(&h))
            .unwrap_or(Route::Dashboard);
        let sig = RwSignal::new(init);
        // 浏览器前进/后退（hash 变化但非 navigate 触发）→ 同步信号
        let s2 = sig;
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            let h = web_sys::window()
                .and_then(|w| w.location().hash().ok())
                .unwrap_or_default();
            let r = Route::from_hash(&h);
            if s2.get_untracked() != r {
                s2.set(r);
            }
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("hashchange", cb.as_ref().unchecked_ref());
        }
        cb.forget();
        sig
    });
}

/* ================= 开机数据 ================= */

pub struct DataStore {
    pub data: RwSignal<api::BootData>,
    pub loading: RwSignal<bool>,
}

static DATA: OnceLock<DataStore> = OnceLock::new();

pub fn data_store() -> &'static DataStore {
    DATA.get_or_init(|| DataStore {
        data: RwSignal::new(api::BootData::default()),
        loading: RwSignal::new(false),
    })
}

/// 手动刷新（带 loading）
pub async fn refresh_data() {
    let st = data_store();
    st.loading.set(true);
    match api::DataApi::get_data().await {
        Ok(d) => st.data.set(d),
        Err(e) => crate::toast::toast_error(&format!("加载数据失败：{e}")),
    }
    st.loading.set(false);
}

/// 静默刷新（轮询用）
async fn silent_refresh() {
    if let Ok(d) = api::DataApi::get_data().await {
        data_store().data.set(d);
    }
}

/// 首次加载 + 每 5s 轮询（不可见时暂停）
pub fn start_data_polling() {
    spawn_local(async move {
        refresh_data().await;
    });
    let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
        if visible() {
            spawn_local(async move {
                silent_refresh().await;
            });
        }
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 5000);
    }
    cb.forget();
}

/* ================= 后端设置 ================= */

static SETTINGS: OnceLock<RwSignal<Option<api::AppSettings>>> = OnceLock::new();

pub fn settings_store() -> RwSignal<Option<api::AppSettings>> {
    SETTINGS.get().cloned().unwrap_or_else(|| RwSignal::new(None))
}

pub async fn load_settings() {
    match api::SettingsApi::get().await {
        Ok(s) => settings_store().set(Some(s)),
        Err(e) => crate::toast::toast_error(&format!("加载设置失败：{e}")),
    }
}

/* ================= UI 偏好（侧栏折叠 / 记录视图，localStorage） ================= */

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RecordsView {
    Table,
    Timeline,
}

struct UiStore {
    sidebar_collapsed: RwSignal<bool>,
    records_view: RwSignal<RecordsView>,
}

static UI: OnceLock<UiStore> = OnceLock::new();

fn ui() -> &'static UiStore {
    UI.get_or_init(|| {
        // zustand persist 格式：{"state":{"sidebarCollapsed":true,"recordsView":"table"}}
        let (collapsed, view) = (|| {
            let ls = web_sys::window()?.local_storage().ok()??;
            let raw = ls.get_item("boottracker-ui").ok()??;
            let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
            let state = v.get("state")?;
            let c = state.get("sidebarCollapsed")?.as_bool()?;
            let tv = state
                .get("recordsView")?
                .as_str()?
                .contains("timeline")
                .then_some(RecordsView::Timeline)
                .unwrap_or(RecordsView::Table);
            Some((c, tv))
        })()
        .unwrap_or((true, RecordsView::Table));

        let store = UiStore {
            sidebar_collapsed: RwSignal::new(collapsed),
            records_view: RwSignal::new(view),
        };
        let s = store.sidebar_collapsed;
        let v = store.records_view;
        Effect::new(move || {
            let c = s.get();
            let v = if v.get() == RecordsView::Timeline {
                "timeline"
            } else {
                "table"
            };
            if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
                let _ = ls.set_item(
                    "boottracker-ui",
                    &serde_json::json!({
                        "state": { "sidebarCollapsed": c, "recordsView": v },
                        "version": 0
                    })
                    .to_string(),
                );
            }
        });
        store
    })
}

pub fn sidebar_collapsed() -> RwSignal<bool> {
    ui().sidebar_collapsed
}

pub fn records_view() -> RwSignal<RecordsView> {
    ui().records_view
}

/* ================= 在线状态 / 版本 ================= */

static ONLINE: OnceLock<RwSignal<bool>> = OnceLock::new();
static APP_VERSION: OnceLock<RwSignal<String>> = OnceLock::new();

pub fn online() -> RwSignal<bool> {
    ONLINE.get().cloned().unwrap_or_else(|| RwSignal::new(false))
}

pub fn app_version() -> RwSignal<String> {
    APP_VERSION
        .get()
        .cloned()
        .unwrap_or_else(|| RwSignal::new(String::new()))
}

pub fn init_status() {
    let o = ONLINE.get_or_init(|| RwSignal::new(false));
    let v = APP_VERSION.get_or_init(|| RwSignal::new(String::new()));
    let _ = (o, v);
    spawn_local(async move {
        let ok = api::DataApi::ping().await.map(|p| p.ok).unwrap_or(false);
        online().set(ok);
        if let Ok(info) = api::VersionApi::get().await {
            if !info.version.is_empty() {
                app_version().set(info.version);
            }
        }
    });
}
