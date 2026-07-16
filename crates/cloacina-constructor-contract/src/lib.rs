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

//! # Constructor contract (CLOACI-I-0132)
//!
//! The cloacina-defined CONSTRUCTOR CONTRACT shared by the host loader
//! ([`cloacina`]'s `constructors-wasm` path, CLOACI-T-0823) and a WASM guest
//! constructor. It carries two distinct things:
//!
//! 1. **The constructor MANIFEST schema** ([`ConstructorManifest`] / [`PrimitiveKind`])
//!    — package metadata the macros emit (CLOACI-T-0826) and that the loader
//!    reads to learn which cloacina primitive (task | trigger | accumulator |
//!    reactor) a component implements, plus its param schema and dependencies.
//!    The manifest does NOT cross the per-call WASM boundary; it travels
//!    alongside the component in the fidius package as a sidecar `constructor.json`.
//!
//! 2. **The per-primitive WASM-BOUNDARY wire types.** Constructor methods are
//!    SYNCHRONOUS (the guest has no async runtime — CLOACI-T-0821) and everything
//!    that crosses the sandbox is serde-serializable. For the TASK primitive that
//!    is [`TaskInvocation`] in / [`TaskOutcome`] out. To stay faithful to the
//!    de-risked spike (String wire) and to cloacina's existing
//!    `TaskExecutionRequest`/`TaskExecutionResult` FFI (context carried as a JSON
//!    string), the boundary moves a JSON string: `JSON(TaskInvocation)` in,
//!    `JSON(TaskOutcome)` out.
//!
//! ## The `TaskConstructor` sync trait shape
//!
//! The TASK primitive's WASM interface is a single SYNC method — the
//! WASM-compatible analogue of the async `cloacina_workflow::Task::execute`:
//!
//! ```rust,ignore
//! // guest declares with crate = "fidius_guest"; host loader re-declares the
//! // identical trait with crate = "fidius_core" to obtain the matching
//! // `TaskConstructor_WASM_DESCRIPTOR` (CLOACI-T-0821).
//! #[plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_guest")]
//! pub trait TaskConstructor: Send + Sync {
//!     /// `JSON(TaskInvocation)` in -> `JSON(TaskOutcome)` out. SYNC.
//!     fn execute(&self, invocation_json: String) -> String;
//! }
//! ```
//!
//! The `#[plugin_interface]` declaration itself lives wherever a fidius `crate =`
//! target is bound (guest crate / host loader); it cannot live here because this
//! crate is fidius-agnostic so it stays wasm-buildable from serde alone. The
//! method index for `execute` is `0` ([`METHOD_EXECUTE`]).
//!
//! Sibling primitives (trigger `poll`, accumulator `ingest`, reactor `evaluate`)
//! are all single-arg JSON-String-in/JSON-String-out, sync. CLOACI-T-0824 lands
//! the TRIGGER primitive end-to-end ([`TriggerInvocation`] in / [`PollOutcome`]
//! out, bridged to [`cloacina_workflow::Trigger`] by the host loader) plus the
//! Runtime-registry wiring, and defines the ACCUMULATOR
//! ([`AccumulatorInvocation`] / [`AccumulatorOutcome`]) and REACTOR
//! ([`ReactorInvocation`] / [`ReactorOutcome`]) wire types whose full host
//! bridge/fixture is a noted continuation.

use serde::{Deserialize, Serialize};

/// Re-export of the canonical CLOACI-I-0128 input-slot descriptor. The constructor
/// manifest's `params` reuse this verbatim (NOT a vendored copy), so an
/// constructor's injectable runtime surface is described with the same
/// JSON-Schema-typed slots the rest of cloacina uses.
pub use cloacina_api_types::InputSlot;

/// The error an `#[constructor]`-authored body returns (`Result<(), ConstructorError>`,
/// CLOACI-T-0826). Deliberately tiny + serde/wasm-safe: it carries a message the
/// macro-emitted glue stringifies into the failed `TaskOutcome.error`. Authors
/// construct it with [`ConstructorError::msg`] or via `From<String>` / `From<&str>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorError {
    pub message: String,
}

impl ConstructorError {
    /// Build an error from a message.
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

/// The vtable method index of `TaskConstructor::execute` (the single sync method).
pub const METHOD_EXECUTE: usize = 0;

/// The vtable method index of `TriggerConstructor::poll` (the single sync method).
pub const METHOD_POLL: usize = 0;

/// The vtable method index of `AccumulatorConstructor::ingest` (CLOACI-T-0824
/// continuation — wire type defined, bridge sketched).
pub const METHOD_INGEST: usize = 0;

/// The vtable method index of `ReactorConstructor::evaluate` (CLOACI-T-0824
/// continuation — wire type defined, bridge sketched).
pub const METHOD_EVALUATE: usize = 0;

/// The vtable method index of `StreamAccumulatorConstructor::source` — the single
/// SERVER-STREAMING method of a stream (loop-owning) accumulator (CLOACI-T-0904).
/// Its own one-method interface, so index `0`.
pub const METHOD_SOURCE: usize = 0;

/// The fidius interface version of the TASK-constructor contract. Must match the
/// `version` passed to the guest/host `#[plugin_interface(version = ..)]` and is
/// cross-checked against [`ConstructorManifest::interface_version`] by the loader.
pub const TASK_CONSTRUCTOR_INTERFACE_VERSION: u32 = 1;

/// The fidius interface version of the TRIGGER-constructor contract. Cross-checked
/// against [`ConstructorManifest::interface_version`] for trigger primitives.
pub const TRIGGER_CONSTRUCTOR_INTERFACE_VERSION: u32 = 1;

/// The fidius interface version of the ACCUMULATOR-constructor contract
/// (continuation — see [`AccumulatorInvocation`]).
pub const ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION: u32 = 1;

/// The fidius interface version of the REACTOR-constructor contract
/// (continuation — see [`ReactorInvocation`]).
pub const REACTOR_CONSTRUCTOR_INTERFACE_VERSION: u32 = 1;

/// The fidius interface version of the STREAM-accumulator contract (CLOACI-T-0904).
/// A distinct one-method (`source`) server-streaming interface — separate from the
/// per-event `AccumulatorConstructor` (`ingest`) so a stream accumulator does not
/// force a version bump on existing per-event accumulators. The member's
/// `ConstructorManifest.interface` is `"stream-accumulator-constructor"`.
pub const STREAM_ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION: u32 = 1;

/// The `interface` string a stream accumulator member's `ConstructorManifest`
/// carries — the loader switches on it to take the `call_streaming` path instead
/// of the per-event `ingest` path (CLOACI-T-0904).
pub const STREAM_ACCUMULATOR_INTERFACE: &str = "stream-accumulator-constructor";

/// Which cloacina runtime primitive a WASM constructor implements. The loader
/// (CLOACI-T-0823 for `Task`; the rest land in T-0824) switches on this to pick
/// the matching fidius interface descriptor and register the component against
/// the right runtime subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveKind {
    /// `execute(context) -> context` — the unit of work (implemented here).
    Task,
    /// `poll() -> TriggerResult` — a specialized trigger (T-0824).
    Trigger,
    /// `ingest(event) -> buffer` — accumulates boundary events (T-0824).
    Accumulator,
    /// `evaluate(criteria) -> fire?` — fires a computation graph (T-0824).
    Reactor,
}

/// One `#[config]` field of a constructor, recorded in DECLARATION order — the
/// order the guest's generated config struct bincode-decodes (CLOACI-T-0829).
///
/// fidius binds config via bincode (positional, NOT self-describing), so the
/// consumer (`constructor!(config = { name = value })`) cannot blindly serialize
/// the author's values in WRITTEN order — it must reorder them into the guest's
/// declaration order first. This carries that order plus each field's Rust type
/// name so the consumer can encode each kwarg value AS the concrete type the
/// guest expects, giving `config = { … }` true kwarg (name-keyed) semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigField {
    /// The `#[config]` field name — the kwarg key a `constructor!` author writes.
    pub name: String,
    /// The field's Rust type name (last path segment), e.g. `String`, `i64`,
    /// `bool`, `f64`. Used by the consumer to bincode-encode the kwarg value as
    /// the concrete type the guest's config struct decodes.
    pub ty: String,
}

/// The cloacina constructor manifest — emitted by the macros (the same seam that
/// emits packaged-workflow `PackageTasksMetadata`, CLOACI-T-0826), serialized
/// into the fidius package as a sidecar `constructor.json`, and read by the loader.
///
/// `interface` / `interface_version` link the manifest to the fidius descriptor
/// the host must load: the descriptor's `interface_export` is what gets linked,
/// `interface_version` must match, and the `fidius-interface-hash` export gates
/// integrity (CLOACI-T-0821). The manifest only tells the loader *which*
/// descriptor + primitive; it is metadata, not a call payload.
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
    /// fidius interface version — must match the descriptor's interface version.
    pub interface_version: u32,
    /// Declared param schema (CLOACI-I-0128 [`InputSlot`]s) for the constructor's
    /// injectable runtime surface (task context keys / trigger config / etc.).
    #[serde(default)]
    pub params: Vec<InputSlot>,
    /// The constructor's `#[config]` fields in DECLARATION order (CLOACI-T-0829).
    /// The `constructor!` consumer binds `config = { name = value }` BY NAME
    /// against this list, reordering the author's values into declaration order
    /// before bincode-serializing the config tuple fidius binds at load. Empty
    /// for constructors with no `#[config]` fields.
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

/// Which runtime substrate a provider's component is loaded through
/// (CLOACI-T-0902 / I-0139). The whole suite shares one runtime — a provider is
/// authored the same way regardless (the `#[constructor]` surface is
/// target-agnostic); only the emitted glue + load path differ.
///
/// **Trust asymmetry (I-0139 sub-question (e)):** WASM providers are sandboxed
/// (wasmtime) and their `grants` are ENFORCED by the host capability layer.
/// NATIVE providers run in-process with full host trust — the same trust surface
/// a tenant already extends to a packaged Rust workflow cdylib — so `grants`
/// degrade to ADVISORY/audit metadata (a native provider opens its sockets/files
/// directly regardless of a grant). The discriminator makes this explicit at
/// author + install time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRuntime {
    /// Sandboxed WASM component (`wasm32-wasip2`), loaded via
    /// `load_wasm_configured`; `grants` enforced. The default for pre-native
    /// `provider.json` manifests.
    #[default]
    Wasm,
    /// Trusted native cdylib (host target), loaded via `load_library` +
    /// `PluginHandle::configure_in_process`; `grants` advisory only.
    Native,
}

impl ProviderRuntime {
    /// Whether this runtime's `grants` are enforced (WASM sandbox) vs. advisory
    /// (native, trusted in-process). Consumed by the loader (skip grant
    /// translation for native) and by author/CLI trust-tier surfacing.
    pub fn grants_enforced(self) -> bool {
        matches!(self, ProviderRuntime::Wasm)
    }
}

/// The package-level manifest for a **provider** — a *suite* of constructors
/// (CLOACI-A-0011). One provider crate compiles to ONE component that may
/// expose **N constructors**; this manifest (the package's `provider.json`) is the
/// `List[Constructor]` index over them. A consumer selects a member by
/// `constructor = "<name>"`, and the loader carries that name in the `configure`
/// payload (so all members share the one per-kind fidius interface — no per-member
/// interface, no fidius change). A single-constructor provider is just a suite of
/// one.
///
/// `Constructor` is [`ConstructorManifest`] (the per-member descriptor); the member
/// list replaces the older per-constructor `constructor.json` sidecars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderManifest {
    /// Provider name — the Cargo package name (the `from = "<name>"` a consumer
    /// references; CLOACI-A-0010).
    pub name: String,
    /// Provider version (semver string), independent of cloacina's version.
    pub version: String,
    /// The single component filename inside the package that implements every
    /// member constructor (one component per provider; CLOACI-A-0011). A `.wasm`
    /// component for `runtime = wasm`; a native cdylib (`.so`/`.dylib`) for
    /// `runtime = native` (per-arch artifact selection is CLOACI-T-0905).
    pub component: String,
    /// Which runtime substrate this provider loads through (CLOACI-T-0902). The
    /// whole suite shares one runtime. Defaults to [`ProviderRuntime::Wasm`] so
    /// pre-native `provider.json` manifests deserialize unchanged.
    #[serde(default)]
    pub runtime: ProviderRuntime,
    /// The member constructors this provider exposes, in declaration order.
    pub constructors: Vec<ConstructorManifest>,
}

impl ProviderManifest {
    /// Look up a member constructor by its `name` (the consumer's
    /// `constructor = "<name>"` selector). `None` if this provider has no such
    /// member.
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
// A provider is a SUITE: one WASM component exposes N member constructors behind
// ONE per-kind fidius interface, with the member chosen by name carried in the
// `configure` payload. The crate-level `constructor_provider!` shell holds the
// selected member as a `Box<dyn <Kind>Object>` and dispatches each call to it.
//
// These traits are the boxable, OBJECT-SAFE form of each primitive's single sync
// WASM method (`JSON(...Invocation)` in → `JSON(...Outcome)` out). They are plain
// Rust — deliberately fidius-agnostic — so the `#[constructor]`-generated
// `__<Struct>Configured` type implements one of them on every target (the impl is
// pure serde) while the fidius `#[plugin_impl]` glue lives only in the shell. The
// method names match the per-kind fidius interface (`execute`/`poll`/`ingest`/
// `evaluate`) but carry no `#[plugin_interface]` attribute — that is the shell's
// job. A single-constructor provider is just a suite of one.

/// Object-safe form of the TASK member: `JSON(TaskInvocation)` in →
/// `JSON(TaskOutcome)` out. Implemented by the macro-generated configured type;
/// held by the provider shell as `Box<dyn TaskObject>`.
pub trait TaskObject: Send + Sync {
    /// Run the configured task member. `JSON(TaskInvocation)` in, `JSON(TaskOutcome)` out.
    fn execute(&self, invocation_json: String) -> String;
}

/// Object-safe form of the TRIGGER member: `JSON(TriggerInvocation)` in →
/// `JSON(PollOutcome)` out.
pub trait TriggerObject: Send + Sync {
    /// Run the configured trigger member. `JSON(TriggerInvocation)` in, `JSON(PollOutcome)` out.
    fn poll(&self, invocation_json: String) -> String;
}

/// Object-safe form of the ACCUMULATOR member: `JSON(AccumulatorInvocation)` in →
/// `JSON(AccumulatorOutcome)` out.
pub trait AccumulatorObject: Send + Sync {
    /// Run the configured accumulator member. `JSON(AccumulatorInvocation)` in,
    /// `JSON(AccumulatorOutcome)` out.
    fn ingest(&self, invocation_json: String) -> String;
}

/// Object-safe form of a STREAM (loop-owning) accumulator member (CLOACI-T-0904):
/// instead of a per-event `ingest`, the configured member produces the WHOLE
/// stream of boundary events at once. Each yielded `String` is one boundary JSON
/// (the same shape `AccumulatorOutcome.boundary_json` carries per event).
///
/// The suite shell's server-streaming `source` method calls this and wraps the
/// returned iterator in a `fidius::Stream<String>`; the host drains it via
/// `call_streaming` → `ChunkStream`. The iterator is `Send + 'static` because it
/// crosses to the host stream bridge and outlives the borrow of `&self` — a real
/// source (e.g. a Kafka consumer) owns what it needs, built from the bound config.
pub trait StreamAccumulatorObject: Send + Sync {
    /// Start the configured stream source, yielding boundary-JSON items lazily.
    ///
    /// KEEPALIVE convention (CLOACI-T-0906): an **empty string** item is a
    /// liveness tick, not a boundary — the host skips it. A source whose `next()`
    /// blocks on an external poll (e.g. Kafka) should yield `""` on each poll
    /// timeout so the host's pump thread gets a periodic send and teardown of an
    /// idle stream resolves within one poll interval instead of parking forever.
    fn source(&self) -> Box<dyn Iterator<Item = String> + Send>;
}

/// Object-safe form of the REACTOR member: `JSON(ReactorInvocation)` in →
/// `JSON(ReactorOutcome)` out.
pub trait ReactorObject: Send + Sync {
    /// Run the configured reactor member. `JSON(ReactorInvocation)` in,
    /// `JSON(ReactorOutcome)` out.
    fn evaluate(&self, invocation_json: String) -> String;
}

// ---------------------------------------------------------------------------
// TASK primitive — the WASM-boundary wire types.
// ---------------------------------------------------------------------------

/// What crosses INTO a task constructor's `execute`: the runtime context as a JSON
/// string (mirrors `cloacina`'s `TaskExecutionRequest.context_json`). The host
/// async bridge serializes `Context<serde_json::Value>` to this before the
/// blocking WASM call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskInvocation {
    /// JSON-serialized `Context<serde_json::Value>` data map.
    pub context_json: String,
}

/// What crosses OUT of a task constructor's `execute` (mirrors cloacina's
/// `TaskExecutionResult`). The host bridge rebuilds `Context` from
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
    /// A successful outcome carrying the updated context JSON.
    pub fn ok(context_json: String) -> Self {
        Self {
            success: true,
            context_json: Some(context_json),
            error: None,
        }
    }

    /// A failed outcome carrying an error message (surfaced as a `TaskError`).
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
/// boundary, so the host passes this (currently-empty) envelope. `context_json`
/// is reserved so a future host can feed the last-fired context back into a
/// poll without a wire break.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TriggerInvocation {
    /// Reserved: JSON-serialized context from a prior fire, if the host chooses
    /// to thread it through. Empty for all current poll calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_json: Option<String>,
}

/// What crosses OUT of a trigger constructor's `poll`. The host bridge maps this to
/// a [`cloacina_workflow::TriggerResult`]: `fire == true` →
/// `Fire(context_json?)`, `fire == false` → `Skip`. A populated `error` is
/// surfaced as a `TriggerError::PollError` (polling then continues next tick).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PollOutcome {
    /// Whether the workflow should fire this tick.
    pub fire: bool,
    /// Optional JSON-serialized `Context` to fire with (only meaningful when
    /// `fire`). `None` fires with no context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_json: Option<String>,
    /// Optional poll error; surfaced as `TriggerError::PollError` by the host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PollOutcome {
    /// Fire the workflow, optionally carrying a context JSON.
    pub fn fire(context_json: Option<String>) -> Self {
        Self {
            fire: true,
            context_json,
            error: None,
        }
    }

    /// Skip this tick (keep polling).
    pub fn skip() -> Self {
        Self {
            fire: false,
            context_json: None,
            error: None,
        }
    }

    /// A failed poll, surfaced host-side as `TriggerError::PollError`.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            fire: false,
            context_json: None,
            error: Some(message.into()),
        }
    }
}

// ---------------------------------------------------------------------------
// ACCUMULATOR primitive — wire types (CLOACI-T-0824 continuation).
// ---------------------------------------------------------------------------

/// What crosses INTO an accumulator constructor's `ingest`: one raw event. Mirrors
/// `cloacina`'s `Accumulator::process(event: Vec<u8>) -> Option<Output>`, except
/// the bytes are base64/JSON-string-carried to keep the boundary a JSON String
/// like the other primitives. The host event loop calls `ingest` per event and
/// forwards any produced boundary to the reactor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccumulatorInvocation {
    /// Raw event payload as a JSON string (the guest owns deserialization, the
    /// runtime is format-agnostic — mirrors the native `Accumulator` contract).
    pub event_json: String,
}

/// What crosses OUT of an accumulator constructor's `ingest`: an optional boundary
/// for the reactor (mirrors `Option<Self::Output>`). `boundary_json == None`
/// means "buffered, no boundary emitted this event".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccumulatorOutcome {
    /// JSON-serialized boundary to forward to the reactor, if one was produced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundary_json: Option<String>,
    /// Optional ingest error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// REACTOR primitive — wire types (CLOACI-T-0824 continuation).
// ---------------------------------------------------------------------------

/// What crosses INTO a reactor constructor's `evaluate`: the set of currently-held
/// boundaries (one JSON blob per named accumulator slot). Mirrors the reactor's
/// firing-criteria evaluation over its accumulator set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorInvocation {
    /// JSON-serialized boundaries keyed by accumulator name.
    pub boundaries_json: String,
}

/// What crosses OUT of a reactor constructor's `evaluate`: whether to fire the
/// downstream computation graph, and the context to fire it with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorOutcome {
    /// Whether the reactor's firing criteria are satisfied.
    pub fire: bool,
    /// JSON-serialized context to fire the graph with (only when `fire`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_json: Option<String>,
    /// Optional evaluate error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
            interface_version: TASK_CONSTRUCTOR_INTERFACE_VERSION,
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
        assert_eq!(back.params[0].name, "name");
        assert_eq!(back.config_fields[0].name, "prefix");
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

        let err = TaskOutcome::err("boom");
        assert!(!err.success);
        assert_eq!(err.error.as_deref(), Some("boom"));
    }

    #[test]
    fn trigger_wire_round_trips() {
        let inv = TriggerInvocation::default();
        let s = serde_json::to_string(&inv).unwrap();
        let back: TriggerInvocation = serde_json::from_str(&s).unwrap();
        assert_eq!(inv, back);

        let fire = PollOutcome::fire(Some(r#"{"hit":true}"#.into()));
        let s = serde_json::to_string(&fire).unwrap();
        let back: PollOutcome = serde_json::from_str(&s).unwrap();
        assert_eq!(fire, back);
        assert!(back.fire);
        assert_eq!(back.context_json.as_deref(), Some(r#"{"hit":true}"#));

        let skip = PollOutcome::skip();
        assert!(!skip.fire);
        assert!(skip.context_json.is_none());

        let err = PollOutcome::err("poll boom");
        assert!(!err.fire);
        assert_eq!(err.error.as_deref(), Some("poll boom"));
    }

    #[test]
    fn accumulator_and_reactor_wire_round_trip() {
        let acc = AccumulatorOutcome {
            boundary_json: Some(r#"{"sum":3}"#.into()),
            error: None,
        };
        let s = serde_json::to_string(&acc).unwrap();
        assert_eq!(serde_json::from_str::<AccumulatorOutcome>(&s).unwrap(), acc);

        let rea = ReactorOutcome {
            fire: true,
            context_json: Some(r#"{"go":1}"#.into()),
            error: None,
        };
        let s = serde_json::to_string(&rea).unwrap();
        assert_eq!(serde_json::from_str::<ReactorOutcome>(&s).unwrap(), rea);
    }
}
