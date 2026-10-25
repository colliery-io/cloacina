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

//! Create and edit a sync (COLLIERY-I-0635 / CLOACINA-T-1006). Each side
//! picks a connector from the catalog and fills the config its schema asks
//! for (or an advanced JSON override), with a check button that runs the
//! connector where Sync runs. The stream comes from discovery, which re-runs
//! when the source's config changes. A definition side names a stored secret,
//! which can be created here. Saving goes through the instances route; the
//! server checks the params again and its reasons are shown.

use std::collections::BTreeMap;
use std::sync::Arc;

use aurora_leptos::components::{
    Alert, Button, Loading, Modal, PageHeader, Panel, PasswordInput, Select, Switch, TextInput,
    Textarea,
};
use aurora_leptos::tokens::token;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde_json::{Map, Value, json};

use cloacina_api_types::{
    ConnectorCheckResponse, ConnectorProbeRequest, ConnectorSummary, CreateInstanceRequest,
    CreateSecretRequest, DiscoveredStream, UpdateInstanceRequest,
};

use crate::auth::{client_for, use_auth};
use crate::data::once_resource;

/// One config field the connector's schema asks for.
#[derive(Clone)]
struct ConfigField {
    name: String,
    description: String,
    kind: String,
    secret: bool,
    required: bool,
    choices: Vec<String>,
    value: RwSignal<String>,
}

/// The fields of a JSON Schema object, in its order.
fn fields_of(schema: Option<&Value>) -> Vec<ConfigField> {
    let Some(schema) = schema else {
        return Vec::new();
    };
    let required: Vec<String> = schema["required"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    schema["properties"]
        .as_object()
        .map(|props| {
            props
                .iter()
                .map(|(name, p)| ConfigField {
                    name: name.clone(),
                    description: p["description"].as_str().unwrap_or_default().to_string(),
                    kind: p["type"].as_str().unwrap_or("string").to_string(),
                    secret: p["airbyte_secret"].as_bool().unwrap_or(false)
                        || p["format"].as_str() == Some("password"),
                    required: required.contains(name),
                    choices: p["enum"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                    value: RwSignal::new(
                        p["default"]
                            .as_str()
                            .map(str::to_string)
                            .or_else(|| p["default"].as_i64().map(|n| n.to_string()))
                            .unwrap_or_default(),
                    ),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// One side of the sync, as the form holds it.
#[derive(Clone, Copy)]
struct Side {
    role: &'static str,
    connector: RwSignal<String>,
    fields: RwSignal<Vec<ConfigField>>,
    advanced: RwSignal<String>,
    secret: RwSignal<String>,
    grants: RwSignal<String>,
    credential: RwSignal<String>,
    checked: RwSignal<Option<Result<ConnectorCheckResponse, String>>>,
    checking: RwSignal<bool>,
    /// A stored config waiting for the connector's fields, when editing.
    pending: RwSignal<Option<Map<String, Value>>>,
}

impl Side {
    fn new(role: &'static str) -> Self {
        Self {
            role,
            connector: RwSignal::new(String::new()),
            fields: RwSignal::new(Vec::new()),
            advanced: RwSignal::new(String::new()),
            secret: RwSignal::new(String::new()),
            grants: RwSignal::new(String::new()),
            credential: RwSignal::new(String::new()),
            checked: RwSignal::new(None),
            checking: RwSignal::new(false),
            pending: RwSignal::new(None),
        }
    }

    fn entry(&self, catalog: &[ConnectorSummary]) -> Option<ConnectorSummary> {
        let name = self.connector.get();
        catalog.iter().find(|c| c.name == name).cloned()
    }

    /// The config object: the advanced JSON when given, else the fields.
    fn config(&self) -> Result<Map<String, Value>, String> {
        let advanced = self.advanced.get();
        if !advanced.trim().is_empty() {
            return match serde_json::from_str::<Value>(&advanced) {
                Ok(Value::Object(o)) => Ok(o),
                Ok(_) => Err(format!("the {} config must be a JSON object", self.role)),
                Err(e) => Err(format!("the {} config is not JSON: {e}", self.role)),
            };
        }
        let mut out = Map::new();
        for f in self.fields.get() {
            let v = f.value.get();
            if v.trim().is_empty() {
                if f.required {
                    return Err(format!("the {} field `{}` is required", self.role, f.name));
                }
                continue;
            }
            let value = match f.kind.as_str() {
                "integer" => v
                    .trim()
                    .parse::<i64>()
                    .map(Value::from)
                    .map_err(|_| format!("the {} field `{}` is a number", self.role, f.name))?,
                "boolean" => Value::Bool(v == "true"),
                _ => Value::String(v),
            };
            out.insert(f.name.clone(), value);
        }
        Ok(out)
    }

    fn json(&self, text: RwSignal<String>, what: &str) -> Result<Option<Value>, String> {
        let t = text.get();
        if t.trim().is_empty() {
            return Ok(None);
        }
        serde_json::from_str(&t)
            .map(Some)
            .map_err(|e| format!("the {} {what} is not JSON: {e}", self.role))
    }

    /// The side as a check or discover request.
    fn probe(&self, catalog: &[ConnectorSummary]) -> Result<ConnectorProbeRequest, String> {
        let entry = self
            .entry(catalog)
            .ok_or_else(|| format!("choose a {}", self.role))?;
        let config = Value::Object(self.config()?);
        Ok(if entry.kind == "declarative" {
            ConnectorProbeRequest {
                side: self.role.into(),
                definition: Some(entry.name),
                config: Some(config),
                secret: Some(self.secret.get()).filter(|s| !s.is_empty()),
                ..Default::default()
            }
        } else {
            ConnectorProbeRequest {
                side: self.role.into(),
                provider: Some(entry.name),
                member: Some(self.role.into()),
                config: Some(config),
                grants: self.json(self.grants, "grants")?,
                credential: self.json(self.credential, "credential")?,
                ..Default::default()
            }
        })
    }

    /// The side's sync params.
    fn params(
        &self,
        catalog: &[ConnectorSummary],
        out: &mut Map<String, Value>,
    ) -> Result<(), String> {
        let entry = self
            .entry(catalog)
            .ok_or_else(|| format!("choose a {}", self.role))?;
        let r = self.role;
        let config = self.config()?;
        if entry.kind == "declarative" {
            out.insert(format!("{r}_definition"), json!(entry.name));
            out.insert(format!("{r}_config"), Value::Object(config));
            if !self.secret.get().is_empty() {
                out.insert(format!("{r}_secret"), json!(self.secret.get()));
            }
        } else {
            out.insert(format!("{r}_provider"), json!(entry.name));
            out.insert(format!("{r}_member"), json!(r));
            out.insert(
                format!("{r}_config"),
                json!(Value::Object(config).to_string()),
            );
            if let Some(g) = self.json(self.grants, "grants")? {
                out.insert(format!("{r}_grants"), json!(g.to_string()));
            }
            if let Some(c) = self.json(self.credential, "credential")? {
                out.insert(format!("{r}_credential"), json!(c.to_string()));
            }
        }
        Ok(())
    }

    /// Fill the side from stored params. The config waits in `pending` until
    /// the connector's fields are known.
    fn load(&self, params: &Map<String, Value>) {
        let r = self.role;
        let text = |k: String| params.get(&k).and_then(Value::as_str).map(str::to_string);
        let name = text(format!("{r}_definition")).or_else(|| text(format!("{r}_provider")));
        let Some(name) = name else {
            return;
        };
        let config: Map<String, Value> = match params.get(&format!("{r}_config")) {
            Some(Value::Object(o)) => o.clone(),
            Some(Value::String(s)) => serde_json::from_str(s).unwrap_or_default(),
            _ => Map::new(),
        };
        self.pending.set(Some(config));
        self.secret
            .set(text(format!("{r}_secret")).unwrap_or_default());
        self.grants
            .set(text(format!("{r}_grants")).unwrap_or_default());
        self.credential
            .set(text(format!("{r}_credential")).unwrap_or_default());
        self.connector.set(name);
    }

    /// Show `fields`, filled from a pending stored config: into the fields
    /// when they cover it, else into the advanced JSON.
    fn apply(&self, fields: Vec<ConfigField>) {
        if let Some(config) = self.pending.get_untracked() {
            let known: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
            if config.keys().all(|k| known.contains(&k.as_str())) {
                for f in &fields {
                    if let Some(v) = config.get(&f.name) {
                        f.value.set(match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        });
                    }
                }
            } else {
                self.advanced.set(Value::Object(config).to_string());
            }
            self.pending.set(None);
        }
        self.fields.set(fields);
    }
}

#[component]
pub fn SyncForm() -> impl IntoView {
    let auth = use_auth();
    let navigate = StoredValue::new(use_navigate());
    let params = use_params_map();
    let editing = Signal::derive(move || params.read().get("name"));

    let catalog = once_resource(|c| async move { c.list_connectors(None).await });
    let secrets_refresh = RwSignal::new(0u32);
    let secrets = once_resource(move |c| {
        secrets_refresh.track();
        async move { c.list_secrets(None).await }
    });
    let catalog_items = Signal::derive(move || {
        catalog
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items)
            .unwrap_or_default()
    });
    let secret_names = Signal::derive(move || {
        secrets
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.items.into_iter().map(|s| s.name).collect::<Vec<_>>())
            .unwrap_or_default()
    });

    let name = RwSignal::new(String::new());
    let source = Side::new("source");
    let destination = Side::new("destination");
    let stream = RwSignal::new(String::new());
    let sync_mode = RwSignal::new("FullRefresh".to_string());
    let cursor = RwSignal::new(String::new());
    let write_mode = RwSignal::new("Append".to_string());
    let keys = RwSignal::new(String::new());
    let cron = RwSignal::new(String::new());
    let timezone = RwSignal::new("UTC".to_string());
    let enabled = RwSignal::new(true);
    let error = RwSignal::new(String::new());
    let saving = RwSignal::new(false);

    // A new connector choice brings its schema's fields: the catalog's, or a
    // compiled connector's own spec, read where connectors run.
    for side in [source, destination] {
        Effect::new(move |prev: Option<String>| {
            let chosen = side.connector.get();
            let cat = catalog_items.get();
            if prev.as_ref() == Some(&chosen) {
                return chosen;
            }
            if prev.is_some() && side.pending.get_untracked().is_none() {
                side.advanced.set(String::new());
                side.checked.set(None);
            }
            let entry = cat.into_iter().find(|c| c.name == chosen);
            let from_catalog = fields_of(entry.as_ref().and_then(|e| e.config_schema.as_ref()));
            match entry {
                Some(e) if from_catalog.is_empty() && e.kind == "compiled" => {
                    if let Some(conn) = auth.connection() {
                        let request = ConnectorProbeRequest {
                            side: side.role.into(),
                            provider: Some(e.name.clone()),
                            member: Some(side.role.into()),
                            ..Default::default()
                        };
                        leptos::task::spawn_local(async move {
                            let spec = async {
                                client_for(&conn)?
                                    .spec_connector(&request, None)
                                    .await
                                    .map_err(|e| e.to_string())
                            }
                            .await;
                            match spec {
                                Ok(s) => side.apply(fields_of(Some(&s.config_schema))),
                                Err(_) => side.apply(Vec::new()),
                            }
                        });
                    }
                }
                _ => side.apply(from_catalog),
            }
            chosen
        });
    }

    // Discovery: the source's streams, again whenever its config settles.
    let discovered = RwSignal::new(Vec::<DiscoveredStream>::new());
    let discovering = RwSignal::new(false);
    let discover_error = RwSignal::new(String::new());
    let generation = StoredValue::new(0u64);
    let discover = move || {
        let Some(conn) = auth.connection() else {
            return;
        };
        let request = match source.probe(&catalog_items.get_untracked()) {
            Ok(r) => r,
            Err(_) => return,
        };
        generation.update_value(|g| *g += 1);
        let mine = generation.get_value();
        discovering.set(true);
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(600).await;
            if generation.get_value() != mine {
                return;
            }
            let result = async {
                client_for(&conn)?
                    .discover_connector(&request, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            if generation.get_value() != mine {
                return;
            }
            discovering.set(false);
            match result {
                Ok(r) => {
                    discover_error.set(String::new());
                    discovered.set(r.streams);
                }
                Err(e) => discover_error.set(e),
            }
        });
    };
    Effect::new(move |_| {
        // Track the source's choice and every config value.
        source.connector.track();
        source.advanced.track();
        source.secret.track();
        for f in source.fields.get() {
            f.value.track();
        }
        discover();
    });
    let chosen_stream = Signal::derive(move || {
        let s = stream.get();
        discovered.get().into_iter().find(|d| d.name == s)
    });
    // Picking a stream proposes its mode, cursor and keys.
    Effect::new(move |_| {
        if let Some(d) = chosen_stream.get() {
            if !d.sync_modes.contains(&sync_mode.get_untracked()) {
                sync_mode.set(
                    d.sync_modes
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "FullRefresh".into()),
                );
            }
            if cursor.get_untracked().is_empty() {
                cursor.set(d.default_cursor_field.clone().unwrap_or_default());
            }
            if keys.get_untracked().is_empty() {
                keys.set(d.primary_key.clone().unwrap_or_default().join(", "));
            }
        }
    });

    // Editing: load the sync once the catalog is in.
    let loaded = RwSignal::new(false);
    Effect::new(move |_| {
        let Some(n) = editing.get() else {
            return;
        };
        let cat = catalog_items.get();
        if cat.is_empty() || loaded.get_untracked() {
            return;
        }
        let Some(conn) = auth.connection() else {
            return;
        };
        loaded.set(true);
        leptos::task::spawn_local(async move {
            let result = async {
                client_for(&conn)?
                    .get_sync(&n, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            match result {
                Ok(detail) => {
                    name.set(detail.summary.name.clone());
                    let p = detail
                        .params
                        .and_then(|v| v.as_object().cloned())
                        .unwrap_or_default();
                    source.load(&p);
                    destination.load(&p);
                    if let Some(s) = p
                        .get("stream")
                        .and_then(Value::as_str)
                        .and_then(|s| serde_json::from_str::<Value>(s).ok())
                    {
                        stream.set(s["stream"].as_str().unwrap_or_default().to_string());
                        sync_mode.set(s["sync_mode"].as_str().unwrap_or("FullRefresh").to_string());
                        cursor.set(s["cursor_field"].as_str().unwrap_or_default().to_string());
                        match &s["write_mode"] {
                            Value::String(m) => write_mode.set(m.clone()),
                            Value::Object(o) if o.contains_key("Upsert") => {
                                write_mode.set("Upsert".into());
                                keys.set(
                                    o["Upsert"]["business_keys"]
                                        .as_array()
                                        .map(|a| {
                                            a.iter()
                                                .filter_map(|v| v.as_str())
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        })
                                        .unwrap_or_default(),
                                );
                            }
                            _ => {}
                        }
                    }
                    cron.set(detail.summary.cron.clone().unwrap_or_default());
                    timezone.set(
                        detail
                            .summary
                            .timezone
                            .clone()
                            .unwrap_or_else(|| "UTC".into()),
                    );
                    enabled.set(detail.summary.enabled);
                }
                Err(e) => error.set(e),
            }
        });
    });

    let check = move |side: Side| {
        let Some(conn) = auth.connection() else {
            return;
        };
        let request = match side.probe(&catalog_items.get_untracked()) {
            Ok(r) => r,
            Err(e) => {
                side.checked.set(Some(Err(e)));
                return;
            }
        };
        side.checking.set(true);
        leptos::task::spawn_local(async move {
            let result = async {
                client_for(&conn)?
                    .check_connector(&request, None)
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            side.checking.set(false);
            side.checked.set(Some(result));
        });
    };

    let save = move || {
        error.set(String::new());
        let Some(conn) = auth.connection() else {
            return;
        };
        let instance = name.get().trim().to_string();
        if instance.is_empty() {
            error.set("Name the sync.".into());
            return;
        }
        let cat = catalog_items.get_untracked();
        let mut p = Map::new();
        p.insert("sync".into(), json!(instance));
        if let Err(e) = source
            .params(&cat, &mut p)
            .and_then(|_| destination.params(&cat, &mut p))
        {
            error.set(e);
            return;
        }
        if stream.get().trim().is_empty() {
            error.set("Choose a stream.".into());
            return;
        }
        let write = match write_mode.get().as_str() {
            "Upsert" => {
                let k: Vec<String> = keys
                    .get()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if k.is_empty() {
                    error.set("An upsert needs business keys.".into());
                    return;
                }
                json!({ "Upsert": { "business_keys": k } })
            }
            other => json!(other),
        };
        let primary_key: Option<Vec<String>> = {
            let k: Vec<String> = keys
                .get()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            (!k.is_empty()).then_some(k)
        };
        let configured = json!({
            "stream": stream.get().trim(),
            "sync_mode": sync_mode.get(),
            "cursor_field": Some(cursor.get()).filter(|c| !c.trim().is_empty()),
            "primary_key": primary_key,
            "write_mode": write,
            "mapping": { "ops": [] },
        });
        p.insert("stream".into(), json!(configured.to_string()));
        let cron_v = Some(cron.get().trim().to_string()).filter(|c| !c.is_empty());
        let tz = Some(timezone.get().trim().to_string()).filter(|t| !t.is_empty());
        let editing_now = editing.get_untracked();
        saving.set(true);
        leptos::task::spawn_local(async move {
            let result = async {
                let c = client_for(&conn)?;
                match editing_now {
                    Some(n) => c
                        .update_instance(
                            "sync",
                            &n,
                            &UpdateInstanceRequest {
                                params: Some(Value::Object(p)),
                                cron: cron_v,
                                timezone: tz,
                                enabled: Some(enabled.get_untracked()),
                            },
                            None,
                        )
                        .await
                        .map(|_| ()),
                    None => c
                        .create_instance(
                            "sync",
                            &CreateInstanceRequest {
                                instance_name: instance.clone(),
                                params: Some(Value::Object(p)),
                                cron: cron_v,
                                timezone: tz,
                                enabled: Some(enabled.get_untracked()),
                            },
                            None,
                        )
                        .await
                        .map(|_| ()),
                }
                .map_err(|e| e.to_string())
            }
            .await;
            saving.set(false);
            match result {
                Ok(()) => navigate.with_value(|n| {
                    n(
                        &format!("/syncs/{}", urlencoding::encode(&instance)),
                        Default::default(),
                    )
                }),
                Err(e) => error.set(e),
            }
        });
    };

    // A secret made here, for a definition side.
    let new_secret_for = RwSignal::new(Option::<Side>::None);
    let new_secret_open = RwSignal::new(false);
    Effect::new(move |_| new_secret_open.set(new_secret_for.get().is_some()));
    let new_secret_name = RwSignal::new(String::new());
    let new_secret_fields = RwSignal::new(Vec::<(String, RwSignal<String>)>::new());
    let new_secret_error = RwSignal::new(String::new());
    let open_new_secret = move |side: Side| {
        let wanted = side
            .entry(&catalog_items.get_untracked())
            .map(|e| e.secret_fields)
            .unwrap_or_default();
        new_secret_name.set(String::new());
        new_secret_error.set(String::new());
        new_secret_fields.set(
            wanted
                .into_iter()
                .map(|f| (f, RwSignal::new(String::new())))
                .collect(),
        );
        new_secret_for.set(Some(side));
    };
    let create_secret = move || {
        let Some(side) = new_secret_for.get_untracked() else {
            return;
        };
        let Some(conn) = auth.connection() else {
            return;
        };
        let secret = new_secret_name.get().trim().to_string();
        let fields: BTreeMap<String, String> = new_secret_fields
            .get()
            .into_iter()
            .map(|(k, v)| (k, v.get()))
            .collect();
        if secret.is_empty() || fields.values().any(|v| v.is_empty()) {
            new_secret_error.set("Name the secret and fill every field.".into());
            return;
        }
        leptos::task::spawn_local(async move {
            let result = async {
                client_for(&conn)?
                    .create_secret(
                        &CreateSecretRequest {
                            name: secret.clone(),
                            fields,
                        },
                        None,
                    )
                    .await
                    .map_err(|e| e.to_string())
            }
            .await;
            match result {
                Ok(_) => {
                    secrets_refresh.update(|x| *x += 1);
                    side.secret.set(secret);
                    new_secret_for.set(None);
                }
                Err(e) => new_secret_error.set(e),
            }
        });
    };

    let side_view = move |side: Side, title: &'static str| {
        let role_cap = if side.role == "source" {
            "Source"
        } else {
            "Destination"
        };
        let options = Signal::derive(move || {
            let mut opts = vec![(String::new(), format!("Choose a {}…", side.role))];
            opts.extend(
                catalog_items
                    .get()
                    .into_iter()
                    .filter(|c| c.roles.iter().any(|r| r == role_cap))
                    .map(|c| {
                        let label = if c.kind == "declarative" {
                            format!("{} (definition)", c.name)
                        } else {
                            c.name.clone()
                        };
                        (c.name, label)
                    }),
            );
            opts
        });
        let entry = Signal::derive(move || side.entry(&catalog_items.get()));
        let is_definition =
            Signal::derive(move || entry.get().is_some_and(|e| e.kind == "declarative"));
        let secret_options = Signal::derive(move || {
            let mut o = vec![(String::new(), "No secret".to_string())];
            o.extend(secret_names.get().into_iter().map(|n| (n.clone(), n)));
            o
        });
        view! {
            <Panel title=title>
                <div class="app-col">
                    <Select label=role_cap option_pairs=options value=side.connector />
                    <Show when=move || entry.get().is_some()>
                        <For
                            each=move || side.fields.get()
                            key=|f| f.name.clone()
                            children=move |f| {
                                let label = if f.required { format!("{} *", f.name) } else { f.name.clone() };
                                let desc = f.description.clone();
                                if !f.choices.is_empty() {
                                    let pairs: Vec<(String, String)> = std::iter::once((String::new(), "—".to_string()))
                                        .chain(f.choices.iter().map(|c| (c.clone(), c.clone())))
                                        .collect();
                                    view! { <Select label=label option_pairs=pairs value=f.value description=desc /> }.into_any()
                                } else if f.secret {
                                    view! {
                                        <PasswordInput label=label value=f.value description=Signal::derive(move || {
                                            "Stored in the sync's params until connectors read secrets (COLLIERY-I-0651).".to_string()
                                        }) />
                                    }.into_any()
                                } else {
                                    view! { <TextInput label=label value=f.value mono=true description=desc /> }.into_any()
                                }
                            }
                        />
                        <Show when=move || is_definition.get()>
                            <div class="app-row app-row--end">
                                <div class="app-grow">
                                    <Select
                                        label="Secret"
                                        option_pairs=secret_options
                                        value=side.secret
                                        description=Signal::derive(move || {
                                            let wanted = entry.get().map(|e| e.secret_fields).unwrap_or_default();
                                            if wanted.is_empty() { String::new() } else { format!("Holds: {}", wanted.join(", ")) }
                                        })
                                    />
                                </div>
                                <Button variant="default" on_click=Callback::new(move |_| open_new_secret(side))>
                                    "New secret"
                                </Button>
                            </div>
                        </Show>
                        <Show when=move || !is_definition.get()>
                            <Textarea label="Grants (JSON)" value=side.grants mono=true rows=2
                                placeholder=r#"{"tcp": ["db.example.org:5432"]}"#
                                description=Signal::derive(|| "Where the connector may reach. Without grants it can reach nothing.".to_string()) />
                            <Textarea label="Credential (JSON, optional)" value=side.credential mono=true rows=2
                                placeholder=r#"{"kind": "header", "secret": "api", "name": "Authorization", "field": "token", "prefix": "Bearer "}"# />
                        </Show>
                        <Textarea label="Advanced: config as JSON" value=side.advanced mono=true rows=3
                            description=Signal::derive(|| "When given, it replaces the fields above.".to_string()) />
                        <div class="app-row">
                            <Button variant="default" loading=side.checking on_click=Callback::new(move |_| check(side))>
                                "Check"
                            </Button>
                            {move || side.checked.get().map(|r| match r {
                                Ok(c) if c.ok => view! { <span class="app-small app-ok">"✓ The connector answered."</span> }.into_any(),
                                Ok(c) => view! { <span class="app-error">{format!("✗ {}", c.message.unwrap_or_else(|| "the check failed".into()))}</span> }.into_any(),
                                Err(e) => view! { <span class="app-error">{e}</span> }.into_any(),
                            })}
                        </div>
                    </Show>
                </div>
            </Panel>
        }
    };

    let stream_options = Signal::derive(move || {
        let mut o = vec![(String::new(), "Choose a stream…".to_string())];
        o.extend(
            discovered
                .get()
                .into_iter()
                .map(|d| (d.name.clone(), d.name)),
        );
        let current = stream.get();
        if !current.is_empty() && !o.iter().any(|(v, _)| *v == current) {
            o.push((current.clone(), current));
        }
        o
    });
    let mode_options = Signal::derive(move || {
        chosen_stream
            .get()
            .map(|d| d.sync_modes)
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| vec!["FullRefresh".into(), "Incremental".into()])
    });
    let cursor_options = Signal::derive(move || {
        chosen_stream
            .get()
            .map(|d| d.fields.into_iter().map(|f| f.name).collect::<Vec<_>>())
            .unwrap_or_default()
    });

    let secret_footer: ChildrenFn = Arc::new(move || {
        view! {
            <Button variant="default" on_click=Callback::new(move |_| new_secret_for.set(None))>"Cancel"</Button>
            <Button on_click=Callback::new(move |_| create_secret())>"Create"</Button>
        }
        .into_any()
    });

    view! {
        <div class="app-page app-narrow">
            {move || view! {
                <PageHeader
                    title=editing.get().map(|n| format!("Edit {n}")).unwrap_or_else(|| "New sync".into())
                    back_href=editing.get().map(|n| format!("/syncs/{}", urlencoding::encode(&n))).unwrap_or_else(|| "/syncs".into())
                    back_label=if editing.get().is_some() { "Sync" } else { "Syncs" }
                />
            }}
            <Show
                when=move || auth.can_write()
                fallback=|| view! { <Alert color=token::GOLD>"Making or changing a sync needs a write key."</Alert> }
            >
                <Show
                    when=move || catalog.get().is_some()
                    fallback=|| view! { <Loading label="Loading connectors…" /> }
                >
                    <form class="app-col app-col--loose" on:submit=move |ev| { ev.prevent_default(); save() }>
                        <Panel title="Sync">
                            <TextInput
                                label="Name"
                                value=name
                                mono=true
                                required=true
                                disabled=Signal::derive(move || editing.get().is_some())
                                description=Signal::derive(|| "Its cursor and dead-letters are kept under this name; it cannot change.".to_string())
                            />
                        </Panel>
                        {side_view(source, "Source")}
                        <Panel title="Stream">
                            <div class="app-col">
                                <div class="app-row app-row--end">
                                    <div class="app-grow">
                                        <Select label="Stream" option_pairs=stream_options value=stream />
                                    </div>
                                    <Button variant="default" loading=discovering on_click=Callback::new(move |_| discover())>
                                        "Discover again"
                                    </Button>
                                </div>
                                <Show when=move || !discover_error.get().is_empty()>
                                    <span class="app-error">{move || discover_error.get()}</span>
                                </Show>
                                <Select label="Sync mode" options=mode_options value=sync_mode />
                                <Show when=move || sync_mode.get() == "Incremental">
                                    <Show
                                        when=move || !cursor_options.get().is_empty()
                                        fallback=move || view! { <TextInput label="Cursor field" value=cursor mono=true /> }
                                    >
                                        <Select label="Cursor field" options=cursor_options value=cursor />
                                    </Show>
                                </Show>
                                <Select
                                    label="Write mode"
                                    options=vec!["Append".to_string(), "Upsert".to_string(), "Overwrite".to_string()]
                                    value=write_mode
                                />
                                <Show
                                    when=move || write_mode.get() == "Upsert"
                                    fallback=move || view! {
                                        <TextInput label="Primary key" value=keys mono=true placeholder="id"
                                            description=Signal::derive(|| "Comma-separated, optional.".to_string()) />
                                    }
                                >
                                    <TextInput label="Business keys *" value=keys mono=true placeholder="id"
                                        description=Signal::derive(|| "Comma-separated. An upsert matches on them.".to_string()) />
                                </Show>
                            </div>
                        </Panel>
                        {side_view(destination, "Destination")}
                        <Panel title="Schedule">
                            <div class="app-col">
                                <TextInput label="Cron" value=cron mono=true placeholder="0 * * * *"
                                    description=Signal::derive(|| "Empty: the sync runs only by hand.".to_string()) />
                                <TextInput label="Timezone" value=timezone mono=true />
                                <Switch checked=enabled label="Enabled" />
                            </div>
                        </Panel>
                        <Show when=move || !error.get().is_empty()>
                            <Alert title="Not saved" color=token::BAD>{move || error.get()}</Alert>
                        </Show>
                        <div class="app-row app-row--end">
                            <Button button_type="submit" loading=saving>
                                {move || if editing.get().is_some() { "Save changes" } else { "Create sync" }}
                            </Button>
                        </div>
                    </form>
                </Show>
            </Show>

            <Modal open=new_secret_open title="New secret" footer=secret_footer on_close=Callback::new(move |_| new_secret_for.set(None))>
                <div class="app-col">
                    <TextInput label="Name" value=new_secret_name mono=true />
                    <For
                        each=move || new_secret_fields.get()
                        key=|(k, _)| k.clone()
                        children=|(k, v)| view! { <PasswordInput label=k value=v /> }
                    />
                    <span class="app-hint">"Values are write-only: they are never shown again."</span>
                    <Show when=move || !new_secret_error.get().is_empty()>
                        <span class="app-error">{move || new_secret_error.get()}</span>
                    </Show>
                </div>
            </Modal>
        </div>
    }
}
