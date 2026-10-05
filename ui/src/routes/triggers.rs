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

use aurora_leptos::components::{
    Button, Dot, Empty, IconBolt, IconPlay, Loading, PageHeader, SectionLabel, Table, TableRow,
};
use aurora_leptos::data::use_now;
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::TriggerScheduleSummary;

use crate::auth::{client_for, use_auth};
use crate::components::TriggerFireModal;
use crate::data::poll_resource;
use crate::util::ago;

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

fn fmt_secs(ms: f64) -> String {
    let s = (ms / 1000.0).floor().max(0.0) as i64;
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m {}s", s / 60, s % 60)
    } else {
        format!("{}h {}m", s / 3600, (s % 3600) / 60)
    }
}

/// Derived (last, next) for a polling trigger. The stored `last_poll_at` is
/// stale by up to a fetch cycle by the time it renders, so a raw age never
/// reads "0s" — it starts at the data latency and drifts. Instead, while the
/// scheduler is on cadence (age < 2 intervals) we project the sawtooth:
/// age = (now − last_poll) mod interval, so the clock resets at each poll
/// boundary and "next" counts down to it (UAT round 4, T-0938).
fn poll_times(t: &TriggerScheduleSummary) -> (String, String) {
    let last = t.last_poll_at.clone().or_else(|| t.last_run_at.clone());
    match (&last, t.poll_interval_ms) {
        (Some(l), Some(ms)) => {
            let last_ms = js_sys::Date::parse(l);
            let ivl = ms as f64;
            if last_ms.is_nan() || ivl <= 0.0 {
                return (l.clone(), "—".into());
            }
            let raw = js_sys::Date::now() - last_ms;
            if raw < 0.0 {
                // Clock skew: trust the stamp, show it plainly.
                (
                    format!("{} ago", fmt_secs(0.0)),
                    format!("in {}", fmt_secs(ivl)),
                )
            } else if raw <= 2.0 * ivl {
                let age = raw % ivl;
                (
                    format!("{} ago", fmt_secs(age)),
                    format!("in {}", fmt_secs(ivl - age)),
                )
            } else {
                // Scheduler hasn't stamped in >2 intervals — genuinely stale.
                (format!("{} ago", fmt_secs(raw)), "overdue".into())
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

/// A section title with a one-line hint on the right.
#[component]
fn Section(#[prop(into)] label: String, #[prop(into)] hint: String) -> impl IntoView {
    view! {
        <SectionLabel
            label=label
            action=Box::new(move || view! { <span class="app-meta app-meta--md app-faint">{hint}</span> }.into_any())
        />
    }
}

#[component]
fn StateCell(enabled: bool) -> impl IntoView {
    view! {
        <span class="app-row app-row--tight">
            <Dot color=if enabled { token::OK } else { token::FAINT } size=7 />
            <span class="app-small" class:app-fg2=enabled class:app-faint=!enabled>
                {if enabled { "enabled" } else { "disabled" }}
            </span>
        </span>
    }
}

#[component]
pub fn Triggers() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let now = use_now();

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
        let schedule_text = if poll_mode {
            t.poll_interval_ms
                .map(|ms| format!("every {}", fmt_poll_interval(ms)))
                .unwrap_or_else(|| t.trigger_name.clone().unwrap_or_else(|| "—".into()))
        } else {
            t.cron_expression.clone().unwrap_or_else(|| "—".into())
        };
        // Live cells: re-derive on the 1s clock so "Ns ago"/"due now" advance
        // between data refreshes (UAT round 2).
        let tt = StoredValue::new(t.clone());
        let times = move || {
            now.track();
            tt.with_value(|t| {
                if poll_mode {
                    poll_times(t)
                } else {
                    (
                        // Same relative form the poll rows use, so the two
                        // tables read consistently (UAT round 2).
                        t.last_run_at
                            .as_deref()
                            .map(|ts| ago(Some(ts)))
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "—".into()),
                        t.next_run_at.clone().unwrap_or_else(|| "—".into()),
                    )
                }
            })
        };
        let enabled = t.enabled;
        let wf_for_run = t.workflow_name.clone();
        let trig_for_fire = t.trigger_name.clone();
        view! {
            <TableRow on_click=Callback::new(move |_| {
                navigate.with_value(|n| n(
                    &format!("/triggers/{}", urlencoding::encode(&detail_name)),
                    Default::default(),
                ))
            })>
                <td>
                    <span class="app-name app-block app-ellipsis">{t.workflow_name.clone()}</span>
                    {t.trigger_name.clone().map(|n| view! { <span class="app-meta app-block app-ellipsis">{n}</span> })}
                </td>
                <td class="app-meta app-meta--md">{schedule_text}</td>
                <td><StateCell enabled=enabled /></td>
                <td class="app-meta app-meta--sm">{move || times().1}</td>
                <td class="app-meta app-meta--sm">{move || times().0}</td>
                // Fire column — headed, left-justified.
                <td>
                    <Show when={
                        let has = trig_for_fire.is_some();
                        move || auth.can_write() && has
                    }>
                        {
                            let trig = trig_for_fire.clone();
                            view! {
                                <Button
                                    variant="subtle"
                                    size="xs"
                                    aria_label="Fire trigger"
                                    title="Fire this trigger → all subscribed workflows"
                                    stop_propagation=true
                                    on_click=Callback::new(move |_| {
                                        fire_target.set(trig.clone());
                                        fire_open.set(true);
                                    })
                                >
                                    <span class="app-hue app-gold"><IconBolt size=16 /></span>
                                </Button>
                            }
                        }
                    </Show>
                </td>
                // Run column — headed, left-justified, larger icon.
                <td>
                    <Show when=move || auth.can_write()>
                        {
                            let wf = wf_for_run.clone();
                            view! {
                                <Button
                                    variant="subtle"
                                    size="xs"
                                    aria_label="Run workflow"
                                    title="Run the workflow now (bypasses the schedule)"
                                    disabled=running
                                    stop_propagation=true
                                    on_click=Callback::new(move |_| run_now(wf.clone()))
                                >
                                    <span class="app-hue app-ice"><IconPlay size=18 /></span>
                                </Button>
                            }
                        }
                    </Show>
                </td>
            </TableRow>
        }
    };

    // Fixed layout + shared widths so the cron and polling tables align
    // column-for-column (UAT round 2).
    let widths = || {
        ["22%", "16%", "12%", "22%", "18%", "52px", "52px"]
            .iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>()
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
                    <th>"Fire"</th>
                    <th>"Run"</th>
                </tr>
            </thead>
        }
    };

    view! {
        <div class="app-page">
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
                        <Section
                            label="Cron schedules"
                            hint="fire on a wall-clock expression; the scheduler owns the cadence"
                        />
                        <Show
                            when=move || !crons.get().is_empty()
                            fallback=|| view! { <Empty message="No cron schedules." /> }
                        >
                            <div class="app-panel app-panel--flush">
                                <Table fixed=true widths=widths() label="Cron schedules">
                                    {table_head()}
                                    <tbody>
                                        <For
                                            each=move || crons.get()
                                            key=|t| (t.id.clone(), t.enabled, t.next_run_at.clone())
                                            children=move |t| row(t, false)
                                        />
                                    </tbody>
                                </Table>
                            </div>
                        </Show>
                    </div>

                    // ---- Polling triggers ----
                    <div>
                        <Section
                            label="Polling triggers"
                            hint="evaluated every poll interval; fire when their condition holds"
                        />
                        <Show
                            when=move || !polls.get().is_empty()
                            fallback=|| view! { <Empty message="No polling triggers." /> }
                        >
                            <div class="app-panel app-panel--flush">
                                <Table fixed=true widths=widths() label="Polling triggers">
                                    {table_head()}
                                    <tbody>
                                        <For
                                            each=move || polls.get()
                                            key=|t| (t.id.clone(), t.enabled, t.last_poll_at.clone())
                                            children=move |t| row(t, true)
                                        />
                                    </tbody>
                                </Table>
                            </div>
                        </Show>
                    </div>
                </Show>
            </Show>

            <TriggerFireModal open=fire_open trigger=fire_target />
        </div>
    }
}
