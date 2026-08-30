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

//! App-specific shared components (CLOACI-T-0933): run-history dots and the
//! Run-workflow modal. Generic rendering comes from the pack; these carry
//! cloacina vocabulary (execution statuses, declared-input slots).

use aurora_leptos::components::{Button, Modal, Switch, TextInput};
use aurora_leptos::tokens::status_color;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::ExecutionSummary;

use crate::auth::{client_for, use_auth};
use crate::data::once_resource;

const MONO: &str = "'IBM Plex Mono', monospace";

/// Last-N run dots for a workflow (React `RunCircles`): newest first, colored
/// by status, capped at 7.
#[component]
pub fn RunCircles(runs: Vec<ExecutionSummary>) -> impl IntoView {
    view! {
        <div style:display="flex" style:gap="4px" style:align-items="center">
            {runs
                .into_iter()
                .take(7)
                .map(|r| {
                    view! {
                        <span
                            title=format!("{} · {}", r.status.to_lowercase(), crate::util::short_id(&r.id))
                            style:width="8px"
                            style:height="8px"
                            style:border-radius="50%"
                            style:background=status_color(&r.status)
                            style:flex="none"
                        ></span>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Run-workflow modal (React `RunWorkflowModal`): fetches the package's
/// declared inputs, renders a typed field per slot, executes with the built
/// context, and navigates to the new execution.
#[component]
pub fn RunWorkflowModal(
    open: RwSignal<bool>,
    /// `(package_name, workflow_name)` of the run target.
    target: RwSignal<Option<(String, String)>>,
) -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let running = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    // Slot values keyed by name, entered as strings; coerced per-schema-type
    // at run time (parity: the React modal kept typed field state).
    let values = RwSignal::new(std::collections::HashMap::<String, String>::new());
    let bools = RwSignal::new(std::collections::HashMap::<String, bool>::new());

    let detail = once_resource(move |c| {
        let pkg = target.get().map(|(p, _)| p);
        async move {
            match pkg {
                Some(p) => c.get_workflow(&p, None).await.map(Some),
                None => Ok(None),
            }
        }
    });

    let params = Signal::derive(move || {
        detail
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|d| d.declared_params)
            .unwrap_or_default()
    });

    let run = move || {
        let Some((_, workflow)) = target.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        running.set(true);
        error.set(String::new());
        let slots = params.get_untracked();
        let vals = values.get_untracked();
        let bvals = bools.get_untracked();
        leptos::task::spawn_local(async move {
            let mut context = serde_json::Map::new();
            for slot in &slots {
                let ty = slot.schema["type"].as_str().unwrap_or("string");
                if ty == "boolean" {
                    if let Some(b) = bvals.get(&slot.name) {
                        context.insert(slot.name.clone(), serde_json::Value::Bool(*b));
                    }
                    continue;
                }
                let Some(raw) = vals.get(&slot.name) else {
                    continue;
                };
                if raw.is_empty() {
                    continue;
                }
                let v = match ty {
                    "integer" => raw
                        .parse::<i64>()
                        .map(serde_json::Value::from)
                        .unwrap_or_else(|_| serde_json::Value::String(raw.clone())),
                    "number" => raw
                        .parse::<f64>()
                        .map(serde_json::Value::from)
                        .unwrap_or_else(|_| serde_json::Value::String(raw.clone())),
                    _ => serde_json::Value::String(raw.clone()),
                };
                context.insert(slot.name.clone(), v);
            }
            let result = async {
                let client = client_for(&conn)?;
                client
                    .execute_workflow(&workflow, serde_json::Value::Object(context))
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            running.set(false);
            match result {
                Ok(res) => {
                    open.set(false);
                    values.set(Default::default());
                    bools.set(Default::default());
                    navigate.with_value(|n| {
                        n(
                            &format!("/executions/{}", res.execution_id),
                            Default::default(),
                        )
                    });
                }
                Err(e) => error.set(e),
            }
        });
    };

    let title = move || {
        target
            .get()
            .map(|(_, w)| format!("Run {w}"))
            .unwrap_or_else(|| "Run".to_string())
    };

    view! {
        <Show when=move || open.get()>
            <Modal open=open title=title()>
                <div style:display="flex" style:flex-direction="column" style:gap="14px">
                    <Show
                        when=move || !params.get().is_empty()
                        fallback=|| view! {
                            <div style:color="var(--muted)" style:font-size="13px">
                                "This workflow declares no inputs — run it with an empty context."
                            </div>
                        }
                    >
                        <For
                            each=move || params.get()
                            key=|p| p.name.clone()
                            children=move |p| {
                                let ty = p.schema["type"].as_str().unwrap_or("any").to_string();
                                let label = format!(
                                    "{}{} · {}",
                                    p.name,
                                    if p.required { " *" } else { "" },
                                    ty
                                );
                                if ty == "boolean" {
                                    let name = p.name.clone();
                                    let checked = RwSignal::new(false);
                                    Effect::new(move |_| {
                                        let v = checked.get();
                                        bools.update(|m| {
                                            m.insert(name.clone(), v);
                                        });
                                    });
                                    view! { <Switch checked=checked label=label /> }.into_any()
                                } else {
                                    let name = p.name.clone();
                                    let field = RwSignal::new(String::new());
                                    Effect::new(move |_| {
                                        let v = field.get();
                                        values.update(|m| {
                                            m.insert(name.clone(), v);
                                        });
                                    });
                                    let placeholder = p
                                        .default
                                        .as_ref()
                                        .map(|d| d.to_string())
                                        .unwrap_or_default();
                                    view! { <TextInput label=label placeholder=placeholder value=field /> }
                                        .into_any()
                                }
                            }
                        />
                    </Show>

                    <Show when=move || !error.get().is_empty()>
                        <div style:color="var(--bad)" style:font-size="12.5px">{move || error.get()}</div>
                    </Show>

                    <div style:display="flex" style:justify-content="flex-end" style:gap="10px">
                        <Button variant="default" on_click=Callback::new(move |_| open.set(false))>
                            "Cancel"
                        </Button>
                        <button
                            class="cl-btn cl-btn--filled"
                            disabled=move || running.get()
                            on:click=move |_| run()
                        >
                            "▸ Run"
                        </button>
                    </div>
                </div>
            </Modal>
        </Show>
    }
}

/// Version / status pill (the inline `pillBg` chips).
#[component]
pub fn TagPill(#[prop(into)] color: String, children: Children) -> impl IntoView {
    view! {
        <span
            style:background=aurora_leptos::tokens::pill_bg(&color)
            style:color=color.clone()
            style:border-radius="10px"
            style:padding="1px 7px"
            style:font-family=MONO
            style:font-size="10.5px"
        >
            {children()}
        </span>
    }
}
