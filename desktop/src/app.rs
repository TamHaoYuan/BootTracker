//! 纯原生桌面应用主体（egui/eframe）
//!
//! 替代原 Tauri WebView 壳 + Leptos/WASM 方案：无 WebView、无 HTML，
//! 全部用 egui 即时模式原生渲染；通过 HTTP 调用 Python 后端 REST API。

use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use egui::{Context, Margin, Rounding};
use serde::de::DeserializeOwned;

use crate::api::{
    Anomalies, AppMsg, AppSettings, BootSession, Client, DailyStat, DataResponse, Overview,
    TrashResponse, TrendResponse, VersionInfo, WeeklyStat, WidgetToggleResponse,
};
use crate::fmt;
use crate::tray::{self, TrayAction};

/// 记录的排序键（表头可点击切换）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Boot,
    Shutdown,
    Duration,
}

/// Toast 语义：成功 / 提示 / 失败——决定描边与图标配色
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Ok,
    Info,
    Err,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Records,
    Charts,
    Settings,
    Admin,
}

impl Page {
    /// 页面名（走 i18n；`t!` 在中文模式下无查表开销）
    pub fn label(&self) -> &'static str {
        match self {
            Page::Dashboard => crate::t!("仪表盘"),
            Page::Records => crate::t!("记录"),
            Page::Charts => crate::t!("图表"),
            Page::Settings => crate::t!("设置"),
            Page::Admin => crate::t!("管理"),
        }
    }

    /// 每个页面对应的 Phosphor 图标（侧栏 / 顶栏使用）。
    pub fn icon(&self) -> &'static str {
        match self {
            Page::Dashboard => egui_phosphor::regular::GAUGE,
            Page::Records => egui_phosphor::regular::CLOCK,
            Page::Charts => egui_phosphor::regular::CHART_LINE_UP,
            Page::Settings => egui_phosphor::regular::GEAR,
            Page::Admin => egui_phosphor::regular::SHIELD,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RecordsView {
    Table,
    Timeline,
}

/// 待确认的危险操作
pub enum PendingAction {
    DeleteSession(String),
    RestoreFromTrash(String),
    DeleteFromTrash(String),
    ClearTrash,
    RestoreBackup(String),
    DeleteBackup(String),
    CleanBackups,
    ClearData,
    MergeSessions,
}

pub struct BootTrackerApp {
    pub(crate) base: String,
    pub(crate) client: Client,
    pub(crate) tx: Sender<AppMsg>,
    pub(crate) rx: Receiver<AppMsg>,
    pub(crate) tray_tx: Sender<TrayAction>,
    pub(crate) tray_rx: Receiver<TrayAction>,

    // 数据
    pub(crate) data: Option<DataResponse>,
    pub(crate) overview: Option<Overview>,
    pub(crate) trend: Option<TrendResponse>,
    pub(crate) daily: Option<std::collections::HashMap<String, DailyStat>>,
    pub(crate) weekly: Option<std::collections::HashMap<String, WeeklyStat>>,
    pub(crate) anomalies: Option<Anomalies>,
    pub(crate) settings: Option<AppSettings>,
    pub(crate) version: Option<VersionInfo>,
    pub(crate) trash: Vec<BootSession>,
    pub(crate) backups: Vec<String>,
    pub(crate) tunnel_url: Option<String>,
    /// 隧道完整状态（安装情况 / 运行中 / 错误原因 / 下载进度）
    pub(crate) tunnel_status: Option<crate::api::TunnelStatus>,
    /// 隧道状态轮询节流（设置页可见时每 2s 一次）
    pub(crate) last_tunnel_poll: f64,

    // UI 状态
    pub(crate) page: Page,
    /// 侧栏是否折叠（常态）——悬浮时做临时展开，不改这个字段
    pub(crate) sidebar_collapsed: bool,
    /// 上一帧侧栏占的屏幕矩形，用于判断指针是否悬浮在侧栏上
    pub(crate) sidebar_rect: Option<egui::Rect>,
    /// 侧栏当前展开进度 0（折叠 64px）→ 1（展开 208px），逐帧缓动
    pub(crate) sidebar_anim: f32,
    /// 上一帧的指针悬浮判定，用于滞回：展开后指针要离开更宽的判定区才收起
    pub(crate) sidebar_hover: bool,
    /// 当前已应用的界面缩放，用于检测 settings.uiScale 变化
    pub(crate) applied_scale: Option<f32>,
    /// 当前已应用的界面语言，用于检测 settings.language 变化
    pub(crate) applied_lang: Option<crate::i18n::Lang>,
    pub(crate) loading: bool,
    pub(crate) initialized: bool,
    /// 首帧居中是否已成功（显示器信息可能稍后才可用，需重试）
    pub(crate) centered: bool,
    pub(crate) toast: Option<(String, ToastKind, f64)>,
    pub(crate) pending_toast: Option<(String, ToastKind)>,
    pub(crate) applied_theme: Option<(bool, String)>,
    pub(crate) last_settings_poll: f64,
    /// 仅语言字段的轮询时刻（整份设置轮询在「设置-网络」页会跳过）
    pub(crate) last_lang_poll: f64,
    /// 最近一次成功请求的时刻（判断后端是否失联）
    pub(crate) last_ok: f64,
    /// 连续失败次数（顶栏状态灯三态判定）
    pub(crate) err_streak: u32,
    pub(crate) tray_ready: bool,
    pub(crate) tray: Option<tray_icon::TrayIcon>,
    pub(crate) widget_check: Option<tray_icon::menu::CheckMenuItem>,
    /// 托盘菜单点了「退出」：此时允许窗口真正关闭，而不是隐藏到托盘
    pub(crate) quitting: bool,

    // 记录页
    pub(crate) filter_text: String,
    pub(crate) filter_from: String,
    pub(crate) filter_to: String,
    pub(crate) records_view: RecordsView,
    pub(crate) selected: HashSet<String>,
    pub(crate) sort_key: SortKey,
    pub(crate) sort_asc: bool,

    // 确认弹窗
    pub(crate) pending: Option<PendingAction>,

    // 页面内部子状态
    pub(crate) settings_tab: u8,
    pub(crate) chart_mode: u8,

    // 管理页
    pub(crate) tunnel_checked: bool,
    /// 入场动效：记录「当前展示的页面」与「该页入场的起始时刻」。
    /// 页面切换时重置起始时刻，各页据此计算 progress。
    pub(crate) anim_page: Page,
    pub(crate) anim_start: f64,
}

impl BootTrackerApp {
    /// 启动语言：优先读磁盘上的 `settings.json`（后端已在首次运行时把系统语言固化进去），
    /// 读不到再按操作系统语言判断。必须在首帧之前拿到，否则中文用户会被先画一帧中文再跳英文。
    ///
    /// 注：`settings.json` 在后端 `server/config.py:19 SETTINGS_FILE = os.path.join(APP_DIR, "settings.json")`，
    /// 开发态 APP_DIR 就是项目根（当前工作目录），打包态在 exe 同级。
    pub fn initial_lang() -> crate::i18n::Lang {
        let mut candidates: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from("settings.json")];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join("settings.json"));
                // 打包态 desktop 二进制在 _internal/desktop/target/release/ 下，往上找四层
                let mut p = dir.to_path_buf();
                for _ in 0..4 {
                    if let Some(parent) = p.parent() {
                        p = parent.to_path_buf();
                        candidates.push(p.join("settings.json"));
                    }
                }
            }
        }
        for path in candidates {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            // 解析逻辑抽在 i18n 里（可单测）
            if let Some(lang) = crate::i18n::lang_from_settings_text(&text) {
                return lang;
            }
            // 是合法的 settings.json 但 language 为空串 = 后端还没固化，
            // 这时按系统语言判断，不再继续看后面的候选路径。
            if serde_json::from_str::<serde_json::Value>(&text).is_ok() {
                break;
            }
        }
        crate::i18n::Lang::from_code(&crate::i18n::detect_os_language())
    }

    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let port = std::env::var("BOOTTRACKER_PORT").unwrap_or_else(|_| "18792".into());
        let base = format!("http://127.0.0.1:{port}");
        let client = Client { base: base.clone() };
        let (tx, rx) = channel::<AppMsg>();
        let (tray_tx, tray_rx) = channel::<TrayAction>();
        BootTrackerApp {
            base,
            client,
            tx,
            rx,
            tray_tx,
            tray_rx,
            data: None,
            overview: None,
            trend: None,
            daily: None,
            weekly: None,
            anomalies: None,
            settings: None,
            version: None,
            trash: Vec::new(),
            backups: Vec::new(),
            tunnel_url: None,
            tunnel_status: None,
            last_tunnel_poll: 0.0,
            page: Page::Dashboard,
            // 与 web 端一致：侧栏常态折叠，靠悬浮临时展开
            sidebar_collapsed: true,
            sidebar_rect: None,
            sidebar_anim: 0.0,
            sidebar_hover: false,
            applied_scale: None,
            applied_lang: None,
            loading: true,
            initialized: false,
            centered: false,
            toast: None,
            pending_toast: None,
            applied_theme: None,
            last_settings_poll: -100.0,
            last_lang_poll: -100.0,
            last_ok: 0.0,
            err_streak: 0,
            tray_ready: false,
            tray: None,
            widget_check: None,
            quitting: false,
            filter_text: String::new(),
            filter_from: String::new(),
            filter_to: String::new(),
            records_view: RecordsView::Table,
            selected: HashSet::new(),
            // 默认按开机时间倒序（最新在最前），与概览「最近记录」一致
            sort_key: SortKey::Boot,
            sort_asc: false,
            pending: None,
            settings_tab: 0,
            chart_mode: 0,
            tunnel_checked: false,
            anim_page: Page::Dashboard,
            anim_start: 0.0,
        }
    }

    /// 当前配色（对齐 web UI token），由 settings.app_mode + settings.app_theme 决定
    pub(crate) fn palette(&self) -> crate::theme::Palette {
        let s = self.settings.as_ref();
        let dark = s.map(|s| s.app_mode != "light").unwrap_or(true);
        let theme = s.map(|s| s.app_theme.as_str()).unwrap_or("blue");
        crate::theme::palette(dark, theme)
    }

    // ---------- 入场动效 ----------

    /// 页面入场进度 0→1（供 `pages` 做淡入 / 逐行错峰）
    pub(crate) fn enter_t(&self, ctx: &Context) -> f32 {
        crate::anim::progress(
            crate::anim::now(ctx),
            self.anim_start,
            0.0,
            crate::anim::DUR_PAGE,
        )
    }

    /// 第 index 行的入场进度 0→1（逐行阶梯，对齐 web 表格的错峰入场）
    pub(crate) fn row_t(&self, ctx: &Context, index: usize) -> f32 {
        crate::anim::progress(
            crate::anim::now(ctx),
            self.anim_start,
            crate::anim::row_delay(index),
            crate::anim::DUR_ROW,
        )
    }

    /// 是否仍在入场动画期间（决定要不要逐帧重绘）
    pub(crate) fn animating(&self, ctx: &Context) -> bool {
        crate::anim::now(ctx) - self.anim_start < crate::anim::total() as f64
    }

    // ---------- 通用请求 ----------

    pub(crate) fn go<F, G>(&self, f: F, g: G)
    where
        F: FnOnce(Client) -> Result<String, String> + Send + 'static,
        G: FnOnce(Result<String, String>) -> AppMsg + Send + 'static,
    {
        let c = self.client.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let r = f(c);
            let _ = tx.send(g(r));
        });
    }

    fn parse<T: DeserializeOwned>(r: Result<String, String>) -> Result<T, String> {
        match r {
            Ok(s) => serde_json::from_str::<T>(&s).map_err(|e| e.to_string()),
            Err(e) => Err(e),
        }
    }

    pub fn refresh_all(&mut self) {
        self.loading = true;
        self.go(
            |c| c.get("/api/data"),
            |r| AppMsg::DataLoaded(Self::parse::<DataResponse>(r)),
        );
        self.go(
            |c| c.get("/api/stats/overview"),
            |r| AppMsg::OverviewLoaded(Self::parse::<Overview>(r)),
        );
        self.go(
            |c| c.get("/api/settings"),
            |r| AppMsg::SettingsLoaded(Self::parse::<AppSettings>(r)),
        );
        self.go(
            |c| c.get("/api/version"),
            |r| AppMsg::VersionLoaded(Self::parse::<VersionInfo>(r)),
        );
    }

    pub fn refresh_data(&self) {
        self.go(
            |c| c.get("/api/data"),
            |r| AppMsg::DataLoaded(Self::parse::<DataResponse>(r)),
        );
        self.go(
            |c| c.get("/api/stats/overview"),
            |r| AppMsg::OverviewLoaded(Self::parse::<Overview>(r)),
        );
    }

    fn spawn_settings(&self) {
        self.go(
            |c| c.get("/api/settings"),
            |r| AppMsg::SettingsLoaded(Self::parse::<AppSettings>(r)),
        );
    }

    /// 只问后台要语言字段。整份设置轮询在「设置-网络」页会被跳过
    /// （防覆盖用户尚未 lost_focus 的 Token 输入），代价是该页上主题/语言
    /// 也一起停更；语言是跨端共享的（web 端改了 desktop 也该跟随），
    /// 所以单独走这条轻量通道。
    fn poll_language(&self) {
        self.go(
            |c| c.get("/api/settings"),
            |r| AppMsg::LanguageLoaded(Self::parse::<crate::api::LanguageOnly>(r)),
        );
    }

    /// 应用语言并同步窗口标题。已经切过的语言直接返回，避免每 2s 重发标题命令。
    fn apply_language(&mut self, ctx: &egui::Context, code: &str) {
        let lang = crate::i18n::Lang::from_code(code);
        if self.applied_lang == Some(lang) {
            return;
        }
        crate::i18n::set_lang(lang);
        self.applied_lang = Some(lang);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(
            crate::t!("开机记录").to_string(),
        ));
    }

    pub fn load_stats(&self) {
        self.go(
            |c| c.get("/api/stats/trend?days=30"),
            |r| AppMsg::TrendLoaded(Self::parse::<TrendResponse>(r)),
        );
        self.go(
            |c| c.get("/api/stats/daily"),
            |r| AppMsg::DailyLoaded(Self::parse::<crate::api::DailyResponse>(r)),
        );
        self.go(
            |c| c.get("/api/stats/weekly"),
            |r| AppMsg::WeeklyLoaded(Self::parse::<crate::api::WeeklyResponse>(r)),
        );
        self.go(
            |c| c.get("/api/stats/anomalies"),
            |r| AppMsg::AnomaliesLoaded(Self::parse::<Anomalies>(r)),
        );
    }

    pub fn load_trash(&self) {
        self.go(
            |c| c.get("/api/trash"),
            |r| AppMsg::TrashLoaded(Self::parse::<TrashResponse>(r)),
        );
    }

    pub fn load_backups(&self) {
        self.go(
            |c| c.get("/api/backups"),
            |r| AppMsg::BackupsLoaded(Self::parse::<crate::api::BackupsResponse>(r)),
        );
    }

    pub fn load_tunnel(&self) {
        self.go(
            |c| c.get("/api/tunnel-url"),
            |r| AppMsg::TunnelLoaded(Self::parse::<crate::api::TunnelResponse>(r)),
        );
    }

    /// 拉取隧道完整状态（设置页-网络 每 2s 轮询，也是「一键临时隧道」的主数据源）
    pub fn load_tunnel_status(&self) {
        self.go(
            |c| c.get("/api/tunnel-status"),
            |r| AppMsg::TunnelStatusLoaded(Self::parse::<crate::api::TunnelStatus>(r)),
        );
    }

    /// 启动隧道：无 token 即 Cloudflare 临时隧道。失败原因由后端 error key 带回。
    pub fn start_tunnel(&self) {
        self.go(
            |c| c.post_json("/api/tunnel-start", &serde_json::json!({})),
            |r| {
                AppMsg::TunnelStatusLoaded(
                    Self::parse::<crate::api::TunnelStatus>(r).map_err(|e| e.to_string()),
                )
            },
        );
    }

    pub fn stop_tunnel(&self) {
        self.go(
            |c| c.post_json("/api/tunnel-stop", &serde_json::json!({})),
            |r| {
                AppMsg::TunnelStatusLoaded(
                    Self::parse::<crate::api::TunnelStatus>(r).map_err(|e| e.to_string()),
                )
            },
        );
    }

    /// 一键下载 cloudflared（后台线程，界面靠轮询看进度）
    pub fn download_cloudflared(&self) {
        self.go(
            |c| c.post_json("/api/tunnel-download", &serde_json::json!({})),
            |r| {
                AppMsg::TunnelStatusLoaded(
                    Self::parse::<crate::api::TunnelStatus>(r).map_err(|e| e.to_string()),
                )
            },
        );
    }

    fn toggle_widget(&self) {
        self.go(
            |c| c.post_json("/api/widget-toggle", &serde_json::json!({})),
            |r| {
                // 后端返回 {"ok":true,"widgetEnabled":bool}，按真实状态回传；
                // 失败时透传 Err，由 handler 显示真实错误（而非误导性的"已关闭"）
                AppMsg::WidgetToggled(
                    Self::parse::<WidgetToggleResponse>(r).map(|w| w.widget_enabled),
                )
            },
        );
    }

    /// 保存单个设置项
    pub fn save_setting(&self, key: &str, value: serde_json::Value, what: &'static str) {
        let body = serde_json::json!({ key: value });
        self.go(
            move |c| c.put_json("/api/settings", &body),
            move |r| AppMsg::Action(r.map(|_| ()), what.to_string()),
        );
    }

    /// 切换主题
    ///
    /// 注意：`appMode` 不在 `/api/settings` 的合法键白名单里，
    /// 主题必须走 `/api/window-theme`（后端会持久化 appMode 并同步窗口外壳）。
    pub fn set_theme(&self, mode: &'static str) {
        let body = serde_json::json!({ "mode": mode });
        let what = crate::t!("切换主题").to_string();
        self.go(
            move |c| c.put_json("/api/window-theme", &body),
            move |r| AppMsg::Action(r.map(|_| ()), what.clone()),
        );
    }

    // ---------- 记录操作 ----------

    pub fn request_delete_session(&mut self, id: String) {
        self.pending = Some(PendingAction::DeleteSession(id));
    }
    pub fn request_restore_trash(&mut self, id: String) {
        self.pending = Some(PendingAction::RestoreFromTrash(id));
    }
    pub fn request_delete_trash(&mut self, id: String) {
        self.pending = Some(PendingAction::DeleteFromTrash(id));
    }
    pub fn request_clear_trash(&mut self) {
        self.pending = Some(PendingAction::ClearTrash);
    }
    pub fn request_restore_backup(&mut self, name: String) {
        self.pending = Some(PendingAction::RestoreBackup(name));
    }
    pub fn request_delete_backup(&mut self, name: String) {
        self.pending = Some(PendingAction::DeleteBackup(name));
    }
    pub fn request_clean_backups(&mut self) {
        self.pending = Some(PendingAction::CleanBackups);
    }
    pub fn request_clear_data(&mut self) {
        self.pending = Some(PendingAction::ClearData);
    }
    pub fn request_merge(&mut self) {
        if self.selected.len() >= 2 {
            self.pending = Some(PendingAction::MergeSessions);
        } else {
            self.notify(crate::t!("请至少选择两条记录"));
        }
    }

    fn do_pending(&mut self) {
        let action = match self.pending.take() {
            Some(a) => a,
            None => return,
        };
        // 文案在动作分发前一次性取出（`t!` 返回 &'static str，直接进闭包即可）
        match action {
            PendingAction::DeleteSession(id) => {
                let body = serde_json::json!({ "session_id": id });
                self.go(
                    move |c| c.delete_json(&format!("/api/sessions/{id}"), &body),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("删除记录").into()),
                );
            }
            PendingAction::RestoreFromTrash(id) => {
                let body = serde_json::json!({ "id": id });
                self.go(
                    move |c| c.post_json("/api/trash/restore", &body),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("恢复记录").into()),
                );
            }
            PendingAction::DeleteFromTrash(id) => {
                let body = serde_json::json!({ "id": id });
                self.go(
                    move |c| c.post_json("/api/trash/delete", &body),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("永久删除").into()),
                );
            }
            PendingAction::ClearTrash => {
                self.go(
                    |c| c.post_json("/api/trash/clear", &serde_json::json!({})),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("清空回收站").into()),
                );
            }
            PendingAction::RestoreBackup(name) => {
                let body = serde_json::json!({ "filename": name });
                self.go(
                    move |c| c.post_json("/api/backup/restore", &body),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("恢复备份").into()),
                );
            }
            PendingAction::DeleteBackup(name) => {
                let body = serde_json::json!({ "filename": name });
                self.go(
                    move |c| c.delete_json("/api/delete-backup", &body),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("删除备份").into()),
                );
            }
            PendingAction::CleanBackups => {
                self.go(
                    |c| c.post_json("/api/clean-backups", &serde_json::json!({ "keep": 10 })),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("清理旧备份").into()),
                );
            }
            PendingAction::ClearData => {
                self.go(
                    |c| c.post_json("/api/clear", &serde_json::json!({})),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("清空数据").into()),
                );
            }
            PendingAction::MergeSessions => {
                let ids: Vec<String> = self.selected.iter().cloned().collect();
                let body = serde_json::json!({ "ids": ids });
                self.go(
                    move |c| c.post_json("/api/merge-sessions", &body),
                    |r| AppMsg::Action(r.map(|_| ()), crate::t!("合并记录").into()),
                );
                self.selected.clear();
            }
        }
    }

    // ---------- 导入/导出 ----------

    pub fn export_csv(&mut self, sessions: &[BootSession]) {
        let csv = sessions_to_csv(sessions);
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("boot-records.csv")
            .add_filter("CSV", &["csv"])
            .save_file()
        {
            match std::fs::write(&path, csv) {
                Ok(_) => self.notify(crate::t!("已导出 CSV")),
                Err(e) => self.notify(&crate::tf!("导出失败: {e}", e)),
            }
        }
    }

    pub fn export_xlsx(&mut self, sessions: &[BootSession]) {
        match sessions_to_xlsx(sessions) {
            Ok(buf) => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("boot-records.xlsx")
                    .add_filter("Excel", &["xlsx"])
                    .save_file()
                {
                    match std::fs::write(&path, buf) {
                        Ok(_) => self.notify(crate::t!("已导出 XLSX")),
                        Err(e) => self.notify(&crate::tf!("导出失败: {e}", e)),
                    }
                }
            }
            Err(e) => self.notify(&crate::tf!("生成 XLSX 失败: {e}", e)),
        }
    }

    pub fn import_json(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<DataResponse>(&text) {
                    Ok(d) => {
                        let body = serde_json::to_value(&d).unwrap_or(serde_json::json!({}));
                        self.go(
                            move |c| c.post_json("/api/data", &body),
                            |r| AppMsg::Action(r.map(|_| ()), crate::t!("导入数据").into()),
                        );
                    }
                    Err(e) => self.notify(&crate::tf!("JSON 解析失败: {e}", e)),
                },
                Err(e) => self.notify(&crate::tf!("读取文件失败: {e}", e)),
            }
        }
    }

    // ---------- 工具 ----------

    fn handle(&mut self, m: AppMsg, ctx: &egui::Context) {
        match m {
            AppMsg::DataLoaded(r) => {
                self.loading = false;
                match r {
                    Ok(d) => self.data = Some(d),
                    Err(e) => self.notify_kind(&crate::tf!("数据加载失败: {e}", e), ToastKind::Err),
                }
            }
            AppMsg::OverviewLoaded(r) => {
                if let Ok(o) = r {
                    self.overview = Some(o);
                }
            }
            AppMsg::TrendLoaded(r) => {
                if let Ok(t) = r {
                    self.trend = Some(t);
                }
            }
            AppMsg::DailyLoaded(r) => {
                if let Ok(d) = r {
                    self.daily = Some(d.data);
                }
            }
            AppMsg::WeeklyLoaded(r) => {
                if let Ok(w) = r {
                    self.weekly = Some(w.data);
                }
            }
            AppMsg::AnomaliesLoaded(r) => {
                if let Ok(a) = r {
                    self.anomalies = Some(a);
                }
            }
            AppMsg::SettingsLoaded(r) => match r {
                Ok(s) => self.settings = Some(s),
                Err(e) => self.notify_kind(&crate::tf!("设置加载失败: {e}", e), ToastKind::Err),
            },
            AppMsg::LanguageLoaded(r) => {
                if let Ok(l) = r {
                    // 语言是跨端共享的：web 端改过之后 desktop 也要跟随。
                    // 只回写 self.settings 里的语言字段，绝不整份替换——
                    // 这条轮询恰恰是在「设置-网络」页（用户正在编辑 Token）时还在跑的。
                    let code = l.language.clone();
                    if let Some(s) = self.settings.as_mut() {
                        s.language = code.clone();
                    }
                    self.apply_language(ctx, &code);
                }
            }
            AppMsg::VersionLoaded(r) => {
                if let Ok(v) = r {
                    self.version = Some(v);
                }
            }
            AppMsg::TrashLoaded(r) => {
                if let Ok(t) = r {
                    self.trash = t.sessions;
                }
            }
            AppMsg::BackupsLoaded(r) => {
                if let Ok(b) = r {
                    self.backups = b.backups;
                }
            }
            AppMsg::TunnelLoaded(r) => {
                if let Ok(t) = r {
                    self.tunnel_url = t.url;
                }
            }
            AppMsg::TunnelStatusLoaded(r) => match r {
                Ok(s) => {
                    if !s.url.is_empty() {
                        self.tunnel_url = Some(s.url.clone());
                    } else if !s.running {
                        self.tunnel_url = None;
                    }
                    self.tunnel_status = Some(s);
                }
                Err(e) => {
                    // 拉状态都失败（后端没起来 / 旧版本没有这个路由）时别静默
                    self.notify_kind(&crate::tf!("隧道状态获取失败: {e}", e), ToastKind::Err);
                }
            },
            AppMsg::Action(r, what) => match r {
                Ok(_) => {
                    let ok_msg = format!("{what}成功");
                    self.notify_kind(
                        crate::i18n::tr_or(crate::t!("{what}成功"), &ok_msg),
                        ToastKind::Ok,
                    );
                    self.refresh_data();
                    if what.contains("备份") || what.contains("回收站") {
                        self.load_trash();
                        self.load_backups();
                    }
                }
                Err(e) => {
                    let fail_msg = format!("{what}失败: {e}");
                    self.notify_kind(
                        crate::i18n::tr_or(crate::t!("{what}失败: {e}"), &fail_msg),
                        ToastKind::Err,
                    );
                }
            },
            AppMsg::WidgetToggled(r) => match r {
                Ok(on) => {
                    if let Some(c) = &self.widget_check {
                        c.set_checked(on);
                    }
                    if let Some(s) = &mut self.settings {
                        s.widget_enabled = on;
                    }
                    // 状态词与模板都要过 i18n，且**必须走占位符替换**：
                    // 英文模板是 `Desktop widget {}`，直接把状态词拼在后面会得到
                    // `Desktop widgetOn`（中文没空格、英文要空格）。
                    let state = crate::t!(if on { "开启" } else { "关闭" });
                    let template = crate::t!("桌面组件已{}");
                    let fallback = format!("桌面组件已{state}");
                    let msg = if crate::i18n::is_en() {
                        crate::i18n::format_placeholders(template, &[&state])
                    } else {
                        fallback
                    };
                    self.notify_kind(&msg, ToastKind::Ok);
                }
                Err(e) => self.notify_kind(&crate::tf!("组件切换失败: {e}", e), ToastKind::Err),
            },
        }
    }

    /// 是否使用 12 小时制（读用户设置，缺省 24h）。
    pub fn hour12(&self) -> bool {
        self.settings
            .as_ref()
            .map(|s| s.time_format == "12h")
            .unwrap_or(false)
    }

    /// 过滤 + 排序后的记录（记录页与导出共用）
    pub(crate) fn filtered_sessions(&self) -> Vec<BootSession> {
        let ft = self.filter_text.trim().to_lowercase();
        let sessions = self
            .data
            .as_ref()
            .map(|d| &d.sessions)
            .cloned()
            .unwrap_or_default();
        let mut out: Vec<BootSession> = sessions
            .into_iter()
            .filter(|s| {
                if !ft.is_empty() && !s.id.to_lowercase().contains(&ft) {
                    return false;
                }
                // 过滤用日期（不含小时），不受 12/24 小时制影响，可稳定做字符串比较
                if !self.filter_from.is_empty() && fmt::fmt_date(&s.boot_time) < self.filter_from {
                    return false;
                }
                if !self.filter_to.is_empty() && fmt::fmt_date(&s.boot_time) > self.filter_to {
                    return false;
                }
                true
            })
            .collect();
        // 排序：开机时间/关机时间按 ISO 字符串比较（UTC 同格式可直接比），
        // 时长按毫秒；进行中（无关机时间/无时长）恒排在最后。
        let key = self.sort_key;
        out.sort_by(|a, b| {
            let ord = match key {
                SortKey::Boot => a.boot_time.cmp(&b.boot_time),
                SortKey::Shutdown => {
                    let av = a.shutdown_time.clone().unwrap_or_default();
                    let bv = b.shutdown_time.clone().unwrap_or_default();
                    match (av.is_empty(), bv.is_empty()) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => av.cmp(&bv),
                    }
                }
                SortKey::Duration => a.duration.unwrap_or(-1).cmp(&b.duration.unwrap_or(-1)),
            };
            if self.sort_asc {
                ord
            } else {
                ord.reverse()
            }
        });
        out
    }

    // ---------- 主循环 ----------

    pub fn app_update(&mut self, ctx: &Context) {
        // 托盘动作
        while let Ok(a) = self.tray_rx.try_recv() {
            match a {
                TrayAction::Show => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true)),
                TrayAction::Hide => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false)),
                TrayAction::Quit => {
                    // 先置位，再请求关闭：下一帧的 close_requested 才不会又被 CancelClose 取消
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                TrayAction::ToggleWidget => self.toggle_widget(),
            }
        }

        // 懒初始化托盘（事件循环已就绪）
        if !self.tray_ready {
            if let Some((t, c)) = tray::build_tray(self.tray_tx.clone(), crate::icon::tray_icon()) {
                self.tray = Some(t);
                self.widget_check = Some(c);
            }
            self.tray_ready = true;
        }

        // 关闭窗口 → 隐藏到托盘（与原 Tauri 行为一致）；
        // 但托盘菜单的「退出」必须能真正结束进程，否则点退出只是窗口消失、
        // 桌面端仍在后台运行，而一直存在的托盘进程也会让 watchdog 永远看不到退出。
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.quitting {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

        // 处理异步消息，并据此维护连接健康度（顶栏状态灯三态）
        let now = ctx.input(|i| i.time);
        while let Ok(m) = self.rx.try_recv() {
            if failed(&m) {
                self.err_streak = self.err_streak.saturating_add(1);
            } else {
                self.err_streak = 0;
                self.last_ok = now;
            }
            self.handle(m, ctx);
        }

        // 待显示 toast（按语义上色）
        if let Some((m, kind)) = self.pending_toast.take() {
            self.toast = Some((m, kind, now + 2.6));
        }

        // 应用主题：对齐 web UI Dark Gallery 配色（背景/文字/主色随 app_mode + app_theme）
        if let Some(s) = &self.settings {
            let dark = s.app_mode != "light";
            let theme = s.app_theme.clone();
            let key = (dark, theme.clone());
            if self.applied_theme != Some(key) {
                ctx.set_style(crate::theme::style(&crate::theme::palette(dark, &theme)));
                self.applied_theme = Some((dark, theme));
            }
            // 界面缩放：与 web 端 uiScale 同源（settings.uiScale），改完立即生效
            let scale = s.ui_scale.clamp(0.5, 2.0);
            if self.applied_scale != Some(scale) {
                ctx.set_zoom_factor(scale);
                self.applied_scale = Some(scale);
            }
            // 界面语言：与 web 端 language 同源。改完立即重绘并更新窗口标题，
            // 但托盘菜单文案由 OS 持有，需要重启才更新（见 tray.rs 注释）。
            // 注意这里读的是 `self.settings`，而停留在「设置-网络」页时整份设置
            // 是**不拉取**的（见下面 editing_network），所以语言单独走一条轮询，
            // 否则在网络页改语言（或从 web 端改语言后正好停在该页）永远不生效。
            // clone 一次是为了结束 `self.settings` 的不可变借用。
            let lang_code = s.language.clone();
            self.apply_language(ctx, &lang_code);
        }

        // 语言轮询：独立于整份设置，周期 2s（用户可能正停在网络页编辑 Token，
        // 那条路径不能拉整份设置，否则会覆盖尚未提交的输入）。
        if now - self.last_lang_poll > 2.0 {
            self.last_lang_poll = now;
            self.poll_language();
        }

        // 周期拉取设置（同步主题/组件开关）
        if now - self.last_settings_poll > 15.0 {
            self.last_settings_poll = now;
            // 用户停留在「设置-网络」页编辑 Token/自定义域名时跳过，
            // 否则后台整体替换 self.settings 会覆盖正在输入、尚未 lost_focus 提交的内容
            let editing_network = self.page == Page::Settings && self.settings_tab == 2;
            if !editing_network {
                self.spawn_settings();
            }
        }

        // 隧道状态轮询：只在「设置-网络」页做（2s 一次）。
        // 刚启动隧道时 URL 要等 cloudflared 打印出来，下载进度也要靠这里刷新，
        // 所以这个轮询是「一键临时隧道」体验的关键，不能省。
        if self.page == Page::Settings && self.settings_tab == 2 && now - self.last_tunnel_poll > 2.0 {
            self.last_tunnel_poll = now;
            self.load_tunnel_status();
        }

        // 首帧尝试居中（显示器信息可能稍后才可用，失败则下一帧重试）
        if !self.centered {
            if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(cmd);
                self.centered = true;
            }
        }

        // 首次加载
        if !self.initialized {
            self.initialized = true;
            self.refresh_all();
            self.load_stats();
            self.load_trash();
            self.load_backups();
            self.load_tunnel_status();
        }

        // 全局快捷键：Esc 关确认弹窗、Ctrl+R 刷新、Ctrl+F 聚焦搜索
        self.handle_shortcuts(ctx);

        // 保持实时刷新（本次会话计时）
        ctx.request_repaint_after(Duration::from_secs(1));
        // 入场动画期间需要逐帧重绘（否则只有 1s 一次，动画会卡成幻灯片）
        if self.animating(ctx) {
            ctx.request_repaint();
        }

        self.render(ctx);
    }

    /// 在系统默认浏览器中打开 Web 端页面。
    ///
    /// 桌面端与 Web 端共用同一个后端（`self.base` 即 `http://127.0.0.1:<port>`），
    /// 所以直接把该地址交给系统 shell 打开即可，无需额外依赖。
    pub fn open_web(&mut self) {
        let url = self.base.clone();
        #[cfg(target_os = "windows")]
        let spawned = std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn();
        #[cfg(target_os = "macos")]
        let spawned = std::process::Command::new("open").arg(&url).spawn();
        #[cfg(all(unix, not(target_os = "macos")))]
        let spawned = std::process::Command::new("xdg-open").arg(&url).spawn();
        match spawned {
            Ok(_) => self.notify_kind(crate::t!("已在浏览器中打开 Web 端"), ToastKind::Ok),
            Err(e) => self.notify_kind(&crate::tf!("打开浏览器失败：{e}", e), ToastKind::Err),
        }
    }

    /// 界面缩放（0.5×~2.0×）：写回 `settings.uiScale` 并立即应用。
    ///
    /// 注意**不要**在这里写 `self.applied_scale`：缩放必须由 `app_update()` 里
    /// 的「变化检测 → ctx.set_zoom_factor()」统一驱动。早期版本在这里提前把
    /// `applied_scale` 置成新值，导致帧循环里 `applied_scale != Some(scale)` 恒为假，
    /// `set_zoom_factor` 永远不被调用——滑块能拖、数值能存，但界面纹丝不动。
    pub fn set_ui_scale(&mut self, scale: f32) {
        let v = (scale.clamp(0.5, 2.0) * 100.0).round() / 100.0;
        if let Some(s) = self.settings.as_mut() {
            s.ui_scale = v;
        }
        self.save_setting("uiScale", serde_json::json!(v), crate::t!("界面缩放"));
    }

    /// 全局快捷键（在页面渲染前处理，避免与输入框冲突）
    fn handle_shortcuts(&mut self, ctx: &Context) {
        let (esc, ctrl_r, ctrl_f, zoom_in, zoom_out, zoom_reset, wheel) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.modifiers.command && i.key_pressed(egui::Key::R),
                i.modifiers.command && i.key_pressed(egui::Key::F),
                i.modifiers.command
                    && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)),
                i.modifiers.command && i.key_pressed(egui::Key::Minus),
                i.modifiers.command && i.key_pressed(egui::Key::Num0),
                // Ctrl+滚轮（触控板捏合也会变成带 ctrl 的滚轮事件）
                if i.modifiers.command {
                    i.raw_scroll_delta.y + i.smooth_scroll_delta.y
                } else {
                    0.0
                },
            )
        });
        if esc && self.pending.is_some() {
            self.pending = None;
        }
        if ctrl_r {
            self.refresh_data();
            self.notify_kind(crate::t!("正在刷新数据"), ToastKind::Info);
        }
        if ctrl_f {
            self.page = Page::Records;
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("records_search")));
        }
        let cur = self
            .settings
            .as_ref()
            .map(|s| s.ui_scale)
            .unwrap_or(1.0);
        if zoom_in {
            self.set_ui_scale(cur + 0.1);
        } else if zoom_out {
            self.set_ui_scale(cur - 0.1);
        } else if zoom_reset {
            self.set_ui_scale(1.0);
        } else if wheel.abs() > 0.5 {
            let step = if wheel > 0.0 { 0.05 } else { -0.05 };
            let target = ((cur + step) * 20.0).round() / 20.0;
            self.set_ui_scale(target);
        }
    }

    fn render(&mut self, ctx: &Context) {
        // 页面切换 → 重置入场起始时刻
        if self.anim_page != self.page {
            self.anim_page = self.page;
            self.anim_start = crate::anim::now(ctx);
        }

        let p = self.palette();
        // 外壳用 bg_bar（web 里 Sider/Header 即 var(--bg-card)），中央内容区才是 bg_page，
        // 这样才有 web 的层级感：侧栏/顶栏比页面底亮一档，边界靠 1px 细线区分。
        let bar_stroke = egui::Stroke::new(1.0_f32, p.border_light);

        // ---- 侧栏：对齐 web 的悬浮 Sider ----
        // web 端 Sider 是 position:fixed + 内容区固定留白 64px：悬浮展开时侧栏浮在
        // 内容之上覆盖，而不是把内容挤走。egui 的 SidePanel 会占据布局、把 CentralPanel
        // 推窄，鼠标一划过就整页位移，所以这里改用 Area 浮层 + 宽度缓动复刻 web 行为。
        // 用 latest_pos 而不是 hover_pos：hover_pos 在指针刚进入窗口、或窗口
        // 被系统移动/未持有指针所有权时会返回 None，判定会晚一帧甚至完全失灵
        // （表现为「鼠标压在侧栏上也不展开」）。latest_pos 只要收到过移动事件就有值。
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        let avail = ctx.available_rect();
        // 滞回：折叠态在 64px 判定区内展开；展开后判定区加宽到整个 208px 侧栏区域，
        // 指针在展开区域内移动时不会因为滑出 64px 而突然收起。
        let zone_w = if self.sidebar_hover {
            crate::theme::SIDEBAR_W
        } else {
            crate::theme::SIDEBAR_W_COLLAPSED + 8.0
        };
        let over_rail = pointer
            .map(|pos| pos.x <= zone_w && pos.y >= avail.top() && pos.y <= avail.bottom())
            .unwrap_or(false);
        self.sidebar_hover = over_rail;

        let target = if !self.sidebar_collapsed || self.sidebar_hover {
            1.0
        } else {
            0.0
        };
        // 指数缓动：与帧率无关，收尾时吸附到目标值
        let step = 1.0 - (-13.0 * ctx.input(|i| i.stable_dt).clamp(0.001, 0.05)).exp();
        self.sidebar_anim += (target - self.sidebar_anim) * step;
        if (target - self.sidebar_anim).abs() < 0.002 {
            self.sidebar_anim = target;
        }
        let anim = self.sidebar_anim;
        let width = crate::theme::SIDEBAR_W_COLLAPSED
            + (crate::theme::SIDEBAR_W - crate::theme::SIDEBAR_W_COLLAPSED) * anim;
        // 展开过半即按展开态排版文字（文字被面板裁剪，随宽度一起"滑"出来）
        let expanded = anim > 0.45;
        if (target - anim).abs() > 0.0005 {
            ctx.request_repaint();
        }

        let bar_bottom = avail.bottom();
        egui::Area::new(egui::Id::new("sidebar"))
            .order(egui::Order::Middle)
            .fixed_pos(egui::pos2(0.0, 0.0))
            .interactable(true)
            .show(ctx, |ui| {
                let rect = egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(width, bar_bottom),
                );
                // 浮层不参与布局：显式占位，内容超出宽度由 Area 裁剪
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());
                self.sidebar_rect = Some(rect);
                egui::Frame::none()
                    .fill(p.bg_bar)
                    .stroke(bar_stroke)
                    // 左右只留 4px：折叠态可用宽度 = 64 - 8，正好容纳 48px 导航项 + 余量
                    .inner_margin(Margin {
                        left: 4.0,
                        right: 10.0,
                        top: 10.0,
                        bottom: 10.0,
                    })
                    .show(ui, |ui| {
                        self.render_sidebar(ui, expanded, anim);
                    });
            });

        // ---- 顶栏 + 中央：左侧固定留出折叠态宽度，展开时侧栏覆盖其上 ----
        let content_margin = Margin {
            left: crate::theme::SIDEBAR_W_COLLAPSED,
            ..Margin::symmetric(18.0, 6.0)
        };
        egui::TopBottomPanel::top("topbar")
            .frame(
                egui::Frame::none()
                    .fill(p.bg_bar)
                    .stroke(bar_stroke)
                    .inner_margin(content_margin),
            )
            .show(ctx, |ui| self.render_topbar(ui));

        // 中央内容区：只按折叠宽度留白（web 的 marginLeft: SIDER_COLLAPSED_WIDTH），
        // 侧栏展开时浮在其上，内容不重排、不抖动。
        //
        // 显式 fill(bg_page)：桌面端页面底与卡片底不同色（浅色下页面底比纯白卡片深一档），
        // 不填的话中央区会沿用 panel_fill，卡片与页面底糊在一起、看不出卡片边界。
        let central_frame = egui::Frame::none()
            .fill(p.bg_page)
            .inner_margin(Margin {
                left: crate::theme::SIDEBAR_W_COLLAPSED + 18.0,
                right: 18.0,
                top: 10.0,
                bottom: 10.0,
            });
        egui::CentralPanel::default().frame(central_frame).show(ctx, |ui| {
            // 页面入场：从下方 20px 滑上来（对齐 web 的 riseIn）。
            // 这里刻意不再叠加 ui.set_opacity 淡入：整帧透明度会让 KPI hero 卡的
            // accent 低透明度底与页面底混合发白，而卡内文字又是接近纯白的前景色，
            // 实测「本次会话」卡在暗色下几乎读不出内容。位移动画已足够表达入场。
            let t = self.enter_t(ctx);
            if t < 1.0 {
                let dy = crate::anim::RISE_OFFSET * (1.0 - t);
                if dy > 0.5 {
                    ui.add_space(dy);
                }
            }
            egui::ScrollArea::vertical().show(ui, |ui| match self.page {
                Page::Dashboard => crate::pages::dashboard(ui, self),
                Page::Records => crate::pages::records(ui, self),
                Page::Charts => crate::pages::charts(ui, self),
                Page::Settings => crate::pages::settings(ui, self),
                Page::Admin => crate::pages::admin(ui, self),
            });
        });

        // Toast：顶部居中，按语义（成功/提示/失败）上色，2.6s 后消失，点击立即关闭
        #[cfg(feature = "debug-hud")]
        {
            // 侧栏悬浮判定依赖指针事件，出问题时很难从画面推断，所以留一个按需开启的
            // 诊断浮层：BOOTTRACKER_DEBUG_HUD=1 启动（需 --features debug-hud 构建）。
            if std::env::var("BOOTTRACKER_DEBUG_HUD").is_ok() {
                let dbg_pos = ctx.input(|i| i.pointer.latest_pos());
                let dbg_hover = ctx.input(|i| i.pointer.hover_pos());
                let dbg_zoom = ctx.zoom_factor();
                let dbg_ppp = ctx.pixels_per_point();
                let dbg_mode = self
                    .settings
                    .as_ref()
                    .map(|s| format!("{} {}", s.app_mode, s.app_theme))
                    .unwrap_or_else(|| "-".to_string());
                egui::Area::new(egui::Id::new("dbg_hud"))
                    .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 8.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_black_alpha(220))
                            .inner_margin(Margin::same(6.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "L={dbg_pos:?}\nH={dbg_hover:?}\nz={zone_w:.0} h={} a={:.2}\nzoom={dbg_zoom:.2} ppp={dbg_ppp:.2}\n{d}",
                                        self.sidebar_hover as u8, self.sidebar_anim, d = dbg_mode
                                    ))
                                    .size(11.0)
                                    .color(egui::Color32::YELLOW),
                                );
                            });
                    });
            }
        }
        if let Some((msg, kind, until)) = self.toast.clone() {
            if ctx.input(|i| i.time) > until {
                self.toast = None;
            } else {
                let p = self.palette();
                let (color, icon) = match kind {
                    ToastKind::Ok => (p.success, egui_phosphor::regular::CHECK_CIRCLE),
                    ToastKind::Info => (p.accent, egui_phosphor::regular::INFO),
                    ToastKind::Err => (p.danger, egui_phosphor::regular::WARNING_OCTAGON),
                };
                let dismissed = egui::Area::new(egui::Id::new("toast"))
                    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 18.0))
                    .order(egui::Order::Foreground)
                    .interactable(true)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(crate::theme::tinted_fill(&p, color, 0.16))
                            .stroke(egui::Stroke::new(1.0_f32, color))
                            .rounding(Rounding::same(crate::theme::R_CARD))
                            .inner_margin(Margin::symmetric(16.0, 10.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!("{icon}  {msg}"))
                                        .color(p.text_primary)
                                        .size(15.0),
                                );
                            })
                            .response
                            .clicked()
                    })
                    .inner;
                if dismissed {
                    self.toast = None;
                }
            }
        }

        // 确认弹窗（危险操作用警示色 + 影响说明）
        if self.pending.is_some() {
            let p = self.palette();
            let (label, danger) = self.pending_desc();
            let color = if danger { p.danger } else { p.accent };
            let icon = if danger {
                egui_phosphor::regular::WARNING_OCTAGON
            } else {
                egui_phosphor::regular::QUESTION
            };
            let mut open = true;
            egui::Window::new(crate::t!("确认操作"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).size(22.0).color(color));
                        ui.label(
                            egui::RichText::new(label.as_str())
                                .size(16.0)
                                .color(p.text_primary),
                        );
                    });
                    if danger {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(crate::t!("该操作不可撤销，请确认后继续。"))
                                .size(13.0)
                                .color(p.text_muted),
                        );
                    }
                    ui.add_space(6.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                egui::RichText::new(if danger {
                        crate::t!("确认删除")
                    } else {
                        crate::t!("确认")
                    })
                                    .color(color),
                            )
                            .clicked()
                        {
                            self.do_pending();
                        }
                        if ui.button(crate::t!("取消")).clicked() {
                            self.pending = None;
                        }
                    });
                });
            if !open {
                self.pending = None;
            }
        }
    }

    /// 确认弹窗文案 + 是否属于破坏性操作
    fn pending_desc(&self) -> (String, bool) {
        match &self.pending {
            Some(PendingAction::DeleteSession(_)) => (crate::t!("确认将该记录移入回收站？").into(), false),
            Some(PendingAction::RestoreFromTrash(_)) => (crate::t!("确认从回收站恢复该记录？").into(), false),
            Some(PendingAction::DeleteFromTrash(_)) => {
                (crate::t!("确认永久删除该记录？").into(), true)
            }
            Some(PendingAction::ClearTrash) => (crate::t!("确认清空回收站？").into(), true),
            Some(PendingAction::RestoreBackup(_)) => {
                (crate::t!("确认恢复该备份？将覆盖当前全部数据。").into(), true)
            }
            Some(PendingAction::DeleteBackup(_)) => (crate::t!("确认删除该备份文件？").into(), true),
            Some(PendingAction::CleanBackups) => {
                (crate::t!("确认清理旧备份（保留最近 10 个）？").into(), true)
            }
            Some(PendingAction::ClearData) => (crate::t!("确认清空全部数据？").into(), true),
            Some(PendingAction::MergeSessions) => (
                format!("确认合并选中的 {} 条记录？", self.selected.len()),
                false,
            ),
            None => (String::new(), false),
        }
    }

    /// `expanded` 为「当前是否处于展开态」——折叠态悬浮时也会临时为 true，
    /// 因此不能直接用 `self.sidebar_collapsed` 判断要不要显示文字。
    /// `anim` 是 0→1 的展开进度，用于让文字/图标随宽度一起淡入。
    ///
    /// 导航项对齐 web 的 `.ant-menu-item`：常态透明、hover 淡 accent 底、
    /// 选中 accent 底 + 左侧 3px 竖条 + 图标与文字染 accent。
    fn render_sidebar(&mut self, ui: &mut egui::Ui, expanded: bool, anim: f32) {
        let p = self.palette();
        ui.add_space(6.0);
        // 品牌区：折叠态只留图标，展开态图标 + 名称（名称随动画淡入）
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(egui_phosphor::regular::GAUGE)
                    .size(if expanded { 22.0 } else { 24.0 })
                    .color(p.accent),
            );
            if anim > 0.02 {
                // 名称随展开进度淡入；egui 的 RichText 没有 fade_in，用
                // text_primary 与背景的 alpha 混合模拟（overlay 保证不透明）
                let name_color = crate::theme::overlay(p.bg_bar, p.text_primary, anim);                let mut name = egui::RichText::new(crate::t!("开机记录"))
                    .size(18.0)
                    .strong()
                    .color(name_color);
                if expanded {
                    name = name.strong();
                }
                ui.add(egui::Label::new(name).selectable(false));
            }
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        let pages = [
            Page::Dashboard,
            Page::Records,
            Page::Charts,
            Page::Settings,
            Page::Admin,
        ];
        // 折叠态（或动画早期）用 tooltip 补名字，展开后不再弹
        let show_tip = !expanded;
        for page in pages {
            let selected = self.page == page;
            let resp = crate::components::nav_item(
                ui,
                &p,
                page.icon(),
                page.label(),
                selected,
                expanded,
                if expanded { 1.0 } else { anim.clamp(0.0, 1.0) },
            );
            // 点击导航即收起临时展开，避免浮层一直挡着内容
            let resp = if show_tip {
                resp.on_hover_text(page.label())
            } else {
                resp
            };
            if resp.clicked() && !selected {
                self.page = page;
                if !self.sidebar_collapsed {
                    self.sidebar_hover = false;
                }
                // 进入某些页时主动拉数据
                if page == Page::Charts {
                    self.load_stats();
                }
                if page == Page::Admin {
                    self.load_trash();
                    self.load_backups();
                    if !self.tunnel_checked {
                        self.load_tunnel();
                        self.tunnel_checked = true;
                    }
                }
            }
        }

        // 底部：展开时给一个「固定展开」开关，允许不用悬浮也常驻展开
        if expanded {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(4.0);
                let pinned = !self.sidebar_collapsed;
                let icon = if pinned {
                    egui_phosphor::regular::PUSH_PIN
                } else {
                    egui_phosphor::regular::PUSH_PIN_SLASH
                };
                if crate::components::icon_button(ui, icon, if pinned { crate::t!("取消固定") } else { crate::t!("固定展开") })
                    .clicked()
                {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                    self.sidebar_hover = false;
                }
            });
        }
    }

    fn render_topbar(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        ui.set_min_height(crate::theme::TOPBAR_H - 12.0);
        ui.horizontal(|ui| {
            // 标题：图标 + 页面名（与侧栏呼应）
            ui.label(
                egui::RichText::new(self.page.icon())
                    .size(20.0)
                    .color(p.accent),
            );
            ui.label(
                egui::RichText::new(self.page.label())
                    .size(20.0)
                    .strong()
                    .color(p.text_primary),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 主题切换：亮/暗一键互换
                let dark = self
                    .settings
                    .as_ref()
                    .map(|s| s.app_mode != "light")
                    .unwrap_or(true);
                let theme_icon = if dark {
                    egui_phosphor::regular::SUN
                } else {
                    egui_phosphor::regular::MOON
                };
                if crate::components::icon_button(ui, theme_icon, "")
                    .on_hover_text(if dark { crate::t!("切换到亮色") } else { crate::t!("切换到暗色") })
                    .clicked()
                {
                    let new_mode = if dark { "light" } else { "dark" };
                    if let Some(s) = &mut self.settings {
                        s.app_mode = new_mode.to_string();
                    }
                    self.set_theme(new_mode);
                }
                // 在系统默认浏览器里打开 Web 端（同一后端 REST 服务，端口取自 BOOTTRACKER_PORT）
                if crate::components::icon_button(
                    ui,
                    egui_phosphor::regular::BROWSER,
                    "",
                )
                .on_hover_text(crate::t!("在浏览器中打开 Web 端"))
                .clicked()
                {
                    self.open_web();
                }
                // 桌面小组件开关（复用现成 /api/widget-toggle，状态由后端真实返回）
                let widget_on = self
                    .settings
                    .as_ref()
                    .map(|s| s.widget_enabled)
                    .unwrap_or(false);
                let w_icon = if widget_on {
                    egui_phosphor::regular::SQUARES_FOUR
                } else {
                    egui_phosphor::regular::BROWSERS
                };
                let w_resp = crate::components::icon_button(ui, w_icon, "");
                let w_resp = if widget_on {
                    w_resp.on_hover_text(crate::t!("关闭桌面小组件"))
                } else {
                    w_resp.on_hover_text(crate::t!("开启桌面小组件"))
                };
                if w_resp.clicked() {
                    self.toggle_widget();
                }
                if let Some(v) = &self.version {
                    ui.label(
                        egui::RichText::new(format!("v{}", v.version))
                            .size(13.0)
                            .color(p.text_muted)
                            .monospace(),
                    );
                }
                // 连接状态三态：正常 / 连接中 / 失联
                let now = ui.input(|i| i.time);
                let (text, color, icon) = if self.data.is_none() && self.err_streak == 0 {
                    (crate::t!("连接中"), p.text_muted, egui_phosphor::regular::PLUG)
                } else if self.err_streak >= 3 || (self.last_ok > 0.0 && now - self.last_ok > 20.0)
                {
                    (crate::t!("连接失败"), p.danger, egui_phosphor::regular::PLUGS)
                } else {
                    (
                        crate::t!("已连接"),
                        p.success,
                        egui_phosphor::regular::PLUGS_CONNECTED,
                    )
                };
                crate::components::status_pill(ui, &p, icon, text, color)
                    .on_hover_text(crate::t!("后端 REST 服务状态（127.0.0.1:18792）"));
            });
        });
    }

    /// 把确认弹窗之外的普通提示按「信息」处理（兼容既有调用点）
    pub fn notify(&mut self, msg: &str) {
        self.notify_kind(msg, ToastKind::Info);
    }

    /// 带语义的提示（成功/信息/失败），决定 toast 的配色与图标
    pub fn notify_kind(&mut self, msg: &str, kind: ToastKind) {
        self.pending_toast = Some((msg.to_string(), kind));
    }
}

// ---------- CSV / XLSX ----------

/// 该消息是否代表一次失败（用于连接健康度统计）
fn failed(m: &AppMsg) -> bool {
    // 每个变体的 Result 泛型不同，不能用 | 拼成同一模式，逐个展开
    match m {
        AppMsg::DataLoaded(r) => r.is_err(),
        AppMsg::OverviewLoaded(r) => r.is_err(),
        AppMsg::TrendLoaded(r) => r.is_err(),
        AppMsg::DailyLoaded(r) => r.is_err(),
        AppMsg::WeeklyLoaded(r) => r.is_err(),
        AppMsg::AnomaliesLoaded(r) => r.is_err(),
        AppMsg::SettingsLoaded(r) => r.is_err(),
        AppMsg::VersionLoaded(r) => r.is_err(),
        AppMsg::TrashLoaded(r) => r.is_err(),
        AppMsg::BackupsLoaded(r) => r.is_err(),
        AppMsg::TunnelLoaded(r) => r.is_err(),
        AppMsg::TunnelStatusLoaded(r) => r.is_err(),
        AppMsg::LanguageLoaded(r) => r.is_err(),
        AppMsg::WidgetToggled(r) => r.is_err(),
        AppMsg::Action(r, _) => r.is_err(),
    }
}

fn sessions_to_csv(sessions: &[BootSession]) -> String {
    let mut s = String::from("id,bootTime,shutdownTime,durationMs\n");
    for x in sessions {
        s.push_str(&format!(
            "{},{},{},{}\n",
            x.id,
            x.boot_time,
            x.shutdown_time.clone().unwrap_or_default(),
            x.duration.unwrap_or(0)
        ));
    }
    s
}

/// 生成 xlsx 二进制内容
///
/// 注意：calamine 只是**读取**库，不带写能力，因此这里用 rust_xlsxwriter。
fn sessions_to_xlsx(sessions: &[BootSession]) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::Workbook;

    let mut wb = Workbook::new();
    {
        let ws = wb.add_worksheet();
        ws.set_name(crate::t!("记录")).map_err(|e| e.to_string())?;

        let headers = ["id", "bootTime", "shutdownTime", "durationMs"];
        for (c, h) in headers.iter().enumerate() {
            ws.write(0, c as u16, *h).map_err(|e| e.to_string())?;
        }
        for (r, x) in sessions.iter().enumerate() {
            let row = (r + 1) as u32;
            ws.write(row, 0, x.id.as_str()).map_err(|e| e.to_string())?;
            ws.write(row, 1, x.boot_time.as_str())
                .map_err(|e| e.to_string())?;
            let sd = x.shutdown_time.clone().unwrap_or_default();
            ws.write(row, 2, sd.as_str()).map_err(|e| e.to_string())?;
            ws.write(row, 3, x.duration.unwrap_or(0))
                .map_err(|e| e.to_string())?;
        }
    }
    wb.save_to_buffer().map_err(|e| e.to_string())
}

impl eframe::App for BootTrackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app_update(ctx);
    }
}
