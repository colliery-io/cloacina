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

//! Triggers/schedules (Aurora Dark spec 07; reshaped by UAT round 1,
//! CLOACI-T-0938): cron schedules and polling triggers are SEPARATE
//! sections — same storage, different behavior. Poll rows derive last/next
//! run from `last_poll_at` + `poll_interval_ms` (never blank once the
//! scheduler has polled). Actions live in HEADED, left-justified columns
//! ("Fire", "Run") with real icons so the clickables are self-explanatory.

use aurora_leptos::components::{Empty, Loading, PageHeader};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::TriggerScheduleSummary;

use crate::auth::{client_for, use_auth};
use crate::components::{BoltIcon, PlayIcon, TriggerFireModal};
use crate::data::poll_resource;
use crate::util::ago;

const MONO: &str = "'IBM Plex Mono', monospace";

fn fmt_poll_interval(ms: i64) -> String {
    if ms % 3_600_000 == 0 && ms >= 3_600_000 {
        format!("{}h", ms / 3_600_000)
    } else if ms % 60_000 == 0 && ms >= 60_000 {
        format!("{}m", ms / 60_000)
    } else if ms % 1000 == 0 {
        format!("{}s", ms / 1000)
    } else {
        format!("{ms}ms")
    }
}

/// Derived (last, next) for a polling trigger: last = the scheduler's last
/// poll; next = last + interval. Falls back to the stored run stamps.
fn poll_times(t: &TriggerScheduleSummary) -> (String, String) {
    let last = t.last_poll_at.clone().or_else(|| t.last_run_at.clone());
    match (&last, t.poll_interval_ms) {
        (Some(l), Some(ms)) => {
            let last_ms = js_sys::Date::parse(l);
            if last_ms.is_nan() {
                (l.clone(), "—".into())
            } else {
                let next_ms = last_ms + ms as f64;
                let overdue = next_ms <= js_sys::Date::now();
                let next = if overdue {
                    "due now".to_string()
                } else {
                    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(next_ms));
                    d.to_iso_string().as_string().unwrap_or_else(|| "—".into())
                };
                (format!("{} ago", ago_short(l)), next)
            }
        }
        (Some(l), None) => (l.clone(), "—".into()),
        (None, Some(ms)) => (
            "not yet polled".into(),
            format!("within {}", fmt_poll_interval(ms)),
        ),
        (None, None) => ("—".into(), "—".into()),
    }
}

fn ago_short(ts: &str) -> String {
    let s = ago(Some(ts));
    s.trim_end_matches(" ago").to_string()
}

#[component]
fn SectionLabel(#[prop(into)] label: String, #[prop(into)] hint: String) -> impl IntoView {
    view! {
        <div style:margin="6px 0 8px">
            <span
                style:font-family=MONO
                style:font-size="11px"
                style:letter-spacing=".06em"
                style:text-transform="uppercase"
                style:color="var(--muted)"
            >
                {label}
            </span>
            <span style:font-size="11.5px" style:color="var(--faint)" style:margin-left="10px">
                {hint}
            </span>
        </div>
    }
}

#[component]
fn StateCell(enabled: bool) -> impl IntoView {
    view! {
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
    let crons = Signal::derive(move || {
        items
            .get()
            .into_iter()
            .filter(|t| t.cron_expression.is_some())
            .collect::<Vec<_>>()
    });
    let polls = Signal::derive(move || {
        items
            .get()
            .into_iter()
            .filter(|t| t.cron_expression.is_none())
            .collect::<Vec<_>>()
    });

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

    // One shared row renderer; `poll_mode` switches the schedule/next/last
    // cells to the poll-derived values.
    let row = move |t: TriggerScheduleSummary, poll_mode: bool| {
        let detail_name = t
            .trigger_name
            .clone()
            .unwrap_or_else(|| t.workflow_name.clone());
        let (schedule_text, last_text, next_text) = if poll_mode {
            let (last, next) = poll_times(&t);
            (
                t.poll_interval_ms
                    .map(|ms| format!("every {}", fmt_poll_interval(ms)))
                    .unwrap_or_else(|| t.trigger_name.clone().unwrap_or_else(|| "—".into())),
                last,
                next,
            )
        } else {
            (
                t.cron_expression.clone().unwrap_or_else(|| "—".into()),
                t.last_run_at.clone().unwrap_or_else(|| "—".into()),
                t.next_run_at.clone().unwrap_or_else(|| "—".into()),
            )
        };
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
                    {t.trigger_name.clone().map(|n| view! {
                        <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                            {n}
                        </div>
                    })}
                </td>
                <td>
                    <span style:font-family=MONO style:font-size="11.5px" style:color="var(--fg-2)">
                        {schedule_text}
                    </span>
                </td>
                <td><StateCell enabled=enabled /></td>
                <td>
                    <span style:font-family=MONO style:font-size="11px" style:color="var(--faint)">
                        {next_text}
                    </span>
                </td>
                <td>
                    <span style:font-family=MONO style:font-size="11px" style:color="var(--faint)">
                        {last_text}
                    </span>
                </td>
                // Fire column — headed, left-justified.
                <td style:text-align="left">
                    <Show when={
                        let has = trig_for_fire.is_some();
                        move || auth.can_write() && has
                    }>
                        {
                            let trig = trig_for_fire.clone();
                            view! {
                                <button
                                    class="cl-btn cl-btn--subtle cl-btn--xs"
                                    style:color=token::GOLD
                                    title="Fire this trigger → all subscribed workflows"
                                    on:click={
                                        let trig = trig.clone();
                                        move |ev: leptos::ev::MouseEvent| {
                                            ev.stop_propagation();
                                            fire_target.set(trig.clone());
                                            fire_open.set(true);
                                        }
                                    }
                                >
                                    <BoltIcon size=16 />
                                </button>
                            }
                        }
                    </Show>
                </td>
                // Run column — headed, left-justified, larger icon.
                <td style:text-align="left">
                    <Show when=move || auth.can_write()>
                        {
                            let wf = wf_for_run.clone();
                            view! {
                                <button
                                    class="cl-btn cl-btn--subtle cl-btn--xs"
                                    style:color=token::ICE
                                    title="Run the workflow now (bypasses the schedule)"
                                    disabled=move || running.get()
                                    on:click={
                                        let wf = wf.clone();
                                        move |ev: leptos::ev::MouseEvent| {
                                            ev.stop_propagation();
                                            run_now(wf.clone());
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
    };

    let table_head = || {
        view! {
            <thead>
                <tr>
                    <th>"Workflow"</th>
                    <th>"Schedule"</th>
                    <th>"State"</th>
                    <th>"Next run"</th>
                    <th>"Last run"</th>
                    <th style:width="52px">"Fire"</th>
                    <th style:width="52px">"Run"</th>
                </tr>
            </thead>
        }
    };

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="16px">
            <PageHeader title="Triggers" />

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading schedules…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <Empty message="No schedules." /> }
                >
                    // ---- Cron schedules ----
                    <div>
                        <SectionLabel
                            label="Cron schedules"
                            hint="fire on a wall-clock expression; the scheduler owns the cadence"
                        />
                        <Show
                            when=move || !crons.get().is_empty()
                            fallback=|| view! { <Empty message="No cron schedules." /> }
                        >
                            <table class="cl-table">
                                {table_head()}
                                <tbody>
                                    <For
                                        each=move || crons.get()
                                        key=|t| (t.id.clone(), t.enabled, t.next_run_at.clone())
                                        children=move |t| row(t, false)
                                    />
                                </tbody>
                            </table>
                        </Show>
                    </div>

                    // ---- Polling triggers ----
                    <div>
                        <SectionLabel
                            label="Polling triggers"
                            hint="evaluated every poll interval; fire when their condition holds"
                        />
                        <Show
                            when=move || !polls.get().is_empty()
                            fallback=|| view! { <Empty message="No polling triggers." /> }
                        >
                            <table class="cl-table">
                                {table_head()}
                                <tbody>
                                    <For
                                        each=move || polls.get()
                                        key=|t| (t.id.clone(), t.enabled, t.last_poll_at.clone())
                                        children=move |t| row(t, true)
                                    />
                                </tbody>
                            </table>
                        </Show>
                    </div>
                </Show>
            </Show>

            <TriggerFireModal open=fire_open trigger=fire_target />
        </div>
    }
}
