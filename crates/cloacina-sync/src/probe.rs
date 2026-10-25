/*
 *  Copyright 2025-2026 Colliery Software
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

//! Check and discover on an agent (COLLIERY-I-0635 / CLOACINA-T-1002). A
//! connector runs only where Sync's tasks run, never in the server: the
//! built-in workflow `sync_probe` has one task that checks one side of a sync,
//! or discovers a source's streams, and leaves the answer in its context.
//!
//! A side that names a connector definition is discovered from the
//! definition, which declares its streams and their fields; it is checked by
//! lowering it on its first stream (or object) and running the REST
//! runtime's check.

use chrono::Utc;
use cloacina::{Context, TaskError, task, workflow};
use cloacina_api_types::{
    ConnectorCheckResponse, ConnectorDiscoverResponse, ConnectorProbeRequest,
    ConnectorSpecResponse, DiscoveredField, DiscoveredStream,
};
use cloacina_constructor_contract::sync::{Config, DiscoverOutcome};
use cloacina_declarative::definition::FieldKind;
use serde_json::{Map, Value, json};

use crate::credential::{self, CredentialSpec};
use crate::declarative;
use crate::params::Endpoint;
use crate::workflow::load_endpoint;

/// The name of the built-in probe workflow.
pub const PROBE_WORKFLOW: &str = "sync_probe";

pub mod keys {
    /// `check` or `discover`.
    pub const ACTION: &str = "probe_action";
    /// The side to probe, a JSON `ConnectorProbeRequest`.
    pub const REQUEST: &str = "probe_request";
    /// The answer: a `ConnectorCheckResponse` or a `ConnectorDiscoverResponse`.
    pub const RESULT: &str = "probe.result";
}

/// Why a probe could not answer.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("{0}")]
    Request(String),
    #[error("{0}")]
    Connector(String),
}

/// The context of a probe execution for `action` on `request`.
pub fn context(action: &str, request: &ConnectorProbeRequest) -> Context<Value> {
    let mut ctx = Context::new();
    let _ = ctx.insert(keys::ACTION, json!(action));
    let _ = ctx.insert(
        keys::REQUEST,
        json!(serde_json::to_string(request).unwrap_or_default()),
    );
    ctx
}

/// Check that a request names one side the probe can run, before it is sent.
pub fn validate(action: &str, request: &ConnectorProbeRequest) -> Result<(), ProbeError> {
    let bad = |m: &str| Err(ProbeError::Request(m.to_string()));
    match action {
        "check" | "discover" => {}
        // CLOACINA-T-1006: a compiled connector's own description.
        "spec" if request.definition.is_some() => {
            return bad("a definition's config is in the catalog; spec reads a compiled connector");
        }
        "spec" => {}
        _ => return bad("the action is `check`, `discover` or `spec`"),
    }
    match request.side.as_str() {
        "source" => {}
        "destination" if action == "discover" => return bad("discover reads a source"),
        "destination" => {}
        _ => return bad("`side` is `source` or `destination`"),
    }
    match (&request.definition, &request.provider, &request.member) {
        (Some(_), _, _) => Ok(()),
        (None, Some(p), Some(m)) if !p.trim().is_empty() && !m.trim().is_empty() => Ok(()),
        _ => bad("a side names a `definition`, or a `provider` and a `member`"),
    }
}

fn side_name(request: &ConnectorProbeRequest) -> &'static str {
    if request.side == "destination" {
        "destination"
    } else {
        "source"
    }
}

fn spec(request: &ConnectorProbeRequest) -> Result<Option<CredentialSpec>, ProbeError> {
    request
        .credential
        .clone()
        .map(serde_json::from_value::<CredentialSpec>)
        .transpose()
        .map_err(|e| ProbeError::Request(format!("the credential does not parse: {e}")))
}

fn user_config(request: &ConnectorProbeRequest) -> Result<Map<String, Value>, ProbeError> {
    match &request.config {
        None => Ok(Map::new()),
        Some(Value::Object(o)) => Ok(o.clone()),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(Map::new()),
        Some(Value::String(s)) => serde_json::from_str(s)
            .map_err(|e| ProbeError::Request(format!("the config does not parse: {e}"))),
        Some(_) => Err(ProbeError::Request(
            "a definition's config is a JSON object".into(),
        )),
    }
}

/// The endpoint a request names: a compiled connector, or a definition
/// lowered on its first stream or object.
fn endpoint(request: &ConnectorProbeRequest) -> Result<Endpoint, ProbeError> {
    let side = side_name(request);
    let credential = spec(request)?;
    let Some(def) = &request.definition else {
        let config = match &request.config {
            None => "{}".to_string(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        };
        return Ok(Endpoint {
            provider: request.provider.clone().unwrap_or_default(),
            member: request.member.clone().unwrap_or_default(),
            config,
            grants: request.grants.clone(),
            credential,
        });
    };
    let user = user_config(request)?;
    let defs = declarative::definitions().map_err(ProbeError::Request)?;
    if side == "source" {
        let secret = request
            .secret
            .clone()
            .unwrap_or_else(|| declarative::SOURCE_SECRET_BINDING.to_string());
        let first = defs
            .source(def)
            .map_err(|e| ProbeError::Request(e.to_string()))?
            .streams
            .first()
            .map(|s| s.name.clone())
            .ok_or_else(|| ProbeError::Request(format!("the definition `{def}` has no streams")))?;
        declarative::lower_source(
            def,
            &first,
            &user,
            &secret,
            request.grants.clone(),
            credential,
        )
        .map(|l| l.endpoint)
        .map_err(ProbeError::Request)
    } else {
        let secret = request
            .secret
            .clone()
            .unwrap_or_else(|| declarative::DESTINATION_SECRET_BINDING.to_string());
        let first = defs
            .destination(def)
            .map_err(|e| ProbeError::Request(e.to_string()))?
            .objects
            .first()
            .map(|o| o.name.clone())
            .ok_or_else(|| ProbeError::Request(format!("the definition `{def}` has no objects")))?;
        declarative::lower_destination(
            def,
            &first,
            &user,
            &secret,
            request.grants.clone(),
            credential,
        )
        .map_err(ProbeError::Request)
    }
}

fn field_type(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::Int64 => "Integer",
        FieldKind::Float64 => "Float",
        FieldKind::Utf8 => "Str",
        FieldKind::Bool => "Boolean",
        FieldKind::Timestamp => "Timestamp",
    }
}

/// A definition's streams, as it declares them.
fn discover_definition(name: &str) -> Result<ConnectorDiscoverResponse, ProbeError> {
    let defs = declarative::definitions().map_err(ProbeError::Request)?;
    let def = defs
        .source(name)
        .map_err(|e| ProbeError::Request(e.to_string()))?;
    let streams = def
        .streams
        .iter()
        .map(|s| {
            let mut sync_modes = vec!["FullRefresh".to_string()];
            if s.incremental.is_some() {
                sync_modes.push("Incremental".to_string());
            }
            DiscoveredStream {
                name: s.name.clone(),
                namespace: None,
                sync_modes,
                default_cursor_field: s.incremental.as_ref().map(|i| i.cursor_field.clone()),
                primary_key: (!s.primary_key.is_empty()).then(|| s.primary_key.clone()),
                fields: s
                    .schema
                    .iter()
                    .map(|f| DiscoveredField {
                        name: f.name.clone(),
                        field_type: field_type(f.ty).to_string(),
                        nullable: f.nullable,
                    })
                    .collect(),
            }
        })
        .collect();
    Ok(ConnectorDiscoverResponse { streams })
}

/// Run `action` on the side `request` names, on this process: the probe
/// task's body. A secret is read through the execution's resolver.
pub async fn run(
    ctx: &Context<Value>,
    action: &str,
    request: &ConnectorProbeRequest,
) -> Result<Value, ProbeError> {
    validate(action, request)?;
    if action == "discover"
        && let Some(def) = &request.definition
    {
        return serde_json::to_value(discover_definition(def)?)
            .map_err(|e| ProbeError::Connector(e.to_string()));
    }
    let side = side_name(request);
    let ep = endpoint(request)?;
    let cred = credential::resolve(ctx, side, ep.credential.as_ref())
        .await
        .map_err(|e| ProbeError::Connector(e.to_string()))?;
    let action = action.to_string();
    tokio::task::spawn_blocking(move || -> Result<Value, ProbeError> {
        let handle =
            load_endpoint(side, &ep, cred).map_err(|e| ProbeError::Connector(e.to_string()))?;
        let config = Config {
            json: ep.config.clone(),
        };
        if action == "spec" {
            let s = handle
                .spec()
                .map_err(|e| ProbeError::Connector(e.to_string()))?;
            return serde_json::to_value(ConnectorSpecResponse {
                name: s.name,
                connector_version: s.connector_version,
                config_schema: serde_json::from_str(&s.config_schema)
                    .unwrap_or_else(|_| json!({ "type": "object" })),
                roles: s.roles.iter().map(|r| format!("{r:?}")).collect(),
                sync_modes: s
                    .supported_sync_modes
                    .iter()
                    .map(|m| format!("{m:?}"))
                    .collect(),
            })
            .map_err(|e| ProbeError::Connector(e.to_string()));
        }
        if action == "check" {
            let r = handle
                .check(&config)
                .map_err(|e| ProbeError::Connector(e.to_string()))?;
            return serde_json::to_value(ConnectorCheckResponse {
                ok: r.success,
                message: r.message,
            })
            .map_err(|e| ProbeError::Connector(e.to_string()));
        }
        match handle
            .discover(&config)
            .map_err(|e| ProbeError::Connector(e.to_string()))?
        {
            DiscoverOutcome::Catalog(catalog) => {
                let streams = catalog
                    .streams
                    .into_iter()
                    .map(|s| DiscoveredStream {
                        name: s.name,
                        namespace: s.namespace,
                        sync_modes: s
                            .supported_sync_modes
                            .iter()
                            .map(|m| format!("{m:?}"))
                            .collect(),
                        default_cursor_field: s.default_cursor_field,
                        primary_key: s.source_defined_primary_key,
                        fields: Vec::new(),
                    })
                    .collect();
                serde_json::to_value(ConnectorDiscoverResponse { streams })
                    .map_err(|e| ProbeError::Connector(e.to_string()))
            }
            DiscoverOutcome::Error(e) => Err(ProbeError::Connector(format!(
                "discover failed: {}",
                e.message
            ))),
        }
    })
    .await
    .map_err(|e| ProbeError::Connector(e.to_string()))?
}

/// The built-in probe workflow: one task, never retried, so a wrong secret
/// answers at once.
#[workflow(
    name = "sync_probe",
    description = "Check one side of a sync, or discover a source's streams, where connectors run",
    author = "Colliery Software",
    params(probe_action: String, probe_request: String)
)]
pub mod sync_probe {
    use super::*;

    #[task(id = "probe", dependencies = [], retry_attempts = 1)]
    pub async fn probe(context: &mut Context<Value>) -> Result<(), TaskError> {
        let fail = |message: String| TaskError::ExecutionFailed {
            message,
            task_id: "probe".to_string(),
            timestamp: Utc::now(),
        };
        let action = context
            .get(keys::ACTION)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let request: ConnectorProbeRequest = context
            .get(keys::REQUEST)
            .and_then(Value::as_str)
            .map(serde_json::from_str)
            .transpose()
            .map_err(|e| fail(format!("the probe request does not parse: {e}")))?
            .ok_or_else(|| fail("no probe request".into()))?;
        let answer = run(context, &action, &request)
            .await
            .map_err(|e| fail(e.to_string()))?;
        context.remove(keys::RESULT);
        context.insert(keys::RESULT, answer)?;
        Ok(())
    }
}
