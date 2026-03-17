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

//! Concurrency tests for pipeline claiming operations.
//!
//! These tests verify that the pipeline outbox claiming mechanism prevents
//! race conditions where multiple scheduler instances might claim the same
//! pipeline simultaneously.
//!
//! Tests run on all enabled backends (SQLite, PostgreSQL) using `get_all_fixtures()`.

use crate::fixtures::get_all_fixtures;
use cloacina::dal::DAL;
use cloacina::models::pipeline_execution::NewPipelineExecution;
use std::collections::HashSet;

/// Test that two sequential claim_pipeline_batch calls return non-overlapping results.
///
/// This test creates multiple pipelines with outbox entries and verifies that:
/// 1. No pipeline is claimed by more than one batch
/// 2. All pipelines are eventually claimed exactly once
/// 3. The outbox is empty after all claims
#[tokio::test]
async fn test_pipeline_claiming_no_duplicates() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!(
            "Running test_pipeline_claiming_no_duplicates on {}",
            backend
        );

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        const NUM_PIPELINES: usize = 10;
        let mut created_ids = Vec::new();

        // Create pipelines and insert outbox entries
        for i in 0..NUM_PIPELINES {
            let pipeline = dal
                .pipeline_execution()
                .create(NewPipelineExecution {
                    pipeline_name: format!("claim-test-pipeline-{}", i),
                    pipeline_version: "1.0".to_string(),
                    status: "Pending".to_string(),
                    context_id: None,
                })
                .await
                .expect("Failed to create pipeline");

            dal.pipeline_execution()
                .insert_outbox(pipeline.id)
                .await
                .expect("Failed to insert outbox entry");

            created_ids.push(pipeline.id);
        }

        // First claim: get 5
        let batch1 = dal
            .pipeline_execution()
            .claim_pipeline_batch(5)
            .await
            .expect("Failed to claim first batch");

        assert_eq!(
            batch1.len(),
            5,
            "[{}] First batch should contain 5 pipelines, got {}",
            backend,
            batch1.len()
        );

        // Second claim: get remaining 5
        let batch2 = dal
            .pipeline_execution()
            .claim_pipeline_batch(5)
            .await
            .expect("Failed to claim second batch");

        assert_eq!(
            batch2.len(),
            5,
            "[{}] Second batch should contain 5 pipelines, got {}",
            backend,
            batch2.len()
        );

        // Third claim: outbox should be empty
        let batch3 = dal
            .pipeline_execution()
            .claim_pipeline_batch(5)
            .await
            .expect("Failed to claim third batch");

        assert!(
            batch3.is_empty(),
            "[{}] Third batch should be empty, got {}",
            backend,
            batch3.len()
        );

        // Verify no duplicates: union should be 10, intersection should be 0
        let ids1: HashSet<_> = batch1.iter().map(|p| p.id).collect();
        let ids2: HashSet<_> = batch2.iter().map(|p| p.id).collect();

        let intersection: HashSet<_> = ids1.intersection(&ids2).collect();
        assert!(
            intersection.is_empty(),
            "[{}] RACE CONDITION: pipelines claimed in both batches: {:?}",
            backend,
            intersection
        );

        let all_claimed: HashSet<_> = ids1.union(&ids2).collect();
        assert_eq!(
            all_claimed.len(),
            NUM_PIPELINES,
            "[{}] Expected {} unique pipelines claimed, got {}",
            backend,
            NUM_PIPELINES,
            all_claimed.len()
        );

        // Verify all claimed pipelines are from our created set
        let created_set: HashSet<_> = created_ids.iter().collect();
        for id in &all_claimed {
            assert!(
                created_set.contains(id),
                "[{}] Claimed pipeline {:?} was not in created set",
                backend,
                id
            );
        }

        tracing::info!(
            "[{}] Pipeline claiming test passed: {} pipelines claimed in 2 non-overlapping batches",
            backend,
            all_claimed.len()
        );
    }
}

/// Test that requeued pipelines can be reclaimed.
#[tokio::test]
async fn test_pipeline_requeue_and_reclaim() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running test_pipeline_requeue_and_reclaim on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        // Create 3 pipelines with outbox entries
        let mut created_ids = Vec::new();
        for i in 0..3 {
            let pipeline = dal
                .pipeline_execution()
                .create(NewPipelineExecution {
                    pipeline_name: format!("requeue-test-{}", i),
                    pipeline_version: "1.0".to_string(),
                    status: "Running".to_string(),
                    context_id: None,
                })
                .await
                .expect("Failed to create pipeline");

            dal.pipeline_execution()
                .insert_outbox(pipeline.id)
                .await
                .expect("Failed to insert outbox entry");

            created_ids.push(pipeline.id);
        }

        // Claim all 3
        let batch = dal
            .pipeline_execution()
            .claim_pipeline_batch(10)
            .await
            .expect("Failed to claim batch");

        assert_eq!(batch.len(), 3, "[{}] Should claim all 3 pipelines", backend);

        // Requeue the first one
        dal.pipeline_execution()
            .requeue_pipeline(created_ids[0])
            .await
            .expect("Failed to requeue pipeline");

        // Claim again -- should get exactly the requeued one
        let reclaimed = dal
            .pipeline_execution()
            .claim_pipeline_batch(10)
            .await
            .expect("Failed to reclaim");

        assert_eq!(
            reclaimed.len(),
            1,
            "[{}] Should reclaim exactly 1 pipeline, got {}",
            backend,
            reclaimed.len()
        );
        assert_eq!(
            reclaimed[0].id, created_ids[0],
            "[{}] Reclaimed pipeline should be the one we requeued",
            backend
        );

        // Outbox should be empty now
        let empty = dal
            .pipeline_execution()
            .claim_pipeline_batch(10)
            .await
            .expect("Failed to claim empty batch");

        assert!(
            empty.is_empty(),
            "[{}] Outbox should be empty after reclaim",
            backend
        );

        tracing::info!("[{}] Pipeline requeue test passed", backend);
    }
}

/// Test that only Pending/Running pipelines are returned from claims.
#[tokio::test]
async fn test_claim_filters_by_status() {
    for (backend, fixture) in get_all_fixtures().await {
        tracing::info!("Running test_claim_filters_by_status on {}", backend);

        let mut guard = fixture.lock().unwrap_or_else(|e| e.into_inner());
        guard.reset_database().await;
        guard.initialize().await;

        let database = guard.get_database();
        let dal = DAL::new(database.clone());

        // Create a Completed pipeline with an outbox entry
        let completed = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "completed-pipeline".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Pending".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create pipeline");

        // Mark it completed
        dal.pipeline_execution()
            .mark_completed(completed.id)
            .await
            .expect("Failed to mark completed");

        // Insert outbox entry (simulating a stale entry)
        dal.pipeline_execution()
            .insert_outbox(completed.id)
            .await
            .expect("Failed to insert outbox entry");

        // Create a Pending pipeline with an outbox entry
        let pending = dal
            .pipeline_execution()
            .create(NewPipelineExecution {
                pipeline_name: "pending-pipeline".to_string(),
                pipeline_version: "1.0".to_string(),
                status: "Pending".to_string(),
                context_id: None,
            })
            .await
            .expect("Failed to create pipeline");

        dal.pipeline_execution()
            .insert_outbox(pending.id)
            .await
            .expect("Failed to insert outbox entry");

        // Claim -- should only get the Pending one
        let claimed = dal
            .pipeline_execution()
            .claim_pipeline_batch(10)
            .await
            .expect("Failed to claim");

        assert_eq!(
            claimed.len(),
            1,
            "[{}] Should claim only 1 pipeline (the Pending one), got {}",
            backend,
            claimed.len()
        );
        assert_eq!(
            claimed[0].id, pending.id,
            "[{}] Claimed pipeline should be the Pending one",
            backend
        );

        tracing::info!("[{}] Status filter test passed", backend);
    }
}
