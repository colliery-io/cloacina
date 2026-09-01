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

use aurora_leptos::components::{Empty, Loading, PageHeader};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::{FireReactorRequest, GraphStatus};

use crate::auth::{client_for, use_auth};
use crate::components::{GraphInjectModal, TagPill};
use crate::data::{poll_resource, use_clock};
use crate::util::{health_color, node_kind_color, Throughput};

const MONO: &str = "'IBM Plex Mono', monospace";

pub(crate) fn health_state(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    v.get("state")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string()
}

#[component]
fn SectionLabel(#[prop(into)] label: String) -> impl IntoView {
    view! {
        <div
            style:font-family=MONO
            style:font-size="11px"
            style:letter-spacing=".06em"
            style:text-transform="uppercase"
            style:color="var(--muted)"
            style:margin="6px 0 8px"
        >
            {label}
        </div>
    }
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
        <div
            style:margin-top="7px"
            style:display="flex"
            style:align-items="center"
            style:gap="8px"
            style:flex-wrap="wrap"
        >
            {accumulators
                .into_iter()
                .map(|name| view! {
                    <span
                        style:display="inline-flex"
                        style:align-items="center"
                        style:gap="5px"
                        style:font-family=MONO
                        style:font-size="11px"
                        style:color="var(--fg-2)"
                    >
                        <span
                            style:width="6px"
                            style:height="6px"
                            style:border-radius="50%"
                            style:background=node_kind_color("accumulator")
                        ></span>
                        {name}
                    </span>
                })
                .collect_view()}
            <span style:color="var(--faint)">"→"</span>
            <TagPill color=token::VIOLET>{graph}</TagPill>
            {reaction_mode.map(|m| view! {
                <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                    {m}
                </span>
            })}
        </div>
    }
    .into_any()
}

#[component]
pub fn Graphs() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let clock = use_clock();

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

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="16px">
            <PageHeader title="Computation graphs" />
            <div style:font-family=MONO style:font-size="11px" style:color="var(--faint)" style:margin-top="-10px">
                {move || sub.get()}
            </div>

            // ---- Operational overview (UAT round 1, T-0938): the fleet of
            // graphs at a glance, before the per-object sections. ----
            {move || {
                clock.track(); // "last fire" ages by the second
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
                let last_fire = gs
                    .iter()
                    .filter_map(|g| g.last_fired_at.clone())
                    .max();
                // Availability, not activity: socket_only counts as up
                // (UAT round 4 — same semantics as health_color).
                let acc_live = accs_v
                    .iter()
                    .filter(|a| {
                        let s = a
                            .state
                            .clone()
                            .unwrap_or_else(|| health_state(&a.status));
                        crate::util::health_color(&s) == token::OK
                    })
                    .count();
                let most_active = gs.iter().max_by_key(|g| g.fires).map(|g| g.name.clone());
                let tile = |label: &str, value: String, color: &'static str, sub: String| {
                    let label = label.to_string();
                    view! {
                        <div
                            style:background="var(--panel)"
                            style:border="1px solid var(--border)"
                            style:border-radius="10px"
                            style:padding="13px 15px"
                        >
                            <div
                                style:font-family=MONO
                                style:font-size="10px"
                                style:letter-spacing=".07em"
                                style:text-transform="uppercase"
                                style:color="var(--muted)"
                            >
                                {label}
                            </div>
                            <div
                                class="cl-tnum"
                                style:font-size="24px"
                                style:font-weight="600"
                                style:line-height="1.2"
                                style:color=color
                                style:margin="4px 0 2px"
                            >
                                {value}
                            </div>
                            <div style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                {sub}
                            </div>
                        </div>
                    }
                };
                view! {
                    <div style:display="grid" style:grid-template-columns="repeat(5, 1fr)" style:gap="12px">
                        {tile(
                            "Graphs running",
                            format!("{running}/{}", gs.len()),
                            if running == gs.len() && !gs.is_empty() { token::OK } else { token::GOLD },
                            if paused > 0 { format!("{paused} paused") } else { "all unpaused".into() },
                        )}
                        {tile(
                            "Total fires",
                            total_fires.to_string(),
                            token::ICE,
                            "since load".into(),
                        )}
                        {tile(
                            "Last fire",
                            last_fire
                                .as_deref()
                                .map(|t| crate::util::ago(Some(t)))
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| "never".into()),
                            "var(--fg)",
                            "across all graphs".into(),
                        )}
                        {tile(
                            "Accumulators live",
                            format!("{acc_live}/{}", accs_v.len()),
                            if acc_live == accs_v.len() && !accs_v.is_empty() { token::OK } else { token::GOLD },
                            "event sources".into(),
                        )}
                        {tile(
                            "Most active",
                            most_active.clone().unwrap_or_else(|| "—".into()),
                            token::VIOLET,
                            "by fire count".into(),
                        )}
                    </div>
                }
            }}

            // ---- Graphs ----
            <div>
                <SectionLabel label="Graphs" />
                <Show
                    when=move || graphs.get().is_some()
                    fallback=|| view! { <Loading label="Loading graphs…" /> }
                >
                    <Show
                        when=move || !graph_items.get().is_empty()
                        fallback=|| view! { <Empty message="No graphs loaded." /> }
                    >
                        <div style:display="flex" style:flex-direction="column" style:gap="8px">
                            <For
                                each=move || graph_items.get()
                                key=|g: &GraphStatus| (g.name.clone(), g.fires, g.paused)
                                children=move |g| {
                                    let hs = health_state(&g.health);
                                    let hcolor = health_color(&hs);
                                    let rate = rate_of(&g.name, g.fires as f64);
                                    let nav_name = g.name.clone();
                                    view! {
                                        <div
                                            style:background="var(--panel)"
                                            style:border="1px solid var(--border)"
                                            style:border-radius="10px"
                                            style:padding="12px 15px"
                                            style:cursor="pointer"
                                            on:click=move |_| {
                                                navigate.with_value(|n| n(
                                                    &format!("/graphs/{}", urlencoding::encode(&nav_name)),
                                                    Default::default(),
                                                ))
                                            }
                                        >
                                            <div style:display="flex" style:justify-content="space-between" style:align-items="center">
                                                <div style:display="flex" style:gap="10px" style:align-items="center" style:min-width="0">
                                                    <span
                                                        style:width="8px"
                                                        style:height="8px"
                                                        style:border-radius="50%"
                                                        style:background=hcolor
                                                        style:flex="none"
                                                    ></span>
                                                    <span style:font-size="14px" style:font-weight="600" style:color="var(--fg)">
                                                        {g.name.clone()}
                                                    </span>
                                                    <span style:font-size="12px" style:color=hcolor>
                                                        {if hs.is_empty() { "unknown".to_string() } else { hs.clone() }}
                                                    </span>
                                                    <Show when={let p = g.paused; move || p}>
                                                        <TagPill color=token::GOLD>"paused"</TagPill>
                                                    </Show>
                                                </div>
                                                <span style:font-family=MONO style:font-size="11.5px" style:color="var(--faint)">
                                                    {rate.map(|r| format!("~{r}/min")).unwrap_or_else(|| "—".into())}
                                                </span>
                                            </div>
                                            <AccStrip
                                                accumulators=g.accumulators.clone()
                                                graph=g.name.clone()
                                                reaction_mode=g.reaction_mode.clone()
                                            />
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </Show>
                </Show>
            </div>

            // ---- Reactors ----
            <div>
                <SectionLabel label="Reactors" />
                <Show
                    when=move || !reactor_items.get().is_empty()
                    fallback=|| view! { <Empty message="No reactors." /> }
                >
                    <div style:display="flex" style:flex-direction="column" style:gap="8px">
                        <For
                            each=move || reactor_items.get()
                            key=|r| (r.name.clone(), r.paused)
                            children=move |r| {
                                let hs = health_state(&r.health);
                                let hcolor = health_color(&hs);
                                let fire_name = r.name.clone();
                                view! {
                                    <div
                                        style:background="var(--panel)"
                                        style:border="1px solid var(--border)"
                                        style:border-radius="10px"
                                        style:padding="12px 15px"
                                        style:display="flex"
                                        style:justify-content="space-between"
                                        style:align-items="center"
                                    >
                                        <div style:display="flex" style:gap="10px" style:align-items="center">
                                            <span
                                                style:width="8px"
                                                style:height="8px"
                                                style:border-radius="2px"
                                                style:background=node_kind_color("reactor")
                                                style:flex="none"
                                            ></span>
                                            <span style:font-size="13.5px" style:font-weight="600" style:color="var(--fg)">
                                                {r.name.clone()}
                                            </span>
                                            <span style:font-size="12px" style:color=hcolor>{hs.clone()}</span>
                                            <Show when={let p = r.paused; move || p}>
                                                <TagPill color=token::GOLD>"paused"</TagPill>
                                            </Show>
                                            <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                {format!(
                                                    "{} · {}",
                                                    r.reaction_mode.clone().unwrap_or_else(|| "—".into()),
                                                    r.input_strategy.clone().unwrap_or_else(|| "—".into())
                                                )}
                                            </span>
                                        </div>
                                        <Show when=move || auth.can_write()>
                                            {
                                                let name = fire_name.clone();
                                                view! {
                                                    <button
                                                        class="cl-btn cl-btn--subtle cl-btn--xs"
                                                        title="Force-fire with the current cache"
                                                        disabled=move || firing.get().is_some()
                                                        on:click={
                                                            let name = name.clone();
                                                            move |_| force_fire(name.clone())
                                                        }
                                                    >
                                                        "⚡ force-fire"
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
            </div>

            // ---- Accumulators ----
            <div>
                <SectionLabel label="Accumulators" />
                <Show
                    when=move || !acc_items.get().is_empty()
                    fallback=|| view! { <Empty message="No accumulators." /> }
                >
                    <div style:display="flex" style:flex-direction="column" style:gap="8px">
                        <For
                            each=move || acc_items.get()
                            key=|a| a.name.clone()
                            children=move |a| {
                                let state = a
                                    .state
                                    .clone()
                                    .unwrap_or_else(|| health_state(&a.status));
                                let hcolor = health_color(&state);
                                let inj_name = a.name.clone();
                                view! {
                                    <div
                                        style:background="var(--panel)"
                                        style:border="1px solid var(--border)"
                                        style:border-radius="10px"
                                        style:padding="12px 15px"
                                        style:display="flex"
                                        style:justify-content="space-between"
                                        style:align-items="center"
                                    >
                                        <div style:display="flex" style:gap="10px" style:align-items="center">
                                            <span
                                                style:width="8px"
                                                style:height="8px"
                                                style:border-radius="50%"
                                                style:background=node_kind_color("accumulator")
                                                style:flex="none"
                                            ></span>
                                            <span style:font-size="13.5px" style:font-weight="600" style:color="var(--fg)">
                                                {a.name.clone()}
                                            </span>
                                            <span style:font-size="12px" style:color=hcolor>{state.clone()}</span>
                                            {a.reactor.clone().map(|r| view! {
                                                <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                    {format!("→ {r}")}
                                                </span>
                                            })}
                                        </div>
                                        <Show when=move || auth.can_write()>
                                            {
                                                let name = inj_name.clone();
                                                view! {
                                                    <button
                                                        class="cl-btn cl-btn--subtle cl-btn--xs"
                                                        title="Inject a typed event"
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
            </div>

            <GraphInjectModal open=inject_open accumulator=inject_target />
        </div>
    }
}
