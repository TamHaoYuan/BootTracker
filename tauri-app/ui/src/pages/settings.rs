//! 系统设置页：外观 / 通用 / 网络 / 关于

use crate::api;
use crate::components::*;
use crate::fmt;
use crate::store;
use crate::theme::{self, AppMode, ThemeName, AVAILABLE_THEMES};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use leptos::task::spawn_local;

/// 局部更新设置：乐观更新 + 失败回滚
fn save_field<F: FnOnce(&mut api::AppSettings)>(mutate: F, patch: serde_json::Value) {
    let st = store::settings_store();
    let prev = st.get_untracked();
    if let Some(mut s) = st.get_untracked() {
        mutate(&mut s);
        st.set(Some(s));
    }
    spawn_local(async move {
        match api::SettingsApi::update_partial(patch).await {
            Ok(()) => crate::toast::toast_success("已保存"),
            Err(e) => {
                crate::toast::toast_error(&format!("保存失败：{e}"));
                st.set(prev);
            }
        }
    });
}

#[component]
pub fn Settings() -> impl IntoView {
    let settings = store::settings_store();
    let loading = RwSignal::new(true);
    let tab = RwSignal::new("appearance".to_string());

    let tunnel_url = RwSignal::new(Option::<String>::None);
    let tunnel_loading = RwSignal::new(false);
    let version_info = RwSignal::new(String::new());
    let app_mode_label = RwSignal::new("生产".to_string());
    let version_history = RwSignal::new(Vec::<api::VersionHistoryEntry>::new());
    let history_loading = RwSignal::new(false);
    let bump_notes = RwSignal::new(String::new());
    let check_result: RwSignal<Option<api::CheckUpdateResp>> = RwSignal::new(None);
    let check_loading = RwSignal::new(false);

    // 首次加载
    {
        spawn_local(async move {
            store::load_settings().await;
            loading.set(false);
            if let Ok(v) = api::VersionApi::get().await {
                version_info.set(v.version);
            }
            if let Ok(info) = crate::bridge::get_app_info().await {
                if let Some(m) = info.mode {
                    app_mode_label.set(if m == "dev" { "开发".to_string() } else { "生产".to_string() });
                }
            }
        });
    }

    // 版本历史
    {
        spawn_local(async move {
            history_loading.set(true);
            match api::VersionApi::history().await {
                Ok(mut h) => {
                    h.sort_by(|a, b| b.date.cmp(&a.date));
                    version_history.set(h);
                }
                Err(e) => crate::toast::toast_error(&format!("加载版本历史失败：{e}")),
            }
            history_loading.set(false);
        });
    }

    // 隧道地址
    let fetch_tunnel = move || {
        spawn_local(async move {
            tunnel_loading.set(true);
            match api::TunnelApi::get_url().await {
                Ok(r) => tunnel_url.set(r.url),
                Err(_) => tunnel_url.set(None),
            }
            tunnel_loading.set(false);
        });
    };
    Effect::new(move |_| {
        if settings.get().map(|s| s.tunnel_enabled).unwrap_or(false) {
            fetch_tunnel();
        } else {
            tunnel_url.set(None);
        }
    });

    let current_theme = theme::theme();
    let current_mode = theme::mode();

    view! {
        <div style="max-width:960px;margin:0 auto">
            <div class="card" style="padding:0">
                <div class="card-head" style="padding:12px 24px 0;margin-bottom:0">
                    <span class="card-title" style="font-size:17px">"系统设置"</span>
                </div>
                {move || {
                    if loading.get() {
                        return view! { <div class="tbl-empty"><div class="spin large"></div></div> }.into_any();
                    }
                    view! {
                        <div style="padding:0 24px 24px">
                            <div class="tabs large" style="margin-top:12px">
                                {[
                                    ("appearance", "外观"),
                                    ("general", "通用"),
                                    ("network", "网络"),
                                    ("about", "关于"),
                                ]
                                    .iter()
                                    .map(|(k, label)| {
                                        let k = k.to_string();
                                        let k1 = k.clone();
                                        let k2 = k.clone();
                                        view! {
                                            <span
                                                class=move || if tab.get() == k1 { "tabs-item active" } else { "tabs-item" }
                                                on:click=move |_| tab.set(k2.clone())
                                            >
                                                {*label}
                                            </span>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            {move || {
                                match tab.get().as_str() {
                                    "general" => render_general(settings).into_any(),
                                    "network" => {
                                        let f = fetch_tunnel.clone();
                                        render_network(settings, tunnel_url, tunnel_loading, f).into_any()
                                    }
                                    "about" => {
                                        render_about(
                                            version_info,
                                            app_mode_label,
                                            version_history,
                                            history_loading,
                                            bump_notes,
                                            check_result,
                                            check_loading,
                                        )
                                            .into_any()
                                    }
                                    _ => render_appearance(settings, current_mode, current_theme).into_any(),
                                }
                            }}
                        </div>
                    }
                        .into_any()
                }}
            </div>
        </div>
    }
}

/* ================= 外观 ================= */

fn render_appearance(
    settings: RwSignal<Option<api::AppSettings>>,
    mode: RwSignal<AppMode>,
    theme: RwSignal<ThemeName>,
) -> impl IntoView {
    let mode_str = RwSignal::new(mode.get_untracked().as_str().to_string());
    Effect::new(move |_| {
        mode_str.set(mode.get().as_str().to_string());
    });

    // 显示偏好：值变化 → 立即保存
    let chart_type_sig = RwSignal::new(
        settings.get_untracked().map(|s| s.default_chart_type).unwrap_or_else(|| "line".to_string()),
    );
    Effect::new(move |prev: Option<String>| {
        let v = chart_type_sig.get();
        if let Some(p) = prev {
            if p != v {
                save_field(
                    |s: &mut api::AppSettings| s.default_chart_type = v.clone(),
                    serde_json::json!({ "defaultChartType": v }),
                );
            }
        }
        v
    });
    let time_format_sig = RwSignal::new(
        settings.get_untracked().map(|s| s.time_format).unwrap_or_else(|| "24h".to_string()),
    );
    Effect::new(move |prev: Option<String>| {
        let v = time_format_sig.get();
        if let Some(p) = prev {
            if p != v {
                save_field(
                    |s: &mut api::AppSettings| s.time_format = v.clone(),
                    serde_json::json!({ "timeFormat": v }),
                );
            }
        }
        v
    });

    // 背景上传
    let file_input: NodeRef<leptos::html::Input> = NodeRef::new();
    let on_file_change = move |_ev: leptos::ev::Event| {
        let Some(input) = file_input.get() else { return };
        let Some(files) = input.files() else { return };
        let Some(file) = files.get(0) else { return };
        if file.size() > 10.0 * 1024.0 * 1024.0 {
            crate::toast::toast_error("图片大小不能超过 10MB");
            return;
        }
        if !file.type_().starts_with("image/") {
            crate::toast::toast_error("请选择图片文件");
            return;
        }
        spawn_local(async move {
            match api::SystemApi::upload_bg(&file).await {
                Ok(r) => {
                    let url = format!("{}?t={}", r.path, fmt::now_ms() as i64);
                    save_field(
                        |s: &mut api::AppSettings| s.custom_bg_image = url.clone(),
                        serde_json::json!({ "customBgImage": url.clone() }),
                    );
                    crate::toast::toast_success("背景图上传成功");
                }
                Err(e) => crate::toast::toast_error(&format!("上传失败：{e}")),
            }
        });
    };

    view! {
        <div style="display:flex;flex-direction:column;gap:16px">
            // 主题模式
            <div class="card">
                <div class="card-head"><span class="card-title">"主题模式"</span></div>
                <div class="field">
                    <span class="field-label">"外观模式"</span>
                    <div class="segmented">
                        {[("light", "浅色"), ("dark", "深色")]
                            .iter()
                            .map(|(v, label)| {
                                let v = v.to_string();
                                let v1 = v.clone();
                                let v2 = v.clone();
                                view! {
                                    <span
                                        class=move || if mode_str.get() == v1 { "segmented-item active" } else { "segmented-item" }
                                        on:click=move |_| {
                                            let m = if v2 == "light" { AppMode::Light } else { AppMode::Dark };
                                            theme::apply_mode(m);
                                        }
                                    >
                                        {*label}
                                    </span>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            </div>

            // 主题变体
            <div class="card">
                <div class="card-head"><span class="card-title">"主题变体"</span></div>
                <div style="display:flex;flex-wrap:wrap;gap:12px">
                    {AVAILABLE_THEMES
                        .iter()
                        .map(|t| {
                            let t = *t;
                            let (p, s) = t.accents();
                            let label = crate::theme::theme_label(t);
                            view! {
                                <div style="width:120px;cursor:pointer" on:click=move |_| {
                                    theme::apply_theme_variant(t);
                                    crate::toast::toast_success("主题变体已切换");
                                }>
                                    <div style=format!("width:120px;height:80px;border-radius:10px;background:linear-gradient(135deg,{p} 0%,{s} 100%);border:2px solid {};transition:all 0.2s",
                                        if theme.get() == t { "var(--accent)" } else { "var(--border-color)" })>
                                    </div>
                                    <div style=format!("text-align:center;margin-top:6px;font-size:12px;color:{};font-weight:{}",
                                        if theme.get() == t { "var(--accent)" } else { "var(--text-secondary)" },
                                        if theme.get() == t { 600 } else { 400 })>
                                        {label}
                                    </div>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            </div>

            // 显示偏好
            <div class="card">
                <div class="card-head"><span class="card-title">"显示偏好"</span></div>
                <div class="field">
                    <span class="field-label">"默认图表类型"</span>
                    <RadioGroup
                        value=chart_type_sig
                        options=vec![SegmentOpt::new("bar", "柱状图"), SegmentOpt::new("line", "折线图")]
                    />
                </div>
                <div class="field">
                    <span class="field-label">"时间格式"</span>
                    <RadioGroup
                        value=time_format_sig
                        options=vec![SegmentOpt::new("24h", "24 小时制"), SegmentOpt::new("12h", "12 小时制")]
                    />
                </div>
            </div>

            // 自定义背景
            <div class="card">
                <div class="card-head"><span class="card-title">"自定义背景"</span></div>
                {move || {
                    let bg = settings.get().map(|s| s.custom_bg_image).unwrap_or_default();
                    if !bg.is_empty() {
                        view! {
                            <div>
                                <div style="width:240px;height:160px;border-radius:8px;border:1px solid var(--border-color);overflow:hidden">
                                    <img src=bg style="width:100%;height:100%;object-fit:cover" />
                                </div>
                                <button class="btn danger" style="margin-top:12px" on:click=move |_| {
                                    save_field(
                                        |s: &mut api::AppSettings| s.custom_bg_image = String::new(),
                                        serde_json::json!({ "customBgImage": "" }),
                                    );
                                }>"清除背景"</button>
                            </div>
                        }
                            .into_any()
                    } else {
                        view! {
                            <div class="upload-dragger">
                                <p style="font-size:20px;margin:0 0 6px;color:var(--text-muted)">"＋"</p>
                                <p style="margin:0 0 4px">"点击或拖拽图片到此处上传"</p>
                                <p class="hint-text">"支持 jpg / png / webp，单张不超过 10MB"</p>
                                <input
                                    class="text-input"
                                    style="margin-top:10px"
                                    type="file"
                                    accept="image/*"
                                    node_ref=file_input
                                    on:change=on_file_change
                                />
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/* ================= 通用 ================= */

fn render_general(settings: RwSignal<Option<api::AppSettings>>) -> impl IntoView {
    let auto_start = RwSignal::new(settings.get_untracked().map(|s| s.auto_start).unwrap_or(false));
    let auto_backup = RwSignal::new(settings.get_untracked().map(|s| s.auto_backup).unwrap_or(false));
    let backup_count = RwSignal::new(settings.get_untracked().map(|s| s.backup_count).unwrap_or(30));
    let auto_close_idle = RwSignal::new(settings.get_untracked().map(|s| s.auto_close_idle).unwrap_or(false));
    let idle_minutes = RwSignal::new(settings.get_untracked().map(|s| s.idle_close_minutes).unwrap_or(5));
    let widget_enabled = RwSignal::new(settings.get_untracked().map(|s| s.widget_enabled).unwrap_or(false));

    // 数字输入值变化 → 保存
    Effect::new(move |prev: Option<i64>| {
        let v = backup_count.get();
        if let Some(p) = prev {
            if p != v {
                save_field(
                    |s: &mut api::AppSettings| s.backup_count = v,
                    serde_json::json!({ "backupCount": v }),
                );
            }
        }
        v
    });
    Effect::new(move |prev: Option<i64>| {
        let v = idle_minutes.get();
        if let Some(p) = prev {
            if p != v {
                save_field(
                    |s: &mut api::AppSettings| s.idle_close_minutes = v,
                    serde_json::json!({ "idleCloseMinutes": v }),
                );
            }
        }
        v
    });

    let handle_restart = move |_| {
        crate::toast::confirm(
            crate::toast::ConfirmSpec::simple(
                "确认重启服务器？",
                "重启后当前会话将断开，需要重新加载页面。",
                "重启",
                true,
            )
            .with_on_ok(|| {
                spawn_local(async move {
                    match api::SystemApi::restart_server().await {
                        Ok(()) => {
                            crate::toast::toast_success("服务器正在重启，请稍候...");
                            // 3s 后刷新页面
                            let cb = wasm_bindgen::closure::Closure::once_into_js(|| {
                                if let Some(l) = web_sys::window().map(|w| w.location()) {
                                    let _ = l.reload();
                                }
                            });
                            if let Some(w) = web_sys::window() {
                                let _ = w.set_timeout_with_callback(cb.unchecked_ref());
                            }
                        }
                        Err(e) => crate::toast::toast_error(&format!("重启失败：{e}")),
                    }
                });
            }),
        );
    };

    let handle_quit = move |_| {
        spawn_local(async move {
            if let Err(e) = crate::bridge::quit_app().await {
                crate::toast::toast_error(&format!("退出失败：{e}"));
            }
        });
    };

    view! {
        <div style="display:flex;flex-direction:column;gap:16px">
            <div class="card">
                <div class="card-head"><span class="card-title">"启动与备份"</span></div>
                <div class="field">
                    <span class="field-label">
                        "开机自启"
                        <span class="field-hint">
                            {move || {
                                format!("（{}）", if settings.get().map(|s| s.auto_start_registered).unwrap_or(false) { "已注册" } else { "未注册" })
                            }}
                        </span>
                    </span>
                    <Toggle
                        checked=auto_start.read_only()
                        on_change=move |v: bool| {
                            auto_start.set(v);
                            save_field(|s: &mut api::AppSettings| s.auto_start = v, serde_json::json!({ "autoStart": v }));
                        }
                    />
                </div>
                <div class="field">
                    <span class="field-label">"自动备份"</span>
                    <Toggle
                        checked=auto_backup.read_only()
                        on_change=move |v: bool| {
                            auto_backup.set(v);
                            save_field(|s: &mut api::AppSettings| s.auto_backup = v, serde_json::json!({ "autoBackup": v }));
                        }
                    />
                </div>
                <div class="field">
                    <span class="field-label">"备份保留数量"</span>
                    <NumberInput value=backup_count min=1 max=100 disabled=Signal::derive(move || false) />
                </div>
            </div>

            <div class="card">
                <div class="card-head"><span class="card-title">"桌面与小组件"</span></div>
                <div class="field">
                    <span class="field-label">"无活跃会话自动关闭"</span>
                    <Toggle
                        checked=auto_close_idle.read_only()
                        on_change=move |v: bool| {
                            auto_close_idle.set(v);
                            save_field(|s: &mut api::AppSettings| s.auto_close_idle = v, serde_json::json!({ "autoCloseIdle": v }));
                        }
                    />
                </div>
                <div class="field">
                    <span class="field-label">"自动关闭等待（分钟）"</span>
                    <NumberInput value=idle_minutes min=1 max=1440 disabled=Signal::derive(move || !auto_close_idle.get()) />
                </div>
                <div class="field">
                    <span class="field-label">"桌面小组件"</span>
                    <Toggle
                        checked=widget_enabled.read_only()
                        on_change=move |v: bool| {
                            widget_enabled.set(v);
                            save_field(|s: &mut api::AppSettings| s.widget_enabled = v, serde_json::json!({ "widgetEnabled": v }));
                        }
                    />
                </div>
            </div>

            <div class="card danger-zone">
                <div class="card-head">
                    <span class="card-title" style="color:var(--text-danger)">"危险区"</span>
                </div>
                <div style="margin-bottom:22px">
                    <div style="font-weight:600;margin-bottom:6px;color:var(--text-primary)">"重启服务器"</div>
                    <div class="hint-text" style="margin-bottom:12px">"重启后端 HTTP 服务，期间页面会暂时断开。"</div>
                    <button class="btn primary danger" on:click=handle_restart>"重启服务器"</button>
                </div>
                <div>
                    <div style="font-weight:600;margin-bottom:6px;color:var(--text-primary)">"退出应用"</div>
                    <div class="hint-text" style="margin-bottom:12px">"关闭 BootTracker 窗口并退出后台进程。"</div>
                    <button class="btn primary" on:click=handle_quit>"退出应用"</button>
                </div>
            </div>
        </div>
    }
}

/* ================= 网络 ================= */

fn render_network(
    settings: RwSignal<Option<api::AppSettings>>,
    tunnel_url: RwSignal<Option<String>>,
    tunnel_loading: RwSignal<bool>,
    fetch_tunnel: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let lan = RwSignal::new(settings.get_untracked().map(|s| s.lan_access).unwrap_or(false));
    let tunnel_on = RwSignal::new(settings.get_untracked().map(|s| s.tunnel_enabled).unwrap_or(false));
    let token = RwSignal::new(settings.get_untracked().map(|s| s.tunnel_token).unwrap_or_default());
    let domain = RwSignal::new(settings.get_untracked().map(|s| s.custom_domain).unwrap_or_default());

    view! {
        <div style="display:flex;flex-direction:column;gap:16px">
            <div class="card">
                <div class="card-head"><span class="card-title">"局域网访问"</span></div>
                <div class="field">
                    <span class="field-label">"允许局域网设备访问"</span>
                    <Toggle
                        checked=lan.read_only()
                        on_change=move |v: bool| {
                            lan.set(v);
                            save_field(|s: &mut api::AppSettings| s.lan_access = v, serde_json::json!({ "lanAccess": v }));
                        }
                    />
                </div>
            </div>

            <div class="card">
                <div class="card-head"><span class="card-title">"外网隧道（Cloudflare）"</span></div>
                <div class="field">
                    <span class="field-label">"启用外网访问隧道"</span>
                    <Toggle
                        checked=tunnel_on.read_only()
                        on_change=move |v: bool| {
                            tunnel_on.set(v);
                            save_field(|s: &mut api::AppSettings| s.tunnel_enabled = v, serde_json::json!({ "tunnelEnabled": v }));
                            if v {
                                fetch_tunnel();
                            }
                        }
                    />
                </div>
                {move || {
                    if !tunnel_on.get() {
                        return ().into_any();
                    }
                    view! {
                        <div>
                            <div class="field">
                                <span class="field-label">"Cloudflare 隧道令牌"</span>
                                <TextInput value=token placeholder="输入 Cloudflare Zero Trust 隧道令牌" password=true />
                            </div>
                            <div class="field">
                                <span class="field-label">"自定义域名（可选）"</span>
                                <TextInput value=domain placeholder="例如：boottracker.example.com" password=false />
                            </div>
                            <div class="card" style="background:var(--bg-card-light);margin-bottom:16px">
                                <div class="card-head"><span class="card-title" style="font-size:13px">"设置步骤"</span></div>
                                <ol style="margin:0;padding-left:20px;color:var(--text-secondary);font-size:13px;line-height:1.8">
                                    <li>"前往 Cloudflare Zero Trust（one.dash.cloudflare.com）创建账户并登录。"</li>
                                    <li>"在「Networks → Tunnels」中创建一个新 Tunnel，选择「Cloudflared」，复制令牌。"</li>
                                    <li>"将令牌粘贴到上方输入框；如需使用自定义域名，在「Public Hostnames」中添加并填入上方。"</li>
                                </ol>
                            </div>
                            <div class="field">
                                <span class="field-label">"外网访问地址"</span>
                                <div style="display:flex;gap:8px;align-items:center">
                                    <input
                                        class="text-input"
                                        style="flex:1;max-width:none"
                                        readonly=true
                                        placeholder=move || if tunnel_loading.get() { "获取中..." } else { "未启动" }
                                        prop:value=move || tunnel_url.get().unwrap_or_default()
                                    />
                                    <button class="btn" disabled=move || tunnel_url.get().is_none() on:click=move |_| {
                                        if let Some(url) = tunnel_url.get_untracked() {
                                            if let Some(nav) = web_sys::window().map(|w| w.navigator()) {
                                                let _ = nav.clipboard().write_text(&url);
                                            }
                                            crate::toast::toast_success("已复制到剪贴板");
                                        }
                                    }>"复制"</button>
                                </div>
                                <button class="btn link" style="margin-top:4px" on:click=move |_| fetch_tunnel()>
                                    "刷新地址"
                                </button>
                            </div>
                        </div>
                    }
                        .into_any()
                }}
            </div>
        </div>
    }
}

/* ================= 关于 ================= */

#[allow(clippy::too_many_arguments)]
fn render_about(
    version_info: RwSignal<String>,
    app_mode_label: RwSignal<String>,
    version_history: RwSignal<Vec<api::VersionHistoryEntry>>,
    history_loading: RwSignal<bool>,
    bump_notes: RwSignal<String>,
    check_result: RwSignal<Option<api::CheckUpdateResp>>,
    check_loading: RwSignal<bool>,
) -> impl IntoView {
    let do_bump = move |bump_type: &'static str| {
        let notes = bump_notes.get_untracked();
        spawn_local(async move {
            match api::VersionApi::bump(bump_type, &notes).await {
                Ok(r) => {
                    crate::toast::toast_success(&format!("版本已更新：{} → {}", r.previous, r.version));
                    bump_notes.set(String::new());
                    if let Ok(v) = api::VersionApi::get().await {
                        version_info.set(v.version);
                    }
                    if let Ok(mut h) = api::VersionApi::history().await {
                        h.sort_by(|a, b| b.date.cmp(&a.date));
                        version_history.set(h);
                    }
                }
                Err(e) => crate::toast::toast_error(&format!("版本升级失败：{e}")),
            }
        });
    };

    view! {
        <div style="display:flex;flex-direction:column;gap:16px">
            <div class="card">
                <div class="card-head"><span class="card-title">"应用信息"</span></div>
                <div class="stat-row">
                    <span class="hint-text">"当前版本"</span>
                    <span style="display:flex;align-items:center;gap:8px">
                        <span class="tnum mono" style="font-weight:600;color:var(--text-primary)">
                            {move || version_info.get()}
                        </span>
                        <span class="tag blue">{move || app_mode_label.get()}</span>
                    </span>
                </div>
                <div class="stat-row">
                    <span class="hint-text">"运行模式"</span>
                    <span class=move || {
                        if app_mode_label.get() == "开发" { "tag orange" } else { "tag green" }
                    }>
                        {move || app_mode_label.get()}
                    </span>
                </div>
            </div>

            <div class="card">
                <div class="card-head"><span class="card-title">"版本管理"</span></div>
                <div class="field">
                    <span class="field-label">"升级备注（写入版本历史）"</span>
                    <TextArea value=bump_notes placeholder="描述此版本的主要变更..." rows=2 />
                </div>
                <div style="display:flex;gap:10px;flex-wrap:wrap">
                    <button class="btn" on:click=move |_| do_bump("patch")>"补丁版本 (x.x.1)"</button>
                    <button class="btn" on:click=move |_| do_bump("minor")>"次版本 (x.1.0)"</button>
                    <button class="btn primary" on:click=move |_| do_bump("major")>"主版本 (1.0.0)"</button>
                </div>
            </div>

            <div class="card">
                <div class="card-head"><span class="card-title">"版本历史"</span></div>
                {move || {
                    if history_loading.get() {
                        return view! { <div class="tbl-empty"><div class="spin large"></div></div> }.into_any();
                    }
                    let h = version_history.get();
                    if h.is_empty() {
                        return view! { <div class="hint-text">"暂无版本历史记录"</div> }.into_any();
                    }
                    view! {
                        <div>
                            {h.into_iter()
                                .map(|e| {
                                    view! {
                                        <div class="vh-item">
                                            <div class="vh-head">
                                                <span class="tag blue">{format!("v{}", e.version.clone())}</span>
                                                {e.from.as_ref().map(|f| view! { <span class="hint-text">{format!("从 v{}", f)}</span> })}
                                                <span class="vh-date mono">{fmt::fmt_full_time(Some(&e.date))}</span>
                                            </div>
                                            <div style="color:var(--text-secondary);font-size:13px">
                                                {if e.notes.is_empty() { "（无备注）".to_string() } else { e.notes.clone() }}
                                            </div>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }}
            </div>

            <div class="card">
                <div class="card-head"><span class="card-title">"检查更新"</span></div>
                <button class="btn primary" disabled=check_loading.get() on:click=move |_| {
                    spawn_local(async move {
                        check_loading.set(true);
                        check_result.set(None);
                        match api::VersionApi::check_update().await {
                            Ok(r) => {
                                if r.has_update == Some(true) {
                                    crate::toast::toast_success(&format!("发现新版本：{}", r.latest.clone().unwrap_or_default()));
                                } else {
                                    crate::toast::toast_info(r.message.clone().unwrap_or_else(|| "当前已是最新版本".into()).as_str());
                                }
                                check_result.set(Some(r));
                            }
                            Err(e) => crate::toast::toast_error(&format!("检查更新失败：{e}")),
                        }
                        check_loading.set(false);
                    });
                }>"检查更新"</button>
                {move || {
                    check_result.get().map(|r| {
                        view! {
                            <div class="card" style="margin-top:12px;background:var(--bg-card-light)">
                                <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap">
                                    {if r.has_update == Some(true) {
                                        view! { <span class="tag green">"有新版本"</span> }.into_any()
                                    } else {
                                        view! { <span class="tag">"已是最新"</span> }.into_any()
                                    }}
                                    {r.latest.map(|l| view! { <span class="tnum mono" style="font-weight:600">{format!("v{l}")}</span> })}
                                </div>
                                {r.message.map(|m| view! { <div class="hint-text" style="margin-top:6px">{m}</div> })}
                                {r.download_url.map(|u| view! {
                                    <a class="btn link" href=u target="_blank" rel="noopener noreferrer">"前往下载"</a>
                                })}
                            </div>
                        }
                    })
                }}
            </div>
        </div>
    }
}
