//! 后端 REST API 客户端与数据模型（字段与 Python 后端 JSON 严格对应）
//!
//! 说明：
//! - 所有时间字段为 UTC ISO 8601（带 Z 后缀），如 "2026-09-30T12:00:00Z"
//! - `duration` 单位为**毫秒**
//! - 端口由 Python 启动时经 BOOTTRACKER_PORT 注入，默认 18792

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootSession {
    pub id: String,
    pub boot_time: String,
    pub shutdown_time: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataResponse {
    pub boot_count: i64,
    pub shutdown_count: i64,
    pub sessions: Vec<BootSession>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub total_boot: i64,
    pub total_shutdown: i64,
    pub total_duration: i64,
    pub avg_duration: i64,
    pub active_count: i64,
    pub recent_sessions: Vec<BootSession>,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendDay {
    pub boot_count: i64,
    pub shutdown_count: i64,
    pub total_duration: i64,
    pub avg_duration: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TrendResponse {
    pub data: HashMap<String, TrendDay>,
    pub days: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStat {
    pub boot_count: i64,
    pub shutdown_count: i64,
    pub total_duration: i64,
    pub session_count: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DailyResponse {
    pub data: HashMap<String, DailyStat>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyStat {
    pub boot_count: i64,
    pub shutdown_count: i64,
    pub total_duration: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WeeklyResponse {
    pub data: HashMap<String, WeeklyStat>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anomalies {
    pub longest: Vec<BootSession>,
    pub shortest: Vec<BootSession>,
    pub active: Vec<BootSession>,
    pub avg_duration: i64,
    pub threshold_high: i64,
    pub threshold_low: i64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageOnly {
    #[serde(default)]
    pub language: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub auto_backup: bool,
    #[serde(default)]
    pub backup_count: i64,
    #[serde(default)]
    pub auto_close_idle: bool,
    #[serde(default)]
    pub idle_close_minutes: i64,
    #[serde(default)]
    pub default_chart_type: String,
    #[serde(default)]
    pub time_format: String,
    #[serde(default)]
    pub lan_access: bool,
    #[serde(default)]
    pub tunnel_enabled: bool,
    #[serde(default)]
    pub tunnel_token: String,
    #[serde(default)]
    pub custom_domain: String,
    #[serde(default)]
    pub widget_enabled: bool,
    #[serde(default)]
    pub app_mode: String,
    #[serde(default)]
    pub app_theme: String,
    /// 界面缩放比例（1.0 = 100%），由设置页滑块或 Ctrl+滚轮/加减号写入
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    /// 界面语言：`zh-CN` / `en-US`（后端已在首次运行时按系统语言固化）
    #[serde(default)]
    pub language: String,
    #[serde(rename = "_autoStartRegistered", default)]
    pub auto_start_registered: bool,
}

fn default_ui_scale() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub update_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TrashResponse {
    pub sessions: Vec<BootSession>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BackupsResponse {
    pub backups: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TunnelResponse {
    pub url: Option<String>,
}

/// `/api/tunnel-download` 子对象：一键下载 cloudflared 的进度
#[derive(Clone, Debug, Default, Deserialize)]
pub struct TunnelDownload {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub bytes: i64,
}

/// `/api/tunnel-status` 完整状态。`error` / `download.error` 是**错误 key**
/// （`cloudflared_missing`、`download_failed`、`exited`、`url_timeout` …），
/// 由界面本地化成可读文案；`errorDetail` 是原样透传的诊断信息（退出码、日志尾巴）。
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    /// `tunnel-start` / `tunnel-remove` 的动作结果；纯状态查询时后端也会带上（恒 true）
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub error_detail: String,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub binary_version: String,
    #[serde(default)]
    pub binary_path: String,
    #[serde(default)]
    pub download: TunnelDownload,
}

/// `/api/widget-toggle` 响应：后端翻转后返回组件最新开关状态
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetToggleResponse {
    #[serde(default)]
    pub ok: bool,
    pub widget_enabled: bool,
}

/// 异步请求完成后回传给 UI 线程的消息
pub enum AppMsg {
    DataLoaded(Result<DataResponse, String>),
    OverviewLoaded(Result<Overview, String>),
    TrendLoaded(Result<TrendResponse, String>),
    DailyLoaded(Result<DailyResponse, String>),
    WeeklyLoaded(Result<WeeklyResponse, String>),
    AnomaliesLoaded(Result<Anomalies, String>),
    SettingsLoaded(Result<AppSettings, String>),
    VersionLoaded(Result<VersionInfo, String>),
    TrashLoaded(Result<TrashResponse, String>),
    BackupsLoaded(Result<BackupsResponse, String>),
    TunnelLoaded(Result<TunnelResponse, String>),
    /// 隧道完整状态（含 cloudflared 是否安装、错误原因、下载进度）
    TunnelStatusLoaded(Result<TunnelStatus, String>),
    /// 只取语言字段：整份设置轮询在「设置-网络」页是被跳过的（防覆盖未提交输入），
    /// 语言得走这条独立通道，否则在该页切语言永远不生效。
    LanguageLoaded(Result<LanguageOnly, String>),
    /// 通用写操作结果：ok + 操作描述
    Action(Result<(), String>, String),
    /// 托盘翻转小组件后的新状态
    WidgetToggled(Result<bool, String>),
}

#[derive(Clone)]
pub struct Client {
    pub base: String,
}

impl Client {
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    pub fn get(&self, path: &str) -> Result<String, String> {
        ureq::get(&self.url(path))
            .timeout(Duration::from_secs(8))
            .call()
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())
    }

    pub fn post_json(&self, path: &str, body: &serde_json::Value) -> Result<String, String> {
        ureq::post(&self.url(path))
            .timeout(Duration::from_secs(8))
            .send_json(body.clone())
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())
    }

    pub fn put_json(&self, path: &str, body: &serde_json::Value) -> Result<String, String> {
        ureq::put(&self.url(path))
            .timeout(Duration::from_secs(8))
            .send_json(body.clone())
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())
    }

    pub fn delete_json(&self, path: &str, body: &serde_json::Value) -> Result<String, String> {
        ureq::delete(&self.url(path))
            .timeout(Duration::from_secs(8))
            .send_json(body.clone())
            .map_err(|e| e.to_string())?
            .into_string()
            .map_err(|e| e.to_string())
    }
}
