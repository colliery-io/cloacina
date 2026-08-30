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

//! Workflow detail — operational view (CLOACI-T-0764), parity port of
//! `WorkflowDetail.tsx`: header with build badge + execute/pause/delete,
//! build-error alert, task graph (pack graph.rs), named instances
//! (CLOACI-T-0927, read-only). The reliability overlays and chart panels
//! (StatusStrip, RunHeatmap, TaskHealthTable, CombinedTimeline, ScheduleCard,
//! InputsCard, TaskCodeModal) are Wave-4 work (CLOACI-T-0935).

use aurora_leptos::components::{Alert, Empty, Loading, Modal, Panel};
use aurora_leptos::graph::{Graph, GraphEdge, GraphNode};
use aurora_leptos::tokens::token;
use aurora_leptos::widgets::BuildStatusBadge;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::auth::{client_for, use_auth};
use crate::components::{RunWorkflowModal, TagPill};
use crate::data::poll_resource;
use crate::util::ago;

const MONO: &str = "'IBM Plex Mono', monospace";

#[component]
pub fn WorkflowDetail() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let params = use_params_map();
    let name = Signal::derive(move || params.read().get("name").unwrap_or_default());

    let detail = poll_resource(move |c| {
        let name = name.get();
        async move { c.get_workflow(&name, None).await }
    });
    let data = Signal::derive(move || detail.get().and_then(|r| r.ok()));
    let loading = Signal::derive(move || detail.get().is_none());
    let wf_name = Signal::derive(move || {
        data.get()
            .map(|d| {
                if d.workflow_name.is_empty() {
                    name.get()
                } else {
                    d.workflow_name
                }
            })
            .unwrap_or_else(|| name.get())
    });

    let instances = poll_resource(move |c| {
        let wf = wf_name.get();
        async move {
            if wf.is_empty() {
                return Ok(None);
            }
            c.list_instances(&wf, Some(100), Some(0), None)
                .await
                .map(Some)
        }
    });
    let instance_items = Signal::derive(move || {
        instances
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|r| r.items)
            .unwrap_or_default()
    });

    let exec_open = RwSignal::new(false);
    let exec_target = RwSignal::new(Option::<(String, String)>::None);
    let del_open = RwSignal::new(false);
    let del_error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let toggle_pause = move |_| {
        let Some(d) = data.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        let n = name.get_untracked();
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = if d.paused {
                    client.resume_workflow(&n, None).await
                } else {
                    client.pause_workflow(&n, None).await
                };
            }
            busy.set(false);
        });
    };

    let do_delete = move |_| {
        let Some(d) = data.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        let n = name.get_untracked();
        busy.set(true);
        del_error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .delete_workflow(&n, &d.version, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            match result {
                Ok(_) => navigate.with_value(|nav| nav("/workflows", Default::default())),
                Err(e) => del_error.set(e),
            }
        });
    };

    view! {
        <Show
            when=move || !loading.get()
            fallback=|| view! { <Loading label="Loading workflow…" /> }
        >
            <Show
                when=move || data.get().is_some()
                fallback=|| view! { <Empty message="Workflow not found." /> }
            >
                <div style:display="flex" style:flex-direction="column" style:gap="18px">
                    // Header
                    <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                        <div>
                            <a
                                href="/workflows"
                                style:font-family=MONO
                                style:font-size="11.5px"
                                style:color="var(--muted)"
                                style:text-decoration="none"
                            >
                                "← Workflows"
                            </a>
                            <div style:display="flex" style:gap="10px" style:align-items="center" style:margin-top="3px">
                                <h1
                                    style:font-size="23px"
                                    style:font-weight="600"
                                    style:color="var(--fg-bright)"
                                    style:letter-spacing="-.01em"
                                    style:margin="0"
                                >
                                    {move || name.get()}
                                </h1>
                                <Show when=move || data.get().map(|d| d.paused).unwrap_or(false)>
                                    <TagPill color=token::GOLD>"⏸ paused"</TagPill>
                                </Show>
                            </div>
                            <div style:display="flex" style:gap="8px" style:align-items="center" style:margin-top="5px">
                                {move || {
                                    data.get().map(|d| view! {
                                        <BuildStatusBadge status=d.build_status.clone() />
                                        <span style:font-family=MONO style:font-size="11.5px" style:color="var(--faint)">
                                            {format!(
                                                "v{} · created {} · workflow ",
                                                d.version,
                                                ago(Some(d.created_at.as_str()))
                                            )}
                                            <span style:color="var(--muted)">{d.workflow_name.clone()}</span>
                                        </span>
                                    })
                                }}
                            </div>
                        </div>
                        <div style:display="flex" style:gap="8px">
                            <Show when=move || auth.can_write()>
                                <button
                                    class="cl-btn cl-btn--filled"
                                    on:click=move |_| {
                                        exec_target.set(Some((name.get_untracked(), wf_name.get_untracked())));
                                        exec_open.set(true);
                                    }
                                >
                                    "▸ Execute"
                                </button>
                                <button
                                    class="cl-btn cl-btn--default"
                                    disabled=move || busy.get()
                                    on:click=toggle_pause
                                >
                                    {move || {
                                        if data.get().map(|d| d.paused).unwrap_or(false) {
                                            "▸ Resume"
                                        } else {
                                            "⏸ Pause"
                                        }
                                    }}
                                </button>
                            </Show>
                            <button
                                class="cl-btn cl-btn--subtle cl-btn--bad"
                                on:click=move |_| del_open.set(true)
                            >
                                "Delete"
                            </button>
                        </div>
                    </div>

                    // Build error
                    <Show when=move || data.get().and_then(|d| d.build_error).is_some()>
                        <Alert title="Build error" color="var(--bad)">
                            <span style:white-space="pre-wrap">
                                {move || data.get().and_then(|d| d.build_error).unwrap_or_default()}
                            </span>
                        </Alert>
                    </Show>

                    // Task graph
                    <Panel title="Task graph">
                        {move || {
                            let d = data.get();
                            let graph = d.as_ref().map(|d| d.task_graph.clone()).unwrap_or_default();
                            if !graph.is_empty() {
                                let nodes = graph
                                    .iter()
                                    .map(|n| GraphNode::new(n.id.clone(), n.id.clone()).color(token::ICE))
                                    .collect::<Vec<_>>();
                                let edges = graph
                                    .iter()
                                    .flat_map(|n| {
                                        n.dependencies.iter().map(move |dep| GraphEdge {
                                            from: dep.clone(),
                                            to: n.id.clone(),
                                            active: false,
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                view! { <Graph nodes=nodes edges=edges direction="LR" /> }.into_any()
                            } else {
                                let tasks = d.map(|d| d.tasks).unwrap_or_default();
                                if tasks.is_empty() {
                                    view! { <span style:color="var(--muted)" style:font-size="13px">"No tasks."</span> }
                                        .into_any()
                                } else {
                                    view! {
                                        <ul class="cl-list">
                                            {tasks
                                                .into_iter()
                                                .map(|t| view! { <li>{t}</li> })
                                                .collect_view()}
                                        </ul>
                                    }
                                    .into_any()
                                }
                            }
                        }}
                    </Panel>

                    // Named instances (T-0927, read-only)
                    <Panel title="Named instances" caption="persistent param bindings, optionally scheduled">
                        <Show
                            when=move || !instance_items.get().is_empty()
                            fallback=|| view! {
                                <Empty message="No named instances. Create one with `cloacinactl instance create`." />
                            }
                        >
                            <div style:display="flex" style:flex-direction="column" style:gap="6px">
                                <For
                                    each=move || instance_items.get()
                                    key=|i| i.id.clone()
                                    children=|i| {
                                        let cron_pill = i.cron_expression.clone();
                                        view! {
                                            <div style:display="flex" style:gap="12px" style:align-items="baseline">
                                                <span
                                                    style:font-family=MONO
                                                    style:font-size="13px"
                                                    style:color="var(--fg)"
                                                    style:min-width="160px"
                                                >
                                                    {i.instance_name.clone()}
                                                </span>
                                                <TagPill color=if cron_pill.is_some() { token::TEAL } else { token::MUTED }>
                                                    {cron_pill.unwrap_or_else(|| "unscheduled".into())}
                                                </TagPill>
                                                <Show when=move || i.paused>
                                                    <TagPill color=token::GOLD>"⏸ paused"</TagPill>
                                                </Show>
                                                <span
                                                    style:font-family=MONO
                                                    style:font-size="11px"
                                                    style:color="var(--muted)"
                                                    style:flex="1"
                                                    style:overflow="hidden"
                                                    style:text-overflow="ellipsis"
                                                    style:white-space="nowrap"
                                                >
                                                    {i.params
                                                        .as_ref()
                                                        .map(|p| p.to_string())
                                                        .unwrap_or_else(|| "—".into())}
                                                </span>
                                                <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                    {i.next_run_at
                                                        .as_ref()
                                                        .map(|t| format!("next {t}"))
                                                        .unwrap_or_default()}
                                                </span>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        </Show>
                    </Panel>

                    // Modals
                    <RunWorkflowModal open=exec_open target=exec_target />
                    <Show when=move || del_open.get()>
                        <Modal open=del_open title="Delete workflow?">
                            <div style:display="flex" style:flex-direction="column" style:gap="14px">
                                <span style:font-size="13px" style:color="var(--fg-2)">
                                    {move || format!(
                                        "Unregister {} v{}? This removes the package from the tenant.",
                                        name.get(),
                                        data.get().map(|d| d.version).unwrap_or_default()
                                    )}
                                </span>
                                <Show when=move || !del_error.get().is_empty()>
                                    <span style:color="var(--bad)" style:font-size="12.5px">
                                        {move || del_error.get()}
                                    </span>
                                </Show>
                                <div style:display="flex" style:justify-content="flex-end" style:gap="10px">
                                    <button class="cl-btn cl-btn--default" on:click=move |_| del_open.set(false)>
                                        "Cancel"
                                    </button>
                                    <button
                                        class="cl-btn cl-btn--filled cl-btn--bad"
                                        disabled=move || busy.get()
                                        on:click=do_delete
                                    >
                                        "Delete"
                                    </button>
                                </div>
                            </div>
                        </Modal>
                    </Show>
                </div>
            </Show>
        </Show>
    }
}
