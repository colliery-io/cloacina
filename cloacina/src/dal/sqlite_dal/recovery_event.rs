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

use super::DAL;
use crate::database::schema::recovery_events;
use crate::database::universal_types::{current_timestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::recovery_event::{NewRecoveryEvent, RecoveryEvent, RecoveryType};
use diesel::prelude::*;

/// Data access layer for recovery event operations.
///
/// This DAL provides methods for creating and querying recovery events,
/// which track recovery operations performed on tasks and pipelines.
/// Recovery events are used to:
/// - Track automatic and manual recovery attempts
/// - Monitor system health and recovery patterns
/// - Support audit trails for recovery operations
/// - Enable analysis of recovery effectiveness
///
/// # Usage
///
/// ```rust
/// use cloacina::dal::{DAL, RecoveryEventDAL};
/// use cloacina::models::recovery_event::{NewRecoveryEvent, RecoveryType};
///
/// let dal = DAL::new(/* connection details */);
/// let recovery_dal = RecoveryEventDAL { dal: &dal };
///
/// // Create a new recovery event
/// let new_event = NewRecoveryEvent {
///     // ... event details ...
/// };
/// let event = recovery_dal.create(new_event)?;
///
/// // Query recovery events
/// let pipeline_events = recovery_dal.get_by_pipeline(pipeline_id)?;
/// let task_events = recovery_dal.get_by_task(task_id)?;
/// ```
///
/// # Error Handling
///
/// All methods return a `Result` type that can contain either:
/// - `Ok(T)` - The successful result
/// - `Err(ValidationError)` - A validation error that can be either:
///   - `DatabaseConnection` - Failed to establish database connection
///   - `DatabaseQuery` - Failed to execute database query
///
/// # Related Components
///
/// - `RecoveryEvent` - The model representing a recovery event
/// - `NewRecoveryEvent` - The model for creating new recovery events
/// - `RecoveryType` - Enum defining different types of recovery events
/// - `DAL` - The parent data access layer
pub struct RecoveryEventDAL<'a> {
    pub dal: &'a DAL,
}

impl<'a> RecoveryEventDAL<'a> {
    /// Creates a new recovery event record.
    ///
    /// # Arguments
    ///
    /// * `new_event` - The recovery event data to insert. Must contain valid:
    ///   - `pipeline_execution_id` or `task_execution_id`
    ///   - `recovery_type`
    ///   - `recovered_at` timestamp
    ///
    /// # Returns
    ///
    /// Returns the created `RecoveryEvent` on success.
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if:
    /// - Database connection fails
    /// - Required fields are missing
    /// - Database constraints are violated
    ///
    /// # Example
    ///
    /// ```rust
    /// let new_event = NewRecoveryEvent {
    ///     pipeline_execution_id: Some(pipeline_id),
    ///     task_execution_id: None,
    ///     recovery_type: RecoveryType::WorkflowUnavailable,
    ///     recovered_at: Utc::now(),
    ///     // ... other fields ...
    /// };
    /// let event = recovery_dal.create(new_event)?;
    /// ```
    pub fn create(&self, new_event: NewRecoveryEvent) -> Result<RecoveryEvent, ValidationError> {
        let mut conn = self
            .dal
            .pool
            .get()
            .map_err(|e| ValidationError::DatabaseConnection {
                message: format!("Failed to get connection: {}", e),
            })?;

        // For SQLite, we need to manually generate the UUID and timestamps
        let id = UniversalUuid::new_v4();
        let now = current_timestamp();

        // Insert with explicit values for SQLite
        diesel::insert_into(recovery_events::table)
            .values((
                recovery_events::id.eq(&id),
                recovery_events::pipeline_execution_id.eq(&new_event.pipeline_execution_id),
                recovery_events::task_execution_id.eq(&new_event.task_execution_id),
                recovery_events::recovery_type.eq(&new_event.recovery_type),
                recovery_events::recovered_at.eq(&now),
                recovery_events::details.eq(&new_event.details),
                recovery_events::created_at.eq(&now),
                recovery_events::updated_at.eq(&now),
            ))
            .execute(&mut conn)
            .map_err(|e| ValidationError::DatabaseQuery {
                message: format!("Failed to create recovery event: {}", e),
            })?;

        // Retrieve the inserted record
        let result = recovery_events::table
            .find(id)
            .first(&mut conn)
            .map_err(|e| ValidationError::DatabaseQuery {
                message: format!("Failed to retrieve created recovery event: {}", e),
            })?;

        Ok(result)
    }

    /// Gets all recovery events for a specific pipeline execution.
    ///
    /// # Arguments
    ///
    /// * `pipeline_execution_id` - The UUID of the pipeline execution to query
    ///
    /// # Returns
    ///
    /// Returns a vector of `RecoveryEvent` records for the pipeline, ordered by
    /// recovery timestamp in descending order (most recent first).
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if:
    /// - Database connection fails
    /// - Query execution fails
    ///
    /// # Example
    ///
    /// ```rust
    /// let pipeline_id = UniversalUuid::new_v4();
    /// let events = recovery_dal.get_by_pipeline(pipeline_id)?;
    /// for event in events {
    ///     println!("Recovery at {}: {:?}", event.recovered_at, event.recovery_type);
    /// }
    /// ```
    pub fn get_by_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        let mut conn = self
            .dal
            .pool
            .get()
            .map_err(|e| ValidationError::DatabaseConnection {
                message: format!("Failed to get connection: {}", e),
            })?;

        let events = recovery_events::table
            .filter(recovery_events::pipeline_execution_id.eq(pipeline_execution_id))
            .order(recovery_events::recovered_at.desc())
            .load(&mut conn)
            .map_err(|e| ValidationError::DatabaseQuery {
                message: format!("Failed to get recovery events: {}", e),
            })?;

        Ok(events)
    }

    /// Gets all recovery events for a specific task execution.
    ///
    /// # Arguments
    ///
    /// * `task_execution_id` - The UUID of the task execution to query
    ///
    /// # Returns
    ///
    /// Returns a vector of `RecoveryEvent` records for the task, ordered by
    /// recovery timestamp in descending order (most recent first).
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if:
    /// - Database connection fails
    /// - Query execution fails
    ///
    /// # Example
    ///
    /// ```rust
    /// let task_id = UniversalUuid::new_v4();
    /// let events = recovery_dal.get_by_task(task_id)?;
    /// for event in events {
    ///     println!("Task recovery at {}: {:?}", event.recovered_at, event.recovery_type);
    /// }
    /// ```
    pub fn get_by_task(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<Vec<RecoveryEvent>, ValidationError> {
        let mut conn = self
            .dal
            .pool
            .get()
            .map_err(|e| ValidationError::DatabaseConnection {
                message: format!("Failed to get connection: {}", e),
            })?;

        let events = recovery_events::table
            .filter(recovery_events::task_execution_id.eq(task_execution_id))
            .order(recovery_events::recovered_at.desc())
            .load(&mut conn)
            .map_err(|e| ValidationError::DatabaseQuery {
                message: format!("Failed to get recovery events: {}", e),
            })?;

        Ok(events)
    }

    /// Gets recovery events by type for monitoring and analysis.
    ///
    /// # Arguments
    ///
    /// * `recovery_type` - The type of recovery event to filter by (e.g., "workflow_unavailable")
    ///
    /// # Returns
    ///
    /// Returns a vector of `RecoveryEvent` records of the specified type, ordered by
    /// recovery timestamp in descending order (most recent first).
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if:
    /// - Database connection fails
    /// - Query execution fails
    ///
    /// # Example
    ///
    /// ```rust
    /// let events = recovery_dal.get_by_type("workflow_unavailable")?;
    /// for event in events {
    ///     println!("Workflow unavailable at {}: {:?}", event.recovered_at, event.recovery_type);
    /// }
    /// ```
    pub fn get_by_type(&self, recovery_type: &str) -> Result<Vec<RecoveryEvent>, ValidationError> {
        let mut conn = self
            .dal
            .pool
            .get()
            .map_err(|e| ValidationError::DatabaseConnection {
                message: format!("Failed to get connection: {}", e),
            })?;

        let events = recovery_events::table
            .filter(recovery_events::recovery_type.eq(recovery_type))
            .order(recovery_events::recovered_at.desc())
            .load(&mut conn)
            .map_err(|e| ValidationError::DatabaseQuery {
                message: format!("Failed to get recovery events: {}", e),
            })?;

        Ok(events)
    }

    /// Gets all workflow unavailability events for monitoring unknown workflow cleanup.
    ///
    /// This is a convenience method that filters for `RecoveryType::WorkflowUnavailable` events.
    /// It's particularly useful for monitoring and cleanup of orphaned workflows.
    ///
    /// # Returns
    ///
    /// Returns a vector of `RecoveryEvent` records for workflow unavailability, ordered by
    /// recovery timestamp in descending order (most recent first).
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if:
    /// - Database connection fails
    /// - Query execution fails
    ///
    /// # Example
    ///
    /// ```rust
    /// let events = recovery_dal.get_workflow_unavailable_events()?;
    /// for event in events {
    ///     println!("Workflow unavailable at {}: {:?}", event.recovered_at, event.recovery_type);
    /// }
    /// ```
    pub fn get_workflow_unavailable_events(&self) -> Result<Vec<RecoveryEvent>, ValidationError> {
        self.get_by_type(&RecoveryType::WorkflowUnavailable.as_str())
    }

    /// Gets recent recovery events for monitoring purposes.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of events to return. Use a reasonable value (e.g., 100)
    ///             to avoid performance issues with large result sets.
    ///
    /// # Returns
    ///
    /// Returns the most recent recovery events, limited by the specified count and
    /// ordered by recovery timestamp in descending order (most recent first).
    ///
    /// # Errors
    ///
    /// Returns a `ValidationError` if:
    /// - Database connection fails
    /// - Query execution fails
    ///
    /// # Example
    ///
    /// ```rust
    /// let recent_events = recovery_dal.get_recent(100)?;
    /// for event in recent_events {
    ///     println!("Recent recovery at {}: {:?}", event.recovered_at, event.recovery_type);
    /// }
    /// ```
    pub fn get_recent(&self, limit: i64) -> Result<Vec<RecoveryEvent>, ValidationError> {
        let mut conn = self
            .dal
            .pool
            .get()
            .map_err(|e| ValidationError::DatabaseConnection {
                message: format!("Failed to get connection: {}", e),
            })?;

        let events = recovery_events::table
            .order(recovery_events::recovered_at.desc())
            .limit(limit)
            .load(&mut conn)
            .map_err(|e| ValidationError::DatabaseQuery {
                message: format!("Failed to get recent recovery events: {}", e),
            })?;

        Ok(events)
    }
}
