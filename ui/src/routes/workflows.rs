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

//! Workflows list (Aurora Dark spec 06; reshaped by UAT round 1,
//! CLOACI-T-0938): a headed table — package, version, tasks, updated,
//! recent-run dots — with the same left-justified, labeled action columns
//! as /triggers ("Pause", "Run") so every clickable is self-explanatory.

use aurora_leptos::components::{Empty, Loading, PageHeader};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::{ExecutionSummary, ListExecutionsQuery};

use crate::auth::{client_for, use_auth};
use crate::components::{PauseIcon, PlayIcon, RunCircles, RunWorkflowModal, TagPill};
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
    let pausing = RwSignal::new(false);

    let toggle_pause = move |name: String, to_paused: bool| {
        let Some(conn) = auth.connection() else {
            return;
        };
        pausing.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = if to_paused {
                    client.pause_workflow(&name, None).await
                } else {
                    client.resume_workflow(&name, None).await
                };
            }
            pausing.set(false);
        });
    };

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="14px">
            <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                <PageHeader
                    title="Workflows"
                    sub="Registered packages; run history right off each row."
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
                    <table class="cl-table">
                        <thead>
                            <tr>
                                <th>"Package"</th>
                                <th>"Version"</th>
                                <th>"Tasks"</th>
                                <th>"Updated"</th>
                                <th>"Recent runs"</th>
                                <th style:width="60px">"Pause"</th>
                                <th style:width="60px">"Run"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || items.get()
                                key=|w| (w.id.clone(), w.paused, w.version.clone())
                                children=move |w| {
                                    let pkg_for_nav = w.package_name.clone();
                                    let pkg_for_pause = w.package_name.clone();
                                    let pkg_for_run = w.package_name.clone();
                                    let wf_for_run = w.workflow_name.clone();
                                    let paused = w.paused;
                                    let runs = runs_by_workflow
                                        .get()
                                        .get(&w.workflow_name)
                                        .cloned()
                                        .unwrap_or_default();
                                    view! {
                                        <tr
                                            style:cursor="pointer"
                                            on:click=move |_| {
                                                navigate.with_value(|n| n(
                                                    &format!(
                                                        "/workflows/{}",
                                                        urlencoding::encode(&pkg_for_nav)
                                                    ),
                                                    Default::default(),
                                                ))
                                            }
                                        >
                                            <td>
                                                <span style:display="inline-flex" style:gap="9px" style:align-items="center">
                                                    <span
                                                        style:width="10px"
                                                        style:height="10px"
                                                        style:border-radius="2px"
                                                        style:background=token::ICE
                                                        style:flex="none"
                                                    ></span>
                                                    <span style:font-size="13.5px" style:font-weight="600" style:color="var(--fg)">
                                                        {w.package_name.clone()}
                                                    </span>
                                                    <Show when=move || paused>
                                                        <TagPill color=token::GOLD>"paused"</TagPill>
                                                    </Show>
                                                </span>
                                                {w.description.clone().map(|d| view! {
                                                    <div
                                                        style:font-size="11.5px"
                                                        style:color="var(--muted)"
                                                        style:margin-top="2px"
                                                        style:max-width="360px"
                                                        style:overflow="hidden"
                                                        style:text-overflow="ellipsis"
                                                        style:white-space="nowrap"
                                                    >
                                                        {d}
                                                    </div>
                                                })}
                                            </td>
                                            <td>
                                                <TagPill color=token::VIOLET>{format!("v{}", w.version)}</TagPill>
                                            </td>
                                            <td>
                                                <span style:font-family=MONO style:font-size="11.5px" style:color="var(--fg-2)">
                                                    {w.tasks.len()}
                                                </span>
                                            </td>
                                            <td>
                                                <span style:font-family=MONO style:font-size="11px" style:color="var(--faint)">
                                                    {ago(Some(w.created_at.as_str()))}
                                                </span>
                                            </td>
                                            <td><RunCircles runs=runs /></td>
                                            // Pause column — headed, left-justified.
                                            <td style:text-align="left">
                                                <Show when=move || auth.can_write()>
                                                    {
                                                        let pkg = pkg_for_pause.clone();
                                                        view! {
                                                            <button
                                                                class="cl-btn cl-btn--subtle cl-btn--xs"
                                                                style:color=if paused { token::OK } else { token::GOLD }
                                                                title=if paused {
                                                                    "Resume — allow new executions"
                                                                } else {
                                                                    "Pause — refuse new executions"
                                                                }
                                                                disabled=move || pausing.get()
                                                                on:click={
                                                                    let pkg = pkg.clone();
                                                                    move |ev: leptos::ev::MouseEvent| {
                                                                        ev.stop_propagation();
                                                                        toggle_pause(pkg.clone(), !paused);
                                                                    }
                                                                }
                                                            >
                                                                {if paused {
                                                                    view! { <PlayIcon size=16 /> }.into_any()
                                                                } else {
                                                                    view! { <PauseIcon size=16 /> }.into_any()
                                                                }}
                                                            </button>
                                                        }
                                                    }
                                                </Show>
                                            </td>
                                            // Run column — headed, left-justified, larger icon.
                                            <td style:text-align="left">
                                                <Show when=move || auth.can_write()>
                                                    {
                                                        let pkg = pkg_for_run.clone();
                                                        let wf = wf_for_run.clone();
                                                        view! {
                                                            <button
                                                                class="cl-btn cl-btn--subtle cl-btn--xs"
                                                                style:color=token::ICE
                                                                title="Run this workflow now (opens the typed-input form)"
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
                                                                <PlayIcon size=18 />
                                                            </button>
                                                        }
                                                    }
                                                </Show>
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </Show>
            </Show>

            <RunWorkflowModal open=run_open target=run_target />
        </div>
    }
}
