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

use aurora_leptos::components::{Alert, Button, Loading, PageHeader, StatTile};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use crate::auth::{client_for, use_auth};
use crate::data::poll_resource;

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

    let n = |v: Signal<i64>| Signal::derive(move || v.get().to_string());

    view! {
        <div class="app-page app-narrow">
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
                <div class="app-grid-3">
                    <StatTile label="Provisioned" value=n(desired) />
                    <StatTile label="Running" value=n(actual) color=token::ICE />
                    <StatTile label="Effective limit" value=n(limit) />
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
                view! { <div class="app-small app-muted">{line}</div> }
            })}

            <Show
                when=move || auth.can_admin()
                fallback=|| view! {
                    <Alert color=token::GOLD>
                        "You need admin access to provision or deprovision agents."
                    </Alert>
                }
            >
                <div class="app-panel app-col">
                    <div class="app-name">"Scale fleet"</div>
                    <div class="app-row">
                        <Button
                            disabled=Signal::derive(move || busy.get() || at_capacity.get())
                            on_click=Callback::new(move |_| scale("provision"))
                        >
                            "Provision +1"
                        </Button>
                        <Button
                            variant="default"
                            disabled=Signal::derive(move || busy.get() || desired.get() <= 0)
                            on_click=Callback::new(move |_| scale("deprovision"))
                        >
                            "Deprovision −1"
                        </Button>
                        <Show when=move || at_capacity.get()>
                            <span class="app-small app-muted">
                                {move || format!("At capacity ({}).", limit.get())}
                            </span>
                        </Show>
                    </div>
                    <Show when=move || !error.get().is_empty()>
                        <Alert color=token::BAD>{move || error.get()}</Alert>
                    </Show>
                </div>
            </Show>
        </div>
    }
}
