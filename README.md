<p align="center">
  <img src="frontend/public/icons/icon.png" alt="BootTracker Logo" width="96" height="96" />
</p>

<h1 align="center">BootTracker</h1>

<p align="center">
  A lightweight Windows desktop app that automatically records every boot/shutdown,<br />
  with statistics, charts and a floating desktop widget.
</p>

<p align="center">
  <a href="https://github.com/TamHaoYuan/BootTracker/actions/workflows/ci.yml">
    <img src="https://github.com/TamHaoYuan/BootTracker/actions/workflows/ci.yml/badge.svg" alt="CI" />
  </a>
  <a href="./LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?logo=windows&logoColor=white" alt="Platform" />
  <img src="https://img.shields.io/badge/version-2.0.1-8b5cf6" alt="Version" />
</p>

<p align="center">
  <b>English</b> · <a href="./README.zh-CN.md">简体中文</a>
</p>

---

## Features

### Core
- **Automatic boot/shutdown tracking** — a session is recorded on startup and closed on shutdown; stale sessions are auto-closed on next launch
- **SQLite storage** — all records live in a single `boot-data.db` file
- **Data export** — CSV / JSON / XLSX
- **Recycle bin** — deleted records are soft-deleted, restorable or permanently removable
- **Automatic backups** — created on startup, up to 30 retained

### Dashboard & Charts
- **KPI cards** — today's boots / usage duration / 7-day daily average / total records, with dual-encoded trends (arrow + semantic color, colorblind-safe)
- **14-day trend chart** and a structured recent-sessions table
- **Records page** — table view and timeline view (grouped by day, running session highlighted)
- **Charts page** — bar / line / pie trends plus a GitHub-style boot heatmap

### UI & Theming
- **Tauri v2 native window** with immersive dark titlebar (falls back to the system browser)
- **7 accent themes** — Purple / Blue / Green / Orange / Gray / Mica (translucent, Windows-native) / Material You, each with dark & light modes
- **Remembered preferences** — theme, sidebar state and view modes persist across sessions
- **Non-linear motion system** — spring & expo easing, splash screen respects `prefers-reduced-motion`

### Desktop Integration
- **Rust system tray** — unified tray with window/widget controls and app exit
- **Rust desktop widget** — frameless, translucent, draggable card showing today's boot count; follows the main app's theme and dark/light mode live
- **Auto-start**, **idle auto-close**, and a **watchdog** that self-heals crashed window/widget processes

### Remote Access
- **Cloudflare Tunnel** built in — reach your dashboard from anywhere with a custom domain, no public IP needed
- **LAN access** toggle

## Tech Stack

| Layer | Technology |
|---|---|
| Backend | Python 3 stdlib `http.server`, modular route registry, SQLite |
| Frontend | Vite + React 18 + TypeScript + Ant Design 5 + Zustand |
| Desktop shell | Tauri v2 (Rust) — window, tray, watchdog |
| Desktop widget | Rust + Tauri v2 (separate binary, theme-synced) |
| Tunnel | cloudflared |
| Logging | stdlib `logging`, daily rotation under `logs/` |
| Testing | pytest (backend) · Vitest (frontend) |
| Packaging | PyInstaller (onedir) + Inno Setup installer |
| CI/CD | GitHub Actions — typecheck, tests, builds |

## Quick Start

### Option A — Installer (recommended)

Download `BootTracker-Setup-x.y.z.exe` from [Releases](https://github.com/TamHaoYuan/BootTracker/releases), run it, done. User-level install (no admin required).

### Option B — From source

**Requirements:** Windows 10/11 · Python 3.10+ · Node.js 20+ · Rust toolchain (for Tauri window & widget)

```bash
# 1. Python dependencies
pip install -r requirements.txt

# 2. Frontend dependencies + build (outputs to dist-static/)
cd frontend && npm install && npm run build && cd ..

# 3. Run (starts backend + Tauri window; falls back to browser)
python boot-tracker.py
```

> Tunnel feature requires `vendor\cloudflared.exe` (download from the [cloudflared releases](https://github.com/cloudflare/cloudflared/releases)).

### Development

```bash
# Backend + Vite dev server with HMR (add --tauri for the native window)
scripts\dev.bat
```

- Backend: `http://127.0.0.1:18792/`
- Vite HMR: `http://localhost:5173/` (proxies `/api` to backend)

**Debug mode** — launch without recording a boot session (keeps real data clean):

```bash
python boot-tracker.py --debug     # or set BOOTTRACKER_DEBUG=1
```

### Tests

```bash
pytest tests -q                    # backend
cd frontend && npx vitest run      # frontend
```

## Build & Packaging

```bash
scripts\build.bat
```

Runs: frontend build → Tauri release build → widget release build → icon copy → PyInstaller.
Output: `dist/BootTracker/开机记录.exe` (onedir).

Then build the installer with the bundled Inno Setup:

```bash
tools\inno-setup\ISCC.exe packaging\boot-tracker.iss
# → packaging/installer_output/BootTracker-Setup-x.y.z.exe
```

## Project Structure

```
BootTracker/
├── boot-tracker.py        # App entry (instance lock → session → backup → server → window → watchdog)
├── boot-tracker.spec      # PyInstaller config (onedir)
├── server/                # Python backend package
│   ├── config.py          # Ports, paths, version
│   ├── data_store.py      # SQLite storage (sessions / trash / backups)
│   ├── http_handler.py    # HTTP server + static file serving
│   ├── settings.py        # settings.json + autostart
│   ├── tauri_window.py    # Tauri window controller
│   ├── tray.py            # pystray fallback tray (browser mode only)
│   ├── widget.py          # Rust widget process manager
│   ├── tunnel.py          # Cloudflare Tunnel
│   └── routes/            # Modular API routes (data/trash/settings/stats/backup/version/tunnel/window)
├── frontend/              # Vite + React + TS + AntD source
│   └── src/{api,bridge,components,hooks,layouts,pages,stores,styles}
├── tauri-app/             # Tauri v2 desktop shell (Rust): window + tray + watchdog
├── tauri-widget/          # Rust desktop widget (+ widget-src React frontend)
├── tests/                 # pytest backend tests
├── scripts/               # dev.bat / build.bat / tunnel.bat / switch-dpi.bat
├── packaging/             # installer.py / installer.spec / boot-tracker.iss / installer-lang / installer_output
├── vendor/                # Runtime binaries (cloudflared.exe, 7z.exe) — not in git
├── tools/                 # Local build tools (Inno Setup, WebView2 bootstrapper)
├── docs/                  # Design docs & plans
└── static/                # Icons + uploaded background images
```

## API Overview

All endpoints are served on `http://127.0.0.1:18792`.

| Method | Path | Description |
|---|---|---|
| GET | `/api/data` | All boot records |
| POST | `/api/add-session` | Add a session manually |
| PUT / DELETE | `/api/sessions/{id}` | Update / soft-delete a session |
| POST | `/api/merge-sessions` | Merge adjacent sessions |
| GET / POST | `/api/trash` · `/api/trash/restore` | Recycle bin / restore |
| GET / PUT | `/api/settings` | Read / update settings (also starts/stops the tunnel) |
| GET | `/api/stats/{overview,daily,weekly,trend,anomalies}` | Statistics & analysis |
| GET / POST | `/api/backups` · `/api/backup/restore` | List / restore backups |
| GET | `/api/version` · POST `/api/version/bump` | Version info / bump |
| GET | `/api/tunnel-url` | Current tunnel URL |
| POST | `/api/raise-window` · `/api/stop` · `/api/restart-server` | App & window control |
| POST | `/api/widget-toggle` · `/api/upload-bg` | Toggle widget / upload background |

## Configuration

Data files live next to the executable:

| File | Purpose |
|---|---|
| `boot-data.db` | SQLite database (sessions + trash) |
| `settings.json` | User settings |
| `version.json` | Version history |
| `logs/` | Daily-rotated logs |
| `backup/` | Automatic backups (max 30) |

Key settings (`settings.json`):

| Key | Default | Description |
|---|---|---|
| `autoStart` | `true` | Launch on Windows startup |
| `autoBackup` | `true` | Backup on startup |
| `lanAccess` | `true` | Bind 0.0.0.0 (LAN reachable) |
| `widgetEnabled` | `false` | Desktop widget |
| `appMode` / `appTheme` | `dark` / `mica` | Dark-light mode / accent theme |
| `tunnelEnabled` | `false` | Cloudflare Tunnel |
| `autoCloseIdle` | `false` | Close when idle (`idleCloseMinutes`) |

## License

[MIT](./LICENSE)
