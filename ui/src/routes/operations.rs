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

use aurora_leptos::components::{
    DetailList, Dot, KeyValue, LiveIndicator, LiveState, PageHeader, Pill, SectionLabel, Table,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use crate::ops::use_ops_metrics;

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

/// One component card: a title with its state pill, then label / value rows
/// (a value may carry a hue token).
#[component]
fn MetricCard(
    #[prop(into)] title: String,
    #[prop(into)] state: String,
    #[prop(into)] color: String,
    rows: Vec<(String, String, Option<String>)>,
) -> impl IntoView {
    view! {
        <div class="app-panel app-col">
            <div class="app-row app-row--between">
                <span class="app-name">{title}</span>
                <Pill color=color>{state}</Pill>
            </div>
            <DetailList mono=true label_width="96px">
                {rows
                    .into_iter()
                    .map(|(label, value, vcolor)| view! {
                        <KeyValue label=label>
                            // The hue is the value's state (data).
                            <span style:color=vcolor.unwrap_or_else(|| "var(--fg-2)".into())>{value}</span>
                        </KeyValue>
                    })
                    .collect_view()}
            </DetailList>
        </div>
    }
}

#[component]
pub fn Operations() -> impl IntoView {
    let ops = use_ops_metrics();
    let live = Signal::derive(move || ops.get().is_some());
    let live_state = Signal::derive(move || {
        if live.get() {
            LiveState::Live
        } else {
            LiveState::Connecting
        }
    });

    view! {
        <div class="app-page">
            <PageHeader
                title="Operations"
                sub="Deployment health for the connected server, pushed over the control-plane socket."
                meta=Box::new(move || view! {
                    <LiveIndicator state=live_state live_label="live" connecting_label="connecting…" />
                }.into_any())
            />

            <Show
                when=move || live.get()
                fallback=|| view! { <span class="app-hint">"Subscribing to operational metrics…"</span> }
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
                        <div class="app-grid-4">
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
                        <div>
                            <SectionLabel label="Execution agents" divider=true count=Some(fleet.len()) />
                            {if fleet.is_empty() {
                                view! {
                                    <div class="app-empty">
                                        "No agents registered — work runs on the in-process executor."
                                    </div>
                                }
                                .into_any()
                            } else {
                                view! {
                                    <div class="app-panel app-panel--flush">
                                        <Table label="Execution agents">
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
                                                                    <span class="app-row app-row--tight">
                                                                        <Dot color=if stale { token::GOLD } else { token::OK } size=7 />
                                                                        <span class="app-text app-strong">
                                                                            {a["agent_id"].as_str().unwrap_or("—").to_string()}
                                                                        </span>
                                                                    </span>
                                                                </td>
                                                                <td class="app-meta app-meta--md app-faint">
                                                                    {a["target_triple"].as_str().unwrap_or("—").to_string()}
                                                                </td>
                                                                <td class="app-meta app-meta--md">
                                                                    {format!(
                                                                        "{}/{} in flight",
                                                                        a["in_flight"].as_i64().unwrap_or(0),
                                                                        a["max_concurrency"].as_i64().unwrap_or(0)
                                                                    )}
                                                                </td>
                                                                <td class="app-meta app-meta--md app-faint" class:app-gold=stale>
                                                                    {ago_secs(hb)}
                                                                </td>
                                                                <td class="app-meta app-meta--md app-faint">
                                                                    {a["tenant_id"].as_str().unwrap_or("—").to_string()}
                                                                </td>
                                                            </tr>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </tbody>
                                        </Table>
                                    </div>
                                }
                                .into_any()
                            }}
                        </div>
                    }
                }}
            </Show>
        </div>
    }
}
