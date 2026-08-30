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

//! Tenant agent-fleet management (CLOACI-T-0813), parity port of
//! `Fleet.tsx`: desired vs actual vs effective-limit stats, limit
//! provenance line, and admin-gated Provision +1 / Deprovision −1.
//! The fleet/limits endpoints have no typed SDK methods yet (the React app
//! hand-fetched too) — rides the client's public `get_json`/`post_json`
//! escape hatch; typed methods are an SDK follow-up noted in the ticket.

use aurora_leptos::components::{Alert, Loading, PageHeader};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use crate::auth::{client_for, use_auth};
use crate::data::poll_resource;

const MONO: &str = "'IBM Plex Mono', monospace";

#[component]
fn Stat(
    value: Signal<i64>,
    #[prop(into)] label: String,
    #[prop(optional, into)] color: Option<String>,
) -> impl IntoView {
    view! {
        <div
            style:flex="1"
            style:background="var(--sidebar)"
            style:border="1px solid var(--border)"
            style:border-radius="12px"
            style:padding="14px 16px"
        >
            <div
                style:font-family=MONO
                style:font-size="28px"
                style:font-weight="600"
                style:color=color.unwrap_or_else(|| "var(--fg)".into())
            >
                {move || value.get()}
            </div>
            <div
                style:font-family=MONO
                style:font-size="10px"
                style:letter-spacing=".1em"
                style:text-transform="uppercase"
                style:color="var(--faint)"
                style:margin-top="4px"
            >
                {label}
            </div>
        </div>
    }
}

#[component]
pub fn Fleet() -> impl IntoView {
    let auth = use_auth();
    let refresh = RwSignal::new(0u32);
    let tenant = move || auth.connection().map(|c| c.tenant).unwrap_or_default();

    let fleet = poll_resource(move |c| {
        refresh.get();
        let t = use_auth()
            .connection()
            .map(|c| c.tenant)
            .unwrap_or_default();
        async move {
            c.get_json::<serde_json::Value>(&format!("/v1/tenants/{t}/fleet"))
                .await
        }
    });
    let limits = poll_resource(move |c| {
        let t = use_auth()
            .connection()
            .map(|c| c.tenant)
            .unwrap_or_default();
        async move {
            c.get_json::<serde_json::Value>(&format!("/v1/tenants/{t}/limits"))
                .await
        }
    });

    let state = Signal::derive(move || fleet.get().and_then(|r| r.ok()));
    let desired = Signal::derive(move || {
        state
            .get()
            .and_then(|s| s["desired_count"].as_i64())
            .unwrap_or(0)
    });
    let actual = Signal::derive(move || {
        state
            .get()
            .and_then(|s| s["actual_count"].as_i64())
            .unwrap_or(0)
    });
    let limit = Signal::derive(move || {
        state
            .get()
            .and_then(|s| s["effective_limit"].as_i64())
            .unwrap_or(0)
    });
    let at_capacity = Signal::derive(move || state.get().is_some() && desired.get() >= limit.get());

    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());

    let scale = move |direction: &'static str| {
        let Some(conn) = auth.connection() else {
            return;
        };
        let t = conn.tenant.clone();
        busy.set(true);
        error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .post_json::<serde_json::Value, serde_json::Value>(
                        &format!("/v1/tenants/{t}/fleet/{direction}"),
                        &serde_json::Value::Null,
                    )
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            refresh.update(|n| *n += 1);
            if let Err(e) = result {
                error.set(e);
            }
        });
    };

    view! {
        <div style:max-width="820px" style:display="flex" style:flex-direction="column" style:gap="14px">
            <PageHeader
                title="Agent fleet"
                sub=format!(
                    "Provisioned vs running agents for tenant {}, against this tenant's effective agent limit.",
                    tenant()
                )
            />

            <Show
                when=move || state.get().is_some()
                fallback=|| view! { <Loading label="Loading fleet…" /> }
            >
                <div style:display="flex" style:gap="12px">
                    <Stat value=desired label="Provisioned" />
                    <Stat value=actual label="Running" color=token::ICE />
                    <Stat value=limit label="Effective limit" />
                </div>
            </Show>

            {move || limits.get().and_then(|r| r.ok()).map(|l| {
                let effective = l["effective_limit"].as_i64().unwrap_or(0);
                let line = match l["tenant_override"].as_i64() {
                    Some(o) => format!("Effective limit {effective} = tenant override {o}."),
                    None => format!(
                        "Effective limit {effective} = platform default {} (no tenant override).",
                        l["default_max_agents"].as_i64().unwrap_or(0)
                    ),
                };
                view! { <div style:font-size="12px" style:color="var(--muted)">{line}</div> }
            })}

            <Show
                when=move || auth.can_admin()
                fallback=|| view! {
                    <Alert color="var(--gold)">
                        "You need admin access to provision or deprovision agents."
                    </Alert>
                }
            >
                <div
                    style:background="var(--sidebar)"
                    style:border="1px solid var(--border)"
                    style:border-radius="12px"
                    style:padding="16px"
                >
                    <div style:font-size="13px" style:font-weight="600" style:color="var(--fg)" style:margin-bottom="10px">
                        "Scale fleet"
                    </div>
                    <div style:display="flex" style:gap="10px" style:align-items="center">
                        <button
                            class="cl-btn cl-btn--filled"
                            disabled=move || busy.get() || at_capacity.get()
                            on:click=move |_| scale("provision")
                        >
                            "Provision +1"
                        </button>
                        <button
                            class="cl-btn cl-btn--default"
                            disabled=move || busy.get() || desired.get() <= 0
                            on:click=move |_| scale("deprovision")
                        >
                            "Deprovision −1"
                        </button>
                        <Show when=move || at_capacity.get()>
                            <span style:font-size="12px" style:color="var(--muted)">
                                {move || format!("At capacity ({}).", limit.get())}
                            </span>
                        </Show>
                    </div>
                    <Show when=move || !error.get().is_empty()>
                        <div style:margin-top="10px">
                            <Alert color="var(--bad)">{move || error.get()}</Alert>
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
