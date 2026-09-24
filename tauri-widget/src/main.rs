//! BootTracker 桌面小组件入口
//!
//! 无边框置顶小窗口，替代旧版 Tkinter 组件（deprecated/widget_tkinter.py）。

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    boot_tracker_widget::run();
}
