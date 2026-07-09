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

//! Parameterized workflow instances (CLOACI-I-0116).
//!
//! A [`WorkflowInstance`] is a *partial*: a clonable, serializable value of
//! `(workflow name + fully-resolved bound parameters)`. It is NOT a captured
//! closure — the same workflow may run in-process, inside a dlopen'd package,
//! or on a remote fleet agent, so what an instance binds must be data that
//! travels with the run (via `Context`). Defaults are snapshotted at
//! [`WorkflowInstanceBuilder::build`]; a registered instance is an immutable
//! snapshot (re-register to adopt new defaults).
//!
//! Params are delivered as FLAT top-level context keys — the same mapping the
//! server's validated execute path uses — with the scheduler's reserved keys
//! (`scheduled_time`, `schedule_id`, `schedule_timezone`,
//! `schedule_expression`, `trigger_name`, `triggered_at`) always winning on
//! conflict so a binding can never spoof them.

use serde::{Deserialize, Serialize};

use cloacina_api_types::InputSlot;

/// Context keys the cron/trigger schedulers own; instance params can never
/// override these (they are stamped after the params merge).
pub const RESERVED_FIRE_KEYS: &[&str] = &[
    "scheduled_time",
    "schedule_id",
    "schedule_timezone",
    "schedule_expression",
    "trigger_name",
    "triggered_at",
];

/// Errors from building or using a workflow instance.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowInstanceError {
    #[error("unknown param '{0}' — the workflow does not declare it")]
    UnknownParam(String),
    #[error("required param '{0}' was not supplied and has no default")]
    MissingParam(String),
    #[error("param '{0}' conflicts with the reserved scheduler key of the same name")]
    ReservedParam(String),
    #[error("param '{name}' failed schema validation: {message}")]
    InvalidParam { name: String, message: String },
    #[error(
        "param '{0}' is declared as an encrypted secret and must be bound with a \
         {{\"$secret\": \"name\"}} reference, not a literal value"
    )]
    SecretRequiresRef(String),
    #[error(
        "param '{0}' is not declared as a secret but was bound with a \
         {{\"$secret\": \"name\"}} reference"
    )]
    UnexpectedSecretRef(String),
    #[error("param '{name}' has a malformed secret reference: {message}")]
    MalformedSecretRef { name: String, message: String },
    #[error("serialization: {0}")]
    Serialization(String),
}

/// The instance-param marker key for a secret reference (CLOACI-I-0133 / T-0859,
/// design D-4): an instance binds a declared encrypted input by giving the param
/// the value `{"$secret": "secret_name"}`.
pub const SECRET_REF_MARKER: &str = "$secret";

/// Classify an instance-param value as a secret reference.
///
/// Returns:
/// - `Ok(Some(secret_name))` when `value` is exactly `{"$secret": "<name>"}`
///   (a single `$secret` key mapping to a non-empty string);
/// - `Ok(None)` for any value that is not a `$secret` marker object (a plain
///   param);
/// - `Err(message)` when the value *looks like* a secret reference but is
///   malformed (`$secret` present but not a lone non-empty string key) — a clear
///   error rather than a silent mis-route.
pub fn secret_ref_target(value: &serde_json::Value) -> Result<Option<String>, String> {
    let serde_json::Value::Object(map) = value else {
        return Ok(None);
    };
    if !map.contains_key(SECRET_REF_MARKER) {
        return Ok(None);
    }
    if map.len() != 1 {
        return Err(format!(
            "a '{}' reference object must contain only the '{}' key",
            SECRET_REF_MARKER, SECRET_REF_MARKER
        ));
    }
    match map.get(SECRET_REF_MARKER) {
        Some(serde_json::Value::String(name)) if !name.is_empty() => Ok(Some(name.clone())),
        _ => Err(format!(
            "'{}' must reference a non-empty secret name string",
            SECRET_REF_MARKER
        )),
    }
}

/// A fully-resolved, immutable, serializable workflow "partial"
/// (CLOACI-I-0116 decision #1). Build via [`WorkflowInstanceBuilder`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    pub workflow_name: String,
    /// Fully-resolved params: every declared param present (defaults
    /// snapshotted at build), validated against the declared slots.
    pub params: serde_json::Map<String, serde_json::Value>,
}

impl WorkflowInstance {
    /// Start building an instance of `workflow_name`.
    pub fn builder(workflow_name: impl Into<String>) -> WorkflowInstanceBuilder {
        WorkflowInstanceBuilder {
            workflow_name: workflow_name.into(),
            supplied: serde_json::Map::new(),
        }
    }

    /// Construct from an ALREADY-RESOLVED param map, trusting the caller
    /// (dynamic surfaces — e.g. the Python bindings — where the declared
    /// slots aren't at hand; the validated path is [`Self::builder`]).
    pub fn from_resolved(
        workflow_name: impl Into<String>,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Self {
        Self {
            workflow_name: workflow_name.into(),
            params,
        }
    }

    /// The instance's params as a `Context`-ready JSON object string
    /// (persisted on the schedule row at register time).
    pub fn params_json(&self) -> Result<String, WorkflowInstanceError> {
        serde_json::to_string(&self.params)
            .map_err(|e| WorkflowInstanceError::Serialization(e.to_string()))
    }
}

/// Builder: `.param(k, v)` binds values; [`build`](Self::build) validates
/// against the workflow's declared input slots and snapshots defaults.
pub struct WorkflowInstanceBuilder {
    workflow_name: String,
    supplied: serde_json::Map<String, serde_json::Value>,
}

impl WorkflowInstanceBuilder {
    /// Bind a param value. Values must be serde-serializable data — you can
    /// bind a path, a mode, an ID; not a live handle (decision #1).
    pub fn param(
        mut self,
        name: impl Into<String>,
        value: impl Serialize,
    ) -> Result<Self, WorkflowInstanceError> {
        let name = name.into();
        let value = serde_json::to_value(value)
            .map_err(|e| WorkflowInstanceError::Serialization(e.to_string()))?;
        self.supplied.insert(name, value);
        Ok(self)
    }

    /// Validate against the workflow's declared input slots and produce the
    /// fully-resolved instance:
    /// - unknown supplied param → error
    /// - required (no default) and omitted → error
    /// - omitted with default → default snapshotted NOW (decision #3)
    /// - param named like a reserved scheduler key → error
    pub fn build(self, declared: &[InputSlot]) -> Result<WorkflowInstance, WorkflowInstanceError> {
        for name in self.supplied.keys() {
            if !declared.iter().any(|s| &s.name == name) {
                return Err(WorkflowInstanceError::UnknownParam(name.clone()));
            }
            if RESERVED_FIRE_KEYS.contains(&name.as_str()) {
                return Err(WorkflowInstanceError::ReservedParam(name.clone()));
            }
        }

        let mut resolved = serde_json::Map::new();
        for slot in declared {
            if RESERVED_FIRE_KEYS.contains(&slot.name.as_str()) {
                return Err(WorkflowInstanceError::ReservedParam(slot.name.clone()));
            }
            match self.supplied.get(&slot.name) {
                Some(v) => {
                    // CLOACI-T-0859: an encrypted slot must be bound with a
                    // `{"$secret": name}` reference (never a literal value, which
                    // would leak into the plaintext context); a plaintext slot
                    // must NOT be bound with a secret reference.
                    let is_ref = secret_ref_target(v)
                        .map_err(|message| WorkflowInstanceError::MalformedSecretRef {
                            name: slot.name.clone(),
                            message,
                        })?
                        .is_some();
                    if slot.encrypted && !is_ref {
                        return Err(WorkflowInstanceError::SecretRequiresRef(slot.name.clone()));
                    }
                    if !slot.encrypted && is_ref {
                        return Err(WorkflowInstanceError::UnexpectedSecretRef(
                            slot.name.clone(),
                        ));
                    }
                    resolved.insert(slot.name.clone(), v.clone());
                }
                None => match &slot.default {
                    Some(d) => {
                        resolved.insert(slot.name.clone(), d.clone());
                    }
                    None if slot.required => {
                        return Err(WorkflowInstanceError::MissingParam(slot.name.clone()));
                    }
                    None => {}
                },
            }
        }

        Ok(WorkflowInstance {
            workflow_name: self.workflow_name,
            params: resolved,
        })
    }
}

/// Merge a schedule row's stored instance params into a fire context as flat
/// top-level keys, SKIPPING the reserved scheduler keys (reserved always
/// wins) and — via `Context::update` semantics on the caller side — letting
/// bound params override a trigger-produced payload (OQ-3). Shared by the
/// cron and trigger fire paths.
pub fn merge_instance_params(
    context: &mut crate::Context<serde_json::Value>,
    params_json: &str,
) -> Result<(), String> {
    let params: serde_json::Map<String, serde_json::Value> = serde_json::from_str(params_json)
        .map_err(|e| format!("instance params JSON parse: {}", e))?;

    // CLOACI-T-0859: `{"$secret": name}` bindings are routed AWAY from the
    // plaintext context. We accumulate only the non-sensitive
    // `local_binding_name -> secret_name` alias here (NAMES only, never values);
    // the resolved secret value never touches the context (NFR-001). The alias
    // map is stored under the reserved `SECRET_REFS_KEY` so the T-0858 accessor
    // can map a task's declared local name to the concrete secret at fire time.
    let mut secret_refs = serde_json::Map::new();

    for (k, v) in params {
        if RESERVED_FIRE_KEYS.contains(&k.as_str()) {
            continue;
        }
        if k == cloacina_workflow::secret::SECRET_REFS_KEY {
            return Err(format!(
                "instance param '{}' collides with the reserved secret-reference key",
                k
            ));
        }

        match secret_ref_target(&v).map_err(|m| {
            format!(
                "instance param '{}' has a malformed secret reference: {}",
                k, m
            )
        })? {
            // A `$secret` reference: record the alias, keep the value out of the
            // plaintext context entirely.
            Some(secret_name) => {
                secret_refs.insert(k, serde_json::Value::String(secret_name));
            }
            // A plain param: merged as before. Bound params override any
            // same-named key already in the context (e.g. a trigger-produced
            // payload key) — update-or-insert.
            None => {
                if context.update(&k, v.clone()).is_err() {
                    context
                        .insert(k.as_str(), v)
                        .map_err(|e| format!("instance param '{}' insert: {}", k, e))?;
                }
            }
        }
    }

    if !secret_refs.is_empty() {
        let alias_map = serde_json::Value::Object(secret_refs);
        let key = cloacina_workflow::secret::SECRET_REFS_KEY;
        if context.update(key, alias_map.clone()).is_err() {
            context
                .insert(key, alias_map)
                .map_err(|e| format!("secret-reference map insert: {}", e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(name: &str, required: bool, default: Option<serde_json::Value>) -> InputSlot {
        InputSlot {
            name: name.to_string(),
            required,
            default,
            schema: serde_json::json!({"type": "string"}),
            encrypted: false,
        }
    }

    #[test]
    fn build_resolves_defaults_and_validates() {
        let declared = vec![
            slot("source", true, None),
            slot("mode", false, Some(serde_json::json!("copy"))),
        ];

        // required omitted → error
        let err = WorkflowInstance::builder("sync")
            .build(&declared)
            .unwrap_err();
        assert!(matches!(err, WorkflowInstanceError::MissingParam(_)));

        // unknown → error
        let err = WorkflowInstance::builder("sync")
            .param("nope", "x")
            .unwrap()
            .build(&declared)
            .unwrap_err();
        assert!(matches!(err, WorkflowInstanceError::UnknownParam(_)));

        // defaults snapshot
        let inst = WorkflowInstance::builder("sync")
            .param("source", "/a")
            .unwrap()
            .build(&declared)
            .unwrap();
        assert_eq!(inst.params["source"], serde_json::json!("/a"));
        assert_eq!(inst.params["mode"], serde_json::json!("copy"));

        // serde round-trip (REQ-004)
        let json = serde_json::to_string(&inst).unwrap();
        let back: WorkflowInstance = serde_json::from_str(&json).unwrap();
        assert_eq!(back.params, inst.params);
    }

    #[test]
    fn merge_skips_reserved_and_overrides_payload() {
        let mut ctx: crate::Context<serde_json::Value> = crate::Context::new();
        ctx.insert("from_trigger", serde_json::json!("payload"))
            .unwrap();

        let params = serde_json::json!({
            "source": "/a",
            "from_trigger": "bound-wins",
            "scheduled_time": "spoof-attempt"
        })
        .to_string();
        merge_instance_params(&mut ctx, &params).unwrap();

        assert_eq!(ctx.get("source").cloned().unwrap(), serde_json::json!("/a"));
        // bound param overrides trigger payload (OQ-3)
        assert_eq!(
            ctx.get("from_trigger").cloned().unwrap(),
            serde_json::json!("bound-wins")
        );
        // reserved key skipped — cannot be spoofed by a binding
        assert!(ctx.get("scheduled_time").is_none());
    }

    // ── CLOACI-T-0859: `$secret` reference routing ───────────────────────────

    fn secret_slot(name: &str) -> InputSlot {
        InputSlot::secret(name)
    }

    #[test]
    fn secret_ref_target_classifies_values() {
        // A well-formed reference.
        assert_eq!(
            secret_ref_target(&serde_json::json!({"$secret": "db_prod"})).unwrap(),
            Some("db_prod".to_string())
        );
        // Plain values are not references.
        assert_eq!(
            secret_ref_target(&serde_json::json!("plain")).unwrap(),
            None
        );
        assert_eq!(
            secret_ref_target(&serde_json::json!({"host": "x"})).unwrap(),
            None
        );
        // Malformed: extra keys, or non-string / empty target.
        assert!(secret_ref_target(&serde_json::json!({"$secret": "a", "x": 1})).is_err());
        assert!(secret_ref_target(&serde_json::json!({"$secret": 5})).is_err());
        assert!(secret_ref_target(&serde_json::json!({"$secret": ""})).is_err());
    }

    #[test]
    fn merge_routes_secret_refs_away_from_plaintext_context() {
        let mut ctx: crate::Context<serde_json::Value> = crate::Context::new();
        let params = serde_json::json!({
            "region": "us-east-1",
            "dst_credentials": {"$secret": "s3_prod"}
        })
        .to_string();
        merge_instance_params(&mut ctx, &params).unwrap();

        // The plain param is merged normally.
        assert_eq!(
            ctx.get("region").cloned().unwrap(),
            serde_json::json!("us-east-1")
        );
        // The `$secret` marker is NOT stored under its own param key.
        assert!(ctx.get("dst_credentials").is_none());
        // Only the non-sensitive NAME→NAME alias is recorded (never the value).
        let refs = ctx
            .get(cloacina_workflow::secret::SECRET_REFS_KEY)
            .cloned()
            .unwrap();
        assert_eq!(refs, serde_json::json!({"dst_credentials": "s3_prod"}));

        // The serialized context carries no secret VALUE — only names.
        let json = ctx.to_json().unwrap();
        assert!(!json.contains("$secret"));
        assert!(json.contains("s3_prod")); // the alias target (a name) is fine
    }

    #[test]
    fn merge_rejects_malformed_secret_ref_and_reserved_key() {
        let mut ctx: crate::Context<serde_json::Value> = crate::Context::new();
        let bad = serde_json::json!({"cred": {"$secret": 3}}).to_string();
        assert!(merge_instance_params(&mut ctx, &bad).is_err());

        let mut ctx2: crate::Context<serde_json::Value> = crate::Context::new();
        let reserved = serde_json::json!({
            cloacina_workflow::secret::SECRET_REFS_KEY: {"x": "y"}
        })
        .to_string();
        assert!(merge_instance_params(&mut ctx2, &reserved).is_err());
    }

    #[test]
    fn build_requires_secret_slot_bound_with_ref() {
        let declared = vec![slot("source", true, None), secret_slot("db")];

        // Encrypted slot left unbound → MissingParam (declared-but-unbound).
        let err = WorkflowInstance::builder("sync")
            .param("source", "/a")
            .unwrap()
            .build(&declared)
            .unwrap_err();
        assert!(matches!(err, WorkflowInstanceError::MissingParam(_)));

        // Encrypted slot bound with a literal value → SecretRequiresRef.
        let err = WorkflowInstance::builder("sync")
            .param("source", "/a")
            .unwrap()
            .param("db", "plaintext-password")
            .unwrap()
            .build(&declared)
            .unwrap_err();
        assert!(matches!(err, WorkflowInstanceError::SecretRequiresRef(_)));

        // Encrypted slot bound with a `$secret` ref → OK; the marker rides in params.
        let inst = WorkflowInstance::builder("sync")
            .param("source", "/a")
            .unwrap()
            .param("db", serde_json::json!({"$secret": "db_prod"}))
            .unwrap()
            .build(&declared)
            .unwrap();
        assert_eq!(inst.params["db"], serde_json::json!({"$secret": "db_prod"}));
    }

    #[test]
    fn build_rejects_secret_ref_on_plaintext_slot() {
        let declared = vec![slot("mode", true, None)];
        let err = WorkflowInstance::builder("sync")
            .param("mode", serde_json::json!({"$secret": "leak"}))
            .unwrap()
            .build(&declared)
            .unwrap_err();
        assert!(matches!(err, WorkflowInstanceError::UnexpectedSecretRef(_)));
    }
}
