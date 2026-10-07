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
    /// 外壳（侧栏 / 顶栏）背景 — 与卡片同源，web 里 Sider/Header 即 var(--bg-card)
    pub bg_bar: Color32,
    /// 卡片悬停底 — var(--bg-card-hover)
    pub bg_card_hover: Color32,
    /// 导航项悬停底 — rgba(accent, 0.06)
    pub nav_hover: Color32,
    /// 导航项选中底 — rgba(accent, 0.15)
    pub nav_selected: Color32,
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
    /// 弹窗/浮层背景（比页面底亮一档，暗色下才有"浮起来"的层次）
    pub window_bg: Color32,
}

/// 圆角层级：小元素小圆角、大容器大圆角（对齐 web：输入 10 / 卡片 12 / 容器 16 / 弹窗 20）
pub const R_INPUT: f32 = 10.0;
pub const R_CARD: f32 = 12.0;
pub const R_PANEL: f32 = 16.0;
pub const R_MODAL: f32 = 20.0;

/// 导航项布局：web `.ant-menu-item` 高 40、图标 18、文字 16
pub const NAV_H: f32 = 40.0;
pub const NAV_ICON: f32 = 18.0;
pub const NAV_TEXT: f32 = 16.0;
/// 顶栏高度（web Header 56）
pub const TOPBAR_H: f32 = 56.0;
/// 侧栏展开 / 折叠宽度
pub const SIDEBAR_W: f32 = 208.0;
pub const SIDEBAR_W_COLLAPSED: f32 = 64.0;

/// 根据 (dark, theme_name) 构建配色。
/// `theme` 对应 web `data-theme`：blue/purple/green/orange/gray/mica/material-you。
pub fn palette(dark: bool, theme: &str) -> Palette {
    // 各主题 accent 取自 tokens.css [data-theme="..."]。
    //
    // 关键：web 端 `[data-theme="mica"] { --accent: var(--text-primary) }`、
    // `[data-theme="gray"] { --accent: var(--text-muted) }` 这两个 accent 是
    // **跟随明暗模式**的（浅色下 text-primary 是近黑 #1e293b）。桌面端早期把
    // mica 写死成 rgb(250,250,250)，浅色模式下就会拿近白色当强调色——导航选中项、
    // 按钮 hover/active、超链接全变成白底白字，也就是「亮色模式异常」。
    // 这里按 dark 归一化成实际该用的强调色。
    let (accent, accent2) = match theme {
        "purple" => (rgb(139, 92, 246), rgb(217, 70, 239)),
        "green" => (rgb(16, 185, 129), rgb(52, 211, 153)),
        "orange" => (rgb(249, 115, 22), rgb(245, 158, 11)),
        "gray" => {
            if dark {
                (rgb(100, 116, 139), rgb(148, 163, 184))
            } else {
                (rgb(71, 85, 105), rgb(148, 163, 184))
            }
        }
        "mica" => {
            if dark {
                (rgb(250, 250, 250), rgb(196, 196, 204))
            } else {
                (rgb(30, 41, 59), rgb(100, 116, 139))
            }
        }
        "material-you" => (rgb(208, 188, 255), rgb(182, 157, 248)),
        // 与 web 一致：无 data-theme 时用 tokens.css 的默认紫 #8b5cf6
        _ => (rgb(139, 92, 246), rgb(6, 182, 212)),
    };
    if dark {
        let card = rgb(28, 28, 32);
        Palette {
            dark: true,
            accent,
            accent2,
            // blue 主题页面底 #080a10；其它主题回落到默认 #0a0a0c
            bg_page: match theme {
                "blue" => rgb(8, 10, 16),
                _ => rgb(10, 10, 12),
            },
            bg_card: card,
            // web 里 Sider/Header 与卡片同色，页面底更深 → 形成外壳层级
            bg_bar: card,
            bg_card_hover: rgb(48, 48, 54),
            nav_hover: accent_tint(accent, 15),
            nav_selected: accent_tint(accent, 38),
            text_primary: rgb(250, 250, 250),
            text_secondary: rgb(196, 196, 204),
            text_muted: rgb(139, 139, 148),
            border: rgb(40, 40, 46),
            success: rgb(16, 185, 129),
            danger: rgb(239, 68, 68),
            text_disabled: rgb(82, 82, 91),
            border_light: rgb(43, 43, 48),
            glass_light: rgb(48, 48, 54),
            window_bg: rgb(26, 26, 31),
        }
    } else {
        Palette {
            dark: false,
            accent,
            accent2,
            bg_page: rgb(238, 242, 247),
            bg_card: rgb(255, 255, 255),
            // 浅色下页面底已经很浅，外壳若也用纯白，侧栏/顶栏与内容区完全没有边界
            // （实测截图上侧栏几乎不可见）。web 端 Sider 是 rgba(255,255,255,.85) 叠在
            // --bg-page 上，混出来就是这个略深的灰白。
            bg_bar: rgb(246, 248, 251),
            bg_card_hover: rgb(241, 245, 249),
            nav_hover: accent_tint(accent, 15),
            nav_selected: accent_tint(accent, 38),
            text_primary: rgb(30, 41, 59),
            text_secondary: rgb(71, 85, 105),
            text_muted: rgb(148, 163, 184),
            border: rgb(235, 238, 242),
            success: rgb(16, 185, 129),
            danger: rgb(220, 38, 38),
            text_disabled: rgb(203, 213, 225),
            border_light: rgb(240, 242, 245),
            glass_light: rgb(255, 255, 255),
            window_bg: rgb(255, 255, 255),
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
    // 背景层级：面板 = 页面底；窗口/弹窗 = 浮层底（比页面底亮一档，否则弹窗
    // 和页面糊成一片，暗色下尤其明显）；极端背景更深一档（输入框等）
    v.panel_fill = p.bg_page;
    v.window_fill = p.window_bg;
    v.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: if p.dark {
            Color32::from_black_alpha(120)
        } else {
            Color32::from_black_alpha(40)
        },
    };
    v.popup_shadow = v.window_shadow;
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
    // 控件各态：hover/active/selected 用 accent 强调，其余克制。
    // on_accent 必须按 accent 亮度取反色——浅色模式下蓝色按钮上用近黑文字，
    // 深色模式下用近白文字；直接拿 text_primary 会在浅色下变成黑字压深蓝（可读但脏），
    // 在 mica 浅色下变成白字压近白底（完全读不出）。
    let on_accent = Stroke::new(1.0_f32, on_accent_color(p.accent));
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

/// 实色 accent 底上的前景色：按感知亮度取近黑或近白。
/// 阈值 150 是经验值（锚点：蓝 #3b82f6 亮度 118 → 白字；mica 深色 #1e293b 亮度 40
/// → 白字；mica 浅色 #fafafa 亮度 250 → 黑字；绿 #10b981 亮度 148 → 黑字）。
pub fn on_accent_color(accent: Color32) -> Color32 {
    let lum = 0.299 * accent.r() as f32 + 0.587 * accent.g() as f32 + 0.114 * accent.b() as f32;
    if lum > 150.0 {
        rgb(24, 24, 27)
    } else {
        rgb(250, 250, 250)
    }
}

/// 由基色生成低透明度变体（用于选中/悬停底，避免实色 accent 过艳）。
pub fn accent_tint(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// 由基色生成指定不透明度的实色叠加（用于斑马纹/悬停行，避免透明度叠加导致发灰）。
pub fn overlay(base: Color32, top: Color32, alpha: f32) -> Color32 {
    let a = alpha.clamp(0.0, 1.0);
    let mix = |b: u8, t: u8| (b as f32 * (1.0 - a) + t as f32 * a).round() as u8;
    Color32::from_rgb(
        mix(base.r(), top.r()),
        mix(base.g(), top.g()),
        mix(base.b(), top.b()),
    )
}

/// 语义色浅底（成功/危险 pill、toast 用）：与页面底混合出 10% 语义色底。
pub fn tinted_fill(p: &Palette, c: Color32, alpha: f32) -> Color32 {
    overlay(p.bg_card, c, alpha)
}

/// 卡片顶部的 1px 内高光颜色（近似 CSS `inset 0 1px 0 var(--glass-light)`）。
pub fn highlight(p: &Palette) -> Color32 {
    if p.dark {
        Color32::from_rgba_unmultiplied(255, 255, 255, 14)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 200)
    }
}

/// 当前主题列表（设置页外观选择器用，顺序与 web data-theme 一致）。
///
/// 用 `fn` 而不是 `const`：主题名要走 i18n，而 `crate::t!` 是运行期查表
/// （含 thread-local 语言状态），不能在 const 上下文里求值。
pub fn themes() -> [(&'static str, &'static str); 8] {
    [
        ("", crate::t!("默认")),
        ("blue", crate::t!("蓝色")),
        ("purple", crate::t!("紫色")),
        ("green", crate::t!("绿色")),
        ("orange", crate::t!("橙色")),
        ("gray", crate::t!("灰色")),
        ("mica", crate::t!("云母")),
        ("material-you", "Material You"),
    ]
}

/// 主题色块的展示色（不依赖 dark/light，仅用于选择器预览）
pub fn theme_swatch(theme: &str) -> Color32 {
    palette(true, theme).accent
}
