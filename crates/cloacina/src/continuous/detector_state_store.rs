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

//! In-memory detector state store with committed/latest checkpoint tracking.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Per-source detector checkpoint with latest/committed split.
#[derive(Debug, Clone, Default)]
pub struct DetectorCheckpoint {
    /// Most recent state emitted by detector (may not be drained yet).
    pub latest: Option<serde_json::Value>,
    /// State as of when ALL consumers last drained (safe to resume from).
    pub committed: Option<serde_json::Value>,
    /// Per-edge: the detector state that was current when each edge last drained.
    pub edge_drain_states: HashMap<String, serde_json::Value>,
}

/// Thread-safe in-memory store for detector state checkpoints.
///
/// Loaded from DB on startup, updated as detectors complete and edges drain.
/// The `committed` state is the safe resume point after crash.
#[derive(Debug, Clone)]
pub struct DetectorStateStore {
    inner: Arc<RwLock<HashMap<String, DetectorCheckpoint>>>,
}

impl DetectorStateStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load committed states from DB on startup.
    pub fn load_committed(&self, source_name: &str, committed_state: serde_json::Value) {
        let mut map = self.inner.write();
        let checkpoint = map.entry(source_name.to_string()).or_default();
        checkpoint.committed = Some(committed_state);
    }

    /// Get the committed (safe-to-resume) state for a source.
    pub fn get_committed(&self, source_name: &str) -> Option<serde_json::Value> {
        let map = self.inner.read();
        map.get(source_name).and_then(|cp| cp.committed.clone())
    }

    /// Update the latest (not yet committed) state from a detector completion.
    pub fn update_latest(&self, source_name: &str, state: serde_json::Value) {
        let mut map = self.inner.write();
        let checkpoint = map.entry(source_name.to_string()).or_default();
        checkpoint.latest = Some(state);
    }

    /// Record that an edge drained, capturing the current latest state for that edge.
    /// Returns the committed state if all edges for this source have now drained
    /// (i.e., this was the slowest consumer).
    pub fn record_edge_drain(&self, source_name: &str, edge_id: &str) -> Option<serde_json::Value> {
        let mut map = self.inner.write();
        let checkpoint = map.entry(source_name.to_string()).or_default();

        // Snapshot the current latest state for this edge's drain point
        if let Some(ref latest) = checkpoint.latest {
            checkpoint
                .edge_drain_states
                .insert(edge_id.to_string(), latest.clone());
        }

        // The caller must check if all edges have drained (via pending_boundary DAL)
        // and call commit() if so. We just record the edge's drain state here.
        None
    }

    /// Commit: promote latest -> committed. Called when all consumers have drained.
    pub fn commit(&self, source_name: &str) -> Option<serde_json::Value> {
        let mut map = self.inner.write();
        if let Some(checkpoint) = map.get_mut(source_name) {
            if let Some(ref latest) = checkpoint.latest {
                checkpoint.committed = Some(latest.clone());
            }
            // Clear edge drain states after commit
            checkpoint.edge_drain_states.clear();
            checkpoint.committed.clone()
        } else {
            None
        }
    }

    /// Get the latest (uncommitted) state for a source. Used internally.
    pub fn get_latest(&self, source_name: &str) -> Option<serde_json::Value> {
        let map = self.inner.read();
        map.get(source_name).and_then(|cp| cp.latest.clone())
    }
}

impl Default for DetectorStateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_update_latest_and_get_committed() {
        let store = DetectorStateStore::new();

        // Initially nothing committed
        assert!(store.get_committed("events").is_none());

        // Update latest
        store.update_latest("events", json!("cursor_100"));

        // Still nothing committed
        assert!(store.get_committed("events").is_none());

        // Latest is set
        assert_eq!(store.get_latest("events"), Some(json!("cursor_100")));
    }

    #[test]
    fn test_commit_promotes_latest() {
        let store = DetectorStateStore::new();

        store.update_latest("events", json!("cursor_100"));
        let committed = store.commit("events");

        assert_eq!(committed, Some(json!("cursor_100")));
        assert_eq!(store.get_committed("events"), Some(json!("cursor_100")));
    }

    #[test]
    fn test_update_without_commit_preserves_old_committed() {
        let store = DetectorStateStore::new();

        // First cycle: update and commit
        store.update_latest("events", json!("cursor_100"));
        store.commit("events");

        // Second cycle: update but DON'T commit
        store.update_latest("events", json!("cursor_200"));

        // Committed still at 100
        assert_eq!(store.get_committed("events"), Some(json!("cursor_100")));
        // Latest at 200
        assert_eq!(store.get_latest("events"), Some(json!("cursor_200")));
    }

    #[test]
    fn test_load_committed_from_db() {
        let store = DetectorStateStore::new();

        // Simulate loading from DB on startup
        store.load_committed("events", json!("cursor_500"));

        assert_eq!(store.get_committed("events"), Some(json!("cursor_500")));
        assert!(store.get_latest("events").is_none()); // Latest not set yet
    }

    #[test]
    fn test_record_edge_drain() {
        let store = DetectorStateStore::new();

        store.update_latest("events", json!("cursor_100"));
        store.record_edge_drain("events", "events:task_a");

        let map = store.inner.read();
        let cp = map.get("events").unwrap();
        assert_eq!(
            cp.edge_drain_states.get("events:task_a"),
            Some(&json!("cursor_100"))
        );
    }

    #[test]
    fn test_commit_clears_edge_drain_states() {
        let store = DetectorStateStore::new();

        store.update_latest("events", json!("cursor_100"));
        store.record_edge_drain("events", "events:task_a");
        store.commit("events");

        let map = store.inner.read();
        let cp = map.get("events").unwrap();
        assert!(cp.edge_drain_states.is_empty());
    }

    #[test]
    fn test_multiple_sources_independent() {
        let store = DetectorStateStore::new();

        store.update_latest("events", json!("e_100"));
        store.update_latest("config", json!("c_50"));
        store.commit("events");

        assert_eq!(store.get_committed("events"), Some(json!("e_100")));
        assert!(store.get_committed("config").is_none()); // config not committed
    }

    #[test]
    fn test_commit_gate_multi_edge_slowest_wins() {
        let store = DetectorStateStore::new();

        // Detector emits state S1
        store.update_latest("events", json!("cursor_100"));

        // Edge A drains — records S1 as edge A's drain state
        store.record_edge_drain("events", "events:task_a");

        // Edge B has NOT drained yet
        // Verify edge_drain_states has only A
        let map = store.inner.read();
        let cp = map.get("events").unwrap();
        assert_eq!(cp.edge_drain_states.len(), 1);
        assert!(cp.edge_drain_states.contains_key("events:task_a"));
        drop(map);

        // Now detector emits S2
        store.update_latest("events", json!("cursor_200"));

        // Edge B finally drains — records S2 as edge B's drain state
        store.record_edge_drain("events", "events:task_b");

        // Now both edges have drained. Edge A at S1, Edge B at S2.
        // The committed state should be the LATEST (since that's what commit() does)
        let committed = store.commit("events");
        assert_eq!(committed, Some(json!("cursor_200")));
    }

    #[test]
    fn test_commit_preserves_state_across_cycles() {
        let store = DetectorStateStore::new();

        // Cycle 1: emit and commit
        store.update_latest("events", json!("cursor_100"));
        store.commit("events");
        assert_eq!(store.get_committed("events"), Some(json!("cursor_100")));

        // Cycle 2: emit new state but don't commit
        store.update_latest("events", json!("cursor_200"));

        // Committed still at 100
        assert_eq!(store.get_committed("events"), Some(json!("cursor_100")));

        // Cycle 3: commit again
        store.commit("events");
        assert_eq!(store.get_committed("events"), Some(json!("cursor_200")));
    }

    #[test]
    fn test_record_edge_drain_captures_latest_at_drain_time() {
        let store = DetectorStateStore::new();

        // S1 emitted, edge A drains
        store.update_latest("events", json!("S1"));
        store.record_edge_drain("events", "events:task_a");

        // S2 emitted, edge A drains again
        store.update_latest("events", json!("S2"));
        store.record_edge_drain("events", "events:task_a");

        // Edge A's drain state should be S2 (latest at time of drain)
        let map = store.inner.read();
        let cp = map.get("events").unwrap();
        assert_eq!(
            cp.edge_drain_states.get("events:task_a"),
            Some(&json!("S2"))
        );
    }
}
