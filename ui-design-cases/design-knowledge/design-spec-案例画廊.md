# ui-design-cases 案例画廊 UI 设计解析

> 一句话定位：单页内容型画廊（案例库浏览页），克制专业的深色工具风，深色优先。
> 分析来源：`F:\BootTracker\ui-design-cases\index.html`（单文件、内联 CSS、原生 JS）；分析日期：2026-09-25。
> 结构：单 HTML 文件 ≈ 260 行 CSS + 20 行 JS，无依赖、无图片资源。

## 1. 设计基调

- 产品类型与使用场景：**一次性浏览 + 长期回查**的资料库页面。用户行为以"扫读卡片 → 筛选 → 跳转外部链接"为主。
- 设计取向：全天候工具的暗色克制风——低刺激背景、无阴影、单强调色，让 18 张内容密集卡片不显拥挤。
- 品牌记忆点：**CSS 迷你风格预览**——每张卡片的"缩略图"不是截图，而是用纯 CSS 手工复刻目标产品视觉风格的小示意图（`.prev` 系列，如 `.p-linear` 复刻 Linear 的深色列表、`.p-stripe` 复刻 KPI 卡+表格）。零图片依赖、离线可用。

## 2. Design Tokens（精确值，摘自 `:root`）

### 色彩

| 令牌 | 值 | 用途 |
|---|---|---|
| --bg | `#0f1115` | 页面底色 |
| --panel | `#171a21` | 卡片/统计条/筛选按钮底色 |
| --panel2 | `#1e222b` | （已声明，当前未使用——死令牌） |
| --border | `#2a2f3a` | 全部分隔线与卡片描边 |
| --text | `#e8eaf0` | 主文字 |
| --muted | `#9aa3b2` | 次要文字（副标题、开发者名、统计说明） |
| --accent | `#5b8cff` | **仅可交互元素**：链接色、激活筛选按钮底、卡片 hover 描边 |
| --green | `#4ade80` | 价值信息（"可偷师"行） |
| --amber / --red / --purple | `#fbbf24` / `#f87171` / `#a78bfa` | 已声明、主 UI 未使用（语义色预留） |

卡片正文硬编码 `#c3cad6`（介于 --text 与 --muted 之间）——应提炼为 `--text-secondary` 令牌（见 §7 雷区）。

```css
/* 可直接复制到新项目（已清理死令牌） */
:root{
  --bg:#0f1115; --panel:#171a21; --border:#2a2f3a;
  --text:#e8eaf0; --text-2:#c3cad6; --muted:#9aa3b2;
  --accent:#5b8cff; --green:#4ade80;
}
```

### 字体排印

| 层级 | 字号/字重 | 用于 |
|---|---|---|
| 页面标题 | 26px / 700 / `letter-spacing:.5px` | h1 |
| 卡片标题 | 16px / 700 | .body h3 |
| 统计数字 | 16px / 700（正文 13px 搭配） | .stat b |
| 副标题/正文 | 14px / 正文色 | header p |
| 卡片正文 | 13px / 常规 / `#c3cad6` | .body p |
| 辅助信息 | 12px | .dev、.steal、链接、footer |
| 标签 | 11px | .tag（缩略图角标） |

- 字体栈：`"Segoe UI","PingFang SC","Microsoft YaHei",sans-serif` — 系统字体零加载
- 全局 `line-height:1.6`；层级靠**字重与颜色双轴**（700 主 / 常规次；--text 主 / --muted 次），无斜体无下划线

### 间距 / 圆角 / 阴影

- 间距基数：**非严格网格**，实际常用值 8 / 12 / 14 / 16 / 18 / 24 / 32 / 40px（卡片内边距 `14px 16px 16px`、网格 gap 18px、区块间 24–28px）
- 圆角层级：**小元素小圆角、大容器大圆角** —— 标签 12px、统计条/迷你预览 8px、卡片 12px、筛选按钮 20px（胶囊）
- 阴影：**全站零 box-shadow**，分层完全靠 `1px solid var(--border)` 描边 + 背景色差（--bg → --panel）
- 栅格：内容卡片 `repeat(auto-fill,minmax(330px,1fr))` 自适应栅格；页面无侧边栏，单列纵向流（header → stats → filters → grid → footer）

## 3. 布局系统

- 骨架：`body padding:32px 40px` 的单列文档流；无 max-width 限制（大屏下内容铺满——见 §7 雷区）
- 层级：`header(h1+p)` → `stats 徽章行(flex-wrap)` → `filters 胶囊行(flex-wrap)` → `grid 卡片` → `footer 来源说明`
- 缩略图区固定高度 `150px`，保证任意 CSS 预览内容下卡片头齐脚齐

## 4. 组件模式卡

### Stat 徽章（数据速览）
```css
.stat{background:var(--panel);border:1px solid var(--border);border-radius:8px;
  padding:8px 16px;font-size:13px;color:var(--muted)}
.stat b{color:var(--text);font-size:16px;margin-right:4px}
```
- 结构：`<b>数字</b>说明文字`，数字放大提亮、说明降级
- 使用时机：页面头部快速传达规模（"18 案例 / 3 维度"）

### Filter 胶囊按钮（页面唯一交互控件）
```css
.filters button{background:var(--panel);border:1px solid var(--border);color:var(--muted);
  padding:6px 14px;border-radius:20px;font-size:13px;cursor:pointer;transition:.15s}
.filters button:hover{color:var(--text)}
.filters button.active{background:var(--accent);border-color:var(--accent);color:#fff}
```
- 三态递进：默认 muted → hover 仅提亮文字（不动布局）→ 激活实底 accent 反白
- 使用时机：任何 4 类以内的维度筛选

### Card（核心内容单元）
```css
.card{background:var(--panel);border:1px solid var(--border);border-radius:12px;
  overflow:hidden;display:flex;flex-direction:column;transition:.15s}
.card:hover{border-color:var(--accent);transform:translateY(-2px)}
```
- 四段式结构：`.thumb`(150px 预览) → `.body`(h3 + .dev + p，flex:1 撑开) → `.steal`(虚线分隔的价值要点) → `.link`(外部跳转)
- hover 语法：**描边变色 + 2px 抬升**，无阴影变化
- `display:flex;flex-direction:column` + `.body{flex:1}` 保证多行正文下卡片底对齐

### Tag 角标（缩略图上的类别标识）
```css
.tag{position:absolute;top:10px;right:10px;font-size:11px;padding:3px 10px;
  border-radius:12px;background:rgba(0,0,0,.55);backdrop-filter:blur(4px);
  border:1px solid rgba(255,255,255,.12)}
```
- 半透明黑底 + 毛玻璃 + 白色微边框，任意底色上可读
- 使用时机：悬浮于图像/预览之上的分类标注

### Steal 行（虚线价值分隔）
```css
.steal{font-size:12px;color:var(--green);margin-top:10px;padding-top:10px;
  border-top:1px dashed var(--border)}
.steal::before{content:"⌘ 可偷师：";color:var(--muted)}
```
- `::before` 注入前缀标签（muted）+ 正文（green），一行内完成"标签-内容"层级
- dashed 而非 solid：视觉上区隔"元信息"与"正文"

### 迷你风格预览（本站独有组件）
```css
.prev{border-radius:8px;height:100%;width:100%;overflow:hidden;display:flex;font-size:9px}
.p-linear{background:#0d0e12;flex-direction:column;gap:6px;padding:10px}
.p-linear .row{background:#1a1c22;height:12px;border-radius:3px;...}
```
- 每个被分析产品一个 `.p-*` 类，用 10–20 行 CSS 复刻其核心视觉母题（Linear 的列表行、Stripe 的 KPI+表格、Headspace 的圆形吉祥物、Plausible 的柱状图）
- 使用时机：无版权图片、需要轻量示意产品风格差异的场景

## 5. 交互与动效规范

- 全站统一 `transition:.15s`（卡片与筛选按钮一致）——单一缓动语法
- hover 反馈两类：文字提亮（不改变布局）／描边变色+位移（卡片）
- 筛选逻辑为原生 JS `display:none` 切换，约 10 行，无动画——可接受但与 .15s 的语言略不一致（可优化为 opacity 过渡）
- 无加载/空状态（静态内容不需要，但筛选结果为空时无提示——小缺口）

## 6. 设计原则提炼（带代码证据）

1. **零阴影分层** — 证据：全文件无 `box-shadow`，`.card`/`.stat` 均为 `border:1px solid var(--border)`；意图：安静骨架，深色下阴影本就低效，描边+底色差更稳。
2. **色彩只表达语义** — 证据：`--accent` 仅出现在 `a.link`、`.filters button.active`、`.card:hover` 三处；`--green` 仅用于 `.steal`；意图：页面任何蓝色 = 可点，绿色 = 有干货，色彩即导航。
3. **卡片四段式渐进披露** — 证据：`.thumb → .body → .steal → .link` 结构；意图：视觉印象 → 一句话定位 → 可偷师要点 → 深入入口，每段回答一个问题。
4. **flex 纵列 + flex:1 保证栅格底对齐** — 证据：`.card{display:flex;flex-direction:column}`、`.body{flex:1}`；意图：正文长度不一时卡片仍整齐。
5. **CSS 复刻代替图片** — 证据：`.prev`/`.p-*` 系列共 ~90 行；意图：零资源依赖、离线、加载即渲染。

## 7. 复用指南

### 生成新网站时模仿
- [ ] 深色三层面板：`#0f1115 / #171a21 / 1px #2a2f3a 描边`，不写任何阴影
- [ ] 单强调色只上交互元素；信息价值用第二语义色（绿）点缀
- [ ] 卡片四段式：预览 → 标题+定位 → 价值点（虚线分隔+前缀注入）→ 链接
- [ ] 筛选胶囊三态：muted 默认 / hover 提亮 / 激活实底反白
- [ ] `auto-fill minmax(330px,1fr)` 自适应卡片栅格
- [ ] 系统字体栈 + 字重/颜色双轴排印（26/16/14/13/12/11px 阶梯）
- [ ] 悬浮角标：`rgba(0,0,0,.55)+blur(4px)+rgba(255,255,255,.12) 边框`

### 不要照搬（本站反模式与瑕疵）
- [ ] `--panel2/--amber/--red/--purple` 为死令牌——新项目先删未用 token
- [ ] `.body p` 硬编码 `#c3cad6`，应令牌化为 `--text-2`
- [ ] 无 `max-width`，4K 屏单列卡片栅格会无限拉伸——加 `max-width:1440px; margin:auto`
- [ ] 间距值 8/12/14/16/18 混用，未收敛到 4/8 网格
- [ ] 无 `:focus-visible` 样式，键盘可达性缺失；筛选无空结果提示

### 最小可用起步配置
```html
<style>
:root{--bg:#0f1115;--panel:#171a21;--border:#2a2f3a;--text:#e8eaf0;
  --text-2:#c3cad6;--muted:#9aa3b2;--accent:#5b8cff;--green:#4ade80}
*{margin:0;padding:0;box-sizing:border-box}
body{background:var(--bg);color:var(--text);font-family:"Segoe UI","PingFang SC",sans-serif;line-height:1.6}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(330px,1fr));gap:18px;max-width:1440px;margin:auto}
.card{background:var(--panel);border:1px solid var(--border);border-radius:12px;
  display:flex;flex-direction:column;transition:.15s}
.card:hover{border-color:var(--accent);transform:translateY(-2px)}
</style>
```
