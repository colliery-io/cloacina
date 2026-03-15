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
