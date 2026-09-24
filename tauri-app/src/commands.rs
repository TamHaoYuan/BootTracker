//! Tauri IPC 命令实现
//!
//! 替代原 PyQt5 QWebChannel 桥的原生命令：
//! - set_theme: 切换窗口标题栏暗色/浅色主题
//! - raise_window: 唤起并聚焦主窗口
//! - quit_app: 退出应用
//! - pick_file: 打开文件选择对话框
//! - get_app_info: 获取应用版本信息

use serde::Serialize;
use tauri::{AppHandle, Window};

/// 统一的命令返回结构
#[derive(Serialize)]
pub struct BridgeResult<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T: Serialize> BridgeResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            code: None,
            message: None,
        }
    }
}

/// 空数据的成功返回
impl BridgeResult<()> {
    pub fn ok() -> Self {
        Self {
            ok: true,
            data: Some(()),
            code: None,
            message: None,
        }
    }
}

/// 切换窗口标题栏主题（暗色/浅色）
///
/// 通过 Windows DWM API 设置沉浸式标题栏颜色
#[tauri::command]
pub fn set_theme(window: Window, mode: String) -> BridgeResult<()> {
    let dark = mode != "light";

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};

        unsafe {
            if let Ok(hwnd) = window.hwnd() {
                let hwnd = HWND(hwnd.0 as *mut _);
                let val: i32 = if dark { 1 } else { 0 };
                // 属性 20 = DWMWA_USE_IMMERSIVE_DARK_MODE (Win10 2004+/Win11)
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &val as *const i32 as *const _,
                    std::mem::size_of::<i32>() as u32,
                );
            }
        }
    }

    let _ = dark; // suppress unused warning on non-windows
    BridgeResult::ok()
}

/// 唤起并聚焦主窗口
#[tauri::command]
pub fn raise_window(window: Window) -> BridgeResult<()> {
    let _ = window.show();
    let _ = window.set_focus();
    BridgeResult::ok()
}

/// 退出应用
#[tauri::command]
pub fn quit_app(app: AppHandle) -> BridgeResult<()> {
    app.exit(0);
    BridgeResult::ok()
}

/// 打开文件选择对话框
#[tauri::command]
pub async fn pick_file(filter: Option<String>) -> BridgeResult<Option<String>> {
    let dialog = rfd::AsyncFileDialog::new();

    // 根据 filter 参数设置文件过滤器
    let dialog = if let Some(f) = filter {
        if f.contains("JSON") || f.contains("json") {
            dialog.add_filter("JSON", &["json"])
        } else if f.contains("Image") || f.contains("image") {
            dialog.add_filter("Image", &["png", "jpg", "jpeg", "gif", "bmp", "webp"])
        } else {
            dialog
        }
    } else {
        dialog
    };

    let file = dialog.pick_file().await;
    let path = file.map(|f| f.path().to_string_lossy().to_string());
    BridgeResult::success(path)
}

/// 获取应用版本信息
#[tauri::command]
pub fn get_app_info(app: AppHandle) -> BridgeResult<serde_json::Value> {
    let version = app.package_info().version.to_string();
    let info = serde_json::json!({
        "version": version,
        "frozen": false
    });
    BridgeResult::success(info)
}
