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

//! Core types for computation graph execution.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::fmt;

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

/// The input cache holds the last-seen serialized boundary per source.
///
/// The reactor's receiver task updates this cache continuously. The executor
/// takes a snapshot before calling the compiled graph function. New boundaries
/// arriving during execution update the cache but don't affect the running
/// execution.
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

    /// Update the cached value for a source. Overwrites the previous value.
    pub fn update(&mut self, source: SourceName, bytes: Vec<u8>) {
        self.entries.insert(source, bytes);
    }

    /// Get and deserialize a cached value by source name.
    ///
    /// Returns `None` if the source has never emitted (no entry in cache).
    /// Returns `Some(Err(...))` if deserialization fails (type mismatch, corrupt data).
    pub fn get<T: DeserializeOwned>(&self, name: &str) -> Option<Result<T, GraphError>> {
        let bytes = self.entries.get(&SourceName::new(name))?;
        Some(deserialize::<T>(bytes))
    }

    /// Check if a source has an entry in the cache.
    pub fn has(&self, name: &str) -> bool {
        self.entries.contains_key(&SourceName::new(name))
    }

    /// Get the raw bytes for a source (for forwarding without deserialization).
    pub fn get_raw(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .get(&SourceName::new(name))
            .map(|v| v.as_slice())
    }

    /// Create a snapshot (clone) of the cache for the executor.
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

    /// Replace all entries (used for manual fire-with-state).
    pub fn replace_all(&mut self, other: InputCache) {
        self.entries = other.entries;
    }

    /// List all source names in the cache.
    pub fn sources(&self) -> Vec<&SourceName> {
        self.entries.keys().collect()
    }

    /// Get a reference to the raw entries map (for serialization/persistence).
    pub fn entries_raw(&self) -> &HashMap<SourceName, Vec<u8>> {
        &self.entries
    }

    /// Return entries as a JSON-friendly map (base64-encoded raw bytes per source).
    ///
    /// Used by the reactor WebSocket GetState command to return human-readable state.
    /// In debug mode, attempts JSON deserialization first for readability.
    pub fn entries_as_json(&self) -> std::collections::HashMap<String, String> {
        self.entries
            .iter()
            .map(|(name, bytes)| {
                let value = if cfg!(debug_assertions) {
                    // In debug mode, try to deserialize as JSON for readability
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

/// Serialize a value to bytes using the build-profile-appropriate format.
///
/// - Release: bincode (fast, compact)
/// - Debug: JSON (readable, inspectable in logs)
/// Encode bytes as hex string (for debug display of binary cache entries).
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

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

/// Result of executing a compiled computation graph.
///
/// The graph always runs to completion when called — the reactor already
/// decided to fire. Intermediate branches may short-circuit via `Option<T>`,
/// but the graph itself always produces `Completed` or `Error`.
#[derive(Debug)]
pub enum GraphResult {
    /// Graph executed to completion. Contains terminal node outputs.
    Completed { outputs: Vec<Box<dyn Any + Send>> },
    /// Graph execution failed.
    Error(GraphError),
}

impl GraphResult {
    /// Create a completed result with terminal node outputs.
    pub fn completed(outputs: Vec<Box<dyn Any + Send>>) -> Self {
        Self::Completed { outputs }
    }

    /// Create a completed result with no outputs (all branches short-circuited).
    pub fn completed_empty() -> Self {
        Self::Completed {
            outputs: Vec::new(),
        }
    }

    /// Create an error result.
    pub fn error(err: GraphError) -> Self {
        Self::Error(err)
    }

    /// Check if the graph completed successfully.
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    /// Check if the graph errored.
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        value: f64,
        label: String,
    }

    #[test]
    fn test_input_cache_update_and_get() {
        let mut cache = InputCache::new();
        let data = TestData {
            value: 42.0,
            label: "test".to_string(),
        };
        let bytes = serialize(&data).unwrap();

        cache.update(SourceName::new("alpha"), bytes);

        let result: TestData = cache.get("alpha").unwrap().unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_input_cache_missing_source() {
        let cache = InputCache::new();
        let result: Option<Result<TestData, GraphError>> = cache.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_input_cache_overwrite() {
        let mut cache = InputCache::new();

        let data1 = TestData {
            value: 1.0,
            label: "first".to_string(),
        };
        let data2 = TestData {
            value: 2.0,
            label: "second".to_string(),
        };

        cache.update(SourceName::new("alpha"), serialize(&data1).unwrap());
        cache.update(SourceName::new("alpha"), serialize(&data2).unwrap());

        let result: TestData = cache.get("alpha").unwrap().unwrap();
        assert_eq!(result, data2);
    }

    #[test]
    fn test_input_cache_snapshot() {
        let mut cache = InputCache::new();
        let data = TestData {
            value: 42.0,
            label: "test".to_string(),
        };
        cache.update(SourceName::new("alpha"), serialize(&data).unwrap());

        let snapshot = cache.snapshot();

        // Modify original — snapshot should be unaffected
        let new_data = TestData {
            value: 99.0,
            label: "changed".to_string(),
        };
        cache.update(SourceName::new("alpha"), serialize(&new_data).unwrap());

        let from_snapshot: TestData = snapshot.get("alpha").unwrap().unwrap();
        assert_eq!(from_snapshot, data);

        let from_cache: TestData = cache.get("alpha").unwrap().unwrap();
        assert_eq!(from_cache, new_data);
    }

    #[test]
    fn test_input_cache_has() {
        let mut cache = InputCache::new();
        assert!(!cache.has("alpha"));

        cache.update(SourceName::new("alpha"), serialize(&42u32).unwrap());
        assert!(cache.has("alpha"));
        assert!(!cache.has("beta"));
    }

    #[test]
    fn test_input_cache_len_and_empty() {
        let mut cache = InputCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        cache.update(SourceName::new("alpha"), serialize(&1u32).unwrap());
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);

        cache.update(SourceName::new("beta"), serialize(&2u32).unwrap());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_serialization_round_trip() {
        let data = TestData {
            value: 3.14,
            label: "pi".to_string(),
        };
        let bytes = serialize(&data).unwrap();
        let result: TestData = deserialize(&bytes).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_serialization_round_trip_primitives() {
        // u64
        let bytes = serialize(&42u64).unwrap();
        let result: u64 = deserialize(&bytes).unwrap();
        assert_eq!(result, 42u64);

        // String
        let bytes = serialize(&"hello".to_string()).unwrap();
        let result: String = deserialize(&bytes).unwrap();
        assert_eq!(result, "hello");

        // Vec
        let bytes = serialize(&vec![1.0f64, 2.0, 3.0]).unwrap();
        let result: Vec<f64> = deserialize(&bytes).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_deserialization_type_mismatch() {
        let bytes = serialize(&42u64).unwrap();
        let result: Result<String, GraphError> = deserialize(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_graph_result_completed() {
        let result = GraphResult::completed(vec![Box::new(42u32), Box::new("hello".to_string())]);
        assert!(result.is_completed());
        assert!(!result.is_error());
    }

    #[test]
    fn test_graph_result_completed_empty() {
        let result = GraphResult::completed_empty();
        assert!(result.is_completed());
        if let GraphResult::Completed { outputs } = result {
            assert!(outputs.is_empty());
        }
    }

    #[test]
    fn test_graph_result_error() {
        let result = GraphResult::error(GraphError::MissingInput("alpha".to_string()));
        assert!(result.is_error());
        assert!(!result.is_completed());
    }

    #[test]
    fn test_source_name_equality() {
        let a = SourceName::new("alpha");
        let b = SourceName::from("alpha");
        let c = SourceName::from("beta".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_replace_all() {
        let mut cache1 = InputCache::new();
        cache1.update(SourceName::new("alpha"), serialize(&1u32).unwrap());

        let mut cache2 = InputCache::new();
        cache2.update(SourceName::new("beta"), serialize(&2u32).unwrap());

        cache1.replace_all(cache2);

        assert!(!cache1.has("alpha"));
        assert!(cache1.has("beta"));
    }
}
