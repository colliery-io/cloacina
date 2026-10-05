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

use std::sync::Arc;

use aurora_leptos::components::{Button, Modal, Switch, TextInput};
use aurora_leptos::tokens::status_color;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use cloacina_api_types::ExecutionSummary;

use crate::auth::{client_for, use_auth};
use crate::data::once_resource;

/// Last-N run dots for a workflow (React `RunCircles`): newest first, colored
/// by status, capped at 7.
#[component]
pub fn RunCircles(runs: Vec<ExecutionSummary>) -> impl IntoView {
    view! {
        <div class="app-rundots">
            {runs
                .into_iter()
                .take(7)
                .map(|r| {
                    view! {
                        // The colour is the run status (data).
                        <span
                            class="app-rundot"
                            title=format!("{} · {}", r.status.to_lowercase(), crate::util::short_id(&r.id))
                            style:background=status_color(&r.status)
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

    let footer: ChildrenFn = Arc::new(move || {
        view! {
            <Button variant="default" on_click=Callback::new(move |_| open.set(false))>
                "Cancel"
            </Button>
            <Button loading=running on_click=Callback::new(move |_| run())>
                "▸ Run"
            </Button>
        }
        .into_any()
    });

    view! {
        // Re-mounted on each open so the title follows the target.
        <Show when=move || open.get()>
            <Modal open=open title=title() footer=footer.clone() locked=running>
                <div class="app-col app-col--loose">
                    <Show
                        when=move || !params.get().is_empty()
                        fallback=|| view! {
                            <div class="app-hint">
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
                        <div class="app-error" role="alert">{move || error.get()}</div>
                    </Show>
                </div>
            </Modal>
        </Show>
    }
}

/// Trigger fire modal (React `TriggerFireModal`, CLOACI-T-0777): fetches the
/// trigger's declared pass-through interface, renders one typed field per
/// slot, fires, then lists the fan-out (fired workflows).
#[component]
pub fn TriggerFireModal(open: RwSignal<bool>, trigger: RwSignal<Option<String>>) -> impl IntoView {
    use cloacina_api_types::FireTriggerRequest;

    let auth = use_auth();
    let firing = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let result = RwSignal::new(Option::<cloacina_api_types::FireTriggerResponse>::None);
    let values = RwSignal::new(std::collections::HashMap::<String, String>::new());
    let bools = RwSignal::new(std::collections::HashMap::<String, bool>::new());

    let iface = once_resource(move |c| {
        let name = trigger.get();
        async move {
            match name {
                Some(n) => c.trigger_interface(&n, None).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let slots = Signal::derive(move || {
        iface
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|s| s.slots)
            .unwrap_or_default()
    });

    let close = move || {
        open.set(false);
        values.set(Default::default());
        bools.set(Default::default());
        result.set(None);
        error.set(String::new());
    };

    let fire = move || {
        let Some(name) = trigger.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        firing.set(true);
        error.set(String::new());
        let fields = slots.get_untracked();
        let vals = values.get_untracked();
        let bvals = bools.get_untracked();
        leptos::task::spawn_local(async move {
            let mut event = serde_json::Map::new();
            for slot in &fields {
                let ty = slot.schema["type"].as_str().unwrap_or("string");
                if ty == "boolean" {
                    if let Some(b) = bvals.get(&slot.name) {
                        event.insert(slot.name.clone(), serde_json::Value::Bool(*b));
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
                event.insert(slot.name.clone(), v);
            }
            let request = FireTriggerRequest {
                event: if event.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(event))
                },
            };
            let outcome = async {
                let client = client_for(&conn)?;
                client
                    .fire_trigger(&name, &request, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            firing.set(false);
            match outcome {
                Ok(res) => result.set(Some(res)),
                Err(e) => error.set(e),
            }
        });
    };

    let title = move || {
        trigger
            .get()
            .map(|t| format!("{t} · fire"))
            .unwrap_or_default()
    };

    let footer: ChildrenFn = Arc::new(move || {
        view! {
            <Show
                when=move || result.get().is_some()
                fallback=move || view! {
                    <Button variant="default" on_click=Callback::new(move |_| close())>
                        "Cancel"
                    </Button>
                    <Button loading=firing on_click=Callback::new(move |_| fire())>
                        "⚡ Fire"
                    </Button>
                }
            >
                <Button on_click=Callback::new(move |_| close())>"Done"</Button>
            </Show>
        }
        .into_any()
    });

    view! {
        // Re-mounted on each open so the title follows the trigger.
        <Show when=move || open.get()>
            <Modal
                open=open
                title=title()
                footer=footer.clone()
                locked=firing
                on_close=Callback::new(move |_| close())
            >
                <div class="app-col app-col--loose">
                    <Show
                        when=move || result.get().is_some()
                        fallback=move || view! {
                            <Show
                                when=move || !slots.get().is_empty()
                                fallback=|| view! {
                                    <div class="app-hint">
                                        "No declared inputs — fire with just the trigger metadata."
                                    </div>
                                }
                            >
                                <For
                                    each=move || slots.get()
                                    key=|s| s.name.clone()
                                    children=move |s| {
                                        let ty = s.schema["type"].as_str().unwrap_or("string").to_string();
                                        let label = format!("{} · {}", s.name, ty);
                                        if ty == "boolean" {
                                            let name = s.name.clone();
                                            let checked = RwSignal::new(false);
                                            Effect::new(move |_| {
                                                let v = checked.get();
                                                bools.update(|m| {
                                                    m.insert(name.clone(), v);
                                                });
                                            });
                                            view! { <Switch checked=checked label=label /> }.into_any()
                                        } else {
                                            let name = s.name.clone();
                                            let field = RwSignal::new(String::new());
                                            Effect::new(move |_| {
                                                let v = field.get();
                                                values.update(|m| {
                                                    m.insert(name.clone(), v);
                                                });
                                            });
                                            view! { <TextInput label=label value=field /> }.into_any()
                                        }
                                    }
                                />
                            </Show>

                            <Show when=move || !error.get().is_empty()>
                                <div class="app-error" role="alert">{move || error.get()}</div>
                            </Show>
                        }
                    >
                        {move || result.get().map(|r| view! {
                            <div class="app-text">
                                {format!("Fired {} workflow{}:", r.fired, if r.fired == 1 { "" } else { "s" })}
                            </div>
                            <div class="app-col app-col--tight">
                                {r.executions
                                    .iter()
                                    .map(|e| view! {
                                        <div class="app-mono app-small app-fg2">
                                            {format!("↳ {}", e.workflow_name)}
                                        </div>
                                    })
                                    .collect_view()}
                            </div>
                        })}
                    </Show>
                </div>
            </Modal>
        </Show>
    }
}

/// Accumulator inject modal (React `GraphInjectModal`, CLOACI-T-0753): typed
/// slot fields from the accumulator's declared interface, injected as one
/// JSON event. Falls back to a raw-JSON textarea when nothing is declared.
#[component]
pub fn GraphInjectModal(
    open: RwSignal<bool>,
    accumulator: RwSignal<Option<String>>,
) -> impl IntoView {
    use cloacina_api_types::InjectAccumulatorRequest;

    let auth = use_auth();
    let busy = RwSignal::new(false);
    let error = RwSignal::new(String::new());
    let delivered = RwSignal::new(Option::<usize>::None);
    let values = RwSignal::new(std::collections::HashMap::<String, String>::new());
    let raw_json = RwSignal::new(String::from("{}"));

    let iface = once_resource(move |c| {
        let name = accumulator.get();
        async move {
            match name {
                Some(n) => c.accumulator_interface(&n).await.map(Some),
                None => Ok(None),
            }
        }
    });
    let slots = Signal::derive(move || {
        iface
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|s| s.slots)
            .unwrap_or_default()
    });

    let close = move || {
        open.set(false);
        values.set(Default::default());
        raw_json.set("{}".into());
        delivered.set(None);
        error.set(String::new());
    };

    let inject = move || {
        let Some(name) = accumulator.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        busy.set(true);
        error.set(String::new());
        let fields = slots.get_untracked();
        let vals = values.get_untracked();
        let raw = raw_json.get_untracked();
        leptos::task::spawn_local(async move {
            let event = if fields.is_empty() {
                match serde_json::from_str::<serde_json::Value>(&raw) {
                    Ok(v) => v,
                    Err(e) => {
                        busy.set(false);
                        error.set(format!("event is not valid JSON: {e}"));
                        return;
                    }
                }
            } else {
                let mut obj = serde_json::Map::new();
                for slot in &fields {
                    let ty = slot.schema["type"].as_str().unwrap_or("string");
                    let Some(rawv) = vals.get(&slot.name) else {
                        continue;
                    };
                    if rawv.is_empty() {
                        continue;
                    }
                    let v = match ty {
                        "integer" => rawv
                            .parse::<i64>()
                            .map(serde_json::Value::from)
                            .unwrap_or_else(|_| serde_json::Value::String(rawv.clone())),
                        "number" => rawv
                            .parse::<f64>()
                            .map(serde_json::Value::from)
                            .unwrap_or_else(|_| serde_json::Value::String(rawv.clone())),
                        "boolean" => serde_json::Value::Bool(rawv == "true"),
                        _ => serde_json::Value::String(rawv.clone()),
                    };
                    obj.insert(slot.name.clone(), v);
                }
                serde_json::Value::Object(obj)
            };
            let outcome = async {
                let client = client_for(&conn)?;
                client
                    .inject_accumulator(&name, &InjectAccumulatorRequest { event })
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            busy.set(false);
            match outcome {
                Ok(res) => delivered.set(Some(res.delivered)),
                Err(e) => error.set(e),
            }
        });
    };

    let title = move || {
        accumulator
            .get()
            .map(|a| format!("{a} · inject"))
            .unwrap_or_default()
    };

    let footer: ChildrenFn = Arc::new(move || {
        view! {
            <Show
                when=move || delivered.get().is_some()
                fallback=move || view! {
                    <Button variant="default" on_click=Callback::new(move |_| close())>
                        "Cancel"
                    </Button>
                    <Button loading=busy on_click=Callback::new(move |_| inject())>
                        "＋ Inject"
                    </Button>
                }
            >
                <Button on_click=Callback::new(move |_| close())>"Done"</Button>
            </Show>
        }
        .into_any()
    });

    view! {
        // Re-mounted on each open so the title follows the accumulator.
        <Show when=move || open.get()>
            <Modal
                open=open
                title=title()
                footer=footer.clone()
                locked=busy
                on_close=Callback::new(move |_| close())
            >
                <div class="app-col app-col--loose">
                    <Show
                        when=move || delivered.get().is_some()
                        fallback=move || view! {
                            <Show
                                when=move || !slots.get().is_empty()
                                fallback=move || view! {
                                    <aurora_leptos::components::Textarea
                                        label="Event (JSON)"
                                        value=raw_json
                                        mono=true
                                    />
                                }
                            >
                                <For
                                    each=move || slots.get()
                                    key=|s| s.name.clone()
                                    children=move |s| {
                                        let ty = s.schema["type"].as_str().unwrap_or("string").to_string();
                                        let name = s.name.clone();
                                        let field = RwSignal::new(String::new());
                                        Effect::new(move |_| {
                                            let v = field.get();
                                            values.update(|m| {
                                                m.insert(name.clone(), v);
                                            });
                                        });
                                        view! {
                                            <TextInput label=format!("{} · {}", s.name, ty) value=field />
                                        }
                                    }
                                />
                            </Show>

                            <Show when=move || !error.get().is_empty()>
                                <div class="app-error" role="alert">{move || error.get()}</div>
                            </Show>
                        }
                    >
                        <div class="app-text">
                            {move || format!(
                                "Delivered to {} receiver{}.",
                                delivered.get().unwrap_or(0),
                                if delivered.get() == Some(1) { "" } else { "s" }
                            )}
                        </div>
                    </Show>
                </div>
            </Modal>
        </Show>
    }
}
