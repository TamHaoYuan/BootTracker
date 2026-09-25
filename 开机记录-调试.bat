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

REM Use venv Python; --debug: do NOT record boot session
set "PYTHON_EXE=%SCRIPT_DIR%.venv\Scripts\python.exe"
if exist "%PYTHON_EXE%" (
    "%PYTHON_EXE%" "%LAUNCHER%" --debug
) else (
    echo [WARN] .venv not found, using system Python
    python "%LAUNCHER%" --debug
)
if errorlevel 1 pause