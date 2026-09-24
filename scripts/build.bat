@echo off
REM BootTracker 一键构建脚本
REM 执行：1) 前端 install+build  2) Tauri release build  3) Rust 小组件 build  4) 图标拷贝  5) PyInstaller 打包
REM 说明：vite.config.ts build.outDir 已指向 ../dist-static，无需手动拷贝。

setlocal
cd /d "%~dp0.."

echo.
echo ============================================
echo   BootTracker Build - stage 1/5 frontend
echo ============================================
cd frontend
if not exist node_modules (
  echo [1/5] installing frontend dependencies...
  call npm ci --no-audit --no-fund
  if errorlevel 1 ( echo frontend install failed & exit /b 1 )
)
echo [1/5] building frontend...
call npm run build
if errorlevel 1 ( echo frontend build failed & exit /b 1 )
cd ..

echo.
echo ============================================
echo   BootTracker Build - stage 2/5 tauri
echo ============================================
cd tauri-app
if exist Cargo.toml (
  echo [2/5] building Tauri release...
  cargo build --release
  if errorlevel 1 ( echo tauri build failed & exit /b 1 )
  echo Tauri binary: tauri-app\target\release\boot-tracker.exe
) else (
  echo [2/5] skipping Tauri build (Cargo.toml not found)
)
cd ..

echo.
echo ============================================
echo   BootTracker Build - stage 3/5 widget
echo ============================================
cd tauri-widget
if exist Cargo.toml (
  where cargo >nul 2>nul
  if errorlevel 1 (
    echo [3/5] skipping widget build (cargo not found in PATH; install Rust toolchain via https://rustup.rs)
    if not exist target\release\boot-tracker-widget.exe (
      echo WARNING: prebuilt widget binary missing; onedir bundle WITHOUT widget fallback to Tkinter!
    ) else (
      echo Widget binary (prebuilt): tauri-widget\target\release\boot-tracker-widget.exe
    )
  ) else (
    echo [3/5] building Rust widget release...
    cargo build --release --manifest-path Cargo.toml
    if errorlevel 1 ( echo widget build failed & exit /b 1 )
    echo Widget binary: tauri-widget\target\release\boot-tracker-widget.exe
  )
) else (
  echo [3/5] skipping widget build (Cargo.toml not found)
)
cd ..

echo.
echo ============================================
echo   BootTracker Build - stage 4/5 copy icons
echo ============================================
REM static/icons 可能在后续页面中通过 SVG <use> 引用；保留一份到 dist-static/icons
if exist static\icons (
  if not exist dist-static\icons mkdir dist-static\icons
  xcopy /E /I /Y static\icons dist-static\icons >nul
  echo static/icons copied to dist-static/icons
)

echo.
echo ============================================
echo   BootTracker Build - stage 5/5 pyinstaller
echo ============================================
pyinstaller boot-tracker.spec --noconfirm
if errorlevel 1 ( echo pyinstaller build failed & exit /b 1 )

echo.
echo Build done. Output at dist\BootTracker\
endlocal
