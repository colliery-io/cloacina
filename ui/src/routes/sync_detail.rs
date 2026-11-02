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

//! One sync (COLLIERY-I-0635 / CLOACINA-T-1005): what it moves and when, its
//! passes (one entry per pass, its segments' executions inside), its stream's
//! cursor and captured schema with "Accept" on drift, and its dead-letters.
//! An execution's logs are its own page.

use aurora_leptos::components::{
    Alert, Button, DetailList, Empty, KeyValue, Loading, PageHeader, Panel, Pill, StatusBadge,
    TabItem, TabPanel, Table, TableRow, Tabs,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use cloacina_api_types::{SyncPageQuery, SyncPass};

use crate::auth::{client_for, use_auth};
use crate::data::poll_resource;
use crate::routes::syncs::{dead_letters_text, side_text};
use crate::util::{ago, format_duration, short_id};

#[component]
pub fn SyncDetail() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let params = use_params_map();
    let name = Signal::derive(move || params.read().get("name").unwrap_or_default());
    let refresh = RwSignal::new(0u32);

    let detail = poll_resource(move |c| {
        refresh.track();
        let name = name.get();
        async move { c.get_sync(&name, None).await }
    });
    let passes = poll_resource(move |c| {
        refresh.track();
        let name = name.get();
        async move {
            c.list_sync_passes(
                &name,
                &SyncPageQuery {
                    limit: Some(50),
                    offset: None,
                },
                None,
            )
            .await
        }
    });
    let streams = poll_resource(move |c| {
        refresh.track();
        let name = name.get();
        async move { c.list_sync_streams(&name, None).await }
    });
    let dead = poll_resource(move |c| {
        refresh.track();
        let name = name.get();
        async move {
            c.list_sync_dead_letters(
                &name,
                &SyncPageQuery {
                    limit: Some(50),
                    offset: None,
                },
                None,
            )
            .await
        }
    });

    let data = Signal::derive(move || detail.get().and_then(|r| r.ok()));
    let loading = Signal::derive(move || detail.get().is_none());
    let pass_items = Signal::derive(move || {
        passes
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let stream_items = Signal::derive(move || {
        streams
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let dead_items = Signal::derive(move || {
        dead.get()
            .and_then(|r| r.ok())
            .map(|r| (r.items, r.total))
            .unwrap_or_default()
    });

    let error = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    // Run or accept, then refresh.
    let act = move |accept: bool| {
        let Some(conn) = auth.connection() else {
            return;
        };
        let name = name.get_untracked();
        busy.set(true);
        error.set(String::new());
        leptos::task::spawn_local(async move {
            let result = async {
                let c = client_for(&conn)?;
                if accept {
                    c.accept_sync_schema(&name, None).await.map(|_| ())
                } else {
                    c.run_sync(&name, None).await.map(|_| ())
                }
                .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            match result {
                Ok(()) => refresh.update(|x| *x += 1),
                Err(e) => error.set(e),
            }
        });
    };

    // The pass whose segments are shown; the newest by default.
    let selected = RwSignal::new(Option::<String>::None);
    let shown = Signal::derive(move || {
        let items = pass_items.get();
        match selected.get() {
            Some(id) => items.into_iter().find(|p| p.pass_id == id),
            None => items.into_iter().next(),
        }
    });

    let tab = RwSignal::new("passes".to_string());

    let pass_row = move |p: SyncPass| {
        let id = p.pass_id.clone();
        let is_selected = {
            let id = id.clone();
            Signal::derive(move || shown.get().map(|s| s.pass_id == id).unwrap_or(false))
        };
        view! {
            <TableRow
                selected=is_selected
                on_click=Callback::new(move |_| selected.set(Some(id.clone())))
            >
                <td><StatusBadge status=p.status.clone() /></td>
                <td class="app-meta app-meta--sm">{ago(Some(&p.started_at))}</td>
                <td class="app-meta app-meta--sm">
                    {format_duration(Some(&p.started_at), p.completed_at.as_deref())}
                </td>
                <td class="app-meta app-meta--md">{p.segments}</td>
                <td class="app-meta app-meta--md app-ellipsis">
                    {p.cursor.clone().unwrap_or_else(|| "—".into())}
                </td>
            </TableRow>
        }
    };

    let passes_view = move || {
        view! {
            <Show
                when=move || !pass_items.get().is_empty()
                fallback=|| view! { <Empty message="No passes yet." hint="Run the sync, or wait for its schedule." /> }
            >
                <div class="app-panel app-panel--flush">
                    <Table fixed=true label="Passes">
                        <thead>
                            <tr>
                                <th>"Status"</th>
                                <th>"Started"</th>
                                <th>"Took"</th>
                                <th>"Segments"</th>
                                <th>"Cursor after"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || pass_items.get()
                                key=|p| (p.pass_id.clone(), p.status.clone(), p.segments)
                                children=pass_row
                            />
                        </tbody>
                    </Table>
                </div>
                {move || shown.get().map(|p| view! {
                    <Panel title=format!("Segments of the pass {}", short_id(&p.pass_id))>
                        <Table mono=true label="Segments">
                            <thead>
                                <tr>
                                    <th>"Execution"</th>
                                    <th>"Status"</th>
                                    <th>"Started"</th>
                                    <th>"Took"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {p.executions.clone().into_iter().map(|e| {
                                    let id = e.id.clone();
                                    let short = short_id(&e.id);
                                    let started = ago(Some(&e.started_at));
                                    let took = format_duration(Some(&e.started_at), e.completed_at.as_deref());
                                    let status = e.status.clone();
                                    view! {
                                        <TableRow on_click=Callback::new(move |_| navigate.with_value(|n| n(
                                            &format!("/executions/{id}"),
                                            Default::default(),
                                        )))>
                                            <td>{short}</td>
                                            <td><StatusBadge status=status /></td>
                                            <td>{started}</td>
                                            <td>{took}</td>
                                        </TableRow>
                                    }
                                }).collect_view()}
                            </tbody>
                        </Table>
                        <span class="app-hint">"Open a segment for its tasks and logs."</span>
                    </Panel>
                })}
            </Show>
        }
    };

    let streams_view = move || {
        view! {
            <For
                each=move || stream_items.get()
                key=|s| (s.stream.clone(), s.cursor.clone(), s.broken.clone(), s.fields.len())
                children=move |s| {
                    let broken = s.broken.clone();
                    view! {
                        <Panel title=s.stream.clone()>
                            {broken.map(|why| view! {
                                <Alert title="Schema drift" color=token::GOLD>
                                    <div class="app-col">
                                        <span class="app-prewrap app-block">{why}</span>
                                        <span class="app-hint app-block">
                                            "Records that do not fit go to the dead-letters. Accept the new shape to let the next write capture it."
                                        </span>
                                        <Show when=move || auth.can_write()>
                                            <div>
                                                <Button loading=busy on_click=Callback::new(move |_| act(true))>
                                                    "Accept new schema"
                                                </Button>
                                            </div>
                                        </Show>
                                    </div>
                                </Alert>
                            })}
                            <DetailList mono=true>
                                <KeyValue label="Cursor">{s.cursor.clone().unwrap_or_else(|| "—".into())}</KeyValue>
                                <KeyValue label="Moved">{ago(s.updated_at.as_deref())}</KeyValue>
                                <KeyValue label="Source ended">{if s.exhausted { "yes" } else { "not yet" }}</KeyValue>
                            </DetailList>
                            {if s.fields.is_empty() {
                                view! { <span class="app-hint">"No schema captured yet."</span> }.into_any()
                            } else {
                                view! {
                                    <Table mono=true label="Schema">
                                        <thead><tr><th>"Field"</th><th>"Type"</th><th>"Nullable"</th></tr></thead>
                                        <tbody>
                                            {s.fields.iter().map(|f| view! {
                                                <tr>
                                                    <td>{f.name.clone()}</td>
                                                    <td>{f.field_type.clone()}</td>
                                                    <td>{if f.nullable { "yes" } else { "no" }}</td>
                                                </tr>
                                            }).collect_view()}
                                        </tbody>
                                    </Table>
                                }.into_any()
                            }}
                        </Panel>
                    }
                }
            />
        }
    };

    let dead_view = move || {
        view! {
            <Show
                when=move || !dead_items.get().0.is_empty()
                fallback=|| view! { <Empty message="No dead-letters." hint="Records the sync could not map, fit or write land here, with the reason." /> }
            >
                <span class="app-hint">
                    {move || format!("The newest {} of {}.", dead_items.get().0.len(), dead_items.get().1)}
                </span>
                <div class="app-panel app-panel--flush">
                    <Table
                        fixed=true
                        widths={ ["12%", "44%", "44%"].iter().map(|w| w.to_string()).collect::<Vec<_>>() }
                        label="Dead-letters"
                    >
                        <thead>
                            <tr>
                                <th>"When"</th>
                                <th>"Reason"</th>
                                <th>"Record"</th>
                            </tr>
                        </thead>
                        <tbody>
                            <For
                                each=move || dead_items.get().0
                                key=|d| d.id.clone()
                                children=move |d| {
                                    let exec = d.execution_id.clone();
                                    view! {
                                        <TableRow on_click=Callback::new(move |_| {
                                            if let Some(id) = exec.clone() {
                                                navigate.with_value(|n| n(&format!("/executions/{id}"), Default::default()))
                                            }
                                        })>
                                            <td class="app-meta app-meta--sm">{ago(Some(&d.created_at))}</td>
                                            <td class="app-small app-prewrap">{d.reason.clone()}</td>
                                            <td class="app-meta app-meta--sm app-ellipsis" title=d.record.clone()>{d.record.clone()}</td>
                                        </TableRow>
                                    }
                                }
                            />
                        </tbody>
                    </Table>
                </div>
            </Show>
        }
    };

    view! {
        <div class="app-page">
            {move || view! {
                <PageHeader
                    title=name.get()
                    back_href="/syncs"
                    back_label="Syncs"
                    actions=Box::new(move || view! {
                        <Show when=move || auth.can_write() && data.get().is_some()>
                            <span class="app-row app-row--tight">
                                <Button
                                    variant="default"
                                    href=format!("/syncs/{}/edit", urlencoding::encode(&name.get()))
                                >
                                    "Edit"
                                </Button>
                                <Button loading=busy on_click=Callback::new(move |_| act(false))>
                                    "▸ Run now"
                                </Button>
                            </span>
                        </Show>
                    }.into_any())
                />
            }}

            <Show when=move || !error.get().is_empty()>
                <span class="app-error" role="alert">{move || error.get()}</span>
            </Show>

            <Show
                when=move || !loading.get()
                fallback=|| view! { <Loading label="Loading sync…" /> }
            >
                <Show
                    when=move || data.get().is_some()
                    fallback=|| view! { <Empty message="Sync not found." href="/syncs" link="All syncs" /> }
                >
                    {move || data.get().map(|d| {
                        let s = d.summary;
                        view! {
                            <div class="app-panel app-col">
                                <div class="app-row">
                                    <Pill color=token::ICE>{s.sync_mode.clone()}</Pill>
                                    <Pill color=token::VIOLET>{s.write_mode.clone()}</Pill>
                                    {s.schema_broken.then(|| view! { <Pill color=token::GOLD>"schema drift"</Pill> })}
                                    {(s.dead_letters > 0).then(|| view! {
                                        <Pill color=token::BAD>{dead_letters_text(s.dead_letters)}</Pill>
                                    })}
                                </div>
                                <DetailList mono=true>
                                    <KeyValue label="Source">{side_text(&s.source)}</KeyValue>
                                    <KeyValue label="Destination">{side_text(&s.destination)}</KeyValue>
                                    <KeyValue label="Stream">{s.stream.clone()}</KeyValue>
                                    <KeyValue label="Schedule">
                                        {s.cron.clone().map(|c| format!("{c} ({})", s.timezone.clone().unwrap_or_else(|| "UTC".into()))).unwrap_or_else(|| "by hand".into())}
                                    </KeyValue>
                                    {s.next_run_at.clone().map(|n| view! { <KeyValue label="Next run">{n}</KeyValue> })}
                                    {(s.sync_name != s.name).then(|| view! {
                                        <KeyValue label="Kept under">{s.sync_name.clone()}</KeyValue>
                                    })}
                                </DetailList>
                            </div>
                        }
                    })}

                    <Tabs
                        tabs=vec![
                            TabItem::new("passes", "Passes"),
                            TabItem::new("streams", "Stream and schema"),
                            TabItem::new("dead", "Dead-letters"),
                        ]
                        value=tab
                        label="Sync views"
                    >
                        <TabPanel value="passes">{passes_view}</TabPanel>
                        <TabPanel value="streams">{streams_view}</TabPanel>
                        <TabPanel value="dead">{dead_view}</TabPanel>
                    </Tabs>
                </Show>
            </Show>
        </div>
    }
}
