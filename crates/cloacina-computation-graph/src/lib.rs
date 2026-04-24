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

//! Core types for Cloacina computation graph plugins.
//!
//! This crate contains the types that packaged computation graph cdylibs need
//! at compile time. It is the computation-graph equivalent of `cloacina-workflow`
//! — a thin crate that avoids pulling in the full engine.
//!
//! The `#[computation_graph]` macro expands into code that references types from
//! this crate. Embedded-mode users get these types re-exported from `cloacina`.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// SourceName
// ---------------------------------------------------------------------------

/// Identifies an accumulator source by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceName(pub String);

impl SourceName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for SourceName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for SourceName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers (bincode wire format)
// ---------------------------------------------------------------------------

/// Serialize a value to bincode bytes.
///
/// Bincode is used for all internal wire formats (boundary channels,
/// checkpoint persistence, accumulator-to-reactor messaging).
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, GraphError> {
    bincode::serialize(value).map_err(|e| GraphError::Serialization(e.to_string()))
}

/// Deserialize bincode bytes to a value.
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, GraphError> {
    bincode::deserialize(bytes).map_err(|e| GraphError::Deserialization(e.to_string()))
}

/// Convert a JSON string to bincode bytes for a given type.
///
/// Convenience for external producers pushing events via WebSocket.
/// The WebSocket handler calls this to convert incoming JSON to the
/// internal bincode wire format before forwarding to accumulators.
pub fn json_to_wire<T: Serialize + DeserializeOwned>(
    json_str: &str,
) -> Result<Vec<u8>, GraphError> {
    let value: T =
        serde_json::from_str(json_str).map_err(|e| GraphError::Serialization(e.to_string()))?;
    serialize(&value)
}

// ---------------------------------------------------------------------------
// InputCache
// ---------------------------------------------------------------------------

/// The input cache holds the last-seen serialized boundary per source.
///
/// The reactor's receiver task updates this cache continuously. The executor
/// takes a snapshot before calling the compiled graph function.
///
/// Serialization format: bincode (compact binary). The FFI packaging bridge
/// converts bincode→JSON at the boundary for plugin compatibility.
#[derive(Debug, Clone)]
pub struct InputCache {
    entries: HashMap<SourceName, Vec<u8>>,
}

impl InputCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Update the cached value for a source.
    pub fn update(&mut self, source: SourceName, bytes: Vec<u8>) {
        self.entries.insert(source, bytes);
    }

    /// Get and deserialize a cached value by source name.
    pub fn get<T: DeserializeOwned>(&self, name: &str) -> Option<Result<T, GraphError>> {
        let bytes = self.entries.get(&SourceName::new(name))?;
        Some(deserialize::<T>(bytes))
    }

    /// Check if a source has an entry in the cache.
    pub fn has(&self, name: &str) -> bool {
        self.entries.contains_key(&SourceName::new(name))
    }

    /// Get the raw bytes for a source.
    pub fn get_raw(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .get(&SourceName::new(name))
            .map(|v| v.as_slice())
    }

    /// Create a snapshot (clone) of the cache.
    pub fn snapshot(&self) -> InputCache {
        self.clone()
    }

    /// Number of sources in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Replace all entries.
    pub fn replace_all(&mut self, other: InputCache) {
        self.entries = other.entries;
    }

    /// List all source names in the cache.
    pub fn sources(&self) -> Vec<&SourceName> {
        self.entries.keys().collect()
    }

    /// Get a reference to the raw entries map.
    pub fn entries_raw(&self) -> &HashMap<SourceName, Vec<u8>> {
        &self.entries
    }

    /// Return entries as a JSON-friendly map.
    pub fn entries_as_json(&self) -> HashMap<String, String> {
        self.entries
            .iter()
            .map(|(name, bytes)| {
                let value = if cfg!(debug_assertions) {
                    serde_json::from_slice::<serde_json::Value>(bytes)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| hex_encode(bytes))
                } else {
                    hex_encode(bytes)
                };
                (name.as_str().to_string(), value)
            })
            .collect()
    }
}

impl Default for InputCache {
    fn default() -> Self {
        Self::new()
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// GraphResult / GraphError
// ---------------------------------------------------------------------------

/// Result of executing a compiled computation graph.
#[derive(Debug)]
pub enum GraphResult {
    /// Graph executed to completion. Contains terminal node outputs.
    Completed { outputs: Vec<Box<dyn Any + Send>> },
    /// Graph execution failed.
    Error(GraphError),
}

impl GraphResult {
    pub fn completed(outputs: Vec<Box<dyn Any + Send>>) -> Self {
        Self::Completed { outputs }
    }

    pub fn completed_empty() -> Self {
        Self::Completed {
            outputs: Vec::new(),
        }
    }

    pub fn error(err: GraphError) -> Self {
        Self::Error(err)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

/// Errors that can occur during graph execution.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("serialization failed: {0}")]
    Serialization(String),

    #[error("deserialization failed: {0}")]
    Deserialization(String),

    #[error("missing input: source '{0}' not found in cache")]
    MissingInput(String),

    #[error("node execution failed: {0}")]
    NodeExecution(String),

    #[error("graph execution failed: {0}")]
    Execution(String),
}

// ---------------------------------------------------------------------------
// CompiledGraphFn
// ---------------------------------------------------------------------------

/// Type alias for the compiled graph function.
pub type CompiledGraphFn =
    Arc<dyn Fn(InputCache) -> Pin<Box<dyn Future<Output = GraphResult> + Send>> + Send + Sync>;

// ---------------------------------------------------------------------------
// Computation graph constructor types
// ---------------------------------------------------------------------------
//
// The process-global computation-graph registry was removed in CLOACI-T-0509.
// Constructors are owned by `cloacina::Runtime`, which is seeded from the
// `inventory` entries emitted by the `#[computation_graph]` macro.

/// Metadata about a registered computation graph.
///
/// The shape carries both the legacy bundled-form metadata
/// (`accumulator_names` + `reaction_mode`, consumed by packaging FFI and the
/// reconciler) and the new split-form metadata (`entry_accumulators` +
/// `trigger_reactor`). Graphs emitted from the bundled form populate both.
/// Graphs emitted from the split form also populate both — legacy fields are
/// copied from the referenced reactor's declaration so downstream code that
/// still reads them keeps working during the I-0101 migration.
/// Trigger-less graphs carry empty legacy fields and `trigger_reactor = None`.
pub struct ComputationGraphRegistration {
    /// The compiled graph function.
    pub graph_fn: CompiledGraphFn,
    /// Accumulator names used by the graph's entry nodes. Enforced to be a
    /// subset of the referenced reactor's `ACCUMULATORS` at macro expansion
    /// for split form; identical to `accumulator_names` for bundled form.
    pub entry_accumulators: Vec<String>,
    /// Name of the reactor this graph is bound to, if any. `None` for
    /// trigger-less graphs (T-02/T-03 invoke these directly from workflow
    /// tasks or Python tasks).
    pub trigger_reactor: Option<String>,
    /// Accumulator names — legacy field kept for packaging FFI + reconciler
    /// compatibility. For split-form graphs this mirrors the reactor's
    /// accumulators; for trigger-less graphs it is empty.
    pub accumulator_names: Vec<String>,
    /// Reaction mode: `"when_any"`, `"when_all"`, or `"none"` for
    /// trigger-less graphs. Legacy field retained for the same reasons as
    /// `accumulator_names`.
    pub reaction_mode: String,
}

pub type ComputationGraphConstructor = Box<dyn Fn() -> ComputationGraphRegistration + Send + Sync>;

// ---------------------------------------------------------------------------
// Reactor
// ---------------------------------------------------------------------------
//
// A reactor is a named bundle of accumulators + firing criteria. It fires an
// `InputCache` whenever its criteria are met and publishes to any graph bound
// to it by name.
//
// The `#[reactor]` attribute macro emits a unit struct + `impl Reactor for X`.
// The struct is a compile-time handle: `#[computation_graph(trigger =
// reactor(X))]` references it by type path so the graph macro can const-check
// that its entry accumulators are a subset of `<X as Reactor>::ACCUMULATORS`.
//
// The synthesized bundled-form reactor struct is named `__Reactor_<graphname>`
// (double-underscore prefix) so operational listings can filter it out by
// default.

/// How a reactor decides when to fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactionMode {
    /// Fire as soon as any one accumulator has new input.
    WhenAny,
    /// Fire only when every accumulator has new input since the last firing.
    WhenAll,
}

impl ReactionMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            ReactionMode::WhenAny => "when_any",
            ReactionMode::WhenAll => "when_all",
        }
    }
}

impl fmt::Display for ReactionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Compile-time handle for a reactor declaration.
///
/// Implemented by the unit struct emitted by `#[reactor]`. The constants are
/// readable at `const` context, which the `#[computation_graph]` macro uses to
/// const-check the graph's entry accumulators against the reactor's
/// accumulator set.
pub trait Reactor {
    /// Reactor name as declared in `#[reactor(name = "...")]`.
    const NAME: &'static str;
    /// Declared accumulator names. Order matches the `accumulators = [...]`
    /// clause; uniqueness is enforced at macro expansion.
    const ACCUMULATORS: &'static [&'static str];
    /// Firing criteria.
    const REACTION_MODE: ReactionMode;
}

/// Runtime-side description of a reactor.
///
/// Populated by the `#[reactor]` macro's emitted inventory entry and by the
/// bundled `#[computation_graph]` form's synthesized reactor.
#[derive(Debug, Clone)]
pub struct ReactorRegistration {
    /// Reactor name. User-declared reactors carry the name the user wrote.
    /// Bundled-form synthesized reactors carry `__Reactor_<graphname>`.
    pub name: String,
    pub accumulator_names: Vec<String>,
    pub reaction_mode: ReactionMode,
}

pub type ReactorConstructor = Box<dyn Fn() -> ReactorRegistration + Send + Sync>;

// Re-export types module for backward compat path: `cloacina_computation_graph::types::serialize`
pub mod types {
    pub use crate::{deserialize, serialize, GraphError, GraphResult, InputCache, SourceName};
}
