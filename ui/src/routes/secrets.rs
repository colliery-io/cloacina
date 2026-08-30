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

//! Tenant secrets (CLOACI-I-0133 / T-0862), parity port of `Secrets.tsx`:
//! metadata list + create / rotate / delete. Value inputs are WRITE-ONLY —
//! never populated from a GET (reads carry no values), and rotate seeds the
//! KNOWN field names with empty values.

use std::collections::BTreeMap;

use aurora_leptos::components::{Alert, Loading, Modal, PageHeader, PasswordInput, TextInput};
use leptos::prelude::*;

use cloacina_api_types::{CreateSecretRequest, RotateSecretRequest, SecretMetadataResponse};

use crate::auth::{client_for, use_auth};
use crate::data::poll_resource;

/// One editable field row: (key, value) signals.
type FieldRow = (RwSignal<String>, RwSignal<String>);

fn rows_to_fields(rows: &[FieldRow]) -> BTreeMap<String, String> {
    rows.iter()
        .filter_map(|(k, v)| {
            let key = k.get_untracked().trim().to_string();
            if key.is_empty() {
                None
            } else {
                Some((key, v.get_untracked()))
            }
        })
        .collect()
}

#[component]
pub fn Secrets() -> impl IntoView {
    let auth = use_auth();
    let refresh = RwSignal::new(0u32);

    let list = poll_resource(move |c| {
        refresh.get();
        async move { c.list_secrets(None).await }
    });
    let items = Signal::derive(move || {
        list.get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let loading = Signal::derive(move || list.get().is_none());

    // Create form state.
    let name = RwSignal::new(String::new());
    let rows = RwSignal::new(vec![(RwSignal::new(String::new()), RwSignal::new(String::new()))]);
    let create_error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    // Rotate modal state.
    let rotate_for = RwSignal::new(Option::<SecretMetadataResponse>::None);
    let rotate_rows = RwSignal::new(Vec::<FieldRow>::new());
    let rotate_open = RwSignal::new(false);
    Effect::new(move |_| rotate_open.set(rotate_for.get().is_some()));

    let submit_create = move |_| {
        let Some(conn) = auth.connection() else { return };
        let n = name.get_untracked().trim().to_string();
        let fields = rows_to_fields(&rows.get_untracked());
        if n.is_empty() || fields.is_empty() {
            create_error.set("A name and at least one field are required".into());
            return;
        }
        busy.set(true);
        create_error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let client = client_for(&conn)?;
                client
                    .create_secret(&CreateSecretRequest { name: n, fields }, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            match result {
                Ok(_) => {
                    name.set(String::new());
                    rows.set(vec![(RwSignal::new(String::new()), RwSignal::new(String::new()))]);
                    refresh.update(|x| *x += 1);
                }
                Err(e) => create_error.set(e),
            }
        });
    };

    let open_rotate = move |s: SecretMetadataResponse| {
        rotate_rows.set(
            s.field_names
                .iter()
                .map(|k| (RwSignal::new(k.clone()), RwSignal::new(String::new())))
                .collect(),
        );
        rotate_for.set(Some(s));
    };

    let submit_rotate = move |_| {
        let Some(target) = rotate_for.get_untracked() else {
            return;
        };
        let fields = rows_to_fields(&rotate_rows.get_untracked());
        if fields.is_empty() {
            return;
        }
        let Some(conn) = auth.connection() else { return };
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client
                    .rotate_secret(&target.name, &RotateSecretRequest { fields }, None)
                    .await;
            }
            busy.set(false);
            rotate_for.set(None);
            rotate_rows.set(Vec::new());
            refresh.update(|x| *x += 1);
        });
    };

    let delete = move |name: String| {
        let Some(conn) = auth.connection() else { return };
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client.delete_secret(&name, None).await;
            }
            busy.set(false);
            refresh.update(|x| *x += 1);
        });
    };

    let tenant = move || auth.connection().map(|c| c.tenant).unwrap_or_default();

    view! {
        <div style:max-width="820px" style:display="flex" style:flex-direction="column" style:gap="14px">
            <PageHeader
                title="Secrets"
                sub=format!(
                    "Encrypted, named-field credentials for tenant {}. Values are write-only — never shown after creation; rotation takes effect on the next fire.",
                    tenant()
                )
            />

            // Create — admin only.
            <Show
                when=move || auth.can_admin()
                fallback=|| view! {
                    <Alert color="var(--gold)">"You need admin access to manage secrets."</Alert>
                }
            >
                <div
                    style:background="var(--sidebar)"
                    style:border="1px solid var(--border)"
                    style:border-radius="12px"
                    style:padding="16px"
                    style:display="flex"
                    style:flex-direction="column"
                    style:gap="10px"
                >
                    <div style:font-size="13px" style:font-weight="600" style:color="var(--fg)">
                        "Create secret"
                    </div>
                    <TextInput label="Name" placeholder="db_prod" value=name />
                    <div style:font-size="12px" style:color="var(--muted)">"Fields"</div>
                    <For
                        each={move || rows.get().into_iter().enumerate().collect::<Vec<_>>()}
                        key=|(i, _)| *i
                        children=move |(i, (k, v))| {
                            view! {
                                <div style:display="flex" style:gap="8px" style:align-items="flex-end">
                                    <div style:flex="1">
                                        <TextInput placeholder="password" value=k />
                                    </div>
                                    <div style:flex="1">
                                        <PasswordInput placeholder="value (write-only)" value=v />
                                    </div>
                                    <button
                                        class="cl-btn cl-btn--subtle cl-btn--bad cl-btn--xs"
                                        aria-label="remove field"
                                        on:click=move |_| {
                                            rows.update(|r| {
                                                if r.len() > 1 {
                                                    r.remove(i);
                                                }
                                            })
                                        }
                                    >
                                        "✕"
                                    </button>
                                </div>
                            }
                        }
                    />
                    <div style:display="flex" style:justify-content="space-between">
                        <button
                            class="cl-btn cl-btn--subtle cl-btn--xs"
                            on:click=move |_| rows.update(|r| {
                                r.push((RwSignal::new(String::new()), RwSignal::new(String::new())))
                            })
                        >
                            "+ Add field"
                        </button>
                        <button
                            class="cl-btn cl-btn--filled"
                            disabled=move || busy.get()
                            on:click=submit_create
                        >
                            "Create"
                        </button>
                    </div>
                    <Show when=move || !create_error.get().is_empty()>
                        <Alert color="var(--bad)">{move || create_error.get()}</Alert>
                    </Show>
                </div>
            </Show>

            // List — metadata only.
            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading secrets…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! {
                        <span style:color="var(--muted)" style:font-size="13px">"No secrets yet."</span>
                    }
                >
                    <table class="cl-table">
                        <thead>
                            <tr>
                                <th>"Name"</th>
                                <th>"Fields"</th>
                                <th>"Updated"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || items.get()
                                key=|s| (s.id.clone(), s.updated_at.clone())
                                children=move |s| {
                                    let for_rotate = s.clone();
                                    let name_for_delete = s.name.clone();
                                    view! {
                                        <tr>
                                            <td style:font-weight="500">{s.name.clone()}</td>
                                            <td style:color="var(--muted)" style:font-size="12.5px">
                                                {s.field_names.join(", ")}
                                            </td>
                                            <td style:color="var(--muted)" style:font-size="12.5px">
                                                {s.updated_at.clone()}
                                            </td>
                                            <td>
                                                <Show when=move || auth.can_admin()>
                                                    {
                                                        let for_rotate = for_rotate.clone();
                                                        let name = name_for_delete.clone();
                                                        view! {
                                                            <span style:display="inline-flex" style:gap="6px" style:justify-content="flex-end">
                                                                <button
                                                                    class="cl-btn cl-btn--subtle cl-btn--xs"
                                                                    on:click={
                                                                        let s = for_rotate.clone();
                                                                        move |_| open_rotate(s.clone())
                                                                    }
                                                                >
                                                                    "Rotate"
                                                                </button>
                                                                <button
                                                                    class="cl-btn cl-btn--subtle cl-btn--bad cl-btn--xs"
                                                                    disabled=move || busy.get()
                                                                    on:click={
                                                                        let name = name.clone();
                                                                        move |_| delete(name.clone())
                                                                    }
                                                                >
                                                                    "Delete"
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

            // Rotate modal — seeded with KNOWN names, EMPTY values.
            <Show when=move || rotate_for.get().is_some()>
                <Modal
                    open=rotate_open
                    title=rotate_for.get_untracked().map(|s| format!("Rotate {}", s.name)).unwrap_or_default()
                >
                    <div style:display="flex" style:flex-direction="column" style:gap="12px">
                        <For
                            each={move || rotate_rows.get().into_iter().enumerate().collect::<Vec<_>>()}
                            key=|(i, _)| *i
                            children=|(_, (k, v))| {
                                view! {
                                    <div style:display="flex" style:gap="8px" style:align-items="flex-end">
                                        <div style:flex="1">
                                            <TextInput value=k />
                                        </div>
                                        <div style:flex="1">
                                            <PasswordInput placeholder="new value" value=v />
                                        </div>
                                    </div>
                                }
                            }
                        />
                        <button
                            class="cl-btn cl-btn--filled"
                            disabled=move || busy.get()
                            on:click=submit_rotate
                        >
                            "Rotate"
                        </button>
                    </div>
                </Modal>
            </Show>
        </div>
    }
}
