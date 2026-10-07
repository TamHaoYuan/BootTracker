//! 共享 UI 组件（卡片 / 区块 / 导航项 / 状态胶囊 / KPI 卡 / 列表行）
//!
//! 抽出来的动机：`records_table`、`kpi_card`、各页 heading 原本各自手写
//! 「圆角 + bg_card 填充 + border_light 描边 + 顶部内高光」，全应用出现三种
//! 卡片风格。这里统一成 `card()` / `section()`，与 web 的 Card 组件对齐
//! （`frontend/src/styles/tokens.css` 的卡片圆角 12/16、`--bg-card`、`--border-light`）。

use egui::{Align2, Color32, FontId, Margin, Pos2, Rect, Response, RichText, Rounding, Stroke, Vec2};

use crate::theme::{self, Palette, R_CARD, R_INPUT, R_PANEL};

/// 卡片顶部 1px 内高光——近似 web `inset 0 1px 0 var(--glass-light)`。
/// egui 没有 backdrop-filter，玻璃质感只能靠这根亮线 + 1px 描边。
pub fn inner_highlight(painter: &egui::Painter, rect: Rect, p: &Palette) {
    let inset = 8.0;
    painter.line_segment(
        [
            Pos2::new(rect.left() + inset, rect.top() + 1.0),
            Pos2::new(rect.right() - inset, rect.top() + 1.0),
        ],
        Stroke::new(1.0_f32, theme::highlight(p)),
    );
}

/// 统一卡片容器：圆角 R_PANEL + bg_card 填充 + border_light 描边 + 内高光。
pub fn card<R>(ui: &mut egui::Ui, p: &Palette, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(p.bg_card)
        .stroke(Stroke::new(1.0_f32, p.border_light))
        .rounding(Rounding::same(R_PANEL))
        .inner_margin(Margin::same(16.0))
        .show(ui, |ui| {
            let r = add(ui);
            // 内容绘制完再补内高光，压在 Frame 的填充之上
            inner_highlight(ui.painter(), ui.min_rect().expand2(Vec2::new(16.0, 16.0)), p);
            r
        })
        .inner
}

/// 带标题的卡片区块：图标 + 标题 +（可选）右侧操作区 + 内容。
pub fn section<R>(
    ui: &mut egui::Ui,
    p: &Palette,
    icon: &str,
    title: &str,
    right: impl FnOnce(&mut egui::Ui),
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    card(ui, p, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(18.0).color(p.accent));
            ui.label(RichText::new(title).size(17.0).strong().color(p.text_primary));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
        });
        ui.add_space(2.0);
        add(ui)
    })
}

/// 带图标的按钮（默认按钮字号 15，与图标视觉重量匹配）
pub fn icon_button(ui: &mut egui::Ui, icon: &str, text: &str) -> Response {
    let label = if text.is_empty() {
        RichText::new(icon)
    } else {
        RichText::new(format!("{icon}  {text}"))
    };
    ui.button(label)
}

/// 图标 + 文字混排（Proportional 族逐字回落，图标与中文可同串）
pub fn ic(icon: &str, text: &str) -> RichText {
    RichText::new(format!("{icon}  {text}"))
}

/// 状态胶囊：全圆角 + 语义色 12% 底 + 同色 30% 描边 + 语义色文字。
///
/// 对齐 web Header 里的连接状态 pill；不同语义（在线/离线/危险）共用一套形状，
/// 只换颜色，避免每种状态各画一套。
pub fn status_pill(ui: &mut egui::Ui, p: &Palette, icon: &str, text: &str, color: Color32) -> Response {
    let gal = ui.painter().layout_no_wrap(
        format!("{icon}  {text}"),
        FontId::new(13.0, egui::FontFamily::Proportional),
        color,
    );
    let size = Vec2::new(gal.size().x + 20.0, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(rect.height() / 2.0),
        theme::tinted_fill(p, color, 0.14),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(rect.height() / 2.0),
        Stroke::new(1.0_f32, theme::tinted_fill(p, color, 0.45)),
    );
    painter.galley(
        Pos2::new(rect.left() + 10.0, rect.center().y - gal.size().y / 2.0),
        gal,
        color,
    );
    resp
}

/// 侧栏导航项：常态透明 / hover 淡 accent 底 / 选中 accent 底 + 左侧竖条。
///
/// `expanded=false` 时只画居中图标，用 tooltip 补名字（对齐 web 折叠态 Sider）。
/// `anim`（0→1）让展开态的文字/图标随侧栏宽度一起淡入，避免硬切。
pub fn nav_item(
    ui: &mut egui::Ui,
    p: &Palette,
    icon: &str,
    label: &str,
    selected: bool,
    expanded: bool,
    anim: f32,
) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, theme::NAV_H), egui::Sense::click());
    let hovered = resp.hovered();
    let painter = ui.painter();
    let rounding = Rounding::same(R_INPUT);
    if selected {
        painter.rect_filled(rect, rounding, p.nav_selected);
        // 左侧 3px 竖条作为选中标记（色盲可用，不只靠底色）
        let bar = Rect::from_min_size(
            Pos2::new(rect.left() + 2.0, rect.top() + 8.0),
            Vec2::new(3.0, rect.height() - 16.0),
        );
        painter.rect_filled(bar, Rounding::same(2.0), p.accent);
    } else if hovered {
        painter.rect_filled(rect, rounding, p.nav_hover);
    }
    let fg = if selected {
        p.accent
    } else if hovered {
        p.text_primary
    } else {
        p.text_secondary
    };
    if expanded {
        // 文字随展开进度淡入：宽度动画中文字不会突然"跳"出来
        let a = anim.clamp(0.0, 1.0);
        let icon_fg = Color32::from_rgba_unmultiplied(fg.r(), fg.g(), fg.b(), (255.0 * a) as u8);
        painter.text(
            Pos2::new(rect.left() + 14.0, rect.center().y),
            Align2::LEFT_CENTER,
            icon,
            FontId::new(theme::NAV_ICON, egui::FontFamily::Proportional),
            icon_fg,
        );
        painter.text(
            Pos2::new(rect.left() + 44.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::new(theme::NAV_TEXT, egui::FontFamily::Proportional),
            icon_fg,
        );
    } else {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::new(theme::NAV_ICON + 2.0, egui::FontFamily::Proportional),
            fg,
        );
    }
    if !expanded {
        resp.on_hover_text(label.to_owned())
    } else {
        resp
    }
}

/// 列表行：hover 高亮 + 斑马纹 + 底部 1px 分隔线，替换裸 `ui.horizontal` 行。
///
/// 行底先画（在内层 Ui 之前），内容之后不再叠加，避免半透明底色叠出灰块。
pub fn list_row<R>(
    ui: &mut egui::Ui,
    p: &Palette,
    striped: bool,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let w = ui.available_width();
    let h = 32.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::hover());
    let painter = ui.painter().clone();
    if resp.hovered() {
        painter.rect_filled(rect.expand2(Vec2::new(4.0, 0.0)), Rounding::same(8.0), p.nav_hover);
    } else if striped {
        painter.rect_filled(
            rect.expand2(Vec2::new(4.0, 0.0)),
            Rounding::same(8.0),
            theme::overlay(p.bg_card, p.text_primary, 0.03),
        );
    }
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(8.0, 0.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let out = add(&mut child);
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom()),
            Pos2::new(rect.right(), rect.bottom()),
        ],
        Stroke::new(1.0_f32, p.border_light),
    );
    out
}

/// KPI 卡（对齐 web `KpiCard.tsx`）
///
/// 公式：标签（12px muted / hero 13px）+ 大数值（26px/700，hero 40px）+ 可选趋势行。
/// 趋势用「箭头 + 语义色」双编码（色盲可用）。hero 变体对应 web「今日开机」主卡：
/// accent 低透明度底 + accent 描边。
pub fn kpi_card(
    ui: &mut egui::Ui,
    p: &Palette,
    title: &str,
    value: &str,
    trend: Option<(&str, Color32)>,
    hero: bool,
    accent: bool,
) -> Response {
    // mica 等主题 accent 接近纯白：hero 底色若仍用 46α 会变一块白板，
    // 白字压白底读不清——按 accent 亮度自适应降低叠加浓度。
    let accent_lum =
        0.299 * p.accent.r() as f32 + 0.587 * p.accent.g() as f32 + 0.114 * p.accent.b() as f32;
    let hero_alpha = if accent_lum > 180.0 { 26 } else { 46 };
    // 注意必须用 unmultiplied：from_rgba_premultiplied 会按「已乘 alpha」解释通道值，
    // 传 (250,250,250,26) 这种直通色会渲染成一块接近纯白的板子，白字压白底读不清。
    let (fill, stroke_color) = if hero {
        (
            crate::theme::overlay(p.bg_card, p.accent, hero_alpha as f32 / 255.0),
            crate::theme::overlay(p.bg_card, p.accent, 90.0 / 255.0),
        )
    } else {
        (p.bg_card, p.border_light)
    };
    let title_size = if hero { 13.0 } else { 12.0 };
    let value_size = if hero { 40.0 } else { 26.0 };

    // layout_no_wrap 精确测量文本（纯计算、无 Ui 副作用）。曾用 sizing_pass 探针
    // 在 wrapped 布局下量出过小宽度，导致文字逐字换行溢出卡片。
    let title_gal = ui.painter().layout_no_wrap(
        title.to_owned(),
        FontId::new(title_size, egui::FontFamily::Proportional),
        Color32::WHITE,
    );
    // 数值用等宽族（tabular 数字），避免实时计时时数字宽度跳动
    let value_gal = ui.painter().layout_no_wrap(
        value.to_owned(),
        FontId::new(value_size, egui::FontFamily::Monospace),
        Color32::WHITE,
    );
    let trend_gal = trend.map(|(t, _)| {
        ui.painter().layout_no_wrap(
            t.to_owned(),
            FontId::new(12.0, egui::FontFamily::Proportional),
            Color32::WHITE,
        )
    });

    let trend_w = trend_gal.as_ref().map(|g| g.size().x).unwrap_or(0.0);
    let inner_w = title_gal.size().x.max(value_gal.size().x).max(trend_w);
    let inner_h = title_gal.size().y
        + 2.0
        + value_gal.size().y
        + trend_gal.as_ref().map(|g| 2.0 + g.size().y).unwrap_or(0.0);
    // 纵向布局（hero 独占一行）撑满可用宽度；横向排列按内容收缩
    let w = if ui.layout().is_vertical() {
        ui.available_width()
    } else {
        inner_w + 36.0
    };
    let size = Vec2::new(w, inner_h + 32.0);

    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let hovered = resp.hovered();
    let rect = rect.translate(Vec2::new(0.0, if hovered { -3.0 } else { 0.0 }));
    let painter = ui.painter().clone();
    painter.rect_filled(rect, Rounding::same(R_CARD), fill);
    if hovered {
        painter.rect_filled(
            rect,
            Rounding::same(R_CARD),
            Color32::from_rgba_unmultiplied(p.accent.r(), p.accent.g(), p.accent.b(), 22),
        );
    }
    painter.rect_stroke(
        rect,
        Rounding::same(R_CARD),
        Stroke::new(1.0_f32, if hovered { p.accent } else { stroke_color }),
    );
    inner_highlight(&painter, rect, p);

    // 内容用 painter 按测量结果直接定位，与卡片矩形严格一致，绝无换行溢出
    let x = rect.left() + 18.0;
    let mut y = rect.top() + 16.0;
    painter.text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        title,
        FontId::new(title_size, egui::FontFamily::Proportional),
        if hero { p.text_secondary } else { p.text_muted },
    );
    y += title_gal.size().y + 2.0;
    painter.text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        value,
        FontId::new(value_size, egui::FontFamily::Monospace),
        if accent { p.accent } else { p.text_primary },
    );
    if let (Some((t, c)), Some(g)) = (trend, trend_gal) {
        y += value_gal.size().y + 2.0;
        painter.text(
            Pos2::new(x, y),
            Align2::LEFT_TOP,
            t,
            FontId::new(12.0, egui::FontFamily::Proportional),
            c,
        );
        let _ = g;
    }
    resp
}

/// 表格单元格：固定宽高 + 垂直居中 + 超长截断，保证各行列对齐。
pub fn cell(ui: &mut egui::Ui, w: f32, h: f32, content: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        Vec2::new(w, h),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(Vec2::new(w, h));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            content(ui);
        },
    );
}

/// 空状态引导卡（对齐 web `EmptyState.tsx`：图标 + 标题 + 说明）
pub fn empty_state(ui: &mut egui::Ui, p: &Palette, icon: &str, title: &str, desc: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(RichText::new(icon).size(36.0).color(p.text_disabled));
        ui.add_space(6.0);
        ui.label(
            RichText::new(title)
                .size(15.0)
                .strong()
                .color(p.text_secondary),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(desc).size(13.0).color(p.text_muted));
        ui.add_space(24.0);
    });
}

/// 加载态：与 `empty_state` 同构，图标换成会转的圆弧指示器。
///
/// egui 无法旋转字形，静态 CIRCLE_NOTCH 看着像卡住了；用 `anim::spinner`
/// 画一圈透明度递减的圆弧并持续重绘。
pub fn loading_state(ui: &mut egui::Ui, p: &Palette, title: &str, desc: &str) {
    let t = crate::anim::now(ui.ctx());
    ui.ctx().request_repaint();
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
        crate::anim::spinner(ui.painter(), rect.center(), 15.0, t, p.text_disabled);
        ui.add_space(6.0);
        ui.label(
            RichText::new(title)
                .size(15.0)
                .strong()
                .color(p.text_secondary),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(desc).size(13.0).color(p.text_muted));
        ui.add_space(24.0);
    });
}
