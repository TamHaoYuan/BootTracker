//! 应用布局：固定侧栏 + 顶栏 + 内容区（含快捷键 / 在线状态 / 版本入口）

use crate::store::{self, Route};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// 菜单项
struct NavItem {
    route: Route,
    label: &'static str,
    icon: NavIcon,
}

#[derive(Clone, Copy)]
enum NavIcon {
    Dashboard,
    Records,
    Charts,
    Admin,
    Settings,
}

fn nav_icon_svg(icon: NavIcon) -> impl IntoView {
    let (path, extra): (String, Option<&'static str>) = match icon {
        NavIcon::Dashboard => (
            "M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z".into(),
            None,
        ),
        NavIcon::Records => (
            "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6zM8 12h8v2H8v-2zm0 4h8v2H8v-2zm0-8h4v2H8V8z".into(),
            None,
        ),
        NavIcon::Charts => (
            "M18 20V10M12 20V4M6 20v-6".into(),
            Some("M18 20V10M12 20V4M6 20v-6"),
        ),
        NavIcon::Admin => (
            "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.2 2.2 0 0 1-3.13 0 2.2 2.2 0 0 1 0-3.13l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z".into(),
            None,
        ),
        NavIcon::Settings => (
            "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z".into(),
            Some("M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.08a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.08a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.08a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"),
        ),
    };
    let _ = extra;
    view! {
        <svg
            class="menu-icon"
            width="18" height="18" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2"
            stroke-linecap="round" stroke-linejoin="round"
        >
            <path d=path></path>
        </svg>
    }
}

/// 侧栏导航时钟（每秒，仅重渲染自身）
#[component]
fn NavClock() -> impl IntoView {
    let now = store::now_signal();
    view! {
        <span class="nav-clock">
            {move || {
                now.get();
                crate::fmt::fmt_now_clock()
            }}
        </span>
    }
}

#[component]
pub fn AppLayout() -> impl IntoView {
    let route = store::route();
    let collapsed = store::sidebar_collapsed();
    let hover_expanded = RwSignal::new(false);
    let mode = crate::theme::mode();
    let online = store::online();
    let app_version = store::app_version();

    let main_group = vec![
        NavItem { route: Route::Dashboard, label: "首页", icon: NavIcon::Dashboard },
        NavItem { route: Route::Records, label: "开机记录", icon: NavIcon::Records },
        NavItem { route: Route::Charts, label: "图表", icon: NavIcon::Charts },
    ];
    let sys_group = vec![
        NavItem { route: Route::Admin, label: "管理", icon: NavIcon::Admin },
        NavItem { route: Route::Settings, label: "设置", icon: NavIcon::Settings },
    ];

    // 版本号三击（1.5s 内）→ Admin
    let click_count = RwSignal::new(0i32);
    let click_timer: RwSignal<Option<i32>> = RwSignal::new(None);
    let handle_version_click = move |_| {
        click_count.update(|c| *c += 1);
        // 重置计数定时器
        if let Some(t) = click_timer.get_untracked() {
            if let Some(w) = web_sys::window() {
                w.clear_timeout_with_handle(t);
            }
        }
        let c2 = click_count;
        let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
            c2.set(0);
        });
        if let Some(w) = web_sys::window() {
            let t = w.set_timeout_with_callback(cb.unchecked_ref()).unwrap_or(0);
            click_timer.set(Some(t));
        }
        if click_count.get_untracked() >= 3 {
            click_count.set(0);
            if let Some(t) = click_timer.get_untracked() {
                if let Some(w) = web_sys::window() {
                    w.clear_timeout_with_handle(t);
                }
            }
            store::navigate(Route::Admin);
        }
    };

    // 键盘快捷键：Ctrl+Shift+A → Admin；Alt+1~5 切页
    {
        let r = route;
        Effect::new(move |_| {
            r.get(); // 依赖路由（保持 handler 引用最新路由状态）
        });
    }
    let keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if ev.ctrl_key() && ev.shift_key() && (key == "A" || key == "a") {
            ev.prevent_default();
            store::navigate(Route::Admin);
            return;
        }
        if ev.alt_key() && !ev.ctrl_key() && !ev.shift_key() {
            let target = match key.as_str() {
                "1" => Some(Route::Dashboard),
                "2" => Some(Route::Records),
                "3" => Some(Route::Charts),
                "4" => Some(Route::Admin),
                "5" => Some(Route::Settings),
                _ => None,
            };
            if let Some(t) = target {
                ev.prevent_default();
                store::navigate(t);
            }
        }
    };
    let k2 = keydown;
    Effect::new(move |_| {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev| {
            k2(ev);
        });
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    });

    let sider_class = move || {
        let mut cls = String::new();
        if collapsed.get() {
            cls.push_str("sider collapsed");
            if hover_expanded.get() {
                cls.push_str(" expanded-hover");
            }
        } else {
            cls.push_str("sider");
        }
        cls
    };

    let render_group = move |label: &'static str, items: Vec<NavItem>| {
        let r = store::route();
        view! {
            <div class="menu-label">{label}</div>
            {items
                .into_iter()
                .map(|item| {
                    let r2 = r;
                    let (label, icon) = (item.label, item.icon);
                    let rt = item.route;
                    view! {
                        <div
                            class=move || if r2.get() == rt { "menu-item active" } else { "menu-item" }
                            on:click=move |_| store::navigate(rt)
                        >
                            {nav_icon_svg(icon)}
                            <span class="menu-label-text">{label}</span>
                        </div>
                    }
                })
                .collect_view()}
        }
    };

    let group_main = render_group("主功能", main_group);
    let group_sys = render_group("系统", sys_group);

    view! {
        <div class="app-root">
            // Glass Orb 背景装饰（仅首页）
            <Show when=move || route.get() == Route::Dashboard>
                <div class="glass-orb orb-1"></div>
                <div class="glass-orb orb-2"></div>
                <div class="glass-orb orb-3"></div>
            </Show>
            <div class="app-shell">
                <div
                    class=sider_class
                    on:mouseenter=move |_| {
                        if collapsed.get() {
                            hover_expanded.set(true);
                        }
                    }
                    on:mouseleave=move |_| hover_expanded.set(false)
                >
                    // 品牌区
                    <div class="sider-brand">
                        <img src="icons/icon.png" alt="BootTracker" width="28" height="28" />
                        <span class="sider-brand-name">"BootTracker"</span>
                    </div>
                    // 菜单
                    <div class="menu">
                        {group_main}
                        {group_sys}
                    </div>
                    // 底部：在线 + 时钟 + 版本 + 模式切换
                    <div class="sider-foot">
                        <div class="sider-foot-inner">
                            <div class="sider-foot-row">
                                <span class="online-badge">
                                    <span class=move || {
                                        if online.get() { "online-dot" } else { "online-dot offline" }
                                    }></span>
                                    {move || if online.get() { "在线" } else { "离线" }}
                                </span>
                                <NavClock />
                            </div>
                            <div class="sider-foot-row">
                                <span class="version-tag" on:click=handle_version_click>
                                    {move || {
                                        let v = app_version.get();
                                        if v.is_empty() { String::new() } else { format!("v{v}") }
                                    }}
                                </span>
                                <button
                                    class=move || {
                                        if mode.get() == crate::theme::AppMode::Dark {
                                            "mode-toggle dark-mode"
                                        } else {
                                            "mode-toggle"
                                        }
                                    }
                                    title=move || if mode.get() == crate::theme::AppMode::Dark {
                                        "切换到浅色模式"
                                    } else {
                                        "切换到深色模式"
                                    }
                                    on:click=move |_| crate::theme::toggle_mode()
                                >
                                    {move || {
                                        if mode.get() == crate::theme::AppMode::Dark {
                                            view! {
                                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                    <circle cx="12" cy="12" r="5"></circle>
                                                    <line x1="12" y1="1" x2="12" y2="3"></line>
                                                    <line x1="12" y1="21" x2="12" y2="23"></line>
                                                    <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
                                                    <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
                                                    <line x1="1" y1="12" x2="3" y2="12"></line>
                                                    <line x1="21" y1="12" x2="23" y2="12"></line>
                                                    <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
                                                    <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
                                                </svg>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
                                                </svg>
                                            }.into_any()
                                        }
                                    }}
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
                <div class=move || if collapsed.get() { "main collapsed" } else { "main" }>
                    <header class="app-header">
                        <button
                            class="mode-toggle"
                            style="font-size:16px;color:var(--text-secondary)"
                            title=move || if collapsed.get() { "展开侧栏" } else { "收起侧栏" }
                            on:click=move |_| collapsed.update(|c| *c = !*c)
                        >
                            {move || {
                                if collapsed.get() {
                                    view! {
                                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                            <polyline points="13 17 18 12 13 7"></polyline>
                                            <polyline points="6 17 11 12 6 7"></polyline>
                                        </svg>
                                    }.into_any()
                                } else {
                                    view! {
                                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                            <polyline points="11 17 6 12 11 7"></polyline>
                                            <polyline points="18 17 13 12 18 7"></polyline>
                                        </svg>
                                    }.into_any()
                                }
                            }}
                        </button>
                        <span class="app-header-title">{move || route.get().title()}</span>
                    </header>
                    <main class="app-content page-enter">
                        {move || {
                            match route.get() {
                                Route::Dashboard => view! { <crate::pages::dashboard::Dashboard /> }.into_any(),
                                Route::Records => view! { <crate::pages::records::Records /> }.into_any(),
                                Route::Charts => view! { <crate::pages::charts::Charts /> }.into_any(),
                                Route::Admin => view! { <crate::pages::admin::Admin /> }.into_any(),
                                Route::Settings => view! { <crate::pages::settings::Settings /> }.into_any(),
                            }
                        }}
                    </main>
                </div>
            </div>
        </div>
    }
}
