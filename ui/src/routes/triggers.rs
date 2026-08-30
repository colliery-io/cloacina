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

//! Triggers/schedules list (Aurora Dark spec 07), parity port of
//! `Triggers.tsx`: workflow, type pill (cron/poll), schedule, state,
//! next/last run, and the fire / run-now actions behind the write gate.
//! (Cron expressions render raw rather than cronstrue-humanized — an
//! acceptable Wave-3 dent; revisit if the visual gate objects.)

use aurora_leptos::components::{Empty, Loading, PageHeader};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::auth::{client_for, use_auth};
use crate::components::{TagPill, TriggerFireModal};
use crate::data::poll_resource;

const MONO: &str = "'IBM Plex Mono', monospace";

fn fmt_poll_interval(ms: i64) -> String {
    if ms % 60_000 == 0 && ms >= 60_000 {
        format!("{}m", ms / 60_000)
    } else if ms % 1000 == 0 {
        format!("{}s", ms / 1000)
    } else {
        format!("{ms}ms")
    }
}

#[component]
pub fn Triggers() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());

    let list = poll_resource(|c| async move { c.list_triggers(Some(200), Some(0), None).await });
    let items = Signal::derive(move || {
        list.get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let loading = Signal::derive(move || list.get().is_none());

    let fire_open = RwSignal::new(false);
    let fire_target = RwSignal::new(Option::<String>::None);
    let running = RwSignal::new(false);

    let run_now = move |workflow: String| {
        let Some(conn) = auth.connection() else {
            return;
        };
        running.set(true);
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

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="14px">
            <PageHeader title="Triggers" />

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading schedules…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <Empty message="No schedules." /> }
                >
                    <table class="cl-table">
                        <thead>
                            <tr>
                                <th>"Workflow"</th>
                                <th>"Type"</th>
                                <th>"Schedule"</th>
                                <th>"State"</th>
                                <th>"Next run"</th>
                                <th>"Last run"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || items.get()
                                key=|t| (t.id.clone(), t.enabled, t.next_run_at.clone())
                                children=move |t| {
                                    let detail_name = t
                                        .trigger_name
                                        .clone()
                                        .unwrap_or_else(|| t.workflow_name.clone());
                                    let is_cron = t.cron_expression.is_some();
                                    let schedule_text = t
                                        .cron_expression
                                        .clone()
                                        .or_else(|| t.poll_interval_ms.map(|ms| format!("every {}", fmt_poll_interval(ms))))
                                        .or_else(|| t.trigger_name.clone())
                                        .unwrap_or_else(|| "—".into());
                                    let enabled = t.enabled;
                                    let wf_for_run = t.workflow_name.clone();
                                    let trig_for_fire = t.trigger_name.clone();
                                    view! {
                                        <tr
                                            style:cursor="pointer"
                                            on:click=move |_| {
                                                navigate.with_value(|n| n(
                                                    &format!("/triggers/{}", urlencoding::encode(&detail_name)),
                                                    Default::default(),
                                                ))
                                            }
                                        >
                                            <td>
                                                <span style:font-size="13px" style:font-weight="600" style:color="var(--fg)">
                                                    {t.workflow_name.clone()}
                                                </span>
                                            </td>
                                            <td>
                                                <TagPill color=if is_cron { token::VIOLET } else { token::TEAL }>
                                                    {if is_cron { "cron" } else { "poll" }}
                                                </TagPill>
                                            </td>
                                            <td>
                                                <span style:font-family=MONO style:font-size="11.5px" style:color="var(--fg-2)">
                                                    {schedule_text}
                                                </span>
                                            </td>
                                            <td>
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
                                            </td>
                                            <td>
                                                <span style:font-family=MONO style:font-size="11px" style:color="var(--faint)">
                                                    {t.next_run_at.clone().unwrap_or_else(|| "—".into())}
                                                </span>
                                            </td>
                                            <td>
                                                <span style:font-family=MONO style:font-size="11px" style:color="var(--faint)">
                                                    {t.last_run_at.clone().unwrap_or_else(|| "—".into())}
                                                </span>
                                            </td>
                                            <td>
                                                <span style:display="inline-flex" style:gap="4px" style:justify-content="flex-end">
                                                    <Show when={
                                                        let has_trigger = trig_for_fire.is_some();
                                                        move || auth.can_write() && has_trigger
                                                    }>
                                                        {
                                                            let trig = trig_for_fire.clone();
                                                            view! {
                                                                <button
                                                                    class="cl-btn cl-btn--subtle cl-btn--xs"
                                                                    title="Fire → all subscribers"
                                                                    on:click={
                                                                        let trig = trig.clone();
                                                                        move |ev: leptos::ev::MouseEvent| {
                                                                            ev.stop_propagation();
                                                                            fire_target.set(trig.clone());
                                                                            fire_open.set(true);
                                                                        }
                                                                    }
                                                                >
                                                                    "⚡"
                                                                </button>
                                                            }
                                                        }
                                                    </Show>
                                                    <Show when=move || auth.can_write()>
                                                        {
                                                            let wf = wf_for_run.clone();
                                                            view! {
                                                                <button
                                                                    class="cl-btn cl-btn--subtle cl-btn--xs"
                                                                    title="Run now"
                                                                    disabled=move || running.get()
                                                                    on:click={
                                                                        let wf = wf.clone();
                                                                        move |ev: leptos::ev::MouseEvent| {
                                                                            ev.stop_propagation();
                                                                            run_now(wf.clone());
                                                                        }
                                                                    }
                                                                >
                                                                    "▸"
                                                                </button>
                                                            }
                                                        }
                                                    </Show>
                                                </span>
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </Show>
            </Show>

            <TriggerFireModal open=fire_open trigger=fire_target />
        </div>
    }
}
