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

//! Trigger execution audit trail models for tracking event-based workflow handoffs.
//!
//! This module provides domain structures for recording every handoff from the trigger scheduler
//! to the pipeline executor, including deduplication tracking.
//! These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a trigger execution audit record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerExecution {
    pub id: UniversalUuid,
    pub trigger_name: String,
    pub context_hash: String,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub started_at: UniversalTimestamp,
    pub completed_at: Option<UniversalTimestamp>,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

impl TriggerExecution {
    /// Returns true if this execution is currently in progress (not completed).
    pub fn is_in_progress(&self) -> bool {
        self.completed_at.is_none()
    }

    /// Returns the duration of this execution if completed.
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|completed| completed.0 - self.started_at.0)
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at.0
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at.map(|ts| ts.0)
    }
}

/// Structure for creating new trigger execution audit records (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTriggerExecution {
    pub id: Option<UniversalUuid>,
    pub trigger_name: String,
    pub context_hash: String,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub started_at: Option<UniversalTimestamp>,
    pub completed_at: Option<UniversalTimestamp>,
}

impl NewTriggerExecution {
    /// Creates a new trigger execution record.
    pub fn new(trigger_name: &str, context_hash: &str) -> Self {
        Self {
            id: Some(UniversalUuid::new_v4()),
            trigger_name: trigger_name.to_string(),
            context_hash: context_hash.to_string(),
            pipeline_execution_id: None,
            started_at: None,
            completed_at: None,
        }
    }

    /// Creates a new trigger execution record with pipeline execution ID.
    pub fn with_pipeline_execution(
        trigger_name: &str,
        context_hash: &str,
        pipeline_execution_id: UniversalUuid,
    ) -> Self {
        Self {
            id: Some(UniversalUuid::new_v4()),
            trigger_name: trigger_name.to_string(),
            context_hash: context_hash.to_string(),
            pipeline_execution_id: Some(pipeline_execution_id),
            started_at: None,
            completed_at: None,
        }
    }

    /// Creates a new trigger execution record with a specific started_at time.
    pub fn with_started_at(
        trigger_name: &str,
        context_hash: &str,
        pipeline_execution_id: Option<UniversalUuid>,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Some(UniversalUuid::new_v4()),
            trigger_name: trigger_name.to_string(),
            context_hash: context_hash.to_string(),
            pipeline_execution_id,
            started_at: Some(UniversalTimestamp(started_at)),
            completed_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::current_timestamp;
    use chrono::Duration;

    #[test]
    fn test_new_trigger_execution() {
        let new_execution = NewTriggerExecution::new("test_trigger", "abc123");

        assert_eq!(new_execution.trigger_name, "test_trigger");
        assert_eq!(new_execution.context_hash, "abc123");
        assert!(new_execution.pipeline_execution_id.is_none());
        assert!(new_execution.started_at.is_none());
        assert!(new_execution.completed_at.is_none());
    }

    #[test]
    fn test_trigger_execution_in_progress() {
        let now = current_timestamp();
        let execution = TriggerExecution {
            id: UniversalUuid::new_v4(),
            trigger_name: "test_trigger".to_string(),
            context_hash: "abc123".to_string(),
            pipeline_execution_id: Some(UniversalUuid::new_v4()),
            started_at: now,
            completed_at: None,
            created_at: now,
            updated_at: now,
        };

        assert!(execution.is_in_progress());
        assert!(execution.duration().is_none());
    }

    #[test]
    fn test_trigger_execution_completed() {
        let start = Utc::now() - Duration::minutes(5);
        let end = Utc::now();

        let execution = TriggerExecution {
            id: UniversalUuid::new_v4(),
            trigger_name: "test_trigger".to_string(),
            context_hash: "abc123".to_string(),
            pipeline_execution_id: Some(UniversalUuid::new_v4()),
            started_at: UniversalTimestamp(start),
            completed_at: Some(UniversalTimestamp(end)),
            created_at: UniversalTimestamp(start),
            updated_at: UniversalTimestamp(end),
        };

        assert!(!execution.is_in_progress());
        let duration = execution.duration().unwrap();
        assert!(duration >= Duration::minutes(4) && duration <= Duration::minutes(6));
    }
}
