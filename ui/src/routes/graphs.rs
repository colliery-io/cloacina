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

//! Computation graphs (Aurora Dark spec 08), parity port of `Graphs.tsx`:
//! graphs / reactors / accumulators as card rows, with per-name events/min
//! derived from the monotonic fire counters, reactor force-fire, and the
//! accumulator inject modal.

use aurora_leptos::components::{
    Button, Card, Dot, Empty, Loading, PageHeader, Pill, SectionLabel, StatTile, Table,
};
use aurora_leptos::data::use_now;
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use cloacina_api_types::{FireReactorRequest, GraphStatus};

use crate::auth::{client_for, use_auth};
use crate::components::GraphInjectModal;
use crate::data::poll_resource;
use crate::util::{health_color, node_kind_color, Throughput};

pub(crate) fn health_state(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    v.get("state")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string()
}

/// The accumulators → graph flow strip under a graph card.
#[component]
fn AccStrip(
    accumulators: Vec<String>,
    #[prop(into)] graph: String,
    reaction_mode: Option<String>,
) -> impl IntoView {
    if accumulators.is_empty() {
        return ().into_any();
    }
    view! {
        <div class="app-accstrip">
            {accumulators
                .into_iter()
                .map(|name| view! {
                    <span class="app-row app-row--tight app-meta app-meta--sm app-fg2">
                        <Dot color=node_kind_color("accumulator") size=6 />
                        {name}
                    </span>
                })
                .collect_view()}
            <span class="app-faint" aria-hidden="true">"→"</span>
            <Pill color=token::VIOLET>{graph}</Pill>
            {reaction_mode.map(|m| view! { <span class="app-meta">{m}</span> })}
        </div>
    }
    .into_any()
}

#[component]
pub fn Graphs() -> impl IntoView {
    let auth = use_auth();
    let now = use_now();

    let graphs = poll_resource(|c| async move { c.list_graphs().await });
    let reactors = poll_resource(|c| async move { c.list_reactors().await });
    let accs = poll_resource(|c| async move { c.list_accumulators().await });

    let graph_items = Signal::derive(move || {
        graphs
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let reactor_items = Signal::derive(move || {
        reactors
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let acc_items = Signal::derive(move || {
        accs.get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });

    // Events/min from the monotonic fire counters (T-0744 semantics).
    let throughput = StoredValue::new(Throughput::default());
    let rate_of = move |name: &str, total: f64| -> Option<f64> {
        let mut out = None;
        throughput.update_value(|t| out = t.sample(name, total));
        out
    };

    let inject_open = RwSignal::new(false);
    let inject_target = RwSignal::new(Option::<String>::None);
    let firing = RwSignal::new(Option::<String>::None);

    let force_fire = move |name: String| {
        let Some(conn) = auth.connection() else {
            return;
        };
        firing.set(Some(name.clone()));
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client
                    .fire_reactor(&name, &FireReactorRequest::default())
                    .await;
            }
            firing.set(None);
        });
    };

    let sub = Signal::derive(move || {
        format!(
            "{} graphs · {} reactors · {} accumulators",
            graph_items.get().len(),
            reactor_items.get().len(),
            acc_items.get().len()
        )
    });

    // ---- Operational overview (UAT round 1, T-0938): the fleet of graphs
    // at a glance, before the per-object sections. ----
    let overview = move || {
        now.track(); // "last fire" ages by the second
        let gs = graph_items.get();
        let accs_v = acc_items.get();
        let running = gs
            .iter()
            .filter(|g| {
                matches!(
                    health_state(&g.health).to_lowercase().as_str(),
                    "running" | "live" | "ok" | "healthy"
                )
            })
            .count();
        let paused = gs.iter().filter(|g| g.paused).count();
        let total_fires: u64 = gs.iter().map(|g| g.fires).sum();
        let last_fire = gs.iter().filter_map(|g| g.last_fired_at.clone()).max();
        // Availability, not activity: socket_only counts as up
        // (UAT round 4 — same semantics as health_color).
        let acc_live = accs_v
            .iter()
            .filter(|a| {
                let s = a.state.clone().unwrap_or_else(|| health_state(&a.status));
                crate::util::health_color(&s) == token::OK
            })
            .count();
        let most_active = gs.iter().max_by_key(|g| g.fires).map(|g| g.name.clone());
        let all_running = running == gs.len() && !gs.is_empty();
        let all_live = acc_live == accs_v.len() && !accs_v.is_empty();
        view! {
            <div class="app-grid-5">
                <StatTile
                    label="Graphs running"
                    value=format!("{running}/{}", gs.len())
                    color={if all_running { token::OK } else { token::GOLD }}
                    sub={if paused > 0 { format!("{paused} paused") } else { "all unpaused".to_string() }}
                />
                <StatTile label="Total fires" value=total_fires.to_string() color=token::ICE sub="since load".to_string() />
                <StatTile
                    label="Last fire"
                    value={last_fire
                        .as_deref()
                        .map(|t| crate::util::ago(Some(t)))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "never".into())}
                    sub="across all graphs".to_string()
                />
                <StatTile
                    label="Accumulators live"
                    value=format!("{acc_live}/{}", accs_v.len())
                    color={if all_live { token::OK } else { token::GOLD }}
                    sub="event sources".to_string()
                />
                <StatTile
                    label="Most active"
                    value=most_active.unwrap_or_else(|| "—".into())
                    color=token::VIOLET
                    sub="by fire count".to_string()
                />
            </div>
        }
    };

    view! {
        <div class="app-page">
            <PageHeader
                title="Computation graphs"
                meta=Box::new(move || view! { <span class="app-meta app-meta--sm">{move || sub.get()}</span> }.into_any())
            />

            {overview}

            // ---- Graphs ----
            <div>
                <SectionLabel label="Graphs" count=Signal::derive(move || Some(graph_items.get().len())) />
                <Show
                    when=move || graphs.get().is_some()
                    fallback=|| view! { <Loading label="Loading graphs…" /> }
                >
                    <Show
                        when=move || !graph_items.get().is_empty()
                        fallback=|| view! { <Empty message="No graphs loaded." /> }
                    >
                        <div class="app-col">
                            <For
                                each=move || graph_items.get()
                                key=|g: &GraphStatus| (g.name.clone(), g.fires, g.paused)
                                children=move |g| {
                                    let hs = health_state(&g.health);
                                    let hcolor = health_color(&hs);
                                    let rate = rate_of(&g.name, g.fires as f64);
                                    let paused = g.paused;
                                    view! {
                                        <Card href=format!("/graphs/{}", urlencoding::encode(&g.name))>
                                            <div class="app-row app-row--between">
                                                <div class="app-row">
                                                    <Dot color=hcolor />
                                                    <span class="app-name app-bright">{g.name.clone()}</span>
                                                    <Pill color=hcolor>
                                                        {if hs.is_empty() { "unknown".to_string() } else { hs.clone() }}
                                                    </Pill>
                                                    <Show when=move || paused>
                                                        <Pill color=token::GOLD>"paused"</Pill>
                                                    </Show>
                                                </div>
                                                <span class="app-meta app-meta--md app-faint">
                                                    {rate.map(|r| format!("~{r}/min")).unwrap_or_else(|| "—".into())}
                                                </span>
                                            </div>
                                            <AccStrip
                                                accumulators=g.accumulators.clone()
                                                graph=g.name.clone()
                                                reaction_mode=g.reaction_mode.clone()
                                            />
                                        </Card>
                                    }
                                }
                            />
                        </div>
                    </Show>
                </Show>
            </div>

            // ---- Reactors ----
            <div>
                <SectionLabel label="Reactors" count=Signal::derive(move || Some(reactor_items.get().len())) />
                <Show
                    when=move || !reactor_items.get().is_empty()
                    fallback=|| view! { <Empty message="No reactors." /> }
                >
                    <div class="app-panel app-panel--flush">
                        <Table label="Reactors">
                            <thead>
                                <tr>
                                    <th>"Reactor"</th>
                                    <th>"Health"</th>
                                    <th>"Mode · strategy"</th>
                                    <th class="app-w-wide-action"><span class="cl-sr-only">"Actions"</span></th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || reactor_items.get()
                                    key=|r| (r.name.clone(), r.paused)
                                    children=move |r| {
                                        let hs = health_state(&r.health);
                                        let hcolor = health_color(&hs);
                                        let fire_name = r.name.clone();
                                        let paused = r.paused;
                                        view! {
                                            <tr>
                                                <td>
                                                    <span class="app-row">
                                                        <span class="app-square" style:background=node_kind_color("reactor")></span>
                                                        <span class="app-name">{r.name.clone()}</span>
                                                        <Show when=move || paused>
                                                            <Pill color=token::GOLD>"paused"</Pill>
                                                        </Show>
                                                    </span>
                                                </td>
                                                <td><Pill color=hcolor>{hs.clone()}</Pill></td>
                                                <td class="app-meta">
                                                    {format!(
                                                        "{} · {}",
                                                        r.reaction_mode.clone().unwrap_or_else(|| "—".into()),
                                                        r.input_strategy.clone().unwrap_or_else(|| "—".into())
                                                    )}
                                                </td>
                                                <td class="app-right">
                                                    <Show when=move || auth.can_write()>
                                                        {
                                                            let name = fire_name.clone();
                                                            view! {
                                                                <Button
                                                                    variant="subtle"
                                                                    size="xs"
                                                                    title="Force-fire with the current cache"
                                                                    disabled=Signal::derive(move || firing.get().is_some())
                                                                    on_click=Callback::new(move |_| force_fire(name.clone()))
                                                                >
                                                                    "⚡ force-fire"
                                                                </Button>
                                                            }
                                                        }
                                                    </Show>
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </Table>
                    </div>
                </Show>
            </div>

            // ---- Accumulators ----
            <div>
                <SectionLabel label="Accumulators" count=Signal::derive(move || Some(acc_items.get().len())) />
                <Show
                    when=move || !acc_items.get().is_empty()
                    fallback=|| view! { <Empty message="No accumulators." /> }
                >
                    <div class="app-panel app-panel--flush">
                        <Table label="Accumulators">
                            <thead>
                                <tr>
                                    <th>"Accumulator"</th>
                                    <th>"State"</th>
                                    <th>"Reactor"</th>
                                    <th class="app-w-wide-action"><span class="cl-sr-only">"Actions"</span></th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || acc_items.get()
                                    key=|a| a.name.clone()
                                    children=move |a| {
                                        let state = a.state.clone().unwrap_or_else(|| health_state(&a.status));
                                        let hcolor = health_color(&state);
                                        let inj_name = a.name.clone();
                                        view! {
                                            <tr>
                                                <td>
                                                    <span class="app-row">
                                                        <Dot color=node_kind_color("accumulator") />
                                                        <span class="app-name">{a.name.clone()}</span>
                                                    </span>
                                                </td>
                                                <td><Pill color=hcolor>{state.clone()}</Pill></td>
                                                <td class="app-meta">
                                                    {a.reactor.clone().map(|r| format!("→ {r}")).unwrap_or_else(|| "—".into())}
                                                </td>
                                                <td class="app-right">
                                                    <Show when=move || auth.can_write()>
                                                        {
                                                            let name = inj_name.clone();
                                                            view! {
                                                                <Button
                                                                    variant="subtle"
                                                                    size="xs"
                                                                    title="Inject a typed event"
                                                                    on_click=Callback::new(move |_| {
                                                                        inject_target.set(Some(name.clone()));
                                                                        inject_open.set(true);
                                                                    })
                                                                >
                                                                    "＋ inject"
                                                                </Button>
                                                            }
                                                        }
                                                    </Show>
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </Table>
                    </div>
                </Show>
            </div>

            <GraphInjectModal open=inject_open accumulator=inject_target />
        </div>
    }
}
