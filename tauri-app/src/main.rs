//! BootTracker 桌面应用入口
//!
//! 基于 Tauri v2 框架，替代原 PyQt5 WebEngine 窗口层。
//! 前端使用 React (Vite) 构建，后端 API 由 Python HTTP 服务器提供。

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    boot_tracker::run();
}
