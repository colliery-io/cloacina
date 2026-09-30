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

//! Trigger detail (T-0654 / WS-6), parity port of `TriggerDetail.tsx`:
//! schedule fields + recent executions + Run now. Enable/disable stays
//! read-only (no server toggle endpoint — an I-0124 non-goal), and recent
//! executions stay unlinked (rows carry a schedule-execution id, not a
//! workflow-execution id — the SDK/server gap noted in the task).

use aurora_leptos::components::{
    Button, DetailList, Dot, Empty, KeyValue, Loading, PageHeader, Panel, Pill, Table,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::auth::{client_for, use_auth};
use crate::data::poll_resource;

#[component]
pub fn TriggerDetail() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let params = use_params_map();
    let name = Signal::derive(move || params.read().get("name").unwrap_or_default());

    let detail = poll_resource(move |c| {
        let name = name.get();
        async move { c.get_trigger(&name, None).await }
    });
    let data = Signal::derive(move || detail.get().and_then(|r| r.ok()));
    let loading = Signal::derive(move || detail.get().is_none());

    let running = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let run_now = move || {
        let Some(workflow) = data.get_untracked().map(|d| d.schedule.workflow_name) else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        running.set(true);
        error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .execute_workflow(&workflow, serde_json::json!({}))
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            running.set(false);
            match result {
                Ok(res) => navigate.with_value(|n| {
                    n(
                        &format!("/executions/{}", res.execution_id),
                        Default::default(),
                    )
                }),
                Err(e) => error.set(e),
            }
        });
    };

    view! {
        <div class="app-page">
            {move || view! {
                <PageHeader
                    title=name.get()
                    back_href="/triggers"
                    back_label="Triggers"
                    actions=Box::new(move || view! {
                        <Show when=move || auth.can_write() && data.get().is_some()>
                            <Button loading=running on_click=Callback::new(move |_| run_now())>
                                "▸ Run now"
                            </Button>
                        </Show>
                    }.into_any())
                />
            }}

            <Show when=move || !error.get().is_empty()>
                <span class="app-error" role="alert">{move || error.get()}</span>
            </Show>

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading schedule…" /> }
            >
                <Show
                    when=move || data.get().is_some()
                    fallback=|| view! { <Empty message="Trigger not found." /> }
                >
                    {move || data.get().map(|d| {
                        let is_cron = d.schedule.cron_expression.is_some();
                        let enabled = d.schedule.enabled;
                        view! {
                            <div class="app-panel app-col">
                                <div class="app-row">
                                    <Pill color=if is_cron { token::VIOLET } else { token::TEAL }>
                                        {if is_cron { "cron schedule" } else { "polling trigger" }}
                                    </Pill>
                                    <span class="app-row app-row--tight">
                                        <Dot color=if enabled { token::OK } else { token::FAINT } size=7 />
                                        <span class="app-small" class:app-fg2=enabled class:app-faint=!enabled>
                                            {if enabled { "enabled" } else { "disabled" }}
                                        </span>
                                    </span>
                                </div>
                                <DetailList mono=true>
                                    <KeyValue label="Fires workflow">{d.schedule.workflow_name.clone()}</KeyValue>
                                    {d.schedule.cron_expression.clone().map(|c| view! {
                                        <KeyValue label="Cron">{c}</KeyValue>
                                    })}
                                    {d.schedule.poll_interval_ms.map(|ms| view! {
                                        <KeyValue label="Polls">{format!("every {}ms", ms)}</KeyValue>
                                    })}
                                    {d.schedule.trigger_name.clone().map(|t| view! {
                                        <KeyValue label="Trigger">{t}</KeyValue>
                                    })}
                                </DetailList>
                            </div>

                            <Panel title="Recent executions">
                                {if d.recent_executions.is_empty() {
                                    view! { <span class="app-hint">"No recent executions."</span> }.into_any()
                                } else {
                                    view! {
                                        <Table mono=true label="Recent executions">
                                            <thead>
                                                <tr>
                                                    <th>"Scheduled"</th>
                                                    <th>"Started"</th>
                                                    <th>"Completed"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {d.recent_executions
                                                    .iter()
                                                    .map(|e| view! {
                                                        <tr>
                                                            <td>{e.scheduled_time.clone().unwrap_or_else(|| "—".into())}</td>
                                                            <td>{e.started_at.clone()}</td>
                                                            <td>{e.completed_at.clone().unwrap_or_else(|| "—".into())}</td>
                                                        </tr>
                                                    })
                                                    .collect_view()}
                                            </tbody>
                                        </Table>
                                    }
                                    .into_any()
                                }}
                            </Panel>
                        }
                    })}
                </Show>
            </Show>
        </div>
    }
}
