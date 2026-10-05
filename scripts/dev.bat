@echo off
chcp 65001 >nul
REM BootTracker 一键开发启动脚本
REM 并行启动：
REM   1) Python 后端（boot-tracker.py）
REM   2) 网页前端 dev server（Vite，5173，代理 /api 到后端）
REM   3) --desktop 模式额外启动纯原生桌面前端（cargo run，desktop/）
REM
REM 后端默认端口与 server/config.py 一致：18792
REM 桌面前端已改为纯原生 Rust（egui/eframe），无 WebView / 无 HTML。
REM
REM 用法：dev.bat [--desktop]   添加 --desktop 启动原生桌面前端（cargo run）

setlocal
cd /d "%~dp0.."

REM 检测 venv Python
set "PYTHON_EXE="
if exist ".venv\Scripts\python.exe" set "PYTHON_EXE=.venv\Scripts\python.exe"
if not defined PYTHON_EXE (
    echo [ERROR] .venv not found. Run: python -m venv .venv ^&^& .venv\Scripts\pip install -r requirements.txt
    pause
    exit /b 1
)

REM 解析参数（--tauri 兼容旧写法，统一指向原生桌面端）
set "DESKTOP_MODE=0"
if "%~1"=="--desktop" set "DESKTOP_MODE=1"
if "%~1"=="--tauri" set "DESKTOP_MODE=1"

echo [1/3] starting BootTracker backend (HTTP on 18792) ...
if "%DESKTOP_MODE%"=="1" (
    echo       native desktop dev mode enabled
    set "BOOTTRACKER_DEV_MODE=1"
)
start "BootTracker-backend" cmd /k "%PYTHON_EXE% boot-tracker.py --debug"

REM 给后端 3s 启动时间（HTTP 服务绑定端口）
timeout /t 3 /nobreak >nul

echo [2/3] starting web frontend dev server (5173, proxy /api -> 18792) ...
cd frontend
start "BootTracker-web-frontend" cmd /k "npm run dev"
cd ..

if "%DESKTOP_MODE%"=="1" (
    echo [3/3] starting native desktop (cargo run, desktop/) ...
    cd desktop
    start "BootTracker-desktop" cmd /k "cargo run"
    cd ..
) else (
    echo [3/3] native desktop skipped (add --desktop to enable)
)

echo.
echo Dev servers launched:
echo   - backend:                 http://127.0.0.1:18792/
echo   - web frontend (Vite/HMR): http://localhost:5173/
if "%DESKTOP_MODE%"=="1" echo   - native desktop:           cargo run (desktop/)
echo.
endlocal
