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

//! Syncs (COLLIERY-I-0635 / CLOACINA-T-1005): every sync by name, with its
//! source, destination, schedule and last pass, and run, edit and delete.
//! A sync is an instance of the built-in `sync` workflow; this page never
//! makes the user name the workflow.

use aurora_leptos::components::{
    Button, ConfirmDialog, Dot, Empty, IconPlay, Loading, PageHeader, Pill, StatusBadge, Table,
    TableRow,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::{SyncSide, SyncSummary};

use crate::auth::{client_for, use_auth};
use crate::data::poll_resource;
use crate::util::ago;

/// "1 dead-letter", "3 dead-letters".
pub fn dead_letters_text(n: i64) -> String {
    if n == 1 {
        "1 dead-letter".into()
    } else {
        format!("{n} dead-letters")
    }
}

/// A side as one line: its definition, or its connector.
pub fn side_text(side: &SyncSide) -> String {
    side.definition
        .clone()
        .or_else(|| side.provider.clone())
        .unwrap_or_else(|| "—".into())
}

#[component]
pub fn Syncs() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let refresh = RwSignal::new(0u32);

    let list = poll_resource(move |c| {
        refresh.track();
        async move { c.list_syncs(None).await }
    });
    let items = Signal::derive(move || {
        list.get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let loading = Signal::derive(move || list.get().is_none());
    let error = RwSignal::new(String::new());

    let running = RwSignal::new(false);
    let run_now = move |name: String| {
        let Some(conn) = auth.connection() else {
            return;
        };
        running.set(true);
        error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                client_for(&conn)?
                    .run_sync(&name, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            running.set(false);
            match result {
                Ok(_) => refresh.update(|x| *x += 1),
                Err(e) => error.set(e),
            }
        });
    };

    // Delete asks first: the sync's cursor and dead-letters stay, but it
    // stops running.
    let delete_for = RwSignal::new(Option::<String>::None);
    let delete_open = RwSignal::new(false);
    let busy = RwSignal::new(false);
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
            let result = async {
                client_for(&conn)?
                    .delete_instance("sync", &name, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            delete_for.set(None);
            if let Err(e) = result {
                error.set(e);
            }
            refresh.update(|x| *x += 1);
        });
    };

    let row = move |s: SyncSummary| {
        let name = s.name.clone();
        let to_detail = name.clone();
        let to_edit = name.clone();
        let to_run = name.clone();
        let to_delete = name.clone();
        let pass = s.last_pass.clone();
        view! {
            <TableRow on_click=Callback::new(move |_| {
                navigate.with_value(|n| n(
                    &format!("/syncs/{}", urlencoding::encode(&to_detail)),
                    Default::default(),
                ))
            })>
                <td>
                    <span class="app-name app-block app-ellipsis">{name.clone()}</span>
                    <span class="app-meta app-block app-ellipsis">{s.stream.clone()}</span>
                </td>
                <td class="app-meta app-meta--md app-ellipsis">
                    {format!("{} → {}", side_text(&s.source), side_text(&s.destination))}
                </td>
                <td class="app-meta app-meta--md">
                    <span class="app-row app-row--tight">
                        <Dot color=if s.enabled { token::OK } else { token::FAINT } size=7 />
                        {s.cron.clone().unwrap_or_else(|| "by hand".into())}
                    </span>
                </td>
                <td>
                    {match pass {
                        Some(p) => view! {
                            <span class="app-row app-row--tight">
                                <StatusBadge status=p.status.clone() />
                                <span class="app-meta app-meta--sm">{ago(Some(&p.started_at))}</span>
                            </span>
                        }.into_any(),
                        None => view! { <span class="app-faint app-small">"never run"</span> }.into_any(),
                    }}
                </td>
                <td>
                    <span class="app-row app-row--tight">
                        {s.schema_broken.then(|| view! { <Pill color=token::GOLD>"schema drift"</Pill> })}
                        {(s.dead_letters > 0).then(|| view! {
                            <Pill color=token::BAD>{dead_letters_text(s.dead_letters)}</Pill>
                        })}
                    </span>
                </td>
                <td>
                    <Show when=move || auth.can_write()>
                        {
                            let run = to_run.clone();
                            let edit = to_edit.clone();
                            let del = to_delete.clone();
                            view! {
                                <span class="app-row app-row--tight">
                                    <Button
                                        variant="subtle"
                                        size="xs"
                                        aria_label="Run sync"
                                        title="Run a pass now"
                                        disabled=running
                                        stop_propagation=true
                                        on_click=Callback::new(move |_| run_now(run.clone()))
                                    >
                                        <span class="app-hue app-ice"><IconPlay size=16 /></span>
                                    </Button>
                                    <Button
                                        variant="subtle"
                                        size="xs"
                                        stop_propagation=true
                                        href=format!("/syncs/{}/edit", urlencoding::encode(&edit))
                                    >
                                        "Edit"
                                    </Button>
                                    <Button
                                        variant="subtle"
                                        size="xs"
                                        bad=true
                                        stop_propagation=true
                                        on_click=Callback::new(move |_| delete_for.set(Some(del.clone())))
                                    >
                                        "Delete"
                                    </Button>
                                </span>
                            }
                        }
                    </Show>
                </td>
            </TableRow>
        }
    };

    let widths = || {
        ["16%", "20%", "11%", "14%", "19%", "20%"]
            .iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>()
    };

    view! {
        <div class="app-page">
            <PageHeader
                title="Syncs"
                sub="Records moved from a source to a destination, segment by segment, with a cursor that moves only after the write."
                actions=Box::new(move || view! {
                    <Show when=move || auth.can_write()>
                        <Button href="/syncs/new">"+ New sync"</Button>
                    </Show>
                }.into_any())
            />

            <Show when=move || !error.get().is_empty()>
                <span class="app-error" role="alert">{move || error.get()}</span>
            </Show>

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading syncs…" /> }
            >
                <Show
                    when=move || !items.get().is_empty()
                    fallback=|| view! {
                        <Empty message="No syncs yet." hint="A sync copies a stream from a source to a destination." href="/syncs/new" link="New sync" />
                    }
                >
                    <div class="app-panel app-panel--flush">
                        <Table fixed=true widths=widths() label="Syncs">
                            <thead>
                                <tr>
                                    <th>"Sync"</th>
                                    <th>"Source → destination"</th>
                                    <th>"Schedule"</th>
                                    <th>"Last pass"</th>
                                    <th>"Attention"</th>
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || items.get()
                                    key=|s| (s.name.clone(), s.last_pass.as_ref().map(|p| (p.pass_id.clone(), p.status.clone())), s.dead_letters, s.schema_broken, s.enabled)
                                    children=row
                                />
                            </tbody>
                        </Table>
                    </div>
                </Show>
            </Show>

            <ConfirmDialog
                open=delete_open
                title="Delete sync?"
                confirm_label="Delete"
                busy=busy
                on_confirm=Callback::new(move |_| delete())
                on_cancel=Callback::new(move |_| delete_for.set(None))
            >
                <span class="app-text app-fg2">
                    {move || delete_for.get().map(|n| format!(
                        "Delete {n}? It stops running. Its cursor and dead-letters stay, kept under its name."
                    )).unwrap_or_default()}
                </span>
            </ConfirmDialog>
        </div>
    }
}
