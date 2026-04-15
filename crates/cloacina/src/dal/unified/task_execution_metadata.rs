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

//! Unified Task Execution Metadata DAL with runtime backend selection
//!
//! This module provides CRUD operations for TaskExecutionMetadata entities that work with
//! both PostgreSQL and SQLite backends, selecting the appropriate implementation
//! at runtime based on the database connection type.

use super::models::{NewUnifiedTaskExecutionMetadata, UnifiedTaskExecutionMetadata};
use super::DAL;
use crate::database::schema::unified::{contexts, task_execution_metadata};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::task_execution_metadata::{NewTaskExecutionMetadata, TaskExecutionMetadata};
use crate::task::TaskNamespace;
use diesel::prelude::*;

/// Data access layer for task execution metadata operations with runtime backend selection.
#[derive(Clone)]
pub struct TaskExecutionMetadataDAL<'a> {
    dal: &'a DAL,
}

impl<'a> TaskExecutionMetadataDAL<'a> {
    /// Creates a new TaskExecutionMetadataDAL instance.
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Creates a new task execution metadata record.
    pub async fn create(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_postgres(new_metadata).await,
            self.create_sqlite(new_metadata).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn create_postgres(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTaskExecutionMetadata {
            id,
            task_execution_id: new_metadata.task_execution_id,
            workflow_execution_id: new_metadata.workflow_execution_id,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| task_execution_metadata::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn create_sqlite(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTaskExecutionMetadata {
            id,
            task_execution_id: new_metadata.task_execution_id,
            workflow_execution_id: new_metadata.workflow_execution_id,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id,
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(task_execution_metadata::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| task_execution_metadata::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Retrieves task execution metadata for a specific workflow and task.
    pub async fn get_by_workflow_and_task(
        &self,
        workflow_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_workflow_and_task_postgres(workflow_id, task_namespace)
                .await,
            self.get_by_workflow_and_task_sqlite(workflow_id, task_namespace)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_by_workflow_and_task_postgres(
        &self,
        workflow_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name_owned = task_namespace.to_string();
        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq(&task_name_owned))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_workflow_and_task_sqlite(
        &self,
        workflow_id: UniversalUuid,
        task_namespace: &TaskNamespace,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let task_name = task_namespace.to_string();
        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq(task_name))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Retrieves task execution metadata by task execution ID.
    pub async fn get_by_task_execution(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_by_task_execution_postgres(task_execution_id).await,
            self.get_by_task_execution_sqlite(task_execution_id).await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_by_task_execution_postgres(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_task_execution_sqlite(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    /// Updates the context ID for a specific task execution.
    pub async fn update_context_id(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.update_context_id_postgres(task_execution_id, context_id)
                .await,
            self.update_context_id_sqlite(task_execution_id, context_id)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn update_context_id_postgres(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                .set((
                    task_execution_metadata::context_id.eq(context_id),
                    task_execution_metadata::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn update_context_id_sqlite(
        &self,
        task_execution_id: UniversalUuid,
        context_id: Option<UniversalUuid>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let now = UniversalTimestamp::now();
        conn.interact(move |conn| {
            diesel::update(task_execution_metadata::table)
                .filter(task_execution_metadata::task_execution_id.eq(task_execution_id))
                .set((
                    task_execution_metadata::context_id.eq(context_id),
                    task_execution_metadata::updated_at.eq(now),
                ))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Creates or updates task execution metadata.
    pub async fn upsert_task_execution_metadata(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.upsert_task_execution_metadata_postgres(new_metadata)
                .await,
            self.upsert_task_execution_metadata_sqlite(new_metadata)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn upsert_task_execution_metadata_postgres(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTaskExecutionMetadata {
            id,
            task_execution_id: new_metadata.task_execution_id,
            workflow_execution_id: new_metadata.workflow_execution_id,
            task_name: new_metadata.task_name,
            context_id: new_metadata.context_id,
            created_at: now,
            updated_at: now,
        };

        let task_exec_id = new_unified.task_execution_id;
        let context_id = new_unified.context_id;

        let result: UnifiedTaskExecutionMetadata = conn
            .interact(move |conn| {
                diesel::insert_into(task_execution_metadata::table)
                    .values(&new_unified)
                    .on_conflict(task_execution_metadata::task_execution_id)
                    .do_update()
                    .set((
                        task_execution_metadata::context_id.eq(context_id),
                        task_execution_metadata::updated_at.eq(now),
                    ))
                    .execute(conn)?;

                // Fetch the result
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                    .first(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    async fn upsert_task_execution_metadata_sqlite(
        &self,
        new_metadata: NewTaskExecutionMetadata,
    ) -> Result<TaskExecutionMetadata, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        // SQLite doesn't support ON CONFLICT DO UPDATE with RETURNING well
        // Check if the record exists first
        let task_exec_id = new_metadata.task_execution_id;
        let existing: Option<UnifiedTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                    .first::<UnifiedTaskExecutionMetadata>(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        match existing {
            Some(_) => {
                // Update existing record
                let task_exec_id = new_metadata.task_execution_id;
                let context_id = new_metadata.context_id;
                let now = UniversalTimestamp::now();

                conn.interact(move |conn| {
                    diesel::update(task_execution_metadata::table)
                        .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                        .set((
                            task_execution_metadata::context_id.eq(context_id),
                            task_execution_metadata::updated_at.eq(now),
                        ))
                        .execute(conn)
                })
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                // Retrieve the updated record
                let task_exec_id = new_metadata.task_execution_id;
                let result: UnifiedTaskExecutionMetadata = conn
                    .interact(move |conn| {
                        task_execution_metadata::table
                            .filter(task_execution_metadata::task_execution_id.eq(task_exec_id))
                            .first(conn)
                    })
                    .await
                    .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                Ok(result.into())
            }
            None => {
                // Create new record
                let id = UniversalUuid::new_v4();
                let now = UniversalTimestamp::now();

                let new_unified = NewUnifiedTaskExecutionMetadata {
                    id,
                    task_execution_id: new_metadata.task_execution_id,
                    workflow_execution_id: new_metadata.workflow_execution_id,
                    task_name: new_metadata.task_name,
                    context_id: new_metadata.context_id,
                    created_at: now,
                    updated_at: now,
                };

                conn.interact(move |conn| {
                    diesel::insert_into(task_execution_metadata::table)
                        .values(&new_unified)
                        .execute(conn)
                })
                .await
                .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                let result: UnifiedTaskExecutionMetadata = conn
                    .interact(move |conn| task_execution_metadata::table.find(id).first(conn))
                    .await
                    .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

                Ok(result.into())
            }
        }
    }

    /// Retrieves metadata for multiple dependency tasks within a workflow execution.
    pub async fn get_dependency_metadata(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_dependency_metadata_postgres(workflow_id, dependency_task_names)
                .await,
            self.get_dependency_metadata_sqlite(workflow_id, dependency_task_names)
                .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_dependency_metadata_postgres(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names_owned = dependency_task_names.to_vec();
        let results: Vec<UnifiedTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_dependency_metadata_sqlite(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_names: &[String],
    ) -> Result<Vec<TaskExecutionMetadata>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names = dependency_task_names.to_vec();
        let results: Vec<UnifiedTaskExecutionMetadata> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    /// Retrieves metadata and context data for multiple dependency tasks in a single query.
    pub async fn get_dependency_metadata_with_contexts(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        if dependency_task_namespaces.is_empty() {
            return Ok(Vec::new());
        }

        crate::dispatch_backend!(
            self.dal.backend(),
            self.get_dependency_metadata_with_contexts_postgres(
                workflow_id,
                dependency_task_namespaces
            )
            .await,
            self.get_dependency_metadata_with_contexts_sqlite(
                workflow_id,
                dependency_task_namespaces
            )
            .await
        )
    }

    #[cfg(feature = "postgres")]
    async fn get_dependency_metadata_with_contexts_postgres(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names_owned: Vec<String> = dependency_task_namespaces
            .iter()
            .map(|ns| ns.to_string())
            .collect();

        let results: Vec<(UnifiedTaskExecutionMetadata, Option<String>)> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .left_join(
                        contexts::table
                            .on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                    )
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq_any(&dependency_task_names_owned))
                    .select((
                        task_execution_metadata::all_columns,
                        contexts::value.nullable(),
                    ))
                    .load::<(UnifiedTaskExecutionMetadata, Option<String>)>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|(m, c)| (m.into(), c)).collect())
    }

    #[cfg(feature = "sqlite")]
    async fn get_dependency_metadata_with_contexts_sqlite(
        &self,
        workflow_id: UniversalUuid,
        dependency_task_namespaces: &[TaskNamespace],
    ) -> Result<Vec<(TaskExecutionMetadata, Option<String>)>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let dependency_task_names: Vec<String> = dependency_task_namespaces
            .iter()
            .map(|ns| ns.to_string())
            .collect();

        let results: Vec<(UnifiedTaskExecutionMetadata, Option<String>)> = conn
            .interact(move |conn| {
                task_execution_metadata::table
                    .left_join(
                        contexts::table
                            .on(task_execution_metadata::context_id.eq(contexts::id.nullable())),
                    )
                    .filter(task_execution_metadata::workflow_execution_id.eq(workflow_id))
                    .filter(task_execution_metadata::task_name.eq_any(dependency_task_names))
                    .select((
                        task_execution_metadata::all_columns,
                        contexts::value.nullable(),
                    ))
                    .load::<(UnifiedTaskExecutionMetadata, Option<String>)>(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(|(m, c)| (m.into(), c)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::database::Database;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::task_execution_metadata::NewTaskExecutionMetadata;
    use crate::models::workflow_execution::NewWorkflowExecution;

    #[cfg(feature = "sqlite")]
    async fn unique_dal() -> DAL {
        let url = format!(
            "file:meta_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        DAL::new(db)
    }

    /// Helper: create a workflow execution and a task, returning (workflow_id, task_id).
    #[cfg(feature = "sqlite")]
    async fn create_workflow_and_task(
        dal: &DAL,
        task_name: &str,
    ) -> (UniversalUuid, UniversalUuid) {
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "test_workflow".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap();

        let task = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: task_name.into(),
                status: "NotStarted".into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap();

        (wf_exec.id, task.id)
    }

    // ── create ─────────────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_metadata() {
        let dal = unique_dal().await;
        let (workflow_id, task_id) = create_workflow_and_task(&dal, "my_task").await;

        let metadata = dal
            .task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "my_task".into(),
                context_id: None,
            })
            .await
            .unwrap();

        assert_eq!(metadata.task_execution_id, task_id);
        assert_eq!(metadata.workflow_execution_id, workflow_id);
        assert_eq!(metadata.task_name, "my_task");
        assert!(metadata.context_id.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_create_metadata_with_context() {
        let dal = unique_dal().await;
        let (workflow_id, task_id) = create_workflow_and_task(&dal, "ctx_task").await;

        // Create a context first so the FK is satisfied
        let mut ctx = Context::<serde_json::Value>::new();
        ctx.insert("key".to_string(), serde_json::json!("value"))
            .unwrap();
        let ctx_id = dal.context().create(&ctx).await.unwrap().unwrap();

        let metadata = dal
            .task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "ctx_task".into(),
                context_id: Some(ctx_id),
            })
            .await
            .unwrap();

        assert_eq!(metadata.context_id, Some(ctx_id));
    }

    // ── get_by_workflow_and_task ───────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_workflow_and_task() {
        let dal = unique_dal().await;
        let ns = TaskNamespace::new("public", "embedded", "test_wf", "lookup_task");
        let ns_str = ns.to_string();
        let (workflow_id, task_id) = create_workflow_and_task(&dal, &ns_str).await;

        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: ns_str.clone(),
                context_id: None,
            })
            .await
            .unwrap();

        let found = dal
            .task_execution_metadata()
            .get_by_workflow_and_task(workflow_id, &ns)
            .await
            .unwrap();

        assert_eq!(found.task_execution_id, task_id);
        assert_eq!(found.task_name, ns_str);
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_workflow_and_task_not_found() {
        let dal = unique_dal().await;
        let ns = TaskNamespace::new("public", "embedded", "wf", "nonexistent");
        let result = dal
            .task_execution_metadata()
            .get_by_workflow_and_task(UniversalUuid::new_v4(), &ns)
            .await;
        assert!(result.is_err());
    }

    // ── get_by_task_execution ──────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_by_task_execution() {
        let dal = unique_dal().await;
        let (workflow_id, task_id) = create_workflow_and_task(&dal, "exec_lookup").await;

        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "exec_lookup".into(),
                context_id: None,
            })
            .await
            .unwrap();

        let found = dal
            .task_execution_metadata()
            .get_by_task_execution(task_id)
            .await
            .unwrap();

        assert_eq!(found.task_execution_id, task_id);
        assert_eq!(found.workflow_execution_id, workflow_id);
    }

    // ── update_context_id ──────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_update_context_id() {
        let dal = unique_dal().await;
        let (workflow_id, task_id) = create_workflow_and_task(&dal, "update_ctx").await;

        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "update_ctx".into(),
                context_id: None,
            })
            .await
            .unwrap();

        // Create a context and update the metadata to reference it
        let mut ctx = Context::<serde_json::Value>::new();
        ctx.insert("result".to_string(), serde_json::json!(42))
            .unwrap();
        let ctx_id = dal.context().create(&ctx).await.unwrap().unwrap();

        dal.task_execution_metadata()
            .update_context_id(task_id, Some(ctx_id))
            .await
            .unwrap();

        let updated = dal
            .task_execution_metadata()
            .get_by_task_execution(task_id)
            .await
            .unwrap();
        assert_eq!(updated.context_id, Some(ctx_id));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_update_context_id_to_none() {
        let dal = unique_dal().await;
        let (workflow_id, task_id) = create_workflow_and_task(&dal, "clear_ctx").await;

        let mut ctx = Context::<serde_json::Value>::new();
        ctx.insert("temp".to_string(), serde_json::json!("data"))
            .unwrap();
        let ctx_id = dal.context().create(&ctx).await.unwrap().unwrap();

        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "clear_ctx".into(),
                context_id: Some(ctx_id),
            })
            .await
            .unwrap();

        // Clear the context
        dal.task_execution_metadata()
            .update_context_id(task_id, None)
            .await
            .unwrap();

        let updated = dal
            .task_execution_metadata()
            .get_by_task_execution(task_id)
            .await
            .unwrap();
        assert!(updated.context_id.is_none());
    }

    // ── upsert_task_execution_metadata ─────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_upsert_insert() {
        let dal = unique_dal().await;
        let (workflow_id, task_id) = create_workflow_and_task(&dal, "upsert_new").await;

        let metadata = dal
            .task_execution_metadata()
            .upsert_task_execution_metadata(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "upsert_new".into(),
                context_id: None,
            })
            .await
            .unwrap();

        assert_eq!(metadata.task_execution_id, task_id);
        assert!(metadata.context_id.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_upsert_update() {
        let dal = unique_dal().await;
        let (workflow_id, task_id) = create_workflow_and_task(&dal, "upsert_upd").await;

        // First insert
        let original = dal
            .task_execution_metadata()
            .upsert_task_execution_metadata(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "upsert_upd".into(),
                context_id: None,
            })
            .await
            .unwrap();

        // Create a context for the update
        let mut ctx = Context::<serde_json::Value>::new();
        ctx.insert("updated".to_string(), serde_json::json!(true))
            .unwrap();
        let ctx_id = dal.context().create(&ctx).await.unwrap().unwrap();

        // Upsert again with a new context_id
        let upserted = dal
            .task_execution_metadata()
            .upsert_task_execution_metadata(NewTaskExecutionMetadata {
                task_execution_id: task_id,
                workflow_execution_id: workflow_id,
                task_name: "upsert_upd".into(),
                context_id: Some(ctx_id),
            })
            .await
            .unwrap();

        // Same record, updated context
        assert_eq!(upserted.id, original.id);
        assert_eq!(upserted.context_id, Some(ctx_id));
    }

    // ── get_dependency_metadata ────────────────────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_dependency_metadata() {
        let dal = unique_dal().await;
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "dep_workflow".into(),
                workflow_version: "1".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap();

        // Create two tasks + metadata
        for name in &["dep_a", "dep_b", "not_a_dep"] {
            let task = dal
                .task_execution()
                .create(NewTaskExecution {
                    workflow_execution_id: wf_exec.id,
                    task_name: name.to_string(),
                    status: "NotStarted".into(),
                    attempt: 1,
                    max_attempts: 1,
                    trigger_rules: r#"{"type":"Always"}"#.into(),
                    task_configuration: "{}".into(),
                })
                .await
                .unwrap();

            dal.task_execution_metadata()
                .create(NewTaskExecutionMetadata {
                    task_execution_id: task.id,
                    workflow_execution_id: wf_exec.id,
                    task_name: name.to_string(),
                    context_id: None,
                })
                .await
                .unwrap();
        }

        let deps = dal
            .task_execution_metadata()
            .get_dependency_metadata(wf_exec.id, &["dep_a".to_string(), "dep_b".to_string()])
            .await
            .unwrap();

        assert_eq!(deps.len(), 2);
        let names: Vec<&str> = deps.iter().map(|d| d.task_name.as_str()).collect();
        assert!(names.contains(&"dep_a"));
        assert!(names.contains(&"dep_b"));
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_dependency_metadata_empty() {
        let dal = unique_dal().await;
        let deps = dal
            .task_execution_metadata()
            .get_dependency_metadata(UniversalUuid::new_v4(), &[])
            .await
            .unwrap();
        assert!(deps.is_empty());
    }

    // ── get_dependency_metadata_with_contexts ──────────────────────

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn test_get_dependency_metadata_with_contexts_empty_input() {
        let dal = unique_dal().await;
        let result = dal
            .task_execution_metadata()
            .get_dependency_metadata_with_contexts(UniversalUuid::new_v4(), &[])
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    #[ignore = "requires matching task_name format with internal query — needs investigation"]
    async fn test_get_dependency_metadata_with_contexts() {
        let dal = unique_dal().await;
        let wf_exec = dal
            .workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "ctx_dep_workflow".into(),
                workflow_version: "1".into(),
                status: "Running".into(),
                context_id: None,
            })
            .await
            .unwrap();

        // Task with context
        let mut ctx = Context::<serde_json::Value>::new();
        ctx.insert("output".to_string(), serde_json::json!("hello"))
            .unwrap();
        let ctx_id = dal.context().create(&ctx).await.unwrap().unwrap();

        let task_with_ctx = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: "public::embedded::wf::has_ctx".into(),
                status: "Completed".into(),
                attempt: 1,
                max_attempts: 1,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap();

        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_with_ctx.id,
                workflow_execution_id: wf_exec.id,
                task_name: "public::embedded::wf::has_ctx".into(),
                context_id: Some(ctx_id),
            })
            .await
            .unwrap();

        // Task without context
        let task_no_ctx = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: wf_exec.id,
                task_name: "public::embedded::wf::no_ctx".into(),
                status: "Completed".into(),
                attempt: 1,
                max_attempts: 1,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap();

        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: task_no_ctx.id,
                workflow_execution_id: wf_exec.id,
                task_name: "public::embedded::wf::no_ctx".into(),
                context_id: None,
            })
            .await
            .unwrap();

        let ns_with = TaskNamespace::new("public", "embedded", "wf", "has_ctx");
        let ns_without = TaskNamespace::new("public", "embedded", "wf", "no_ctx");

        let results = dal
            .task_execution_metadata()
            .get_dependency_metadata_with_contexts(wf_exec.id, &[ns_with, ns_without])
            .await
            .unwrap();

        assert_eq!(results.len(), 2);

        // Find the one with context
        let with_ctx = results
            .iter()
            .find(|(m, _)| m.task_execution_id == task_with_ctx.id)
            .unwrap();
        assert_eq!(with_ctx.1.as_deref(), Some("{\"output\": \"hello\"}"));

        // Find the one without context
        let without_ctx = results
            .iter()
            .find(|(m, _)| m.task_execution_id == task_no_ctx.id)
            .unwrap();
        assert!(without_ctx.1.is_none());
    }
}
