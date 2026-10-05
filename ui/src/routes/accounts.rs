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

use std::sync::Arc;

use aurora_leptos::components::{
    Alert, Button, ConfirmDialog, Loading, Modal, PageHeader, PasswordInput, Pill, Select, Table,
    TextInput,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use crate::auth::{client_for, use_auth};
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

    let submit_create = move || {
        let Some(conn) = auth.connection() else {
            return;
        };
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

    // Disable asks first (ConfirmDialog): the user can no longer sign in.
    let disable_for = RwSignal::new(Option::<AccountRow>::None);
    let disable_open = RwSignal::new(false);
    Effect::new(move |_| disable_open.set(disable_for.get().is_some()));
    let disable = move || {
        let Some(target) = disable_for.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client.disable_account(&target.id, None).await;
            }
            busy.set(false);
            disable_for.set(None);
            refresh.update(|n| *n += 1);
        });
    };

    let do_reset = move || {
        let Some(target) = reset_for.get_untracked() else {
            return;
        };
        let pass = new_password.get_untracked();
        if pass.is_empty() {
            return;
        }
        let Some(conn) = auth.connection() else {
            return;
        };
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

    let reset_footer: ChildrenFn = Arc::new(move || {
        view! {
            <Button variant="default" on_click=Callback::new(move |_| reset_for.set(None))>"Cancel"</Button>
            <Button loading=busy on_click=Callback::new(move |_| do_reset())>"Reset password"</Button>
        }
        .into_any()
    });

    view! {
        <div class="app-page app-narrow">
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
                    <Alert color=token::GOLD>"You need admin access to manage accounts."</Alert>
                }
            >
                <form
                    class="app-panel app-col"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        submit_create();
                    }
                >
                    <div class="app-name">"Create account"</div>
                    <div class="app-fieldrow">
                        <div class="app-grow">
                            <TextInput label="Username" value=username autocomplete="off" />
                        </div>
                        <div class="app-grow">
                            <PasswordInput label="Initial password" value=password autocomplete="new-password" />
                        </div>
                        <div class="app-w-role">
                            <Select
                                label="Role"
                                options=vec!["read".to_string(), "write".to_string(), "admin".to_string()]
                                value=role
                            />
                        </div>
                        <Button button_type="submit" loading=busy>"Create"</Button>
                    </div>
                    <Show when=move || !create_error.get().is_empty()>
                        <Alert color=token::BAD>{move || create_error.get()}</Alert>
                    </Show>
                </form>
            </Show>

            // List
            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading accounts…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <span class="app-hint">"No local accounts yet."</span> }
                >
                    <div class="app-panel app-panel--flush">
                        <Table label="Local accounts">
                            <thead>
                                <tr>
                                    <th>"Username"</th>
                                    <th>"Role"</th>
                                    <th>"Status"</th>
                                    <th class="app-w-actions"><span class="cl-sr-only">"Actions"</span></th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || items.get()
                                    key=|a| (a.id.clone(), a.status.clone())
                                    children=move |a| {
                                        let active = a.status == "active";
                                        let for_reset = a.clone();
                                        let for_disable = a.clone();
                                        view! {
                                            <tr>
                                                <td class="app-text app-strong">{a.username.clone()}</td>
                                                <td class="app-mono app-small">{a.role.clone()}</td>
                                                <td>
                                                    <Pill color=if active { token::OK } else { token::MUTED }>
                                                        {a.status.clone()}
                                                    </Pill>
                                                </td>
                                                <td class="app-right">
                                                    <Show when=move || auth.can_admin()>
                                                        {
                                                            let for_reset = for_reset.clone();
                                                            let for_disable = for_disable.clone();
                                                            view! {
                                                                <span class="app-row app-row--tight app-row--end">
                                                                    <Button
                                                                        variant="subtle"
                                                                        size="xs"
                                                                        on_click=Callback::new(move |_| reset_for.set(Some(for_reset.clone())))
                                                                    >
                                                                        "Reset password"
                                                                    </Button>
                                                                    <Button
                                                                        variant="subtle"
                                                                        size="xs"
                                                                        bad=true
                                                                        disabled=Signal::derive(move || !active || busy.get())
                                                                        on_click=Callback::new(move |_| disable_for.set(Some(for_disable.clone())))
                                                                    >
                                                                        "Disable"
                                                                    </Button>
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
                        </Table>
                    </div>
                </Show>
            </Show>

            // Reset-password modal
            <Show when=move || reset_for.get().is_some()>
                <Modal
                    open=reset_open
                    title=reset_for.get_untracked().map(|a| format!("Reset password — {}", a.username)).unwrap_or_default()
                    footer=reset_footer.clone()
                    locked=busy
                    on_close=Callback::new(move |_| reset_for.set(None))
                >
                    <PasswordInput label="New password" value=new_password autocomplete="new-password" />
                </Modal>
            </Show>

            // Disable confirm
            <ConfirmDialog
                open=disable_open
                title="Disable account?"
                confirm_label="Disable"
                busy=busy
                on_confirm=Callback::new(move |_| disable())
                on_cancel=Callback::new(move |_| disable_for.set(None))
            >
                <span class="app-text app-fg2">
                    {move || disable_for.get().map(|a| format!(
                        "Disable {}? They can no longer sign in to this tenant.",
                        a.username
                    )).unwrap_or_default()}
                </span>
            </ConfirmDialog>
        </div>
    }
}
