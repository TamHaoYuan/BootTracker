# 开机记录 (BootTracker)

> 轻量级 Windows 桌面应用，自动记录每次开机/关机时间，提供统计分析、图表可视化与桌面浮动组件。

版本 `1.0.0`  ·  端口 `18792`  ·  Python `3.10+`

---

## 功能特性

### 核心功能
- **自动记录开关机** — 启动即记录本次开机时间，支持手动/自动记录关机，自动关闭未结束会话
- **会话时长统计** — 计算每次开机到关机的运行时长
- **数据导出** — 支持 CSV / JSON / XLSX 三种格式
- **回收站** — 删除的记录进入回收站，可恢复或彻底清除
- **自动备份** — 启动时自动备份，最多保留 30 份

### 图表与统计
- **趋势图表** — 柱状图 / 折线图 / 饼图，支持切换
- **开机热度图** — 类 GitHub 贡献热力图，直观展示开机频率
- **统计分析** — 每日/每周指标、趋势分析、异常检测

### 界面与主题
- **PyQt5 WebEngine 主窗口** — 原生窗口体验，沉浸式暗色标题栏
- **多主题** — 紫/蓝/绿/橙/灰/Mica/Material You 等主题色
- **Dark Gallery 暗色模式** — 深炭黑背景、高对比文字、各主题专属暗色
- **过渡动画** — 卡片、按钮、图表的流畅过渡与入场动画

### 桌面集成
- **系统托盘** — pystray 托盘图标带开机进度环，菜单支持显示窗口/退出
- **桌面小组件** — Tkinter 无边框半透明浮动卡片，可拖动、位置记忆、右键菜单
- **开机自启** — 可选开机自动启动
- **空闲检测** — 超时空闲自动关闭

### 远程访问
- **Cloudflare Tunnel** — 内置隧道功能，支持自定义域名，无需公网 IP 即可远程访问
- **局域网访问** — 可选开启 LAN 访问

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 后端 | Python 3 标准库 `http.server`，模块化路由架构 |
| 前端 | 原生 HTML / CSS / JS（无框架），SVG 图表 |
| 主窗口 | PyQt5 WebEngine |
| 桌面组件 | Tkinter |
| 系统托盘 | pystray |
| 隧道 | cloudflared |
| 日志 | 标准 `logging` 模块，每日轮转 |
| 打包 | PyInstaller + 自定义安装器（7z 解压） |

---

## 项目结构

```
BootTracker/
├── boot-tracker.py            # 应用入口（实例锁 → 会话 → 备份 → 服务器 → 窗口 → 托盘）
├── index.html                 # 主界面 HTML
├── icon.ico                   # 应用图标
├── installer.py               # 自定义安装器（tkinter 界面 + 7z 解压）
├── installer.spec             # 安装器 PyInstaller 配置
├── boot-tracker.spec          # 应用 PyInstaller 配置
├── 开机记录-启动.bat           # 启动脚本（pythonw 优先，无控制台）
├── 开机记录-调试.bat           # 调试启动脚本
│
├── server/                    # 后端服务包
│   ├── __init__.py            # 模块导出
│   ├── config.py              # 配置常量（端口、路径、版本）
│   ├── logging_config.py      # 日志配置（每日轮转 + 控制台）
│   ├── data_store.py          # 数据存储（记录/回收站/备份/实例锁）
│   ├── http_handler.py        # HTTP 服务器
│   ├── settings.py            # 设置管理 + 开机自启
│   ├── version.py             # 版本管理 + 更新检查
│   ├── qt_window.py           # PyQt5 WebEngine 主窗口
│   ├── tray.py                # pystray 系统托盘
│   ├── widget.py              # Tkinter 桌面小组件
│   ├── tunnel.py              # Cloudflare Tunnel 隧道
│   └── routes/                # 模块化 API 路由
│       ├── __init__.py        # 路由注册表 + 装饰器
│       ├── data_routes.py      # 数据 CRUD
│       ├── trash_routes.py     # 回收站
│       ├── settings_routes.py  # 设置
│       ├── stats_routes.py     # 统计分析
│       ├── backup_routes.py    # 备份
│       ├── version_routes.py  # 版本
│       ├── tunnel_routes.py   # 隧道
│       └── window_routes.py   # 窗口控制
│
├── static/                    # 前端静态资源
│   ├── css/
│   │   ├── style.css          # 主样式（主题/暗色/动画/组件）
│   │   └── utils.css          # 工具样式
│   ├── js/
│   │   └── app.js             # 前端逻辑
│   └── icons/
│       └── icons.svg          # SVG 图标集
│
└── tools/                     # 工具（Inno Setup 等）
```

---

## 快速开始

### 环境要求
- Windows 10/11
- Python 3.10+

### 安装依赖

```bash
pip install PyQt5 pystray
```

> 隧道功能需要 `cloudflared.exe`（放置于项目根目录）

### 运行

**方式一：启动脚本（推荐）**

双击 `开机记录-启动.bat`，自动用 `pythonw` 启动（无控制台窗口）。

**方式二：命令行**

```bash
python boot-tracker.py
```

启动后：
- 主界面以 PyQt5 窗口打开（加载 `http://127.0.0.1:18792/`）
- 系统托盘显示带进度环的图标
- 数据文件 `boot-data.json` 自动创建于运行目录

---

## 打包

### 打包应用本体

```bash
pyinstaller boot-tracker.spec --noconfirm
```

输出：`dist/BootTracker/开机记录.exe`（onedir 模式）

### 打包安装程序

```bash
# 1. 生成 payload（需 7z.exe 在项目根目录）
7z a payload.7z * -mx=9    # 在 dist/BootTracker 目录内执行

# 2. 打包安装器
pyinstaller installer.spec --noconfirm
```

输出：`dist/BootTracker-Setup.exe`（单文件自包含安装包）

安装器功能：自定义安装目录、桌面快捷方式、开机自启、卸载注册。

---

## API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/data` | 获取开机记录 |
| POST | `/api/data` | 添加/更新记录 |
| DELETE | `/api/data` | 删除记录（进回收站） |
| GET | `/api/trash` | 获取回收站数据 |
| POST | `/api/trash/restore` | 恢复回收站记录 |
| GET | `/api/settings` | 获取设置 |
| PUT | `/api/settings` | 更新设置 |
| GET | `/api/stats/overview` | 统计概览 |
| GET | `/api/stats/daily` | 每日趋势 |
| GET | `/api/stats/weekly` | 每周趋势 |
| GET | `/api/stats/anomalies` | 异常检测 |
| GET | `/api/backup/list` | 备份列表 |
| POST | `/api/backup/create` | 创建备份 |
| POST | `/api/backup/restore` | 恢复备份 |
| GET | `/api/version` | 版本信息 |
| POST | `/api/version/bump` | 版本升级 |
| GET | `/api/tunnel/status` | 隧道状态 |
| POST | `/api/tunnel/start` | 启动隧道 |
| POST | `/api/tunnel/stop` | 停止隧道 |
| POST | `/api/raise-window` | 唤起主窗口 |
| POST | `/api/window/theme` | 切换窗口主题 |
| POST | `/api/stop` | 停止应用 |

---

## 配置说明

### 数据文件（运行目录下）
| 文件 | 说明 |
|------|------|
| `boot-data.json` | 开机记录数据 |
| `trash-data.json` | 回收站数据 |
| `settings.json` | 用户设置 |
| `version.json` | 版本信息 |
| `boot-tracker.log` | 运行日志（每日轮转） |
| `backup/` | 自动备份目录（最多 30 份） |

### 关键设置项（`settings.json`）
| 键 | 默认 | 说明 |
|----|------|------|
| `autoStart` | `true` | 开机自启 |
| `autoBackup` | `true` | 自动备份 |
| `lanAccess` | `true` | 局域网访问 |
| `widgetEnabled` | `false` | 桌面小组件 |
| `tunnelEnabled` | `false` | Cloudflare 隧道 |
| `customDomain` | `""` | 隧道自定义域名 |
| `autoCloseIdle` | `false` | 空闲自动关闭 |
| `idleCloseMinutes` | `5` | 空闲超时分钟数 |

---

## 开发规范

- Python 文件统一头部：`#!/usr/bin/env python3` + `# -*- coding: utf-8 -*-` + 模块文档字符串
- 导入分组：标准库 → 第三方 → 内部模块
- 路由使用装饰器注册（`@get` / `@post` / `@put` / `@delete`）
- CSS 变量集中管理于 `:root`，按功能分类（`--bg-*` / `--text-*` 等）
- CSS 按 `/* ========== 标题 ========== */` 分隔组织
- 跨线程通信使用 `pyqtSignal`（禁用 `QTimer.singleShot` 跨线程）
- 日志使用标准 `logging` 模块

---

## 许可证

MIT
