//! BootTracker 桌面端纯 Rust 前端（Leptos CSR → WASM）
//!
//! 模块组织：
//! - api：HTTP 客户端 + 数据类型（对接 Python 后端 /api）
//! - bridge：Tauri IPC 桥（window.__TAURI__）
//! - theme：主题单一事实源（mode/theme，与网页端 zustand persist 互通）
//! - store：全局状态（路由 / 数据轮询 / 设置 / UI 偏好 / 可见性）
//! - components / table：基础组件
//! - toast（overlay.rs）：Toast + 确认模态框
//! - pages：各路由页面
//! - layout：应用布局（侧栏 + 顶栏 + 内容）
//! - fmt：时间/数字格式化

pub mod api;
pub mod bridge;
pub mod components;
pub mod fmt;
pub mod layout;
pub mod pages;
pub mod store;
pub mod table;
pub mod theme;

/// overlay.rs 同时承载 Toast 与 Modal；外部以 `crate::toast::*` 引用。
#[path = "overlay.rs"]
pub mod toast;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// 应用根组件：布局 + 全局 Toast/Modal 层 + 启动动画
#[component]
fn App() -> impl IntoView {
    // 主题：初始化 DOM（data-theme / light-mode / data-native）
    theme::mode();
    theme::theme();

    // 启动时回写后端（供小组件同步取色）+ 同步原生标题栏
    Effect::new(move |_: Option<()>| {
        spawn_local(async move {
            let mode = theme::mode().get_untracked();
            let variant = theme::theme().get_untracked();
            let _ = api::SettingsApi::update_partial(
                serde_json::json!({ "appMode": mode.as_str(), "appTheme": variant.as_str() }),
            )
            .await;
            let _ = bridge::set_theme(mode.as_str()).await;
        });
    });

    // 启动动画（与网页端 BootSplash 节奏一致）
    let show_splash = RwSignal::new(true);
    let reduced = web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .map(|mq| mq.matches())
        .unwrap_or(false);
    let ms = if reduced { 400 } else { 1350 };
    {
        let s = show_splash;
        let cb = wasm_bindgen::closure::Closure::once_into_js(move || s.set(false));
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
        }
    }

    view! {
        <layout::AppLayout />
        <toast::ToastLayer />
        <toast::ModalLayer />
        <Show when=move || show_splash.get() fallback=|| ()>
            <BootSplash />
        </Show>
    }
}

/// 启动动画（图标弹入 + 名称 tracking-in + accent 进度线）
#[component]
fn BootSplash() -> impl IntoView {
    let mode = theme::mode();
    view! {
        <div
            class="boot-splash-overlay"
            style=move || format!(
                "position:fixed;inset:0;z-index:9999;display:flex;flex-direction:column;align-items:center;justify-content:center;background:{};animation:splashFadeOut 0.4s 0.95s var(--ease-out-expo) forwards",
                if mode.get() == theme::AppMode::Dark { "#0a0a0c" } else { "#f8fafc" }
            )
        >
            <div style="display:flex;align-items:center">
                <img class="boot-splash-icon" src="icons/icon.png" alt="BootTracker" width="56" height="56"
                    style="animation:splashIconIn 0.55s var(--ease-spring) forwards" />
                <span class="boot-splash-name" style=move || format!(
                    "margin-left:14px;font-size:26px;font-weight:700;color:{};opacity:0;animation:splashNameIn 0.6s 0.3s var(--ease-out-expo) forwards",
                    if mode.get() == theme::AppMode::Dark { "#fafafa" } else { "#1e293b" }
                )>"BootTracker"</span>
            </div>
            <div class="boot-splash-progress" style=move || format!(
                "margin-top:22px;width:132px;height:2px;border-radius:1px;overflow:hidden;background:{}",
                if mode.get() == theme::AppMode::Dark { "rgba(255,255,255,0.08)" } else { "rgba(0,0,0,0.06)" }
            )>
                <div class="boot-splash-progress-bar" style="height:100%;border-radius:1px;background:var(--accent);transform-origin:left;animation:splashProgress 1.25s var(--ease-out-expo) forwards"></div>
            </div>
        </div>
    }
}

/// 应用入口：解析后端端口 → 初始化全局单例 → 挂载到 body
///
/// `#[wasm_bindgen(start)]` 让 wasm-bindgen 在模块加载时自动调用，
/// 否则链接器会把整个 `run`（及所有页面/组件/中文字符串）当作死代码 tree-shake 掉。
#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();

    spawn_local(async move {
        // 后端端口：Tauri 环境由 IPC get_app_info 下发（Python 注入 BOOTTRACKER_PORT）
        let port = if bridge::is_tauri() {
            bridge::get_app_info()
                .await
                .ok()
                .and_then(|i| i.port)
                .unwrap_or(18792)
        } else {
            18792
        };
        api::init_base(port);

        // 全局单例初始化（OnceLock）
        store::init_visibility();
        store::init_router();
        store::now_signal();
        store::init_status();
        store::start_data_polling();

        // 拉一次后端设置（布局/图表默认项用到）
        store::load_settings().await;

        leptos::mount::mount_to_body(App);
    });
}
