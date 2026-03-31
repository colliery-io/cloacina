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

//! Unified schedule management module for both cron and trigger-based workflow execution.
//!
//! This module provides domain structures for the unified `schedules` and
//! `schedule_executions` tables, replacing the separate cron and trigger models.

use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// The type of schedule — determines which fields are relevant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleType {
    Cron,
    Trigger,
}

impl From<&str> for ScheduleType {
    fn from(s: &str) -> Self {
        match s {
            "trigger" => ScheduleType::Trigger,
            _ => ScheduleType::Cron,
        }
    }
}

impl From<String> for ScheduleType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl std::fmt::Display for ScheduleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleType::Cron => write!(f, "cron"),
            ScheduleType::Trigger => write!(f, "trigger"),
        }
    }
}

/// Represents a unified schedule record (domain type).
///
/// Contains fields for both cron and trigger schedules. Fields irrelevant
/// to the `schedule_type` will be `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: UniversalUuid,
    pub schedule_type: String,
    pub workflow_name: String,
    pub enabled: UniversalBool,

    // Cron-specific
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub catchup_policy: Option<String>,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,

    // Trigger-specific
    pub trigger_name: Option<String>,
    pub poll_interval_ms: Option<i32>,
    pub allow_concurrent: Option<UniversalBool>,

    // Shared scheduling state
    pub next_run_at: Option<UniversalTimestamp>,
    pub last_run_at: Option<UniversalTimestamp>,
    pub last_poll_at: Option<UniversalTimestamp>,

    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

impl Schedule {
    /// Returns the schedule type as an enum.
    pub fn get_type(&self) -> ScheduleType {
        ScheduleType::from(self.schedule_type.as_str())
    }

    /// Returns true if this is a cron schedule.
    pub fn is_cron(&self) -> bool {
        self.get_type() == ScheduleType::Cron
    }

    /// Returns true if this is a trigger schedule.
    pub fn is_trigger(&self) -> bool {
        self.get_type() == ScheduleType::Trigger
    }

    /// Returns true if the schedule is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.is_true()
    }

    /// Returns the poll interval as a Duration (trigger schedules only).
    pub fn poll_interval(&self) -> Option<Duration> {
        self.poll_interval_ms
            .map(|ms| Duration::from_millis(ms as u64))
    }

    /// Returns true if concurrent executions are allowed (trigger schedules only).
    pub fn allows_concurrent(&self) -> bool {
        self.allow_concurrent
            .as_ref()
            .map(|b| b.is_true())
            .unwrap_or(false)
    }
}

/// Structure for creating new schedule records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSchedule {
    pub schedule_type: String,
    pub workflow_name: String,
    pub enabled: Option<UniversalBool>,

    // Cron-specific
    pub cron_expression: Option<String>,
    pub timezone: Option<String>,
    pub catchup_policy: Option<String>,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,

    // Trigger-specific
    pub trigger_name: Option<String>,
    pub poll_interval_ms: Option<i32>,
    pub allow_concurrent: Option<UniversalBool>,

    // Shared
    pub next_run_at: Option<UniversalTimestamp>,
}

impl NewSchedule {
    /// Create a new cron schedule.
    pub fn cron(
        workflow_name: &str,
        cron_expression: &str,
        next_run_at: UniversalTimestamp,
    ) -> Self {
        Self {
            schedule_type: "cron".to_string(),
            workflow_name: workflow_name.to_string(),
            enabled: Some(UniversalBool::new(true)),
            cron_expression: Some(cron_expression.to_string()),
            timezone: Some("UTC".to_string()),
            catchup_policy: Some("skip".to_string()),
            start_date: None,
            end_date: None,
            trigger_name: None,
            poll_interval_ms: None,
            allow_concurrent: None,
            next_run_at: Some(next_run_at),
        }
    }

    /// Create a new trigger schedule.
    pub fn trigger(trigger_name: &str, workflow_name: &str, poll_interval: Duration) -> Self {
        Self {
            schedule_type: "trigger".to_string(),
            workflow_name: workflow_name.to_string(),
            enabled: Some(UniversalBool::new(true)),
            cron_expression: None,
            timezone: None,
            catchup_policy: None,
            start_date: None,
            end_date: None,
            trigger_name: Some(trigger_name.to_string()),
            poll_interval_ms: Some(poll_interval.as_millis() as i32),
            allow_concurrent: Some(UniversalBool::new(false)),
            next_run_at: None,
        }
    }
}

/// Represents a schedule execution record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: Option<UniversalUuid>,

    // Cron-specific
    pub scheduled_time: Option<UniversalTimestamp>,
    pub claimed_at: Option<UniversalTimestamp>,

    // Trigger-specific
    pub context_hash: Option<String>,

    pub started_at: UniversalTimestamp,
    pub completed_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new schedule execution records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewScheduleExecution {
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub scheduled_time: Option<UniversalTimestamp>,
    pub claimed_at: Option<UniversalTimestamp>,
    pub context_hash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::current_timestamp;

    #[test]
    fn test_schedule_type_conversions() {
        assert_eq!(ScheduleType::from("cron"), ScheduleType::Cron);
        assert_eq!(ScheduleType::from("trigger"), ScheduleType::Trigger);
        assert_eq!(ScheduleType::from("unknown"), ScheduleType::Cron);
        assert_eq!(ScheduleType::Cron.to_string(), "cron");
        assert_eq!(ScheduleType::Trigger.to_string(), "trigger");
    }

    #[test]
    fn test_new_cron_schedule() {
        let now = current_timestamp();
        let schedule = NewSchedule::cron("my_workflow", "0 2 * * *", now);
        assert_eq!(schedule.schedule_type, "cron");
        assert_eq!(schedule.workflow_name, "my_workflow");
        assert_eq!(schedule.cron_expression.as_deref(), Some("0 2 * * *"));
        assert!(schedule.trigger_name.is_none());
    }

    #[test]
    fn test_new_trigger_schedule() {
        let schedule =
            NewSchedule::trigger("file_watcher", "process_files", Duration::from_secs(5));
        assert_eq!(schedule.schedule_type, "trigger");
        assert_eq!(schedule.workflow_name, "process_files");
        assert_eq!(schedule.trigger_name.as_deref(), Some("file_watcher"));
        assert_eq!(schedule.poll_interval_ms, Some(5000));
        assert!(schedule.cron_expression.is_none());
    }

    #[test]
    fn test_schedule_helpers() {
        let now = current_timestamp();
        let schedule = Schedule {
            id: UniversalUuid::new_v4(),
            schedule_type: "trigger".to_string(),
            workflow_name: "test".to_string(),
            enabled: UniversalBool::new(true),
            cron_expression: None,
            timezone: None,
            catchup_policy: None,
            start_date: None,
            end_date: None,
            trigger_name: Some("my_trigger".to_string()),
            poll_interval_ms: Some(5000),
            allow_concurrent: Some(UniversalBool::new(false)),
            next_run_at: None,
            last_run_at: None,
            last_poll_at: None,
            created_at: now,
            updated_at: now,
        };

        assert!(schedule.is_trigger());
        assert!(!schedule.is_cron());
        assert!(schedule.is_enabled());
        assert_eq!(schedule.poll_interval(), Some(Duration::from_secs(5)));
        assert!(!schedule.allows_concurrent());
    }
}
