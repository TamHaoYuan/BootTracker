//! Toast 消息 + 确认模态框（全局单例）

use leptos::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use wasm_bindgen::JsCast;

/* ================= Toast ================= */

#[derive(Clone, Copy, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
    Warning,
}

#[derive(Clone)]
struct Toast {
    id: u64,
    kind: ToastKind,
    text: String,
}

static TOASTS: OnceLock<RwSignal<Vec<Toast>>> = OnceLock::new();

fn toasts() -> RwSignal<Vec<Toast>> {
    TOASTS.get().cloned().unwrap_or_else(|| RwSignal::new(Vec::new()))
}

fn push(kind: ToastKind, text: &str) {
    static NEXT_ID: OnceLock<RwSignal<u64>> = OnceLock::new();
    let id = {
        let sig = NEXT_ID.get_or_init(|| RwSignal::new(0));
        let id = sig.get_untracked() + 1;
        sig.set(id);
        id
    };
    toasts().update(|list| {
        list.push(Toast {
            id,
            kind,
            text: text.to_string(),
        })
    });
    // 3s 后移除
    let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
        toasts().update(|list| list.retain(|t| t.id != id));
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback(cb.unchecked_ref());
    }
}

pub fn toast_success(text: &str) {
    push(ToastKind::Success, text);
}
pub fn toast_error(text: &str) {
    push(ToastKind::Error, text);
}
pub fn toast_info(text: &str) {
    push(ToastKind::Info, text);
}
pub fn toast_warning(text: &str) {
    push(ToastKind::Warning, text);
}

/// 顶层 Toast 渲染层
#[component]
pub fn ToastLayer() -> impl IntoView {
    view! {
        <div class="toast-layer">
            <For
                each=move || toasts().get()
                key=|t| t.id
                let:t
            >
                <div class=move || {
                    let cls = match t.kind {
                        ToastKind::Success => "toast success",
                        ToastKind::Error => "toast error",
                        ToastKind::Info => "toast info",
                        ToastKind::Warning => "toast warning",
                    };
                    cls.to_string()
                }>
                    <span class="t-dot"></span>
                    {t.text.clone()}
                </div>
            </For>
        </div>
    }
}

/* ================= 确认模态框 ================= */

#[derive(Clone)]
pub struct ConfirmSpec {
    pub title: String,
    /// 正文（纯文本，支持换行）
    pub body: String,
    /// 正文内的警示框文案（如将删除 N 条记录）
    pub warn_box: String,
    pub ok_text: String,
    pub danger: bool,
    /// 确认回调（内部自行 spawn_local 处理异步）
    pub on_ok: Arc<Mutex<Option<Box<dyn FnOnce() + Send + Sync + 'static>>>>,
}

impl ConfirmSpec {
    pub fn simple(title: &str, body: &str, ok_text: &str, danger: bool) -> Self {
        Self {
            title: title.to_string(),
            body: body.to_string(),
            warn_box: String::new(),
            ok_text: ok_text.to_string(),
            danger,
            on_ok: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_on_ok(self, f: impl FnOnce() + Send + Sync + 'static) -> Self {
        *self.on_ok.lock().unwrap() = Some(Box::new(f));
        self
    }
}

static CONFIRM: OnceLock<RwSignal<Option<ConfirmSpec>>> = OnceLock::new();

fn confirm_signal() -> RwSignal<Option<ConfirmSpec>> {
    CONFIRM.get().cloned().unwrap_or_else(|| RwSignal::new(None))
}

pub fn confirm(spec: ConfirmSpec) {
    confirm_signal().set(Some(spec));
}

/// 顶层模态框渲染层
#[component]
pub fn ModalLayer() -> impl IntoView {
    let open = move || confirm_signal().get().is_some();
    view! {
        <Show when=open>
            <div class="modal-mask" on:click=move |_| confirm_signal().set(None)>
                <div class="modal" on:click=move |ev| ev.stop_propagation()>
                    {move || {
                        confirm_signal().get().map(|spec| {
                            let title = spec.title.clone();
                            let body = spec.body.clone();
                            let warn = spec.warn_box.clone();
                            let ok_text = spec.ok_text.clone();
                            let danger = spec.danger;
                            let on_ok = spec.on_ok.clone();
                            view! {
                                <div class="modal-head">{title}</div>
                                <div class="modal-body">
                                    <p>{body}</p>
                                    {(!warn.is_empty()).then(|| view! {
                                        <div class="warn-box">{warn}</div>
                                    })}
                                </div>
                                <div class="modal-foot">
                                    <button class="btn" on:click=move |_| confirm_signal().set(None)>
                                        "取消"
                                    </button>
                                    <button
                                        class=move || if danger { "btn primary danger".to_string() } else { "btn primary".to_string() }
                                        on:click=move |_| {
                                            confirm_signal().set(None);
                                            if let Some(f) = on_ok.lock().unwrap().take() {
                                                f();
                                            }
                                        }
                                    >
                                        {ok_text}
                                    </button>
                                </div>
                            }
                        })
                    }}
                </div>
            </div>
        </Show>
    }
}
