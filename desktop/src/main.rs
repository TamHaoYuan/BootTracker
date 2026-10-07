//! BootTracker 纯原生桌面前端入口（egui/eframe，无 WebView）

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// 数据模型刻意保留后端 REST API 的完整字段（部分暂未使用），不必报死代码告警
#![allow(dead_code)]

mod anim;
mod api;
mod app;
mod chart;
mod components;
mod fmt;
mod i18n;
mod icon;
mod pages;
mod theme;
mod tray;

use eframe::NativeOptions;
use egui::{Vec2, ViewportBuilder};

fn main() -> eframe::Result<()> {
    let icon = icon::egui_icon();
    // 窗口标题在启动时按 settings.language 取一次。运行中切换语言会立即更新界面文案与
    // 窗口标题（app.rs 里发 ViewportCommand::Title），但托盘菜单文案由 OS 持有，需重启。
    let lang = app::BootTrackerApp::initial_lang();
    i18n::set_lang(lang);
    let title = i18n::tr("开机记录");
    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(Vec2::new(1000.0, 720.0))
            .with_min_inner_size(Vec2::new(720.0, 520.0))
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(
        title,
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
            // 成熟图标库：Phosphor Icons，**必须放在 `cjk` 之后**。
            //
            // 这里有个很隐蔽的坑（实测：英文界面里 "Dashboard" 被压成一个字母）：
            // egui 在字族里挑字体用的是「第一个声明自己有这些字形」的字体，而不是
            // 「第一个真正有非零宽度字形」的字体（`egui::text::fonts::Font::font` 里
            // `has_glyphs` 命中即返回）。`egui_phosphor::add_to_fonts` 会把 phosphor
            // 插到 Proportional 族**第 0 位**，而 phosphor 的 cmap 对拉丁字母也自称
            // 有字形，于是拉丁文本全部落到图标字体上，每个字母量出来的宽度接近 0 ——
            // 表现为整串文字被压成一两个字母，数值（Monospace 族）和中文（cjk 在
            // 第 0 位）反而正常。所以这里只在末尾补一次 phosphor，绝不往前提。
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            ctx.set_fonts(fonts);
            return;
        }
    }
    eprintln!("[boot-tracker] 未找到系统中文字体，中文将显示为方格");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 英文界面出现过一个很隐蔽的 bug：文案在界面上被压成一两个字母（"Dashboard"
    /// 只剩 "D"），而数值（Monospace 族）和中文正常。
    ///
    /// 根因：`egui_phosphor::add_to_fonts` 把 phosphor 插到 Proportional 族**第 0 位**，
    /// 而 phosphor 的 cmap 对拉丁字母也自称有字形；egui 挑字体用的是「第一个声明拥有
    /// 这些字形的字体」而不是「第一个有非零宽度的字体」（`egui::text::fonts::Font::font`
    /// 里 `has_glyphs` 命中即返回），于是拉丁文本全落到图标字体上、每个字母宽度接近 0。
    ///
    /// 这条测试把「拉丁与中文都必须占到正常宽度」钉住：谁再往字族前面（或 cjk 前面）
    /// 插装饰性字体，宽度就会塌下去。
    #[test]
    fn latin_and_cjk_keep_their_width_after_font_registration() {
        fn width_of(ctx: &egui::Context, text: &str) -> f32 {
            ctx.fonts(|f| {
                f.layout_no_wrap(
                    text.to_owned(),
                    egui::FontId::new(16.0, egui::FontFamily::Proportional),
                    egui::Color32::WHITE,
                )
            })
            .size()
            .x
        }

        let ctx = egui::Context::default();
        register_cjk_font(&ctx);
        // `ctx.fonts()` 在 pass 之外会 panic（context.rs:1011），先跑一个空 pass 把
        // 字体系统初始化出来。
        let _ = ctx.run(egui::RawInput::default(), |_| {});

        // 9 个拉丁字符在 16px 下正常约 72px（被图标字体接管时约 8px）
        let latin = width_of(&ctx, "Dashboard");
        assert!(
            latin > 45.0,
            "拉丁文字被挤压，宽度只有 {latin}px —— 检查字体族的顺序（phosphor 必须在 cjk 之后）"
        );
        // 中文必须真的占到宽度（cjk 没加载成功时会是 0）
        let cjk = width_of(&ctx, "仪表盘");
        assert!(cjk > 30.0, "中文字宽只有 {cjk}px");
    }
}
