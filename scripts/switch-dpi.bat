@echo off
chcp 65001 >nul 2>&1
setlocal enabledelayedexpansion

:: Windows DPI Switch Tool - Quick switch display scaling
:: Usage: switch-dpi.bat [100|125|150|175|200|list]

set "SCALE=%~1"

if "%SCALE%"=="list" goto :list
if "%SCALE%"=="" goto :menu

:: Direct mode
for %%s in (100 125 150 175 200) do (
    if "%SCALE%"=="%%s" set "VALID=1"
)
if not defined VALID (
    echo Invalid scale: %SCALE%
    echo Use: 100 125 150 175 200
    exit /b 1
)

:: Get current DPI
for /f "tokens=3" %%a in ('reg query "HKCU\Control Panel\Desktop" /v LogPixels 2^>nul ^| findstr LogPixels') do set "CURRENT_DPI=%%a"
if not defined CURRENT_DPI set "CURRENT_DPI=96"

:: Scale -> DPI map
if "%SCALE%"=="100" set "TARGET_DPI=96"
if "%SCALE%"=="125" set "TARGET_DPI=120"
if "%SCALE%"=="150" set "TARGET_DPI=144"
if "%SCALE%"=="175" set "TARGET_DPI=168"
if "%SCALE%"=="200" set "TARGET_DPI=192"

if "%TARGET_DPI%"=="%CURRENT_DPI%" (
    echo.
    echo Already at %SCALE%%, no change needed.
    exit /b 0
)

:: Apply
reg add "HKCU\Control Panel\Desktop" /v LogPixels /t REG_DWORD /d %TARGET_DPI% /f >nul 2>&1
reg add "HKCU\Control Panel\Desktop" /v DpiScalingVer /t REG_DWORD /d 0x400 /f >nul 2>&1

echo.
echo Set scaling: %SCALE%% ^(DPI=%TARGET_DPI%^)
echo Logoff or restart required to take effect.
echo.
set /p CONFIRM="Logoff now? (y/N): "
if /i "%CONFIRM%"=="y" (
    echo Logging off...
    timeout /t 1 /nobreak >nul
    logoff /force
)
exit /b 0

:list
:: Show current status
for /f "tokens=3" %%a in ('reg query "HKCU\Control Panel\Desktop" /v LogPixels 2^>nul ^| findstr LogPixels') do set "CUR=%%a"
if not defined CUR set "CUR=96"

echo.
if "%CUR%"=="96"     echo Current: 100%% (DPI=96)
if "%CUR%"=="120"    echo Current: 125%% (DPI=120)
if "%CUR%"=="144"    echo Current: 150%% (DPI=144)
if "%CUR%"=="168"    echo Current: 175%% (DPI=168)
if "%CUR%"=="192"    echo Current: 200%% (DPI=192)
echo.
echo Available options:
echo   [1] 100%%  (DPI=96)
echo   [2] 125%%  (DPI=120)
echo   [3] 150%%  (DPI=144)
echo   [4] 175%%  (DPI=168)
echo   [5] 200%%  (DPI=192)
echo.
echo Usage: switch-dpi.bat ^<percentage^>
echo        switch-dpi.bat           ^(interactive^)
exit /b 0

:menu
:: Interactive menu
for /f "tokens=3" %%a in ('reg query "HKCU\Control Panel\Desktop" /v LogPixels 2^>nul ^| findstr LogPixels') do set "CUR=%%a"
if not defined CUR set "CUR=96"

if "%CUR%"=="96"     set "CUR_S=100"
if "%CUR%"=="120"    set "CUR_S=125"
if "%CUR%"=="144"    set "CUR_S=150"
if "%CUR%"=="168"    set "CUR_S=175"
if "%CUR%"=="192"    set "CUR_S=200"

echo.
echo ================================
echo  Windows DPI Switch Tool
echo ================================
echo.
echo Current: %CUR_S%%% (DPI=%CUR%)
echo.
echo   [1] 100%%
echo   [2] 125%%
echo   [3] 150%%
echo   [4] 175%%
echo   [5] 200%%
echo   [0] Exit
echo.
set /p CHOICE="Select: "

if "%CHOICE%"=="1" set "SEL=100"  & set "TGT=96"
if "%CHOICE%"=="2" set "SEL=125"  & set "TGT=120"
if "%CHOICE%"=="3" set "SEL=150"  & set "TGT=144"
if "%CHOICE%"=="4" set "SEL=175"  & set "TGT=168"
if "%CHOICE%"=="5" set "SEL=200"  & set "TGT=192"
if "%CHOICE%"=="0" exit /b 0
if not defined SEL exit /b 0

if "%TGT%"=="%CUR%" (
    echo.
    echo Already %SEL%%%.
    exit /b 0
)

reg add "HKCU\Control Panel\Desktop" /v LogPixels /t REG_DWORD /d %TGT% /f >nul 2>&1
reg add "HKCU\Control Panel\Desktop" /v DpiScalingVer /t REG_DWORD /d 0x400 /f >nul 2>&1

echo.
echo Done: %CUR_S%%% --^> %SEL%%% (DPI=%TGT%)
echo Logoff or restart required.
