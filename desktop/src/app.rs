//! 纯原生桌面应用主体（egui/eframe）
//!
//! 替代原 Tauri WebView 壳 + Leptos/WASM 方案：无 WebView、无 HTML，
//! 全部用 egui 即时模式原生渲染；通过 HTTP 调用 Python 后端 REST API。

use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use egui::{Color32, Context};
use serde::de::DeserializeOwned;

use crate::api::{
    Anomalies, AppMsg, AppSettings, BootSession, Client, DailyStat, DataResponse, Overview,
    TrashResponse, TrendResponse, VersionInfo, WeeklyStat, WidgetToggleResponse,
};
use crate::fmt;
use crate::tray::{self, TrayAction};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Records,
    Charts,
    Settings,
    Admin,
}

impl Page {
    pub fn label(&self) -> &'static str {
        match self {
            Page::Dashboard => "仪表盘",
            Page::Records => "记录",
            Page::Charts => "图表",
            Page::Settings => "设置",
            Page::Admin => "管理",
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

    // UI 状态
    pub(crate) page: Page,
    /// 侧栏是否折叠（常态）——悬浮时用 `sidebar_rect` 做临时展开，不改这个字段
    pub(crate) sidebar_collapsed: bool,
    /// 上一帧侧栏占的屏幕矩形，用于判断指针是否悬浮在侧栏上
    pub(crate) sidebar_rect: Option<egui::Rect>,
    pub(crate) loading: bool,
    pub(crate) initialized: bool,
    /// 首帧居中是否已成功（显示器信息可能稍后才可用，需重试）
    pub(crate) centered: bool,
    pub(crate) toast: Option<(String, f64)>,
    pub(crate) pending_toast: Option<String>,
    pub(crate) applied_theme: Option<(bool, String)>,
    pub(crate) last_settings_poll: f64,
    pub(crate) tray_ready: bool,
    pub(crate) tray: Option<tray_icon::TrayIcon>,
    pub(crate) widget_check: Option<tray_icon::menu::CheckMenuItem>,

    // 记录页
    pub(crate) filter_text: String,
    pub(crate) filter_from: String,
    pub(crate) filter_to: String,
    pub(crate) records_view: RecordsView,
    pub(crate) selected: HashSet<String>,

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
            page: Page::Dashboard,
            // 与 web 端一致：侧栏常态折叠，靠悬浮临时展开
            sidebar_collapsed: true,
            sidebar_rect: None,
            loading: true,
            initialized: false,
            centered: false,
            toast: None,
            pending_toast: None,
            applied_theme: None,
            last_settings_poll: -100.0,
            tray_ready: false,
            tray: None,
            widget_check: None,
            filter_text: String::new(),
            filter_from: String::new(),
            filter_to: String::new(),
            records_view: RecordsView::Table,
            selected: HashSet::new(),
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
        self.go(
            move |c| c.put_json("/api/window-theme", &body),
            |r| AppMsg::Action(r.map(|_| ()), "切换主题".to_string()),
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
            self.notify("请至少选择两条记录");
        }
    }

    fn do_pending(&mut self) {
        let action = match self.pending.take() {
            Some(a) => a,
            None => return,
        };
        match action {
            PendingAction::DeleteSession(id) => {
                let body = serde_json::json!({ "session_id": id });
                self.go(
                    move |c| c.delete_json(&format!("/api/sessions/{id}"), &body),
                    |r| AppMsg::Action(r.map(|_| ()), "删除记录".into()),
                );
            }
            PendingAction::RestoreFromTrash(id) => {
                let body = serde_json::json!({ "id": id });
                self.go(
                    move |c| c.post_json("/api/trash/restore", &body),
                    |r| AppMsg::Action(r.map(|_| ()), "恢复记录".into()),
                );
            }
            PendingAction::DeleteFromTrash(id) => {
                let body = serde_json::json!({ "id": id });
                self.go(
                    move |c| c.post_json("/api/trash/delete", &body),
                    |r| AppMsg::Action(r.map(|_| ()), "永久删除".into()),
                );
            }
            PendingAction::ClearTrash => {
                self.go(
                    |c| c.post_json("/api/trash/clear", &serde_json::json!({})),
                    |r| AppMsg::Action(r.map(|_| ()), "清空回收站".into()),
                );
            }
            PendingAction::RestoreBackup(name) => {
                let body = serde_json::json!({ "filename": name });
                self.go(
                    move |c| c.post_json("/api/backup/restore", &body),
                    |r| AppMsg::Action(r.map(|_| ()), "恢复备份".into()),
                );
            }
            PendingAction::DeleteBackup(name) => {
                let body = serde_json::json!({ "filename": name });
                self.go(
                    move |c| c.delete_json("/api/delete-backup", &body),
                    |r| AppMsg::Action(r.map(|_| ()), "删除备份".into()),
                );
            }
            PendingAction::CleanBackups => {
                self.go(
                    |c| c.post_json("/api/clean-backups", &serde_json::json!({ "keep": 10 })),
                    |r| AppMsg::Action(r.map(|_| ()), "清理旧备份".into()),
                );
            }
            PendingAction::ClearData => {
                self.go(
                    |c| c.post_json("/api/clear", &serde_json::json!({})),
                    |r| AppMsg::Action(r.map(|_| ()), "清空数据".into()),
                );
            }
            PendingAction::MergeSessions => {
                let ids: Vec<String> = self.selected.iter().cloned().collect();
                let body = serde_json::json!({ "ids": ids });
                self.go(
                    move |c| c.post_json("/api/merge-sessions", &body),
                    |r| AppMsg::Action(r.map(|_| ()), "合并记录".into()),
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
                Ok(_) => self.notify("已导出 CSV"),
                Err(e) => self.notify(&format!("导出失败: {e}")),
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
                        Ok(_) => self.notify("已导出 XLSX"),
                        Err(e) => self.notify(&format!("导出失败: {e}")),
                    }
                }
            }
            Err(e) => self.notify(&format!("生成 XLSX 失败: {e}")),
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
                            |r| AppMsg::Action(r.map(|_| ()), "导入数据".into()),
                        );
                    }
                    Err(e) => self.notify(&format!("JSON 解析失败: {e}")),
                },
                Err(e) => self.notify(&format!("读取文件失败: {e}")),
            }
        }
    }

    // ---------- 工具 ----------

    pub fn notify(&mut self, msg: &str) {
        self.pending_toast = Some(msg.to_string());
    }

    fn handle(&mut self, m: AppMsg) {
        match m {
            AppMsg::DataLoaded(r) => {
                self.loading = false;
                match r {
                    Ok(d) => self.data = Some(d),
                    Err(e) => self.notify(&format!("数据加载失败: {e}")),
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
                Err(e) => self.notify(&format!("设置加载失败: {e}")),
            },
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
            AppMsg::Action(r, what) => match r {
                Ok(_) => {
                    self.notify(&format!("{what}成功"));
                    self.refresh_data();
                    if what.contains("备份") || what.contains("回收站") {
                        self.load_trash();
                        self.load_backups();
                    }
                }
                Err(e) => self.notify(&format!("{what}失败: {e}")),
            },
            AppMsg::WidgetToggled(r) => match r {
                Ok(on) => {
                    if let Some(c) = &self.widget_check {
                        c.set_checked(on);
                    }
                    if let Some(s) = &mut self.settings {
                        s.widget_enabled = on;
                    }
                    self.notify(&format!("桌面组件已{}", if on { "开启" } else { "关闭" }));
                }
                Err(e) => self.notify(&format!("组件切换失败: {e}")),
            },
        }
    }

    pub(crate) fn filtered_sessions(&self) -> Vec<BootSession> {
        let ft = self.filter_text.trim().to_lowercase();
        let sessions = self
            .data
            .as_ref()
            .map(|d| &d.sessions)
            .cloned()
            .unwrap_or_default();
        sessions
            .into_iter()
            .filter(|s| {
                if !ft.is_empty() && !s.id.to_lowercase().contains(&ft) {
                    return false;
                }
                if !self.filter_from.is_empty() && fmt::fmt_date(&s.boot_time) < self.filter_from {
                    return false;
                }
                if !self.filter_to.is_empty() && fmt::fmt_date(&s.boot_time) > self.filter_to {
                    return false;
                }
                true
            })
            .collect()
    }

    // ---------- 主循环 ----------

    pub fn app_update(&mut self, ctx: &Context) {
        // 托盘动作
        while let Ok(a) = self.tray_rx.try_recv() {
            match a {
                TrayAction::Show => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true)),
                TrayAction::Hide => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false)),
                TrayAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
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

        // 关闭窗口 → 隐藏到托盘（与原 Tauri 行为一致）
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        // 处理异步消息
        while let Ok(m) = self.rx.try_recv() {
            self.handle(m);
        }

        // 待显示 toast
        if let Some(m) = self.pending_toast.take() {
            self.toast = Some((m, ctx.input(|i| i.time) + 3.0));
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
        }

        // 周期拉取设置（同步主题/组件开关）
        if ctx.input(|i| i.time) - self.last_settings_poll > 15.0 {
            self.last_settings_poll = ctx.input(|i| i.time);
            // 用户停留在「设置-网络」页编辑 Token/自定义域名时跳过，
            // 否则后台整体替换 self.settings 会覆盖正在输入、尚未 lost_focus 提交的内容
            let editing_network = self.page == Page::Settings && self.settings_tab == 2;
            if !editing_network {
                self.spawn_settings();
            }
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
        }

        // 保持实时刷新（本次会话计时）
        ctx.request_repaint_after(Duration::from_secs(1));
        // 入场动画期间需要逐帧重绘（否则只有 1s 一次，动画会卡成幻灯片）
        if self.animating(ctx) {
            ctx.request_repaint();
        }

        self.render(ctx);
    }

    fn render(&mut self, ctx: &Context) {
        // 页面切换 → 重置入场起始时刻
        if self.anim_page != self.page {
            self.anim_page = self.page;
            self.anim_start = crate::anim::now(ctx);
        }

        // 侧边栏：常态折叠，鼠标悬浮时临时展开（对齐 web 的 hoverExpanded）。
        // 之前顶栏还有一个手动折叠按钮，与悬浮展开功能重复，已移除。
        // 用上一帧记下的侧栏矩形判断指针是否悬停（当前帧矩形要等 show 才知道）。
        let hover_expand = self
            .sidebar_rect
            .and_then(|r| ctx.input(|i| i.pointer.hover_pos()).map(|p| r.contains(p)))
            .unwrap_or(false);
        let expanded = !self.sidebar_collapsed || hover_expand;
        let width = if expanded { 200.0 } else { 64.0 };
        egui::SidePanel::left("sidebar")
            .min_width(width)
            .default_width(width)
            .resizable(false)
            .show(ctx, |ui| {
                self.sidebar_rect = Some(ui.max_rect());
                self.render_sidebar(ui, expanded);
            });

        // 顶栏 + 中央
        egui::TopBottomPanel::top("topbar").show(ctx, |ui| self.render_topbar(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            // 页面入场：淡入 + 从下方 20px 滑上来（对齐 web 的 riseIn）
            let t = self.enter_t(ctx);
            if t < 1.0 {
                ui.set_opacity(0.15 + 0.85 * t);
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

        // Toast
        if let Some((msg, until)) = &self.toast {
            if ctx.input(|i| i.time) > *until {
                self.toast = None;
            } else {
                egui::Window::new("toast")
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -30.0))
                    .title_bar(false)
                    .resizable(false)
                    .auto_sized()
                    .show(ctx, |ui| {
                        ui.label(egui::RichText::new(msg).color(Color32::WHITE));
                    });
            }
        }

        // 确认弹窗
        if self.pending.is_some() {
            let mut open = true;
            egui::Window::new("确认操作")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label(self.pending_label());
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("确认").clicked() {
                            self.do_pending();
                        }
                        if ui.button("取消").clicked() {
                            self.pending = None;
                        }
                    });
                });
            if !open {
                self.pending = None;
            }
        }
    }

    fn pending_label(&self) -> String {
        match &self.pending {
            Some(PendingAction::DeleteSession(_)) => "确认删除该记录？".into(),
            Some(PendingAction::RestoreFromTrash(_)) => "确认从回收站恢复？".into(),
            Some(PendingAction::DeleteFromTrash(_)) => "确认永久删除（不可恢复）？".into(),
            Some(PendingAction::ClearTrash) => "确认清空回收站？".into(),
            Some(PendingAction::RestoreBackup(_)) => "确认恢复该备份（将覆盖当前数据）？".into(),
            Some(PendingAction::DeleteBackup(_)) => "确认删除该备份文件？".into(),
            Some(PendingAction::CleanBackups) => "确认清理旧备份（保留最近 10 个）？".into(),
            Some(PendingAction::ClearData) => "确认清空全部数据？此操作不可恢复！".into(),
            Some(PendingAction::MergeSessions) => "确认合并选中的记录？".into(),
            None => String::new(),
        }
    }

    /// `expanded` 为「当前是否处于展开态」——折叠态悬浮时也会临时为 true，
    /// 因此不能直接用 `self.sidebar_collapsed` 判断要不要显示文字。
    fn render_sidebar(&mut self, ui: &mut egui::Ui, expanded: bool) {
        let p = self.palette();
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            if expanded {
                ui.heading("开机记录");
            } else {
                ui.label(egui::RichText::new(egui_phosphor::regular::GAUGE).size(22.0));
            }
        });
        ui.separator();
        ui.add_space(6.0);
        let pages = [
            Page::Dashboard,
            Page::Records,
            Page::Charts,
            Page::Settings,
            Page::Admin,
        ];
        for p_ in pages {
            let icon = p_.icon();
            let selected = self.page == p_;
            let color = if selected { p.accent } else { p.text_primary };
            let content = if expanded {
                egui::RichText::new(icon).size(22.0).color(color)
            } else {
                egui::RichText::new(format!("{}   {}", icon, p_.label()))
                    .size(16.0)
                    .color(color)
            };
            let resp = ui.selectable_value(&mut self.page, p_, content);
            if resp.clicked() {
                // 进入某些页时主动拉数据
                if p_ == Page::Charts {
                    self.load_stats();
                }
                if p_ == Page::Admin {
                    self.load_trash();
                    self.load_backups();
                    if !self.tunnel_checked {
                        self.load_tunnel();
                        self.tunnel_checked = true;
                    }
                }
            }
            ui.add_space(2.0);
        }
    }

    fn render_topbar(&mut self, ui: &mut egui::Ui) {
        let p = self.palette();
        ui.set_min_height(56.0);
        ui.horizontal(|ui| {
            // 顶栏不再放折叠按钮：侧栏改为「常态折叠 + 悬浮展开」，
            // 手动按钮与悬浮展开是重复入口（与 web 端一致，只保留悬浮）
            // 标题带上该页图标，与侧栏呼应
            let title = format!("{}   {}", self.page.icon(), self.page.label());
            ui.heading(egui::RichText::new(title).size(20.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let online = self.data.is_some();
                let status = if online {
                    format!("{}  已连接", egui_phosphor::regular::PLUGS_CONNECTED)
                } else {
                    format!("{}  连接中", egui_phosphor::regular::PLUG)
                };
                ui.label(egui::RichText::new(status).color(if online {
                    p.success
                } else {
                    p.text_muted
                }));
                if let Some(v) = &self.version {
                    ui.label(format!("v{}", v.version));
                }
                // 主题切换
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
                if ui
                    .button(egui::RichText::new(theme_icon).size(18.0))
                    .clicked()
                {
                    let new_mode = if dark { "light" } else { "dark" };
                    if let Some(s) = &mut self.settings {
                        s.app_mode = new_mode.to_string();
                    }
                    self.set_theme(new_mode);
                }
            });
        });
        // 底部分隔线
        let rect = ui.min_rect();
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0_f32, p.border),
        );
    }
}

// ---------- CSV / XLSX ----------

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
        ws.set_name("记录").map_err(|e| e.to_string())?;

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
