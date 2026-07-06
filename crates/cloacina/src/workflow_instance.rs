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
    #[error("serialization: {0}")]
    Serialization(String),
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
    for (k, v) in params {
        if RESERVED_FIRE_KEYS.contains(&k.as_str()) {
            continue;
        }
        // Bound params override any same-named key already in the context
        // (e.g. a trigger-produced payload key) — update-or-insert.
        if context.update(&k, v.clone()).is_err() {
            context
                .insert(k.as_str(), v)
                .map_err(|e| format!("instance param '{}' insert: {}", k, e))?;
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
}
