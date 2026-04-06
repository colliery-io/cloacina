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

//! Unified Workflow Packages DAL with runtime backend selection
//!
//! This module provides CRUD operations for WorkflowPackage entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::models::{NewUnifiedWorkflowPackage, UnifiedWorkflowPackage};
use super::DAL;
use crate::database::schema::unified::workflow_packages;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::models::workflow_packages::WorkflowPackage;
use crate::registry::error::RegistryError;
use crate::registry::loader::package_loader::PackageMetadata;
use diesel::prelude::*;
use uuid::Uuid;

/// Data access layer for workflow package operations with runtime backend selection.
#[derive(Clone)]
pub struct WorkflowPackagesDAL<'a> {
    dal: &'a DAL,
}

impl<'a> WorkflowPackagesDAL<'a> {
    /// Creates a new WorkflowPackagesDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Store package metadata in the database.
    pub async fn store_package_metadata(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
        storage_type: crate::models::workflow_packages::StorageType,
        tenant_id: Option<&str>,
    ) -> Result<Uuid, RegistryError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.store_package_metadata_postgres(
                registry_id,
                package_metadata,
                storage_type,
                tenant_id
            )
            .await,
            self.store_package_metadata_sqlite(
                registry_id,
                package_metadata,
                storage_type,
                tenant_id
            )
            .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn store_package_metadata_postgres(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
        storage_type: crate::models::workflow_packages::StorageType,
        tenant_id: Option<&str>,
    ) -> Result<Uuid, RegistryError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let tenant_id_owned = tenant_id.map(|s| s.to_string());
        let new_unified = NewUnifiedWorkflowPackage {
            id,
            registry_id: UniversalUuid(registry_uuid),
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            storage_type: storage_type.as_str().to_string(),
            created_at: now,
            updated_at: now,
            tenant_id: tenant_id_owned,
        };

        let package_name_clone = package_metadata.package_name.clone();
        let version_clone = package_metadata.version.clone();

        conn.interact(move |conn| {
            diesel::insert_into(workflow_packages::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _info,
            ) => RegistryError::PackageExists {
                package_name: package_name_clone,
                version: version_clone,
            },
            _ => RegistryError::Database(format!("Database error: {}", e)),
        })?;

        Ok(id.0)
    }

    #[cfg(feature = "sqlite")]
    async fn store_package_metadata_sqlite(
        &self,
        registry_id: &str,
        package_metadata: &PackageMetadata,
        storage_type: crate::models::workflow_packages::StorageType,
        tenant_id: Option<&str>,
    ) -> Result<Uuid, RegistryError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let registry_uuid = Uuid::parse_str(registry_id).map_err(RegistryError::InvalidUuid)?;
        let metadata =
            serde_json::to_string(package_metadata).map_err(RegistryError::Serialization)?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let tenant_id_owned = tenant_id.map(|s| s.to_string());
        let new_unified = NewUnifiedWorkflowPackage {
            id,
            registry_id: UniversalUuid(registry_uuid),
            package_name: package_metadata.package_name.clone(),
            version: package_metadata.version.clone(),
            description: package_metadata.description.clone(),
            author: package_metadata.author.clone(),
            metadata,
            storage_type: storage_type.as_str().to_string(),
            created_at: now,
            updated_at: now,
            tenant_id: tenant_id_owned,
        };

        let package_name_clone = package_metadata.package_name.clone();
        let version_clone = package_metadata.version.clone();

        conn.interact(move |conn| {
            diesel::insert_into(workflow_packages::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _info,
            ) => RegistryError::PackageExists {
                package_name: package_name_clone,
                version: version_clone,
            },
            _ => RegistryError::Database(format!("Database error: {}", e)),
        })?;

        Ok(id.0)
    }

    /// Retrieve package metadata from the database.
    pub async fn get_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_package_metadata_postgres(package_name, version)
                .await,
            self.get_package_metadata_sqlite(package_name, version)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_package_metadata_postgres(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let result: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = result {
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "sqlite")]
    async fn get_package_metadata_sqlite(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        let result: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = result {
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// Retrieve package metadata by UUID from the database.
    pub async fn get_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_package_metadata_by_id_postgres(package_id).await,
            self.get_package_metadata_by_id_sqlite(package_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_package_metadata_by_id_postgres(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let id = UniversalUuid(package_id);
        let result: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(id))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = result {
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "sqlite")]
    async fn get_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<Option<(String, PackageMetadata)>, RegistryError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let id = UniversalUuid(package_id);
        let result: Option<UnifiedWorkflowPackage> = conn
            .interact(move |conn| {
                workflow_packages::table
                    .filter(workflow_packages::id.eq(id))
                    .first::<UnifiedWorkflowPackage>(conn)
                    .optional()
            })
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        if let Some(record) = result {
            let metadata: PackageMetadata =
                serde_json::from_str(&record.metadata).map_err(RegistryError::Serialization)?;
            Ok(Some((record.registry_id.0.to_string(), metadata)))
        } else {
            Ok(None)
        }
    }

    /// List all packages in the registry.
    pub async fn list_all_packages(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.list_all_packages_postgres().await,
            self.list_all_packages_sqlite().await
        )
    }

    #[cfg(feature = "postgres")]
    async fn list_all_packages_postgres(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let results: Vec<UnifiedWorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<UnifiedWorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn list_all_packages_sqlite(&self) -> Result<Vec<WorkflowPackage>, RegistryError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let results: Vec<UnifiedWorkflowPackage> = conn
            .interact(move |conn| workflow_packages::table.load::<UnifiedWorkflowPackage>(conn))
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?
            .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Delete package metadata from the database.
    pub async fn delete_package_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_package_metadata_postgres(package_name, version)
                .await,
            self.delete_package_metadata_sqlite(package_name, version)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn delete_package_metadata_postgres(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        conn.interact(move |conn| {
            diesel::delete(
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version)),
            )
            .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn delete_package_metadata_sqlite(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let package_name = package_name.to_string();
        let version = version.to_string();

        conn.interact(move |conn| {
            diesel::delete(
                workflow_packages::table
                    .filter(workflow_packages::package_name.eq(&package_name))
                    .filter(workflow_packages::version.eq(&version)),
            )
            .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }

    /// Delete package metadata by UUID from the database.
    pub async fn delete_package_metadata_by_id(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.delete_package_metadata_by_id_postgres(package_id)
                .await,
            self.delete_package_metadata_by_id_sqlite(package_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn delete_package_metadata_by_id_postgres(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let id = UniversalUuid(package_id);

        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(id)))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn delete_package_metadata_by_id_sqlite(
        &self,
        package_id: Uuid,
    ) -> Result<(), RegistryError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| RegistryError::Database(e.to_string()))?;

        let id = UniversalUuid(package_id);

        conn.interact(move |conn| {
            diesel::delete(workflow_packages::table.filter(workflow_packages::id.eq(id)))
                .execute(conn)
        })
        .await
        .map_err(|e| RegistryError::Database(e.to_string()))?
        .map_err(|e| RegistryError::Database(format!("Database error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;
    use crate::registry::loader::package_loader::{PackageMetadata, TaskMetadata};

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:wfpkg_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    #[cfg(feature = "sqlite")]
    fn sample_metadata(name: &str, version: &str) -> PackageMetadata {
        PackageMetadata {
            package_name: name.to_string(),
            version: version.to_string(),
            description: Some("A test package".to_string()),
            author: Some("test-author".to_string()),
            tasks: vec![TaskMetadata {
                index: 0,
                local_id: "task1".to_string(),
                namespaced_id_template: "{tenant}.{package}.{workflow}.task1".to_string(),
                dependencies: vec![],
                description: "test task".to_string(),
                source_location: "test.rs:1".to_string(),
            }],
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![],
        }
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_store_and_get_package_metadata() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("my-package", "1.0.0");

        let pkg_id = dal
            .workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        assert_ne!(pkg_id, Uuid::nil());

        // Retrieve by name and version
        let result = dal
            .workflow_packages()
            .get_package_metadata("my-package", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_some());
        let (reg_id, retrieved_meta) = result.unwrap();
        assert_eq!(reg_id, registry_id);
        assert_eq!(retrieved_meta.package_name, "my-package");
        assert_eq!(retrieved_meta.version, "1.0.0");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_not_found() {
        let dal = unique_dal().await;

        let result = dal
            .workflow_packages()
            .get_package_metadata("nonexistent", "0.0.0")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_by_id() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("id-lookup", "2.0.0");

        let pkg_id = dal
            .workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        let result = dal
            .workflow_packages()
            .get_package_metadata_by_id(pkg_id)
            .await
            .unwrap();
        assert!(result.is_some());
        let (_, retrieved_meta) = result.unwrap();
        assert_eq!(retrieved_meta.package_name, "id-lookup");
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_package_metadata_by_id_not_found() {
        let dal = unique_dal().await;

        let result = dal
            .workflow_packages()
            .get_package_metadata_by_id(Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_list_all_packages() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();

        // Initially empty
        let list = dal.workflow_packages().list_all_packages().await.unwrap();
        assert!(list.is_empty());

        // Store two packages
        let meta1 = sample_metadata("pkg-a", "1.0.0");
        let meta2 = sample_metadata("pkg-b", "1.0.0");
        dal.workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta1,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();
        dal.workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta2,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        let list = dal.workflow_packages().list_all_packages().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_package_metadata() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("to-delete", "1.0.0");

        dal.workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        // Confirm it exists
        let result = dal
            .workflow_packages()
            .get_package_metadata("to-delete", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_some());

        // Delete it
        dal.workflow_packages()
            .delete_package_metadata("to-delete", "1.0.0")
            .await
            .unwrap();

        // Confirm it is gone
        let result = dal
            .workflow_packages()
            .get_package_metadata("to-delete", "1.0.0")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_package_metadata_by_id() {
        let dal = unique_dal().await;
        let registry_id = Uuid::new_v4().to_string();
        let meta = sample_metadata("delete-by-id", "1.0.0");

        let pkg_id = dal
            .workflow_packages()
            .store_package_metadata(
                &registry_id,
                &meta,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();

        // Delete by ID
        dal.workflow_packages()
            .delete_package_metadata_by_id(pkg_id)
            .await
            .unwrap();

        // Confirm it is gone
        let result = dal
            .workflow_packages()
            .get_package_metadata_by_id(pkg_id)
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_delete_nonexistent_does_not_error() {
        let dal = unique_dal().await;

        // Deleting something that does not exist should succeed
        dal.workflow_packages()
            .delete_package_metadata("nonexistent", "0.0.0")
            .await
            .unwrap();

        dal.workflow_packages()
            .delete_package_metadata_by_id(Uuid::new_v4())
            .await
            .unwrap();
    }
}
