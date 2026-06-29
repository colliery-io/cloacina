// CLOACI-T-0822 constructor contract — shared, language-neutral contract types.
//
// This crate is the cloacina-defined CONSTRUCTOR CONTRACT shared by the host and
// the WASM guest. It carries two distinct things:
//
//   1. The constructor MANIFEST schema ([`ConstructorManifest`] / [`PrimitiveKind`]) —
//      package metadata the macros emit and that the loader (T-0823) reads to
//      learn which cloacina primitive (task | trigger | accumulator | reactor)
//      a component implements, plus its param schema and dependencies. The
//      manifest does NOT cross the per-call WASM boundary; it travels alongside
//      the component in the fidius package.
//
//   2. The per-primitive WASM-BOUNDARY wire types. Constructor methods are
//      SYNCHRONOUS (the guest has no async runtime — T-0821) and everything
//      that crosses the sandbox is serde-serializable. For the TASK primitive
//      that is [`TaskInvocation`] in / [`TaskOutcome`] out. To stay faithful to
//      the de-risked spike (String->String wire) and to cloacina's existing
//      `TaskExecutionRequest`/`TaskExecutionResult` FFI (context carried as a
//      JSON string), the boundary moves a JSON string: `JSON(TaskInvocation)`
//      in, `JSON(TaskOutcome)` out.
//
// NOTE: [`InputSlot`] below is a structural copy of `cloacina_api_types::InputSlot`
// (the CLOACI-I-0128 descriptor). It is vendored here ONLY so this example stays
// self-contained and removable; in the real integration the manifest reuses the
// canonical `cloacina_api_types::InputSlot` verbatim.

use serde::{Deserialize, Serialize};

/// The error an `#[constructor]`-authored body returns (`Result<(), ConstructorError>`).
///
/// Deliberately tiny + serde/wasm-safe: it carries a message the macro-emitted
/// glue stringifies into the failed `TaskOutcome.error`. Authors construct it
/// with [`ConstructorError::msg`] or via `From<String>` / `From<&str>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorError {
    pub message: String,
}

impl ConstructorError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConstructorError {}

impl From<String> for ConstructorError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for ConstructorError {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

/// Which cloacina runtime primitive a WASM constructor implements. The loader
/// (T-0823) switches on this to pick the matching fidius interface descriptor
/// and register the component against the right runtime subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveKind {
    /// `execute(context) -> context` — the unit of work (this first cut).
    Task,
    /// `poll() -> TriggerResult` — a specialized trigger.
    Trigger,
    /// `ingest(event) -> buffer` — accumulates boundary events.
    Accumulator,
    /// `evaluate(criteria) -> fire?` — fires a computation graph.
    Reactor,
}

/// One declared input slot — a named, JSON-Schema-typed value the constructor's
/// runtime surface accepts. Structural copy of `cloacina_api_types::InputSlot`
/// (CLOACI-I-0128); see module docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSlot {
    pub name: String,
    pub schema: serde_json::Value,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

impl InputSlot {
    pub fn required(name: impl Into<String>, schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            schema,
            required: true,
            default: None,
        }
    }

    pub fn optional(
        name: impl Into<String>,
        schema: serde_json::Value,
        default: Option<serde_json::Value>,
    ) -> Self {
        Self {
            name: name.into(),
            schema,
            required: false,
            default,
        }
    }
}

/// The cloacina constructor manifest — emitted by the macros (the same seam that
/// emits packaged-workflow `PackageTasksMetadata`), serialized into the fidius
/// package, and read by the loader (T-0823).
///
/// `interface` / `interface_version` link the manifest to the fidius descriptor
/// the host must load: the descriptor's `interface_export` is what gets linked,
/// `interface_version` must match, and the `fidius-interface-hash` export gates
/// integrity (T-0821). The manifest only tells the loader *which* descriptor +
/// primitive; it is metadata, not a call payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorManifest {
    /// Constructor name (author-given, unique within a package).
    pub name: String,
    /// Constructor version (semver string).
    pub version: String,
    /// Which runtime primitive this constructor plugs into.
    pub primitive_kind: PrimitiveKind,
    /// fidius interface name (kebab) the component exports, e.g. `task-constructor`.
    pub interface: String,
    /// fidius interface version — must match the descriptor's `interface_version`.
    pub interface_version: u32,
    /// Declared param schema (CLOACI-I-0128 `InputSlot`s) for the constructor's
    /// injectable runtime surface (task context keys / trigger config / etc.).
    #[serde(default)]
    pub params: Vec<InputSlot>,
    /// Other constructors/packages this constructor depends on (names).
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Optional human description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

impl ConstructorManifest {
    /// Serialize to the JSON form that travels in the package (sidecar
    /// `constructor.json`). JSON (not TOML) because `InputSlot::schema` is an
    /// arbitrary JSON-Schema fragment.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse the package's `constructor.json` back into a manifest.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

// ---------------------------------------------------------------------------
// TASK primitive — the WASM-boundary wire types (this first cut).
// ---------------------------------------------------------------------------

/// What crosses INTO a task constructor's `execute`: the runtime context as a JSON
/// string (mirrors `cloacina`'s `TaskExecutionRequest.context_json`). The host
/// async wrapper serializes `Context<serde_json::Value>` to this before the
/// blocking WASM call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskInvocation {
    /// JSON-serialized `Context<serde_json::Value>` data map.
    pub context_json: String,
}

/// What crosses OUT of a task constructor's `execute` (mirrors cloacina's
/// `TaskExecutionResult`). The host wrapper rebuilds `Context` from
/// `context_json` on success, or surfaces `error` as a `TaskError`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskOutcome {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl TaskOutcome {
    pub fn ok(context_json: String) -> Self {
        Self {
            success: true,
            context_json: Some(context_json),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            context_json: None,
            error: Some(message.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// TRIGGER primitive — the WASM-boundary wire types (CLOACI-T-0824).
// ---------------------------------------------------------------------------

/// What crosses INTO a trigger constructor's `poll`. The async `Trigger::poll`
/// takes no arguments, but every constructor method is single-arg over the WASM
/// boundary, so the host passes this (currently-empty) envelope.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TriggerInvocation {
    /// Reserved: JSON context from a prior fire, if the host threads it through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_json: Option<String>,
}

/// What crosses OUT of a trigger constructor's `poll`. The host maps this to a
/// `TriggerResult`: `fire` → `Fire(context_json?)`, else `Skip`; `error` →
/// `TriggerError::PollError`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PollOutcome {
    pub fire: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PollOutcome {
    pub fn fire(context_json: Option<String>) -> Self {
        Self {
            fire: true,
            context_json,
            error: None,
        }
    }

    pub fn skip() -> Self {
        Self {
            fire: false,
            context_json: None,
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            fire: false,
            context_json: None,
            error: Some(message.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips_through_json() {
        let m = ConstructorManifest {
            name: "greet".into(),
            version: "0.1.0".into(),
            primitive_kind: PrimitiveKind::Task,
            interface: "task-constructor".into(),
            interface_version: 1,
            params: vec![InputSlot::required(
                "name",
                serde_json::json!({"type": "string"}),
            )],
            dependencies: vec![],
            description: Some("prefixes a name".into()),
            author: None,
        };
        let json = m.to_json().unwrap();
        let back = ConstructorManifest::from_json(&json).unwrap();
        assert_eq!(m, back);
        assert_eq!(back.primitive_kind, PrimitiveKind::Task);
    }

    #[test]
    fn task_wire_round_trips() {
        let inv = TaskInvocation {
            context_json: r#"{"name":"world"}"#.into(),
        };
        let s = serde_json::to_string(&inv).unwrap();
        let back: TaskInvocation = serde_json::from_str(&s).unwrap();
        assert_eq!(inv, back);

        let out = TaskOutcome::ok(r#"{"name":"world","result":"hi world"}"#.into());
        let s = serde_json::to_string(&out).unwrap();
        let back: TaskOutcome = serde_json::from_str(&s).unwrap();
        assert_eq!(out, back);
        assert!(back.success);
    }
}
