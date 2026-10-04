//! 通用分页表格（列定义式，支持行选择）

use leptos::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct Column<T> {
    pub title: &'static str,
    /// 附加单元格 class（num=右对齐等宽 / mono=等宽字体）
    pub class: &'static str,
    pub render: Arc<dyn Fn(&T, usize) -> AnyView + Send + Sync>,
}

impl<T: 'static> Column<T> {
    pub fn new<V: IntoView + 'static>(
        title: &'static str,
        class: &'static str,
        render: impl Fn(&T) -> V + Send + Sync + 'static,
    ) -> Self {
        Self {
            title,
            class,
            render: Arc::new(move |t: &T, _idx: usize| render(t).into_any()),
        }
    }

    /// 序号列（页内 1 开始）
    pub fn indexed() -> Self {
        Self {
            title: "序号",
            class: "idx",
            render: Arc::new(|_t: &T, idx: usize| {
                view! { <span class="tnum">{(idx + 1).to_string()}</span> }.into_any()
            }),
        }
    }
}

#[component]
pub fn DataTable<T: Clone + Send + Sync + 'static>(
    rows: Memo<Vec<T>>,
    columns: Vec<Column<T>>,
    row_key: Arc<dyn Fn(&T) -> String + Send + Sync>,
    loading: RwSignal<bool>,
    #[prop(default = false)] selectable: bool,
    #[prop(optional)] selected: Option<RwSignal<Vec<String>>>,
    /// 空状态渲染
    empty: Option<Arc<dyn Fn() -> AnyView + Send + Sync>>,
) -> impl IntoView {
    let page = RwSignal::new(1usize);
    let size = RwSignal::new(15usize);

    // 行集变化（筛选/刷新）时回到第 1 页
    Effect::new(move |_| {
        let _ = rows.get().len();
        page.set(1);
    });

    let columns_c = columns.clone();
    let rk = row_key.clone();
    let selected_c = selected.clone();
    let empty_c = empty.clone();

    let table_view = move || {
        let list = rows.get();
        if loading.get() {
            return view! { <div class="tbl-empty"><div class="spin large"></div></div> }.into_any();
        }
        if list.is_empty() {
            return empty_c
                .as_ref()
                .map(|e| e())
                .unwrap_or_else(|| {
                    view! { <div class="tbl-empty hint-text">"暂无数据"</div> }.into_any()
                });
        }
        let p = page.get();
        let s = size.get();
        let start = (p - 1).saturating_mul(s);
        let end = (start + s).min(list.len());
        let slice: Vec<T> = if start >= list.len() {
            list.clone()
        } else {
            list[start..end].to_vec()
        };

        view! {
            <table class="tbl">
                <thead>
                    <tr>
                        {selectable.then(|| view! { <th style="width:36px"></th> })}
                        {columns_c
                            .iter()
                            .map(|c| {
                                view! { <th class=c.class>{c.title}</th> }
                            })
                            .collect_view()}
                    </tr>
                </thead>
                <tbody>
                    {slice
                        .iter()
                        .enumerate()
                        .map(|(i, row)| {
                            let key = rk(row);
                            let cols_a = columns_c.clone();
                            let cols_b = columns_c.clone();
                            let row_a = row.clone();
                            let row_b = row.clone();
                            let idx = i;
                            view! {
                                <tr>
                                    {selectable.then(move || {
                                        let s2 = selected_c;
                                        let k1 = key.clone();
                                        let k2 = key.clone();
                                        let cols2 = cols_a.clone();
                                        let row2 = row_a.clone();
                                        let idx2 = idx;
                                        view! {
                                            <td>
                                                <input
                                                    type="checkbox"
                                                    class="ck"
                                                    prop:checked=move || {
                                                        s2.as_ref()
                                                            .map(|s| s.get().contains(&k1))
                                                            .unwrap_or(false)
                                                    }
                                                    on:change=move |ev| {
                                                        let Some(s) = s2 else { return };
                                                        let checked = event_target_checked(&ev);
                                                        s.update(|ids| {
                                                            if checked {
                                                                if !ids.contains(&k2) {
                                                                    ids.push(k2.clone());
                                                                }
                                                            } else {
                                                                ids.retain(|x| *x != k2);
                                                            }
                                                        });
                                                    }
                                                />
                                            </td>
                                            {cols2
                                                .iter()
                                                .map(|c| {
                                                    let cell = (c.render)(&row2, idx2);
                                                    view! { <td class=c.class>{cell}</td> }
                                                })
                                                .collect_view()}
                                        }
                                    })}
                                    {(!selectable).then(move || {
                                        cols_b.iter()
                                            .map(|c| {
                                                let cell = (c.render)(&row_b, idx);
                                                view! { <td class=c.class>{cell}</td> }
                                            })
                                            .collect_view()
                                    })}
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        }
        .into_any()
    };

    let pagination_view = move || {
        let total = rows.get().len();
        if total == 0 || loading.get() {
            return ().into_any();
        }
        let p = page;
        let s = size;
        view! {
            <div class="pagination">
                <span>{move || format!("共 {} 条", total)}</span>
                <select on:change=move |ev| {
                    if let Ok(n) = event_target_value(&ev).parse::<usize>() {
                        s.set(n.max(1));
                        p.set(1);
                    }
                }>
                    {["10", "15", "30", "50", "100"]
                        .iter()
                        .map(|opt| {
                            let cur = s.get_untracked().to_string();
                            view! {
                                <option value=*opt selected=*opt == cur>
                                    {*opt}
                                </option>
                            }
                        })
                        .collect_view()}
                </select>
                <button disabled=move || p.get() <= 1 on:click=move |_| {
                    p.update(|x| *x = (*x).saturating_sub(1));
                }>"‹"</button>
                <span>
                    {move || format!("{} / {}", p.get(), ((total + s.get() - 1) / s.get()).max(1))}
                </span>
                <button disabled=move || p.get() * s.get() >= total on:click=move |_| {
                    p.update(|x| *x += 1);
                }>"›"</button>
            </div>
        }
        .into_any()
    };

    view! {
        <div class="table-wrap">
            {table_view}
            {pagination_view}
        </div>
    }
}
