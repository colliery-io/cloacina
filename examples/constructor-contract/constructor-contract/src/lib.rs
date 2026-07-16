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

/// One `#[config]` field of a constructor, in DECLARATION order (CLOACI-T-0829) —
/// the order the guest's generated config struct bincode-decodes. The consumer
/// (`constructor!`) binds `config = { name = value }` BY NAME against this, so the
/// kwarg field names — not their written order — determine the bincode layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigField {
    /// The `#[config]` field name (the kwarg key the author writes).
    pub name: String,
    /// The field's Rust type name (e.g. `String`, `i64`, `bool`, `f64`).
    pub ty: String,
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
    /// The constructor's `#[config]` fields in DECLARATION order (CLOACI-T-0829),
    /// so the `constructor!` consumer can bind `config = { name = value }` by NAME.
    #[serde(default)]
    pub config_fields: Vec<ConfigField>,
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

/// The package-level manifest for a **provider** — a *suite* of constructors
/// (CLOACI-A-0011). One provider crate compiles to ONE WASM component that may
/// expose **N constructors**; this manifest (the package's `provider.json`) is the
/// `List[Constructor]` index over them. A consumer selects a member by
/// `constructor = "<name>"`, and the loader carries that name in the `configure`
/// payload. A single-constructor provider is just a suite of one.
/// Which runtime substrate a provider's component loads through (CLOACI-T-0902 /
/// I-0139). Mirrors `cloacina_constructor_contract::ProviderRuntime` — the
/// vendored copy the fixture macros emit against; the macro's
/// `__provider_manifest()` sets `runtime`, so this MUST stay in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRuntime {
    /// Sandboxed WASM component; grants enforced. The default for pre-native
    /// `provider.json` manifests (`#[serde(default)]` below).
    #[default]
    Wasm,
    /// Trusted native cdylib (host target); grants advisory only.
    Native,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderManifest {
    /// Provider name — the `from = "<name>"` a consumer references (CLOACI-A-0010).
    pub name: String,
    /// Provider version (semver string), independent of cloacina's version.
    pub version: String,
    /// The single component filename inside the package implementing every
    /// member constructor (one component per provider; CLOACI-A-0011).
    pub component: String,
    /// Which runtime substrate this provider loads through (CLOACI-T-0902).
    /// `#[serde(default)]` → pre-native `provider.json` deserializes as Wasm.
    #[serde(default)]
    pub runtime: ProviderRuntime,
    /// The member constructors this provider exposes, in declaration order.
    pub constructors: Vec<ConstructorManifest>,
}

impl ProviderManifest {
    /// Look up a member constructor by its `name` (the consumer's
    /// `constructor = "<name>"` selector). `None` if there is no such member.
    pub fn constructor(&self, name: &str) -> Option<&ConstructorManifest> {
        self.constructors.iter().find(|c| c.name == name)
    }

    /// Serialize to the JSON form that travels in the package (`provider.json`).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse the package's `provider.json` back into a provider manifest.
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

// ---------------------------------------------------------------------------
// Object-safe per-kind member traits — the suite dispatch seam (CLOACI-A-0011).
// ---------------------------------------------------------------------------
//
// The boxable, OBJECT-SAFE form of each primitive's single sync WASM method. The
// `#[constructor]`-generated configured type implements one of these (pure serde,
// on every target); the `constructor_provider!` shell holds the selected member as
// a `Box<dyn <Kind>Object>` and dispatches each call to it. fidius-agnostic by
// design — the fidius `#[plugin_impl]` glue lives only in the shell.

/// Object-safe form of the TASK member: `JSON(TaskInvocation)` in → `JSON(TaskOutcome)` out.
pub trait TaskObject: Send + Sync {
    /// Run the configured task member.
    fn execute(&self, invocation_json: String) -> String;
}

/// Object-safe form of the TRIGGER member: `JSON(TriggerInvocation)` in → `JSON(PollOutcome)` out.
pub trait TriggerObject: Send + Sync {
    /// Run the configured trigger member.
    fn poll(&self, invocation_json: String) -> String;
}

/// Object-safe form of the ACCUMULATOR member: `JSON(AccumulatorInvocation)` in →
/// `JSON(AccumulatorOutcome)` out.
pub trait AccumulatorObject: Send + Sync {
    /// Run the configured accumulator member.
    fn ingest(&self, invocation_json: String) -> String;
}

/// Object-safe form of the REACTOR member: `JSON(ReactorInvocation)` in →
/// `JSON(ReactorOutcome)` out.
pub trait ReactorObject: Send + Sync {
    /// Run the configured reactor member.
    fn evaluate(&self, invocation_json: String) -> String;
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

// ---------------------------------------------------------------------------
// ACCUMULATOR primitive — the WASM-boundary wire types (CLOACI-T-0828).
// ---------------------------------------------------------------------------

/// What crosses INTO an accumulator constructor's `ingest`: one raw event as a
/// JSON string (mirrors `cloacina`'s `Accumulator::process(event: Vec<u8>)`).
/// The host event loop calls `ingest` per event and forwards any produced
/// boundary to the reactor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccumulatorInvocation {
    /// Raw event payload as a JSON string (the guest owns deserialization).
    pub event_json: String,
}

/// What crosses OUT of an accumulator constructor's `ingest`: an optional
/// boundary for the reactor (mirrors `Option<Self::Output>`). `boundary_json ==
/// None` means "buffered, no boundary emitted this event".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccumulatorOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AccumulatorOutcome {
    /// Emit a boundary for the reactor.
    pub fn emit(boundary_json: String) -> Self {
        Self {
            boundary_json: Some(boundary_json),
            error: None,
        }
    }

    /// Buffer this event without emitting a boundary.
    pub fn buffered() -> Self {
        Self {
            boundary_json: None,
            error: None,
        }
    }

    /// A failed ingest.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            boundary_json: None,
            error: Some(message.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// REACTOR primitive — the WASM-boundary wire types (CLOACI-T-0828).
// ---------------------------------------------------------------------------

/// What crosses INTO a reactor constructor's `evaluate`: the set of
/// currently-held boundaries as a JSON object keyed by accumulator/source name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorInvocation {
    /// JSON-serialized boundaries keyed by source name.
    pub boundaries_json: String,
}

/// What crosses OUT of a reactor constructor's `evaluate`: whether to fire the
/// downstream computation graph, and the context to fire it with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorOutcome {
    pub fire: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ReactorOutcome {
    /// Fire the graph, optionally with a context JSON.
    pub fn fire(context_json: Option<String>) -> Self {
        Self {
            fire: true,
            context_json,
            error: None,
        }
    }

    /// Do not fire this evaluation.
    pub fn hold() -> Self {
        Self {
            fire: false,
            context_json: None,
            error: None,
        }
    }

    /// A failed evaluate.
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
            config_fields: vec![ConfigField {
                name: "prefix".into(),
                ty: "String".into(),
            }],
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
