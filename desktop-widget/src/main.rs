//! BootTracker 纯原生桌面浮窗组件（egui/eframe，无 WebView）
//!
//! 替代原 Tauri 小组件：无边框、置顶、半透明、可拖动，每 30 秒拉取一次数据。
//! 右键菜单：打开主界面 / 隐藏组件。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// 数据模型刻意保留后端返回字段（部分暂未使用），不必报死代码告警
#![allow(dead_code)]

use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use egui::{Color32, Context, Pos2, RichText, Rounding, Sense, Vec2, ViewportCommand};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BootSession {
    id: String,
    boot_time: String,
    shutdown_time: Option<String>,
    duration: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataResponse {
    sessions: Vec<BootSession>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    #[serde(default)]
    widget_enabled: bool,
    #[serde(default)]
    app_mode: String,
    #[serde(default)]
    widget_position: String,
    #[serde(default)]
    app_theme: String,
}

enum WMsg {
    Data(Result<DataResponse, String>),
    Settings(Result<AppSettings, String>),
    Action(Result<(), String>),
}

struct WidgetApp {
    base: String,
    tx: Sender<WMsg>,
    rx: Receiver<WMsg>,
    sessions: Vec<BootSession>,
    dark: bool,
    theme: String,
    pos: Option<(f32, f32)>,
    last_fetch: f64,
    last_settings: f64,
    dragging: bool,
}

fn fetch(base: &str, path: &str) -> Result<String, String> {
    ureq::get(&format!("{base}{path}"))
        .timeout(Duration::from_secs(8))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn fetch_json(base: &str, path: &str, body: &serde_json::Value) -> Result<String, String> {
    ureq::post(&format!("{base}{path}"))
        .timeout(Duration::from_secs(8))
        .send_json(body.clone())
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

/// 更新设置：后端 `/api/settings` 的写入方法是 PUT，不是 POST
fn put_json(base: &str, path: &str, body: &serde_json::Value) -> Result<String, String> {
    ureq::put(&format!("{base}{path}"))
        .timeout(Duration::from_secs(8))
        .send_json(body.clone())
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn parse<T: serde::de::DeserializeOwned>(s: Result<String, String>) -> Result<T, String> {
    match s {
        Ok(x) => serde_json::from_str::<T>(&x).map_err(|e| e.to_string()),
        Err(e) => Err(e),
    }
}

impl WidgetApp {
    fn new() -> Self {
        let port = std::env::var("BOOTTRACKER_PORT").unwrap_or_else(|_| "18792".into());
        let base = format!("http://127.0.0.1:{port}");
        let (tx, rx) = channel::<WMsg>();
        let app = WidgetApp {
            base,
            tx,
            rx,
            sessions: Vec::new(),
            dark: true,
            theme: "blue".into(),
            pos: None,
            last_fetch: -100.0,
            last_settings: -100.0,
            dragging: false,
        };
        app.spawn_settings();
        app.spawn_data();
        app
    }

    fn spawn_data(&self) {
        let base = self.base.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(WMsg::Data(parse::<DataResponse>(fetch(&base, "/api/data"))));
        });
    }

    fn spawn_settings(&self) {
        let base = self.base.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(WMsg::Settings(parse::<AppSettings>(fetch(
                &base, "/api/settings",
            ))));
        });
    }

    fn raise_main(&self) {
        let base = self.base.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(WMsg::Action(fetch_json(
                &base,
                "/api/raise-window",
                &serde_json::json!({}),
            )
            .map(|_| ())));
        });
    }

    fn hide_self(&self) {
        let base = self.base.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(WMsg::Action(put_json(
                &base,
                "/api/settings",
                &serde_json::json!({ "widgetEnabled": false }),
            )
            .map(|_| ())));
        });
    }

    fn save_position(&self) {
        if let Some((x, y)) = self.pos {
            let base = self.base.clone();
            let body = serde_json::json!({ "widgetPosition": format!("{},{}", x as i32, y as i32) });
            std::thread::spawn(move || {
                let _ = put_json(&base, "/api/settings", &body);
            });
        }
    }
}

impl eframe::App for WidgetApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // 处理异步消息
        while let Ok(m) = self.rx.try_recv() {
            match m {
                WMsg::Data(r) => {
                    if let Ok(d) = r {
                        self.sessions = d.sessions;
                    }
                }
                WMsg::Settings(r) => {
                    if let Ok(s) = r {
                        self.dark = s.app_mode != "light";
                if !s.app_theme.is_empty() {
                    self.theme = s.app_theme.clone();
                }
                        if let Some((x, y)) = s
                            .widget_position
                            .split_once(',')
                            .and_then(|(a, b)| Some((a.parse::<f32>().ok()?, b.parse::<f32>().ok()?)))
                        {
                            if self.pos.is_none() {
                                self.pos = Some((x, y));
                                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(x, y)));
                            }
                        }
                    }
                }
                WMsg::Action(_) => {}
            }
        }

        // 周期刷新
        let t = ctx.input(|i| i.time);
        if t - self.last_fetch > 30.0 {
            self.last_fetch = t;
            self.spawn_data();
        }
        if t - self.last_settings > 60.0 {
            self.last_settings = t;
            self.spawn_settings();
        }

        // 透明主题：浮窗背景透明，但 hyperlink/selection 跟随 accent
        let accent = accent_for(&self.theme);
        let mut v = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        v.window_fill = Color32::TRANSPARENT;
        v.panel_fill = Color32::TRANSPARENT;
        v.extreme_bg_color = Color32::TRANSPARENT;
        v.hyperlink_color = accent;
        v.selection.bg_fill = accent;
        ctx.set_visuals(v);

        ctx.request_repaint_after(Duration::from_secs(1));

        // 卡片底色对齐 web var(--bg-card)：dark (28,28,32)，light 白；带 alpha 做半透明
        let card_fill = if self.dark {
            Color32::from_rgba_premultiplied(28, 28, 32, 220)
        } else {
            Color32::from_rgba_premultiplied(255, 255, 255, 235)
        };
        // 文字对齐 web var(--text-primary)
        let text_color = if self.dark {
            Color32::from_rgb(250, 250, 250)
        } else {
            Color32::from_rgb(30, 41, 59)
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            let avail = ui.available_size();
            let rect = egui::Rect::from_min_size(Pos2::ZERO, avail);
            let resp = ui.interact(rect, egui::Id::new("card"), Sense::drag());
            ui.painter().rect_filled(
                rect,
                Rounding::same(12.0),
                card_fill,
            );
            ui.painter().rect_stroke(
                rect,
                Rounding::same(12.0),
                egui::Stroke::new(1.0_f32, accent),
            );

            // 拖动
            if resp.dragged() {
                self.dragging = true;
                let d = resp.drag_delta();
                let cur = self.pos.unwrap_or((0.0, 0.0));
                let np = (cur.0 + d.x, cur.1 + d.y);
                self.pos = Some(np);
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(np.0, np.1)));
            } else if self.dragging && resp.drag_stopped() {
                self.dragging = false;
                self.save_position();
            }

            // 内容
            let now = chrono::Utc::now();
            let active = self
                .sessions
                .iter()
                .filter(|s| s.shutdown_time.is_none())
                .max_by_key(|s| s.boot_time.clone());
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!(
                        "{}  本次开机",
                        egui_phosphor::regular::POWER
                    ))
                    .color(text_color)
                    .weak(),
                );
                match active {
                    Some(s) => {
                        if let Some(bt) = chrono::DateTime::parse_from_rfc3339(&s.boot_time)
                            .ok()
                            .map(|d| d.with_timezone(&chrono::Utc))
                        {
                            let dur = (now - bt).num_milliseconds();
                            // 等宽数字：秒数变化时宽度不跳，浮窗不会左右抖
                            ui.label(
                                RichText::new(crate::fmt_duration(dur))
                                    .font(egui::FontId::new(22.0, egui::FontFamily::Monospace))
                                    .color(text_color)
                                    .strong(),
                            );
                        }
                    }
                    None => {
                        ui.label(
                            RichText::new(format!(
                                "{}  无进行中会话",
                                egui_phosphor::regular::MOON
                            ))
                            .color(text_color),
                        );
                    }
                }
                ui.add_space(6.0);
            });

            // 右键菜单
            resp.context_menu(|ui| {
                if ui
                    .button(format!(
                        "{}  打开主界面",
                        egui_phosphor::regular::ARROW_UP_RIGHT
                    ))
                    .clicked()
                {
                    self.raise_main();
                    ui.close_menu();
                }
                if ui
                    .button(format!("{}  隐藏组件", egui_phosphor::regular::X))
                    .clicked()
                {
                    self.hide_self();
                    ui.close_menu();
                }
            });
        });
    }
}

fn fmt_duration(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}时{m}分{s}秒")
    } else if m > 0 {
        format!("{m}分{s}秒")
    } else {
        format!("{s}秒")
    }
}

/// 各 accent 主题主色，取自 web tokens.css [data-theme="..."] --accent。
/// desktop-widget 浮窗用它统一描边/hyperlink，与 web 端主题切换联动。
fn accent_for(theme: &str) -> Color32 {
    match theme {
        "purple" => Color32::from_rgb(139, 92, 246),
        "green" => Color32::from_rgb(16, 185, 129),
        "orange" => Color32::from_rgb(249, 115, 22),
        "gray" => Color32::from_rgb(100, 116, 139),
        "mica" => Color32::from_rgb(250, 250, 250),
        "material-you" => Color32::from_rgb(208, 188, 255),
        // blue 为默认
        _ => Color32::from_rgb(59, 130, 246),
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("开机记录组件")
            .with_inner_size(Vec2::new(240.0, 96.0))
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "开机记录组件",
        options,
        Box::new(|cc| {
            register_cjk_font(&cc.egui_ctx);
            Ok(Box::new(WidgetApp::new()))
        }),
    )
}

/// 注册系统中文字体，否则 egui 默认字体不含 CJK，中文显示为方格。
/// 优先单 TTF（ab_glyph 兼容性最好），.ttc 作为兜底。
fn register_cjk_font(ctx: &Context) {
    const CANDIDATES: &[&str] = &[
        "C:\\Windows\\Fonts\\simhei.ttf",
        "C:\\Windows\\Fonts\\Deng.ttf",
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\simsun.ttc",
    ];
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cjk".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk".to_owned());
            // Phosphor 必须提到第 0 位（SimHei 在 PUA 区有自己的字形，会顶掉图标）
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                if let Some(pos) = fam.iter().position(|k| k == "phosphor") {
                    let key = fam.remove(pos);
                    fam.insert(0, key);
                }
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
    eprintln!("[boot-tracker-widget] 未找到系统中文字体，中文将显示为方格");
}
