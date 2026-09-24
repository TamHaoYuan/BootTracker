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

REM Use venv Python
set "PYTHON_EXE=%SCRIPT_DIR%.venv\Scripts\python.exe"
if exist "%PYTHON_EXE%" (
    "%PYTHON_EXE%" "%LAUNCHER%"
) else (
    echo [WARN] .venv not found, using system Python
    python "%LAUNCHER%"
)
if errorlevel 1 pause