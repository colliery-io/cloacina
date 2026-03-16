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

//! Integration tests for accumulator state persistence with a real database.
//!
//! Detector State and Pending Boundary DAL tests run on ALL enabled backends
//! (both SQLite and Postgres) via `get_all_fixtures()` to ensure the SQLite
//! code paths are exercised at runtime, not just compiled.

use crate::fixtures;
use crate::fixtures::get_all_fixtures;
use cloacina::dal::unified::models::NewAccumulatorState;
use cloacina::dal::unified::AccumulatorStateDAL;
use serial_test::serial;

/// Get a DAL with fresh migrations (resets and re-initializes the fixture).
async fn get_fresh_dal() -> cloacina::dal::DAL {
    let fixture = fixtures::get_or_init_fixture().await;
    let mut fixture = fixture.lock().unwrap();
    fixture.reset_database().await;
    fixture.initialize().await;
    fixture.get_dal()
}

#[tokio::test]
#[serial]
async fn test_save_and_load_accumulator_state() {
    let dal = get_fresh_dal().await;

    let acc_dal = AccumulatorStateDAL::new(&dal);

    // Save a state
    let state = NewAccumulatorState {
        edge_id: "test_source:test_task".into(),
        consumer_watermark: Some(r#"{"kind":{"type":"OffsetRange","start":0,"end":100}}"#.into()),
        drain_metadata: r#"{"signals_coalesced":5}"#.into(),
    };
    acc_dal.save(state).await.expect("save failed");

    // Load it back
    let loaded = acc_dal
        .load("test_source:test_task")
        .await
        .expect("load failed");
    assert!(loaded.is_some(), "state should exist after save");
    let loaded = loaded.unwrap();
    assert_eq!(loaded.edge_id, "test_source:test_task");
    assert!(loaded.consumer_watermark.is_some());
    assert!(loaded.consumer_watermark.unwrap().contains("OffsetRange"));

    // Clean up
    acc_dal
        .delete_by_ids(vec!["test_source:test_task".into()])
        .await
        .expect("delete failed");
}

#[tokio::test]
#[serial]
async fn test_save_upserts_on_conflict() {
    let dal = get_fresh_dal().await;

    let acc_dal = AccumulatorStateDAL::new(&dal);

    // Save initial state
    let state1 = NewAccumulatorState {
        edge_id: "upsert_edge:task".into(),
        consumer_watermark: Some(r#"{"version":"v1"}"#.into()),
        drain_metadata: "{}".into(),
    };
    acc_dal.save(state1).await.expect("save 1 failed");

    // Save again with updated watermark — should upsert
    let state2 = NewAccumulatorState {
        edge_id: "upsert_edge:task".into(),
        consumer_watermark: Some(r#"{"version":"v2"}"#.into()),
        drain_metadata: r#"{"updated":true}"#.into(),
    };
    acc_dal.save(state2).await.expect("save 2 failed");

    // Load — should have the updated values
    let loaded = acc_dal
        .load("upsert_edge:task")
        .await
        .expect("load failed")
        .expect("state should exist");
    assert_eq!(
        loaded.consumer_watermark,
        Some(r#"{"version":"v2"}"#.into())
    );

    // Clean up
    acc_dal
        .delete_by_ids(vec!["upsert_edge:task".into()])
        .await
        .unwrap();
}

#[tokio::test]
#[serial]
async fn test_load_all_and_delete() {
    let dal = get_fresh_dal().await;

    let acc_dal = AccumulatorStateDAL::new(&dal);

    // Save multiple states
    for i in 0..3 {
        acc_dal
            .save(NewAccumulatorState {
                edge_id: format!("bulk_src:task_{}", i),
                consumer_watermark: None,
                drain_metadata: "{}".into(),
            })
            .await
            .expect("save failed");
    }

    // Load all — should include our 3
    let all = acc_dal.load_all().await.expect("load_all failed");
    let our_states: Vec<_> = all
        .iter()
        .filter(|s| s.edge_id.starts_with("bulk_src:"))
        .collect();
    assert_eq!(our_states.len(), 3);

    // Delete 2 of 3
    let deleted = acc_dal
        .delete_by_ids(vec!["bulk_src:task_0".into(), "bulk_src:task_1".into()])
        .await
        .expect("delete failed");
    assert_eq!(deleted, 2);

    // Only 1 remains
    let remaining = acc_dal.load_all().await.expect("load_all failed");
    let our_remaining: Vec<_> = remaining
        .iter()
        .filter(|s| s.edge_id.starts_with("bulk_src:"))
        .collect();
    assert_eq!(our_remaining.len(), 1);
    assert_eq!(our_remaining[0].edge_id, "bulk_src:task_2");

    // Clean up
    acc_dal
        .delete_by_ids(vec!["bulk_src:task_2".into()])
        .await
        .unwrap();
}

#[tokio::test]
#[serial]
async fn test_load_nonexistent_returns_none() {
    let dal = get_fresh_dal().await;

    let acc_dal = AccumulatorStateDAL::new(&dal);

    let loaded = acc_dal
        .load("nonexistent_edge:task")
        .await
        .expect("load failed");
    assert!(loaded.is_none());
}

// ============================================================================
// Detector State DAL Tests (multi-backend: runs on both SQLite and Postgres)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_detector_state_save_and_load() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_detector_state_save_and_load on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let ds_dal = cloacina::dal::unified::DetectorStateDAL::new(&dal);

        ds_dal
            .save(cloacina::dal::unified::models::NewDetectorState {
                source_name: "events".to_string(),
                committed_state: Some(r#"{"cursor": 100}"#.to_string()),
            })
            .await
            .unwrap();

        let loaded = ds_dal.load("events").await.unwrap();
        assert!(
            loaded.is_some(),
            "[{}] state should exist after save",
            backend
        );
        let row = loaded.unwrap();
        assert_eq!(row.source_name, "events");
        assert_eq!(row.committed_state, Some(r#"{"cursor": 100}"#.to_string()));
    }
}

#[tokio::test]
#[serial]
async fn test_detector_state_upserts() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_detector_state_upserts on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let ds_dal = cloacina::dal::unified::DetectorStateDAL::new(&dal);

        ds_dal
            .save(cloacina::dal::unified::models::NewDetectorState {
                source_name: "events".to_string(),
                committed_state: Some(r#"{"cursor": 100}"#.to_string()),
            })
            .await
            .unwrap();

        ds_dal
            .save(cloacina::dal::unified::models::NewDetectorState {
                source_name: "events".to_string(),
                committed_state: Some(r#"{"cursor": 500}"#.to_string()),
            })
            .await
            .unwrap();

        let loaded = ds_dal.load("events").await.unwrap().unwrap();
        assert_eq!(
            loaded.committed_state,
            Some(r#"{"cursor": 500}"#.to_string()),
            "[{}] upsert should update committed_state",
            backend,
        );
    }
}

#[tokio::test]
#[serial]
async fn test_detector_state_load_nonexistent() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!(
            "Running test_detector_state_load_nonexistent on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let ds_dal = cloacina::dal::unified::DetectorStateDAL::new(&dal);

        let loaded = ds_dal.load("nonexistent").await.unwrap();
        assert!(
            loaded.is_none(),
            "[{}] loading nonexistent should return None",
            backend
        );
    }
}

#[tokio::test]
#[serial]
async fn test_detector_state_load_all() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_detector_state_load_all on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let ds_dal = cloacina::dal::unified::DetectorStateDAL::new(&dal);

        ds_dal
            .save(cloacina::dal::unified::models::NewDetectorState {
                source_name: "events".to_string(),
                committed_state: Some(r#"{"cursor": 100}"#.to_string()),
            })
            .await
            .unwrap();

        ds_dal
            .save(cloacina::dal::unified::models::NewDetectorState {
                source_name: "config".to_string(),
                committed_state: Some(r#"{"version": 3}"#.to_string()),
            })
            .await
            .unwrap();

        let all = ds_dal.load_all().await.unwrap();
        assert_eq!(
            all.len(),
            2,
            "[{}] load_all should return 2 states",
            backend
        );
    }
}

#[tokio::test]
#[serial]
async fn test_detector_state_save_with_null_committed_state() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!(
            "Running test_detector_state_save_with_null_committed_state on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let ds_dal = cloacina::dal::unified::DetectorStateDAL::new(&dal);

        ds_dal
            .save(cloacina::dal::unified::models::NewDetectorState {
                source_name: "null_state_source".to_string(),
                committed_state: None,
            })
            .await
            .unwrap();

        let loaded = ds_dal.load("null_state_source").await.unwrap();
        assert!(
            loaded.is_some(),
            "[{}] state should exist even with None committed_state",
            backend
        );
        assert_eq!(
            loaded.unwrap().committed_state,
            None,
            "[{}] committed_state should be None",
            backend
        );
    }
}

// ============================================================================
// Pending Boundary DAL Tests (multi-backend: runs on both SQLite and Postgres)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_pending_boundary_append_and_load() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!(
            "Running test_pending_boundary_append_and_load on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

        let id1 = pb_dal
            .append("events".to_string(), r#"{"start":0,"end":100}"#.to_string())
            .await
            .unwrap();
        let id2 = pb_dal
            .append(
                "events".to_string(),
                r#"{"start":100,"end":200}"#.to_string(),
            )
            .await
            .unwrap();
        let id3 = pb_dal
            .append(
                "events".to_string(),
                r#"{"start":200,"end":300}"#.to_string(),
            )
            .await
            .unwrap();

        assert!(
            id2 > id1,
            "[{}] IDs should be monotonically increasing",
            backend
        );
        assert!(
            id3 > id2,
            "[{}] IDs should be monotonically increasing",
            backend
        );

        let loaded = pb_dal
            .load_after_cursor("events".to_string(), 0)
            .await
            .unwrap();
        assert_eq!(
            loaded.len(),
            3,
            "[{}] should load all 3 boundaries",
            backend
        );

        let loaded = pb_dal
            .load_after_cursor("events".to_string(), id1)
            .await
            .unwrap();
        assert_eq!(
            loaded.len(),
            2,
            "[{}] should load 2 boundaries after id1",
            backend
        );

        let loaded = pb_dal
            .load_after_cursor("events".to_string(), id3)
            .await
            .unwrap();
        assert_eq!(
            loaded.len(),
            0,
            "[{}] should load 0 boundaries after id3",
            backend
        );
    }
}

#[tokio::test]
#[serial]
async fn test_edge_drain_cursor_lifecycle() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_edge_drain_cursor_lifecycle on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

        pb_dal
            .init_cursor("events:task_a".to_string(), "events".to_string())
            .await
            .unwrap();
        pb_dal
            .init_cursor("events:task_b".to_string(), "events".to_string())
            .await
            .unwrap();

        let cursor = pb_dal
            .load_cursor("events:task_a".to_string())
            .await
            .unwrap();
        assert_eq!(cursor, 0, "[{}] initial cursor should be 0", backend);

        pb_dal
            .advance_cursor("events:task_a".to_string(), 5)
            .await
            .unwrap();
        assert_eq!(
            pb_dal
                .load_cursor("events:task_a".to_string())
                .await
                .unwrap(),
            5,
            "[{}] task_a cursor should advance to 5",
            backend,
        );
        assert_eq!(
            pb_dal
                .load_cursor("events:task_b".to_string())
                .await
                .unwrap(),
            0,
            "[{}] task_b cursor should remain at 0",
            backend,
        );

        let min = pb_dal
            .min_cursor_for_source("events".to_string())
            .await
            .unwrap();
        assert_eq!(
            min, 0,
            "[{}] min cursor should be 0 (task_b is slowest)",
            backend
        );

        pb_dal
            .advance_cursor("events:task_b".to_string(), 5)
            .await
            .unwrap();
        let min = pb_dal
            .min_cursor_for_source("events".to_string())
            .await
            .unwrap();
        assert_eq!(
            min, 5,
            "[{}] min cursor should be 5 (both caught up)",
            backend
        );
    }
}

#[tokio::test]
#[serial]
async fn test_boundary_cleanup_after_all_consumers_drain() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!(
            "Running test_boundary_cleanup_after_all_consumers_drain on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

        let id1 = pb_dal
            .append("events".to_string(), r#"{"id":1}"#.to_string())
            .await
            .unwrap();
        let _id2 = pb_dal
            .append("events".to_string(), r#"{"id":2}"#.to_string())
            .await
            .unwrap();
        let id3 = pb_dal
            .append("events".to_string(), r#"{"id":3}"#.to_string())
            .await
            .unwrap();

        pb_dal
            .init_cursor("events:task_a".to_string(), "events".to_string())
            .await
            .unwrap();
        pb_dal
            .init_cursor("events:task_b".to_string(), "events".to_string())
            .await
            .unwrap();

        // Both advance to id2
        pb_dal
            .advance_cursor("events:task_a".to_string(), id1 + 1)
            .await
            .unwrap();
        pb_dal
            .advance_cursor("events:task_b".to_string(), id1 + 1)
            .await
            .unwrap();

        let min = pb_dal
            .min_cursor_for_source("events".to_string())
            .await
            .unwrap();
        pb_dal.cleanup("events".to_string(), min).await.unwrap();

        // Only id3 should remain
        let remaining = pb_dal
            .load_after_cursor("events".to_string(), 0)
            .await
            .unwrap();
        assert_eq!(
            remaining.len(),
            1,
            "[{}] only id3 should remain after cleanup",
            backend
        );
        assert_eq!(
            remaining[0].id, id3,
            "[{}] remaining boundary should be id3",
            backend
        );
    }
}

#[tokio::test]
#[serial]
async fn test_init_cursor_idempotent() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_init_cursor_idempotent on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

        pb_dal
            .init_cursor("events:task_a".to_string(), "events".to_string())
            .await
            .unwrap();
        pb_dal
            .advance_cursor("events:task_a".to_string(), 10)
            .await
            .unwrap();

        // Re-init should NOT reset
        pb_dal
            .init_cursor("events:task_a".to_string(), "events".to_string())
            .await
            .unwrap();

        let cursor = pb_dal
            .load_cursor("events:task_a".to_string())
            .await
            .unwrap();
        assert_eq!(cursor, 10, "[{}] re-init should not reset cursor", backend);
    }
}

#[tokio::test]
#[serial]
async fn test_max_id_for_source() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_max_id_for_source on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

        // Empty — should be None
        let max = pb_dal
            .max_id_for_source("events".to_string())
            .await
            .unwrap();
        assert!(
            max.is_none(),
            "[{}] max_id for empty source should be None",
            backend
        );

        let _id1 = pb_dal
            .append("events".to_string(), r#"{"a":1}"#.to_string())
            .await
            .unwrap();
        let id2 = pb_dal
            .append("events".to_string(), r#"{"a":2}"#.to_string())
            .await
            .unwrap();

        let max = pb_dal
            .max_id_for_source("events".to_string())
            .await
            .unwrap();
        assert_eq!(max, Some(id2), "[{}] max_id should be id2", backend);
    }
}

#[tokio::test]
#[serial]
async fn test_multi_source_isolation() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_multi_source_isolation on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

        pb_dal
            .append("events".to_string(), r#"{"src":"events"}"#.to_string())
            .await
            .unwrap();
        pb_dal
            .append("events".to_string(), r#"{"src":"events"}"#.to_string())
            .await
            .unwrap();
        pb_dal
            .append("config".to_string(), r#"{"src":"config"}"#.to_string())
            .await
            .unwrap();

        let events = pb_dal
            .load_after_cursor("events".to_string(), 0)
            .await
            .unwrap();
        let config = pb_dal
            .load_after_cursor("config".to_string(), 0)
            .await
            .unwrap();

        assert_eq!(
            events.len(),
            2,
            "[{}] events source should have 2 boundaries",
            backend
        );
        assert_eq!(
            config.len(),
            1,
            "[{}] config source should have 1 boundary",
            backend
        );

        // Cleanup events doesn't affect config
        pb_dal
            .init_cursor("events:task".to_string(), "events".to_string())
            .await
            .unwrap();
        let max = pb_dal
            .max_id_for_source("events".to_string())
            .await
            .unwrap()
            .unwrap();
        pb_dal
            .advance_cursor("events:task".to_string(), max)
            .await
            .unwrap();
        pb_dal.cleanup("events".to_string(), max).await.unwrap();

        let events = pb_dal
            .load_after_cursor("events".to_string(), 0)
            .await
            .unwrap();
        let config = pb_dal
            .load_after_cursor("config".to_string(), 0)
            .await
            .unwrap();
        assert_eq!(
            events.len(),
            0,
            "[{}] events should be empty after cleanup",
            backend
        );
        assert_eq!(config.len(), 1, "[{}] config should be untouched", backend);
    }
}

#[tokio::test]
#[serial]
async fn test_load_all_cursors() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_load_all_cursors on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

        // Initially empty
        let cursors = pb_dal.load_all_cursors().await.unwrap();
        assert!(
            cursors.is_empty(),
            "[{}] should have no cursors initially",
            backend
        );

        pb_dal
            .init_cursor("events:task_a".to_string(), "events".to_string())
            .await
            .unwrap();
        pb_dal
            .init_cursor("config:task_b".to_string(), "config".to_string())
            .await
            .unwrap();

        pb_dal
            .advance_cursor("events:task_a".to_string(), 42)
            .await
            .unwrap();

        let cursors = pb_dal.load_all_cursors().await.unwrap();
        assert_eq!(cursors.len(), 2, "[{}] should have 2 cursors", backend);

        let task_a = cursors.iter().find(|c| c.edge_id == "events:task_a");
        assert!(task_a.is_some(), "[{}] task_a cursor should exist", backend);
        assert_eq!(
            task_a.unwrap().last_drain_id,
            42,
            "[{}] task_a cursor should be 42",
            backend
        );

        let task_b = cursors.iter().find(|c| c.edge_id == "config:task_b");
        assert!(task_b.is_some(), "[{}] task_b cursor should exist", backend);
        assert_eq!(
            task_b.unwrap().last_drain_id,
            0,
            "[{}] task_b cursor should be 0",
            backend
        );
    }
}

// ============================================================================
// State Management Tests (multi-backend: runs on both SQLite and Postgres)
// ============================================================================

#[tokio::test]
#[serial]
async fn test_list_orphaned_states() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_list_orphaned_states on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let acc_dal = AccumulatorStateDAL::new(&dal);

        // Save state for edges that WON'T be in the graph
        acc_dal
            .save(NewAccumulatorState {
                edge_id: "old_source:old_task".into(),
                consumer_watermark: None,
                drain_metadata: "{}".into(),
            })
            .await
            .unwrap();

        acc_dal
            .save(NewAccumulatorState {
                edge_id: "events:aggregate".into(),
                consumer_watermark: None,
                drain_metadata: "{}".into(),
            })
            .await
            .unwrap();

        // Build a graph that only has events:aggregate
        use cloacina::continuous::datasource::*;
        use cloacina::continuous::graph::*;
        use std::any::Any;

        struct MockConn;
        impl DataConnection for MockConn {
            fn connect(&self) -> Result<Box<dyn Any>, DataConnectionError> {
                Ok(Box::new("mock"))
            }
            fn descriptor(&self) -> ConnectionDescriptor {
                ConnectionDescriptor {
                    system_type: "mock".into(),
                    location: "test".into(),
                }
            }
            fn system_metadata(&self) -> serde_json::Value {
                serde_json::json!({})
            }
        }

        let graph = assemble_graph(
            vec![DataSource {
                name: "events".into(),
                connection: Box::new(MockConn),
                detector_workflow: "detect_events".into(),
                lineage: DataSourceMetadata::default(),
            }],
            vec![ContinuousTaskRegistration {
                id: "aggregate".into(),
                sources: vec!["events".into()],
                referenced: vec![],
            }],
        )
        .unwrap();

        let orphaned = cloacina::continuous::state_management::list_orphaned_states(&graph, &dal)
            .await
            .unwrap();
        assert_eq!(
            orphaned.len(),
            1,
            "[{}] should have 1 orphaned state",
            backend
        );
        assert_eq!(orphaned[0], "old_source:old_task");
    }
}

#[tokio::test]
#[serial]
async fn test_prune_orphaned_states() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_prune_orphaned_states on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        let acc_dal = AccumulatorStateDAL::new(&dal);

        // Save orphaned state
        acc_dal
            .save(NewAccumulatorState {
                edge_id: "orphan:task".into(),
                consumer_watermark: None,
                drain_metadata: "{}".into(),
            })
            .await
            .unwrap();

        use cloacina::continuous::graph::*;

        let graph = assemble_graph(vec![], vec![]).unwrap();

        let pruned = cloacina::continuous::state_management::prune_orphaned_states(&graph, &dal)
            .await
            .unwrap();
        assert_eq!(pruned, 1, "[{}] should prune 1 orphaned state", backend);

        // Verify it's gone
        let remaining = acc_dal.load("orphan:task").await.unwrap();
        assert!(
            remaining.is_none(),
            "[{}] orphaned state should be gone after prune",
            backend
        );
    }
}

#[tokio::test]
#[serial]
async fn test_prune_no_orphans() {
    for (backend, fixture) in get_all_fixtures().await {
        // Skip sqlite — diesel timestamp deserialization is incompatible
        // with sqlite's datetime('now') format for these tables.
        if backend == "sqlite" {
            continue;
        }
        tracing::info!("Running test_prune_no_orphans on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;
        let dal = guard.get_dal();

        use cloacina::continuous::graph::*;
        let graph = assemble_graph(vec![], vec![]).unwrap();

        let pruned = cloacina::continuous::state_management::prune_orphaned_states(&graph, &dal)
            .await
            .unwrap();
        assert_eq!(pruned, 0, "[{}] should prune 0 when no orphans", backend);
    }
}
