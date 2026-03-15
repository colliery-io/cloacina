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

use crate::fixtures;
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
// Detector State DAL Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_detector_state_save_and_load() {
    let dal = get_fresh_dal().await;
    let ds_dal = cloacina::dal::unified::DetectorStateDAL::new(&dal);

    ds_dal
        .save(cloacina::dal::unified::models::NewDetectorState {
            source_name: "events".to_string(),
            committed_state: Some(r#"{"cursor": 100}"#.to_string()),
        })
        .await
        .unwrap();

    let loaded = ds_dal.load("events").await.unwrap();
    assert!(loaded.is_some());
    let row = loaded.unwrap();
    assert_eq!(row.source_name, "events");
    assert_eq!(row.committed_state, Some(r#"{"cursor": 100}"#.to_string()));
}

#[tokio::test]
#[serial]
async fn test_detector_state_upserts() {
    let dal = get_fresh_dal().await;
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
        Some(r#"{"cursor": 500}"#.to_string())
    );
}

#[tokio::test]
#[serial]
async fn test_detector_state_load_nonexistent() {
    let dal = get_fresh_dal().await;
    let ds_dal = cloacina::dal::unified::DetectorStateDAL::new(&dal);

    let loaded = ds_dal.load("nonexistent").await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
#[serial]
async fn test_detector_state_load_all() {
    let dal = get_fresh_dal().await;
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
    assert_eq!(all.len(), 2);
}

// ============================================================================
// Pending Boundary DAL Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_pending_boundary_append_and_load() {
    let dal = get_fresh_dal().await;
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

    assert!(id2 > id1);
    assert!(id3 > id2);

    let loaded = pb_dal
        .load_after_cursor("events".to_string(), 0)
        .await
        .unwrap();
    assert_eq!(loaded.len(), 3);

    let loaded = pb_dal
        .load_after_cursor("events".to_string(), id1)
        .await
        .unwrap();
    assert_eq!(loaded.len(), 2);

    let loaded = pb_dal
        .load_after_cursor("events".to_string(), id3)
        .await
        .unwrap();
    assert_eq!(loaded.len(), 0);
}

#[tokio::test]
#[serial]
async fn test_edge_drain_cursor_lifecycle() {
    let dal = get_fresh_dal().await;
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
    assert_eq!(cursor, 0);

    pb_dal
        .advance_cursor("events:task_a".to_string(), 5)
        .await
        .unwrap();
    assert_eq!(
        pb_dal
            .load_cursor("events:task_a".to_string())
            .await
            .unwrap(),
        5
    );
    assert_eq!(
        pb_dal
            .load_cursor("events:task_b".to_string())
            .await
            .unwrap(),
        0
    );

    let min = pb_dal
        .min_cursor_for_source("events".to_string())
        .await
        .unwrap();
    assert_eq!(min, 0); // B is slowest

    pb_dal
        .advance_cursor("events:task_b".to_string(), 5)
        .await
        .unwrap();
    let min = pb_dal
        .min_cursor_for_source("events".to_string())
        .await
        .unwrap();
    assert_eq!(min, 5); // Both caught up
}

#[tokio::test]
#[serial]
async fn test_boundary_cleanup_after_all_consumers_drain() {
    let dal = get_fresh_dal().await;
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
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, id3);
}

#[tokio::test]
#[serial]
async fn test_init_cursor_idempotent() {
    let dal = get_fresh_dal().await;
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
    assert_eq!(cursor, 10);
}

#[tokio::test]
#[serial]
async fn test_max_id_for_source() {
    let dal = get_fresh_dal().await;
    let pb_dal = cloacina::dal::unified::PendingBoundaryDAL::new(&dal);

    // Empty — should be None
    let max = pb_dal
        .max_id_for_source("events".to_string())
        .await
        .unwrap();
    assert!(max.is_none());

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
    assert_eq!(max, Some(id2));
}

#[tokio::test]
#[serial]
async fn test_multi_source_isolation() {
    let dal = get_fresh_dal().await;
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

    assert_eq!(events.len(), 2);
    assert_eq!(config.len(), 1);

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
    assert_eq!(events.len(), 0);
    assert_eq!(config.len(), 1); // Untouched
}
