//! 主题单一事实源（与网页端 zustand persist 存储格式互通）
//!
//! - mode（dark/light）：同步后端 settings.appMode + 原生桥同步 DWM 标题栏
//! - theme（purple/blue/...）：仅前端持久化

use crate::bridge;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppMode {
    Dark,
    Light,
}

impl AppMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppMode::Dark => "dark",
            AppMode::Light => "light",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeName {
    Purple,
    Blue,
    Green,
    Orange,
    Gray,
    Mica,
    MaterialYou,
}

impl ThemeName {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeName::Purple => "purple",
            ThemeName::Blue => "blue",
            ThemeName::Green => "green",
            ThemeName::Orange => "orange",
            ThemeName::Gray => "gray",
            ThemeName::Mica => "mica",
            ThemeName::MaterialYou => "material-you",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "blue" => ThemeName::Blue,
            "green" => ThemeName::Green,
            "orange" => ThemeName::Orange,
            "gray" => ThemeName::Gray,
            "material-you" => ThemeName::MaterialYou,
            // 存量 purple 随新默认迁移到 mica（与网页端迁移规则一致）
            _ => ThemeName::Mica,
        }
    }
    /// 主题变体强调色（与 antd-theme.ts THEME_ACCENTS 对齐）
    pub fn accents(&self) -> (&'static str, &'static str) {
        match self {
            ThemeName::Purple => ("#8b5cf6", "#d946ef"),
            ThemeName::Blue => ("#3b82f6", "#06b6d4"),
            ThemeName::Green => ("#10b981", "#34d399"),
            ThemeName::Orange => ("#f97316", "#f59e0b"),
            ThemeName::Gray => ("#64748b", "#94a3b8"),
            ThemeName::Mica => ("#fafafa", "#c4c4cc"),
            ThemeName::MaterialYou => ("#d0bcff", "#b69df8"),
        }
    }
}

pub const AVAILABLE_THEMES: [ThemeName; 7] = [
    ThemeName::Purple,
    ThemeName::Blue,
    ThemeName::Green,
    ThemeName::Orange,
    ThemeName::Gray,
    ThemeName::Mica,
    ThemeName::MaterialYou,
];

pub fn theme_label(t: ThemeName) -> &'static str {
    match t {
        ThemeName::Purple => "紫色",
        ThemeName::Blue => "蓝色",
        ThemeName::Green => "绿色",
        ThemeName::Orange => "橙色",
        ThemeName::Gray => "灰色",
        ThemeName::Mica => "云母",
        ThemeName::MaterialYou => "Material You",
    }
}

struct ThemeStore {
    mode: RwSignal<AppMode>,
    theme: RwSignal<ThemeName>,
}

static THEME: OnceLock<ThemeStore> = OnceLock::new();

fn store() -> &'static ThemeStore {
    THEME.get_or_init(|| {
        // 读 zustand persist 格式：{"state":{"mode":"dark","theme":"mica"},"version":0}
        let (mode, theme) = (|| {
            let ls = web_sys::window()?.local_storage().ok()??;
            let raw = ls.get_item("boottracker-theme").ok()??;
            let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
            let state = v.get("state")?;
            let mode = state
                .get("mode")?
                .as_str()?
                .contains("light")
                .then_some(AppMode::Light)
                .unwrap_or(AppMode::Dark);
            let theme = state
                .get("theme")?
                .as_str()
                .map(ThemeName::from_str)
                .unwrap_or(ThemeName::Mica);
            Some((mode, theme))
        })()
        .unwrap_or((AppMode::Dark, ThemeName::Mica));

        let s = ThemeStore {
            mode: RwSignal::new(mode),
            theme: RwSignal::new(theme),
        };
        apply_dom(mode, theme);
        s
    })
}

pub fn mode() -> RwSignal<AppMode> {
    store().mode
}

pub fn theme() -> RwSignal<ThemeName> {
    store().theme
}

fn ls_set(key: &str, value: &str) {
    if let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = ls.set_item(key, value);
    }
}

/// 应用到 DOM：data-theme + body.light-mode + 原生环境标记
pub fn apply_dom(mode: AppMode, theme: ThemeName) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(html) = doc.document_element() {
        let _ = html.set_attribute("data-theme", theme.as_str());
        if bridge::is_tauri() {
            let _ = html.set_attribute("data-native", "tauri");
        }
    }
    if let Some(body) = doc.body() {
        if mode == AppMode::Light {
            let _ = body.class_list().add_1("light-mode");
        } else {
            let _ = body.class_list().remove_1("light-mode");
        }
    }
}

fn persist(mode: AppMode, theme: ThemeName) {
    let json = serde_json::json!({
        "state": { "mode": mode.as_str(), "theme": theme.as_str() },
        "version": 0
    });
    ls_set("boottracker-theme", &json.to_string());
}

/// 切换深浅模式：DOM + 持久化 + 后端 appMode + 原生标题栏
pub fn apply_mode(new_mode: AppMode) {
    let theme = store().theme.get_untracked();
    store().mode.set(new_mode);
    apply_dom(new_mode, theme);
    persist(new_mode, theme);
    spawn_local(async move {
        let _ = crate::api::SettingsApi::update_partial(
            serde_json::json!({ "appMode": new_mode.as_str() }),
        )
        .await;
        let _ = bridge::set_theme(new_mode.as_str()).await;
    });
}

/// 切换主题变体：仅前端持久化 + 同步后端 appTheme（供小组件取色）
pub fn apply_theme_variant(new_theme: ThemeName) {
    let mode = store().mode.get_untracked();
    store().theme.set(new_theme);
    apply_dom(mode, new_theme);
    persist(mode, new_theme);
    spawn_local(async move {
        let _ = crate::api::SettingsApi::update_partial(
            serde_json::json!({ "appTheme": new_theme.as_str() }),
        )
        .await;
    });
}

pub fn toggle_mode() {
    let cur = store().mode.get_untracked();
    apply_mode(if cur == AppMode::Dark {
        AppMode::Light
    } else {
        AppMode::Dark
    });
}
