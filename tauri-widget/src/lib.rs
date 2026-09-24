//! BootTracker 桌面小组件（Rust 实现）
//!
//! 替代旧版 Tkinter 组件：无边框、置顶、可拖动、位置记忆、Mica 背景。
//! - 数据通道：每 3 秒从 Python 后端 `/api/data` 拉取，通过事件推给前端
//! - 右键菜单：打开主界面 / 隐藏组件 / 退出应用
//! - 位置记忆：读写 settings.json 的 widgetPosition（与旧版格式兼容）

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use serde::{Deserialize, Serialize};
use tauri::{
    menu::{ContextMenu, Menu, MenuItem},
    AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, WebviewWindow, WindowEvent, Wry,
};

/// 窗口尺寸（与 tauri.conf.json 一致），用于默认右下角定位
const WIDTH: f64 = 230.0;
const HEIGHT: f64 = 116.0;

/// 位置保存节流：拖动时每秒最多写一次 settings.json
static LAST_SAVED_MS: AtomicI64 = AtomicI64::new(0);

/// 数据轮询线程退出标志
static SHOULD_STOP: AtomicBool = AtomicBool::new(false);

/// 推送到前端的数据载荷
#[derive(Serialize, Clone)]
struct WidgetData {
    today_count: u32,
    /// 当前活跃会话的开机时间（ISO 字符串，前端据此实时计时）
    boot_time: Option<String>,
    /// 后端是否在线
    online: bool,
}

#[derive(Deserialize)]
struct Session {
    #[serde(rename = "bootTime")]
    boot_time: Option<String>,
    #[serde(rename = "shutdownTime")]
    shutdown_time: Option<String>,
}

#[derive(Deserialize)]
struct ApiResponse {
    #[serde(default)]
    sessions: Vec<Session>,
}

// ——— 环境解析 ———

fn api_base() -> String {
    let port = std::env::var("BOOTTRACKER_PORT").unwrap_or_else(|_| "18792".into());
    format!("http://127.0.0.1:{port}")
}

fn settings_path() -> Option<PathBuf> {
    std::env::var("BOOTTRACKER_SETTINGS")
        .ok()
        .map(PathBuf::from)
}

// ——— 数据拉取 ———

fn fetch_widget_data() -> WidgetData {
    let offline = WidgetData {
        today_count: 0,
        boot_time: None,
        online: false,
    };

    let url = format!("{}/api/data", api_base());
    let resp = match minreq::get(&url).with_timeout(3).send() {
        Ok(r) if r.status_code == 200 => r,
        _ => return offline,
    };

    let data: ApiResponse = match serde_json::from_str(resp.as_str().unwrap_or("")) {
        Ok(d) => d,
        Err(_) => return offline,
    };

    use chrono::{DateTime, Local};
    let today = Local::now().date_naive();
    let mut today_count = 0u32;
    let mut active_boot: Option<String> = None;

    for s in &data.sessions {
        if let Some(bt) = s.boot_time.as_deref() {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(bt) {
                if parsed.with_timezone(&Local).date_naive() == today {
                    today_count += 1;
                }
            }
        }
        // shutdownTime 为空 = 活跃会话
        if s.shutdown_time.is_none() && active_boot.is_none() {
            active_boot = s.boot_time.clone();
        }
    }

    WidgetData {
        today_count,
        boot_time: active_boot,
        online: true,
    }
}

/// 数据轮询线程：启动后立即推送一次，之后每 3 秒一次
fn spawn_data_loop(app: AppHandle) {
    std::thread::spawn(move || {
        // 启动后立刻发一次，避免界面 0~3s 空白
        let first = fetch_widget_data();
        let _ = app.emit("widget-data", &first);

        loop {
            if SHOULD_STOP.load(Ordering::Relaxed) {
                break;
            }
            let data = fetch_widget_data();
            let _ = app.emit("widget-data", &data);
            // 分段睡眠（1 秒粒度），以便及时响应退出信号
            for _ in 0..3 {
                if SHOULD_STOP.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    });
}

// ——— settings.json 读写（widgetPosition / widgetEnabled）———

fn load_position() -> Option<(i32, i32)> {
    let path = settings_path()?;
    let raw = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let pos = v.get("widgetPosition")?.as_str()?;
    let mut parts = pos.split(',');
    let x = parts.next()?.trim().parse().ok()?;
    let y = parts.next()?.trim().parse().ok()?;
    Some((x, y))
}

fn save_position(x: i32, y: i32) {
    if let Some(path) = settings_path() {
        update_settings(&path, |obj| {
            obj.insert(
                "widgetPosition".into(),
                serde_json::Value::String(format!("{x},{y}")),
            );
        });
    }
}

/// 隐藏组件：settings.widgetEnabled=false 后退出进程
fn hide_widget_and_exit(app: &AppHandle) {
    if let Some(path) = settings_path() {
        update_settings(&path, |obj| {
            obj.insert("widgetEnabled".into(), serde_json::Value::Bool(false));
        });
    }
    SHOULD_STOP.store(true, Ordering::Relaxed);
    app.exit(0);
}

/// 退出应用：请求主程序关闭，失败时仅退出 widget 自身
fn quit_app_and_exit(app: &AppHandle) {
    let url = format!("{}/api/stop", api_base());
    let _ = minreq::post(&url)
        .with_header("Content-Type", "application/json")
        .with_body("{}")
        .with_timeout(2)
        .send();
    SHOULD_STOP.store(true, Ordering::Relaxed);
    app.exit(0);
}

/// 读取-修改-写回 settings.json，保留其余键不变。
/// 写入使用 tmp 文件 + rename 原子替换，避免并发写损坏。
fn update_settings<F>(path: &std::path::Path, modify: F)
where
    F: FnOnce(&mut serde_json::Map<String, serde_json::Value>),
{
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(_) => return,
    };
    let mut v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(obj) = v.as_object_mut() {
        modify(obj);
        let out = match serde_json::to_string_pretty(&v) {
            Ok(s) => s,
            Err(_) => return,
        };
        let dir = match path.parent() {
            Some(d) => d.to_path_buf(),
            None => PathBuf::from("."),
        };
        let tmp = dir.join(format!(
            ".widget_settings.{}.tmp",
            std::process::id()
        ));
        if std::fs::write(&tmp, &out).is_ok() {
            // 同盘 rename 原子替换；失败 fallback 到覆盖写
            if std::fs::rename(&tmp, path).is_err() {
                let _ = std::fs::write(path, out);
                let _ = std::fs::remove_file(tmp);
            }
        }
    }
}

// ——— 主界面唤起 ———

fn open_main_ui_action() {
    let url = format!("{}/api/raise-window", api_base());
    std::thread::spawn(move || {
        let _ = minreq::post(&url)
            .with_header("Content-Type", "application/json")
            .with_body("{}")
            .with_timeout(2)
            .send();
    });
}

// ——— Windows DWM：Mica/云母背景（fallback：窗口效果）———

#[cfg(windows)]
fn apply_mica<W: raw_window_handle::HasWindowHandle + Clone>(window: &W) {
    // window-vibrancy 接受任何实现 HasWindowHandle 的类型
    #[allow(unused_variables)]
    {
        // 若窗口类型直接支持 apply_mica，则调用；否则静默忽略
        let _ = window_vibrancy::apply_mica(window, Some(true));
    }
}

#[cfg(not(windows))]
fn apply_mica<W>(_window: &W) {}

// ——— IPC 命令 ———

/// 前端右键触发：在鼠标位置弹出原生上下文菜单
#[tauri::command]
fn show_context_menu(window: WebviewWindow, menu: tauri::State<'_, Menu<Wry>>) {
    // 菜单已在 setup 阶段通过 app.manage(menu) 注入；
    // popup 需要 Window（而非 WebviewWindow），经 AsRef<Webview>::window() 取得。
    let _ = menu.popup(window.as_ref().window());
}

/// 前端双击触发：打开主界面
#[tauri::command]
fn open_main_ui() {
    open_main_ui_action();
}

// ——— 应用入口 ———

pub fn run() {
    let app = tauri::Builder::default()
        // 菜单事件全局钩子：右键菜单项点击分发
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_main" => open_main_ui_action(),
            "hide_widget" => hide_widget_and_exit(app),
            "quit_app" => quit_app_and_exit(app),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![show_context_menu, open_main_ui])
        .setup(|app| {
            let window = app
                .get_webview_window("widget")
                .expect("widget window not found");

            // Mica 已禁用（与 transparent:false 配合使用纯色背景）

            // 保险：显式设置尺寸 + show，防止窗口被创建为 14x14 最小化态
            let _ = window.set_size(tauri::PhysicalSize::new(WIDTH as u32, HEIGHT as u32));
            let _ = window.show();

            // 位置恢复：优先 settings 记忆，否则屏幕右下角
            if let Some((x, y)) = load_position() {
                let _ = window.set_position(PhysicalPosition::new(x, y));
            } else if let Ok(Some(monitor)) = window.current_monitor() {
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let x = size.width as f64 / scale - WIDTH - 20.0;
                let y = size.height as f64 / scale - HEIGHT - 80.0;
                let _ = window.set_position(tauri::LogicalPosition::new(x, y));
            }

            // 右键菜单：打开主界面 / 隐藏组件 / 退出应用
            let open_item = MenuItem::with_id(app, "open_main", "打开主界面", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide_widget", "隐藏组件", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit_app", "退出应用", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &hide_item, &quit_item])?;
            app.manage(menu);

            // 拖动位置记忆 + 关闭请求拦截（关闭即退出 widget 进程）
            let win_for_close = window.clone();
            window.on_window_event(move |event| match event {
                WindowEvent::Moved(position) => {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    let last = LAST_SAVED_MS.load(Ordering::Relaxed);
                    if now_ms - last > 1000 {
                        LAST_SAVED_MS.store(now_ms, Ordering::Relaxed);
                        save_position(position.x, position.y);
                    }
                }
                WindowEvent::CloseRequested { api, .. } => {
                    // 关闭按钮即退出进程（先保存位置，再停止轮询）
                    SHOULD_STOP.store(true, Ordering::Relaxed);
                    if let Ok(pos) = win_for_close.outer_position() {
                        save_position(pos.x, pos.y);
                    }
                    // 不阻止默认：允许底层继续关闭
                    let _ = api;
                }
                _ => {}
            });

            spawn_data_loop(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building widget application");

    app.run(|app_handle, event| {
        // 退出前：设 stop flag + 兜底保存最终位置
        if let RunEvent::Exit = event {
            SHOULD_STOP.store(true, Ordering::Relaxed);
            if let Some(window) = app_handle.get_webview_window("widget") {
                if let Ok(pos) = window.outer_position() {
                    save_position(pos.x, pos.y);
                }
            }
        }
    });
}
