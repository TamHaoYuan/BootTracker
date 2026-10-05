# BootTracker Web UI 设计解析

> 一句话定位：Windows 开机/关机追踪工具，克制 + 高信息密度 + 玻璃质感，深色优先。
> 分析来源：`F:\BootTracker\frontend`（Vite + React 18 + TS + Ant Design 5 + Zustand）；分析日期：2026-10-04。

## 1. 设计基调

- **产品类型与场景**：系统级常驻工具（开机自启 + 系统托盘 + 桌面浮窗）。用户每天多次瞥视仪表盘，属于**长时间、高频次、低注意力**使用。
- **设计取向推断**：深色优先 + 近单色（"Dark Gallery" 母题——极深炭黑底，卡片像展品一样亮起）。装饰极少，视觉层级靠**背景分层 + 边框 + 字重**，而非色彩。强调色只出现在可交互元素与状态上。
  - 证据：`tokens.css` 注释「Dark Gallery：极深炭黑底，卡片如展品亮起」；`--accent` 只用于 hover/选中/focus/趋势。
- **品牌记忆点**：
  - **Glass Orb 背景装饰球**——仅首页显示的三颗高斯模糊球体（`filter: blur(80px)`，20/25/30s 缓慢浮动），是全局最独特的视觉母题。
  - **启动动画**（`BootSplash`）：图标弹性弹入 + 名称 tracking-in（字距 6px→0.5px、blur 4px→0）+ accent 进度线扫过。
  - 侧栏品牌名用 accent 色 + `font-weight: 700` + `letter-spacing: 0.3`。

## 2. Design Tokens（精确值）

### 色彩

深色（默认，blue 主题）：

| 令牌 | 值 | 用途 |
|---|---|---|
| --bg-page | `#0a0a0c`（blue 主题 `#080a10`） | 页面底色 |
| --bg-page2 / --bg-page3 | `#111114` / `#08080a` | 次级背景层 |
| --bg-card | `rgba(28,28,32,0.96)` | 卡片/侧栏/顶栏 |
| --bg-card-light | `rgba(42,42,48,0.85)` | 表头、次级块 |
| --bg-input | `rgba(28,28,32,0.96)` | 输入框/表头 |
| --text-primary | `#fafafa` | 主文字 |
| --text-secondary | `#c4c4cc` | 次文字 |
| --text-muted | `#8b8b94` | 弱文字/标签 |
| --text-disabled | `#52525b` | 禁用/空状态图标 |
| --accent (blue) | `#3b82f6`，rgb `59,130,246` | 强调（可交互元素） |
| --accent2 | `#06b6d4` | 图表第二色 |
| --accent-light | `#60a5fa` | hover/发光 |
| --text-success | `#6ee7b7`（强 `#10b981`） | 上升趋势/在线 |
| --text-danger | `#f87171`（强 `#ef4444`） | 下降趋势/危险操作 |
| --border-color | `rgba(255,255,255,0.1)` | 主分隔线 |
| --border-light | `rgba(255,255,255,0.06)` | 卡片/行分隔 |
| --border-muted | `rgba(255,255,255,0.16)` | 输入框边框 |
| --shadow-color | `rgba(0,0,0,0.5)` | 卡片投影 |
| --glass-light | `rgba(255,255,255,0.06)` | 卡片内高光（inset） |
| --glass-hover | `rgba(255,255,255,0.05)` | 行 hover 底 |

浅色（`body.light-mode`）：

| 令牌 | 值 |
|---|---|
| --bg-page | `#f8fafc` |
| --bg-card | `rgba(255,255,255,0.85)` |
| --bg-input | `rgba(255,255,255,0.95)` |
| --text-primary / secondary / muted | `#1e293b` / `#475569` / `#94a3b8` |
| --border-color | `rgba(0,0,0,0.08)` |
| --accent | 同深色（`#3b82f6`） |

7 套 accent 主题（`data-theme`）：purple `#8b5cf6`、blue `#3b82f6`（默认）、green `#10b981`、orange `#f97316`、gray `#64748b`、mica `#fafafa`、material-you `#d0bcff`。

```css
/* 可直接复制到新项目 */
:root {
  --bg-page: #0a0a0c;
  --bg-page2: #111114;
  --bg-page3: #08080a;
  --bg-card: rgba(28, 28, 32, 0.96);
  --bg-card-light: rgba(42, 42, 48, 0.85);
  --bg-input: rgba(28, 28, 32, 0.96);
  --bg-content: rgba(18, 18, 22, 0.5);

  --text-primary: #fafafa;
  --text-secondary: #c4c4cc;
  --text-muted: #8b8b94;
  --text-disabled: #52525b;

  --accent: #3b82f6;
  --accent2: #06b6d4;
  --accent-light: #60a5fa;
  --accent-rgb: 59, 130, 246;

  --text-success: #6ee7b7;
  --text-success-dark: #10b981;
  --text-danger: #f87171;
  --text-danger-dark: #ef4444;

  --border-color: rgba(255, 255, 255, 0.1);
  --border-light: rgba(255, 255, 255, 0.06);
  --border-muted: rgba(255, 255, 255, 0.16);

  --shadow-color: rgba(0, 0, 0, 0.5);
  --glass-light: rgba(255, 255, 255, 0.06);
  --glass-hover: rgba(255, 255, 255, 0.05);
  --glass-border: rgba(255, 255, 255, 0.08);

  --ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
  --ease-smooth: cubic-bezier(0.45, 0, 0.15, 1);
  --ease-in-out-quart: cubic-bezier(0.76, 0, 0.24, 1);
}
```

### 字体排印

字体栈：`'Inter', 'Noto Sans SC', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', sans-serif`；数字场景用 `'JetBrains Mono', monospace`。

| 层级 | 字号 / 字重 | 用于 |
|---|---|---|
| KPI 主数值（hero） | 40px / 700 | 今日开机卡（`line-height: 1.15`） |
| KPI 数值 | 26px / 700 | 常规 KPI 卡（`line-height: 1.25`） |
| 空状态标题 | 15px / 600 | `.empty-title` |
| 页面标题（顶栏） | 15px / 600 | AppLayout Header |
| 侧栏品牌名 | 16px / 700 | `letter-spacing: 0.3` |
| 正文 | 14px / 400 | AntD 默认 |
| KPI 标签 | 12px（hero 13px） | muted / secondary |
| 空状态说明 | 13px | `--text-muted`，`line-height: 1.7`，`max-width: 360px` |
| 趋势行 | 12px | `.kpi-trend`，tabular-nums |
| 侧栏时钟 | 11px | JetBrains Mono，`letter-spacing: 0.3` |
| 版本号 | 10px | `--text-disabled` |
| 空状态图标 | 36px | `--text-disabled` |

关键技巧：所有数值加 `.tnum`（`font-variant-numeric: tabular-nums`），避免数字跳动。

### 间距 / 圆角 / 阴影

- **间距基数**：4/8px 网格。最常用值：KPI 卡 `padding: 16px 18px`；内容区 `24px`；卡片内元素 gap `2~10px`；空状态 `padding: 48px 24px`。
- **圆角层级**（小元素小圆角、大容器大圆角）：
  - 输入框 / Select：`10px`
  - KPI 卡 / Segmented：`12px`
  - Card / Table 容器：`16px`
  - Modal：`20px`
  - 导航按钮块：AntD 默认
- **阴影策略**：**不以重阴影分层**，而是 `backdrop-filter: blur()` + `1px` 半透明边框 + `inset 0 1px 0 var(--glass-light)` 内高光。卡片投影仅 `0 8px 32px var(--shadow-color)`（很淡）。
- **栅格**：侧栏 208px（收起 64px）；顶栏 56px；内容区 padding 24px；KPI 用 `Row/Col` 自适应换行。

## 3. 布局系统

- **骨架**：`fixed` 侧栏（208/64，hover 悬浮展开）+ sticky 顶栏（56）+ 玻璃态内容区（`--bg-content` 半透明）。
- **侧栏内部三段**：品牌区（56px 高，底部分隔线）→ 菜单（分组：主功能 / 系统）→ 底部信息区（在线状态 + 时钟 + 版本号 + 明暗切换）。
- **侧栏定位**：`position: fixed; height: 100vh`（注释明确说明：用 absolute 时内容超一屏会露出底色断层）。
- **断点**：`768px`。`<768` 时侧栏改为 `Drawer` 抽屉，由顶栏菜单按钮触发。
- **容器最大宽度**：未限制（全宽铺满）。

## 4. 组件模式卡

### Card（玻璃态容器）
- **样式**：`background: var(--bg-card)`；`backdrop-filter: blur(24px) saturate(150%)`；`border: 1px solid var(--border-light)`；`border-radius: 16px`；`box-shadow: 0 8px 32px var(--shadow-color), inset 0 1px 0 var(--glass-light)`。
- **hover**：`border-color: rgba(accent-rgb, 0.25)`；`box-shadow: 0 12px 40px rgba(accent-rgb,0.08), inset 0 1px 0 var(--glass-border)`；`transform: translateY(-3px)`。
- **过渡**：`transform .35s var(--ease-spring), border-color .3s ease, box-shadow .4s var(--ease-smooth)`。
- **使用时机**：所有内容分区（KPI 区、图表、表格、设置分组）。

### KPI 卡（Stripe 公式）
- **结构**：标签（12px muted / hero 13px secondary）→ 大数值（26px/700，hero 40px/700，`.tnum`）→ 趋势行（箭头 + 文本，12px）。
- **hero 变体**：背景 `linear-gradient(135deg, rgba(accent-rgb,0.18), rgba(accent-rgb,0.04))`；边框 `rgba(accent-rgb,0.35)`；加 class `today-boot-card` 触发 `glow 4s infinite`。
- **趋势双编码**：`ArrowUp/Down/Minus` 图标 + 语义色（up=成功色、down=危险色、flat=muted）——**不靠颜色单编码**（无障碍）。
- **加载**：`Skeleton`（title 50% 宽 + 1 行 70% 宽）。
- **使用时机**：仪表盘顶部指标条。

### 导航（侧栏菜单）
- **容器**：`background: var(--bg-card)` + `backdrop-filter: blur(60px) saturate(180%)` + 右侧 1px 边框。
- **菜单项**：默认 `--text-muted`；hover 变 `--text-primary` + 底 `rgba(accent-rgb,0.06)`。
- **选中态**：底 `rgba(accent-rgb,0.15)` + `box-shadow: 0 2px 8px rgba(accent-rgb,0.1)` + **图标变 accent 色**。
- **收起态**：只留图标，品牌名隐藏，底部信息区改纵向排列。

### 表格
- **容器**：`background: var(--bg-card)` + `blur(40px)` + `border-radius: 16px` + `overflow: hidden`。
- **表头**：`background: var(--bg-input)`；文字 `--text-muted`；`font-weight: 500`；底部 1px 边框。
- **行动画**：`animation: slideInLeft .45s var(--ease-out-expo) both`，逐行 `animation-delay` 递增（0.02s 起，步长 0.03s，第 9 行后固定 0.26s）——**阶梯式入场**是精致感的关键。
- **行 hover**：`background: rgba(accent-rgb, 0.05)`。

### 输入框 / Select
- **常态**：`background: var(--bg-input)`；`border-color: var(--border-muted)`；`border-radius: 10px`。
- **focus**：`border-color: rgba(accent-rgb, 0.5)` + `box-shadow: 0 0 0 3px rgba(accent-rgb, 0.1)`（3px 光环）。

### 分段控制器（Segmented）
- `background: var(--bg-card-darker)`；`border-radius: 12px`；`padding: 4px`。
- 选中项：`background: rgba(accent-rgb,0.15)` + `box-shadow: 0 2px 8px rgba(accent-rgb,0.1)`。

### 空状态（Vercel 模式）
- **结构**：图标（36px，`--text-disabled`）→ 标题（15px/600，`--text-secondary`）→ 说明（13px，`--text-muted`，`max-width: 360px`，行高 1.7）→ 可选动作按钮。
- **容器**：`padding: 48px 24px`，居中，gap 6px。
- **设计意图**（源码注释）：「空状态占感知精致度 80%」——无数据时展示引导而非空白表格。

## 5. 交互与动效规范

- **统一过渡**：全局 `body * { transition: background-color .35s ease, color .3s ease, border-color .35s ease, box-shadow .35s ease, transform .25s cubic-bezier(.4,0,.2,1) }`。主题切换时全站平滑过渡。
- **缓动曲线**（非线性，避免线性生硬）：
  - `--ease-out-expo: cubic-bezier(0.16,1,0.3,1)` — 入场/位移
  - `--ease-spring: cubic-bezier(0.34,1.56,0.64,1)` — 卡片 hover（带轻微过冲）
  - `--ease-smooth: cubic-bezier(0.45,0,0.15,1)` — 阴影
  - `--ease-in-out-quart: cubic-bezier(0.76,0,0.24,1)` — 循环动画（glow/浮动）
- **页面切换**：`.page-enter` → `riseIn .5s var(--ease-out-expo)`（透明度 + 上移 20px + 微缩放 0.988→1），配合 `key={pathname}` 重挂载重放。
- **hover 语法**：卡片=抬升 +3px 上移 + 边框亮起；菜单/行=底色 accent 低透明度；按钮=AntD 默认。
- **加载态**：KPI 用 `Skeleton`；路由懒加载用 `<Spin>` 居中（侧栏/顶栏不动，仅内容区占位）。
- **背景装饰**：三颗 `glass-orb`（`filter: blur(80px)`，accent 透明度 0.08/0.06/0.04，20/25/30s 浮动），仅首页显示。
- **数字反馈**：`countPulse`（scale 1→1.15 + accent-light 色）用于数值更新。
- **无障碍**：`prefers-reduced-motion: reduce` 时启动动画瞬时完成；趋势用「箭头 + 颜色」双编码。
- **键盘**：`Alt+1~5` 切页、`Ctrl+Shift+A` 进管理；版本号连点 3 次进 Admin（隐藏入口）。

## 6. 设计原则提炼（带证据）

1. **背景分层而非阴影分层** — 证据：`global.css` 的 `.ant-card` 用 `backdrop-filter: blur(24px)` + `inset 0 1px 0 var(--glass-light)`，投影只有很淡的 `0 8px 32px`。意图：玻璃质感来自"透光 + 内高光"，重阴影会显脏。
2. **强调色只服务交互与状态** — 证据：`--accent` 仅出现在 hover/选中/focus/趋势箭头/品牌名；正文与容器一律近单色。意图：降低常驻视觉噪声，适合长时间使用。
3. **圆角随容器尺度递增** — 证据：输入 10 → 卡片 12 → Card/Table 16 → Modal 20。意图：形成稳定的视觉层级，避免全站同一圆角的扁平感。
4. **动效必须有阶梯与非线性缓动** — 证据：Table 行 `animation-delay` 逐行 +0.03s；`--ease-spring` 带过冲。意图：同时出现的元素错峰入场，是"精致"最廉价的来源。
5. **空状态与加载态是一等公民** — 证据：`EmptyState.tsx` 四段式结构 + 源码注释「空状态占感知精致度 80%」；KPI 用 Skeleton 而非 spinner。意图：数据工具必然遇到空/加载，处理好了显著提升品质感。
6. **数字必须等宽** — 证据：`.tnum { font-variant-numeric: tabular-nums }` 与 JetBrains Mono。意图：实时跳动的时长/计数不抖动。
7. **状态用双编码（形状 + 颜色）** — 证据：`KpiCard` 的 `ArrowUp/Down/Minus` + `trend-up/down/flat` 语义色。意图：色盲可用。

## 7. 复用指南

### 生成同类工具站时模仿
- [ ] 深色底 + 半透明卡片 + `backdrop-filter` 玻璃态，投影克制
- [ ] 强调色只给交互与状态，正文保持近单色
- [ ] 圆角按尺度分层（10/12/16/20）
- [ ] 列表/表格逐行阶梯入场（非线性缓动）
- [ ] 数值用 tabular-nums
- [ ] 空状态四段式：图标 + 标题 + 说明 + 动作
- [ ] 加载用骨架屏而非全局 spinner
- [ ] 侧栏 `position: fixed` + `100vh`（防滚动断层）

### 不要照搬
- [ ] `glass-orb` 背景球：是本项目品牌母题，且 `filter: blur(80px)` 开销大，非必要不复制（且仅首页显示）
- [ ] 隐藏手势（版本号连点 3 次进 Admin）：不利于可发现性，仅因个人工具容忍
- [ ] `!important` 满天飞的 AntD 覆盖写法：是改动既有组件库的权宜之计，新项目应从 token 层配置
- [ ] 全局 `body * { transition: ... }`：性能敏感场景会拖累，仅因桌面工具页面简单才用
- [ ] 业务术语（开机/关机/回收站/隧道）：与产品强绑定

### 最小可用起步配置
```html
<!-- tokens.css 变量 + 一张玻璃态卡片 + 一个 KPI 数值 -->
<style>
  :root { /* 见第 2 节 CSS 变量块 */ }
  body { background: var(--bg-page); color: var(--text-primary);
         font-family: 'Inter','Noto Sans SC',system-ui,sans-serif; }
  .card { background: var(--bg-card); backdrop-filter: blur(24px) saturate(150%);
          border: 1px solid var(--border-light); border-radius: 16px; padding: 16px 18px;
          box-shadow: 0 8px 32px var(--shadow-color), inset 0 1px 0 var(--glass-light);
          transition: transform .35s var(--ease-spring), border-color .3s ease; }
  .card:hover { transform: translateY(-3px); border-color: rgba(var(--accent-rgb), .25); }
  .kpi-label { color: var(--text-muted); font-size: 12px; }
  .kpi-value { font-size: 26px; font-weight: 700; font-variant-numeric: tabular-nums; line-height: 1.25; }
</style>
<div class="card">
  <div class="kpi-label">今日开机</div>
  <div class="kpi-value">3</div>
</div>
```
