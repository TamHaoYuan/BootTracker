//! BootTracker 纯原生桌面前端入口（egui/eframe，无 WebView）

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// 数据模型刻意保留后端 REST API 的完整字段（部分暂未使用），不必报死代码告警
#![allow(dead_code)]

mod anim;
mod api;
mod app;
mod fmt;
mod icon;
mod pages;
mod theme;
mod tray;

use eframe::NativeOptions;
use egui::{Vec2, ViewportBuilder};

fn main() -> eframe::Result<()> {
    let icon = icon::egui_icon();
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("开机记录")
            .with_inner_size(Vec2::new(1000.0, 720.0))
            .with_min_inner_size(Vec2::new(720.0, 520.0))
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        "开机记录",
        options,
        Box::new(|cc| {
            register_cjk_font(&cc.egui_ctx);
            Ok(Box::new(app::BootTrackerApp::new(cc)))
        }),
    )
}

/// 注册系统中文字体。
///
/// egui 默认字体只含拉丁字形，不含 CJK，中文会全部显示为方格（tofu）。
/// 从 Windows 系统字体目录加载一个中文字体并注册为首选，解决中文渲染。
/// 优先单 TTF（ab_glyph 兼容性最好），.ttc 作为兜底。
fn register_cjk_font(ctx: &egui::Context) {
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
            // 成熟图标库：Phosphor Icons。注意 add_to_fonts 固定把 phosphor 插到
            // Proportional 族第 1 位——而我们的 cjk 在第 0 位，SimHei 在 PUA 区
            // 有自己的字形（实心块/圆点等），会把图标全部顶掉。所以插入后必须把
            // phosphor 提到第 0 位：图标码位命中 phosphor；中文/拉丁 phosphor
            // 没有（cmap 里是 gid 0）→ 正确回落 cjk。
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
    eprintln!("[boot-tracker] 未找到系统中文字体，中文将显示为方格");
}
