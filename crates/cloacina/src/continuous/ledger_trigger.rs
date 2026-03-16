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

//! LedgerTrigger — watches the ExecutionLedger for task completions
//! and fires detector workflows for derived data sources.
//!
//! This completes the reactive feedback loop: task completes →
//! LedgerTrigger fires → detector runs → downstream boundaries flow.
//!
//! See CLOACI-S-0007 for the full specification.

use super::ledger::{ExecutionLedger, LedgerEvent};
use crate::trigger::{Trigger, TriggerError, TriggerResult};
use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

/// How to match watched task completions.
#[derive(Debug, Clone, PartialEq)]
pub enum LedgerMatchMode {
    /// Fire when any watched task completes.
    Any,
    /// Fire when all watched tasks have completed since last fire.
    All,
}

/// A trigger that watches the `ExecutionLedger` for task completions.
///
/// Used for derived data sources: when an upstream task completes,
/// the LedgerTrigger fires the associated detector workflow.
#[derive(Debug)]
pub struct LedgerTrigger {
    /// Trigger name (usually matches the detector workflow name).
    trigger_name: String,
    /// Task IDs to watch for completion.
    watch_tasks: Vec<String>,
    /// Fire when any or all watched tasks complete.
    match_mode: LedgerMatchMode,
    /// Reference to the shared execution ledger.
    ledger: Arc<RwLock<ExecutionLedger>>,
    /// Last observed ledger position.
    cursor: Mutex<usize>,
    /// For All mode: tracks which watched tasks have completed since last fire.
    seen_completions: Mutex<HashSet<String>>,
    /// Event-driven notification from the ledger. When present, `poll()` is
    /// only a fallback — the trigger framework should `await` the notify
    /// between polls to avoid unnecessary wake-ups.
    notify: Option<Arc<Notify>>,
}

impl LedgerTrigger {
    /// Create a new LedgerTrigger.
    pub fn new(
        trigger_name: String,
        watch_tasks: Vec<String>,
        match_mode: LedgerMatchMode,
        ledger: Arc<RwLock<ExecutionLedger>>,
    ) -> Self {
        // Subscribe to ledger notifications
        let notify = {
            let l = ledger.read();
            Some(l.subscribe())
        };
        Self {
            trigger_name,
            watch_tasks,
            match_mode,
            ledger,
            cursor: Mutex::new(0),
            seen_completions: Mutex::new(HashSet::new()),
            notify,
        }
    }

    /// Get the notification handle for event-driven wake-up.
    /// Callers can `await` on `notified()` instead of relying on `poll_interval()`.
    pub fn notify_handle(&self) -> Option<&Arc<Notify>> {
        self.notify.as_ref()
    }
}

#[async_trait]
impl Trigger for LedgerTrigger {
    fn name(&self) -> &str {
        &self.trigger_name
    }

    fn poll_interval(&self) -> Duration {
        // Fallback interval — primary wake-up is via Notify from ledger.append().
        // This is a safety net in case a notification is missed.
        Duration::from_secs(5)
    }

    fn allow_concurrent(&self) -> bool {
        false
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let ledger = self.ledger.read();

        let mut cursor = self.cursor.lock();
        let new_events = ledger.events_since(*cursor);

        if new_events.is_empty() {
            return Ok(TriggerResult::Skip);
        }

        // Scan for matching TaskCompleted events
        let mut matched_context = None;
        let mut any_matched = false;

        for event in new_events {
            if let LedgerEvent::TaskCompleted { task, context, .. } = event {
                if self.watch_tasks.iter().any(|w| w.as_str() == task.as_str()) {
                    any_matched = true;
                    matched_context = Some(context);

                    if self.match_mode == LedgerMatchMode::All {
                        let mut seen = self.seen_completions.lock();
                        seen.insert(task.clone());
                    }
                }
            }
        }

        // Advance cursor past all events we've seen
        *cursor = ledger.len();

        match self.match_mode {
            LedgerMatchMode::Any => {
                if any_matched {
                    // Clone the context data for the fire result
                    let fire_ctx = matched_context.map(|ctx| ctx.clone_data());
                    Ok(TriggerResult::Fire(fire_ctx))
                } else {
                    Ok(TriggerResult::Skip)
                }
            }
            LedgerMatchMode::All => {
                let mut seen = self.seen_completions.lock();
                let all_seen = self.watch_tasks.iter().all(|w| seen.contains(w));

                if all_seen {
                    seen.clear(); // Reset for next cycle
                    let fire_ctx = matched_context.map(|ctx| ctx.clone_data());
                    Ok(TriggerResult::Fire(fire_ctx))
                } else {
                    Ok(TriggerResult::Skip)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use cloacina_workflow::Context;

    fn make_ledger_with_completions(tasks: &[&str]) -> Arc<RwLock<ExecutionLedger>> {
        let mut ledger = ExecutionLedger::new();
        for task in tasks {
            ledger.append(LedgerEvent::TaskCompleted {
                task: task.to_string(),
                at: Utc::now(),
                context: Context::new(),
            });
        }
        Arc::new(RwLock::new(ledger))
    }

    #[tokio::test]
    async fn test_any_mode_fires_on_single_match() {
        let ledger = make_ledger_with_completions(&["task_a"]);
        let trigger = LedgerTrigger::new(
            "detect_derived".into(),
            vec!["task_a".into(), "task_b".into()],
            LedgerMatchMode::Any,
            ledger,
        );

        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());
    }

    #[tokio::test]
    async fn test_any_mode_skips_on_no_match() {
        let ledger = make_ledger_with_completions(&["unrelated_task"]);
        let trigger = LedgerTrigger::new(
            "detect_derived".into(),
            vec!["task_a".into()],
            LedgerMatchMode::Any,
            ledger,
        );

        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
    }

    #[tokio::test]
    async fn test_all_mode_waits_for_all() {
        let ledger = make_ledger_with_completions(&["task_a"]);
        let trigger = LedgerTrigger::new(
            "detect_derived".into(),
            vec!["task_a".into(), "task_b".into()],
            LedgerMatchMode::All,
            ledger.clone(),
        );

        // Only task_a completed — should not fire
        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());

        // Add task_b completion
        {
            let mut l = ledger.write();
            l.append(LedgerEvent::TaskCompleted {
                task: "task_b".into(),
                at: Utc::now(),
                context: Context::new(),
            });
        }

        // Now both completed — should fire
        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());
    }

    #[tokio::test]
    async fn test_all_mode_resets_after_fire() {
        let ledger = make_ledger_with_completions(&["task_a", "task_b"]);
        let trigger = LedgerTrigger::new(
            "detect".into(),
            vec!["task_a".into(), "task_b".into()],
            LedgerMatchMode::All,
            ledger.clone(),
        );

        // First poll fires
        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());

        // Second poll with no new events — should skip
        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
    }

    #[tokio::test]
    async fn test_cursor_idempotency() {
        let ledger = make_ledger_with_completions(&["task_a"]);
        let trigger = LedgerTrigger::new(
            "detect".into(),
            vec!["task_a".into()],
            LedgerMatchMode::Any,
            ledger.clone(),
        );

        // First poll fires
        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());

        // Second poll — cursor advanced, no new events
        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());

        // Add new completion
        {
            let mut l = ledger.write();
            l.append(LedgerEvent::TaskCompleted {
                task: "task_a".into(),
                at: Utc::now(),
                context: Context::new(),
            });
        }

        // Third poll sees only the new event
        let result = trigger.poll().await.unwrap();
        assert!(result.should_fire());
    }

    #[tokio::test]
    async fn test_empty_ledger_skips() {
        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
        let trigger = LedgerTrigger::new(
            "detect".into(),
            vec!["task_a".into()],
            LedgerMatchMode::Any,
            ledger,
        );

        let result = trigger.poll().await.unwrap();
        assert!(!result.should_fire());
    }

    #[test]
    fn test_trigger_metadata() {
        let ledger = Arc::new(RwLock::new(ExecutionLedger::new()));
        let trigger = LedgerTrigger::new(
            "my_trigger".into(),
            vec!["task_a".into()],
            LedgerMatchMode::Any,
            ledger,
        );

        assert_eq!(trigger.name(), "my_trigger");
        assert_eq!(trigger.poll_interval(), Duration::from_secs(5));
        assert!(!trigger.allow_concurrent());
    }
}
