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

//! Core trait definitions for the workflow registry system.
//!
//! This module defines the fundamental traits that enable pluggable storage
//! backends and consistent registry operations across different implementations.

use async_trait::async_trait;

use crate::models::workflow_packages::StorageType;
use crate::registry::error::{RegistryError, StorageError};
use crate::registry::types::{LoadedWorkflow, WorkflowMetadata, WorkflowPackageId};

/// Main trait for workflow registry operations.
///
/// The `WorkflowRegistry` trait defines the core operations for managing
/// packaged workflows. Implementations handle the coordination between
/// binary storage and metadata management.
///
/// # Implementation Notes
///
/// - Implementations should validate packages before registration
/// - Task namespace isolation must be maintained
/// - Version conflicts should be handled gracefully
/// - Cleanup should be thorough when unregistering workflows
///
/// # Examples
///
/// ```rust,no_run
/// use cloacina::registry::WorkflowRegistry;
/// use std::fs;
///
/// # async fn example(mut registry: impl WorkflowRegistry) -> Result<(), Box<dyn std::error::Error>> {
/// // Register a new workflow from file
/// let package_data = fs::read("analytics.cloacina")?;
/// let id = registry.register_workflow(package_data).await?;
///
/// // List all workflows
/// let workflows = registry.list_workflows().await?;
/// for workflow in workflows {
///     println!("{} v{}", workflow.package_name, workflow.version);
/// }
///
/// // Unregister a specific version
/// registry.unregister_workflow("analytics", "1.0.0").await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait WorkflowRegistry: Send + Sync {
    /// Register a new packaged workflow from binary data.
    ///
    /// This operation:
    /// 1. Extracts metadata from the binary package data
    /// 2. Validates the package structure and metadata
    /// 3. Stores the binary data in registry storage
    /// 4. Stores metadata in the database
    /// 5. Registers tasks with the global task registry
    ///
    /// # Arguments
    ///
    /// * `package_data` - Raw binary data of the .cloacina package file
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowPackageId)` - Unique identifier for the registered package
    /// * `Err(RegistryError)` - If registration fails
    ///
    /// # Errors
    ///
    /// - `RegistryError::PackageExists` - If package/version already exists
    /// - `RegistryError::ValidationError` - If package validation fails
    /// - `RegistryError::StorageError` - If storage operations fail
    /// - `RegistryError::Loader` - If metadata extraction fails
    async fn register_workflow(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<WorkflowPackageId, RegistryError>;

    /// Retrieve a specific workflow package by name and version.
    ///
    /// This operation:
    /// 1. Queries metadata from the database
    /// 2. Retrieves binary data from storage if found
    /// 3. Returns both metadata and binary for loading
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the workflow package
    /// * `version` - Specific version to retrieve
    ///
    /// # Returns
    ///
    /// * `Ok(Some(LoadedWorkflow))` - If the workflow exists
    /// * `Ok(None)` - If no matching workflow is found
    /// * `Err(RegistryError)` - If retrieval fails
    async fn get_workflow(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<LoadedWorkflow>, RegistryError>;

    /// List all registered workflows in the registry.
    ///
    /// Returns metadata for all workflows without loading binary data.
    /// This enables efficient browsing of available workflows.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<WorkflowMetadata>)` - List of all workflow metadata
    /// * `Err(RegistryError)` - If listing fails
    ///
    /// # Performance Note
    ///
    /// This operation only queries the metadata table, not the binary storage,
    /// making it efficient even with many registered workflows.
    async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, RegistryError>;

    /// Unregister a workflow package from the registry.
    ///
    /// This operation:
    /// 1. Unregisters all tasks from the global task registry
    /// 2. Removes metadata from the database
    /// 3. Deletes binary data from storage (via cascade)
    /// 4. Cleans up any active schedules
    ///
    /// # Arguments
    ///
    /// * `package_name` - Name of the workflow package
    /// * `version` - Specific version to unregister
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If unregistration succeeds
    /// * `Err(RegistryError)` - If unregistration fails
    ///
    /// # Errors
    ///
    /// - `RegistryError::PackageNotFound` - If package/version doesn't exist
    /// - `RegistryError::InUse` - If workflow has active executions
    async fn unregister_workflow(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError>;

    /// Check whether a `package_signatures` row exists for the given
    /// SHA-256 hash. Used by the reconciler's defense-in-depth
    /// signature-existence check (CLOACI-T-0571) when
    /// `--require-signatures` is on. Implementations that don't have a
    /// signature backing store return `Ok(false)`.
    ///
    /// # Arguments
    ///
    /// * `package_hash` — hex-encoded SHA-256 of the package source archive.
    async fn find_signature(&self, package_hash: &str) -> Result<bool, RegistryError> {
        // Default impl: no signature store. Implementations backed by the
        // unified DB schema override this to query `package_signatures`.
        let _ = package_hash;
        Ok(false)
    }

    /// Persist a workflow's task list (local ids + dependency edges) into the
    /// stored package metadata.
    ///
    /// This exists for the **Python** load path: Python packages produce no
    /// cdylib, so the cdylib-based metadata extraction in `mark_build_success`
    /// never runs and the row's `tasks` stay empty. The reconciler captures the
    /// task graph from the scoped Runtime at load time and calls this to write
    /// it back, so the API/UI can show the task DAG the same way Rust packages
    /// do. (CLOACI-T-0672)
    ///
    /// # Arguments
    /// * `package_id` — the workflow package row id.
    /// * `tasks` — `(local_task_id, [dependency_local_ids])` per task.
    ///
    /// Default impl is a no-op for registries that don't persist metadata
    /// (filesystem, in-memory, mocks).
    async fn persist_task_graph(
        &self,
        package_id: WorkflowPackageId,
        tasks: Vec<(String, Vec<String>)>,
    ) -> Result<(), RegistryError> {
        let _ = (package_id, tasks);
        Ok(())
    }

    /// Fetch the bundled constructor **providers** stored for a package
    /// (CLOACI-T-0836): `(provider_name, packed provider archive bytes)` pairs the
    /// reconciler unpacks into a `providers/` tree before resolving the package's
    /// `constructor!` nodes. Empty for packages that use no constructors.
    ///
    /// Default impl: no provider store (filesystem, in-memory, mocks).
    /// Implementations backed by the unified DB schema override this to query
    /// `package_providers`.
    async fn get_package_providers(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<(String, Vec<u8>)>, RegistryError> {
        let _ = (package_name, version);
        Ok(Vec::new())
    }

    /// CLOACI-T-0835: ask the compiler to REBUILD this package from its
    /// retained source, because its compiled artifact is stale (built against
    /// an older plugin ABI / interface version than this host expects).
    ///
    /// Returns `true` when a rebuild was actually scheduled. Default impl: not
    /// supported (`false`) — registries without a build pipeline (filesystem,
    /// in-memory, mocks) can't recompile. The unified-DB registry overrides
    /// this to flip `build_status` back to `pending` for the compiler to claim.
    async fn request_recompile(
        &self,
        package_id: WorkflowPackageId,
    ) -> Result<bool, RegistryError> {
        let _ = package_id;
        Ok(false)
    }
}

/// Trait for binary storage backends.
///
/// The `RegistryStorage` trait provides a simple key-value interface for
/// storing workflow binary data. This abstraction enables different storage
/// backends (PostgreSQL, object storage, filesystem) to be used interchangeably.
///
/// # Implementation Notes
///
/// - Storage should be content-addressable where possible
/// - Binary integrity should be verified
/// - Cleanup should be atomic with metadata deletion
///
/// # Examples
///
/// ```rust,no_run
/// use cloacina::registry::RegistryStorage;
///
/// # async fn example(mut storage: impl RegistryStorage) -> Result<(), Box<dyn std::error::Error>> {
/// // Store binary data
/// let data = std::fs::read("workflow.so")?;
/// let id = storage.store_binary(data).await?;
///
/// // Retrieve binary data
/// if let Some(data) = storage.retrieve_binary(&id).await? {
///     println!("Retrieved {} bytes", data.len());
/// }
///
/// // Delete when no longer needed
/// storage.delete_binary(&id).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait RegistryStorage: Send + Sync {
    /// Store binary workflow data in the storage backend.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw binary data (.so file contents)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Unique identifier for retrieving the data
    /// * `Err(StorageError)` - If storage fails
    ///
    /// # Implementation Note
    ///
    /// The returned ID should be suitable for use as a foreign key
    /// in the workflow_packages table.
    async fn store_binary(&mut self, data: Vec<u8>) -> Result<String, StorageError>;

    /// Retrieve binary workflow data from storage.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier from store_binary
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Vec<u8>))` - Binary data if found
    /// * `Ok(None)` - If no data exists for the given ID
    /// * `Err(StorageError)` - If retrieval fails
    async fn retrieve_binary(&self, id: &str) -> Result<Option<Vec<u8>>, StorageError>;

    /// Delete binary workflow data from storage.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier from store_binary
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If deletion succeeds (or data doesn't exist)
    /// * `Err(StorageError)` - If deletion fails
    ///
    /// # Note
    ///
    /// Implementations should be idempotent - deleting non-existent
    /// data should succeed silently.
    async fn delete_binary(&mut self, id: &str) -> Result<(), StorageError>;

    /// Returns the storage type for this backend.
    ///
    /// This is used to record where the binary data is stored
    /// in the workflow_packages metadata table.
    ///
    /// # Returns
    ///
    /// * `StorageType::Database` - For database-backed storage (PostgreSQL, SQLite)
    /// * `StorageType::Filesystem` - For filesystem-backed storage
    fn storage_type(&self) -> StorageType;
}
