@echo off
chcp 65001 >nul
rem BootTracker - debug mode (shows console with errors)

set "SCRIPT_DIR=%~dp0"
set "LAUNCHER=%SCRIPT_DIR%boot-tracker.py"

if not exist "%LAUNCHER%" (
    echo [ERROR] boot-tracker.py not found in: %SCRIPT_DIR%
    pause
    exit /b 1
)

C:\Python313\python.exe "%LAUNCHER%"
if errorlevel 1 pause
