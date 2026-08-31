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

//! Execution detail (T-0653 + T-0656), parity port of `ExecutionDetail.tsx`.
//!
//! OQ-6 merge, unchanged: the REST event log is the backfill; the live
//! delivery-WS tail (`follow_execution_events`, wasm transport) layers on
//! top, deduped on `sequence_num`. Status polls until terminal, then the
//! stream tears down and the REST log refetches for the authoritative final
//! history. Each live event also triggers a task-row refetch so the DAG and
//! table recolour immediately (CLOACI-T-0719).
//!
//! Wave-4 follow-ups (CLOACI-T-0935): the TaskGantt timeline and the
//! task-source modal (TaskCodeModal).

use aurora_leptos::components::{Loading, StatusBadge};
use aurora_leptos::graph::{Graph, GraphEdge, GraphNode};
use aurora_leptos::tokens::{pill_bg, status_color, token};
use futures_util::StreamExt;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use cloacina_api_types::ExecutionEvent;

use crate::auth::{client_for, use_auth};
use crate::components::TagPill;
use crate::data::{once_resource, poll_resource};
use crate::util::format_duration;

const MONO: &str = "'IBM Plex Mono', monospace";

fn is_terminal(status: &str) -> bool {
    matches!(
        status.to_lowercase().as_str(),
        "completed" | "failed" | "cancelled" | "canceled"
    )
}

fn local_id(name: &str) -> String {
    name.rsplit("::").next().unwrap_or(name).to_string()
}

#[component]
fn Field(#[prop(into)] label: String, value: Signal<String>) -> impl IntoView {
    view! {
        <div>
            <div
                style:font-family=MONO
                style:font-size="10px"
                style:letter-spacing=".07em"
                style:text-transform="uppercase"
                style:color="var(--faint)"
            >
                {label}
            </div>
            <div style:font-size="13.5px" style:color="var(--fg)" style:margin-top="3px">
                {move || value.get()}
            </div>
        </div>
    }
}

#[component]
fn SectionHeader(#[prop(into)] title: String, live: Signal<bool>) -> impl IntoView {
    view! {
        <div
            style:display="flex"
            style:gap="10px"
            style:align-items="center"
            style:border-bottom="1px solid var(--border-soft)"
            style:padding-bottom="8px"
            style:margin-bottom="10px"
        >
            <span style:font-size="14px" style:font-weight="600" style:color="var(--fg)">{title}</span>
            <Show when=move || live.get()>
                <span style:font-family=MONO style:font-size="10.5px" style:color=token::ICE>"live"</span>
            </Show>
        </div>
    }
}

/// Route wrapper: reads the execution id from the URL and renders the
/// reusable [`ExecutionView`].
#[component]
pub fn ExecutionDetail() -> impl IntoView {
    let params = use_params_map();
    let id = Signal::derive(move || params.read().get("id").unwrap_or_default());
    view! { <ExecutionView id=id /> }
}

/// The full live execution view (status, DAG, tasks, timeline, event log).
/// Reused by the /executions/:id route AND WorkflowDetail's
/// current-execution tab (UAT round 1, T-0938).
#[component]
pub fn ExecutionView(
    id: Signal<String>,
    /// True when embedded in another page (WorkflowDetail's current-execution
    /// tab): swaps the page header for a compact link to the full view.
    #[prop(default = false)]
    embedded: bool,
) -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());

    // Status: polled until terminal (parity with livePoll).
    let detail = poll_resource(move |c| {
        let id = id.get();
        async move { c.get_execution(&id, None).await }
    });
    let status = Signal::derive(move || {
        detail
            .get()
            .and_then(|r| r.ok())
            .map(|d| d.status)
            .unwrap_or_default()
    });
    let terminal = Signal::derive(move || status.get().is_empty() || is_terminal(&status.get()));

    // Task rows: polled while live (they also drive start/duration + DAG colors).
    let tasks = poll_resource(move |c| {
        let id = id.get();
        let conn_tenant = use_auth()
            .connection()
            .map(|c| c.tenant)
            .unwrap_or_else(|| "public".into());
        async move { c.get_execution_tasks(&conn_tenant, &id).await }
    });
    let task_list = Signal::derive(move || {
        tasks
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.tasks)
            .unwrap_or_default()
    });

    // REST event backfill.
    let events = poll_resource(move |c| {
        let id = id.get();
        async move { c.get_execution_events(&id, None).await }
    });

    // Live WS tail while in progress (wasm delivery stream).
    let live_events = RwSignal::new(Vec::<ExecutionEvent>::new());
    let generation = StoredValue::new(0u64);
    Effect::new(move |_| {
        let exec_id = id.get();
        let done = terminal.get();
        let my_gen = generation.with_value(|g| g + 1);
        generation.set_value(my_gen);
        live_events.set(Vec::new());
        if done || exec_id.is_empty() {
            return;
        }
        let Some(conn) = auth.connection() else {
            return;
        };
        leptos::task::spawn_local(async move {
            let Ok(client) = client_for(&conn) else {
                return;
            };
            let stream = client.follow_execution_events(&exec_id);
            let mut stream = std::pin::pin!(stream);
            while let Some(ev) = stream.next().await {
                if generation.with_value(|g| *g) != my_gen {
                    return;
                }
                let Ok(ev) = ev else { continue };
                if let Ok(parsed) = serde_json::from_value::<ExecutionEvent>(ev) {
                    live_events.update(|v| v.push(parsed));
                }
            }
        });
    });

    // Merge REST + live on sequence_num (OQ-6).
    let merged = Signal::derive(move || {
        let mut by_seq = std::collections::BTreeMap::<i64, ExecutionEvent>::new();
        for e in events
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.events)
            .unwrap_or_default()
        {
            by_seq.insert(e.sequence_num, e);
        }
        for e in live_events.get() {
            by_seq.insert(e.sequence_num, e);
        }
        by_seq.into_values().collect::<Vec<_>>()
    });

    // Workflow DAG: package name = 2nd namespace segment of any task id;
    // workflow name = 3rd (WS-12).
    let pkg_name = Signal::derive(move || {
        task_list
            .get()
            .first()
            .and_then(|t| t.task_name.split("::").nth(1).map(String::from))
            .unwrap_or_default()
    });
    let workflow_name = Signal::derive(move || {
        task_list
            .get()
            .first()
            .and_then(|t| t.task_name.split("::").nth(2).map(String::from))
            .unwrap_or_default()
    });
    let workflow = once_resource(move |c| {
        let pkg = pkg_name.get();
        async move {
            if pkg.is_empty() {
                Ok(None)
            } else {
                c.get_workflow(&pkg, None).await.map(Some)
            }
        }
    });
    let task_graph = Signal::derive(move || {
        workflow
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|w| w.task_graph)
            .unwrap_or_default()
    });

    // Start/end derived from task rows (detail endpoint exposes only status).
    let started_at = Signal::derive(move || {
        task_list
            .get()
            .iter()
            .map(|t| t.started_at.clone().unwrap_or_else(|| t.created_at.clone()))
            .min()
    });
    let ended_at = Signal::derive(move || {
        if !terminal.get() {
            return None;
        }
        task_list
            .get()
            .iter()
            .map(|t| {
                t.completed_at
                    .clone()
                    .unwrap_or_else(|| t.updated_at.clone())
            })
            .max()
    });

    let completed_of = Signal::derive(move || {
        let list = task_list.get();
        format!(
            "{}/{}",
            list.iter()
                .filter(|t| t.status.eq_ignore_ascii_case("completed"))
                .count(),
            if list.is_empty() {
                "—".to_string()
            } else {
                list.len().to_string()
            }
        )
    });

    let rerunning = RwSignal::new(false);
    let rerun = move |_| {
        let wf = workflow_name.get_untracked();
        if wf.is_empty() {
            return;
        }
        let Some(conn) = auth.connection() else {
            return;
        };
        rerunning.set(true);
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .execute_workflow(&wf, serde_json::json!({}))
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            rerunning.set(false);
            if let Ok(res) = result {
                navigate.with_value(|n| {
                    n(
                        &format!("/executions/{}", res.execution_id),
                        Default::default(),
                    )
                });
            }
        });
    };

    let live = Signal::derive(move || !terminal.get());

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="16px">
            // Header
            <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                <div>
                    <Show
                        when=move || !embedded
                        fallback=move || view! {
                            <a
                                href=move || format!("/executions/{}", id.get())
                                style:font-family=MONO
                                style:font-size="11.5px"
                                style:color="var(--muted)"
                                style:text-decoration="none"
                            >
                                {move || format!("{} — open full view →", id.get())}
                            </a>
                        }
                    >
                        <a href="/executions" style:font-size="11.5px" style:color="var(--muted)" style:text-decoration="none">
                            "← Executions"
                        </a>
                        <h1 style:font-size="22px" style:font-weight="600" style:color="var(--fg-bright)" style:margin="2px 0 0">
                            {move || {
                                let w = workflow_name.get();
                                if w.is_empty() { "Execution".to_string() } else { w }
                            }}
                        </h1>
                        <div style:font-family=MONO style:font-size="11px" style:color="var(--faint)" style:margin-top="2px">
                            {move || id.get()}
                        </div>
                    </Show>
                </div>
                <button
                    class="cl-btn cl-btn--default"
                    disabled=move || rerunning.get() || workflow_name.get().is_empty()
                    on:click=rerun
                >
                    "↻ Re-run"
                </button>
            </div>

            // Meta card
            <Show
                when=move || detail.get().is_some()
                fallback=|| view! { <Loading label="Loading execution…" /> }
            >
                <div
                    style:background="var(--panel)"
                    style:border="1px solid var(--border)"
                    style:border-radius="10px"
                    style:padding="15px 18px"
                    style:display="flex"
                    style:gap="32px"
                    style:align-items="center"
                >
                    <span data-testid="execution-status">
                        {move || view! { <StatusBadge status=status.get() /> }}
                    </span>
                    <Show when=move || live.get()>
                        <span
                            style:background=pill_bg(token::ICE)
                            style:color=token::ICE
                            style:border-radius="10px"
                            style:padding="2px 9px"
                            style:font-family=MONO
                            style:font-size="10.5px"
                            style:display="inline-flex"
                            style:align-items="center"
                            style:gap="5px"
                        >
                            <span
                                class="cl-pulse"
                                style:width="6px"
                                style:height="6px"
                                style:border-radius="50%"
                                style:background=token::ICE
                                style:display="inline-block"
                            ></span>
                            " live"
                        </span>
                    </Show>
                    <Field
                        label="Started"
                        value=Signal::derive(move || started_at.get().unwrap_or_else(|| "—".into()))
                    />
                    <Field
                        label="Duration"
                        value=Signal::derive(move || {
                            format_duration(started_at.get().as_deref(), ended_at.get().as_deref())
                        })
                    />
                    <Field label="Tasks" value=completed_of />
                </div>
            </Show>

            // Task graph (pack graph.rs; nodes colored by live task status)
            <Show when=move || !task_graph.get().is_empty()>
                <div>
                    <SectionHeader title="Task graph" live=live />
                    {move || {
                        let by_task: std::collections::HashMap<String, String> = task_list
                            .get()
                            .iter()
                            .map(|t| (local_id(&t.task_name), t.status.clone()))
                            .collect();
                        let nodes = task_graph
                            .get()
                            .iter()
                            .map(|n| {
                                let color = by_task
                                    .get(&n.id)
                                    .map(|s| status_color(s).to_string())
                                    .unwrap_or_else(|| token::MUTED.to_string());
                                GraphNode::new(n.id.clone(), n.id.clone()).color(color)
                            })
                            .collect::<Vec<_>>();
                        let edges = task_graph
                            .get()
                            .iter()
                            .flat_map(|n| {
                                let running = by_task
                                    .get(&n.id)
                                    .map(|s| s.eq_ignore_ascii_case("running"))
                                    .unwrap_or(false);
                                n.dependencies.iter().map(move |d| GraphEdge {
                                    from: d.clone(),
                                    to: n.id.clone(),
                                    active: running,
                                })
                            })
                            .collect::<Vec<_>>();
                        view! { <Graph nodes=nodes edges=edges direction="LR" /> }
                    }}
                </div>
            </Show>

            // Tasks table
            <div>
                <SectionHeader title="Tasks" live=live />
                <Show
                    when=move || tasks.get().is_some()
                    fallback=|| view! { <Loading label="Loading tasks…" /> }
                >
                    <table class="cl-table cl-table--mono">
                        <thead>
                            <tr>
                                <th>"Task"</th>
                                <th>"Status"</th>
                                <th>"Attempt"</th>
                                <th>"Duration"</th>
                                <th>"Error"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || task_list.get()
                                key=|t| (t.id.clone(), t.status.clone(), t.attempt)
                                children=|t| {
                                    let start = t.started_at.clone().unwrap_or_else(|| t.created_at.clone());
                                    let end = t.completed_at.clone();
                                    view! {
                                        <tr>
                                            <td>{local_id(&t.task_name)}</td>
                                            <td><StatusBadge status=t.status.clone() /></td>
                                            <td>{format!("{}/{}", t.attempt, t.max_attempts)}</td>
                                            <td class="cl-tnum">
                                                {format_duration(Some(start.as_str()), end.as_deref())}
                                            </td>
                                            <td style:color="var(--bad)" style:font-size="11.5px">
                                                {t.last_error.clone().unwrap_or_default()}
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </Show>
            </div>

            // Timeline (TaskGantt, T-0935)
            <div>
                <SectionHeader title="Timeline" live=live />
                <Show
                    when=move || tasks.get().is_some()
                    fallback=|| view! { <Loading label="Loading timeline…" /> }
                >
                    {move || view! { <crate::charts::TaskGantt tasks=task_list.get() /> }}
                </Show>
            </div>

            // Event log (REST backfill + WS live tail, merged on sequence_num)
            <div>
                <SectionHeader title="Event log" live=live />
                <Show
                    when=move || events.get().is_some()
                    fallback=|| view! { <Loading label="Loading events…" /> }
                >
                    <div
                        style:background="var(--inset)"
                        style:border="1px solid var(--border-soft)"
                        style:border-radius="10px"
                        style:padding="10px 13px"
                        style:max-height="380px"
                        style:overflow-y="auto"
                    >
                        <For
                            each=move || merged.get()
                            key=|e| e.sequence_num
                            children=|e| {
                                view! {
                                    <div
                                        style:display="flex"
                                        style:gap="10px"
                                        style:align-items="baseline"
                                        style:padding="3px 0"
                                        style:font-family=MONO
                                        style:font-size="11.5px"
                                    >
                                        <span style:color="var(--fainter)" style:flex="none">
                                            {e.created_at.clone()}
                                        </span>
                                        <TagPill color=status_color(&e.event_type).to_string()>
                                            {e.event_type.clone()}
                                        </TagPill>
                                        <span style:color="var(--fg-2)">
                                            {e.task_name.clone().unwrap_or_default()}
                                        </span>
                                    </div>
                                }
                            }
                        />
                    </div>
                </Show>
            </div>
        </div>
    }
}
