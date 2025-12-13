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

//! CRUD operations for trigger executions.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use super::TriggerExecutionDAL;
use crate::dal::unified::models::{NewUnifiedTriggerExecution, UnifiedTriggerExecution};
use crate::database::schema::unified::trigger_executions;
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::trigger_execution::{NewTriggerExecution, TriggerExecution};

impl<'a> TriggerExecutionDAL<'a> {
    #[cfg(feature = "postgres")]
    pub(super) async fn create_postgres(
        &self,
        new_execution: NewTriggerExecution,
    ) -> Result<TriggerExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = new_execution.id.unwrap_or_else(UniversalUuid::new_v4);
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTriggerExecution {
            id,
            trigger_name: new_execution.trigger_name,
            context_hash: new_execution.context_hash,
            pipeline_execution_id: new_execution.pipeline_execution_id,
            started_at: new_execution.started_at.unwrap_or(now),
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(trigger_executions::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTriggerExecution = conn
            .interact(move |conn| trigger_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn create_sqlite(
        &self,
        new_execution: NewTriggerExecution,
    ) -> Result<TriggerExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let id = new_execution.id.unwrap_or_else(UniversalUuid::new_v4);
        let now = UniversalTimestamp::now();

        let new_unified = NewUnifiedTriggerExecution {
            id,
            trigger_name: new_execution.trigger_name,
            context_hash: new_execution.context_hash,
            pipeline_execution_id: new_execution.pipeline_execution_id,
            started_at: new_execution.started_at.unwrap_or(now),
            created_at: now,
            updated_at: now,
        };

        conn.interact(move |conn| {
            diesel::insert_into(trigger_executions::table)
                .values(&new_unified)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        let result: UnifiedTriggerExecution = conn
            .interact(move |conn| trigger_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_by_id_postgres(
        &self,
        id: UniversalUuid,
    ) -> Result<TriggerExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTriggerExecution = conn
            .interact(move |conn| trigger_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_by_id_sqlite(
        &self,
        id: UniversalUuid,
    ) -> Result<TriggerExecution, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let result: UnifiedTriggerExecution = conn
            .interact(move |conn| trigger_executions::table.find(id).first(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(result.into())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn has_active_execution_postgres(
        &self,
        trigger_name: &str,
        context_hash: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name = trigger_name.to_string();
        let context_hash = context_hash.to_string();
        let count: i64 = conn
            .interact(move |conn| {
                trigger_executions::table
                    .filter(trigger_executions::trigger_name.eq(&trigger_name))
                    .filter(trigger_executions::context_hash.eq(&context_hash))
                    .filter(trigger_executions::completed_at.is_null())
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count > 0)
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn has_active_execution_sqlite(
        &self,
        trigger_name: &str,
        context_hash: &str,
    ) -> Result<bool, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name = trigger_name.to_string();
        let context_hash = context_hash.to_string();
        let count: i64 = conn
            .interact(move |conn| {
                trigger_executions::table
                    .filter(trigger_executions::trigger_name.eq(&trigger_name))
                    .filter(trigger_executions::context_hash.eq(&context_hash))
                    .filter(trigger_executions::completed_at.is_null())
                    .count()
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(count > 0)
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn complete_postgres(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let timestamp = UniversalTimestamp(completed_at);
        conn.interact(move |conn| {
            diesel::update(trigger_executions::table.find(id))
                .set(trigger_executions::completed_at.eq(Some(timestamp)))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn complete_sqlite(
        &self,
        id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let timestamp = UniversalTimestamp(completed_at);
        conn.interact(move |conn| {
            diesel::update(trigger_executions::table.find(id))
                .set(trigger_executions::completed_at.eq(Some(timestamp)))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn link_pipeline_execution_postgres(
        &self,
        id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::update(trigger_executions::table.find(id))
                .set(trigger_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn link_pipeline_execution_sqlite(
        &self,
        id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            diesel::update(trigger_executions::table.find(id))
                .set(trigger_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)))
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn get_recent_postgres(
        &self,
        trigger_name: &str,
        limit: i64,
    ) -> Result<Vec<TriggerExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name = trigger_name.to_string();
        let results: Vec<UnifiedTriggerExecution> = conn
            .interact(move |conn| {
                trigger_executions::table
                    .filter(trigger_executions::trigger_name.eq(&trigger_name))
                    .order(trigger_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn get_recent_sqlite(
        &self,
        trigger_name: &str,
        limit: i64,
    ) -> Result<Vec<TriggerExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name = trigger_name.to_string();
        let results: Vec<UnifiedTriggerExecution> = conn
            .interact(move |conn| {
                trigger_executions::table
                    .filter(trigger_executions::trigger_name.eq(&trigger_name))
                    .order(trigger_executions::started_at.desc())
                    .limit(limit)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn list_by_trigger_postgres(
        &self,
        trigger_name: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TriggerExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name = trigger_name.to_string();
        let results: Vec<UnifiedTriggerExecution> = conn
            .interact(move |conn| {
                trigger_executions::table
                    .filter(trigger_executions::trigger_name.eq(&trigger_name))
                    .order(trigger_executions::started_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn list_by_trigger_sqlite(
        &self,
        trigger_name: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<TriggerExecution>, ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let trigger_name = trigger_name.to_string();
        let results: Vec<UnifiedTriggerExecution> = conn
            .interact(move |conn| {
                trigger_executions::table
                    .filter(trigger_executions::trigger_name.eq(&trigger_name))
                    .order(trigger_executions::started_at.desc())
                    .limit(limit)
                    .offset(offset)
                    .load(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(results.into_iter().map(Into::into).collect())
    }

    #[cfg(feature = "postgres")]
    pub(super) async fn complete_by_pipeline_postgres(
        &self,
        pipeline_execution_id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let timestamp = UniversalTimestamp(completed_at);
        conn.interact(move |conn| {
            diesel::update(
                trigger_executions::table
                    .filter(
                        trigger_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)),
                    )
                    .filter(trigger_executions::completed_at.is_null()),
            )
            .set(trigger_executions::completed_at.eq(Some(timestamp)))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(super) async fn complete_by_pipeline_sqlite(
        &self,
        pipeline_execution_id: UniversalUuid,
        completed_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        let timestamp = UniversalTimestamp(completed_at);
        conn.interact(move |conn| {
            diesel::update(
                trigger_executions::table
                    .filter(
                        trigger_executions::pipeline_execution_id.eq(Some(pipeline_execution_id)),
                    )
                    .filter(trigger_executions::completed_at.is_null()),
            )
            .set(trigger_executions::completed_at.eq(Some(timestamp)))
            .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
}
