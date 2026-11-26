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

//! Run-finished notices for the server's alert webhook (CLOACINA-T-1024).
//!
//! When a run reaches Failed, Completed or Cancelled, the DAL can write one
//! `delivery_outbox` row in the same transaction as the status change. The row
//! is addressed to [`RECIPIENT`] and holds a [`RunFinishedNotice`]. A sender
//! in `cloacina-server` claims the row, sends the webhook and acks the row.
//! This crate sends nothing.
//!
//! The rows are off by default. A server with an alert webhook calls
//! [`enable`] at start-up with the outcomes it sends. With no call, no row is
//! written and nothing else changes.

use std::sync::atomic::{AtomicU8, Ordering};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::dal::DAL;
use crate::dal::unified::models::NewUnifiedDeliveryOutbox;
use crate::database::schema::unified::{schedule_executions, schedules};
use crate::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;

/// The outbox recipient of run-finished notices.
pub const RECIPIENT: &str = "alert_webhook";

/// The outbox kind of run-finished notices.
pub const KIND: &str = "workflow_execution_finished";

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunOutcome {
    Failed,
    Completed,
    Cancelled,
}

impl RunOutcome {
    /// All outcomes.
    pub const ALL: [RunOutcome; 3] = [
        RunOutcome::Failed,
        RunOutcome::Completed,
        RunOutcome::Cancelled,
    ];

    /// The lower-case name: `failed`, `completed`, `cancelled`.
    pub fn as_str(&self) -> &'static str {
        match self {
            RunOutcome::Failed => "failed",
            RunOutcome::Completed => "completed",
            RunOutcome::Cancelled => "cancelled",
        }
    }

    /// Reads a lower-case name.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "failed" => Some(RunOutcome::Failed),
            "completed" => Some(RunOutcome::Completed),
            "cancelled" => Some(RunOutcome::Cancelled),
            _ => None,
        }
    }

    const fn bit(self) -> u8 {
        match self {
            RunOutcome::Failed => 1,
            RunOutcome::Completed => 2,
            RunOutcome::Cancelled => 4,
        }
    }
}

/// The outcomes that write a notice, as a bit mask. 0 = none.
static ENABLED: AtomicU8 = AtomicU8::new(0);

fn mask_of(outcomes: &[RunOutcome]) -> u8 {
    outcomes.iter().fold(0, |mask, o| mask | o.bit())
}

/// Switches on the notices for these outcomes, for the whole process. An empty
/// list switches them off.
pub fn enable(outcomes: &[RunOutcome]) {
    ENABLED.store(mask_of(outcomes), Ordering::Relaxed);
}

/// Whether a run that ends with `outcome` writes a notice.
pub fn is_enabled(outcome: RunOutcome) -> bool {
    ENABLED.load(Ordering::Relaxed) & outcome.bit() != 0
}

/// The payload of a notice row: which run ended and how. The sender reads the
/// rest from the run's own rows when it sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunFinishedNotice {
    pub execution_id: uuid::Uuid,
    /// `failed`, `completed` or `cancelled`.
    pub outcome: String,
}

impl RunFinishedNotice {
    pub fn new(execution_id: uuid::Uuid, outcome: RunOutcome) -> Self {
        Self {
            execution_id,
            outcome: outcome.as_str().to_string(),
        }
    }

    /// The JSON bytes stored in the outbox row.
    pub fn to_bytes(&self) -> Vec<u8> {
        // Two plain fields: serialization cannot fail.
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Reads the payload of an outbox row. `None` when it is not a notice.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }

    /// The outcome, when the name is known.
    pub fn run_outcome(&self) -> Option<RunOutcome> {
        RunOutcome::parse(&self.outcome)
    }
}

/// The outbox row for a run that just ended. The DAL inserts it in the
/// transaction that changes the status.
pub(crate) fn outbox_row(
    execution_id: UniversalUuid,
    outcome: RunOutcome,
    tenant: Option<String>,
    now: UniversalTimestamp,
) -> NewUnifiedDeliveryOutbox {
    NewUnifiedDeliveryOutbox {
        recipient: RECIPIENT.to_string(),
        kind: KIND.to_string(),
        tenant_id: tenant,
        payload: UniversalBinary::from(RunFinishedNotice::new(execution_id.0, outcome).to_bytes()),
        delivery_state: "pending".to_string(),
        delivery_attempts: 0,
        created_at: now,
    }
}

/// The schedule that started a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSchedule {
    /// `cron` or `trigger`.
    pub schedule_type: String,
    /// The time the cron schedule asked for. `None` for a trigger.
    pub scheduled_time: Option<DateTime<Utc>>,
}

/// The schedule that started this run, or `None` when no schedule did (a
/// manual run, or a run started from code).
pub async fn schedule_of_run(
    dal: &DAL,
    execution_id: UniversalUuid,
) -> Result<Option<RunSchedule>, ValidationError> {
    let found: Option<(String, Option<UniversalTimestamp>)> =
        crate::interact_on_backend!(dal, |conn| {
            let link: Option<(UniversalUuid, Option<UniversalTimestamp>)> =
                schedule_executions::table
                    .filter(schedule_executions::workflow_execution_id.eq(Some(execution_id)))
                    .order(schedule_executions::created_at.desc())
                    .select((
                        schedule_executions::schedule_id,
                        schedule_executions::scheduled_time,
                    ))
                    .first(conn)
                    .optional()?;
            let Some((schedule_id, scheduled_time)) = link else {
                return Ok(None);
            };
            let schedule_type: Option<String> = schedules::table
                .find(schedule_id)
                .select(schedules::schedule_type)
                .first(conn)
                .optional()?;
            Ok::<_, diesel::result::Error>(schedule_type.map(|t| (t, scheduled_time)))
        })?;
    Ok(found.map(|(schedule_type, scheduled_time)| RunSchedule {
        schedule_type,
        scheduled_time: scheduled_time.map(|t| t.0),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_alert_outcome_names_round_trip() {
        for outcome in RunOutcome::ALL {
            assert_eq!(RunOutcome::parse(outcome.as_str()), Some(outcome));
        }
        assert_eq!(RunOutcome::parse("paused"), None);
    }

    #[test]
    fn run_alert_mask_holds_only_the_given_outcomes() {
        assert_eq!(mask_of(&[]), 0);
        let mask = mask_of(&[RunOutcome::Failed, RunOutcome::Cancelled]);
        assert_ne!(mask & RunOutcome::Failed.bit(), 0);
        assert_eq!(mask & RunOutcome::Completed.bit(), 0);
        assert_ne!(mask & RunOutcome::Cancelled.bit(), 0);
    }

    #[test]
    fn run_alert_notice_round_trips_and_rejects_other_payloads() {
        let id = uuid::Uuid::new_v4();
        let notice = RunFinishedNotice::new(id, RunOutcome::Failed);
        let back = RunFinishedNotice::from_bytes(&notice.to_bytes()).unwrap();
        assert_eq!(back, notice);
        assert_eq!(back.run_outcome(), Some(RunOutcome::Failed));
        assert!(RunFinishedNotice::from_bytes(b"not json").is_none());
    }

    /// The relay and the sweeper leave notice rows alone: only the alert
    /// sender handles them.
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn run_alert_rows_are_not_seen_by_the_relay_or_the_sweeper() {
        use crate::models::delivery_outbox::NewDeliveryOutbox;

        let url = format!(
            "file:run_alert_relay_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = crate::database::Database::new(&url, "", 5);
        db.run_migrations()
            .await
            .expect("migrations should succeed");
        let dal = DAL::new(db);
        let outbox = dal.delivery_outbox();
        let enqueue = |recipient: &str| NewDeliveryOutbox {
            recipient: recipient.to_string(),
            kind: KIND.to_string(),
            tenant_id: None,
            payload: b"{}".to_vec(),
        };
        let pending_notice = outbox.enqueue(enqueue(RECIPIENT)).await.unwrap();
        let claimed_notice = outbox.enqueue(enqueue(RECIPIENT)).await.unwrap();
        outbox.mark_delivered(claimed_notice.id).await.unwrap();
        let other = outbox.enqueue(enqueue("agent:some-agent")).await.unwrap();

        let pending: Vec<i64> = outbox
            .list_pending(10)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(pending, vec![other.id]);

        let future = UniversalTimestamp(Utc::now() + chrono::Duration::hours(1));
        let stuck: Vec<i64> = outbox
            .list_stuck(future, 10)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(stuck, vec![other.id]);

        // The alert sender still finds both by recipient.
        let open: Vec<i64> = outbox
            .list_open_for_recipient(RECIPIENT, 10)
            .await
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(open, vec![pending_notice.id, claimed_notice.id]);
    }
}
