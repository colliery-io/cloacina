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
use std::sync::Arc;

use aurora_leptos::components::{
    Alert, Button, ConfirmDialog, Loading, Modal, PageHeader, PasswordInput, Table, TextInput,
};
use aurora_leptos::tokens::token;
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
    let rows = RwSignal::new(vec![(
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
    )]);
    let create_error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    // Rotate modal state.
    let rotate_for = RwSignal::new(Option::<SecretMetadataResponse>::None);
    let rotate_rows = RwSignal::new(Vec::<FieldRow>::new());
    let rotate_open = RwSignal::new(false);
    Effect::new(move |_| rotate_open.set(rotate_for.get().is_some()));

    let submit_create = move || {
        let Some(conn) = auth.connection() else {
            return;
        };
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
                    rows.set(vec![(
                        RwSignal::new(String::new()),
                        RwSignal::new(String::new()),
                    )]);
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

    let submit_rotate = move || {
        let Some(target) = rotate_for.get_untracked() else {
            return;
        };
        let fields = rows_to_fields(&rotate_rows.get_untracked());
        if fields.is_empty() {
            return;
        }
        let Some(conn) = auth.connection() else {
            return;
        };
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

    // Delete asks first (ConfirmDialog): a secret delete breaks every
    // workflow that reads it.
    let delete_for = RwSignal::new(Option::<String>::None);
    let delete_open = RwSignal::new(false);
    Effect::new(move |_| delete_open.set(delete_for.get().is_some()));
    let delete = move || {
        let Some(name) = delete_for.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        busy.set(true);
        leptos::task::spawn_local(async move {
            if let Ok(client) = client_for(&conn) {
                let _ = client.delete_secret(&name, None).await;
            }
            busy.set(false);
            delete_for.set(None);
            refresh.update(|x| *x += 1);
        });
    };

    let tenant = move || auth.connection().map(|c| c.tenant).unwrap_or_default();

    let rotate_footer: ChildrenFn = Arc::new(move || {
        view! {
            <Button variant="default" on_click=Callback::new(move |_| rotate_for.set(None))>"Cancel"</Button>
            <Button loading=busy on_click=Callback::new(move |_| submit_rotate())>"Rotate"</Button>
        }
        .into_any()
    });

    view! {
        <div class="app-page app-narrow">
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
                    <Alert color=token::GOLD>"You need admin access to manage secrets."</Alert>
                }
            >
                <form
                    class="app-panel app-col"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        submit_create();
                    }
                >
                    <div class="app-name">"Create secret"</div>
                    <TextInput label="Name" placeholder="db_prod" value=name />
                    <div class="app-small app-muted">"Fields"</div>
                    <For
                        each={move || rows.get().into_iter().enumerate().collect::<Vec<_>>()}
                        key=|(i, _)| *i
                        children=move |(i, (k, v))| {
                            view! {
                                <div class="app-fieldrow">
                                    <div class="app-grow">
                                        <TextInput placeholder="password" value=k />
                                    </div>
                                    <div class="app-grow">
                                        <PasswordInput placeholder="value (write-only)" value=v />
                                    </div>
                                    <Button
                                        variant="subtle"
                                        size="xs"
                                        bad=true
                                        button_type="button"
                                        aria_label="remove field"
                                        on_click=Callback::new(move |_| {
                                            rows.update(|r| {
                                                if r.len() > 1 {
                                                    r.remove(i);
                                                }
                                            })
                                        })
                                    >
                                        "✕"
                                    </Button>
                                </div>
                            }
                        }
                    />
                    <div class="app-row app-row--between">
                        <Button
                            variant="subtle"
                            size="xs"
                            button_type="button"
                            on_click=Callback::new(move |_| rows.update(|r| {
                                r.push((RwSignal::new(String::new()), RwSignal::new(String::new())))
                            }))
                        >
                            "+ Add field"
                        </Button>
                        <Button button_type="submit" loading=busy>"Create"</Button>
                    </div>
                    <Show when=move || !create_error.get().is_empty()>
                        <Alert color=token::BAD>{move || create_error.get()}</Alert>
                    </Show>
                </form>
            </Show>

            // List — metadata only.
            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading secrets…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! { <span class="app-hint">"No secrets yet."</span> }
                >
                    <div class="app-panel app-panel--flush">
                        <Table label="Secrets">
                            <thead>
                                <tr>
                                    <th>"Name"</th>
                                    <th>"Fields"</th>
                                    <th>"Updated"</th>
                                    <th class="app-w-wide-action"><span class="cl-sr-only">"Actions"</span></th>
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
                                                <td class="app-text app-strong">{s.name.clone()}</td>
                                                <td class="app-small app-muted">{s.field_names.join(", ")}</td>
                                                <td class="app-small app-muted">{s.updated_at.clone()}</td>
                                                <td class="app-right">
                                                    <Show when=move || auth.can_admin()>
                                                        {
                                                            let for_rotate = for_rotate.clone();
                                                            let name = name_for_delete.clone();
                                                            view! {
                                                                <span class="app-row app-row--tight app-row--end">
                                                                    <Button
                                                                        variant="subtle"
                                                                        size="xs"
                                                                        on_click=Callback::new(move |_| open_rotate(for_rotate.clone()))
                                                                    >
                                                                        "Rotate"
                                                                    </Button>
                                                                    <Button
                                                                        variant="subtle"
                                                                        size="xs"
                                                                        bad=true
                                                                        disabled=busy
                                                                        on_click=Callback::new(move |_| delete_for.set(Some(name.clone())))
                                                                    >
                                                                        "Delete"
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

            // Rotate modal — seeded with KNOWN names, EMPTY values.
            <Show when=move || rotate_for.get().is_some()>
                <Modal
                    open=rotate_open
                    title=rotate_for.get_untracked().map(|s| format!("Rotate {}", s.name)).unwrap_or_default()
                    footer=rotate_footer.clone()
                    locked=busy
                    on_close=Callback::new(move |_| rotate_for.set(None))
                >
                    <div class="app-col">
                        <For
                            each={move || rotate_rows.get().into_iter().enumerate().collect::<Vec<_>>()}
                            key=|(i, _)| *i
                            children=|(_, (k, v))| {
                                view! {
                                    <div class="app-fieldrow">
                                        <div class="app-grow">
                                            <TextInput value=k />
                                        </div>
                                        <div class="app-grow">
                                            <PasswordInput placeholder="new value" value=v />
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                </Modal>
            </Show>

            // Delete confirm
            <ConfirmDialog
                open=delete_open
                title="Delete secret?"
                confirm_label="Delete"
                busy=busy
                on_confirm=Callback::new(move |_| delete())
                on_cancel=Callback::new(move |_| delete_for.set(None))
            >
                <span class="app-text app-fg2">
                    {move || delete_for.get().map(|n| format!(
                        "Delete {n}? Workflows that read it fail on their next fire."
                    )).unwrap_or_default()}
                </span>
            </ConfirmDialog>
        </div>
    }
}
