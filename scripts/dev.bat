@echo off
REM BootTracker 一键开发启动脚本
REM 并行启动：1) Python 后端（boot-tracker.py）  2) Vite dev server（前端独立开发，代理 /api 到后端）
REM
REM 后端默认端口与 server/config.py 一致：18792
REM 前端 dev server 默认端口：5173（vite.config.ts 中配置）
REM
REM 用法：dev.bat [--tauri]  添加 --tauri 使用 Tauri 开发模式

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

REM 解析参数
set "TAURI_MODE=0"
if "%~1"=="--tauri" set "TAURI_MODE=1"

echo [1/2] starting BootTracker backend (HTTP on 18792) ...
if "%TAURI_MODE%"=="1" (
    echo       Tauri dev mode enabled
    set "BOOTTRACKER_DEV_MODE=1"
)
start "BootTracker-backend" cmd /k "%PYTHON_EXE% boot-tracker.py"

REM 给后端 3s 启动时间（HTTP 服务绑定端口）
timeout /t 3 /nobreak >nul

echo [2/2] starting Vite dev server (5173, proxy /api -> 18792) ...
cd frontend
start "BootTracker-frontend" cmd /k "npm run dev"
cd ..

echo.
echo Dev servers launched:
echo   - backend (Tauri/Qt/browser): http://127.0.0.1:18792/
echo   - frontend Vite with HMR:     http://localhost:5173/
echo.
if "%TAURI_MODE%"=="1" echo   Tauri window will open automatically.
endlocal
