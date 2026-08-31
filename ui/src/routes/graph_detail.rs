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

use aurora_leptos::components::{Empty, Loading, Panel};
use aurora_leptos::graph::{Graph, GraphEdge, GraphNode};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use cloacina_api_types::FireReactorRequest;

use crate::auth::{client_for, use_auth};
use crate::components::{GraphInjectModal, TagPill, ViewTabs};
use crate::data::poll_resource;
use crate::routes::graphs::health_state;
use crate::util::{ago, health_color, node_kind_color};

const MONO: &str = "'IBM Plex Mono', monospace";

#[component]
pub fn GraphDetail() -> impl IntoView {
    let auth = use_auth();
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
    let view_mode = RwSignal::new("live");
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

    let force_fire = move |_| {
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

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="16px">
            // Header
            <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                <div>
                    <a
                        href="/graphs"
                        style:font-family=MONO
                        style:font-size="11.5px"
                        style:color="var(--muted)"
                        style:text-decoration="none"
                    >
                        "← Graphs"
                    </a>
                    <div style:display="flex" style:gap="10px" style:align-items="center" style:margin-top="3px">
                        <h1
                            style:font-size="23px"
                            style:font-weight="600"
                            style:color="var(--fg-bright)"
                            style:margin="0"
                        >
                            {move || name.get()}
                        </h1>
                        {move || data.get().map(|d| {
                            let hs = health_state(&d.health);
                            let hcolor = health_color(&hs);
                            view! {
                                <span style:font-size="12.5px" style:color=hcolor>{hs}</span>
                            }
                        })}
                        <Show when=move || data.get().map(|d| d.paused).unwrap_or(false)>
                            <TagPill color=token::GOLD>"paused"</TagPill>
                        </Show>
                    </div>
                    <div
                        style:font-family=MONO
                        style:font-size="11px"
                        style:color="var(--faint)"
                        style:margin-top="4px"
                    >
                        {move || data.get().map(|d| format!(
                            "reactor {} · {} · {} · {} fires",
                            d.reactor.clone().unwrap_or_else(|| "—".into()),
                            d.reaction_mode.clone().unwrap_or_else(|| "—".into()),
                            d.input_strategy.clone().unwrap_or_else(|| "—".into()),
                            d.fires
                        )).unwrap_or_default()}
                    </div>
                </div>
                <Show when=move || auth.can_write() && data.get().and_then(|d| d.reactor).is_some()>
                    <button
                        class="cl-btn cl-btn--default"
                        disabled=move || firing.get()
                        on:click=force_fire
                    >
                        "⚡ Force-fire"
                    </button>
                </Show>
            </div>

            // View switcher (UAT round 1, T-0938)
            <ViewTabs
                tabs=vec![("live", "Live"), ("history", "Operational history")]
                active=view_mode
            />

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading graph…" /> }
            >
                // ---- Operational-history view ----
                <Show when=move || view_mode.get() == "history">
                    <Panel title="Fire activity" caption="fires per minute · last 60 minutes">
                        {move || {
                            let b = buckets.get();
                            if b.is_empty() {
                                return view! { <Empty message="No fire activity recorded." /> }
                                    .into_any();
                            }
                            let max = b.iter().copied().max().unwrap_or(1).max(1);
                            view! {
                                <div style:display="flex" style:gap="2px" style:align-items="flex-end" style:height="56px">
                                    {b
                                        .iter()
                                        .map(|&count| {
                                            let h = if count == 0 {
                                                2.0
                                            } else {
                                                6.0 + 50.0 * (count as f64 / max as f64)
                                            };
                                            view! {
                                                <div
                                                    style:flex="1"
                                                    style:height=format!("{h:.0}px")
                                                    style:border-radius="1px"
                                                    style:background=if count == 0 {
                                                        "var(--border)"
                                                    } else {
                                                        token::ICE
                                                    }
                                                    title=format!("{count} fires")
                                                ></div>
                                            }
                                        })
                                        .collect_view()}
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
                            <table class="cl-table">
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
                                        children=|f| {
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
                                                .unwrap_or_else(|| {
                                                    format!("{} output(s)", f.outputs.len())
                                                });
                                            view! {
                                                <tr>
                                                    <td>
                                                        <span style:font-family=MONO style:font-size="11px" style:color="var(--faint)">
                                                            {ago(Some(f.fired_at.as_str()))}
                                                        </span>
                                                    </td>
                                                    <td>
                                                        <span
                                                            style:font-size="12px"
                                                            style:color=if ok { token::OK } else { "var(--bad)" }
                                                        >
                                                            {if ok { "ok" } else { "failed" }}
                                                        </span>
                                                    </td>
                                                    <td>
                                                        <span class="cl-tnum" style:font-size="11.5px" style:color="var(--fg-2)">
                                                            {format!("{}ms", f.duration_ms)}
                                                        </span>
                                                    </td>
                                                    <td>
                                                        <span style:font-family=MONO style:font-size="11px" style:color="var(--muted)">
                                                            {if inputs.is_empty() { "—".to_string() } else { inputs }}
                                                        </span>
                                                    </td>
                                                    <td>
                                                        <span
                                                            style:font-family=MONO
                                                            style:font-size="11px"
                                                            style:color=if ok { "var(--faint)" } else { "var(--bad)" }
                                                            style:overflow="hidden"
                                                            style:text-overflow="ellipsis"
                                                            style:white-space="nowrap"
                                                            style:max-width="340px"
                                                            style:display="inline-block"
                                                        >
                                                            {detail}
                                                        </span>
                                                    </td>
                                                </tr>
                                            }
                                        }
                                    />
                                </tbody>
                            </table>
                        </Show>
                    </Panel>
                </Show>

                // ---- Live view ----
                <Show when=move || view_mode.get() == "live">
                // Topology: sources → reactor → compute nodes (WS-4).
                <Panel title="Topology">
                    {move || {
                        let Some(d) = data.get() else {
                            return view! { <Empty message="Graph not found." /> }.into_any();
                        };
                        let Some(topo) = d.topology.clone().filter(|t| !t.nodes.is_empty()) else {
                            return view! { <Empty message="No topology emitted for this graph." /> }
                                .into_any();
                        };
                        let mut nodes: Vec<GraphNode> = topo
                            .nodes
                            .iter()
                            .map(|n| {
                                GraphNode::new(n.id.clone(), n.id.clone())
                                    .color(node_kind_color("compute"))
                            })
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
                        let acc_ids: Vec<String> =
                            d.accumulators.iter().map(|a| format!("acc:{a}")).collect();
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
                                edges.push(GraphEdge { from: a.clone(), to: rid.clone(), active: false });
                            }
                            for r in &roots {
                                edges.push(GraphEdge { from: rid.clone(), to: r.clone(), active: false });
                            }
                        } else {
                            for a in &acc_ids {
                                for r in &roots {
                                    edges.push(GraphEdge { from: a.clone(), to: r.clone(), active: false });
                                }
                            }
                        }
                        view! { <Graph nodes=nodes edges=edges direction="LR" /> }.into_any()
                    }}
                </Panel>

                // Accumulator freshness + inject
                <Panel title="Accumulators" caption="state · events · last event">
                    <Show
                        when=move || !acc_rows.get().is_empty()
                        fallback=|| view! { <Empty message="No accumulators feed this graph." /> }
                    >
                        <div style:display="flex" style:flex-direction="column" style:gap="6px">
                            <For
                                each=move || acc_rows.get()
                                key=|a| a.name.clone()
                                children=move |a| {
                                    let state = a
                                        .state
                                        .clone()
                                        .unwrap_or_else(|| health_state(&a.status));
                                    let inj = a.name.clone();
                                    view! {
                                        <div style:display="flex" style:gap="12px" style:align-items="center">
                                            <span
                                                style:width="7px"
                                                style:height="7px"
                                                style:border-radius="50%"
                                                style:background=health_color(&state)
                                                style:flex="none"
                                            ></span>
                                            <span
                                                style:font-family=MONO
                                                style:font-size="12.5px"
                                                style:color="var(--fg)"
                                                style:min-width="180px"
                                            >
                                                {a.name.clone()}
                                            </span>
                                            <span style:font-size="12px" style:color=health_color(&state)>
                                                {state.clone()}
                                            </span>
                                            <span style:flex="1"></span>
                                            <Show when=move || auth.can_write()>
                                                {
                                                    let name = inj.clone();
                                                    view! {
                                                        <button
                                                            class="cl-btn cl-btn--subtle cl-btn--xs"
                                                            on:click={
                                                                let name = name.clone();
                                                                move |_| {
                                                                    inject_target.set(Some(name.clone()));
                                                                    inject_open.set(true);
                                                                }
                                                            }
                                                        >
                                                            "＋ inject"
                                                        </button>
                                                    }
                                                }
                                            </Show>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </Show>
                </Panel>
                </Show>
            </Show>

            <GraphInjectModal open=inject_open accumulator=inject_target />
        </div>
    }
}
