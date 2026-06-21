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

//! Complete implementation of the workflow registry.
//!
//! This module provides the `WorkflowRegistryImpl` that combines all registry
//! components - storage, loading, validation, and task registration - into a
//! cohesive system for managing packaged workflows.

mod database;
pub mod filesystem;
mod package;

pub use database::{build_queue_stats, reconciler_stats, BuildQueueStats, ReconcilerStats};

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::database::Database;
use crate::registry::error::RegistryError;
use crate::registry::loader::{PackageLoader, TaskRegistrar};
use crate::registry::traits::{RegistryStorage, WorkflowRegistry};
use crate::registry::types::{
    LoadedWorkflow, WorkflowMetadata, WorkflowPackageId, WorkflowSourceFile,
};
use crate::task::TaskNamespace;

/// Per-file size cap when extracting source for display (CLOACI-T-0750).
/// Files larger than this are omitted rather than streamed wholesale through
/// the API — workflow source files are small and a generous cap keeps a
/// pathological archive from ballooning a response.
const MAX_SOURCE_FILE_BYTES: u64 = 1024 * 1024;

/// Complete implementation of the workflow registry.
///
/// This registry implementation combines storage backends, package loading,
/// validation, and task registration to provide a full-featured system for
/// managing packaged workflows with proper lifecycle management.
pub struct WorkflowRegistryImpl<S: RegistryStorage> {
    /// Storage backend for binary data
    pub(super) storage: S,
    /// Database for metadata storage
    pub(super) database: Database,
    /// Package loader for metadata extraction (FFI-driven; the
    /// reconciler reads metadata via fidius directly).
    #[allow(dead_code)]
    loader: PackageLoader,
    /// Task registrar for global registry integration
    registrar: TaskRegistrar,
    /// Map of package IDs to registered task namespaces for cleanup tracking
    pub(super) loaded_packages: HashMap<Uuid, Vec<TaskNamespace>>,
}

impl<S: RegistryStorage> WorkflowRegistryImpl<S> {
    /// Create a new workflow registry implementation.
    ///
    /// # Arguments
    ///
    /// * `storage` - Storage backend for binary workflow data
    /// * `database` - Database for metadata storage
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowRegistryImpl)` - Successfully created registry
    /// * `Err(RegistryError)` - If creation fails
    pub fn new(storage: S, database: Database) -> Result<Self, RegistryError> {
        let loader = PackageLoader::new().map_err(RegistryError::Loader)?;
        let registrar = TaskRegistrar::new().map_err(RegistryError::Loader)?;

        Ok(Self {
            storage,
            database,
            loader,
            registrar,
            loaded_packages: HashMap::new(),
        })
    }

    /// Get the number of currently loaded packages.
    pub fn loaded_package_count(&self) -> usize {
        self.loaded_packages.len()
    }

    /// Get the total number of registered tasks across all packages.
    pub fn total_registered_tasks(&self) -> usize {
        self.loaded_packages.values().map(|tasks| tasks.len()).sum()
    }

    // ========================================================================
    // Public convenience methods for tests and direct usage
    // ========================================================================

    /// Register a workflow package (alias for register_workflow via the trait).
    ///
    /// This is a convenience method that provides the same functionality as
    /// the `register_workflow` trait method.
    pub async fn register_workflow_package(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<Uuid, RegistryError> {
        // Use the trait implementation directly
        WorkflowRegistry::register_workflow(self, package_data).await
    }

    /// Get the source archive bytes for a package the compiler service has
    /// claimed. Unlike `get_workflow_package_by_id`, this does *not* filter
    /// by `build_status = 'success'` — the compiler needs to read source for
    /// rows currently in `building` state, which the reconciler-facing filter
    /// would hide. Still excludes superseded rows.
    pub async fn get_source_for_build(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError> {
        let ins = match self.inspect_package_by_id(package_id).await? {
            Some(ins) => ins,
            None => return Ok(None),
        };
        let registry_id = ins.metadata.registry_id.to_string();
        let package_data = match self.storage.retrieve_binary(&registry_id).await? {
            Some(data) => data,
            None => {
                return Err(RegistryError::Internal(
                    "Package metadata exists but binary data is missing".to_string(),
                ));
            }
        };
        Ok(Some((ins.metadata, package_data)))
    }

    /// Extract the human-readable source files from a package's retained
    /// `.cloacina` archive for read-only display (CLOACI-T-0750).
    ///
    /// The original source tree is always kept in registry storage (the
    /// compiler unpacks it to build), independent of build status — so this
    /// works for `building` and `failed` rows too, not just `success`. The
    /// archive is unpacked in a temp dir and every UTF-8 text file is returned
    /// (path relative to the source root + contents); binary and oversized
    /// files (> `MAX_SOURCE_FILE_BYTES`) are skipped. Returns `None` when no
    /// inspectable package exists for `package_id`.
    pub async fn get_workflow_source(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(WorkflowMetadata, Vec<WorkflowSourceFile>)>, RegistryError> {
        let (metadata, archive_bytes) = match self.get_source_for_build(package_id).await? {
            Some(pair) => pair,
            None => return Ok(None),
        };

        // Unpacking (bzip2 + tar + filesystem walk) is blocking work; keep it
        // off the async runtime's worker threads.
        let files = tokio::task::spawn_blocking(move || extract_source_files(&archive_bytes))
            .await
            .map_err(|e| {
                RegistryError::Internal(format!("source extraction task panicked: {}", e))
            })??;

        Ok(Some((metadata, files)))
    }

    /// Whether the workflow addressed by `name` (its executable `workflow_name`
    /// or `package_name`) is paused (CLOACI-T-0749). Unknown workflows are
    /// treated as not paused so resolution/“not found” is handled downstream by
    /// the executor. Used by the execute chokepoint to refuse new runs.
    pub async fn is_workflow_paused(&self, name: &str) -> Result<bool, RegistryError> {
        let workflows = self.list_workflows().await?;
        Ok(workflows
            .into_iter()
            .find(|w| w.workflow_name == name || w.package_name == name)
            .map(|w| w.paused)
            .unwrap_or(false))
    }

    /// Declared input params for the workflow addressed by `name` (its
    /// `workflow_name` or `package_name`), or empty when the workflow is unknown
    /// or declares none (CLOACI-I-0128). The execute chokepoint validates the
    /// provided context against these.
    pub async fn get_workflow_declared_params(
        &self,
        name: &str,
    ) -> Result<Vec<cloacina_api_types::InputSlot>, RegistryError> {
        let workflows = self.list_workflows().await?;
        Ok(workflows
            .into_iter()
            .find(|w| w.workflow_name == name || w.package_name == name)
            .map(|w| w.declared_params)
            .unwrap_or_default())
    }

    /// Declared input slots for a non-workflow injectable surface
    /// (`kind` = `"graph"` / `"reactor"` / `"accumulator"`) addressed by `name`,
    /// scanned across all registered packages. Empty when the surface is unknown
    /// or declares no typed interface (CLOACI-I-0128 T-0758). The operator
    /// fire/inject endpoints validate payloads against these (Task E).
    pub async fn find_surface_input_slots(
        &self,
        kind: &str,
        name: &str,
    ) -> Result<Vec<cloacina_api_types::InputSlot>, RegistryError> {
        let workflows = self.list_workflows().await?;
        for w in workflows {
            for surface in w.declared_surfaces {
                if surface.kind == kind && surface.name == name {
                    return Ok(surface.slots);
                }
            }
        }
        Ok(Vec::new())
    }

    /// The declared input slot for an accumulator addressed by `name`
    /// (CLOACI-I-0128 T-0758). An accumulator's boundary type is carried as a
    /// per-source slot inside its graph/reactor surface, so this scans all
    /// surfaces' slots for one whose name matches. `None` when unknown or
    /// untyped. The accumulator-inject endpoint validates the pushed event
    /// against this (Task E).
    pub async fn find_accumulator_input_slot(
        &self,
        name: &str,
    ) -> Result<Option<cloacina_api_types::InputSlot>, RegistryError> {
        let workflows = self.list_workflows().await?;
        for w in workflows {
            for surface in w.declared_surfaces {
                if let Some(slot) = surface.slots.into_iter().find(|s| s.name == name) {
                    return Ok(Some(slot));
                }
            }
        }
        Ok(None)
    }

    /// Pause or resume the workflow addressed by `name` (CLOACI-T-0749).
    /// Resolves `name` against the active package list (by `workflow_name` or
    /// `package_name`) and sets its `paused` flag. Returns the affected package
    /// id, or `None` if no matching active workflow exists.
    pub async fn set_workflow_paused(
        &self,
        name: &str,
        paused: bool,
    ) -> Result<Option<Uuid>, RegistryError> {
        let workflows = self.list_workflows().await?;
        let Some(target) = workflows
            .into_iter()
            .find(|w| w.workflow_name == name || w.package_name == name)
        else {
            return Ok(None);
        };
        self.set_package_paused(target.id, paused).await?;
        Ok(Some(target.id))
    }

    /// Get a workflow package by its UUID.
    ///
    /// Returns the package metadata and binary data if found.
    pub async fn get_workflow_package_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError> {
        // Get metadata from database
        let (registry_id, metadata, _compiled) =
            match self.get_package_metadata_by_id(package_id).await? {
                Some(data) => data,
                None => return Ok(None),
            };

        // Get binary data from storage
        let package_data = match self.storage.retrieve_binary(&registry_id).await? {
            Some(data) => data,
            None => {
                return Err(RegistryError::Internal(
                    "Package metadata exists but binary data is missing".to_string(),
                ));
            }
        };

        Ok(Some((metadata, package_data)))
    }

    /// Get a workflow package by name and version.
    ///
    /// Returns the package metadata and binary data if found.
    pub async fn get_workflow_package_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(WorkflowMetadata, Vec<u8>)>, RegistryError> {
        // Use the trait implementation and convert the result
        match self.get_workflow(package_name, version).await? {
            Some(loaded) => Ok(Some((loaded.metadata, loaded.package_data))),
            None => Ok(None),
        }
    }

    /// Check if a package exists by ID.
    pub async fn exists_by_id(&self, package_id: Uuid) -> Result<bool, RegistryError> {
        Ok(self.get_package_metadata_by_id(package_id).await?.is_some())
    }

    /// Check if a package exists by name and version.
    pub async fn exists_by_name(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<bool, RegistryError> {
        Ok(self
            .get_package_metadata(package_name, version)
            .await?
            .is_some())
    }

    /// List all packages in the registry.
    ///
    /// Returns metadata for all registered packages.
    pub async fn list_packages(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        self.list_all_packages().await
    }

    /// Unregister a workflow package by ID.
    pub async fn unregister_workflow_package_by_id(
        &mut self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        // Get package metadata to find the registry_id for storage cleanup
        let (registry_id, _metadata, _compiled) =
            match self.get_package_metadata_by_id(package_id).await? {
                Some(data) => data,
                None => return Ok(()), // Idempotent - already doesn't exist
            };

        // Unregister tasks from global registry
        if let Some(_namespaces) = self.loaded_packages.remove(&package_id) {
            self.registrar
                .unregister_package_tasks(&package_id.to_string())
                .map_err(RegistryError::Loader)?;
        }

        // Delete metadata from database
        self.delete_package_metadata_by_id(package_id).await?;

        // Delete binary data from storage
        self.storage.delete_binary(&registry_id).await?;

        Ok(())
    }

    /// Unregister a workflow package by name and version.
    pub async fn unregister_workflow_package_by_name(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        // Check if package exists first
        if self
            .get_package_metadata(package_name, version)
            .await?
            .is_none()
        {
            return Ok(()); // Idempotent - already doesn't exist
        }

        // Use the trait implementation
        self.unregister_workflow(package_name, version).await
    }
}

#[async_trait]
impl<S: RegistryStorage + Send + Sync> WorkflowRegistry for WorkflowRegistryImpl<S> {
    async fn register_workflow(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<WorkflowPackageId, RegistryError> {
        // 1. Require a bzip2 source archive (.cloacina)
        if !Self::is_cloacina_package(&package_data) {
            return Err(RegistryError::ValidationError {
                reason: "Package data is not a valid .cloacina bzip2 source archive. \
                         Raw library registration is not supported."
                    .to_string(),
            });
        }

        // 2. Read the manifest to extract package name/version for duplicate checking.
        //    We do this by writing to a temp dir and calling unpack + load_manifest.
        let work_dir = tempfile::TempDir::new()
            .map_err(|e| RegistryError::Internal(format!("Failed to create temp dir: {}", e)))?;
        let archive_path = work_dir.path().join("pkg.cloacina");
        std::fs::write(&archive_path, &package_data)
            .map_err(|e| RegistryError::Internal(format!("Failed to write archive: {}", e)))?;
        let extract_dir = work_dir.path().join("source");
        std::fs::create_dir_all(&extract_dir)
            .map_err(|e| RegistryError::Internal(format!("Failed to create extract dir: {}", e)))?;

        let source_dir = fidius_core::package::unpack_package(&archive_path, &extract_dir)
            .map_err(|e| RegistryError::ValidationError {
                reason: format!("Failed to unpack source archive: {}", e),
            })?;

        let manifest = fidius_core::package::load_manifest::<
            cloacina_workflow_plugin::CloacinaMetadata,
        >(&source_dir)
        .map_err(|e| RegistryError::ValidationError {
            reason: format!("Failed to load package.toml: {}", e),
        })?;

        let pkg_name = manifest.package.name.clone();
        let pkg_version = manifest.package.version.clone();

        // 3. Content hash for idempotency + audit of what's installed.
        let content_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&package_data);
            format!("{:x}", hasher.finalize())
        };

        // 4. Look up the currently-active row for this package name.
        //    - Same hash  → idempotent, return existing id (no storage churn, no supersede)
        //    - Different  → supersede old + insert new atomically
        //    - None       → insert new
        let active = self.get_active_package_by_name(&pkg_name).await?;
        if let Some((existing_id, _, ref existing_hash)) = active {
            if existing_hash == &content_hash {
                return Ok(existing_id);
            }
        }

        let package_metadata = crate::registry::loader::package_loader::PackageMetadata {
            package_name: pkg_name,
            // Workflow name from the package.toml manifest — available at upload
            // (pre-build); the build-success extraction later overwrites the full
            // metadata (incl. tasks) with the authoritative cdylib values.
            workflow_name: manifest.metadata.workflow_name.clone().unwrap_or_default(),
            version: pkg_version,
            description: manifest.metadata.description.clone(),
            author: manifest.metadata.author.clone(),
            tasks: vec![],
            graph_data: None,
            architecture: std::env::consts::ARCH.to_string(),
            symbols: vec![],
            workflow_triggers: vec![],
            // Filled at build success from the cdylib's input-interface
            // entrypoint (CLOACI-I-0128); empty at upload.
            declared_params: vec![],
            declared_surfaces: vec![],
        };

        let registry_id = self.storage.store_binary(package_data).await?;

        let old_id = active.map(|(id, _, _)| id);

        // Content-hash artifact reuse (T-0523): if an earlier row compiled the
        // same bytes successfully, skip the build queue and pre-populate the
        // new row as `success` with the existing compiled_data.
        let prebuilt = self.find_success_by_hash(&content_hash).await?;
        let prebuilt_bytes = prebuilt.map(|(_, bytes)| bytes);

        let package_id = self
            .supersede_and_insert_with_prebuilt(
                old_id,
                &registry_id,
                &package_metadata,
                &content_hash,
                prebuilt_bytes,
            )
            .await?;

        Ok(package_id)
    }

    async fn get_workflow(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<LoadedWorkflow>, RegistryError> {
        // 1. Get metadata + compiled bytes from database (success rows only)
        let (registry_id, package_metadata, compiled_data) =
            match self.get_package_metadata(package_name, version).await? {
                Some(data) => data,
                None => return Ok(None),
            };

        // 2. Retrieve source archive from storage
        let package_data = match self.storage.retrieve_binary(&registry_id).await? {
            Some(data) => data,
            None => {
                return Err(RegistryError::Internal(
                    "Package metadata exists but binary data is missing".to_string(),
                ));
            }
        };

        // 3. Create loaded workflow
        let workflow_metadata = WorkflowMetadata {
            id: Uuid::new_v4(), // This should be the actual package ID from the database
            registry_id: Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?,
            workflow_name: if package_metadata.workflow_name.is_empty() {
                package_metadata.package_name.clone()
            } else {
                package_metadata.workflow_name.clone()
            },
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            tasks: package_metadata
                .tasks
                .iter()
                .map(|t| t.local_id.clone())
                .collect(),
            task_graph: database::build_task_graph(&package_metadata),
            schedules: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            // This load path is for execution, not the pause gate; the gate
            // reads paused via the list/inspect paths. (CLOACI-T-0749)
            paused: false,
            declared_params: package_metadata.declared_params.clone(),
            declared_surfaces: package_metadata.declared_surfaces.clone(),
        };

        Ok(Some(LoadedWorkflow {
            metadata: workflow_metadata,
            package_data,
            compiled_data,
        }))
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        self.list_all_packages().await
    }

    async fn persist_task_graph(
        &self,
        package_id: crate::registry::types::WorkflowPackageId,
        tasks: Vec<(String, Vec<String>)>,
    ) -> Result<(), RegistryError> {
        self.persist_task_graph_db(package_id, tasks).await
    }

    async fn unregister_workflow(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        // 1. Get package metadata to find the package ID
        let (registry_id, _, _) = self
            .get_package_metadata(package_name, version)
            .await?
            .ok_or_else(|| RegistryError::PackageNotFound {
                package_name: package_name.to_string(),
                version: version.to_string(),
            })?;

        // 2. Find the package ID to unregister tasks
        let package_uuid = Uuid::parse_str(&registry_id).map_err(RegistryError::InvalidUuid)?;

        // 3. Unregister tasks from global registry
        if let Some(_namespaces) = self.loaded_packages.remove(&package_uuid) {
            self.registrar
                .unregister_package_tasks(&package_uuid.to_string())
                .map_err(RegistryError::Loader)?;
        }

        // 4. Delete metadata from database (this will cascade to registry storage via foreign key)
        self.delete_package_metadata(package_name, version).await?;

        // 5. Delete binary data from storage
        self.storage.delete_binary(&registry_id).await?;

        Ok(())
    }

    /// Defense-in-depth signature existence check (CLOACI-T-0571).
    ///
    /// Queries `package_signatures` for any row with the given hash.
    /// Used by the reconciler when `require_signatures` is on.
    async fn find_signature(&self, package_hash: &str) -> Result<bool, RegistryError> {
        use crate::security::{DbPackageSigner, PackageSigner};
        let signer = DbPackageSigner::new(crate::dal::DAL::new(self.database.clone()));
        match signer.find_signature(package_hash).await {
            Ok(opt) => Ok(opt.is_some()),
            Err(e) => Err(RegistryError::Internal(format!(
                "find_signature DAL query failed for hash {}: {}",
                package_hash, e
            ))),
        }
    }
}

/// Unpack a `.cloacina` source archive in a temp dir and collect its UTF-8
/// text files for display (CLOACI-T-0750). Binary, oversized, and unreadable
/// files are skipped; the result is sorted by path. The temp dir is removed
/// when the returned `TempDir` guard drops at end of scope.
fn extract_source_files(archive_bytes: &[u8]) -> Result<Vec<WorkflowSourceFile>, RegistryError> {
    let work_dir = tempfile::TempDir::new()
        .map_err(|e| RegistryError::Internal(format!("Failed to create temp dir: {}", e)))?;
    let archive_path = work_dir.path().join("pkg.cloacina");
    std::fs::write(&archive_path, archive_bytes)
        .map_err(|e| RegistryError::Internal(format!("Failed to write archive: {}", e)))?;
    let extract_dir = work_dir.path().join("source");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| RegistryError::Internal(format!("Failed to create extract dir: {}", e)))?;

    let source_dir =
        fidius_core::package::unpack_package(&archive_path, &extract_dir).map_err(|e| {
            RegistryError::ValidationError {
                reason: format!("Failed to unpack source archive: {}", e),
            }
        })?;

    let mut files = Vec::new();
    collect_source_files(&source_dir, &source_dir, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Recursively walk `dir`, pushing each UTF-8 text file (path relative to
/// `root`) into `out`. Binary, oversized, and unreadable files are silently
/// skipped so a single odd file never fails the whole request.
fn collect_source_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<WorkflowSourceFile>,
) -> Result<(), RegistryError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| RegistryError::Internal(format!("Failed to read source dir: {}", e)))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| RegistryError::Internal(format!("Failed to read dir entry: {}", e)))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| RegistryError::Internal(format!("Failed to stat dir entry: {}", e)))?;

        if file_type.is_dir() {
            collect_source_files(root, &path, out)?;
        } else if file_type.is_file() {
            // Skip oversized files without reading them into memory.
            if let Ok(meta) = entry.metadata() {
                if meta.len() > MAX_SOURCE_FILE_BYTES {
                    continue;
                }
            }
            // Only surface valid UTF-8; binary files are skipped.
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(contents) = String::from_utf8(bytes) else {
                continue;
            };
            let rel = path.strip_prefix(root).unwrap_or(&path);
            out.push(WorkflowSourceFile {
                path: rel.to_string_lossy().replace('\\', "/"),
                contents,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::registry::storage::FilesystemRegistryStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_registry_creation() {
        let temp_dir = TempDir::new().unwrap();
        let _storage = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();

        // Note: This test would need a proper database setup
        // For now, we'll just test the storage creation part
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_registry_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let _storage = FilesystemRegistryStorage::new(temp_dir.path()).unwrap();

        // This would need a database for full testing
        // For now just test that we can create the storage
        assert!(temp_dir.path().exists());
    }
}
