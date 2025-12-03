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
//! This module provides domain structures for recording every handoff from the cron scheduler
//! to the pipeline executor. These are API-level types; backend-specific models handle database storage.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a cron execution audit record (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronExecution {
    pub id: UniversalUuid,
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub scheduled_time: UniversalTimestamp,
    pub claimed_at: UniversalTimestamp,
    pub created_at: UniversalTimestamp,
    pub updated_at: UniversalTimestamp,
}

/// Structure for creating new cron execution audit records (domain type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewCronExecution {
    pub id: Option<UniversalUuid>,
    pub schedule_id: UniversalUuid,
    pub pipeline_execution_id: Option<UniversalUuid>,
    pub scheduled_time: UniversalTimestamp,
    pub claimed_at: Option<UniversalTimestamp>,
    pub created_at: Option<UniversalTimestamp>,
    pub updated_at: Option<UniversalTimestamp>,
}

impl NewCronExecution {
    /// Creates a new cron execution audit record for guaranteed execution.
    pub fn new(schedule_id: UniversalUuid, scheduled_time: UniversalTimestamp) -> Self {
        Self {
            id: Some(UniversalUuid::new_v4()),
            schedule_id,
            pipeline_execution_id: None,
            scheduled_time,
            claimed_at: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Creates a new cron execution record with pipeline execution ID.
    pub fn with_pipeline_execution(
        schedule_id: UniversalUuid,
        pipeline_execution_id: UniversalUuid,
        scheduled_time: UniversalTimestamp,
    ) -> Self {
        Self {
            id: Some(UniversalUuid::new_v4()),
            schedule_id,
            pipeline_execution_id: Some(pipeline_execution_id),
            scheduled_time,
            claimed_at: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Creates a new cron execution record with a specific claimed_at time.
    pub fn with_claimed_at(
        schedule_id: UniversalUuid,
        pipeline_execution_id: Option<UniversalUuid>,
        scheduled_time: UniversalTimestamp,
        claimed_at: DateTime<Utc>,
    ) -> Self {
        let claimed_ts = UniversalTimestamp(claimed_at);
        Self {
            id: Some(UniversalUuid::new_v4()),
            schedule_id,
            pipeline_execution_id,
            scheduled_time,
            claimed_at: Some(claimed_ts),
            created_at: Some(claimed_ts),
            updated_at: Some(claimed_ts),
        }
    }
}

impl CronExecution {
    pub fn scheduled_time(&self) -> DateTime<Utc> {
        self.scheduled_time.0
    }

    pub fn claimed_at(&self) -> DateTime<Utc> {
        self.claimed_at.0
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at.0
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at.0
    }

    pub fn execution_delay(&self) -> chrono::Duration {
        self.claimed_at.0 - self.scheduled_time.0
    }

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
        assert!(new_execution.claimed_at.is_none());
    }

    #[test]
    fn test_cron_execution_delays() {
        let now = Utc::now();
        let scheduled_time = UniversalTimestamp(now - Duration::minutes(1));
        let claimed_at = UniversalTimestamp(now);

        let execution = CronExecution {
            id: UniversalUuid::new_v4(),
            schedule_id: UniversalUuid::new_v4(),
            pipeline_execution_id: Some(UniversalUuid::new_v4()),
            scheduled_time,
            claimed_at,
            created_at: claimed_at,
            updated_at: claimed_at,
        };

        let delay = execution.execution_delay();
        assert_eq!(delay, Duration::minutes(1));
        assert!(execution.is_timely(Duration::minutes(2)));
        assert!(!execution.is_timely(Duration::seconds(30)));
    }
}
