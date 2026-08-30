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

use aurora_leptos::components::{Empty, Loading};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::auth::{client_for, use_auth};
use crate::components::TagPill;
use crate::data::poll_resource;

const MONO: &str = "'IBM Plex Mono', monospace";

#[component]
fn Field(#[prop(into)] label: String, #[prop(into)] value: String) -> impl IntoView {
    view! {
        <div style:display="flex" style:gap="8px" style:align-items="baseline">
            <span
                style:font-family=MONO
                style:font-size="10.5px"
                style:letter-spacing=".04em"
                style:text-transform="uppercase"
                style:color="var(--faint)"
            >
                {label}
            </span>
            <span style:font-family=MONO style:font-size="12.5px" style:color="var(--fg-2)">
                {value}
            </span>
        </div>
    }
}

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
    let run_now = move |_| {
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
        <div style:display="flex" style:flex-direction="column" style:gap="16px">
            <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                <div>
                    <a
                        href="/triggers"
                        style:font-size="11.5px"
                        style:color="var(--muted)"
                        style:text-decoration="none"
                    >
                        "← Triggers"
                    </a>
                    <h1
                        style:font-size="22px"
                        style:font-weight="600"
                        style:color="var(--fg-bright)"
                        style:margin="2px 0 0"
                    >
                        {move || name.get()}
                    </h1>
                </div>
                <Show when=move || auth.can_write() && data.get().is_some()>
                    <button
                        class="cl-btn cl-btn--filled"
                        disabled=move || running.get()
                        on:click=run_now
                    >
                        "▸ Run now"
                    </button>
                </Show>
            </div>

            <Show when=move || !error.get().is_empty()>
                <span style:color="var(--bad)" style:font-size="12.5px">{move || error.get()}</span>
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
                            <div
                                style:background="var(--panel)"
                                style:border="1px solid var(--border)"
                                style:border-radius="10px"
                                style:padding="15px 18px"
                                style:display="flex"
                                style:flex-direction="column"
                                style:gap="10px"
                            >
                                <div style:display="flex" style:gap="10px" style:align-items="center">
                                    <TagPill color=if is_cron { token::VIOLET } else { token::TEAL }>
                                        {if is_cron { "cron schedule" } else { "polling trigger" }}
                                    </TagPill>
                                    <span style:display="inline-flex" style:gap="6px" style:align-items="center">
                                        <span
                                            style:width="7px"
                                            style:height="7px"
                                            style:border-radius="50%"
                                            style:background=if enabled { token::OK } else { token::FAINT }
                                        ></span>
                                        <span
                                            style:font-size="12px"
                                            style:color=if enabled { "var(--fg-2)" } else { "var(--faint)" }
                                        >
                                            {if enabled { "enabled" } else { "disabled" }}
                                        </span>
                                    </span>
                                </div>
                                <Field label="Fires workflow" value=d.schedule.workflow_name.clone() />
                                {d.schedule.cron_expression.clone().map(|c| view! {
                                    <Field label="Cron" value=c />
                                })}
                                {d.schedule.poll_interval_ms.map(|ms| view! {
                                    <Field label="Polls" value=format!("every {}ms", ms) />
                                })}
                                {d.schedule.trigger_name.clone().map(|t| view! {
                                    <Field label="Trigger" value=t />
                                })}
                            </div>

                            <div>
                                <div
                                    style:font-size="14px"
                                    style:font-weight="600"
                                    style:color="var(--fg)"
                                    style:border-bottom="1px solid var(--border-soft)"
                                    style:padding-bottom="8px"
                                    style:margin-bottom="10px"
                                >
                                    "Recent executions"
                                </div>
                                {if d.recent_executions.is_empty() {
                                    view! {
                                        <span style:color="var(--muted)" style:font-size="13px">
                                            "No recent executions."
                                        </span>
                                    }
                                    .into_any()
                                } else {
                                    view! {
                                        <table class="cl-table cl-table--mono">
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
                                        </table>
                                    }
                                    .into_any()
                                }}
                            </div>
                        }
                    })}
                </Show>
            </Show>
        </div>
    }
}
