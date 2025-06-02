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

//! Cron execution audit trail models for tracking scheduled workflow handoffs.
//!
//! This module provides structures for recording every handoff from the cron scheduler
//! to the pipeline executor. The audit trail is critical for preventing duplicate
//! executions and providing observability into the scheduling system.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a cron execution audit record in the database.
///
/// This structure maps to the `cron_executions` table and tracks every handoff
/// from the cron scheduler to the pipeline executor. Each record represents
/// a successful claim and handoff of a scheduled execution.
///
/// # Purpose
/// - **Duplicate Prevention**: The UNIQUE constraint on (schedule_id, scheduled_time)
///   prevents multiple executions for the same schedule at the same time
/// - **Audit Trail**: Provides complete history of what was scheduled and when
/// - **Correlation**: Links cron schedules to their resulting pipeline executions
/// - **Observability**: Enables monitoring and debugging of the scheduling system
///
/// # Fields
/// * `id` - Unique identifier for this execution record
/// * `schedule_id` - Foreign key to the cron schedule that triggered this execution
/// * `pipeline_execution_id` - Foreign key to the resulting pipeline execution
/// * `scheduled_time` - The original scheduled execution time from the cron expression
/// * `claimed_at` - When the cron scheduler claimed and handed off this execution
/// * `created_at` - Timestamp when the audit record was created
/// * `updated_at` - Timestamp when the audit record was last modified
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::cron_executions)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "sqlite", diesel(check_for_backend(diesel::sqlite::Sqlite)))]
pub struct CronExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: UniversalUuid,
    pub scheduled_time: UniversalTimestamp,
    pub claimed_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new cron execution audit records.
///
/// This structure is used when inserting new execution records into the database
/// after successfully claiming a schedule and handing it off to the pipeline executor.
///
/// # Fields
/// * `schedule_id` - ID of the cron schedule being executed
/// * `pipeline_execution_id` - ID of the pipeline execution that was created
/// * `scheduled_time` - The original time this execution was scheduled for
/// * `claimed_at` - When the handoff occurred (defaults to current time)
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::database::schema::cron_executions)]
pub struct NewCronExecution {
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub scheduled_time: UniversalTimestamp,
    pub claimed_at: Option<UniversalTimestamp>,
}

impl NewCronExecution {
    /// Creates a new cron execution audit record for guaranteed execution.
    ///
    /// This creates an execution intent record BEFORE handing off to the pipeline executor.
    /// The pipeline_execution_id will be set later after successful handoff.
    ///
    /// # Arguments
    /// * `schedule_id` - ID of the cron schedule
    /// * `scheduled_time` - The original scheduled execution time
    ///
    /// # Returns
    /// * `NewCronExecution` - Ready to insert into database
    ///
    /// # Examples
    /// ```rust
    /// use cloacina::models::cron_execution::NewCronExecution;
    /// use cloacina::database::universal_types::{UniversalUuid, UniversalTimestamp};
    /// use chrono::Utc;
    ///
    /// let schedule_id = UniversalUuid::new_v4();
    /// let scheduled_time = UniversalTimestamp(Utc::now());
    ///
    /// let new_execution = NewCronExecution::new(schedule_id, scheduled_time);
    /// ```
    pub fn new(schedule_id: UniversalUuid, scheduled_time: UniversalTimestamp) -> Self {
        Self {
            schedule_id,
            pipeline_execution_id: None, // Will be set after successful handoff
            scheduled_time,
            claimed_at: Some(UniversalTimestamp::now()),
        }
    }

    /// Creates a new cron execution record with pipeline execution ID (for legacy/direct usage).
    ///
    /// # Arguments
    /// * `schedule_id` - ID of the cron schedule
    /// * `pipeline_execution_id` - ID of the created pipeline execution
    /// * `scheduled_time` - The original scheduled execution time
    ///
    /// # Returns
    /// * `NewCronExecution` - Ready to insert into database
    pub fn with_pipeline_execution(
        schedule_id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
        scheduled_time: UniversalTimestamp,
    ) -> Self {
        Self {
            schedule_id,
            pipeline_execution_id: Some(pipeline_execution_id),
            scheduled_time,
            claimed_at: Some(UniversalTimestamp::now()),
        }
    }

    /// Creates a new cron execution record with a specific claimed_at time.
    ///
    /// # Arguments
    /// * `schedule_id` - ID of the cron schedule
    /// * `pipeline_execution_id` - Optional ID of the created pipeline execution
    /// * `scheduled_time` - The original scheduled execution time
    /// * `claimed_at` - When the execution was claimed and handed off
    ///
    /// # Returns
    /// * `NewCronExecution` - Ready to insert into database
    pub fn with_claimed_at(
        schedule_id: UniversalUuid,
        pipeline_execution_id: Option<UniversalUuid>,
        scheduled_time: UniversalTimestamp,
        claimed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schedule_id,
            pipeline_execution_id,
            scheduled_time,
            claimed_at: Some(UniversalTimestamp(claimed_at)),
        }
    }
}

impl CronExecution {
    /// Returns the scheduled time as a DateTime<Utc>.
    pub fn scheduled_time(&self) -> DateTime<Utc> {
        self.scheduled_time.0
    }

    /// Returns the claimed time as a DateTime<Utc>.
    pub fn claimed_at(&self) -> DateTime<Utc> {
        self.claimed_at.0
    }

    /// Returns the creation time as a DateTime<Utc>.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at.0
    }

    /// Returns the last update time as a DateTime<Utc>.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at.0
    }

    /// Calculates the delay between scheduled time and when it was actually claimed.
    ///
    /// # Returns
    /// * `chrono::Duration` - Time difference (positive if claimed late, negative if early)
    ///
    /// # Examples
    /// ```rust
    /// let execution = get_cron_execution_from_db();
    /// let delay = execution.execution_delay();
    ///
    /// if delay.num_seconds() > 60 {
    ///     println!("Execution was {} seconds late", delay.num_seconds());
    /// }
    /// ```
    pub fn execution_delay(&self) -> chrono::Duration {
        self.claimed_at.0 - self.scheduled_time.0
    }

    /// Checks if this execution was claimed within the expected time window.
    ///
    /// # Arguments
    /// * `tolerance` - Maximum acceptable delay
    ///
    /// # Returns
    /// * `bool` - True if execution was timely, false if delayed beyond tolerance
    pub fn is_timely(&self, tolerance: chrono::Duration) -> bool {
        let delay = self.execution_delay();
        delay <= tolerance && delay >= chrono::Duration::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::current_timestamp;
    use chrono::Duration;

    #[test]
    fn test_new_cron_execution() {
        let schedule_id = UniversalUuid::new_v4();
        let scheduled_time = current_timestamp();

        let new_execution = NewCronExecution::new(schedule_id, scheduled_time);

        assert_eq!(new_execution.schedule_id, schedule_id);
        assert_eq!(new_execution.pipeline_execution_id, None);
        assert_eq!(new_execution.scheduled_time, scheduled_time);
        assert!(new_execution.claimed_at.is_some());
    }

    #[test]
    fn test_new_cron_execution_with_pipeline() {
        let schedule_id = UniversalUuid::new_v4();
        let pipeline_id = UniversalUuid::new_v4();
        let scheduled_time = current_timestamp();

        let new_execution =
            NewCronExecution::with_pipeline_execution(schedule_id, pipeline_id, scheduled_time);

        assert_eq!(new_execution.schedule_id, schedule_id);
        assert_eq!(new_execution.pipeline_execution_id, Some(pipeline_id));
        assert_eq!(new_execution.scheduled_time, scheduled_time);
        assert!(new_execution.claimed_at.is_some());
    }

    #[test]
    fn test_new_cron_execution_with_claimed_at() {
        let schedule_id = UniversalUuid::new_v4();
        let pipeline_id = UniversalUuid::new_v4();
        let scheduled_time = current_timestamp();
        let claimed_at = Utc::now();

        let new_execution = NewCronExecution::with_claimed_at(
            schedule_id,
            Some(pipeline_id),
            scheduled_time,
            claimed_at,
        );

        assert_eq!(new_execution.schedule_id, schedule_id);
        assert_eq!(new_execution.pipeline_execution_id, Some(pipeline_id));
        assert_eq!(new_execution.scheduled_time, scheduled_time);
        assert_eq!(new_execution.claimed_at.unwrap().0, claimed_at);
    }

    #[test]
    fn test_cron_execution_delays() {
        let now = Utc::now();
        let scheduled_time = UniversalTimestamp(now - Duration::minutes(1));
        let claimed_at = UniversalTimestamp(now);

        let execution = CronExecution {
            id: UniversalUuid::new_v4(),
            schedule_id: UniversalUuid::new_v4(),
            pipeline_execution_id: UniversalUuid::new_v4(),
            scheduled_time,
            claimed_at,
            created_at: claimed_at,
            updated_at: claimed_at,
        };

        let delay = execution.execution_delay();
        assert_eq!(delay, Duration::minutes(1));

        // Test timeliness
        assert!(execution.is_timely(Duration::minutes(2))); // Within tolerance
        assert!(!execution.is_timely(Duration::seconds(30))); // Beyond tolerance
    }

    #[test]
    fn test_cron_execution_accessor_methods() {
        let now = Utc::now();
        let scheduled_time = UniversalTimestamp(now - Duration::minutes(1));
        let claimed_at = UniversalTimestamp(now);

        let execution = CronExecution {
            id: UniversalUuid::new_v4(),
            schedule_id: UniversalUuid::new_v4(),
            pipeline_execution_id: UniversalUuid::new_v4(),
            scheduled_time,
            claimed_at,
            created_at: claimed_at,
            updated_at: claimed_at,
        };

        assert_eq!(execution.scheduled_time(), scheduled_time.0);
        assert_eq!(execution.claimed_at(), claimed_at.0);
        assert_eq!(execution.created_at(), claimed_at.0);
        assert_eq!(execution.updated_at(), claimed_at.0);
    }
}
