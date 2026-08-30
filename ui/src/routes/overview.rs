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

//! Aurora Dark Overview (CLOACI-I-0129 spec 01/02), parity port of
//! `Overview.tsx`: metrics + health strip + active executions /
//! computation graphs / recently-completed.

use aurora_leptos::tokens::{status_color, token};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::{ExecutionSummary, GraphStatus, ListExecutionsQuery, WorkflowSummary};

use crate::auth::use_auth;
use crate::data::poll_resource;
use crate::ops::use_ops_metrics;
use crate::util::{ago, format_duration, short_id};

const MONO: &str = "'IBM Plex Mono', monospace";

fn is_running(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "running" | "paused")
}
fn is_done(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "completed" | "failed" | "cancelled" | "canceled"
    )
}

#[component]
fn MetricCard(
    #[prop(into)] label: String,
    value: Signal<usize>,
    #[prop(into)] color: String,
    #[prop(into)] sub: String,
) -> impl IntoView {
    view! {
        <div
            style:background="var(--panel)"
            style:border="1px solid var(--border)"
            style:border-radius="10px"
            style:padding="15px 16px"
        >
            <div
                style:font-family=MONO
                style:font-size="10.5px"
                style:letter-spacing=".07em"
                style:text-transform="uppercase"
                style:color="var(--muted)"
            >
                {label}
            </div>
            <div
                class="cl-tnum"
                style:font-size="30px"
                style:font-weight="600"
                style:line-height="1"
                style:color=color
                style:margin="6px 0 4px"
            >
                {move || value.get()}
            </div>
            <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                {sub}
            </div>
        </div>
    }
}

#[component]
fn HealthTile(
    #[prop(into)] name: String,
    ok: Signal<Option<bool>>,
    detail: Signal<String>,
) -> impl IntoView {
    view! {
        <div
            style:background="var(--panel-2)"
            style:border="1px solid var(--border-soft)"
            style:border-radius="9px"
            style:padding="10px 12px"
        >
            <div style:display="flex" style:align-items="center" style:gap="6px" style:margin-bottom="3px">
                <span
                    style:width="8px"
                    style:height="8px"
                    style:border-radius="50%"
                    style:flex="none"
                    style:background=move || match ok.get() {
                        None => token::MUTED.to_string(),
                        Some(true) => token::OK.to_string(),
                        Some(false) => token::BAD.to_string(),
                    }
                ></span>
                <span style:font-size="12px" style:font-weight="500" style:color="var(--fg)">{name}</span>
            </div>
            <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                {move || detail.get()}
            </div>
        </div>
    }
}

#[component]
fn SectionHeader(
    #[prop(into)] title: String,
    right: Signal<String>,
    #[prop(into)] to: String,
) -> impl IntoView {
    view! {
        <div
            style:display="flex"
            style:justify-content="space-between"
            style:align-items="center"
            style:border-bottom="1px solid var(--border-soft)"
            style:padding-bottom="8px"
            style:margin-bottom="10px"
        >
            <span style:font-size="13px" style:font-weight="600" style:color="var(--fg)">{title}</span>
            <a href=to style:font-size="12px" style:color="var(--ice)" style:text-decoration="none">
                {move || right.get()}
            </a>
        </div>
    }
}

#[component]
fn EmptyCard(#[prop(into)] message: String) -> impl IntoView {
    view! {
        <div
            style:border="1px dashed var(--border)"
            style:border-radius="10px"
            style:padding="18px 15px"
            style:color="var(--faint)"
            style:font-size="12.5px"
        >
            {message}
        </div>
    }
}

/// One in-flight execution (the ActiveRunCard essentials: status pulse,
/// workflow, id chip, started-ago).
#[component]
fn ActiveRunCard(e: ExecutionSummary) -> impl IntoView {
    let navigate = use_navigate();
    let id = e.id.clone();
    let color = status_color(&e.status);
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:gap="12px"
            style:background="var(--panel)"
            style:border="1px solid var(--border)"
            style:border-radius="10px"
            style:padding="12px 15px"
            style:cursor="pointer"
            on:click=move |_| navigate(&format!("/executions/{id}"), Default::default())
        >
            <span
                class="cl-pulse"
                style:width="9px"
                style:height="9px"
                style:border-radius="50%"
                style:background=color.clone()
                style:flex="none"
            ></span>
            <div style:flex="1" style:min-width="0">
                <div style:font-size="13.5px" style:color="var(--fg)">{e.workflow_name.clone()}</div>
                <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                    {short_id(&e.id)}
                </div>
            </div>
            <div style:text-align="right">
                <div style:font-family=MONO style:font-size="11.5px" style:color=color>
                    {e.status.to_lowercase()}
                </div>
                <div style:font-family=MONO style:font-size="10px" style:color="var(--fainter)">
                    {ago(Some(e.started_at.as_str()))}
                </div>
            </div>
        </div>
    }
}

/// One loaded computation graph (GraphMiniCard essentials).
#[component]
fn GraphMiniCard(g: GraphStatus) -> impl IntoView {
    let navigate = use_navigate();
    let name = g.name.clone();
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:gap="12px"
            style:background="var(--panel)"
            style:border="1px solid var(--border)"
            style:border-radius="10px"
            style:padding="12px 15px"
            style:cursor="pointer"
            on:click=move |_| navigate(&format!("/graphs/{name}"), Default::default())
        >
            <span
                style:width="9px"
                style:height="9px"
                style:border-radius="2px"
                style:background=token::TEAL
                style:flex="none"
            ></span>
            <div style:flex="1" style:min-width="0">
                <div style:font-size="13.5px" style:color="var(--fg)">{g.name.clone()}</div>
                <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                    {format!("{} accumulators · reactor {}", g.accumulators.len(), g.reactor.clone().unwrap_or_else(|| "—".into()))}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn Overview() -> impl IntoView {
    let auth = use_auth();
    let ops = use_ops_metrics();

    let workflows = poll_resource(|c| async move { c.list_workflows(None).await });
    let graphs = poll_resource(|c| async move { c.list_graphs().await });
    let recent = poll_resource(|c| async move {
        c.list_executions(
            &ListExecutionsQuery {
                status: None,
                workflow: None,
                limit: Some(200),
                offset: Some(0),
            },
            None,
        )
        .await
    });

    let wf_count = Signal::derive(move || {
        workflows
            .get()
            .and_then(|r| r.ok())
            .map(|r| {
                r.items
                    .iter()
                    .filter(|w: &&WorkflowSummary| !w.tasks.is_empty())
                    .count()
            })
            .unwrap_or(0)
    });
    let recent_items = Signal::derive(move || {
        recent
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let graph_items = Signal::derive(move || {
        graphs
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });

    let active = Signal::derive(move || {
        recent_items
            .get()
            .into_iter()
            .filter(|e| is_running(&e.status))
            .collect::<Vec<_>>()
    });
    let completed = Signal::derive(move || {
        recent_items
            .get()
            .into_iter()
            .filter(|e| is_done(&e.status))
            .collect::<Vec<_>>()
    });
    let running_count = Signal::derive(move || {
        active
            .get()
            .iter()
            .filter(|e| e.status.eq_ignore_ascii_case("running"))
            .count()
    });
    let completed_count = Signal::derive(move || {
        completed
            .get()
            .iter()
            .filter(|e| e.status.eq_ignore_ascii_case("completed"))
            .count()
    });
    let failed_count = Signal::derive(move || {
        completed
            .get()
            .iter()
            .filter(|e| e.status.eq_ignore_ascii_case("failed"))
            .count()
    });

    // Health strip off the warm ops snapshot (ops_metrics:global WS).
    let ops_field = move |f: fn(&serde_json::Value) -> (Option<bool>, String)| {
        Signal::derive(move || {
            ops.get()
                .as_ref()
                .map(f)
                .unwrap_or((None, "connecting…".into()))
        })
    };
    let server = ops_field(|o| {
        let alive = o["server"]["alive"].as_bool().unwrap_or(false);
        let ready = o["server"]["ready"].as_bool().unwrap_or(false);
        (
            Some(alive),
            if ready {
                "alive · ready".into()
            } else {
                "alive".into()
            },
        )
    });
    let compiler = ops_field(|o| {
        (
            Some(true),
            format!(
                "{} building · {} pending",
                o["compiler"]["building"].as_u64().unwrap_or(0),
                o["compiler"]["pending"].as_u64().unwrap_or(0)
            ),
        )
    });
    let reconciler = ops_field(|o| {
        (
            Some(o["reconciler"]["status"].as_str() == Some("ok")),
            format!(
                "{} built · {} failed",
                o["reconciler"]["built"].as_u64().unwrap_or(0),
                o["reconciler"]["failed"].as_u64().unwrap_or(0)
            ),
        )
    });
    let scheduler = ops_field(|o| {
        (
            Some(o["server"]["ready"].as_bool().unwrap_or(false)),
            "ok".into(),
        )
    });
    let database = ops_field(|o| {
        let ready = o["server"]["ready"].as_bool().unwrap_or(false);
        (
            Some(ready),
            if ready {
                "ok".into()
            } else {
                o["server"]["reason"]
                    .as_str()
                    .unwrap_or("not ready")
                    .to_string()
            },
        )
    });
    let agents = ops_field(|o| {
        let n = o["fleet"].as_array().map(|a| a.len()).unwrap_or(0);
        (Some(n > 0), format!("{n} online"))
    });

    let tenant_line = move || {
        format!(
            "tenant {} · {} runs tracked",
            auth.connection()
                .map(|c| c.tenant)
                .unwrap_or_else(|| "—".into()),
            recent_items.get().len()
        )
    };

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="18px">
            // Header
            <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                <div>
                    <h2 style:font-size="22px" style:font-weight="600" style:color="var(--fg-bright)" style:margin="0">
                        "Overview"
                    </h2>
                    <div style:font-family=MONO style:font-size="11px" style:color="var(--faint)" style:margin-top="2px">
                        {tenant_line}
                    </div>
                </div>
                <a
                    href="/executions"
                    style:width="300px"
                    style:background="var(--panel)"
                    style:border="1px solid var(--border)"
                    style:border-radius="9px"
                    style:padding="8px 12px"
                    style:color="var(--faint)"
                    style:font-size="12.5px"
                    style:text-decoration="none"
                >
                    "⌕ Find a workflow, run, or task…"
                </a>
            </div>

            // Metrics
            <div style:display="grid" style:grid-template-columns="repeat(4, 1fr)" style:gap="13px">
                <MetricCard label="Workflows" value=wf_count color="var(--fg)" sub="registered" />
                <MetricCard label="Running" value=Signal::derive(move || running_count.get()) color=token::ICE sub="in flight" />
                <MetricCard label="Completed" value=Signal::derive(move || completed_count.get()) color=token::OK sub="recent" />
                <MetricCard label="Failed" value=Signal::derive(move || failed_count.get()) color=token::BAD sub="recent" />
            </div>

            // Health strip
            <div style:display="grid" style:grid-template-columns="repeat(6, 1fr)" style:gap="9px">
                <HealthTile name="Server" ok=Signal::derive(move || server.get().0) detail=Signal::derive(move || server.get().1) />
                <HealthTile name="Compiler" ok=Signal::derive(move || compiler.get().0) detail=Signal::derive(move || compiler.get().1) />
                <HealthTile name="Reconciler" ok=Signal::derive(move || reconciler.get().0) detail=Signal::derive(move || reconciler.get().1) />
                <HealthTile name="Scheduler" ok=Signal::derive(move || scheduler.get().0) detail=Signal::derive(move || scheduler.get().1) />
                <HealthTile name="Database" ok=Signal::derive(move || database.get().0) detail=Signal::derive(move || database.get().1) />
                <HealthTile name="Agents" ok=Signal::derive(move || agents.get().0) detail=Signal::derive(move || agents.get().1) />
            </div>

            // Two columns
            <div style:display="grid" style:grid-template-columns="7fr 5fr" style:gap="18px">
                <div>
                    <SectionHeader
                        title="Active executions"
                        right=Signal::derive(move || format!("{} in flight", active.get().len()))
                        to="/executions"
                    />
                    <Show
                        when=move || !active.get().is_empty()
                        fallback=|| view! { <EmptyCard message="No executions in flight." /> }
                    >
                        <div style:display="flex" style:flex-direction="column" style:gap="10px">
                            <For each=move || active.get() key=|e| e.id.clone() children=|e| view! { <ActiveRunCard e=e /> } />
                        </div>
                    </Show>

                    <div style:margin-top="18px">
                        <SectionHeader
                            title="Computation graphs"
                            right=Signal::derive(move || format!("{} active", graph_items.get().len()))
                            to="/graphs"
                        />
                        <Show
                            when=move || !graph_items.get().is_empty()
                            fallback=|| view! { <EmptyCard message="No computation graphs loaded." /> }
                        >
                            <div style:display="flex" style:flex-direction="column" style:gap="10px">
                                <For each=move || graph_items.get() key=|g| g.name.clone() children=|g| view! { <GraphMiniCard g=g /> } />
                            </div>
                        </Show>
                    </div>
                </div>

                // Recently completed
                <div>
                    <SectionHeader title="Recently completed" right=Signal::derive(|| "View all".to_string()) to="/executions" />
                    <div
                        style:background="var(--panel)"
                        style:border="1px solid var(--border)"
                        style:border-radius="10px"
                        style:padding="13px 15px"
                    >
                        <Show
                            when=move || !completed.get().is_empty()
                            fallback=|| view! {
                                <div style:color="var(--faint)" style:font-size="12.5px" style:padding="8px 2px">
                                    "No completed runs yet."
                                </div>
                            }
                        >
                            <For
                                each={move || completed.get().into_iter().take(8).collect::<Vec<_>>()}
                                key=|e| e.id.clone()
                                children=|e| {
                                    let navigate = use_navigate();
                                    let id = e.id.clone();
                                    let failed = e.status.eq_ignore_ascii_case("failed");
                                    view! {
                                        <div
                                            style:display="flex"
                                            style:align-items="center"
                                            style:gap="10px"
                                            style:padding="9px 2px"
                                            style:border-top="1px solid var(--border-fainter)"
                                            style:cursor="pointer"
                                            on:click=move |_| navigate(&format!("/executions/{id}"), Default::default())
                                        >
                                            <span
                                                style:width="8px"
                                                style:height="8px"
                                                style:border-radius="50%"
                                                style:flex="none"
                                                style:background=status_color(&e.status)
                                            ></span>
                                            <div style:flex="1" style:min-width="0">
                                                <div
                                                    style:font-size="13px"
                                                    style:color="var(--fg)"
                                                    style:overflow="hidden"
                                                    style:text-overflow="ellipsis"
                                                    style:white-space="nowrap"
                                                >
                                                    {e.workflow_name.clone()}
                                                </div>
                                                <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                    {short_id(&e.id)}
                                                </div>
                                            </div>
                                            <div style:text-align="right">
                                                <div
                                                    style:font-family=MONO
                                                    style:font-size="11.5px"
                                                    style:color=if failed { token::BAD } else { "var(--fg-2)" }
                                                >
                                                    {format_duration(Some(e.started_at.as_str()), e.completed_at.as_deref())}
                                                </div>
                                                <div style:font-family=MONO style:font-size="10px" style:color="var(--fainter)">
                                                    {ago(Some(e.started_at.as_str()))}
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </Show>
                    </div>
                </div>
            </div>
        </div>
    }
}
