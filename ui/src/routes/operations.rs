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

//! Operations / deployment health (Aurora Dark spec 10/11), parity port of
//! `Operations.tsx`: server / compiler / reconciler / fleet metric cards
//! driven by the warm WS ops snapshot, plus the execution-agent roster.
//! (The add-agent enrollment modal was a MOCK in the React app; it stays out
//! until the enrollment API exists.)

use aurora_leptos::tokens::{pill_bg, token};
use leptos::prelude::*;

use crate::ops::use_ops_metrics;

const MONO: &str = "'IBM Plex Mono', monospace";

fn ago_secs(seconds: Option<i64>) -> String {
    match seconds {
        None => "—".into(),
        Some(s) if s < 60 => format!("{s}s ago"),
        Some(s) if s < 3600 => format!("{}m ago", s / 60),
        Some(s) => format!("{}h ago", s / 3600),
    }
}

fn fmt_time(ts: Option<&str>) -> String {
    match ts {
        None => "never".into(),
        Some(t) => t.to_string(),
    }
}

#[component]
fn MetricCard(
    #[prop(into)] title: String,
    #[prop(into)] state: String,
    #[prop(into)] color: String,
    rows: Vec<(String, String, Option<String>)>,
) -> impl IntoView {
    view! {
        <div
            style:background="var(--panel)"
            style:border="1px solid var(--border)"
            style:border-radius="11px"
            style:padding="15px 16px"
        >
            <div
                style:display="flex"
                style:justify-content="space-between"
                style:align-items="center"
                style:margin-bottom="12px"
            >
                <span style:font-size="14.5px" style:font-weight="600" style:color="var(--fg)">{title}</span>
                <span
                    style:background=pill_bg(&color)
                    style:color=color.clone()
                    style:border-radius="10px"
                    style:padding="2px 9px"
                    style:font-family=MONO
                    style:font-size="10.5px"
                >
                    {state}
                </span>
            </div>
            <div style:display="flex" style:flex-direction="column" style:gap="7px">
                {rows
                    .into_iter()
                    .map(|(label, value, vcolor)| view! {
                        <div style:display="flex" style:justify-content="space-between" style:gap="8px">
                            <span style:font-family=MONO style:font-size="11.5px" style:color="var(--faint)">
                                {label}
                            </span>
                            <span
                                style:font-family=MONO
                                style:font-size="11.5px"
                                style:color=vcolor.unwrap_or_else(|| "var(--fg-2)".into())
                            >
                                {value}
                            </span>
                        </div>
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn Operations() -> impl IntoView {
    let ops = use_ops_metrics();
    let live = Signal::derive(move || ops.get().is_some());

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="16px">
            // Header
            <div>
                <div style:display="flex" style:gap="10px" style:align-items="center">
                    <h1 style:font-size="22px" style:font-weight="600" style:color="var(--fg-bright)" style:margin="0">
                        "Operations"
                    </h1>
                    <span
                        style:background=move || pill_bg(if live.get() { token::OK } else { token::MUTED })
                        style:color=move || if live.get() { token::OK } else { token::MUTED }
                        style:border-radius="10px"
                        style:padding="2px 10px"
                        style:font-family=MONO
                        style:font-size="10.5px"
                    >
                        {move || if live.get() { "live" } else { "connecting…" }}
                    </span>
                </div>
                <div style:font-family=MONO style:font-size="11px" style:color="var(--faint)" style:margin-top="3px">
                    "Deployment health for the connected server, pushed over the control-plane socket."
                </div>
            </div>

            <Show
                when=move || live.get()
                fallback=|| view! {
                    <span style:color="var(--faint)" style:font-size="13px">
                        "Subscribing to operational metrics…"
                    </span>
                }
            >
                {move || {
                    let m = ops.get().unwrap_or_default();
                    let fleet = m["fleet"].as_array().cloned().unwrap_or_default();
                    let busy: i64 = fleet.iter().map(|f| f["in_flight"].as_i64().unwrap_or(0)).sum();
                    let capacity: i64 = fleet
                        .iter()
                        .map(|f| f["max_concurrency"].as_i64().unwrap_or(0))
                        .sum();
                    let alive = m["server"]["alive"].as_bool().unwrap_or(false);
                    let ready = m["server"]["ready"].as_bool().unwrap_or(false);
                    let compiler_status = m["compiler"]["status"].as_str().unwrap_or("idle").to_string();
                    let compiler_color = match compiler_status.as_str() {
                        "building" => token::ICE,
                        "backlogged" => token::GOLD,
                        _ => token::MUTED,
                    };
                    let failed = m["reconciler"]["failed"].as_i64().unwrap_or(0);
                    view! {
                        <div style:display="grid" style:grid-template-columns="repeat(4, 1fr)" style:gap="13px">
                            <MetricCard
                                title="Server"
                                state={if alive { "alive" } else { "down" }}
                                color={if alive { token::OK } else { token::BAD }}
                                rows=vec![
                                    (
                                        "readiness".into(),
                                        if ready {
                                            "ready".into()
                                        } else {
                                            m["server"]["reason"].as_str().unwrap_or("not ready").to_string()
                                        },
                                        Some(if ready { token::OK.into() } else { token::BAD.into() }),
                                    ),
                                    (
                                        "liveness".into(),
                                        if alive { "alive".into() } else { "down".into() },
                                        Some(if alive { token::OK.into() } else { token::BAD.into() }),
                                    ),
                                ]
                            />
                            <MetricCard
                                title="Compiler"
                                state=compiler_status.clone()
                                color=compiler_color
                                rows=vec![
                                    ("pending".into(), m["compiler"]["pending"].to_string(), None),
                                    ("building".into(), m["compiler"]["building"].to_string(), None),
                                    (
                                        "last success".into(),
                                        fmt_time(m["compiler"]["last_success_at"].as_str()),
                                        None,
                                    ),
                                ]
                            />
                            <MetricCard
                                title="Reconciler"
                                state={if failed > 0 { "degraded" } else { "healthy" }}
                                color={if failed > 0 { token::BAD } else { token::OK }}
                                rows=vec![
                                    ("available".into(), m["reconciler"]["built"].to_string(), None),
                                    (
                                        "failed builds".into(),
                                        failed.to_string(),
                                        (failed > 0).then(|| token::BAD.into()),
                                    ),
                                    (
                                        "last built".into(),
                                        fmt_time(m["reconciler"]["last_built_at"].as_str()),
                                        None,
                                    ),
                                ]
                            />
                            <MetricCard
                                title="Fleet"
                                state=format!("{} agent{}", fleet.len(), if fleet.len() == 1 { "" } else { "s" })
                                color={if fleet.is_empty() { token::MUTED } else { token::OK }}
                                rows=vec![
                                    ("in flight".into(), busy.to_string(), Some(token::ICE.into())),
                                    ("capacity".into(), capacity.to_string(), None),
                                    ("idle".into(), (capacity - busy).max(0).to_string(), None),
                                ]
                            />
                        </div>

                        // Agents roster
                        <div
                            style:display="flex"
                            style:justify-content="space-between"
                            style:border-bottom="1px solid var(--border-soft)"
                            style:padding-bottom="8px"
                            style:margin-top="6px"
                        >
                            <span style:font-size="14px" style:font-weight="600" style:color="var(--fg)">
                                "Execution agents"
                            </span>
                        </div>
                        {if fleet.is_empty() {
                            view! {
                                <div
                                    style:border="1px dashed var(--border)"
                                    style:border-radius="10px"
                                    style:padding="18px 15px"
                                    style:color="var(--faint)"
                                    style:font-size="12.5px"
                                >
                                    "No agents registered — work runs on the in-process executor."
                                </div>
                            }
                            .into_any()
                        } else {
                            view! {
                                <table class="cl-table">
                                    <thead>
                                        <tr>
                                            <th>"Agent"</th>
                                            <th>"Target"</th>
                                            <th>"Capacity"</th>
                                            <th>"Heartbeat"</th>
                                            <th>"Tenant"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {fleet
                                            .iter()
                                            .map(|a| {
                                                let hb = a["seconds_since_heartbeat"].as_i64();
                                                let stale = hb.map(|s| s > 60).unwrap_or(false);
                                                view! {
                                                    <tr>
                                                        <td>
                                                            <span style:display="inline-flex" style:gap="8px" style:align-items="center">
                                                                <span
                                                                    style:width="7px"
                                                                    style:height="7px"
                                                                    style:border-radius="50%"
                                                                    style:background=if stale { token::GOLD } else { token::OK }
                                                                ></span>
                                                                <span style:font-size="13px" style:font-weight="500" style:color="var(--fg)">
                                                                    {a["agent_id"].as_str().unwrap_or("—").to_string()}
                                                                </span>
                                                            </span>
                                                        </td>
                                                        <td>
                                                            <span style:font-family=MONO style:font-size="11.5px" style:color="var(--faint)">
                                                                {a["target_triple"].as_str().unwrap_or("—").to_string()}
                                                            </span>
                                                        </td>
                                                        <td>
                                                            <span style:font-family=MONO style:font-size="11.5px" style:color="var(--fg-2)">
                                                                {format!(
                                                                    "{}/{} in flight",
                                                                    a["in_flight"].as_i64().unwrap_or(0),
                                                                    a["max_concurrency"].as_i64().unwrap_or(0)
                                                                )}
                                                            </span>
                                                        </td>
                                                        <td>
                                                            <span
                                                                style:font-family=MONO
                                                                style:font-size="11.5px"
                                                                style:color=if stale { token::GOLD } else { "var(--faint)" }
                                                            >
                                                                {ago_secs(hb)}
                                                            </span>
                                                        </td>
                                                        <td>
                                                            <span style:font-family=MONO style:font-size="11.5px" style:color="var(--faint)">
                                                                {a["tenant_id"].as_str().unwrap_or("—").to_string()}
                                                            </span>
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                            .collect_view()}
                                    </tbody>
                                </table>
                            }
                            .into_any()
                        }}
                    }
                }}
            </Show>
        </div>
    }
}
