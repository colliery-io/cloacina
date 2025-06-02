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

//! Cron schedule management module for time-based workflow execution.
//!
//! This module provides structures for working with cron schedules in the database.
//! Cron schedules allow workflows to be executed automatically at specified times
//! using standard cron expressions with timezone support.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a cron schedule record in the database.
///
/// This structure maps to the `cron_schedules` table in the database and provides
/// functionality for querying and managing time-based workflow schedules.
///
/// # Fields
/// * `id` - Unique identifier for the cron schedule
/// * `workflow_name` - Name of the workflow to execute
/// * `cron_expression` - Standard cron expression (e.g., "0 2 * * *")
/// * `timezone` - Timezone for cron interpretation (e.g., "America/New_York", "UTC")
/// * `enabled` - Whether the schedule is currently active
/// * `catchup_policy` - How to handle missed executions ("skip", "run_once", "run_all")
/// * `start_date` - Optional start date for the schedule (None = immediate)
/// * `end_date` - Optional end date for the schedule (None = no end)
/// * `next_run_at` - When this schedule should next execute
/// * `last_run_at` - When this schedule was last executed (None = never)
/// * `created_at` - Timestamp when the schedule was created
/// * `updated_at` - Timestamp when the schedule was last modified
#[derive(Debug, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::cron_schedules)]
#[cfg_attr(feature = "postgres", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "sqlite", diesel(check_for_backend(diesel::sqlite::Sqlite)))]
pub struct CronSchedule {
    pub id: UniversalUuid,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: bool,
    pub catchup_policy: String,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub next_run_at: UniversalTimestamp,
    pub last_run_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new cron schedule records in the database.
///
/// This structure is used when inserting new cron schedules into the database.
/// Fields like `id`, `created_at`, and `updated_at` are automatically populated
/// by the database.
///
/// # Fields
/// * `workflow_name` - Name of the workflow to execute
/// * `cron_expression` - Standard cron expression (e.g., "0 2 * * *")
/// * `timezone` - Timezone for cron interpretation (defaults to "UTC" if not specified)
/// * `enabled` - Whether the schedule should be active (defaults to true)
/// * `catchup_policy` - How to handle missed executions (defaults to "skip")
/// * `start_date` - Optional start date for the schedule
/// * `end_date` - Optional end date for the schedule
/// * `next_run_at` - When this schedule should first execute
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::database::schema::cron_schedules)]
pub struct NewCronSchedule {
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: Option<String>,
    pub enabled: Option<bool>,
    pub catchup_policy: Option<String>,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub next_run_at: UniversalTimestamp,
}

/// Enum representing the different catchup policies for missed executions.
///
/// When a cron schedule misses one or more executions (e.g., due to system downtime),
/// the catchup policy determines how to handle these missed runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CatchupPolicy {
    /// Skip all missed executions and continue with the next scheduled time.
    /// This is the default and safest option for most use cases.
    Skip,
    /// Run the workflow once with the most recent missed execution time.
    /// Useful when you want to process the latest missed data.
    RunOnce,
    /// Run the workflow for all missed execution times in chronological order.
    /// Use with caution as this can cause a large backlog of executions.
    RunAll,
}

impl From<CatchupPolicy> for String {
    fn from(policy: CatchupPolicy) -> Self {
        match policy {
            CatchupPolicy::Skip => "skip".to_string(),
            CatchupPolicy::RunOnce => "run_once".to_string(),
            CatchupPolicy::RunAll => "run_all".to_string(),
        }
    }
}

impl From<String> for CatchupPolicy {
    fn from(s: String) -> Self {
        match s.as_str() {
            "run_once" => CatchupPolicy::RunOnce,
            "run_all" => CatchupPolicy::RunAll,
            _ => CatchupPolicy::Skip, // Default fallback
        }
    }
}

impl From<&str> for CatchupPolicy {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

/// Configuration structure for creating new cron schedules.
///
/// This provides a more convenient interface for creating cron schedules
/// with type-safe enums and optional fields with sensible defaults.
///
/// # Examples
///
/// ```rust
/// use cloacina::models::cron_schedule::{ScheduleConfig, CatchupPolicy};
/// use chrono::{DateTime, Utc};
///
/// // Simple daily backup at 2 AM UTC
/// let config = ScheduleConfig {
///     name: "daily_backup".to_string(),
///     cron: "0 2 * * *".to_string(),
///     workflow: "backup_workflow".to_string(),
///     timezone: "UTC".to_string(),
///     catchup_policy: CatchupPolicy::Skip,
///     start_date: None,
///     end_date: None,
/// };
///
/// // Report generation with time window in EST
/// let start = DateTime::parse_from_rfc3339("2025-02-01T00:00:00Z")?.with_timezone(&Utc);
/// let end = DateTime::parse_from_rfc3339("2025-02-28T23:59:59Z")?.with_timezone(&Utc);
/// let config = ScheduleConfig {
///     name: "february_reports".to_string(),
///     cron: "0 9 * * 1-5".to_string(),  // 9 AM weekdays
///     workflow: "report_workflow".to_string(),
///     timezone: "America/New_York".to_string(),
///     catchup_policy: CatchupPolicy::RunOnce,
///     start_date: Some(start),
///     end_date: Some(end),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ScheduleConfig {
    /// Unique name/identifier for the schedule
    pub name: String,
    /// Cron expression (e.g., "0 2 * * *" for daily at 2 AM)
    pub cron: String,
    /// Name of the workflow to execute
    pub workflow: String,
    /// Timezone for cron interpretation (e.g., "America/New_York", "UTC")
    pub timezone: String,
    /// Policy for handling missed executions
    pub catchup_policy: CatchupPolicy,
    /// Optional start date (None = immediate)
    pub start_date: Option<DateTime<Utc>>,
    /// Optional end date (None = no end)
    pub end_date: Option<DateTime<Utc>>,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            cron: String::new(),
            workflow: String::new(),
            timezone: "UTC".to_string(),
            catchup_policy: CatchupPolicy::Skip,
            start_date: None,
            end_date: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::current_timestamp;

    #[test]
    fn test_cron_schedule_creation() {
        let now = current_timestamp();
        let schedule = CronSchedule {
            id: UniversalUuid::new_v4(),
            workflow_name: "test_workflow".to_string(),
            cron_expression: "0 2 * * *".to_string(),
            timezone: "UTC".to_string(),
            enabled: true,
            catchup_policy: "skip".to_string(),
            start_date: None,
            end_date: None,
            next_run_at: now,
            last_run_at: None,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(schedule.workflow_name, "test_workflow");
        assert_eq!(schedule.cron_expression, "0 2 * * *");
        assert_eq!(schedule.timezone, "UTC");
        assert!(schedule.enabled);
        assert_eq!(schedule.catchup_policy, "skip");
    }

    #[test]
    fn test_new_cron_schedule_creation() {
        let now = current_timestamp();
        let new_schedule = NewCronSchedule {
            workflow_name: "test_workflow".to_string(),
            cron_expression: "0 2 * * *".to_string(),
            timezone: Some("America/New_York".to_string()),
            enabled: Some(true),
            catchup_policy: Some("run_once".to_string()),
            start_date: None,
            end_date: None,
            next_run_at: now,
        };

        assert_eq!(new_schedule.workflow_name, "test_workflow");
        assert_eq!(new_schedule.cron_expression, "0 2 * * *");
        assert_eq!(new_schedule.timezone, Some("America/New_York".to_string()));
        assert_eq!(new_schedule.enabled, Some(true));
        assert_eq!(new_schedule.catchup_policy, Some("run_once".to_string()));
    }

    #[test]
    fn test_catchup_policy_conversions() {
        // String to CatchupPolicy
        assert_eq!(CatchupPolicy::from("skip"), CatchupPolicy::Skip);
        assert_eq!(CatchupPolicy::from("run_once"), CatchupPolicy::RunOnce);
        assert_eq!(CatchupPolicy::from("run_all"), CatchupPolicy::RunAll);
        assert_eq!(CatchupPolicy::from("invalid"), CatchupPolicy::Skip); // Fallback

        // CatchupPolicy to String
        assert_eq!(String::from(CatchupPolicy::Skip), "skip");
        assert_eq!(String::from(CatchupPolicy::RunOnce), "run_once");
        assert_eq!(String::from(CatchupPolicy::RunAll), "run_all");
    }

    #[test]
    fn test_schedule_config_default() {
        let config = ScheduleConfig::default();
        assert_eq!(config.timezone, "UTC");
        assert_eq!(config.catchup_policy, CatchupPolicy::Skip);
        assert!(config.start_date.is_none());
        assert!(config.end_date.is_none());
    }

    #[test]
    fn test_schedule_config_builder_pattern() {
        let config = ScheduleConfig {
            name: "hourly_sync".to_string(),
            cron: "0 * * * *".to_string(),
            workflow: "sync_workflow".to_string(),
            timezone: "Europe/London".to_string(),
            catchup_policy: CatchupPolicy::RunOnce,
            ..ScheduleConfig::default()
        };

        assert_eq!(config.name, "hourly_sync");
        assert_eq!(config.cron, "0 * * * *");
        assert_eq!(config.workflow, "sync_workflow");
        assert_eq!(config.timezone, "Europe/London");
        assert_eq!(config.catchup_policy, CatchupPolicy::RunOnce);
        assert!(config.start_date.is_none());
        assert!(config.end_date.is_none());
    }
}