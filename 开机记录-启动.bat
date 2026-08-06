@echo off
chcp 65001 >nul
rem BootTracker launcher - double-click: open window if running, or start app
rem Saved as ANSI/GBK compatible (no Chinese in commands)

set "SCRIPT_DIR=%~dp0"
set "LAUNCHER=%SCRIPT_DIR%boot-tracker.py"

if not exist "%LAUNCHER%" (
    echo [ERROR] boot-tracker.py not found in: %SCRIPT_DIR%
    pause
    exit /b 1
)

rem Try pythonw first (no console window), fall back to python
where pythonw >nul 2>&1
if %errorlevel%==0 (
    start "" pythonw "%LAUNCHER%"
    exit /b 0
)

rem Try common install paths
if exist "C:\Python313\pythonw.exe" (
    start "" "C:\Python313\pythonw.exe" "%LAUNCHER%"
    exit /b 0
)
if exist "C:\Python312\pythonw.exe" (
    start "" "C:\Python312\pythonw.exe" "%LAUNCHER%"
    exit /b 0
)

rem Final fallback: python (console flashes)
python "%LAUNCHER%"
