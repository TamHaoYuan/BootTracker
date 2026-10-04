//! BootTracker Tauri 应用
//!
//! 替代原 PyQt5 WebEngine 窗口层，提供：
//! - 主窗口创建与管理
//! - 原生 IPC 命令（替代 QWebChannel）
//! - 窗口关闭拦截（隐藏到托盘而非退出）
//! - DWM 标题栏主题控制
//! - 系统托盘（显示窗口 / 桌面小组件开关 / 退出）

mod commands;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    webview::PageLoadEvent,
    Emitter, Manager, RunEvent, WindowEvent,
};

/// 后端 API 基址：Python 启动时注入 BOOTTRACKER_PORT；
/// 开发模式未注入时用默认端口
fn api_base() -> String {
    let port = std::env::var("BOOTTRACKER_PORT").unwrap_or_else(|_| "18792".into());
    format!("http://127.0.0.1:{port}")
}

/// 应用入口：构建并运行 Tauri 应用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::set_theme,
            commands::raise_window,
            commands::quit_app,
            commands::pick_file,
            commands::get_app_info,
        ])
        // 页面加载完成后再显示窗口，消除“白屏一闪 → splash”观感。
        // 桌面前端为内嵌的纯 Rust/WASM（ui/dist），不再导航到后端同源页面；
        // API 由前端跨源请求 http://127.0.0.1:{port}/api/*（后端已全通 CORS），
        // 端口经 IPC get_app_info 下发给前端（BOOTTRACKER_PORT 由 Python 注入本进程）。
        .on_page_load(|webview, payload| {
            if !matches!(payload.event(), PageLoadEvent::Finished) {
                return;
            }
            let _ = webview.show();
            let _ = webview.set_focus();
        })
        .setup(|app| {
            // 获取主窗口
            let window = app.get_webview_window("main")
                .expect("main window not found");

            // 保险：显式设置尺寸，防止窗口被创建为 14x14 最小化态；
            // 不在此处 show：窗口配置为 visible=false，等页面加载完成后
            // 由 on_page_load 显示，避免白屏闪烁与开屏动画重播
            use tauri::PhysicalSize;
            let _ = window.set_size(PhysicalSize::new(960, 760));

            // 安全网：极端情况下（页面加载事件未命中）避免窗口永久隐藏，
            // 4 秒后无条件显示一次（若已显示则为无害幂等）
            let fallback = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(4));
                let _ = fallback.show();
                let _ = fallback.set_focus();
            });

            // 拦截窗口关闭事件：隐藏而非退出（与 PyQt5 行为一致）
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                    // 通知前端窗口已隐藏：暂停轮询与每秒定时器，消除托盘态后台空转。
                    // WebView2 不保证随窗口 hide() 触发 visibilitychange，故用原生事件兜底。
                    let _ = window_clone.emit("window-visibility", false);
                }
            });

            // 构建系统托盘菜单：显示窗口 / 桌面小组件（勾选） / 退出
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let widget_item = CheckMenuItem::with_id(
                app, "toggle_widget", "桌面小组件", true, false, None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &widget_item, &quit_item])?;

            // 托盘初始化后同步小组件勾选状态（后端就绪后拉取，失败保持未勾选）
            let init_item = widget_item.clone();
            std::thread::spawn(move || {
                let url = format!("{}/api/settings", api_base());
                if let Ok(resp) = minreq::get(&url).with_timeout(5).send() {
                    if resp.status_code == 200 {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(
                            resp.as_str().unwrap_or(""),
                        ) {
                            let enabled = v
                                .get("widgetEnabled")
                                .and_then(|x| x.as_bool())
                                .unwrap_or(false);
                            let _ = init_item.set_checked(enabled);
                        }
                    }
                }
            });

            // 创建系统托盘图标
            let toggle_item = widget_item.clone();
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("开机记录")
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.emit("window-visibility", true);
                            }
                        }
                        "toggle_widget" => {
                            // 后端翻转开关并实时启停小组件，响应后同步勾选
                            let url = format!("{}/api/widget-toggle", api_base());
                            let item = toggle_item.clone();
                            std::thread::spawn(move || {
                                let resp = minreq::post(&url)
                                    .with_header("Content-Type", "application/json")
                                    .with_body("{}")
                                    .with_timeout(5)
                                    .send();
                                if let Ok(r) = resp {
                                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(
                                        r.as_str().unwrap_or(""),
                                    ) {
                                        if let Some(on) =
                                            v.get("widgetEnabled").and_then(|x| x.as_bool())
                                        {
                                            let _ = item.set_checked(on);
                                        }
                                    }
                                }
                            });
                        }
                        "quit" => {
                            // 先通知 Python 后端统一清理（小组件/隧道/服务器），再退出本进程
                            let url = format!("{}/api/stop", api_base());
                            let _ = minreq::post(&url)
                                .with_header("Content-Type", "application/json")
                                .with_body("{}")
                                .with_timeout(1)
                                .send();
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键点击托盘图标时显示窗口
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("window-visibility", true);
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // 运行应用事件循环
    app.run(|_app_handle, event| {
        match event {
            RunEvent::ExitRequested { .. } => {
                // 允许正常退出
            }
            _ => {}
        }
    });
}
