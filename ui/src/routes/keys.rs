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

//! API key management (Aurora Dark spec 12), parity port of `Keys.tsx`:
//! card rows, create + ONE-TIME plaintext reveal, revoke-confirm. Uses the
//! tenant-scoped key endpoints (T-0784 self-service).

use std::sync::Arc;

use aurora_leptos::components::{
    Button, ConfirmDialog, Empty, Loading, Modal, PageHeader, Pill, SecretReveal, Select, Table,
    TextInput,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use cloacina_api_types::{KeyInfo, KeyRole};

use crate::auth::{client_for, use_auth};
use crate::data::poll_resource;

fn role_of(s: &str) -> KeyRole {
    match s {
        "write" => KeyRole::Write,
        "admin" => KeyRole::Admin,
        _ => KeyRole::Read,
    }
}

#[component]
pub fn Keys() -> impl IntoView {
    let auth = use_auth();
    let list = poll_resource(|c| async move { c.list_tenant_keys(None).await });
    let items = Signal::derive(move || {
        list.get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let loading = Signal::derive(move || list.get().is_none());

    let create_open = RwSignal::new(false);
    let name = RwSignal::new(String::new());
    let role = RwSignal::new("read".to_string());
    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    // One-time plaintext reveal: (name, key).
    let plaintext = RwSignal::new(Option::<(String, String)>::None);
    let revoke_target = RwSignal::new(Option::<KeyInfo>::None);

    let on_create = move || {
        let Some(conn) = auth.connection() else {
            return;
        };
        let n = name.get_untracked().trim().to_string();
        if n.is_empty() {
            error.set("Name is required".into());
            return;
        }
        let r = role_of(&role.get_untracked());
        busy.set(true);
        error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .create_tenant_key(&n, r, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            match result {
                Ok(res) => {
                    create_open.set(false);
                    name.set(String::new());
                    role.set("read".into());
                    plaintext.set(Some((res.name, res.key)));
                }
                Err(e) => error.set(e),
            }
        });
    };

    let on_revoke = move || {
        let Some(target) = revoke_target.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client.revoke_tenant_key(&target.id, None).await;
            }
            busy.set(false);
            revoke_target.set(None);
        });
    };

    let plaintext_open = RwSignal::new(false);
    Effect::new(move |_| plaintext_open.set(plaintext.get().is_some()));
    let revoke_open = RwSignal::new(false);
    Effect::new(move |_| revoke_open.set(revoke_target.get().is_some()));

    let create_footer: ChildrenFn = Arc::new(move || {
        view! {
            <Button variant="default" on_click=Callback::new(move |_| create_open.set(false))>"Cancel"</Button>
            <Button loading=busy on_click=Callback::new(move |_| on_create())>"Create"</Button>
        }
        .into_any()
    });

    view! {
        <div class="app-page">
            <PageHeader
                title="API Keys"
                sub="Tenant-scoped keys for the SDK, CLI, and agents. Shown once at creation."
                actions=Box::new(move || view! {
                    <Show when=move || auth.can_admin()>
                        <Button on_click=Callback::new(move |_| create_open.set(true))>"+ Create key"</Button>
                    </Show>
                }.into_any())
            />

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading keys…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <Empty message="No API keys for this tenant yet." /> }
                >
                    <div class="app-panel app-panel--flush">
                        <Table label="API keys">
                            <thead>
                                <tr>
                                    <th>"Key"</th>
                                    <th>"Created"</th>
                                    <th class="app-w-action"><span class="cl-sr-only">"Actions"</span></th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || items.get()
                                    key=|k| (k.id.clone(), k.revoked)
                                    children=move |k| {
                                        let revoked = k.revoked;
                                        let for_revoke = k.clone();
                                        view! {
                                            <tr aria-label=k.name.clone()>
                                                <td>
                                                    <div class="app-row">
                                                        <span class="app-keyicon" aria-hidden="true">"🔑"</span>
                                                        <div class="app-grow">
                                                            <div class="app-row app-row--tight">
                                                                <span class="app-name">{k.name.clone()}</span>
                                                                <Show when=move || revoked>
                                                                    <Pill color=token::MUTED>"revoked"</Pill>
                                                                </Show>
                                                            </div>
                                                            <div class="app-meta app-meta--sm">
                                                                {format!("clk_…{} · {}", &k.id[..4.min(k.id.len())], k.permissions)}
                                                            </div>
                                                        </div>
                                                    </div>
                                                </td>
                                                <td class="app-meta">{format!("created {}", k.created_at)}</td>
                                                <td class="app-right">
                                                    <Show when=move || auth.can_admin() && !revoked>
                                                        {
                                                            let target = for_revoke.clone();
                                                            view! {
                                                                <Button
                                                                    variant="subtle"
                                                                    size="xs"
                                                                    bad=true
                                                                    on_click=Callback::new(move |_| revoke_target.set(Some(target.clone())))
                                                                >
                                                                    "Revoke"
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
            </Show>

            // Create modal
            <Modal open=create_open title="Create API key" footer=create_footer locked=busy>
                <div class="app-col app-col--loose">
                    <TextInput label="Name" placeholder="ci-deploy" value=name />
                    <Select
                        label="Role"
                        options=vec!["read".to_string(), "write".to_string(), "admin".to_string()]
                        value=role
                    />
                    <Show when=move || !error.get().is_empty()>
                        <div class="app-error" role="alert">{move || error.get()}</div>
                    </Show>
                </div>
            </Modal>

            // One-time plaintext reveal
            <Modal
                open=plaintext_open
                title="Key created — shown once"
                close_on_scrim=false
                on_close=Callback::new(move |_| plaintext.set(None))
            >
                {move || plaintext.get().map(|(kname, key)| view! {
                    <SecretReveal
                        secret=key
                        label=kname
                        warning="Copy this now — you won't see it again"
                        done_label="Done"
                        on_done=Callback::new(move |_| plaintext.set(None))
                    />
                })}
            </Modal>

            // Revoke confirm
            <ConfirmDialog
                open=revoke_open
                title="Revoke key?"
                confirm_label="Revoke"
                busy=busy
                on_confirm=Callback::new(move |_| on_revoke())
                on_cancel=Callback::new(move |_| revoke_target.set(None))
            >
                <span class="app-text app-fg2">
                    {move || revoke_target.get().map(|k| format!(
                        "Revoke {}? Clients using it stop authenticating immediately.",
                        k.name
                    )).unwrap_or_default()}
                </span>
            </ConfirmDialog>
        </div>
    }
}
