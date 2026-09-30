/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Executions list (Aurora Dark spec 03), parity port of `Executions.tsx`:
//! URL-reflected status chips + a workflow filter; rows are dark cards with
//! status dot, run id, pill, duration, and started-ago. Page size 50.

use aurora_leptos::components::{
    Chip, Dot, Empty, Loading, PageHeader, Pagination, Pill, RelativeTime, StatusBadge, Table,
    TableRow, TextInput,
};
use aurora_leptos::tokens::{status_color, token};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

use cloacina_api_types::ListExecutionsQuery;

use crate::data::poll_resource;
use crate::util::format_duration;

const PAGE_SIZE: i64 = 50;

const CHIPS: [(&str, &str); 5] = [
    ("All", ""),
    ("Running", "Running"),
    ("Completed", "Completed"),
    ("Failed", "Failed"),
    ("Scheduled", "Scheduled"),
];

/// Rewrite the query string (replace navigation — parity with setParams
/// replace:true). Changing any non-offset key resets the offset.
fn set_param(
    navigate: &impl Fn(&str, leptos_router::NavigateOptions),
    current: &str,
    key: &str,
    value: &str,
) {
    let mut pairs: Vec<(String, String)> = current
        .trim_start_matches('?')
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| {
            kv.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect();
    pairs.retain(|(k, _)| k != key && (key == "offset" || k != "offset"));
    if !value.is_empty() {
        pairs.push((key.to_string(), urlencoding::encode(value).into_owned()));
    }
    let qs = pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    let path = if qs.is_empty() {
        "/executions".to_string()
    } else {
        format!("/executions?{qs}")
    };
    navigate(
        &path,
        leptos_router::NavigateOptions {
            replace: true,
            ..Default::default()
        },
    );
}

#[component]
pub fn Executions() -> impl IntoView {
    let navigate = StoredValue::new(use_navigate());
    let query = use_query_map();

    let status = Signal::derive(move || query.read().get("status").unwrap_or_default());
    let workflow = Signal::derive(move || query.read().get("workflow").unwrap_or_default());
    let offset = Signal::derive(move || {
        query
            .read()
            .get("offset")
            .and_then(|o| o.parse::<i64>().ok())
            .filter(|o| *o >= 0)
            .unwrap_or(0)
    });

    let list = poll_resource(move |c| {
        let q = ListExecutionsQuery {
            status: Some(status.get()).filter(|s| !s.is_empty()),
            workflow: Some(workflow.get()).filter(|w| !w.is_empty()),
            limit: Some(PAGE_SIZE),
            offset: Some(offset.get()),
        };
        async move { c.list_executions(&q, None).await }
    });

    let items = Signal::derive(move || {
        list.get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let total = Signal::derive(move || {
        list.get()
            .and_then(|r| r.ok())
            .map(|r| r.total)
            .unwrap_or_else(|| items.get().len())
    });
    let loading = Signal::derive(move || list.get().is_none());
    let count_of = move |s: &str| {
        let s = s.to_lowercase();
        items
            .get()
            .iter()
            .filter(|e| e.status.to_lowercase() == s)
            .count()
    };

    // The current query string for set_param rewrites.
    let current_qs = move || {
        let q = query.read();
        let mut parts = Vec::new();
        for key in ["status", "workflow", "offset"] {
            if let Some(v) = q.get(key) {
                if !v.is_empty() {
                    parts.push(format!("{key}={}", urlencoding::encode(&v)));
                }
            }
        }
        parts.join("&")
    };

    let filter_text = RwSignal::new(String::new());
    // Seed the filter box from the URL once.
    Effect::new(move |prev: Option<()>| {
        if prev.is_none() {
            filter_text.set(workflow.get_untracked());
        }
    });
    // Push filter edits into the URL (replace).
    Effect::new(move |prev: Option<String>| {
        let v = filter_text.get();
        if let Some(p) = prev {
            if p != v {
                navigate.with_value(|n| set_param(n, &current_qs(), "workflow", &v));
            }
        }
        v
    });

    // Aurora Pagination drives these; the URL stays the source of truth
    // (the offset is reflected into ?offset=, replace navigation).
    let page_offset = RwSignal::new(0usize);
    let page_limit = RwSignal::new(PAGE_SIZE as usize);
    Effect::new(move |_| page_offset.set(offset.get() as usize));
    let on_page = Callback::new(move |(next, _limit): (usize, usize)| {
        let v = if next == 0 { String::new() } else { next.to_string() };
        navigate.with_value(|n| set_param(n, &current_qs(), "offset", &v));
    });

    view! {
        <div class="app-page">
            <PageHeader
                title="Executions"
                meta=Box::new(move || view! {
                    <span class="app-meta app-meta--sm">
                        {move || format!(
                            "{} runs · {} running · {} failed",
                            total.get(),
                            count_of("running"),
                            count_of("failed")
                        )}
                    </span>
                }.into_any())
            />

            // Filter bar
            <div class="app-filterbar">
                <div class="app-row app-row--wrap">
                    {CHIPS
                        .iter()
                        .map(|(label, value)| {
                            let value = value.to_string();
                            let value_for_active = value.clone();
                            let active = Signal::derive(move || status.get() == value_for_active);
                            view! {
                                <Chip
                                    label=*label
                                    active=active
                                    on_click=Callback::new(move |_| {
                                        navigate.with_value(|n| set_param(n, &current_qs(), "status", &value))
                                    })
                                />
                            }
                        })
                        .collect_view()}
                </div>
                <div class="app-filterbar__search">
                    <TextInput placeholder="Filter by workflow or run id…" value=filter_text />
                </div>
            </div>

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading executions…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=move || {
                        let msg = if offset.get_untracked() > 0 {
                            "No more executions."
                        } else {
                            "No executions match."
                        };
                        view! { <Empty message=msg /> }
                    }
                >
                    <div class="app-panel app-panel--flush">
                        <Table label="Executions">
                            <thead>
                                <tr>
                                    <th>"Workflow · run"</th>
                                    <th class="app-w-origin"></th>
                                    <th class="app-w-status">"Status"</th>
                                    <th class="app-w-time app-right">"Duration"</th>
                                    <th class="app-w-time app-right">"Started"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || items.get()
                                    key=|e| (e.id.clone(), e.status.clone())
                                    children=move |e| {
                                        let id = e.id.clone();
                                        let running = e.status.eq_ignore_ascii_case("running");
                                        let manual = e.trigger_origin.as_deref() == Some("manual");
                                        view! {
                                            <TableRow on_click=Callback::new(move |_| {
                                                navigate.with_value(|n| n(&format!("/executions/{id}"), Default::default()))
                                            })>
                                                <td>
                                                    <span class="app-row">
                                                        <span class="app-none" class:cl-pulse=running>
                                                            <Dot color=status_color(&e.status) />
                                                        </span>
                                                        <span class="app-grow">
                                                            <span class="app-name app-ellipsis app-block">{e.workflow_name.clone()}</span>
                                                            <span class="app-meta app-block">{e.id.clone()}</span>
                                                        </span>
                                                    </span>
                                                </td>
                                                <td>
                                                    <Show when=move || manual>
                                                        <Pill color=token::GOLD>"manual"</Pill>
                                                    </Show>
                                                </td>
                                                <td><StatusBadge status=e.status.clone() /></td>
                                                <td class="app-num app-meta--md cl-tnum">
                                                    {format_duration(Some(e.started_at.as_str()), e.completed_at.as_deref())}
                                                </td>
                                                <td class="app-num app-meta">
                                                    <RelativeTime iso=e.started_at.clone() />
                                                </td>
                                            </TableRow>
                                        }
                                    }
                                />
                            </tbody>
                        </Table>
                    </div>
                    <Pagination offset=page_offset limit=page_limit total=total on_change=on_page />
                </Show>
            </Show>
        </div>
    }
}
