//! 视觉主题——对齐 web UI (`frontend/src/styles/tokens.css`) 的 Dark Gallery 风格。
//!
//! egui 即时模式无法复刻 CSS 的玻璃态/高斯模糊，这里用不透明色近似 web 的半透明
//! 卡片底，保证桌面端与 Web 端在背景、文字、主色、强调态上观感一致。
//! 配色 token 全部取自 tokens.css（dark + 各 data-theme accent）。

use egui::style::{Spacing, WidgetVisuals};
use egui::{
    Color32, FontFamily, FontId, Margin, Rounding, Stroke, Style, TextStyle, Vec2, Visuals,
};

/// 一套配色 token，对应 web 的 CSS 变量集合
#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    /// 主强调色 — var(--accent)
    pub accent: Color32,
    /// 次强调色 — var(--accent2)，图表第二色
    pub accent2: Color32,
    /// 页面/面板背景 — var(--bg-page)
    pub bg_page: Color32,
    /// 卡片背景 — var(--bg-card)
    pub bg_card: Color32,
    /// 主文字 — var(--text-primary)
    pub text_primary: Color32,
    /// 次文字 — var(--text-secondary)
    pub text_secondary: Color32,
    /// 弱文字 — var(--text-muted)
    pub text_muted: Color32,
    /// 边框 — var(--border-color)
    pub border: Color32,
    /// 成功色 — var(--text-success-dark) #10b981
    pub success: Color32,
    /// 危险色 — var(--text-danger-dark)
    pub danger: Color32,
    /// 禁用 / 空状态图标 — var(--text-disabled)
    pub text_disabled: Color32,
    /// 细边框 — var(--border-light)，用于卡片描边与行分隔
    pub border_light: Color32,
    /// 卡片顶部内高光 — 近似 CSS `inset 0 1px 0 var(--glass-light)`
    /// （egui 无 backdrop-filter，靠这根亮线做出玻璃质感）
    pub glass_light: Color32,
}

/// 圆角层级：小元素小圆角、大容器大圆角（对齐 web：输入 10 / 卡片 12 / 容器 16 / 弹窗 20）
pub const R_INPUT: f32 = 10.0;
pub const R_CARD: f32 = 12.0;
pub const R_PANEL: f32 = 16.0;
pub const R_MODAL: f32 = 20.0;

/// 根据 (dark, theme_name) 构建配色。
/// `theme` 对应 web `data-theme`：blue/purple/green/orange/gray/mica/material-you。
pub fn palette(dark: bool, theme: &str) -> Palette {
    // 各主题 accent 取自 tokens.css [data-theme="..."]
    let (accent, accent2) = match theme {
        "purple" => (rgb(139, 92, 246), rgb(217, 70, 239)),
        "green" => (rgb(16, 185, 129), rgb(52, 211, 153)),
        "orange" => (rgb(249, 115, 22), rgb(245, 158, 11)),
        "gray" => (rgb(100, 116, 139), rgb(148, 163, 184)),
        "mica" => (rgb(250, 250, 250), rgb(196, 196, 204)),
        "material-you" => (rgb(208, 188, 255), rgb(182, 157, 248)),
        // blue 为默认主题
        _ => (rgb(59, 130, 246), rgb(6, 182, 212)),
    };
    if dark {
        Palette {
            dark: true,
            accent,
            accent2,
            // blue 主题页面底 #080a10；其它主题回落到默认 #0a0a0c
            bg_page: match theme {
                "blue" => rgb(8, 10, 16),
                _ => rgb(10, 10, 12),
            },
            bg_card: rgb(28, 28, 32),
            text_primary: rgb(250, 250, 250),
            text_secondary: rgb(196, 196, 204),
            text_muted: rgb(139, 139, 148),
            border: rgb(40, 40, 46),
            success: rgb(16, 185, 129),
            danger: rgb(239, 68, 68),
            text_disabled: rgb(82, 82, 91),
            border_light: rgb(43, 43, 48),
            glass_light: rgb(48, 48, 54),
        }
    } else {
        Palette {
            dark: false,
            accent,
            accent2,
            bg_page: rgb(248, 250, 252),
            bg_card: rgb(255, 255, 255),
            text_primary: rgb(30, 41, 59),
            text_secondary: rgb(71, 85, 105),
            text_muted: rgb(148, 163, 184),
            border: rgb(235, 238, 242),
            success: rgb(16, 185, 129),
            danger: rgb(220, 38, 38),
            text_disabled: rgb(203, 213, 225),
            border_light: rgb(240, 242, 245),
            glass_light: rgb(255, 255, 255),
        }
    }
}

/// 基于 Palette 构建 egui Visuals，使桌面端观感对齐 web Dark Gallery。
pub fn visuals(p: &Palette) -> Visuals {
    let mut v = if p.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.dark_mode = p.dark;
    // 背景层级：面板/窗口 = 页面底；极端背景更深一档（输入框等）
    v.panel_fill = p.bg_page;
    v.window_fill = p.bg_page;
    v.extreme_bg_color = if p.dark {
        rgb(6, 8, 16)
    } else {
        rgb(226, 232, 240)
    };
    v.override_text_color = Some(p.text_primary);
    v.hyperlink_color = p.accent;
    // 弹窗 / Toast 用容器级圆角（20）——方案里的 R_MODAL，别跟控件圆角混用
    v.window_rounding = Rounding::same(R_MODAL);
    // 控件圆角用输入级（10），容器级圆角由 pages 侧按需用 R_CARD/R_PANEL/R_MODAL
    let r = Rounding::same(R_INPUT);
    // 选中态：低透明 accent 底 + accent 文字/图标（避免整块实色过艳）
    v.selection.bg_fill = accent_tint(p.accent, 38);
    v.selection.stroke = Stroke::new(1.0_f32, p.accent);
    // 控件各态：hover/active/selected 用 accent 强调，其余克制
    let on_accent = Stroke::new(1.0_f32, p.text_primary);
    let accent_stroke = Stroke::new(1.0_f32, p.accent);
    v.widgets.noninteractive = WidgetVisuals {
        bg_fill: p.bg_card,
        weak_bg_fill: p.bg_card,
        bg_stroke: Stroke::new(1.0_f32, p.border),
        fg_stroke: Stroke::new(1.0_f32, p.text_secondary),
        rounding: r,
        expansion: 0.0,
    };
    v.widgets.inactive = WidgetVisuals {
        bg_fill: p.bg_card,
        weak_bg_fill: p.bg_card,
        bg_stroke: Stroke::new(1.0_f32, p.border),
        fg_stroke: Stroke::new(1.0_f32, p.text_primary),
        rounding: r,
        expansion: 0.0,
    };
    v.widgets.hovered = WidgetVisuals {
        bg_fill: accent_tint(p.accent, 26),
        weak_bg_fill: accent_tint(p.accent, 26),
        bg_stroke: accent_stroke,
        fg_stroke: on_accent,
        rounding: r,
        expansion: 1.0,
    };
    v.widgets.active = WidgetVisuals {
        bg_fill: p.accent,
        weak_bg_fill: p.accent,
        bg_stroke: accent_stroke,
        fg_stroke: on_accent,
        rounding: r,
        expansion: 1.0,
    };
    v
}

/// 构建全局 Style：在 web 配色（visuals）之上，放大基础字号、间距与控件点击区，
/// 解决 egui 默认控件偏小的问题。控件尺寸（checkbox/slider/button）主要受
/// `interact_size` 与 `button_padding` 控制，这里整体调大一号。
pub fn style(p: &Palette) -> Style {
    Style {
        // 基础字号：14 → 16，按钮/标题同步放大，让整体观感更舒展
        text_styles: [
            (
                TextStyle::Small,
                FontId::new(13.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(15.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Heading,
                FontId::new(24.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(14.0, FontFamily::Monospace),
            ),
        ]
        .into(),
        spacing: Spacing {
            item_spacing: Vec2::new(10.0, 8.0),
            window_margin: Margin::symmetric(16.0, 16.0),
            button_padding: Vec2::new(14.0, 9.0),
            // 滑块 / 勾选框 / 下拉的命中区加大，更跟手也更好看
            interact_size: Vec2::new(48.0, 28.0),
            icon_width: 18.0,
            icon_spacing: 6.0,
            combo_height: 30.0,
            indent: 18.0,
            menu_margin: Margin::symmetric(8.0, 6.0),
            ..Default::default()
        },
        visuals: visuals(p),
        ..Default::default()
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// 由基色生成低透明度变体（用于选中/悬停底，避免实色 accent 过艳）。
fn accent_tint(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
