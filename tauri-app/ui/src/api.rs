//! HTTP API 客户端 + 数据类型（与 Python 后端 /api 对齐）
//!
//! 后端 CORS 全通（Access-Control-Allow-Origin: *），桌面端内嵌资源
//! （tauri://localhost 源）直接跨源请求 http://127.0.0.1:{port}/api/*。

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/* ================= API 基址 ================= */

static API_BASE: OnceLock<String> = OnceLock::new();

/// 初始化 API 基址（main 启动时调用；默认端口 18792）
pub fn init_base(port: u16) {
    let _ = API_BASE.set(format!("http://127.0.0.1:{port}"));
}

fn base() -> String {
    API_BASE
        .get()
        .cloned()
        .unwrap_or_else(|| "http://127.0.0.1:18792".to_string())
}

/* ================= 类型 ================= */

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootSession {
    pub id: String,
    pub boot_time: String,
    pub shutdown_time: Option<String>,
    #[serde(default)]
    pub duration: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BootData {
    pub boot_count: i64,
    pub shutdown_count: i64,
    #[serde(default)]
    pub sessions: Vec<BootSession>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_time: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddSessionBody {
    pub boot_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_time: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub auto_start: bool,
    pub auto_backup: bool,
    pub backup_count: i64,
    pub auto_close_idle: bool,
    pub idle_close_minutes: i64,
    pub default_chart_type: String,
    pub time_format: String,
    pub custom_bg_image: String,
    pub lan_access: bool,
    pub tunnel_enabled: bool,
    pub tunnel_token: String,
    pub custom_domain: String,
    pub widget_enabled: bool,
    pub widget_position: String,
    pub app_mode: String,
    #[serde(default)]
    pub app_theme: Option<String>,
    #[serde(rename = "_autoStartRegistered", default)]
    pub auto_start_registered: bool,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TrendStat {
    pub boot_count: i64,
    pub shutdown_count: i64,
    pub total_duration: f64,
    pub avg_duration: f64,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WeeklyStat {
    pub boot_count: i64,
    pub shutdown_count: i64,
    pub total_duration: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendResp {
    #[serde(default)]
    pub data: std::collections::HashMap<String, TrendStat>,
    #[serde(default)]
    pub days: i64,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct OverviewStats {
    pub total_boot: i64,
    pub total_shutdown: i64,
    pub total_duration: f64,
    pub avg_duration: f64,
    pub active_count: i64,
    #[serde(default)]
    pub recent_sessions: Vec<BootSession>,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub update_url: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionHistoryEntry {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpResp {
    pub version: String,
    pub previous: String,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CheckUpdateResp {
    pub latest: Option<String>,
    pub has_update: Option<bool>,
    pub download_url: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelUrlResp {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TrashData {
    #[serde(default)]
    pub sessions: Vec<BootSession>,
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BackupListResp {
    #[serde(default)]
    pub backups: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanBackupsResp {
    pub deleted: i64,
    pub remaining: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResp {
    #[serde(default)]
    pub ok: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadBgResp {
    #[serde(default)]
    pub path: String,
}

/* ================= 底层请求 ================= */

#[derive(serde::Deserialize)]
struct ErrBody {
    #[serde(default)]
    error: Option<String>,
}

async fn request<T: DeserializeOwned>(
    method: &str,
    path: &str,
    body: Option<String>,
) -> Result<T, String> {
    let url = format!("{}/api{path}", base());
    let window = web_sys::window().ok_or("no window")?;
    let opts = web_sys::RequestInit::new();
    opts.set_method(method);
    if let Some(b) = body {
        opts.set_body(&wasm_bindgen::JsValue::from_str(&b));
    }
    let req = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|_| format!("build request: {url}"))?;
    req.headers()
        .set("Content-Type", "application/json")
        .map_err(|e| format!("set header: {e:?}"))?;

    let resp_val = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| format!("fetch failed: {:?}", e.as_string().unwrap_or_default()))?;
    let resp: web_sys::Response = resp_val.dyn_into().map_err(|_| "not a Response")?;
    let text_promise = resp.text().map_err(|e| format!("read body: {e:?}"))?;
    let text_val = JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("read body failed: {:?}", e.as_string().unwrap_or_default()))?;
    let text = text_val.as_string().unwrap_or_default();

    if let Ok(eb) = serde_json::from_str::<ErrBody>(&text) {
        if let Some(msg) = eb.error {
            return Err(msg);
        }
    }
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    if text.trim().is_empty() {
        return serde_json::from_str("{}").map_err(|e| e.to_string());
    }
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON response: {e}"))
}

/* ================= 端点封装 ================= */

pub struct DataApi;
impl DataApi {
    pub async fn ping() -> Result<PingResp, String> {
        request("GET", "/ping", None).await
    }
    pub async fn get_data() -> Result<BootData, String> {
        request("GET", "/data", None).await
    }
    pub async fn save_data(d: &BootData) -> Result<(), String> {
        let body = serde_json::to_string(d).map_err(|e| e.to_string())?;
        request::<serde_json::Value>("POST", "/data", Some(body)).await?;
        Ok(())
    }
    pub async fn add_session(boot: String, shut: Option<String>) -> Result<(), String> {
        let body = serde_json::to_string(&AddSessionBody {
            boot_time: boot,
            shutdown_time: shut,
        })
        .map_err(|e| e.to_string())?;
        request::<serde_json::Value>("POST", "/add-session", Some(body)).await?;
        Ok(())
    }
    pub async fn update_session(id: &str, patch: SessionPatch) -> Result<(), String> {
        let body = serde_json::json!({ "session_id": id, "patch": patch }).to_string();
        request::<serde_json::Value>("PUT", &format!("/sessions/{id}"), Some(body)).await?;
        Ok(())
    }
    pub async fn clear_data() -> Result<(), String> {
        request::<serde_json::Value>("POST", "/clear", None).await?;
        Ok(())
    }
    pub async fn merge_sessions(ids: Vec<String>) -> Result<(), String> {
        let body = serde_json::json!({ "ids": ids }).to_string();
        request::<serde_json::Value>("POST", "/merge-sessions", Some(body)).await?;
        Ok(())
    }
}

pub struct StatsApi;
impl StatsApi {
    pub async fn trend(days: i64) -> Result<TrendResp, String> {
        request("GET", &format!("/stats/trend?days={days}"), None).await
    }
    pub async fn weekly() -> Result<std::collections::HashMap<String, WeeklyStat>, String> {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct Wrap {
            data: std::collections::HashMap<String, WeeklyStat>,
        }
        let w: Wrap = request("GET", "/stats/weekly", None).await?;
        Ok(w.data)
    }
    pub async fn overview() -> Result<OverviewStats, String> {
        request("GET", "/stats/overview", None).await
    }
}

pub struct SettingsApi;
impl SettingsApi {
    pub async fn get() -> Result<AppSettings, String> {
        request("GET", "/settings", None).await
    }
    /// 局部更新（传 serde_json::json!({ "字段": 值 })）
    pub async fn update_partial(patch: serde_json::Value) -> Result<(), String> {
        let body = patch.to_string();
        request::<serde_json::Value>("PUT", "/settings", Some(body)).await?;
        Ok(())
    }
}

pub struct TrashApi;
impl TrashApi {
    pub async fn get() -> Result<TrashData, String> {
        request("GET", "/trash", None).await
    }
    pub async fn save(t: &TrashData) -> Result<(), String> {
        let body = serde_json::to_string(t).map_err(|e| e.to_string())?;
        request::<serde_json::Value>("POST", "/trash", Some(body)).await?;
        Ok(())
    }
    pub async fn restore(id: &str) -> Result<(), String> {
        let body = serde_json::json!({ "id": id }).to_string();
        request::<serde_json::Value>("POST", "/trash/restore", Some(body)).await?;
        Ok(())
    }
    pub async fn clear() -> Result<(), String> {
        request::<serde_json::Value>("POST", "/trash/clear", None).await?;
        Ok(())
    }
    pub async fn permanent_delete(id: &str) -> Result<(), String> {
        let body = serde_json::json!({ "id": id }).to_string();
        request::<serde_json::Value>("POST", "/trash/delete", Some(body)).await?;
        Ok(())
    }
}

pub struct BackupApi;
impl BackupApi {
    pub async fn list() -> Result<BackupListResp, String> {
        request("GET", "/backups", None).await
    }
    pub async fn restore(filename: &str) -> Result<(), String> {
        let body = serde_json::json!({ "filename": filename }).to_string();
        request::<serde_json::Value>("POST", "/backup/restore", Some(body)).await?;
        Ok(())
    }
    pub async fn delete(filename: &str) -> Result<(), String> {
        let body = serde_json::json!({ "filename": filename }).to_string();
        request::<serde_json::Value>("DELETE", "/delete-backup", Some(body)).await?;
        Ok(())
    }
    pub async fn clean(keep: i64) -> Result<CleanBackupsResp, String> {
        let body = serde_json::json!({ "keep": keep }).to_string();
        request("POST", "/clean-backups", Some(body)).await
    }
}

pub struct VersionApi;
impl VersionApi {
    pub async fn get() -> Result<VersionInfo, String> {
        request("GET", "/version", None).await
    }
    pub async fn history() -> Result<Vec<VersionHistoryEntry>, String> {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct Wrap {
            history: Vec<VersionHistoryEntry>,
        }
        let w: Wrap = request("GET", "/version/history", None).await?;
        Ok(w.history)
    }
    pub async fn bump(bump_type: &str, notes: &str) -> Result<BumpResp, String> {
        let body = serde_json::json!({ "type": bump_type, "notes": notes }).to_string();
        request("POST", "/version/bump", Some(body)).await
    }
    pub async fn check_update() -> Result<CheckUpdateResp, String> {
        request::<CheckUpdateResp>("POST", "/check-update", None).await
    }
}

pub struct TunnelApi;
impl TunnelApi {
    pub async fn get_url() -> Result<TunnelUrlResp, String> {
        request("GET", "/tunnel-url", None).await
    }
}

pub struct SystemApi;
impl SystemApi {
    pub async fn restart_server() -> Result<(), String> {
        request::<serde_json::Value>("POST", "/restart-server", None).await?;
        Ok(())
    }
    /// 上传背景图（multipart/form-data）
    pub async fn upload_bg(file: &web_sys::File) -> Result<UploadBgResp, String> {
        let url = format!("{}/api/upload-bg", base());
        let window = web_sys::window().ok_or("no window")?;
        let fd = web_sys::FormData::new().map_err(|e| format!("FormData: {e:?}"))?;
        fd.append_with_blob("file", file.unchecked_ref())
            .map_err(|e| format!("append file: {e:?}"))?;
        let opts = web_sys::RequestInit::new();
        opts.set_method("POST");
        opts.set_body(&fd);
        let req = web_sys::Request::new_with_str_and_init(&url, &opts)
            .map_err(|_| format!("build request: {url}"))?;
        let resp_val = JsFuture::from(window.fetch_with_request(&req))
            .await
            .map_err(|e| format!("upload failed: {:?}", e.as_string().unwrap_or_default()))?;
        let resp: web_sys::Response = resp_val.dyn_into().map_err(|_| "not a Response")?;
        let text_promise = resp.text().map_err(|e| format!("{e:?}"))?;
        let text_val = JsFuture::from(text_promise)
            .await
            .map_err(|e| format!("{e:?}"))?;
        let text = text_val.as_string().unwrap_or_default();
        if let Ok(eb) = serde_json::from_str::<ErrBody>(&text) {
            if let Some(msg) = eb.error {
                return Err(msg);
            }
        }
        if !resp.ok() {
            return Err(format!("HTTP {}", resp.status()));
        }
        serde_json::from_str(&text).map_err(|e| format!("invalid JSON response: {e}"))
    }
}

/// 软删除：把 session 移入回收站（与网页端 softDeleteSession 行为一致）
pub async fn soft_delete_session(id: &str) -> Result<(), String> {
    let mut data = DataApi::get_data().await?;
    let target = data.sessions.iter().position(|s| s.id == id);
    let Some(idx) = target else {
        return Ok(());
    };
    let session = data.sessions.remove(idx);
    data.boot_count = data.sessions.len() as i64;
    data.shutdown_count = data.sessions.iter().filter(|s| s.shutdown_time.is_some()).count() as i64;
    DataApi::save_data(&data).await?;
    let mut trash = TrashApi::get().await.unwrap_or_default();
    trash.sessions.insert(0, session);
    TrashApi::save(&trash).await
}
