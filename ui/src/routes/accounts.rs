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

//! Tenant-admin local-account management (CLOACI-T-0798), parity port of
//! `Accounts.tsx`: create / list / disable / reset-password for the
//! connected tenant's self-managed accounts. Non-admin keys see the
//! explanatory alert (fail-closed gating).

use aurora_leptos::components::{Alert, Loading, Modal, PageHeader, PasswordInput, Select, TextInput};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use crate::auth::{client_for, use_auth};
use crate::components::TagPill;
use crate::data::poll_resource;

/// Account row decoded from the (Value-typed) accounts listing.
#[derive(Clone, PartialEq, serde::Deserialize)]
struct AccountRow {
    id: String,
    username: String,
    role: String,
    status: String,
}

#[component]
pub fn Accounts() -> impl IntoView {
    let auth = use_auth();
    let refresh = RwSignal::new(0u32);

    let list = poll_resource(move |c| {
        refresh.get();
        async move { c.list_accounts(None).await }
    });
    let items = Signal::derive(move || {
        list.get()
            .and_then(|r| r.ok())
            .and_then(|v| v.get("items").cloned())
            .and_then(|v| serde_json::from_value::<Vec<AccountRow>>(v).ok())
            .unwrap_or_default()
    });
    let loading = Signal::derive(move || list.get().is_none());

    let username = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let role = RwSignal::new("read".to_string());
    let create_error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    let reset_for = RwSignal::new(Option::<AccountRow>::None);
    let new_password = RwSignal::new(String::new());
    let reset_open = RwSignal::new(false);
    Effect::new(move |_| reset_open.set(reset_for.get().is_some()));

    let submit_create = move |_| {
        let Some(conn) = auth.connection() else { return };
        let user = username.get_untracked().trim().to_string();
        let pass = password.get_untracked();
        if user.is_empty() || pass.is_empty() {
            return;
        }
        let r = role.get_untracked();
        busy.set(true);
        create_error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .create_account(&user, &pass, &r, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            match result {
                Ok(_) => {
                    username.set(String::new());
                    password.set(String::new());
                    role.set("read".into());
                    refresh.update(|n| *n += 1);
                }
                Err(e) => create_error.set(e),
            }
        });
    };

    let disable = move |id: String| {
        let Some(conn) = auth.connection() else { return };
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client.disable_account(&id, None).await;
            }
            busy.set(false);
            refresh.update(|n| *n += 1);
        });
    };

    let do_reset = move |_| {
        let Some(target) = reset_for.get_untracked() else {
            return;
        };
        let pass = new_password.get_untracked();
        if pass.is_empty() {
            return;
        }
        let Some(conn) = auth.connection() else { return };
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client.reset_password(&target.id, &pass, None).await;
            }
            busy.set(false);
            reset_for.set(None);
            new_password.set(String::new());
        });
    };

    let tenant = move || auth.connection().map(|c| c.tenant).unwrap_or_default();

    view! {
        <div style:max-width="820px" style:display="flex" style:flex-direction="column" style:gap="14px">
            <PageHeader
                title="Local accounts"
                sub=format!(
                    "Self-managed username/password accounts for tenant {}. Users sign in at the connect screen with these credentials.",
                    tenant()
                )
            />

            // Create — admin only.
            <Show
                when=move || auth.can_admin()
                fallback=|| view! {
                    <Alert color="var(--gold)">"You need admin access to manage accounts."</Alert>
                }
            >
                <div
                    style:background="var(--sidebar)"
                    style:border="1px solid var(--border)"
                    style:border-radius="12px"
                    style:padding="16px"
                >
                    <div style:font-size="13px" style:font-weight="600" style:color="var(--fg)" style:margin-bottom="10px">
                        "Create account"
                    </div>
                    <div style:display="flex" style:gap="10px" style:align-items="flex-end">
                        <div style:flex="1">
                            <TextInput label="Username" value=username />
                        </div>
                        <div style:flex="1">
                            <PasswordInput label="Initial password" value=password />
                        </div>
                        <div style:width="120px">
                            <Select
                                label="Role"
                                options=vec!["read".to_string(), "write".to_string(), "admin".to_string()]
                                value=role
                            />
                        </div>
                        <button
                            class="cl-btn cl-btn--filled"
                            disabled=move || busy.get()
                            on:click=submit_create
                        >
                            "Create"
                        </button>
                    </div>
                    <Show when=move || !create_error.get().is_empty()>
                        <div style:margin-top="10px">
                            <Alert color="var(--bad)">{move || create_error.get()}</Alert>
                        </div>
                    </Show>
                </div>
            </Show>

            // List
            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading accounts…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! {
                        <span style:color="var(--muted)" style:font-size="13px">"No local accounts yet."</span>
                    }
                >
                    <table class="cl-table">
                        <thead>
                            <tr>
                                <th>"Username"</th>
                                <th>"Role"</th>
                                <th>"Status"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || items.get()
                                key=|a| (a.id.clone(), a.status.clone())
                                children=move |a| {
                                    let active = a.status == "active";
                                    let for_reset = a.clone();
                                    let id_for_disable = a.id.clone();
                                    view! {
                                        <tr>
                                            <td style:font-weight="500">{a.username.clone()}</td>
                                            <td>{a.role.clone()}</td>
                                            <td>
                                                <TagPill color=if active { token::OK } else { token::MUTED }>
                                                    {a.status.clone()}
                                                </TagPill>
                                            </td>
                                            <td>
                                                <Show when=move || auth.can_admin()>
                                                    {
                                                        let for_reset = for_reset.clone();
                                                        let id = id_for_disable.clone();
                                                        view! {
                                                            <span style:display="inline-flex" style:gap="6px" style:justify-content="flex-end">
                                                                <button
                                                                    class="cl-btn cl-btn--subtle cl-btn--xs"
                                                                    on:click={
                                                                        let for_reset = for_reset.clone();
                                                                        move |_| reset_for.set(Some(for_reset.clone()))
                                                                    }
                                                                >
                                                                    "Reset password"
                                                                </button>
                                                                <button
                                                                    class="cl-btn cl-btn--subtle cl-btn--bad cl-btn--xs"
                                                                    disabled=move || !active || busy.get()
                                                                    on:click={
                                                                        let id = id.clone();
                                                                        move |_| disable(id.clone())
                                                                    }
                                                                >
                                                                    "Disable"
                                                                </button>
                                                            </span>
                                                        }
                                                    }
                                                </Show>
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </Show>
            </Show>

            // Reset-password modal
            <Show when=move || reset_for.get().is_some()>
                <Modal
                    open=reset_open
                    title=reset_for.get_untracked().map(|a| format!("Reset password — {}", a.username)).unwrap_or_default()
                >
                    <div style:display="flex" style:flex-direction="column" style:gap="12px">
                        <PasswordInput label="New password" value=new_password />
                        <button
                            class="cl-btn cl-btn--filled"
                            disabled=move || busy.get()
                            on:click=do_reset
                        >
                            "Reset password"
                        </button>
                    </div>
                </Modal>
            </Show>
        </div>
    }
}
