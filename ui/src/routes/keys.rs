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

use aurora_leptos::components::{Alert, CopyButton, Empty, Loading, Modal, PageHeader, Select, TextInput};
use aurora_leptos::tokens::token;
use leptos::prelude::*;

use cloacina_api_types::{KeyInfo, KeyRole};

use crate::auth::{client_for, use_auth};
use crate::components::TagPill;
use crate::data::poll_resource;

const MONO: &str = "'IBM Plex Mono', monospace";

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

    let on_create = move |_| {
        let Some(conn) = auth.connection() else { return };
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

    let on_revoke = move |_| {
        let Some(target) = revoke_target.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else { return };
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

    view! {
        <div style:display="flex" style:flex-direction="column" style:gap="14px">
            <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start">
                <PageHeader
                    title="API Keys"
                    sub="Tenant-scoped keys for the SDK, CLI, and agents. Shown once at creation."
                />
                <Show when=move || auth.can_admin()>
                    <button class="cl-btn cl-btn--filled" on:click=move |_| create_open.set(true)>
                        "+ Create key"
                    </button>
                </Show>
            </div>

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading keys…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <Empty message="No API keys for this tenant yet." /> }
                >
                    <div style:display="flex" style:flex-direction="column" style:gap="9px">
                        <For
                            each=move || items.get()
                            key=|k| (k.id.clone(), k.revoked)
                            children=move |k| {
                                let revoked = k.revoked;
                                let for_revoke = k.clone();
                                view! {
                                    <div
                                        role="row"
                                        aria-label=k.name.clone()
                                        style:background="var(--panel-2)"
                                        style:border="1px solid var(--border)"
                                        style:border-radius="10px"
                                        style:padding="12px 15px"
                                        style:display="flex"
                                        style:justify-content="space-between"
                                        style:align-items="center"
                                    >
                                        <div style:display="flex" style:gap="12px" style:align-items="center" style:min-width="0">
                                            <div
                                                style:width="30px"
                                                style:height="30px"
                                                style:border-radius="8px"
                                                style:background="var(--panel)"
                                                style:border="1px solid var(--border)"
                                                style:display="flex"
                                                style:align-items="center"
                                                style:justify-content="center"
                                                style:flex="none"
                                                style:color=token::ICE
                                            >
                                                "🔑"
                                            </div>
                                            <div style:min-width="0">
                                                <div style:display="flex" style:gap="8px" style:align-items="center">
                                                    <span style:font-size="13px" style:font-weight="600" style:color="var(--fg)">
                                                        {k.name.clone()}
                                                    </span>
                                                    <Show when=move || revoked>
                                                        <TagPill color=token::MUTED>"revoked"</TagPill>
                                                    </Show>
                                                </div>
                                                <div style:font-family=MONO style:font-size="11px" style:color="var(--faint)" style:margin-top="2px">
                                                    {format!("clk_…{} · {}", &k.id[..4.min(k.id.len())], k.permissions)}
                                                </div>
                                            </div>
                                        </div>
                                        <div style:display="flex" style:gap="16px" style:align-items="center" style:flex="none">
                                            <span style:font-family=MONO style:font-size="10.5px" style:color="var(--faint)">
                                                {format!("created {}", k.created_at)}
                                            </span>
                                            <Show when=move || auth.can_admin() && !revoked>
                                                {
                                                    let target = for_revoke.clone();
                                                    view! {
                                                        <button
                                                            class="cl-btn cl-btn--subtle cl-btn--bad cl-btn--xs"
                                                            on:click={
                                                                let target = target.clone();
                                                                move |_| revoke_target.set(Some(target.clone()))
                                                            }
                                                        >
                                                            "Revoke"
                                                        </button>
                                                    }
                                                }
                                            </Show>
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                </Show>
            </Show>

            // Create modal
            <Show when=move || create_open.get()>
                <Modal open=create_open title="Create API key">
                    <div style:display="flex" style:flex-direction="column" style:gap="14px">
                        <TextInput label="Name" placeholder="ci-deploy" value=name />
                        <Select
                            label="Role"
                            options=vec!["read".to_string(), "write".to_string(), "admin".to_string()]
                            value=role
                        />
                        <Show when=move || !error.get().is_empty()>
                            <div style:color="var(--bad)" style:font-size="12.5px">{move || error.get()}</div>
                        </Show>
                        <div style:display="flex" style:justify-content="flex-end" style:gap="10px">
                            <button class="cl-btn cl-btn--default" on:click=move |_| create_open.set(false)>
                                "Cancel"
                            </button>
                            <button
                                class="cl-btn cl-btn--filled"
                                disabled=move || busy.get()
                                on:click=on_create
                            >
                                "Create"
                            </button>
                        </div>
                    </div>
                </Modal>
            </Show>

            // One-time plaintext reveal
            <Show when=move || plaintext.get().is_some()>
                <Modal open=plaintext_open title="Key created — shown once">
                    {move || plaintext.get().map(|(kname, key)| view! {
                        <div style:display="flex" style:flex-direction="column" style:gap="14px">
                            <Alert title=kname color="var(--ok)">
                                "Copy this key now. It cannot be shown again."
                            </Alert>
                            <code
                                class="cl-code"
                                style:word-break="break-all"
                                style:font-size="12px"
                            >
                                {key.clone()}
                            </code>
                            <div style:display="flex" style:justify-content="space-between">
                                <CopyButton value=key.clone() />
                                <button
                                    class="cl-btn cl-btn--filled"
                                    on:click=move |_| plaintext.set(None)
                                >
                                    "Done"
                                </button>
                            </div>
                        </div>
                    })}
                </Modal>
            </Show>

            // Revoke confirm
            <Show when=move || revoke_target.get().is_some()>
                <Modal open=revoke_open title="Revoke key?">
                    <div style:display="flex" style:flex-direction="column" style:gap="14px">
                        <span style:font-size="13px" style:color="var(--fg-2)">
                            {move || revoke_target.get().map(|k| format!(
                                "Revoke {}? Clients using it stop authenticating immediately.",
                                k.name
                            )).unwrap_or_default()}
                        </span>
                        <div style:display="flex" style:justify-content="flex-end" style:gap="10px">
                            <button class="cl-btn cl-btn--default" on:click=move |_| revoke_target.set(None)>
                                "Cancel"
                            </button>
                            <button
                                class="cl-btn cl-btn--filled cl-btn--bad"
                                disabled=move || busy.get()
                                on:click=on_revoke
                            >
                                "Revoke"
                            </button>
                        </div>
                    </div>
                </Modal>
            </Show>
        </div>
    }
}
