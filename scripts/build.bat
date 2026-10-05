@echo off
chcp 65001 >nul
REM BootTracker 一键构建脚本
REM 执行：
REM   1) 网页前端 install+build（React → dist-static，仅供浏览器访问）
REM   2) 桌面前端 build（纯原生 Rust egui/eframe → desktop/target/release/boot-tracker.exe）
REM   3) 浮窗组件 build（纯原生 Rust → desktop-widget/target/release/boot-tracker-widget.exe）
REM   4) 图标拷贝
REM   5) PyInstaller 打包
REM 说明：vite.config.ts build.outDir 已指向 ../dist-static，无需手动拷贝。
REM       纯原生方案无 trunk / 无 wasm-opt / 无 WebView 依赖。

setlocal
cd /d "%~dp0.."

echo.
echo ============================================
echo   BootTracker Build - stage 1/5 web frontend
echo ============================================
cd frontend
if not exist node_modules (
  echo [1/5] installing web frontend dependencies...
  call npm ci --no-audit --no-fund
  if errorlevel 1 ( echo frontend install failed & exit /b 1 )
)
echo [1/5] building web frontend...
call npm run build
if errorlevel 1 ( echo frontend build failed & exit /b 1 )
cd ..

echo.
echo ============================================
echo   BootTracker Build - stage 2/5 desktop (native Rust)
echo ============================================
cd desktop
if exist Cargo.toml (
  echo [2/5] building native desktop (cargo build --release)...
  cargo build --release
  if errorlevel 1 ( echo desktop build failed & exit /b 1 )
  echo Desktop binary: desktop\target\release\boot-tracker.exe
) else (
  echo [2/5] skipping desktop build (Cargo.toml not found)
)
cd ..

echo.
echo ============================================
echo   BootTracker Build - stage 3/5 widget (native Rust)
echo ============================================
cd desktop-widget
if exist Cargo.toml (
  echo [3/5] building native widget (cargo build --release)...
  cargo build --release
  if errorlevel 1 ( echo widget build failed & exit /b 1 )
  echo Widget binary: desktop-widget\target\release\boot-tracker-widget.exe
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
