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
//! This module provides domain structures for working with cron schedules.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a cron schedule record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    pub id: UniversalUuid,
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: UniversalBool,
    pub catchup_policy: String,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub next_run_at: UniversalTimestamp,
    pub last_run_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new cron schedule records (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCronSchedule {
    pub workflow_name: String,
    pub cron_expression: String,
    pub timezone: Option<String>,
    pub enabled: Option<UniversalBool>,
    pub catchup_policy: Option<String>,
    pub start_date: Option<UniversalTimestamp>,
    pub end_date: Option<UniversalTimestamp>,
    pub next_run_at: UniversalTimestamp,
}

/// Enum representing the different catchup policies for missed executions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CatchupPolicy {
    Skip,
    RunAll,
}

impl From<CatchupPolicy> for String {
    fn from(policy: CatchupPolicy) -> Self {
        match policy {
            CatchupPolicy::Skip => "skip".to_string(),
            CatchupPolicy::RunAll => "run_all".to_string(),
        }
    }
}

impl From<String> for CatchupPolicy {
    fn from(s: String) -> Self {
        match s.as_str() {
            "run_all" => CatchupPolicy::RunAll,
            "run_once" => CatchupPolicy::Skip,
            _ => CatchupPolicy::Skip,
        }
    }
}

impl From<&str> for CatchupPolicy {
    fn from(s: &str) -> Self {
        Self::from(s.to_string())
    }
}

/// Configuration structure for creating new cron schedules.
#[derive(Debug, Clone)]
pub struct ScheduleConfig {
    pub name: String,
    pub cron: String,
    pub workflow: String,
    pub timezone: String,
    pub catchup_policy: CatchupPolicy,
    pub start_date: Option<DateTime<Utc>>,
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
            enabled: UniversalBool::new(true),
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
        assert!(schedule.enabled.is_true());
    }

    #[test]
    fn test_catchup_policy_conversions() {
        assert_eq!(CatchupPolicy::from("skip"), CatchupPolicy::Skip);
        assert_eq!(CatchupPolicy::from("run_all"), CatchupPolicy::RunAll);
        assert_eq!(CatchupPolicy::from("run_once"), CatchupPolicy::Skip);
        assert_eq!(String::from(CatchupPolicy::Skip), "skip");
        assert_eq!(String::from(CatchupPolicy::RunAll), "run_all");
    }
}
