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

use aurora_leptos::components::{
    Card, Dot, FeedList, FeedRow, PageHeader, RelativeTime, SectionLabel, StatTile, StatusBadge,
};
use aurora_leptos::data::parse_timestamp;
use aurora_leptos::tokens::{status_color, token};
use leptos::prelude::*;

use cloacina_api_types::{ExecutionSummary, GraphStatus, ListExecutionsQuery, WorkflowSummary};

use crate::auth::use_auth;
use crate::data::poll_resource;
use crate::ops::use_ops_metrics;
use crate::util::{format_duration, short_id};

fn is_running(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "running" | "paused")
}
fn is_done(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "completed" | "failed" | "cancelled" | "canceled"
    )
}

/// One component in the health strip: a status dot, the name, a detail line.
#[component]
fn HealthTile(
    #[prop(into)] name: String,
    ok: Signal<Option<bool>>,
    detail: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="app-health">
            <div class="app-row app-row--tight">
                {move || {
                    let color = match ok.get() {
                        None => token::MUTED,
                        Some(true) => token::OK,
                        Some(false) => token::BAD,
                    };
                    view! { <Dot color=color /> }
                }}
                <span class="app-health__name">{name}</span>
            </div>
            <div class="app-meta">{move || detail.get()}</div>
        </div>
    }
}

/// A section title with a link on the right ("3 in flight", "View all").
#[component]
fn SectionHeader(
    #[prop(into)] title: String,
    right: Signal<String>,
    #[prop(into)] to: String,
) -> impl IntoView {
    view! {
        <SectionLabel
            label=title
            divider=true
            action=Box::new(move || {
                view! { <a href=to class="app-link">{move || right.get()}</a> }.into_any()
            })
        />
    }
}

/// One in-flight execution (the ActiveRunCard essentials: status pulse,
/// workflow, id chip, status, started-ago).
#[component]
fn ActiveRunCard(e: ExecutionSummary) -> impl IntoView {
    view! {
        <Card href=format!("/executions/{}", e.id)>
            <div class="app-runcard">
                <span class="cl-pulse app-none"><Dot color=status_color(&e.status) size=9 /></span>
                <div class="app-grow">
                    <div class="app-runcard__name app-ellipsis">{e.workflow_name.clone()}</div>
                    <div class="app-meta">{short_id(&e.id)}</div>
                </div>
                <div class="app-col app-col--tight app-right">
                    <StatusBadge status=e.status.to_lowercase() />
                    <span class="app-meta app-meta--xs"><RelativeTime iso=e.started_at.clone() /></span>
                </div>
            </div>
        </Card>
    }
}

/// One loaded computation graph (GraphMiniCard essentials).
#[component]
fn GraphMiniCard(g: GraphStatus) -> impl IntoView {
    view! {
        <Card href=format!("/graphs/{}", urlencoding::encode(&g.name))>
            <div class="app-runcard">
                <span class="app-square" style:background=token::TEAL></span>
                <div class="app-grow">
                    <div class="app-runcard__name app-ellipsis">{g.name.clone()}</div>
                    <div class="app-meta">
                        {format!("{} accumulators · reactor {}", g.accumulators.len(), g.reactor.clone().unwrap_or_else(|| "—".into()))}
                    </div>
                </div>
            </div>
        </Card>
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

    let count = |n: Signal<usize>| Signal::derive(move || n.get().to_string());

    view! {
        <div class="app-page app-page--loose">
            <PageHeader
                title="Overview"
                meta=Box::new(move || view! { <span class="app-meta app-meta--sm">{tenant_line}</span> }.into_any())
                actions=Box::new(|| view! {
                    <a href="/executions" class="app-search">"⌕ Find a workflow, run, or task…"</a>
                }.into_any())
            />

            // Metrics
            <div class="app-grid-4">
                <StatTile label="Workflows" value=count(wf_count) sub="registered".to_string() />
                <StatTile label="Running" value=count(running_count) color=token::ICE sub="in flight".to_string() />
                <StatTile label="Completed" value=count(completed_count) color=token::OK sub="recent".to_string() />
                <StatTile label="Failed" value=count(failed_count) color=token::BAD sub="recent".to_string() />
            </div>

            // Health strip
            <div class="app-grid-6">
                <HealthTile name="Server" ok=Signal::derive(move || server.get().0) detail=Signal::derive(move || server.get().1) />
                <HealthTile name="Compiler" ok=Signal::derive(move || compiler.get().0) detail=Signal::derive(move || compiler.get().1) />
                <HealthTile name="Reconciler" ok=Signal::derive(move || reconciler.get().0) detail=Signal::derive(move || reconciler.get().1) />
                <HealthTile name="Scheduler" ok=Signal::derive(move || scheduler.get().0) detail=Signal::derive(move || scheduler.get().1) />
                <HealthTile name="Database" ok=Signal::derive(move || database.get().0) detail=Signal::derive(move || database.get().1) />
                <HealthTile name="Agents" ok=Signal::derive(move || agents.get().0) detail=Signal::derive(move || agents.get().1) />
            </div>

            // Two columns
            <div class="app-split">
                <div class="app-col app-col--loose">
                    <div>
                        <SectionHeader
                            title="Active executions"
                            right=Signal::derive(move || format!("{} in flight", active.get().len()))
                            to="/executions"
                        />
                        <Show
                            when=move || !active.get().is_empty()
                            fallback=|| view! { <div class="app-empty">"No executions in flight."</div> }
                        >
                            <div class="app-col">
                                <For each=move || active.get() key=|e| e.id.clone() children=|e| view! { <ActiveRunCard e=e /> } />
                            </div>
                        </Show>
                    </div>

                    <div>
                        <SectionHeader
                            title="Computation graphs"
                            right=Signal::derive(move || format!("{} active", graph_items.get().len()))
                            to="/graphs"
                        />
                        <Show
                            when=move || !graph_items.get().is_empty()
                            fallback=|| view! { <div class="app-empty">"No computation graphs loaded."</div> }
                        >
                            <div class="app-col">
                                <For each=move || graph_items.get() key=|g| g.name.clone() children=|g| view! { <GraphMiniCard g=g /> } />
                            </div>
                        </Show>
                    </div>
                </div>

                // Recently completed
                <div>
                    <SectionHeader title="Recently completed" right=Signal::derive(|| "View all".to_string()) to="/executions" />
                    <div class="app-panel app-panel--feed">
                        <Show
                            when=move || !completed.get().is_empty()
                            fallback=|| view! { <div class="app-hint app-pad">"No completed runs yet."</div> }
                        >
                            <FeedList label="Recently completed executions">
                                <For
                                    each={move || completed.get().into_iter().take(8).collect::<Vec<_>>()}
                                    key=|e| e.id.clone()
                                    children=|e| {
                                        let failed = e.status.eq_ignore_ascii_case("failed");
                                        let at = parse_timestamp(&e.started_at);
                                        view! {
                                            <FeedRow
                                                at=at.unwrap_or_default()
                                                dot=status_color(&e.status)
                                                subject=e.workflow_name.clone()
                                                actor=short_id(&e.id)
                                                href=format!("/executions/{}", e.id)
                                            >
                                                <span class="app-mono" class:app-bad=failed>
                                                    {format_duration(Some(e.started_at.as_str()), e.completed_at.as_deref())}
                                                </span>
                                            </FeedRow>
                                        }
                                    }
                                />
                            </FeedList>
                        </Show>
                    </div>
                </div>
            </div>
        </div>
    }
}
