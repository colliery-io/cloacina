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
// Serialization helpers (profile-aware: JSON in debug, bincode in release)
// ---------------------------------------------------------------------------

/// Serialize a value to bytes using the build-profile-appropriate format.
///
/// - Release: bincode (fast, compact)
/// - Debug: JSON (readable, inspectable in logs)
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, GraphError> {
    #[cfg(debug_assertions)]
    {
        serde_json::to_vec(value).map_err(|e| GraphError::Serialization(e.to_string()))
    }
    #[cfg(not(debug_assertions))]
    {
        bincode::serialize(value).map_err(|e| GraphError::Serialization(e.to_string()))
    }
}

/// Deserialize bytes to a value using the build-profile-appropriate format.
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, GraphError> {
    #[cfg(debug_assertions)]
    {
        serde_json::from_slice(bytes).map_err(|e| GraphError::Deserialization(e.to_string()))
    }
    #[cfg(not(debug_assertions))]
    {
        bincode::deserialize(bytes).map_err(|e| GraphError::Deserialization(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// InputCache
// ---------------------------------------------------------------------------

/// The input cache holds the last-seen serialized boundary per source.
///
/// The reactor's receiver task updates this cache continuously. The executor
/// takes a snapshot before calling the compiled graph function.
///
/// Serialization format:
/// - **Release builds**: bincode (compact binary, fast)
/// - **Debug builds**: JSON (human-readable, inspectable)
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
// Global registry (for embedded-mode auto-registration via #[ctor])
// ---------------------------------------------------------------------------

/// Metadata about a registered computation graph.
pub struct ComputationGraphRegistration {
    /// The compiled graph function.
    pub graph_fn: CompiledGraphFn,
    /// Accumulator names declared in the graph topology.
    pub accumulator_names: Vec<String>,
    /// Reaction mode: "when_any" or "when_all".
    pub reaction_mode: String,
}

pub type ComputationGraphConstructor = Box<dyn Fn() -> ComputationGraphRegistration + Send + Sync>;
pub type GlobalComputationGraphRegistry =
    Arc<parking_lot::RwLock<HashMap<String, ComputationGraphConstructor>>>;

static GLOBAL_COMPUTATION_GRAPH_REGISTRY: once_cell::sync::Lazy<GlobalComputationGraphRegistry> =
    once_cell::sync::Lazy::new(|| Arc::new(parking_lot::RwLock::new(HashMap::new())));

/// Register a computation graph constructor in the global registry.
pub fn register_computation_graph_constructor<F>(graph_name: String, constructor: F)
where
    F: Fn() -> ComputationGraphRegistration + Send + Sync + 'static,
{
    let mut registry = GLOBAL_COMPUTATION_GRAPH_REGISTRY.write();
    registry.insert(graph_name.clone(), Box::new(constructor));
    tracing::debug!("Registered computation graph constructor: {}", graph_name);
}

/// Get the global computation graph registry.
pub fn global_computation_graph_registry() -> GlobalComputationGraphRegistry {
    GLOBAL_COMPUTATION_GRAPH_REGISTRY.clone()
}

/// List all registered computation graph names.
pub fn list_registered_graphs() -> Vec<String> {
    let registry = GLOBAL_COMPUTATION_GRAPH_REGISTRY.read();
    registry.keys().cloned().collect()
}

/// Remove a computation graph from the global registry.
pub fn deregister_computation_graph(graph_name: &str) {
    let mut registry = GLOBAL_COMPUTATION_GRAPH_REGISTRY.write();
    registry.remove(graph_name);
    tracing::debug!("Deregistered computation graph constructor: {}", graph_name);
}

// Re-export types module for backward compat path: `cloacina_computation_graph::types::serialize`
pub mod types {
    pub use crate::{deserialize, serialize, GraphError, GraphResult, InputCache, SourceName};
}
