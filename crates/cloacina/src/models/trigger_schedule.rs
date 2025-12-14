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

//! Trigger schedule management module for event-based workflow execution.
//!
//! This module provides domain structures for working with trigger schedules.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalBool, UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Represents a trigger schedule record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSchedule {
    pub id: UniversalUuid,
    pub trigger_name: String,
    pub workflow_name: String,
    pub poll_interval_ms: i32,
    pub allow_concurrent: UniversalBool,
    pub enabled: UniversalBool,
    pub last_poll_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

impl TriggerSchedule {
    /// Returns the poll interval as a Duration.
    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(self.poll_interval_ms as u64)
    }

    /// Returns true if the trigger is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.is_true()
    }

    /// Returns true if concurrent executions are allowed.
    pub fn allows_concurrent(&self) -> bool {
        self.allow_concurrent.is_true()
    }
}

/// Structure for creating new trigger schedule records (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTriggerSchedule {
    pub id: Option<UniversalUuid>,
    pub trigger_name: String,
    pub workflow_name: String,
    pub poll_interval_ms: i32,
    pub allow_concurrent: Option<UniversalBool>,
    pub enabled: Option<UniversalBool>,
}

impl NewTriggerSchedule {
    /// Creates a new trigger schedule.
    pub fn new(trigger_name: &str, workflow_name: &str, poll_interval: Duration) -> Self {
        Self {
            id: Some(UniversalUuid::new_v4()),
            trigger_name: trigger_name.to_string(),
            workflow_name: workflow_name.to_string(),
            poll_interval_ms: poll_interval.as_millis() as i32,
            allow_concurrent: Some(UniversalBool::new(false)),
            enabled: Some(UniversalBool::new(true)),
        }
    }

    /// Sets whether concurrent executions are allowed.
    pub fn with_allow_concurrent(mut self, allow: bool) -> Self {
        self.allow_concurrent = Some(UniversalBool::new(allow));
        self
    }

    /// Sets whether the trigger is enabled.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(UniversalBool::new(enabled));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::current_timestamp;

    #[test]
    fn test_trigger_schedule_creation() {
        let now = current_timestamp();
        let schedule = TriggerSchedule {
            id: UniversalUuid::new_v4(),
            trigger_name: "test_trigger".to_string(),
            workflow_name: "test_workflow".to_string(),
            poll_interval_ms: 5000,
            allow_concurrent: UniversalBool::new(false),
            enabled: UniversalBool::new(true),
            last_poll_at: None,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(schedule.trigger_name, "test_trigger");
        assert_eq!(schedule.workflow_name, "test_workflow");
        assert_eq!(schedule.poll_interval(), Duration::from_secs(5));
        assert!(schedule.is_enabled());
        assert!(!schedule.allows_concurrent());
    }

    #[test]
    fn test_new_trigger_schedule() {
        let new_schedule =
            NewTriggerSchedule::new("my_trigger", "my_workflow", Duration::from_secs(10));

        assert_eq!(new_schedule.trigger_name, "my_trigger");
        assert_eq!(new_schedule.workflow_name, "my_workflow");
        assert_eq!(new_schedule.poll_interval_ms, 10000);
        assert!(!new_schedule.allow_concurrent.unwrap().is_true());
        assert!(new_schedule.enabled.unwrap().is_true());
    }

    #[test]
    fn test_new_trigger_schedule_builders() {
        let new_schedule =
            NewTriggerSchedule::new("concurrent_trigger", "workflow", Duration::from_millis(500))
                .with_allow_concurrent(true)
                .with_enabled(false);

        assert!(new_schedule.allow_concurrent.unwrap().is_true());
        assert!(!new_schedule.enabled.unwrap().is_true());
    }
}
