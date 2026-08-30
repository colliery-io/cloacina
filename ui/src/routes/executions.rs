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

use aurora_leptos::components::{Chip, Empty, Loading, PageHeader, StatusBadge};
use aurora_leptos::tokens::{status_color, token};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_query_map};

use cloacina_api_types::ListExecutionsQuery;

use crate::components::TagPill;
use crate::data::poll_resource;
use crate::util::{ago, format_duration};

const MONO: &str = "'IBM Plex Mono', monospace";
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

    let page = move |delta: i64| {
        let next = (offset.get_untracked() + delta * PAGE_SIZE).max(0);
        let v = if next == 0 {
            String::new()
        } else {
            next.to_string()
        };
        navigate.with_value(|n| set_param(n, &current_qs(), "offset", &v));
    };

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="14px">
            <PageHeader title="Executions" />
            <div
                style:font-family=MONO
                style:font-size="11px"
                style:color="var(--faint)"
                style:margin-top="-10px"
            >
                {move || format!(
                    "{} runs · {} running · {} failed",
                    total.get(),
                    count_of("running"),
                    count_of("failed")
                )}
            </div>

            // Filter bar
            <div
                style:display="flex"
                style:justify-content="space-between"
                style:align-items="center"
                style:border-bottom="1px solid var(--border-soft)"
                style:padding-bottom="12px"
            >
                <div style:display="flex" style:gap="8px">
                    {CHIPS
                        .iter()
                        .map(|(label, value)| {
                            let value = value.to_string();
                            let value_for_active = value.clone();
                            let active =
                                Signal::derive(move || status.get() == value_for_active);
                            view! {
                                <Chip
                                    label=*label
                                    active=active
                                    on_click=Callback::new(move |_| {
                                        navigate.with_value(|n| {
                                            set_param(n, &current_qs(), "status", &value)
                                        })
                                    })
                                />
                            }
                        })
                        .collect_view()}
                </div>
                <div style:width="240px">
                    <aurora_leptos::components::TextInput
                        placeholder="Filter by workflow or run id…"
                        value=filter_text
                    />
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
                    <div style:display="flex" style:flex-direction="column" style:gap="8px">
                        <For
                            each=move || items.get()
                            key=|e| (e.id.clone(), e.status.clone())
                            children=move |e| {
                                let id = e.id.clone();
                                let running = e.status.eq_ignore_ascii_case("running");
                                let manual = e.trigger_origin.as_deref() == Some("manual");
                                view! {
                                    <div
                                        style:background="var(--panel)"
                                        style:border="1px solid var(--border)"
                                        style:border-radius="10px"
                                        style:padding="11px 15px"
                                        style:cursor="pointer"
                                        style:display="flex"
                                        style:justify-content="space-between"
                                        style:align-items="center"
                                        on:click=move |_| {
                                            navigate.with_value(|n| n(
                                                &format!("/executions/{id}"),
                                                Default::default(),
                                            ))
                                        }
                                    >
                                        <div style:display="flex" style:gap="11px" style:align-items="center" style:min-width="0">
                                            <span
                                                class:cl-pulse=running
                                                style:width="8px"
                                                style:height="8px"
                                                style:border-radius="50%"
                                                style:flex="none"
                                                style:background=status_color(&e.status)
                                            ></span>
                                            <div style:min-width="0">
                                                <div
                                                    style:font-size="13.5px"
                                                    style:font-weight="600"
                                                    style:color="var(--fg)"
                                                    style:overflow="hidden"
                                                    style:text-overflow="ellipsis"
                                                    style:white-space="nowrap"
                                                >
                                                    {e.workflow_name.clone()}
                                                </div>
                                                <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                    {e.id.clone()}
                                                </div>
                                            </div>
                                        </div>
                                        <div style:display="flex" style:gap="18px" style:align-items="center" style:flex="none">
                                            <Show when=move || manual>
                                                <TagPill color=token::GOLD>"manual"</TagPill>
                                            </Show>
                                            <StatusBadge status=e.status.clone() />
                                            <div
                                                class="cl-tnum"
                                                style:font-family=MONO
                                                style:font-size="11.5px"
                                                style:color="var(--fg-2)"
                                                style:width="64px"
                                                style:text-align="right"
                                            >
                                                {format_duration(Some(e.started_at.as_str()), e.completed_at.as_deref())}
                                            </div>
                                            <div
                                                style:font-family=MONO
                                                style:font-size="10.5px"
                                                style:color="var(--fainter)"
                                                style:width="64px"
                                                style:text-align="right"
                                            >
                                                {ago(Some(e.started_at.as_str()))}
                                            </div>
                                        </div>
                                    </div>
                                }
                            }
                        />

                        <div style:display="flex" style:justify-content="flex-end" style:gap="10px" style:margin-top="4px">
                            <button
                                class="cl-btn cl-btn--default cl-btn--xs"
                                disabled=move || offset.get() == 0
                                on:click=move |_| page(-1)
                            >
                                "Previous"
                            </button>
                            <button
                                class="cl-btn cl-btn--default cl-btn--xs"
                                disabled=move || (items.get().len() as i64) < PAGE_SIZE
                                on:click=move |_| page(1)
                            >
                                "Next"
                            </button>
                        </div>
                    </div>
                </Show>
            </Show>
        </div>
    }
}
