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

//! Graph operational view (CLOACI-T-0767), parity port of `GraphDetail.tsx`:
//! header + augmented topology (accumulators → reactor → compute nodes, the
//! WS-4 view) on the pack's SVG graph, per-accumulator freshness, reactor
//! force-fire and accumulator inject. UAT round 1 (CLOACI-T-0938) split the
//! page into a Live view (topology + accumulators) and an Operational-history
//! view (fire activity sparkline + recent fires table).

use aurora_leptos::components::{
    Button, Dot, Empty, Loading, PageHeader, Panel, Pill, RelativeTime, Sparkline, TabItem,
    TabPanel, Table, Tabs,
};
use aurora_leptos::data::use_now;
use aurora_leptos::graph::{Graph, GraphEdge, GraphNode};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use cloacina_api_types::FireReactorRequest;

use crate::auth::{client_for, use_auth};
use crate::components::GraphInjectModal;
use crate::data::poll_resource;
use crate::routes::graphs::health_state;
use crate::util::{ago, health_color, node_kind_color};

/// Last-event age label (UAT round 4): activity info only — the dot's color
/// comes from the accumulator's availability STATE (live/socket_only/…),
/// because liveness is "is this up", not "has it emitted recently".
fn last_event_label(last_event_at: Option<&str>) -> String {
    match last_event_at {
        Some(ts) => {
            let s = ago(Some(ts));
            if s.is_empty() {
                "—".into()
            } else {
                format!("last {s}")
            }
        }
        None => "no events yet".into(),
    }
}

#[component]
pub fn GraphDetail() -> impl IntoView {
    let auth = use_auth();
    let now = use_now();
    let params = use_params_map();
    let name = Signal::derive(move || params.read().get("name").unwrap_or_default());

    let graph = poll_resource(move |c| {
        let name = name.get();
        async move { c.get_graph(&name).await }
    });
    let accs = poll_resource(|c| async move { c.list_accumulators().await });

    let data = Signal::derive(move || graph.get().and_then(|r| r.ok()));
    let loading = Signal::derive(move || graph.get().is_none());
    let acc_rows = Signal::derive(move || {
        let mine = data.get().map(|d| d.accumulators).unwrap_or_default();
        accs.get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
            .into_iter()
            .filter(|a| mine.contains(&a.name))
            .collect::<Vec<_>>()
    });

    // Dual views (UAT round 1, T-0938).
    let view_mode = RwSignal::new("live".to_string());
    let reactor_name = Signal::derive(move || data.get().and_then(|d| d.reactor));
    let fires = poll_resource(move |c| {
        let reactor = reactor_name.get();
        async move {
            match reactor {
                Some(r) => c.list_reactor_fires(&r).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let fire_rows = Signal::derive(move || {
        fires
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let timeseries = poll_resource(move |c| {
        let reactor = reactor_name.get();
        async move {
            match reactor {
                Some(r) => c.reactor_fire_timeseries(&r).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let buckets = Signal::derive(move || {
        timeseries
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|t| t.buckets)
            .unwrap_or_default()
    });

    let inject_open = RwSignal::new(false);
    let inject_target = RwSignal::new(Option::<String>::None);
    let firing = RwSignal::new(false);

    let force_fire = move || {
        let Some(reactor) = data.get_untracked().and_then(|d| d.reactor) else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        firing.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client
                    .fire_reactor(&reactor, &FireReactorRequest::default())
                    .await;
            }
            firing.set(false);
        });
    };

    let header = move || {
        let meta = move || {
            data.get().map(|d| {
                let hs = health_state(&d.health);
                let hcolor = health_color(&hs);
                view! {
                    <Pill color=hcolor>{hs}</Pill>
                    <Show when=move || d.paused>
                        <Pill color=token::GOLD>"paused"</Pill>
                    </Show>
                    <span class="app-meta app-meta--sm">
                        {format!(
                            "reactor {} · {} · {} · {} fires",
                            d.reactor.clone().unwrap_or_else(|| "—".into()),
                            d.reaction_mode.clone().unwrap_or_else(|| "—".into()),
                            d.input_strategy.clone().unwrap_or_else(|| "—".into()),
                            d.fires
                        )}
                    </span>
                }
            })
        };
        view! {
            <PageHeader
                title=name.get()
                back_href="/graphs"
                back_label="Graphs"
                meta=Box::new(move || view! { {meta} }.into_any())
                actions=Box::new(move || view! {
                    <Show when=move || auth.can_write() && data.get().and_then(|d| d.reactor).is_some()>
                        <Button variant="default" loading=firing on_click=Callback::new(move |_| force_fire())>
                            "⚡ Force-fire"
                        </Button>
                    </Show>
                }.into_any())
            />
        }
    };

    let history = move || {
        view! {
            <div class="app-col app-col--loose">
                <Panel title="Fire activity" caption="fires per minute · last 60 minutes">
                    {move || {
                        let b = buckets.get();
                        if b.is_empty() {
                            return view! { <Empty message="No fire activity recorded." /> }.into_any();
                        }
                        let values: Vec<f64> = b.iter().map(|&c| c as f64).collect();
                        view! {
                            <div class="app-fires">
                                <Sparkline
                                    values=values
                                    bars=true
                                    fluid=true
                                    height=56.0
                                    color=token::ICE
                                    label="Fires per minute, last 60 minutes"
                                />
                            </div>
                        }
                        .into_any()
                    }}
                </Panel>

                <Panel title="Recent fires" caption="outcome · duration · inputs">
                    <Show
                        when=move || !fire_rows.get().is_empty()
                        fallback=|| view! { <Empty message="No fires yet for this graph." /> }
                    >
                        <Table label="Recent fires">
                            <thead>
                                <tr>
                                    <th>"Fired"</th>
                                    <th>"Outcome"</th>
                                    <th>"Duration"</th>
                                    <th>"Inputs"</th>
                                    <th>"Detail"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || fire_rows.get()
                                    key=|f| f.fired_at.clone()
                                    children=move |f| {
                                        let ok = f.ok;
                                        let inputs = f
                                            .inputs
                                            .keys()
                                            .cloned()
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        let detail = f
                                            .error
                                            .clone()
                                            .unwrap_or_else(|| format!("{} output(s)", f.outputs.len()));
                                        view! {
                                            <tr>
                                                <td class="app-meta app-meta--sm">
                                                    <RelativeTime iso=f.fired_at.clone() />
                                                </td>
                                                <td>
                                                    {if ok {
                                                        view! { <Pill color=token::OK>"ok"</Pill> }.into_any()
                                                    } else {
                                                        view! { <Pill color=token::BAD>"failed"</Pill> }.into_any()
                                                    }}
                                                </td>
                                                <td class="app-meta app-meta--md cl-tnum">
                                                    {format!("{}ms", f.duration_ms)}
                                                </td>
                                                <td class="app-meta app-meta--sm app-muted">
                                                    {if inputs.is_empty() { "—".to_string() } else { inputs }}
                                                </td>
                                                <td>
                                                    <span
                                                        class="app-meta app-meta--sm app-ellipsis app-clip"
                                                        class:app-bad=!ok
                                                        title=detail.clone()
                                                    >
                                                        {detail.clone()}
                                                    </span>
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </Table>
                    </Show>
                </Panel>
            </div>
        }
    };

    let topology = move || {
        let Some(d) = data.get() else {
            return view! { <Empty message="Graph not found." /> }.into_any();
        };
        let Some(topo) = d.topology.clone().filter(|t| !t.nodes.is_empty()) else {
            return view! { <Empty message="No topology emitted for this graph." /> }.into_any();
        };
        let mut nodes: Vec<GraphNode> = topo
            .nodes
            .iter()
            .map(|n| GraphNode::new(n.id.clone(), n.id.clone()).color(node_kind_color("compute")))
            .collect();
        let mut edges: Vec<GraphEdge> = topo
            .edges
            .iter()
            .map(|e| GraphEdge {
                from: e.from.clone(),
                to: e.to.clone(),
                active: false,
            })
            .collect();
        let has_incoming: std::collections::HashSet<&String> =
            topo.edges.iter().map(|e| &e.to).collect();
        let roots: Vec<String> = topo
            .nodes
            .iter()
            .filter(|n| !has_incoming.contains(&n.id))
            .map(|n| n.id.clone())
            .collect();
        let acc_ids: Vec<String> = d.accumulators.iter().map(|a| format!("acc:{a}")).collect();
        for a in &d.accumulators {
            nodes.push(
                GraphNode::new(format!("acc:{a}"), a.clone())
                    .color(node_kind_color("accumulator"))
                    .sublabel("accumulator"),
            );
        }
        if let Some(reactor) = d.reactor.clone() {
            let rid = format!("reactor:{reactor}");
            nodes.push(
                GraphNode::new(rid.clone(), reactor)
                    .color(node_kind_color("reactor"))
                    .sublabel("reactor"),
            );
            for a in &acc_ids {
                edges.push(GraphEdge {
                    from: a.clone(),
                    to: rid.clone(),
                    active: false,
                });
            }
            for r in &roots {
                edges.push(GraphEdge {
                    from: rid.clone(),
                    to: r.clone(),
                    active: false,
                });
            }
        } else {
            for a in &acc_ids {
                for r in &roots {
                    edges.push(GraphEdge {
                        from: a.clone(),
                        to: r.clone(),
                        active: false,
                    });
                }
            }
        }
        view! {
            <div data-testid="graph-dag">
                <Graph nodes=nodes edges=edges direction="LR" />
            </div>
        }
        .into_any()
    };

    let live = move || {
        view! {
            <div class="app-col app-col--loose">
                // Topology: sources → reactor → compute nodes (WS-4).
                <Panel title="Topology">{topology}</Panel>

                // Accumulator freshness + inject
                <Panel title="Accumulators" caption="state · events · last event">
                    <Show
                        when=move || !acc_rows.get().is_empty()
                        fallback=|| view! { <Empty message="No accumulators feed this graph." /> }
                    >
                        <Table label="Accumulators">
                            <thead>
                                <tr>
                                    <th>"Accumulator"</th>
                                    <th>"State"</th>
                                    <th>"Events"</th>
                                    <th>"Last event"</th>
                                    <th class="app-w-action"><span class="cl-sr-only">"Inject"</span></th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || acc_rows.get()
                                    // Volatile fields in the key so the row
                                    // re-renders when state/activity move.
                                    key=|a| (a.name.clone(), a.state.clone(), a.last_event_at.clone())
                                    children=move |a| {
                                        let state = a
                                            .state
                                            .clone()
                                            .unwrap_or_else(|| health_state(&a.status));
                                        // Dot = availability (state); the event age
                                        // is info text ticking on the 1s clock.
                                        let dot = health_color(&state);
                                        let last_ev = StoredValue::new(a.last_event_at.clone());
                                        let last_label = move || {
                                            now.track();
                                            last_ev.with_value(|ts| last_event_label(ts.as_deref()))
                                        };
                                        let events = a
                                            .events_total
                                            .map(|n| n.to_string())
                                            .unwrap_or_else(|| "—".into());
                                        let inj = a.name.clone();
                                        view! {
                                            <tr>
                                                <td>
                                                    <span class="app-row app-row--tight">
                                                        <Dot color=dot size=9 />
                                                        <span class="app-mono app-small app-fg">{a.name.clone()}</span>
                                                    </span>
                                                </td>
                                                <td><Pill color=dot>{state.clone()}</Pill></td>
                                                <td class="app-meta app-meta--md">{format!("{events} events")}</td>
                                                <td class="app-meta app-meta--sm">{last_label}</td>
                                                <td>
                                                    <Show when=move || auth.can_write()>
                                                        {
                                                            let name = inj.clone();
                                                            view! {
                                                                <Button
                                                                    variant="subtle"
                                                                    size="xs"
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
                    </Show>
                </Panel>
            </div>
        }
    };

    view! {
        <div class="app-page">
            {header}

            // Dual views (UAT round 1, T-0938): live vs operational history.
            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading graph…" /> }
            >
                <Tabs
                    tabs=vec![TabItem::new("live", "Live"), TabItem::new("history", "Operational history")]
                    value=view_mode
                    label="Graph views"
                >
                    <TabPanel value="live">{live}</TabPanel>
                    <TabPanel value="history">{history}</TabPanel>
                </Tabs>
            </Show>

            <GraphInjectModal open=inject_open accumulator=inject_target />
        </div>
    }
}
