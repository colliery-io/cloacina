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

use aurora_leptos::components::{
    Button, Empty, IconPause, IconPlay, Loading, PageHeader, Pill, RelativeTime, Table, TableRow,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::{ExecutionSummary, ListExecutionsQuery};

use crate::auth::{client_for, use_auth};
use crate::components::{RunCircles, RunWorkflowModal};
use crate::data::poll_resource;

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
        <div class="app-page">
            <PageHeader
                title="Workflows"
                sub="Registered packages; run history right off each row."
                actions=Box::new(move || view! {
                    <Show when=move || auth.can_write()>
                        <Button href="/workflows/upload">"↑ Upload package"</Button>
                    </Show>
                }.into_any())
            />

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading workflows…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <Empty message="No workflows uploaded yet." /> }
                >
                    <div class="app-panel app-panel--flush">
                        <Table label="Workflows">
                            <thead>
                                <tr>
                                    <th>"Package"</th>
                                    <th>"Version"</th>
                                    <th>"Tasks"</th>
                                    <th>"Updated"</th>
                                    <th>"Recent runs"</th>
                                    <th class="app-w-action">"Pause"</th>
                                    <th class="app-w-action">"Run"</th>
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
                                        // Reactive: the executions request can land after
                                        // the workflows one, and the row is keyed without it.
                                        let wf_for_runs = w.workflow_name.clone();
                                        let runs = move || {
                                            let runs = runs_by_workflow
                                                .get()
                                                .get(&wf_for_runs)
                                                .cloned()
                                                .unwrap_or_default();
                                            view! { <RunCircles runs=runs /> }
                                        };
                                        view! {
                                            <TableRow on_click=Callback::new(move |_| {
                                                navigate.with_value(|n| n(
                                                    &format!("/workflows/{}", urlencoding::encode(&pkg_for_nav)),
                                                    Default::default(),
                                                ))
                                            })>
                                                <td>
                                                    <span class="app-row app-row--tight">
                                                        <span class="app-square app-square--lg app-square--ice"></span>
                                                        <span class="app-name">{w.package_name.clone()}</span>
                                                        <Show when=move || paused>
                                                            <Pill color=token::GOLD>"paused"</Pill>
                                                        </Show>
                                                    </span>
                                                    {w.description.clone().map(|d| view! {
                                                        <div class="app-desc app-ellipsis" title=d.clone()>{d.clone()}</div>
                                                    })}
                                                </td>
                                                <td>
                                                    <Pill color=token::VIOLET>{format!("v{}", w.version)}</Pill>
                                                </td>
                                                <td class="app-meta app-meta--md">{w.tasks.len()}</td>
                                                <td class="app-meta app-meta--sm">
                                                    <RelativeTime iso=w.created_at.clone() />
                                                </td>
                                                <td>{runs}</td>
                                                // Pause column — headed, left-justified.
                                                <td>
                                                    <Show when=move || auth.can_write()>
                                                        {
                                                            let pkg = pkg_for_pause.clone();
                                                            view! {
                                                                <Button
                                                                    variant="subtle"
                                                                    size="xs"
                                                                    aria_label=if paused { "Resume workflow" } else { "Pause workflow" }
                                                                    title=if paused {
                                                                        "Resume — allow new executions"
                                                                    } else {
                                                                        "Pause — refuse new executions"
                                                                    }
                                                                    disabled=pausing
                                                                    stop_propagation=true
                                                                    on_click=Callback::new(move |_| toggle_pause(pkg.clone(), !paused))
                                                                >
                                                                    {if paused {
                                                                        view! { <span class="app-hue app-ok"><IconPlay size=16 /></span> }.into_any()
                                                                    } else {
                                                                        view! { <span class="app-hue app-gold"><IconPause size=16 /></span> }.into_any()
                                                                    }}
                                                                </Button>
                                                            }
                                                        }
                                                    </Show>
                                                </td>
                                                // Run column — headed, left-justified, larger icon.
                                                <td>
                                                    <Show when=move || auth.can_write()>
                                                        {
                                                            let pkg = pkg_for_run.clone();
                                                            let wf = wf_for_run.clone();
                                                            view! {
                                                                <Button
                                                                    variant="subtle"
                                                                    size="xs"
                                                                    aria_label="Run workflow"
                                                                    title="Run this workflow now (opens the typed-input form)"
                                                                    stop_propagation=true
                                                                    on_click=Callback::new(move |_| {
                                                                        run_target.set(Some((pkg.clone(), wf.clone())));
                                                                        run_open.set(true);
                                                                    })
                                                                >
                                                                    <span class="app-hue app-ice"><IconPlay size=18 /></span>
                                                                </Button>
                                                            }
                                                        }
                                                    </Show>
                                                </td>
                                            </TableRow>
                                        }
                                    }
                                />
                            </tbody>
                        </Table>
                    </div>
                </Show>
            </Show>

            <RunWorkflowModal open=run_open target=run_target />
        </div>
    }
}
