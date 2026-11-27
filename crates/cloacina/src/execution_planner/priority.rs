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

//! Priority of a run and the order of waiting work (CLOACINA-T-1045).
//!
//! The scale and its rules are in [`cloacina_api_types::priority`]. This
//! module has what the planner and the scheduler loop need: how a run gets
//! its priority, who may limit it, and the order of the waiting tasks.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cloacina_api_types::{
    DEFAULT_PRIORITY, PriorityGrant, aged_priority, aging_steps, clamp_priority,
};

use crate::models::task_execution::TaskExecution;

/// The priority a run asks for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunPriority {
    /// The workflow priority the run asks for. `None`: the run asks for
    /// nothing, and the priority the workflow declares applies (5 when it
    /// declares none).
    pub asked: Option<u8>,
    /// The highest priority that who asked may set. `None`: no limit (a run
    /// started by the program itself, with no principal).
    pub limit: Option<u8>,
}

impl RunPriority {
    /// A run that asks for `asked`, by a principal that may set up to
    /// `limit`.
    pub fn asked_by(asked: u8, limit: Option<u8>) -> Self {
        Self {
            asked: Some(asked),
            limit,
        }
    }
}

/// Who may limit the priority that the code of a package declares.
///
/// A server sets one on its runners: the limit is that of the principal that
/// published the package, as it is when the run starts. With none set (the
/// embedded use, where the program is its own author) a declared priority is
/// given as declared.
#[async_trait]
pub trait PriorityAuthority: Send + Sync {
    /// The highest priority the package of `workflow_name` may set now.
    /// `None` is no limit.
    async fn declared_limit(&self, workflow_name: &str) -> Option<u8>;

    /// A run of `workflow_name` started with a priority below the one asked
    /// for. Called for each such run.
    async fn run_held(&self, _workflow_name: &str, _grant: PriorityGrant) {}
}

/// `asked` under `limit`, where `None` is no limit.
pub(crate) fn grant_under(asked: u8, limit: Option<u8>) -> PriorityGrant {
    match limit {
        Some(limit) => PriorityGrant::held_to(asked, Some(limit)),
        None => PriorityGrant::unlimited(asked),
    }
}

/// The workflow priority of a run: what the run asks for under the limit of
/// who asked; else what the workflow declares under the limit of its package;
/// else the default.
pub(crate) fn workflow_grant(
    run: RunPriority,
    declared: Option<u8>,
    declared_limit: Option<u8>,
) -> PriorityGrant {
    match run.asked {
        Some(asked) => grant_under(asked, run.limit),
        None => grant_under(declared.unwrap_or(DEFAULT_PRIORITY), declared_limit),
    }
}

/// The priority of one task of a run: what the task declares under the limit
/// of its package; a task that declares none takes its workflow's.
pub(crate) fn task_priority(
    declared: Option<u8>,
    declared_limit: Option<u8>,
    workflow_given: u8,
) -> u8 {
    match declared {
        Some(declared) => grant_under(declared, declared_limit).given,
        None => workflow_given,
    }
}

/// The place of a waiting task in the order: the aged priority of its
/// workflow, its own aged priority, then the time from which it waits. A
/// smaller key goes first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WaitingOrder {
    pub workflow_priority: u8,
    pub task_priority: u8,
    pub waiting_since: DateTime<Utc>,
}

impl WaitingOrder {
    /// The order of `task` at `now`. For each ageing interval the task has
    /// waited, both priorities improve by one, never below 0.
    pub fn of(task: &TaskExecution, now: DateTime<Utc>, aging_interval: Duration) -> Self {
        // A row that never became Ready has no waiting time: it waits from
        // its last write.
        let waiting_since = task
            .waiting_since
            .map(|at| at.0)
            .unwrap_or(task.updated_at.0);
        let waited = (now - waiting_since).to_std().unwrap_or(Duration::ZERO);
        let steps = aging_steps(waited, aging_interval);
        Self {
            workflow_priority: aged_priority(clamp_priority(task.workflow_priority), steps),
            task_priority: aged_priority(clamp_priority(task.task_priority), steps),
            waiting_since,
        }
    }

    /// The two priorities, without the waiting time.
    pub fn class(&self) -> (u8, u8) {
        (self.workflow_priority, self.task_priority)
    }
}

/// The pair of priorities of a task as stored on its row, without ageing.
fn stored_class(task: &TaskExecution) -> (u8, u8) {
    (
        clamp_priority(task.workflow_priority),
        clamp_priority(task.task_priority),
    )
}

/// The waiting tasks to dispatch in this pass, in order.
///
/// `waiting` is every task that waits: Ready, unclaimed, and not in the
/// backoff of a retry. Some of them are put off for capacity (their
/// `retry_at` is in the future): they are not dispatched now, but they keep
/// their place. A due task is left out of this pass while a task of its pool
/// that was given a strictly better pair of priorities is put off, and is
/// still ahead of it after ageing: the capacity that frees next is for the
/// better task. Tasks that were given the same pair never hold each other
/// back, however long one of them has waited: work that sets no priority
/// packs as it did before priorities.
pub fn dispatch_order(
    waiting: Vec<TaskExecution>,
    now: DateTime<Utc>,
    aging_interval: Duration,
) -> Vec<TaskExecution> {
    let mut keyed: Vec<(WaitingOrder, bool, TaskExecution)> = waiting
        .into_iter()
        .map(|task| {
            let due = task.retry_at.is_none_or(|at| at.0 <= now);
            (WaitingOrder::of(&task, now, aging_interval), due, task)
        })
        .collect();

    // The tasks put off, for each pool: the pair as stored and the pair as
    // aged.
    let mut put_off: HashMap<String, Vec<((u8, u8), (u8, u8))>> = HashMap::new();
    for (order, due, task) in &keyed {
        if !*due {
            put_off
                .entry(task.pool.clone().unwrap_or_default())
                .or_default()
                .push((stored_class(task), order.class()));
        }
    }

    keyed.retain(|(order, due, task)| {
        *due && !put_off
            .get(task.pool.as_deref().unwrap_or_default())
            .is_some_and(|better| {
                better
                    .iter()
                    .any(|(stored, aged)| *stored < stored_class(task) && *aged < order.class())
            })
    });
    // Stable: tasks with the same key keep the order the query gave.
    keyed.sort_by_key(|(order, _, _)| *order);
    keyed.into_iter().map(|(_, _, task)| task).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};

    const TWO_MINUTES: Duration = Duration::from_secs(120);

    fn task(
        name: &str,
        workflow: i32,
        own: i32,
        waited_secs: i64,
        now: DateTime<Utc>,
    ) -> TaskExecution {
        let since = UniversalTimestamp(now - chrono::Duration::seconds(waited_secs));
        TaskExecution {
            id: UniversalUuid::new_v4(),
            workflow_execution_id: UniversalUuid::new_v4(),
            task_name: name.to_string(),
            status: "Ready".to_string(),
            started_at: None,
            completed_at: None,
            attempt: 1,
            max_attempts: 3,
            error_details: None,
            trigger_rules: "{}".to_string(),
            task_configuration: "{}".to_string(),
            retry_at: None,
            last_error: None,
            recovery_attempts: 0,
            last_recovery_at: None,
            sub_status: None,
            claimed_by: None,
            heartbeat_at: None,
            waiting_since: Some(since),
            artifact_fetch_wait_since: None,
            pool: None,
            cpu_request_millis: None,
            memory_request_bytes: None,
            fleet_agent_id: None,
            fleet_dispatch_id: None,
            fleet_packet_row_id: None,
            created_at: since,
            updated_at: since,
            workflow_priority: workflow,
            task_priority: own,
            fleet_dispatch_claimed_by: None,
        }
    }

    fn put_off(mut task: TaskExecution, now: DateTime<Utc>) -> TaskExecution {
        task.retry_at = Some(UniversalTimestamp(now + chrono::Duration::seconds(2)));
        task
    }

    fn names(tasks: &[TaskExecution]) -> Vec<&str> {
        tasks.iter().map(|t| t.task_name.as_str()).collect()
    }

    #[test]
    fn a_run_that_asks_for_nothing_gets_what_the_workflow_declares() {
        let run = RunPriority::default();
        assert_eq!(workflow_grant(run, None, None).given, 5);
        assert_eq!(workflow_grant(run, Some(2), None).given, 2);
        // The package's publisher may set 5 at most.
        let held = workflow_grant(run, Some(2), Some(5));
        assert_eq!((held.asked, held.given), (2, 5));
    }

    #[test]
    fn a_run_that_asks_is_held_to_the_limit_of_who_asked_not_of_the_package() {
        let run = RunPriority::asked_by(1, Some(5));
        let grant = workflow_grant(run, Some(8), Some(0));
        assert_eq!((grant.asked, grant.given), (1, 5));
        let admin = RunPriority::asked_by(1, Some(0));
        assert_eq!(workflow_grant(admin, Some(8), Some(5)).given, 1);
    }

    #[test]
    fn a_task_with_no_priority_takes_its_workflows() {
        assert_eq!(task_priority(None, None, 3), 3);
        assert_eq!(task_priority(Some(1), None, 3), 1);
        assert_eq!(task_priority(Some(1), Some(5), 3), 5);
        assert_eq!(task_priority(Some(9), Some(5), 3), 9);
    }

    #[test]
    fn the_order_is_workflow_priority_then_task_priority_then_time_waited() {
        let now = Utc::now();
        let waiting = vec![
            task("low_workflow", 8, 0, 60, now),
            task("default_new", 5, 5, 10, now),
            task("default_old", 5, 5, 50, now),
            task("default_important_task", 5, 1, 5, now),
            task("high_workflow", 2, 9, 1, now),
        ];
        let order = dispatch_order(waiting, now, TWO_MINUTES);
        assert_eq!(
            names(&order),
            vec![
                "high_workflow",
                "default_important_task",
                "default_old",
                "default_new",
                "low_workflow"
            ]
        );
    }

    #[test]
    fn the_default_priority_keeps_oldest_first() {
        let now = Utc::now();
        let waiting = vec![
            task("third", 5, 5, 1, now),
            task("first", 5, 5, 90, now),
            task("second", 5, 5, 30, now),
        ];
        let order = dispatch_order(waiting, now, TWO_MINUTES);
        assert_eq!(names(&order), vec!["first", "second", "third"]);
    }

    #[test]
    fn ageing_lifts_old_low_priority_work_past_newer_higher_priority_work() {
        let now = Utc::now();
        // Priority 10 against a fresh priority 5: five intervals close the
        // gap, and the older task then wins on time waited.
        let before = dispatch_order(
            vec![
                task("low", 10, 10, 9 * 60 + 59, now),
                task("fresh", 5, 5, 0, now),
            ],
            now,
            TWO_MINUTES,
        );
        assert_eq!(names(&before), vec!["fresh", "low"]);
        let after = dispatch_order(
            vec![
                task("low", 10, 10, 10 * 60, now),
                task("fresh", 5, 5, 0, now),
            ],
            now,
            TWO_MINUTES,
        );
        assert_eq!(names(&after), vec!["low", "fresh"]);
        // The stored numbers do not change.
        assert_eq!(after[0].workflow_priority, 10);
    }

    #[test]
    fn the_lowest_priority_reaches_the_top_after_ten_intervals() {
        let now = Utc::now();
        let low = task("low", 10, 10, 20 * 60, now);
        let order = WaitingOrder::of(&low, now, TWO_MINUTES);
        assert_eq!(order.class(), (0, 0));
        let top = task("top", 0, 0, 0, now);
        let order = dispatch_order(vec![top, low], now, TWO_MINUTES);
        assert_eq!(names(&order), vec!["low", "top"]);
    }

    #[test]
    fn with_ageing_off_low_priority_work_stays_behind() {
        let now = Utc::now();
        let order = dispatch_order(
            vec![task("low", 10, 10, 3600, now), task("fresh", 5, 5, 0, now)],
            now,
            Duration::ZERO,
        );
        assert_eq!(names(&order), vec!["fresh", "low"]);
    }

    #[test]
    fn a_due_task_waits_while_a_better_task_of_its_pool_is_put_off_for_capacity() {
        let now = Utc::now();
        let important = put_off(task("important", 1, 1, 30, now), now);
        let order = dispatch_order(
            vec![important.clone(), task("ordinary", 5, 5, 300, now)],
            now,
            Duration::ZERO,
        );
        assert!(order.is_empty(), "{:?}", names(&order));

        // When the better task is due again, it goes first.
        let mut due = important;
        due.retry_at = None;
        let order = dispatch_order(
            vec![task("ordinary", 5, 5, 300, now), due],
            now,
            Duration::ZERO,
        );
        assert_eq!(names(&order), vec!["important", "ordinary"]);
    }

    #[test]
    fn tasks_of_the_same_priorities_do_not_hold_each_other_back() {
        let now = Utc::now();
        // The big task has waited an hour for room. It is far ahead after
        // ageing, but it was given the same priorities as the small one.
        let big = put_off(task("big", 5, 5, 3600, now), now);
        let order = dispatch_order(vec![big, task("small", 5, 5, 10, now)], now, TWO_MINUTES);
        assert_eq!(names(&order), vec!["small"]);
    }

    #[test]
    fn a_worse_task_is_held_back_only_until_it_has_aged_level_with_the_better_one() {
        let now = Utc::now();
        // The important task is at (1, 1) and cannot be placed. An ordinary
        // task at (5, 5) is level with it after four intervals.
        let important = put_off(task("important", 1, 1, 0, now), now);
        let held = dispatch_order(
            vec![important.clone(), task("ordinary", 5, 5, 7 * 60, now)],
            now,
            TWO_MINUTES,
        );
        assert!(held.is_empty(), "{:?}", names(&held));
        let level = dispatch_order(
            vec![important, task("ordinary", 5, 5, 8 * 60, now)],
            now,
            TWO_MINUTES,
        );
        assert_eq!(names(&level), vec!["ordinary"]);
    }

    #[test]
    fn a_task_put_off_in_one_pool_does_not_hold_back_another_pool() {
        let now = Utc::now();
        let mut important = put_off(task("important", 1, 1, 30, now), now);
        important.pool = Some("large".to_string());
        let order = dispatch_order(
            vec![important, task("ordinary", 5, 5, 10, now)],
            now,
            TWO_MINUTES,
        );
        assert_eq!(names(&order), vec!["ordinary"]);
    }
}
