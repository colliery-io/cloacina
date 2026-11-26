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

/// Enum representing the different catchup policies for missed cron executions.
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

impl CatchupPolicy {
    /// The name stored in `schedules.catchup_policy`.
    pub fn as_str(&self) -> &'static str {
        match self {
            CatchupPolicy::Skip => "skip",
            CatchupPolicy::RunAll => "run_all",
        }
    }

    /// Reads a declared policy: `"skip"` or `"run_all"`, nothing else
    /// (CLOACINA-T-1020). `From<&str>` reads a stored value and turns an
    /// unknown one into `Skip`; a declaration with a typing error must fail.
    pub fn parse_declared(value: &str) -> Result<Self, String> {
        match value {
            "skip" => Ok(CatchupPolicy::Skip),
            "run_all" => Ok(CatchupPolicy::RunAll),
            other => Err(format!(
                "invalid catch-up policy '{}': use \"skip\" or \"run_all\"",
                other
            )),
        }
    }
}

/// What a declaration sets on a cron schedule, next to its expression and
/// timezone (CLOACINA-T-1020). `None` in a field means "not declared": a new
/// schedule gets the default, and an existing schedule keeps its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CronScheduleOptions {
    /// Catch-up policy. Default for a new schedule: `Skip`.
    pub catchup_policy: Option<CatchupPolicy>,
    /// The most runs of the schedule that may be active at once, 1 or more.
    /// At the limit the scheduler takes no fire and leaves the schedule due.
    /// Not declared: a `RunAll` schedule runs one fire at a time, a `Skip`
    /// schedule has no limit.
    pub max_active_runs: Option<i32>,
}

impl CronScheduleOptions {
    /// Builds the options from declared values, as the trigger metadata, the
    /// instance API and the Python bindings carry them.
    pub fn from_declared(
        catchup_policy: Option<&str>,
        max_active_runs: Option<u32>,
    ) -> Result<Self, String> {
        let max_active_runs = match max_active_runs {
            None => None,
            Some(0) => {
                return Err("invalid max_active_runs 0: use 1 or more, or leave it out".to_string());
            }
            Some(limit) => Some(
                i32::try_from(limit)
                    .map_err(|_| format!("invalid max_active_runs {}: it is too large", limit))?,
            ),
        };
        Ok(Self {
            catchup_policy: catchup_policy
                .map(CatchupPolicy::parse_declared)
                .transpose()?,
            max_active_runs,
        })
    }

    /// Sets the declared values on a new schedule row.
    pub fn apply_to(&self, new_schedule: &mut NewSchedule) {
        if let Some(policy) = &self.catchup_policy {
            new_schedule.catchup_policy = Some(policy.as_str().to_string());
        }
        if let Some(limit) = self.max_active_runs {
            new_schedule.max_active_runs = Some(limit);
        }
    }
}

/// The limit on active runs that the scheduler applies to a cron schedule
/// (CLOACINA-T-1020). `None` is no limit.
///
/// A declared limit wins. With none declared, a `run_all` schedule runs one
/// fire at a time: its missed fires run in order, and a long backlog cannot
/// start everything at once. A `skip` schedule has no limit.
pub fn effective_max_active_runs(
    catchup_policy: Option<&str>,
    max_active_runs: Option<i32>,
) -> Option<i64> {
    match max_active_runs {
        Some(limit) => Some(i64::from(limit.max(1))),
        None => match CatchupPolicy::from(catchup_policy.unwrap_or("skip")) {
            CatchupPolicy::RunAll => Some(1),
            CatchupPolicy::Skip => None,
        },
    }
}

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

    /// Transient operator pause (CLOACI-T-0749). When set, the scheduler does
    /// not fire this schedule. Distinct from `enabled` (deliberate on/off).
    pub paused: UniversalBool,
    pub paused_at: Option<UniversalTimestamp>,

    // CLOACI-I-0116: named parameterized instance (both NULL for anonymous
    // schedules). `params` = the instance's FULLY-RESOLVED bound parameters
    // (JSON object, defaults snapshotted at instantiation); merged into the
    // context at every fire. `instance_name` = human management identity,
    // unique per workflow.
    pub params: Option<String>,
    pub instance_name: Option<String>,

    /// The most runs of this cron schedule that may be active at once
    /// (CLOACINA-T-1020). `None` = not declared: a `run_all` schedule runs
    /// one fire at a time, a `skip` schedule has no limit. `serde(default)`
    /// keeps older serialized schedules readable.
    #[serde(default)]
    pub max_active_runs: Option<i32>,
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

    /// Returns true if the schedule is paused (CLOACI-T-0749). A paused
    /// schedule is skipped by the scheduler even when `enabled`.
    pub fn is_paused(&self) -> bool {
        self.paused.is_true()
    }

    /// Returns true if the schedule should currently fire: enabled and not
    /// paused. Callers in the scheduler use this as the single gate.
    pub fn is_active(&self) -> bool {
        self.is_enabled() && !self.is_paused()
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

    // CLOACI-I-0116: named parameterized instance (None = anonymous schedule).
    pub params: Option<String>,
    pub instance_name: Option<String>,

    /// CLOACINA-T-1020: limit on active runs of a cron schedule (None = not
    /// declared).
    #[serde(default)]
    pub max_active_runs: Option<i32>,
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
            params: None,
            instance_name: None,
            max_active_runs: None,
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
            params: None,
            instance_name: None,
            max_active_runs: None,
        }
    }
}

/// Represents a schedule execution record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub workflow_execution_id: Option<UniversalUuid>,

    // Cron-specific
    pub scheduled_time: Option<UniversalTimestamp>,
    pub claimed_at: Option<UniversalTimestamp>,

    // Trigger-specific
    pub context_hash: Option<String>,

    pub started_at: UniversalTimestamp,
    pub completed_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,

    // Recovery accounting (CLOACI-T-0926). These live on the audit row rather
    // than in the recovery service's process memory so the attempt cap survives
    // a restart, and so replicas can CAS for ownership of a lost handoff.
    // `serde(default)` keeps older serialized payloads readable.
    /// Number of recovery re-fire attempts already spent on this handoff.
    #[serde(default)]
    pub recovery_attempts: i32,
    /// Recovery service instance that currently owns the re-fire, if any.
    #[serde(default)]
    pub recovery_claimed_by: Option<UniversalUuid>,
    /// Last liveness beat from the claim owner; a stale beat frees the claim.
    #[serde(default)]
    pub recovery_heartbeat_at: Option<UniversalTimestamp>,
}

/// Structure for creating new schedule execution records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewScheduleExecution {
    pub schedule_id: UniversalUuid,
    pub workflow_execution_id: Option<UniversalUuid>,
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

    /// CLOACINA-T-1020: a declared policy has two values; a stored one that
    /// is unknown still reads as `Skip`.
    #[test]
    fn declared_catchup_policy_is_strict() {
        assert_eq!(
            CatchupPolicy::parse_declared("skip"),
            Ok(CatchupPolicy::Skip)
        );
        assert_eq!(
            CatchupPolicy::parse_declared("run_all"),
            Ok(CatchupPolicy::RunAll)
        );
        assert!(CatchupPolicy::parse_declared("all").is_err());
        assert!(CatchupPolicy::parse_declared("").is_err());
        assert_eq!(CatchupPolicy::from("all"), CatchupPolicy::Skip);

        assert!(CronScheduleOptions::from_declared(Some("RUN_ALL"), None).is_err());
        assert_eq!(
            CronScheduleOptions::from_declared(None, None),
            Ok(CronScheduleOptions::default())
        );

        let now = current_timestamp();
        let mut schedule = NewSchedule::cron("wf", "0 * * * *", now);
        CronScheduleOptions::default().apply_to(&mut schedule);
        assert_eq!(schedule.catchup_policy.as_deref(), Some("skip"));
        CronScheduleOptions::from_declared(Some("run_all"), None)
            .unwrap()
            .apply_to(&mut schedule);
        assert_eq!(schedule.catchup_policy.as_deref(), Some("run_all"));
        assert_eq!(schedule.max_active_runs, None);
    }

    /// CLOACINA-T-1020: a declared limit is 1 or more, and the limit the
    /// scheduler applies follows the declaration, then the policy.
    #[test]
    fn declared_max_active_runs_and_the_limit_in_effect() {
        assert!(CronScheduleOptions::from_declared(None, Some(0)).is_err());
        assert!(CronScheduleOptions::from_declared(None, Some(u32::MAX)).is_err());
        let options = CronScheduleOptions::from_declared(None, Some(4)).unwrap();
        assert_eq!(options.max_active_runs, Some(4));

        let now = current_timestamp();
        let mut schedule = NewSchedule::cron("wf", "0 * * * *", now);
        assert_eq!(schedule.max_active_runs, None);
        options.apply_to(&mut schedule);
        assert_eq!(schedule.max_active_runs, Some(4));
        assert_eq!(schedule.catchup_policy.as_deref(), Some("skip"));

        // Not declared: run_all runs one at a time, skip has no limit.
        assert_eq!(effective_max_active_runs(Some("run_all"), None), Some(1));
        assert_eq!(effective_max_active_runs(Some("skip"), None), None);
        assert_eq!(effective_max_active_runs(None, None), None);
        // Declared: the declaration wins for both policies.
        assert_eq!(effective_max_active_runs(Some("run_all"), Some(8)), Some(8));
        assert_eq!(effective_max_active_runs(Some("skip"), Some(1)), Some(1));
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
            paused: UniversalBool::new(false),
            paused_at: None,
            params: None,
            instance_name: None,
            max_active_runs: None,
        };

        assert!(schedule.is_trigger());
        assert!(!schedule.is_cron());
        assert!(schedule.is_enabled());
        assert_eq!(schedule.poll_interval(), Some(Duration::from_secs(5)));
        assert!(!schedule.allows_concurrent());
    }
}
