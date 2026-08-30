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

//! Workflows list (Aurora Dark spec 06), parity port of `Workflows.tsx`:
//! package cards with version badge, run-history dots, pause/resume, and a
//! Run action behind the write gate.

use aurora_leptos::components::{Empty, Loading, PageHeader};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::{ExecutionSummary, ListExecutionsQuery};

use crate::auth::{client_for, use_auth};
use crate::components::{RunCircles, RunWorkflowModal, TagPill};
use crate::data::poll_resource;
use crate::util::ago;

const MONO: &str = "'IBM Plex Mono', monospace";

#[component]
pub fn Workflows() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());

    let workflows = poll_resource(|c| async move { c.list_workflows(None).await });
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

    let items = Signal::derive(move || {
        workflows
            .get()
            .and_then(|r| r.ok())
            .map(|r| {
                r.items
                    .into_iter()
                    .filter(|w| !w.tasks.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let loading = Signal::derive(move || workflows.get().is_none());
    let runs_by_workflow = Signal::derive(move || {
        let mut m = std::collections::HashMap::<String, Vec<ExecutionSummary>>::new();
        for e in recent
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
        {
            m.entry(e.workflow_name.clone()).or_default().push(e);
        }
        m
    });

    let run_open = RwSignal::new(false);
    let run_target = RwSignal::new(Option::<(String, String)>::None);
    // Bump to force a refetch after a pause/resume mutation.
    let pausing = RwSignal::new(Option::<String>::None);

    let toggle_pause = move |name: String, to_paused: bool| {
        let Some(conn) = auth.connection() else {
            return;
        };
        pausing.set(Some(name.clone()));
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = if to_paused {
                    client.pause_workflow(&name, None).await
                } else {
                    client.resume_workflow(&name, None).await
                };
            }
            pausing.set(None);
            // Next poll tick refreshes the list; nothing else to do.
        });
    };

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="14px">
            <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                <PageHeader
                    title="Workflows"
                    sub=Signal::derive(move || format!("{} packages", items.get().len())).get_untracked()
                />
                <Show when=move || auth.can_write()>
                    <a href="/workflows/upload" class="cl-btn cl-btn--filled" style:text-decoration="none">
                        "↑ Upload package"
                    </a>
                </Show>
            </div>

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading workflows…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <Empty message="No workflows uploaded yet." /> }
                >
                    <div style:display="flex" style:flex-direction="column" style:gap="9px">
                        <For
                            each=move || items.get()
                            key=|w| (w.id.clone(), w.paused, w.version.clone())
                            children=move |w| {
                                let pkg = w.package_name.clone();
                                let pkg_for_nav = pkg.clone();
                                let pkg_for_pause = pkg.clone();
                                let pkg_for_run = pkg.clone();
                                let wf_for_run = w.workflow_name.clone();
                                let paused = w.paused;
                                let runs = runs_by_workflow
                                    .get()
                                    .get(&w.workflow_name)
                                    .cloned()
                                    .unwrap_or_default();
                                view! {
                                    <div
                                        style:background="var(--panel)"
                                        style:border="1px solid var(--border)"
                                        style:border-radius="10px"
                                        style:padding="13px 16px"
                                        style:cursor="pointer"
                                        style:display="flex"
                                        style:justify-content="space-between"
                                        style:align-items="center"
                                        style:gap="12px"
                                        on:click=move |_| {
                                            navigate.with_value(|n| n(
                                                &format!("/workflows/{}", urlencoding::encode(&pkg_for_nav)),
                                                Default::default(),
                                            ))
                                        }
                                    >
                                        <div style:display="flex" style:gap="11px" style:align-items="center" style:min-width="0">
                                            <span
                                                style:width="10px"
                                                style:height="10px"
                                                style:border-radius="2px"
                                                style:background=token::ICE
                                                style:flex="none"
                                            ></span>
                                            <div style:min-width="0">
                                                <div style:display="flex" style:gap="8px" style:align-items="center">
                                                    <span style:font-size="14px" style:font-weight="600" style:color="var(--fg)">
                                                        {w.package_name.clone()}
                                                    </span>
                                                    <TagPill color=token::VIOLET>{format!("v{}", w.version)}</TagPill>
                                                    <Show when=move || paused>
                                                        <TagPill color=token::GOLD>"paused"</TagPill>
                                                    </Show>
                                                    <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                        {format!(
                                                            "{} task{}",
                                                            w.tasks.len(),
                                                            if w.tasks.len() == 1 { "" } else { "s" }
                                                        )}
                                                    </span>
                                                </div>
                                                {w.description.clone().map(|d| view! {
                                                    <div
                                                        style:font-size="12px"
                                                        style:color="var(--muted)"
                                                        style:margin-top="3px"
                                                        style:overflow="hidden"
                                                        style:text-overflow="ellipsis"
                                                        style:white-space="nowrap"
                                                    >
                                                        {format!("{d} · updated {}", ago(Some(w.created_at.as_str())))}
                                                    </div>
                                                })}
                                            </div>
                                        </div>
                                        <div style:display="flex" style:gap="10px" style:align-items="center" style:flex="none">
                                            <RunCircles runs=runs />
                                            <Show when=move || auth.can_write()>
                                                {
                                                    let pkg = pkg_for_pause.clone();
                                                    view! {
                                                        <button
                                                            class="cl-btn cl-btn--subtle cl-btn--xs"
                                                            disabled=move || pausing.get().is_some()
                                                            on:click={
                                                                let pkg = pkg.clone();
                                                                move |ev: leptos::ev::MouseEvent| {
                                                                    ev.stop_propagation();
                                                                    toggle_pause(pkg.clone(), !paused);
                                                                }
                                                            }
                                                        >
                                                            {if paused { "Resume" } else { "Pause" }}
                                                        </button>
                                                    }
                                                }
                                            </Show>
                                            <Show when=move || auth.can_write()>
                                                {
                                                    let pkg = pkg_for_run.clone();
                                                    let wf = wf_for_run.clone();
                                                    view! {
                                                        <button
                                                            class="cl-btn cl-btn--default cl-btn--xs"
                                                            on:click={
                                                                let pkg = pkg.clone();
                                                                let wf = wf.clone();
                                                                move |ev: leptos::ev::MouseEvent| {
                                                                    ev.stop_propagation();
                                                                    run_target.set(Some((pkg.clone(), wf.clone())));
                                                                    run_open.set(true);
                                                                }
                                                            }
                                                        >
                                                            "▸ Run"
                                                        </button>
                                                    }
                                                }
                                            </Show>
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                </Show>
            </Show>

            <RunWorkflowModal open=run_open target=run_target />
        </div>
    }
}
