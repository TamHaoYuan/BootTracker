//! Tauri IPC 桥（纯 Rust WASM → window.__TAURI__）
//!
//! tauri.conf.json 开启 withGlobalTauri，WebView 全局注入 `__TAURI__`；
//! 通过 wasm-bindgen 直接绑定 `__TAURI__.core.invoke`，不再依赖 JS 前端桥。

use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// window.__TAURI__.core.invoke(cmd, args)
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: &JsValue) -> JsValue;

    /// window.__TAURI__.event.listen(event, handler) → unlisten
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &js_sys::Function) -> JsValue;
}

/// 是否运行在 Tauri 环境（__TAURI__ 全局存在）
pub fn is_tauri() -> bool {
    js_sys::Reflect::has(&js_sys::global(), &JsValue::from_str("__TAURI__")).unwrap_or(false)
}

/// IPC 返回结构（与 Rust 壳层 commands.rs BridgeResult 对齐）
#[derive(serde::Deserialize)]
struct RawResult {
    ok: bool,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// 调用 Tauri IPC 命令，返回 data 部分
pub async fn call_ipc<T: DeserializeOwned>(cmd: &str, args: serde_json::Value) -> Result<T, String> {
    let args_json = serde_json::to_string(&args).map_err(|e| format!("serialize args: {e}"))?;
    let args_js = js_sys::JSON::parse(&args_json).map_err(|_| "invalid args JSON".to_string())?;
    let raw = invoke(cmd, &args_js).await;
    let raw_str = js_sys::JSON::stringify(&raw)
        .map(|s| s.as_string().unwrap_or_default())
        .unwrap_or_else(|_| "{}".to_string());
    let parsed: RawResult = serde_json::from_str(&raw_str)
        .map_err(|e| format!("invalid IPC result: {e}"))?;
    if !parsed.ok {
        return Err(parsed
            .message
            .or(parsed.code)
            .unwrap_or_else(|| "IPC business error".into()));
    }
    let data = parsed.data.unwrap_or(serde_json::Value::Null);
    serde_json::from_value(data).map_err(|e| format!("decode IPC data: {e}"))
}

/// 同步窗口标题栏主题（DWM 暗色/浅色）
pub async fn set_theme(mode: &str) -> Result<(), String> {
    let _: Option<()> = call_ipc(
        "set_theme",
        serde_json::json!({ "mode": mode }),
    )
    .await?;
    Ok(())
}

/// 退出应用
pub async fn quit_app() -> Result<(), String> {
    let _: Option<()> = call_ipc("quit_app", serde_json::json!({})).await?;
    Ok(())
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct AppInfo {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub mode: Option<String>,
}

/// 获取应用信息（含后端端口）
pub async fn get_app_info() -> Result<AppInfo, String> {
    call_ipc("get_app_info", serde_json::json!({})).await
}

/// 订阅 window-visibility 事件（托盘隐藏/显示时 Rust 壳层发出）
/// 返回的 unlisten 句柄被泄漏（应用生命周期内存活）
pub fn listen_window_visibility(f: impl Fn(bool) + 'static) {
    let closure = Closure::wrap(Box::new(move |e: JsValue| {
        // payload: { event, id, payload: bool }
        let visible = js_sys::Reflect::get(&e, &JsValue::from_str("payload"))
            .ok()
            .and_then(|v| v.as_bool())
            .map(|b| b != false)
            .unwrap_or(true);
        f(visible);
    }) as Box<dyn FnMut(JsValue)>);
    let handler = closure.as_ref().unchecked_ref::<js_sys::Function>().clone();
    closure.forget();
    let _unlisten = listen("window-visibility", &handler);
}
