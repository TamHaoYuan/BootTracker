//! 管理页：记录管理（编辑/软删/合并/清空）+ 回收站 + 添加记录 + 备份还原

use crate::api::{self, BootSession, OverviewStats, TrashData};
use crate::components::*;
use crate::fmt;
use crate::store;
use crate::table::{Column, DataTable};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;

#[component]
pub fn Admin() -> impl IntoView {
    let data = store::data_store().data;
    let loading = store::data_store().loading;

    let active_tab = RwSignal::new("manage".to_string());
    let selected = RwSignal::new(Vec::<String>::new());
    let trash = RwSignal::new(TrashData::default());
    let trash_loading = RwSignal::new(false);
    let backups = RwSignal::new(Vec::<String>::new());
    let backup_loading = RwSignal::new(false);
    let overview: RwSignal<Option<OverviewStats>> = RwSignal::new(None);

    // 编辑弹窗
    let editing: RwSignal<Option<BootSession>> = RwSignal::new(None);
    let edit_boot = RwSignal::new(String::new());
    let edit_shut = RwSignal::new(String::new());
    // 添加表单
    let add_boot = RwSignal::new(String::new());
    let add_shut = RwSignal::new(String::new());

    let load_trash = move || {
        spawn_local(async move {
            trash_loading.set(true);
            match api::TrashApi::get().await {
                Ok(t) => trash.set(t),
                Err(e) => crate::toast::toast_error(&format!("加载回收站失败：{e}")),
            }
            trash_loading.set(false);
        });
    };
    let load_backups = move || {
        spawn_local(async move {
            backup_loading.set(true);
            match api::BackupApi::list().await {
                Ok(r) => backups.set(r.backups),
                Err(e) => crate::toast::toast_error(&format!("加载备份列表失败：{e}")),
            }
            backup_loading.set(false);
        });
    };
    let load_overview = move || {
        spawn_local(async move {
            match api::StatsApi::overview().await {
                Ok(o) => overview.set(Some(o)),
                Err(e) => crate::toast::toast_error(&format!("加载统计失败：{e}")),
            }
        });
    };

    load_trash();
    load_backups();
    load_overview();

    let sorted = Memo::new(move |_| {
        let mut list = data.get().sessions;
        list.sort_by(|a, b| {
            let at = fmt::parse_iso_ms(&a.boot_time).unwrap_or(0.0);
            let bt = fmt::parse_iso_ms(&b.boot_time).unwrap_or(0.0);
            bt.partial_cmp(&at).unwrap_or(std::cmp::Ordering::Equal)
        });
        list
    });
    let trash_rows = Memo::new(move |_| trash.get().sessions);

    let base_columns: Vec<Column<BootSession>> = vec![
        Column::indexed(),
        Column::new("开机时间", "mono", |s: &BootSession| {
            view! { <span class="tnum mono">{fmt::fmt_full_time(Some(&s.boot_time))}</span> }
        }),
        Column::new("关机时间", "mono", |s: &BootSession| {
            view! {
                <span class="tnum mono">
                    {match &s.shutdown_time {
                        Some(t) => fmt::fmt_full_time(Some(t)),
                        None => "—".to_string(),
                    }}
                </span>
            }
        }),
        Column::new("会话时长", "num", |s: &BootSession| {
            if s.shutdown_time.is_some() {
                view! { <span class="tnum mono">{fmt::fmt_duration(fmt::session_duration(s, fmt::now_ms()))}</span> }.into_any()
            } else {
                let b = s.boot_time.clone();
                view! { <LiveDuration boot_time=b /> }.into_any()
            }
        }),
        Column::new("状态", "", |s: &BootSession| {
            if s.shutdown_time.is_some() {
                view! { <StatusPill tone=PillTone::Success>"已关机"</StatusPill> }.into_any()
            } else {
                view! { <StatusPill tone=PillTone::Accent>"进行中"</StatusPill> }.into_any()
            }
        }),
    ];

    // 管理列（编辑 / 删除）
    let manage_columns: Vec<Column<BootSession>> = {
        let mut cols = base_columns.clone();
        cols.push(Column::new("操作", "", move |s: &BootSession| {
            let id = s.id.clone();
            let id2 = id.clone();
            let boot = s.boot_time.clone();
            view! {
                <span style="display:inline-flex;gap:4px">
                    <button class="btn link small" on:click=move |_| {
                        let session = data.get_untracked()
                            .sessions
                            .into_iter()
                            .find(|x| x.id == id);
                        if let Some(s) = session {
                            edit_boot.set(fmt::iso_to_datetime_local(Some(&s.boot_time)));
                            edit_shut.set(fmt::iso_to_datetime_local(s.shutdown_time.as_deref()));
                            editing.set(Some(s));
                        }
                    }>"编辑"</button>
                    <button class="btn link small danger" on:click=move |_| {
                        let id = id2.clone();
                        let label = fmt::fmt_full_time(Some(&boot));
                        crate::toast::confirm(
                            crate::toast::ConfirmSpec::simple(
                                "移入回收站？",
                                &format!("开机于 {label} 的记录将移入回收站，可还原。"),
                                "移入回收站",
                                true,
                            )
                            .with_on_ok(move || {
                                let id = id.clone();
                                spawn_local(async move {
                                    match api::soft_delete_session(&id).await {
                                        Ok(()) => {
                                            crate::toast::toast_success("已移至回收站");
                                            selected.update(|ids| ids.retain(|x| *x != id));
                                            store::refresh_data().await;
                                            load_trash();
                                        }
                                        Err(e) => crate::toast::toast_error(&format!("删除失败：{e}")),
                                    }
                                });
                            }),
                        );
                    }>"删除"</button>
                </span>
            }.into_any()
        }));
        cols
    };
    let manage_columns = Arc::new(manage_columns);

    // 回收站列（还原 / 永久删除）
    let trash_columns: Vec<Column<BootSession>> = {
        let mut cols = base_columns.clone();
        cols.push(Column::new("操作", "", move |s: &BootSession| {
            let id = s.id.clone();
            let id2 = id.clone();
            let boot = s.boot_time.clone();
            view! {
                <span style="display:inline-flex;gap:4px">
                    <button class="btn link small" on:click=move |_| {
                        let id = id.clone();
                        spawn_local(async move {
                            match api::TrashApi::restore(&id).await {
                                Ok(()) => {
                                    crate::toast::toast_success("已还原");
                                    store::refresh_data().await;
                                    load_trash();
                                    load_overview();
                                }
                                Err(e) => crate::toast::toast_error(&format!("还原失败：{e}")),
                            }
                        });
                    }>"还原"</button>
                    <button class="btn link small danger" on:click=move |_| {
                        let id = id2.clone();
                        let label = fmt::fmt_full_time(Some(&boot));
                        crate::toast::confirm(
                            crate::toast::ConfirmSpec::simple(
                                "永久删除这条记录？",
                                &format!("开机于 {label}，删除后无法恢复。"),
                                "永久删除",
                                true,
                            )
                            .with_on_ok(move || {
                                let id = id.clone();
                                spawn_local(async move {
                                    match api::TrashApi::permanent_delete(&id).await {
                                        Ok(()) => {
                                            crate::toast::toast_success("已永久删除");
                                            load_trash();
                                        }
                                        Err(e) => crate::toast::toast_error(&format!("删除失败：{e}")),
                                    }
                                });
                            }),
                        );
                    }>"永久删除"</button>
                </span>
            }.into_any()
        }));
        cols
    };
    let trash_columns = Arc::new(trash_columns);

    // 合并选中
    let handle_merge = move |_| {
        let ids = selected.get_untracked();
        if ids.len() < 2 {
            crate::toast::toast_warning("请至少选择 2 条记录");
            return;
        }
        spawn_local(async move {
            match api::DataApi::merge_sessions(selected.get_untracked()).await {
                Ok(()) => {
                    crate::toast::toast_success("已合并选中记录");
                    selected.set(Vec::new());
                    store::refresh_data().await;
                    load_overview();
                    load_trash();
                }
                Err(e) => crate::toast::toast_error(&format!("合并失败：{e}")),
            }
        });
    };

    // 清空全部
    let handle_clear_all = move |_| {
        let list = data.get_untracked().sessions;
        let count = list.len();
        let range = if list.is_empty() {
            "当前没有记录".to_string()
        } else {
            let mut min = &list[0].boot_time;
            let mut max = &list[0].boot_time;
            for s in &list {
                if s.boot_time < *min {
                    min = &s.boot_time;
                }
                if s.boot_time > *max {
                    max = &s.boot_time;
                }
            }
            format!(
                "将删除 {} 条记录（{} ～ {}）",
                count,
                fmt::fmt_full_time(Some(min)),
                fmt::fmt_full_time(Some(max))
            )
        };
        crate::toast::confirm(
            crate::toast::ConfirmSpec {
                title: "永久删除全部记录？".into(),
                body: "所有开机记录将被永久删除，无法恢复（不进入回收站）。".into(),
                warn_box: range,
                ok_text: format!("永久删除 {count} 条记录"),
                danger: true,
                on_ok: Arc::new(std::sync::Mutex::new(Some(Box::new(move || {
                    spawn_local(async move {
                        match api::DataApi::clear_data().await {
                            Ok(()) => {
                                crate::toast::toast_success("已清空全部记录");
                                selected.set(Vec::new());
                                store::refresh_data().await;
                                load_overview();
                            }
                            Err(e) => crate::toast::toast_error(&format!("清空失败：{e}")),
                        }
                    });
                }) as Box<dyn FnOnce() + Send + Sync>))),
            },
        );
    };

    // 清空回收站
    let handle_clear_trash = move |_| {
        let count = trash.get_untracked().sessions.len();
        let earliest = trash
            .get_untracked()
            .sessions
            .iter()
            .map(|s| s.boot_time.clone())
            .min()
            .unwrap_or_default();
        let warn = if count == 0 {
            "回收站是空的".to_string()
        } else {
            format!("将删除 {count} 条记录（最早：{}）", fmt::fmt_full_time(Some(&earliest)))
        };
        crate::toast::confirm(
            crate::toast::ConfirmSpec {
                title: "永久删除回收站内容？".into(),
                body: "回收站中的所有记录将被永久删除，无法恢复。".into(),
                warn_box: warn,
                ok_text: format!("永久删除 {count} 条"),
                danger: true,
                on_ok: Arc::new(std::sync::Mutex::new(Some(Box::new(move || {
                    spawn_local(async move {
                        match api::TrashApi::clear().await {
                            Ok(()) => {
                                crate::toast::toast_success("回收站已清空");
                                load_trash();
                            }
                            Err(e) => crate::toast::toast_error(&format!("清空失败：{e}")),
                        }
                    });
                }) as Box<dyn FnOnce() + Send + Sync>))),
            },
        );
    };

    // 添加记录
    let handle_add = move |_| {
        let boot = fmt::datetime_local_to_iso(&add_boot.get_untracked());
        let Some(boot_iso) = boot else {
            crate::toast::toast_error("请选择开机时间");
            return;
        };
        let shut = fmt::datetime_local_to_iso(&add_shut.get_untracked());
        spawn_local(async move {
            match api::DataApi::add_session(boot_iso, shut).await {
                Ok(()) => {
                    crate::toast::toast_success("已添加记录");
                    add_boot.set(String::new());
                    add_shut.set(String::new());
                    store::refresh_data().await;
                    load_overview();
                }
                Err(e) => crate::toast::toast_error(&format!("添加失败：{e}")),
            }
        });
    };

    // 编辑保存
    let handle_edit_save = move |_| {
        let Some(session) = editing.get_untracked() else {
            return;
        };
        let boot_iso = fmt::datetime_local_to_iso(&edit_boot.get_untracked());
        let Some(boot) = boot_iso else {
            crate::toast::toast_error("请选择开机时间");
            return;
        };
        let shut = fmt::datetime_local_to_iso(&edit_shut.get_untracked());
        let id = session.id.clone();
        spawn_local(async move {
            match api::DataApi::update_session(
                &id,
                api::SessionPatch {
                    boot_time: Some(boot),
                    shutdown_time: Some(shut.unwrap_or_default()),
                },
            )
            .await
            {
                Ok(()) => {
                    crate::toast::toast_success("更新成功");
                    editing.set(None);
                    store::refresh_data().await;
                }
                Err(e) => crate::toast::toast_error(&format!("更新失败：{e}")),
            }
        });
    };

    view! {
        <div>
            <div class="toolbar">
                <h4 class="page-title">"管理"</h4>
                <div class="toolbar-group">
                    <button class="btn" disabled=loading.get() on:click=move |_| {
                        store::refresh_data();
                        load_trash();
                        load_backups();
                        load_overview();
                    }>"刷新全部"</button>
                    <button class="btn" on:click=move |_| {
                        let list = data.get_untracked().sessions;
                        if list.is_empty() {
                            crate::toast::toast_warning("没有可导出的记录");
                            return;
                        }
                        crate::pages::records::export_csv(&list);
                    }>"导出 CSV"</button>
                </div>
            </div>

            // 统计卡
            <div class="stat-grid">
                <div class="card stat-card c1">
                    <div class="s-title">"总开机次数"</div>
                    <div class="s-value">{move || overview.get().map(|o| o.total_boot).unwrap_or(0).to_string()}</div>
                </div>
                <div class="card stat-card c2">
                    <div class="s-title">"总关机次数"</div>
                    <div class="s-value">{move || overview.get().map(|o| o.total_shutdown).unwrap_or(0).to_string()}</div>
                </div>
                <div class="card stat-card c3">
                    <div class="s-title">"进行中会话"</div>
                    <div class="s-value">{move || overview.get().map(|o| o.active_count).unwrap_or(0).to_string()}</div>
                </div>
                <div class="card stat-card c4">
                    <div class="s-title">"平均时长"</div>
                    <div class="s-value">{move || fmt::fmt_duration(overview.get().map(|o| o.avg_duration).unwrap_or(0.0))}</div>
                </div>
            </div>

            <div class="card">
                <div class="tabs">
                    {[
                        ("manage", "记录管理"),
                        ("trash", &format!("回收站（{}）", trash.get_untracked().sessions.len())),
                        ("add", "添加记录"),
                        ("backup", "备份还原"),
                    ]
                        .iter()
                        .map(|(k, label)| {
                            let k = k.to_string();
                            let k1 = k.clone();
                            let k2 = k.clone();
                            let label = label.to_string();
                            view! {
                                <span
                                    class=move || if active_tab.get() == k1 { "tabs-item active" } else { "tabs-item" }
                                    on:click=move |_| active_tab.set(k2.clone())
                                >
                                    {label}
                                </span>
                            }
                        })
                        .collect_view()}
                </div>

                {move || {
                    match active_tab.get().as_str() {
                        "manage" => view! {
                            <div>
                                <div class="toolbar-group" style="margin-bottom:12px">
                                    <button class="btn primary" disabled=move || selected.get().len() < 2 on:click=handle_merge>
                                        {move || format!("合并选中（{}）", selected.get().len())}
                                    </button>
                                    <button class="btn danger" on:click=handle_clear_all>"清空全部"</button>
                                    <button class="btn" disabled=loading.get() on:click=move |_| {
                                        spawn_local(store::refresh_data());
                                    }>"刷新记录"</button>
                                </div>
                                <DataTable
                                    rows=sorted
                                    columns=(*manage_columns).clone()
                                    row_key=Arc::new(|s: &BootSession| s.id.clone())
                                    loading=loading
                                    selectable=true
                                    selected=selected
                                    empty=None
                                />
                            </div>
                        }.into_any(),
                        "trash" => view! {
                            <div>
                                <div class="toolbar-group" style="margin-bottom:12px">
                                    <button class="btn danger" on:click=handle_clear_trash>"清空回收站"</button>
                                    <button class="btn" disabled=trash_loading.get() on:click=move |_| load_trash()>"刷新"</button>
                                </div>
                                <DataTable
                                    rows=trash_rows
                                    columns=(*trash_columns).clone()
                                    row_key=Arc::new(|s: &BootSession| s.id.clone())
                                    loading=trash_loading
                                    empty=None
                                />
                            </div>
                        }.into_any(),
                        "add" => view! {
                            <div class="card" style="max-width:560px">
                                <div class="field">
                                    <span class="field-label">"开机时间"</span>
                                    <DateTimeInput value=add_boot />
                                </div>
                                <div class="field">
                                    <span class="field-label">"关机时间（可选，留空表示进行中）"</span>
                                    <DateTimeInput value=add_shut />
                                </div>
                                <div style="display:flex;gap:10px">
                                    <button class="btn primary" on:click=handle_add>"添加记录"</button>
                                    <button class="btn" on:click=move |_| {
                                        add_boot.set(String::new());
                                        add_shut.set(String::new());
                                    }>"重置"</button>
                                </div>
                            </div>
                        }.into_any(),
                        _ => {
                            view! {
                                <div>
                                    <div class="toolbar-group" style="margin-bottom:12px">
                                        <button class="btn" disabled=backup_loading.get() on:click=move |_| load_backups()>"刷新列表"</button>
                                        <button class="btn" on:click=move |_| {
                                            spawn_local(async move {
                                                match api::BackupApi::clean(10).await {
                                                    Ok(r) => {
                                                        crate::toast::toast_success(&format!("已清理 {} 份，剩余 {} 份", r.deleted, r.remaining));
                                                        load_backups();
                                                    }
                                                    Err(e) => crate::toast::toast_error(&format!("清理失败：{e}")),
                                                }
                                            });
                                        }>"清理旧备份（保留最近 10）"</button>
                                    </div>
                                    <div class="table-wrap">
                                        <table class="tbl">
                                            <thead>
                                                <tr>
                                                    <th>"备份文件名"</th>
                                                    <th style="width:200px">"操作"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {move || {
                                                    let list = backups.get();
                                                    if list.is_empty() {
                                                        return view! {
                                                            <tr><td colspan="2" class="tbl-empty hint-text">"暂无备份"</td></tr>
                                                        }.into_any();
                                                    }
                                                    let rows = list.into_iter()
                                                        .map(|name| {
                                                            let n1 = name.clone();
                                                            let n2 = name.clone();
                                                            view! {
                                                                <tr>
                                                                    <td class="mono">{name.clone()}</td>
                                                                    <td>
                                                                        <button class="btn link small" on:click=move |_| {
                                                                            let f = n1.clone();
                                                                            crate::toast::confirm(
                                                                                crate::toast::ConfirmSpec::simple(
                                                                                    &format!("恢复备份：{}？", f),
                                                                                    "当前数据将被备份内容覆盖，此操作不可撤销。",
                                                                                    "确认恢复",
                                                                                    false,
                                                                                )
                                                                                .with_on_ok(move || {
                                                                                    let f = f.clone();
                                                                                    spawn_local(async move {
                                                                                        match api::BackupApi::restore(&f).await {
                                                                                            Ok(()) => {
                                                                                                crate::toast::toast_success("备份已恢复");
                                                                                                store::refresh_data().await;
                                                                                                load_overview();
                                                                                                load_trash();
                                                                                            }
                                                                                            Err(e) => crate::toast::toast_error(&format!("恢复失败：{e}")),
                                                                                        }
                                                                                    });
                                                                                }),
                                                                            );
                                                                        }>"恢复"</button>
                                                                        <button class="btn link small danger" on:click=move |_| {
                                                                            let f = n2.clone();
                                                                            crate::toast::confirm(
                                                                                crate::toast::ConfirmSpec::simple(
                                                                                    &format!("删除备份：{}？", f),
                                                                                    "备份文件将被永久删除。",
                                                                                    "删除",
                                                                                    true,
                                                                                )
                                                                                .with_on_ok(move || {
                                                                                    let f = f.clone();
                                                                                    spawn_local(async move {
                                                                                        match api::BackupApi::delete(&f).await {
                                                                                            Ok(()) => {
                                                                                                crate::toast::toast_success("备份已删除");
                                                                                                load_backups();
                                                                                            }
                                                                                            Err(e) => crate::toast::toast_error(&format!("删除失败：{e}")),
                                                                                        }
                                                                                    });
                                                                                }),
                                                                            );
                                                                        }>"删除"</button>
                                                                    </td>
                                                                </tr>
                                                            }
                                                        })
                                                        .collect_view();
                                                    view! { <>{rows}</> }.into_any()
                                                }}
                                            </tbody>
                                        </table>
                                    </div>
                                </div>
                            }.into_any()
                        },
                    }
                }}
            </div>

            // 编辑弹窗
            <Show when=move || editing.get().is_some() fallback=|| ()>
                <div class="modal-mask" on:click=move |_| editing.set(None)>
                    <div class="modal" on:click=move |ev| ev.stop_propagation()>
                        <div class="modal-head">"编辑开机记录"</div>
                        <div class="modal-body">
                            <div class="field">
                                <span class="field-label">"开机时间"</span>
                                <DateTimeInput value=edit_boot />
                            </div>
                            <div class="field">
                                <span class="field-label">"关机时间（留空表示进行中）"</span>
                                <DateTimeInput value=edit_shut />
                            </div>
                        </div>
                        <div class="modal-foot">
                            <button class="btn" on:click=move |_| editing.set(None)>"取消"</button>
                            <button class="btn primary" on:click=handle_edit_save>"保存"</button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
