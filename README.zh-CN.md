<p align="center">
  <img src="frontend/public/icons/icon.png" alt="BootTracker Logo" width="96" height="96" />
</p>

<h1 align="center">开机记录 (BootTracker)</h1>

<p align="center">
  轻量级 Windows 桌面应用，自动记录每次开机/关机时间，<br />
  提供统计分析、图表可视化与桌面浮动小组件。
</p>

<p align="center">
  <a href="https://github.com/TamHaoYuan/BootTracker/actions/workflows/ci.yml">
    <img src="https://github.com/TamHaoYuan/BootTracker/actions/workflows/ci.yml/badge.svg" alt="CI" />
  </a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?logo=windows&logoColor=white" alt="Platform" />
  <img src="https://img.shields.io/badge/version-2.1.0-8b5cf6" alt="Version" />
</p>

<p align="center">
  <a href="./README.md">English</a> · <b>简体中文</b>
</p>

---

## 功能特性

### 核心功能
- **自动记录开关机** — 启动即记录本次开机，关机自动结束会话；下次启动自动关闭遗留的未结束会话
- **SQLite 存储** — 全部记录存放于单一 `boot-data.db` 文件
- **数据导出** — 支持 CSV / JSON / XLSX 三种格式
- **回收站** — 删除的记录先入回收站，可恢复或彻底清除
- **自动备份** — 启动时自动备份，最多保留 30 份

### 仪表盘与图表
- **KPI 指标卡** — 今日开机 / 使用时长 / 7 日日均 / 累计记录，趋势采用双编码（箭头 + 语义色，色弱可读）
- **近 14 日趋势图** 与结构化最近会话表
- **开机记录页** — 表格 / 时间轴双视图（按日分组，进行中会话高亮呼吸圆点）
- **图表页** — 柱状 / 折线 / 饼图趋势 + 类 GitHub 贡献热力图

### 界面与主题
- **纯原生 Rust 窗口**（egui/eframe，GPU 渲染）——无 WebView、无 HTML、无内嵌浏览器
- **7 套主题色** — 紫 / 蓝 / 绿 / 橙 / 灰 / Mica 云母（半透明原生材质）/ Material You，均支持深浅色模式
- **记住用户设置** — 主题、侧栏折叠、视图模式跨会话持久化
- **非线性动画系统** — spring / expo 缓动；启动动画遵循系统「减少动态效果」偏好

### 桌面集成
- **Rust 系统托盘** — 统一托盘，控制窗口 / 小组件 / 退出
- **Rust 桌面小组件** — 无边框半透明可拖动卡片，显示今日开机次数；实时跟随主应用主题与深浅色
- **开机自启**、**空闲自动关闭**、**看门狗自愈**（窗口 / 小组件进程崩溃自动重启）

### 远程访问
- **内置 Cloudflare Tunnel** — 支持自定义域名，无需公网 IP 即可远程访问
- **局域网访问**开关

## 技术栈

| 层级 | 技术 |
|---|---|
| 后端 | Python 3 标准库 `http.server`，模块化路由注册表，SQLite |
| 网页前端（浏览器） | Vite + React 18 + TypeScript + Ant Design 5 + Zustand（产物 dist-static/） |
| 桌面主程序（App） | 纯原生 Rust — egui + eframe（`desktop/`，wgpu 直接渲染，非网页、非 WebView） |
| 桌面小组件 | 纯原生 Rust — egui + eframe（`desktop-widget/`，无边框置顶半透明浮窗） |
| 桌面与网页 | 两者彻底分离：各自独立调用后端 REST API，互不依赖 |
| 隧道 | cloudflared |
| 日志 | 标准 `logging`，`logs/` 目录每日轮转 |
| 测试 | pytest（后端）· Vitest（前端） |
| 打包 | PyInstaller（onedir）+ Inno Setup 安装程序 |
| CI/CD | GitHub Actions — 类型检查、测试、构建 |

## 快速开始

### 方式一：安装包（推荐）

从 [Releases](https://github.com/TamHaoYuan/BootTracker/releases) 下载 `BootTracker-Setup-x.y.z.exe` 运行即可。用户级安装，无需管理员权限。

### 方式二：源码运行

**环境要求：** Windows 10/11 · Python 3.10+ · Node.js 20+ · Rust 工具链（原生桌面程序 + 小组件）

```bash
# 1. Python 依赖
pip install -r requirements.txt

# 2. 网页前端依赖 + 构建（浏览器访问，输出到 dist-static/）
cd frontend && npm install && npm run build && cd ..

# 3. 原生桌面程序与浮窗小组件（纯 Rust，无需 wasm32 / trunk）
cd desktop && cargo build --release && cd ..
cd desktop-widget && cargo build --release && cd ..

# 4. 运行（启动后端 + 原生窗口）
python boot-tracker.py
```

> 隧道功能需要 `vendor\cloudflared.exe`（从 [cloudflared releases](https://github.com/cloudflare/cloudflared/releases) 下载）。

### 开发模式

```bash
# 后端 + Vite dev server（HMR），加 --desktop 启用纯原生窗口
scripts\dev.bat
```

- 后端 HTTP：`http://127.0.0.1:18792/`
- Vite HMR：`http://localhost:5173/`（代理 `/api` → 后端）

**调试模式** — 启动但不记录开机会话，避免调试污染真实数据：

```bash
python boot-tracker.py --debug     # 或设置环境变量 BOOTTRACKER_DEBUG=1
```

### 测试

```bash
pytest tests -q                    # 后端
cd frontend && npx vitest run      # 前端
```

## 构建与打包

```bash
scripts\build.bat
```

流程：网页前端构建 → 原生桌面程序 release → 原生小组件 release → 图标拷贝 → PyInstaller。
输出：`dist/BootTracker/开机记录.exe`（onedir 模式）。

再用内置 Inno Setup 编译安装程序：

```bash
tools\inno-setup\ISCC.exe packaging\boot-tracker.iss
# → packaging/installer_output/BootTracker-Setup-x.y.z.exe
```

## 项目结构

```
BootTracker/
├── boot-tracker.py        # 应用入口（实例锁 → 会话 → 备份 → 服务器 → 窗口 → 看门狗）
├── boot-tracker.spec      # PyInstaller 打包配置（onedir）
├── server/                # Python 后端服务包
│   ├── config.py          # 端口、路径、版本常量
│   ├── data_store.py      # SQLite 存储（会话 / 回收站 / 备份）
│   ├── http_handler.py    # HTTP 服务器 + 静态资源服务
│   ├── settings.py        # settings.json + 开机自启
│   ├── tauri_window.py    # 原生窗口控制器（启动 desktop/ 的 egui 主程序）
│   ├── tray.py            # pystray 回退托盘（仅浏览器模式）
│   ├── widget.py          # 原生小组件进程管理（desktop-widget/）
│   ├── tunnel.py          # Cloudflare Tunnel 隧道
│   └── routes/            # 模块化 API 路由（data/trash/settings/stats/backup/version/tunnel/window）
├── frontend/              # 网页前端源码（Vite + React + TS + AntD，浏览器端）
│   └── src/{api,bridge,components,hooks,layouts,pages,stores,styles}
├── desktop/               # 纯原生桌面主程序（Rust + egui/eframe）：窗口 + 托盘 + 全部页面
├── desktop-widget/        # 纯原生桌面浮窗小组件（Rust + egui/eframe）
├── tests/                 # pytest 后端测试
├── scripts/               # dev.bat / build.bat / tunnel.bat / switch-dpi.bat
├── packaging/             # installer.py / installer.spec / boot-tracker.iss / installer-lang / installer_output
├── vendor/                # 运行时二进制（cloudflared.exe、7z.exe）—— 不入库
├── tools/                 # 本地构建工具（Inno Setup）
├── docs/                  # 设计文档与方案
└── static/                # 图标 + 上传的背景图
```

## API 概览

所有端点服务于 `http://127.0.0.1:18792`。

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/data` | 获取全部开机记录 |
| POST | `/api/add-session` | 手动添加会话 |
| PUT / DELETE | `/api/sessions/{id}` | 更新 / 软删除会话 |
| POST | `/api/merge-sessions` | 合并相邻会话 |
| GET / POST | `/api/trash` · `/api/trash/restore` | 回收站 / 恢复 |
| GET / PUT | `/api/settings` | 读取 / 更新设置（同时控制隧道启停） |
| GET | `/api/stats/{overview,daily,weekly,trend,anomalies}` | 统计分析 |
| GET / POST | `/api/backups` · `/api/backup/restore` | 备份列表 / 恢复 |
| GET | `/api/version` · POST `/api/version/bump` | 版本信息 / 升级 |
| GET | `/api/tunnel-url` | 当前隧道地址 |
| POST | `/api/raise-window` · `/api/stop` · `/api/restart-server` | 应用与窗口控制 |
| POST | `/api/widget-toggle` · `/api/upload-bg` | 小组件开关 / 上传背景图 |

## 配置说明

数据文件位于可执行文件同目录：

| 文件 | 说明 |
|---|---|
| `boot-data.db` | SQLite 数据库（会话 + 回收站） |
| `settings.json` | 用户设置 |
| `version.json` | 版本历史 |
| `logs/` | 每日轮转日志 |
| `backup/` | 自动备份（最多 30 份） |

关键设置项（`settings.json`）：

| 键 | 默认 | 说明 |
|---|---|---|
| `autoStart` | `true` | 开机自启 |
| `autoBackup` | `true` | 启动时自动备份 |
| `lanAccess` | `true` | 绑定 0.0.0.0（局域网可访问） |
| `widgetEnabled` | `false` | 桌面小组件 |
| `appMode` / `appTheme` | `dark` / `mica` | 深浅色模式 / 主题色 |
| `tunnelEnabled` | `false` | Cloudflare 隧道 |
| `autoCloseIdle` | `false` | 空闲自动关闭（`idleCloseMinutes`） |

## 许可证

[MIT](./LICENSE)
