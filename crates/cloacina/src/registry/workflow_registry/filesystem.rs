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

//! Filesystem-backed workflow registry for daemon mode.
//!
//! Implements `WorkflowRegistry` by scanning directories for `.cloacina` package
//! files. Packages live on disk — the filesystem IS the package store. SQLite
//! handles operational state (schedules, executions) separately.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::registry::error::RegistryError;
use crate::registry::traits::WorkflowRegistry;
use crate::registry::types::{LoadedWorkflow, WorkflowMetadata, WorkflowPackageId};

/// A `WorkflowRegistry` implementation backed by directories of `.cloacina` files.
///
/// The daemon uses this instead of the database-backed registry. Packages are
/// discovered by scanning watch directories for `.cloacina` files. Package data
/// is read from disk on demand — no blobs stored in the database.
///
/// Supports multiple watch directories so users can organize packages across
/// different locations (e.g., `~/.cloacina/packages/`, `/opt/workflows/`,
/// `~/my-project/packages/`).
pub struct FilesystemWorkflowRegistry {
    /// Directories to scan for `.cloacina` package files.
    watch_dirs: Vec<PathBuf>,
}

impl FilesystemWorkflowRegistry {
    /// Create a new filesystem registry watching the given directories.
    ///
    /// Directories that don't exist are logged as warnings but not rejected —
    /// they may be created later (e.g., on first package drop).
    pub fn new(watch_dirs: Vec<PathBuf>) -> Self {
        for dir in &watch_dirs {
            if !dir.exists() {
                warn!(
                    "Watch directory does not exist (will be watched if created later): {}",
                    dir.display()
                );
            }
        }
        Self { watch_dirs }
    }

    /// Scan all watch directories for `.cloacina` files.
    ///
    /// Returns a map of `(package_name, version)` -> `(path, archive_data, metadata)`.
    /// Corrupt or unreadable files are logged and skipped.
    fn scan_packages(&self) -> HashMap<(String, String), (PathBuf, WorkflowMetadata)> {
        let mut packages = HashMap::new();

        for dir in &self.watch_dirs {
            if !dir.exists() {
                debug!("Skipping non-existent watch directory: {}", dir.display());
                continue;
            }

            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(e) => {
                    warn!("Failed to read watch directory {}: {}", dir.display(), e);
                    continue;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        warn!("Failed to read directory entry: {}", e);
                        continue;
                    }
                };

                let path = entry.path();

                // Only process .cloacina files
                if path.extension().and_then(|e| e.to_str()) != Some("cloacina") {
                    continue;
                }

                // Unpack archive to a temp dir and read package.toml
                let tmp = match tempfile::TempDir::new() {
                    Ok(t) => t,
                    Err(e) => {
                        warn!("Failed to create temp dir for {}: {}", path.display(), e);
                        continue;
                    }
                };

                let source_dir = match fidius_core::package::unpack_package(&path, tmp.path()) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Skipping unreadable package {}: {}", path.display(), e);
                        continue;
                    }
                };

                match fidius_core::package::load_manifest::<
                    cloacina_workflow_plugin::CloacinaMetadata,
                >(&source_dir)
                {
                    Ok(manifest) => {
                        let package_name = manifest.package.name.clone();
                        let version = manifest.package.version.clone();

                        // Derive a stable package ID from name+version
                        let fingerprint = format!("{}:{}", package_name, version);
                        let id = uuid_from_fingerprint(&fingerprint);

                        let now = chrono::Utc::now();
                        let metadata = WorkflowMetadata {
                            id,
                            registry_id: id, // Same as id for filesystem registry
                            package_name: package_name.clone(),
                            version: version.clone(),
                            description: manifest.metadata.description.clone(),
                            author: manifest.metadata.author.clone(),
                            tasks: vec![],
                            schedules: Vec::new(),
                            created_at: now,
                            updated_at: now,
                        };

                        debug!(
                            "Found package: {} v{} at {}",
                            package_name,
                            version,
                            path.display()
                        );

                        // If duplicate (same name+version in different dirs), first one wins
                        packages
                            .entry((package_name, version))
                            .or_insert((path.clone(), metadata));
                    }
                    Err(e) => {
                        warn!("Skipping unreadable package {}: {}", path.display(), e);
                    }
                }
            }
        }

        packages
    }

    /// Find the file path for a package by name and version.
    fn find_package_path(&self, package_name: &str, version: &str) -> Option<PathBuf> {
        let packages = self.scan_packages();
        packages
            .get(&(package_name.to_string(), version.to_string()))
            .map(|(path, _)| path.clone())
    }
}

#[async_trait]
impl WorkflowRegistry for FilesystemWorkflowRegistry {
    async fn register_workflow(
        &mut self,
        package_data: Vec<u8>,
    ) -> Result<WorkflowPackageId, RegistryError> {
        // Unpack to temp dir and read package.toml to get package name/version
        let tmp = tempfile::TempDir::new()
            .map_err(|e| RegistryError::Internal(format!("Failed to create temp dir: {}", e)))?;
        let archive_path = tmp.path().join("pkg.cloacina");
        std::fs::write(&archive_path, &package_data)
            .map_err(|e| RegistryError::Internal(format!("Failed to write archive: {}", e)))?;
        let extract_dir = tmp.path().join("source");
        std::fs::create_dir_all(&extract_dir)
            .map_err(|e| RegistryError::Internal(format!("Failed to create extract dir: {}", e)))?;
        let source_dir = fidius_core::package::unpack_package(&archive_path, &extract_dir)
            .map_err(|e| {
                RegistryError::Internal(format!("Failed to read manifest from package data: {}", e))
            })?;
        let manifest = fidius_core::package::load_manifest::<
            cloacina_workflow_plugin::CloacinaMetadata,
        >(&source_dir)
        .map_err(|e| {
            RegistryError::Internal(format!("Failed to read manifest from package data: {}", e))
        })?;

        let filename = format!(
            "{}-{}.cloacina",
            manifest.package.name, manifest.package.version
        );

        // Copy to the first watch directory
        let target_dir = self.watch_dirs.first().ok_or_else(|| {
            RegistryError::Internal("No watch directories configured".to_string())
        })?;

        // Create directory if needed
        if !target_dir.exists() {
            std::fs::create_dir_all(target_dir).map_err(|e| {
                RegistryError::Internal(format!(
                    "Failed to create watch directory {}: {}",
                    target_dir.display(),
                    e
                ))
            })?;
        }

        let target_path = target_dir.join(&filename);

        // Check for existing package
        if target_path.exists() {
            return Err(RegistryError::PackageExists {
                package_name: manifest.package.name,
                version: manifest.package.version,
            });
        }

        std::fs::write(&target_path, &package_data).map_err(|e| {
            RegistryError::Internal(format!(
                "Failed to write package to {}: {}",
                target_path.display(),
                e
            ))
        })?;

        let fingerprint = format!("{}:{}", manifest.package.name, manifest.package.version);
        let id = uuid_from_fingerprint(&fingerprint);

        info!(
            "Registered package {} v{} at {}",
            manifest.package.name,
            manifest.package.version,
            target_path.display()
        );

        Ok(id)
    }

    async fn get_workflow(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<LoadedWorkflow>, RegistryError> {
        let packages = self.scan_packages();

        match packages.get(&(package_name.to_string(), version.to_string())) {
            Some((path, metadata)) => {
                let package_data = std::fs::read(path).map_err(|e| {
                    RegistryError::Internal(format!(
                        "Failed to read package file {}: {}",
                        path.display(),
                        e
                    ))
                })?;

                Ok(Some(LoadedWorkflow {
                    metadata: metadata.clone(),
                    package_data,
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_workflows(&self) -> Result<Vec<WorkflowMetadata>, RegistryError> {
        let packages = self.scan_packages();
        Ok(packages
            .into_values()
            .map(|(_, metadata)| metadata)
            .collect())
    }

    async fn unregister_workflow(
        &mut self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        match self.find_package_path(package_name, version) {
            Some(path) => {
                std::fs::remove_file(&path).map_err(|e| {
                    RegistryError::Internal(format!(
                        "Failed to remove package file {}: {}",
                        path.display(),
                        e
                    ))
                })?;

                info!(
                    "Unregistered package {} v{} (removed {})",
                    package_name,
                    version,
                    path.display()
                );

                Ok(())
            }
            None => Err(RegistryError::PackageNotFound {
                package_name: package_name.to_string(),
                version: version.to_string(),
            }),
        }
    }
}

/// Derive a deterministic UUID from a string fingerprint.
///
/// Uses UUID v5 (SHA-1 based) with a fixed namespace so the same
/// fingerprint always produces the same UUID.
fn uuid_from_fingerprint(fingerprint: &str) -> Uuid {
    // Use the URL namespace as a base — the fingerprint is our "name"
    Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Build a minimal bzip2-tar `.cloacina` source archive in memory.
    ///
    /// The archive contains a top-level `{name}-{version}/` directory with a
    /// `package.toml` and a stub `src/lib.rs`, matching what `pack_package`
    /// produces.
    fn build_test_archive(name: &str, version: &str) -> Vec<u8> {
        use bzip2::write::BzEncoder;
        use bzip2::Compression;

        let prefix = format!("{}-{}", name, version);

        let toml_content = format!(
            r#"[package]
name = "{name}"
version = "{version}"
interface = "cloacina-workflow"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "{name}"
language = "rust"
description = "Test package"
"#
        );

        // Pack into bzip2 tar in memory
        let buf = Vec::new();
        let enc = BzEncoder::new(buf, Compression::fast());
        let mut builder = tar::Builder::new(enc);

        for (rel, content) in &[
            ("package.toml", toml_content.as_bytes()),
            ("src/lib.rs", b"// stub" as &[u8]),
        ] {
            let archive_path = format!("{}/{}", prefix, rel);
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, &archive_path, *content)
                .unwrap();
        }

        let enc = builder.into_inner().unwrap();
        enc.finish().unwrap()
    }

    #[tokio::test]
    async fn test_list_empty_directory() {
        let dir = TempDir::new().unwrap();
        let registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);
        let workflows = registry.list_workflows().await.unwrap();
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_list_discovers_packages() {
        let dir = TempDir::new().unwrap();

        let archive1 = build_test_archive("pkg-a", "1.0.0");
        let archive2 = build_test_archive("pkg-b", "2.0.0");
        std::fs::write(dir.path().join("pkg-a-1.0.0.cloacina"), &archive1).unwrap();
        std::fs::write(dir.path().join("pkg-b-2.0.0.cloacina"), &archive2).unwrap();

        let registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);
        let workflows = registry.list_workflows().await.unwrap();
        assert_eq!(workflows.len(), 2);

        let names: Vec<_> = workflows.iter().map(|w| w.package_name.as_str()).collect();
        assert!(names.contains(&"pkg-a"));
        assert!(names.contains(&"pkg-b"));
    }

    #[tokio::test]
    async fn test_list_multiple_directories() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();

        std::fs::write(
            dir1.path().join("pkg-a.cloacina"),
            build_test_archive("pkg-a", "1.0.0"),
        )
        .unwrap();
        std::fs::write(
            dir2.path().join("pkg-b.cloacina"),
            build_test_archive("pkg-b", "1.0.0"),
        )
        .unwrap();

        let registry = FilesystemWorkflowRegistry::new(vec![
            dir1.path().to_path_buf(),
            dir2.path().to_path_buf(),
        ]);
        let workflows = registry.list_workflows().await.unwrap();
        assert_eq!(workflows.len(), 2);
    }

    #[tokio::test]
    async fn test_get_workflow_returns_archive_bytes() {
        let dir = TempDir::new().unwrap();
        let archive = build_test_archive("my-pkg", "1.0.0");
        std::fs::write(dir.path().join("my-pkg.cloacina"), &archive).unwrap();

        let registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);
        let loaded = registry.get_workflow("my-pkg", "1.0.0").await.unwrap();

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.metadata.package_name, "my-pkg");
        assert_eq!(loaded.metadata.version, "1.0.0");
        assert_eq!(loaded.package_data, archive);
    }

    #[tokio::test]
    async fn test_get_workflow_not_found() {
        let dir = TempDir::new().unwrap();
        let registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);
        let result = registry.get_workflow("nonexistent", "1.0.0").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_register_writes_file() {
        let dir = TempDir::new().unwrap();
        let mut registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);

        let archive = build_test_archive("new-pkg", "0.1.0");
        let id = registry.register_workflow(archive.clone()).await.unwrap();

        // File should exist
        let expected_path = dir.path().join("new-pkg-0.1.0.cloacina");
        assert!(expected_path.exists());
        assert_eq!(std::fs::read(&expected_path).unwrap(), archive);

        // Should be discoverable
        let workflows = registry.list_workflows().await.unwrap();
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].package_name, "new-pkg");

        // ID should be deterministic from name:version fingerprint
        let id2 = uuid_from_fingerprint("new-pkg:0.1.0");
        assert_eq!(id, id2);
    }

    #[tokio::test]
    async fn test_register_duplicate_rejected() {
        let dir = TempDir::new().unwrap();
        let mut registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);

        let archive = build_test_archive("dup-pkg", "1.0.0");
        registry.register_workflow(archive.clone()).await.unwrap();

        let result = registry.register_workflow(archive).await;
        assert!(matches!(result, Err(RegistryError::PackageExists { .. })));
    }

    #[tokio::test]
    async fn test_unregister_removes_file() {
        let dir = TempDir::new().unwrap();
        let archive = build_test_archive("rm-pkg", "1.0.0");
        std::fs::write(dir.path().join("rm-pkg-1.0.0.cloacina"), &archive).unwrap();

        let mut registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);

        assert!(registry
            .get_workflow("rm-pkg", "1.0.0")
            .await
            .unwrap()
            .is_some());

        registry
            .unregister_workflow("rm-pkg", "1.0.0")
            .await
            .unwrap();

        assert!(registry
            .get_workflow("rm-pkg", "1.0.0")
            .await
            .unwrap()
            .is_none());
        assert!(!dir.path().join("rm-pkg-1.0.0.cloacina").exists());
    }

    #[tokio::test]
    async fn test_unregister_not_found() {
        let dir = TempDir::new().unwrap();
        let mut registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);

        let result = registry.unregister_workflow("nonexistent", "1.0.0").await;
        assert!(matches!(result, Err(RegistryError::PackageNotFound { .. })));
    }

    #[tokio::test]
    async fn test_corrupt_file_skipped() {
        let dir = TempDir::new().unwrap();

        // Write a valid package
        std::fs::write(
            dir.path().join("good.cloacina"),
            build_test_archive("good", "1.0.0"),
        )
        .unwrap();

        // Write corrupt data
        std::fs::write(dir.path().join("bad.cloacina"), b"not a valid archive").unwrap();

        // Write a non-.cloacina file (should be ignored)
        std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();

        let registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);
        let workflows = registry.list_workflows().await.unwrap();

        // Only the good package should be listed
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].package_name, "good");
    }

    #[tokio::test]
    async fn test_nonexistent_directory_handled() {
        let registry = FilesystemWorkflowRegistry::new(vec![PathBuf::from(
            "/tmp/definitely-does-not-exist-cloacina-test",
        )]);
        let workflows = registry.list_workflows().await.unwrap();
        assert!(workflows.is_empty());
    }

    #[tokio::test]
    async fn test_register_creates_directory() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("packages");

        let mut registry = FilesystemWorkflowRegistry::new(vec![subdir.clone()]);
        let archive = build_test_archive("auto-dir", "1.0.0");
        registry.register_workflow(archive).await.unwrap();

        assert!(subdir.exists());
        assert!(subdir.join("auto-dir-1.0.0.cloacina").exists());
    }

    #[tokio::test]
    async fn test_deterministic_package_id() {
        let id1 = uuid_from_fingerprint("abc:1.0.0");
        let id2 = uuid_from_fingerprint("abc:1.0.0");
        let id3 = uuid_from_fingerprint("different:1.0.0");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[tokio::test]
    async fn test_package_with_triggers_in_manifest() {
        use bzip2::write::BzEncoder;
        use bzip2::Compression;

        let dir = TempDir::new().unwrap();

        let toml_content = r#"[package]
name = "trigger-pkg"
version = "1.0.0"
interface = "cloacina-workflow"
interface_version = 1
extension = "cloacina"

[metadata]
workflow_name = "trigger-pkg"
language = "rust"

[[metadata.triggers]]
name = "my_trigger"
workflow = "trigger-pkg"
poll_interval = "5s"
allow_concurrent = false
"#;

        let prefix = "trigger-pkg-1.0.0";
        let buf = Vec::new();
        let enc = BzEncoder::new(buf, Compression::fast());
        let mut builder = tar::Builder::new(enc);
        let mut header = tar::Header::new_gnu();
        header.set_size(toml_content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(
                &mut header,
                &format!("{}/package.toml", prefix),
                toml_content.as_bytes(),
            )
            .unwrap();
        let enc = builder.into_inner().unwrap();
        let archive = enc.finish().unwrap();

        std::fs::write(dir.path().join("trigger-pkg.cloacina"), &archive).unwrap();

        let registry = FilesystemWorkflowRegistry::new(vec![dir.path().to_path_buf()]);
        let workflows = registry.list_workflows().await.unwrap();

        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].package_name, "trigger-pkg");
    }
}
