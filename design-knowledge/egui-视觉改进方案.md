# egui 桌面端「视觉简陋」改进方案

> 依据：`design-spec-boottracker.md`（web 端设计体系逆向分析）。
> 目标：让 `desktop/`（egui/eframe）在观感上追平 web 端，而不是简单换个颜色。
> 日期：2026-10-04

## 1. 差距诊断：egui 端 vs web 端

| 维度 | web 端 | egui 端现状 | 差距 |
|---|---|---|---|
| 空状态 | 四段式：图标 36px + 标题 15/600 + 说明 13 + 动作按钮 | `bar_chart` 只画一行灰字「无数据」；dashboard 写「加载中…」 | **最大短板**（web 注释：空状态占感知精致度 80%） |
| KPI 卡 | 标签 + 大数值（26/40，700，tabular-nums）+ 趋势箭头双编码 + hero 渐变底 + glow | `kpi_card`：标题 small weak + 数值 22 strong，**无趋势、无 hero、无渐变** | 信息量与精致度都缺 |
| 圆角 | 分层：输入 10 / 卡片 12 / Card 16 / Modal 20 | 统一 `Rounding::same(8.0)`（theme.rs） | 无层级，显扁平 |
| 卡片质感 | 玻璃底 + `inset 0 1px 0` 内高光 + 淡投影 | 纯实色 `bg_card` 填充，**无内高光、无投影** | 像色块不像卡片 |
| 侧栏 | 图标 + 分组 + 选中项 accent 底色 + 图标染色 | 纯文字（收起时单字「仪/记/图/设/管」），**无图标、无分组** | 辨识度低 |
| 顶栏 | 56px 高 + 折叠按钮 + 底边线 + accent 投影 | `heading` + 连接状态 + 版本 + 主题按钮，**无高度/分隔线** | 结构松散 |
| 表格 | 容器 16px 圆角 + 行 hover 底色 + **逐行阶梯入场** | `Grid::striped`，无容器圆角、无 hover、无动画 | 静态、生硬 |
| 动效 | 统一非线性缓动、页面 riseIn、行阶梯、hover 抬升 -3px | **完全没有动效**（静态即时模式） | 最影响"精致感" |
| 数字 | tabular-nums + JetBrains Mono | 默认比例字体 | 实时数字会抖 |

**根因分两类**：
1. **实现缺失**（可低成本补）：空状态、KPI 卡、圆角分层、侧栏图标、内高光、顶栏结构。
2. **egui 技术限制**（需变通）：无 `backdrop-filter`（不能真模糊）、无 CSS 动画（需手动插值）、无 `tabular-nums`（需换等宽字体）。

## 2. 方案总览（按投入产出排序）

| 优先级 | 项目 | 投入 | 感知提升 |
|---|---|---|---|
| P0 | 统一的空状态组件 | 小 | 极高 |
| P0 | KPI 卡升级（趋势 + hero + 渐变） | 小 | 高 |
| P0 | 圆角分层（10/12/16/20） | 极小 | 中 |
| P0 | 卡片内高光 + 描边 | 小 | 高 |
| P1 | 侧栏加图标 + 选中态强调 | 中 | 高 |
| P1 | 顶栏规范化（56px + 分隔线） | 小 | 中 |
| P1 | hover 反馈（抬升 + 边框亮起） | 中 | 高 |
| P2 | 入场动效（页面 + 逐行阶梯） | 中 | 高 |
| P2 | 背景柔光装饰（替代 glass-orb） | 中 | 中 |
| P2 | 数字等宽 + 数值脉冲 | 小 | 中 |

## 3. P0 方案（建议先做，半天内可完成）

### 3.1 统一空状态组件

**目标**：所有"无数据/加载中"场景使用同一组件，结构对齐 web `EmptyState`。

**新增** `desktop/src/pages.rs`（或单独 `components.rs`）中的 `empty_state()`：

```
布局：垂直居中，padding 48x24
  ├─ 图标区：36px 字符/符号，色 = palette.text_disabled
  ├─ 标题：15px / 600（egui 用 RichText.size(15).strong()），色 = text_secondary
  ├─ 说明：13px，色 = text_muted，最大宽 360，行高 1.7
  └─ 可选动作按钮
```

**替换点**（当前简陋文案）：
- `bar_chart` / `line_chart` 的 `"无数据"` 灰字（`pages.rs` 第 18-26、60-68 行）
- `dashboard` 的 `"加载中…"`（第 103 行）
- `charts` 的 `"加载中…"`（第 385 行）
- `records` 空列表、`admin` 的「回收站为空」「暂无备份」（第 632、658 行）
- `settings` 各 tab 的「加载中…」

**Palette 需补字段**：`text_disabled`（dark `#52525b` / light `#cbd5e1`）。

### 3.2 KPI 卡升级

**目标**：对齐 web `KpiCard` 公式。

**当前**（`pages.rs` 第 182-199 行）：`title`(small weak) + `value`(size 22 strong)。

**改为**：
- 标签：12px，`text_muted`（hero 用 13px `text_secondary`）
- 数值：**26px / strong**（hero **40px**），`line_height` 1.25（hero 1.15）
- 趋势行（新增 `trend: Option<(Dir, String)>`）：箭头符号 ↑ ↓ − + 文本，12px，色按 `success` / `danger` / `text_muted`（**形状 + 颜色双编码**）
- hero 变体：`bg = accent` 低透明度叠加（`Color32::from_rgba_premultiplied(accent.rgb, ~46)`，对应 web 的 `rgba(accent,0.18→0.04)` 渐变，egui 无渐变可用实色近似），边框 `rgba(accent,0.35)`
- 圆角 **12**（非 8）

### 3.3 圆角分层

在 `theme.rs` 导出常量并全局替换：

```rust
pub const R_INPUT: f32 = 10.0;   // 输入框
pub const R_CARD: f32 = 12.0;    // KPI 卡 / Segmented
pub const R_PANEL: f32 = 16.0;   // Card / 表格容器
pub const R_MODAL: f32 = 20.0;   // 弹窗
```

**替换点**：`theme.rs` 的 `Rounding::same(8.0)`（控件态）、`pages.rs` 的 `Rounding::same(8.0)`（kpi_card）、`Rounding::same(2.0)`（柱状条，保持小圆角）、`app.rs` 确认弹窗用 `R_MODAL`。

### 3.4 卡片内高光 + 描边（模拟玻璃质感）

egui 无 `backdrop-filter`，用**内高光 + 描边**廉价地做出"玻璃感"：

```
绘制卡片矩形后：
1. painter.rect_stroke(rect, rounding, Stroke::new(1.0, border_light))   // 外描边
2. 顶部内高光：在 rect 顶部内侧画 1px 亮线
   painter.line_segment([rect.left_top()+inset, rect.right_top()+inset],
                        Stroke::new(1.0, glass_light))
3. 可选投影：egui 0.29 Frame 支持 .shadow(Shadow{...})，用极淡的值
```

**Palette 需补**：`glass_light`（dark `rgba(255,255,255,0.06)` → `(255,255,255,15)`；light 用黑 8）。

## 4. P1 方案

### 4.1 侧栏加图标 + 选中态

**当前**（`app.rs` 第 700-748 行）：`selectable_value` 纯文字，收起时用单字。

**改为**：每个页面配一个符号（egui 无图标字体，用 Unicode 或纯符号）：
- 仪表盘 `▤`、记录 `≡`、图表 `◫`、设置 `⚙`、管理 `⛭`
- 渲染：`ui.horizontal(|ui| { icon; label })`，收起时只显示图标
- 选中态：egui 的 `selectable_value` 自带高亮（用 `visuals.selection`），已在 theme.rs 设为 accent —— 需确认对比度；建议选中项用 `rgba(accent,0.15)` 底 + **图标染 accent 色**

### 4.2 顶栏规范化

**当前**（`app.rs` 第 750-777 行）：一行 `horizontal`。
**改为**：固定高度 56（`ui.set_min_height(56.0)`）+ 底部 1px 分隔线（`painter.line_segment` 画 `border_light`）+ 左侧放折叠按钮（已有 `≡` 在侧栏底部，可移到顶栏）。

### 4.3 hover 反馈

egui 有 `Response::hovered()`，可模拟 web 的 `translateY(-3px)`：

```
在自定义卡片绘制时：
let hovered = resp.hovered();
let rect = if hovered { rect.translate(egui::vec2(0.0, -3.0)) } else { rect };
// 边框色：hovered 时用 rgba(accent,0.25)，否则 border_light
```

注：标准 `egui::Frame`/`egui::Button` 的 hover 已由 `theme.rs` 的 `widgets.hovered`（accent 实底）接管——**当前 hovered 用 accent 实底可能过艳**，建议改为低透明度 accent（对齐 web 的 `rgba(accent,0.06~0.15)`），实底只留给 active/pressed。

## 5. P2 方案（锦上添花）

### 5.1 入场动效

egui 无 CSS 动画，但即时模式天然适合手动插值：
- 记录元素首次出现时间（`ctx.input(|i| i.time)`）
- 每帧计算 `t = (now - start) / duration`，用 ease-out-expo 近似：`1 - 2f32.powf(-10*t)`
- 应用：`alpha = t`，`offset_y = 20 * (1 - t)`，`scale = 0.988 + 0.012*t`
- **逐行阶梯**：每行 `start += index * 0.03`（对齐 web 表格 0.02s 起、步长 0.03s）
- 需配合 `ctx.request_repaint()` 保证动画期间持续重绘（当前已有 1s 一次的 repaint，动画期间需提高到每帧）

### 5.2 背景柔光装饰（替代 glass-orb）

egui 无 `blur(80px)`，用**多层同心圆低透明度叠加**近似柔光：
```
for i in 0..6 {
    painter.circle_filled(center, radius - i*step,
        Color32::from_rgba_unmultiplied(accent_rgb, 6));  // 每层 ~0.02 alpha 叠加
}
```
仅仪表盘显示，静态即可（不做浮动动画，省 CPU）。

### 5.3 数字等宽 + 脉冲

- 时长/计数用等宽渲染：egui 可用 `RichText::font(FontId::monospace(..))`；更彻底的做法是在字体注册时把 Monospace 家族也指向中文字体（已在 `main.rs` 做过）
- 数值变化时脉冲：记录上次值 + 变化时刻，`scale/color` 短暂插值（对齐 web `countPulse`）

## 6. egui 能力边界（重要，避免做无用功）

| web 能力 | egui 能否实现 | 变通方案 |
|---|---|---|
| `backdrop-filter` 真模糊 | ❌ | 半透明底 + 内高光 + 描边近似 |
| `linear-gradient` 渐变 | ⚠️ 无原生 | 多层半透明矩形叠加，或逐行 painter 绘制（开销大，仅 hero 卡用） |
| CSS 动画 | ❌ | 手动时间插值 + `request_repaint` |
| `tabular-nums` | ⚠️ | 用 Monospace 字体族 |
| 阴影 | ✅ | `epaint::Shadow`（用极淡值，别照搬 CSS 的重阴影） |
| hover / 选中态 | ✅ | `Response::hovered()` / `visuals.widgets.*` |
| 图标 | ⚠️ | 无图标字体，用 Unicode 符号或内嵌图片 |

## 7. 建议实施顺序

1. **先做 P0 的 4 项**（空状态、KPI 卡、圆角分层、卡片内高光）——投入小、感知提升最大，且能立刻消除"简陋"的第一印象。
2. **再补 P1 的 hover 与侧栏**——解决"静态、无反馈"的问题。
3. **动效（P2）最后做**，且建议只做「页面入场 + 表格逐行」，不要全站铺开（即时模式每帧重绘有 CPU 成本，小组件尤其要克制）。

每做完一批重新 `cargo clippy --all-targets -- -D warnings` + `cargo build --release`，运行对比 web 端截图确认。

## 8. 验收对照清单

对照 web 端截图逐项核对（P0 / P1 / P2 均已于 2026-10-04 ~ 10-05 实施完成）：
- [x] 空状态有图标 + 标题 + 说明（不是一行灰字）—— `empty_state()`，图标统一 Phosphor
- [x] KPI 数值 26px（hero 40px）、有趋势箭头、hero 卡有 accent 底
- [x] 圆角：输入框 10 / 卡片 12 / 容器 16 / 弹窗 20（`theme.rs` 的 `R_*` 常量）
- [x] 卡片有 1px 描边 + 顶部内高光（`inner_highlight()`）
- [x] hover 时卡片上移 3px 且有 accent 边框（`hover_card()`，不是整块 accent 实底）
- [x] 侧栏每项有图标，选中项图标为 accent 色（`Page::icon()`，Phosphor）
- [x] 顶栏高 56px 且底部有分隔线
- [x] 表格/列表逐行错峰入场（`app.row_t()` + `ui.set_opacity`）
- [x] 时长数字不抖动（等宽，Monospace 族）
- [x] 表格容器 16px 圆角 + 表头 + 整行 hover 底色（自绘 `records_table()`，弃用 Grid）
- [x] 弹窗 / Toast 圆角 20（`visuals.window_rounding = R_MODAL`）

**有意不做**：数值脉冲。本次会话时长每秒刷新一次，脉冲会变成持续闪动的噪音，
投入产出比低于已完成的各项。若将来只在「累计开机/关机次数」这类低频变化的值上做，才值得。

## 9. 补充：图标库选型（2026-10-05 落地）

**选中 `egui-phosphor 0.7.3`（Phosphor Icons）**，理由：
- egui 生态事实标准，MIT，~1500 图标、5 种字重（regular/bold/fill/light/thin）
- 纯字体方案（`add_to_fonts` 注入 PUA 码位），无图片资源、无运行时依赖
- 与中文混排可行：CJK 字体注册在 `Proportional` 族 0 位，Phosphor 插在 1 位，
  egui 逐字回落 → 一条字符串里「图标 + 中文」直接混排，不必拆成两个 label

**版本硬约束**：`egui-phosphor 0.7.x` 依赖 `egui ^0.29`；`0.8+` 需要 egui 0.30。
将来升级 egui 主版本时必须同步升级图标库，否则 Cargo 解析失败。

**已知坑**：regular 变体**没有 `WARNING_DANGER`**，危险按钮改用 `WARNING_OCTAGON`。

已接入：`desktop/`（主程序，按钮/tab/侧栏/顶栏/空状态全覆盖）与 `desktop-widget/`（浮窗）。
