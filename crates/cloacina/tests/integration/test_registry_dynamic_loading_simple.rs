/*
 *  Copyright 2025 Colliery Software
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

//! Integration test for dynamic loading functionality.
//!
//! This test verifies that the reconciler can be created with the new
//! dynamic loading components and that the basic workflow works.

use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::watch;

use cloacina::{
    registry::{
        reconciler::{ReconcilerConfig, RegistryReconciler},
        storage::FilesystemRegistryStorage,
        workflow_registry::WorkflowRegistryImpl,
    },
    Database,
};

/// Test that verifies the reconciler can be created with dynamic loading enabled
/// This test uses SQLite in-memory database
#[tokio::test]
async fn test_reconciler_with_dynamic_loading() {
    // Create temporary database
    let database = Database::new(":memory:", "", 5);

    // Run migrations
    let conn = database.pool().get().await.unwrap();
    conn.interact(move |conn| cloacina::database::run_migrations_sqlite(conn))
        .await
        .unwrap()
        .unwrap();

    // Create temporary storage
    let temp_dir = TempDir::new().unwrap();
    let storage = FilesystemRegistryStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Create workflow registry
    let workflow_registry = WorkflowRegistryImpl::new(storage, database).unwrap();
    let workflow_registry_arc = Arc::new(workflow_registry);

    // Create reconciler config
    let config = ReconcilerConfig::default();

    // Create shutdown channel
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    // Create reconciler - this should succeed with the new dynamic loading components
    let reconciler_result = RegistryReconciler::new(workflow_registry_arc, config, shutdown_rx);

    assert!(
        reconciler_result.is_ok(),
        "Reconciler should be created successfully with dynamic loading components"
    );

    let reconciler = reconciler_result.unwrap();

    // Get initial status
    let status = reconciler.get_status().await;

    // Should start with no packages loaded
    assert_eq!(status.packages_loaded, 0);
    assert_eq!(status.package_details.len(), 0);
}
