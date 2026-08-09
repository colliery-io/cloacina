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

//! The packaged counterpart of the engine's `TaskHandle` (CLOACI-T-0897).
//!
//! A task that takes a handle parameter gets *this* type inside a packaged
//! cdylib, and the engine's own `cloacina::TaskHandle` when built embedded. The
//! surface is the same — `defer_until`, `task_execution_id` — so a workflow's
//! source compiles either way; only the plumbing underneath differs.
//!
//! Embedded, `defer_until` reaches straight into the executor's semaphore and
//! DAL. Packaged, it cannot: those live in the host, on the other side of an
//! FFI boundary, and the engine crate is not even linked. Instead each host
//! operation goes out over the [`CloacinaHost`](crate::CloacinaHost) callback
//! channel, while the user's condition keeps running here in the plugin. The
//! observable behavior matches: the concurrency slot really is released for the
//! duration of the wait.
//!
//! The task-execution id is supplied by the host in the execution request; the
//! plugin shell stashes it for the duration of one invocation so the handle can
//! name which task is calling.

use std::cell::RefCell;
use std::time::Duration;

use cloacina_workflow::TaskError;

use crate::CloacinaHostClient;

thread_local! {
    /// The task-execution id for the invocation running on this thread.
    ///
    /// A thread-local rather than a parameter because the handle is
    /// constructed by macro-generated code that has no other way to see it, and
    /// each `execute_task` invocation is serviced on one thread.
    static CURRENT_TASK_EXECUTION_ID: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Install the current invocation's task-execution id, returning a guard that
/// clears it. Called by the plugin shell around `Task::execute`.
pub struct TaskExecutionIdGuard;

impl TaskExecutionIdGuard {
    /// Set the id for this thread for the lifetime of the guard.
    pub fn set(task_execution_id: impl Into<String>) -> Self {
        CURRENT_TASK_EXECUTION_ID.with(|c| *c.borrow_mut() = Some(task_execution_id.into()));
        Self
    }
}

impl Drop for TaskExecutionIdGuard {
    fn drop(&mut self) {
        CURRENT_TASK_EXECUTION_ID.with(|c| *c.borrow_mut() = None);
    }
}

/// The current invocation's task-execution id, if the shell installed one.
fn current_task_execution_id() -> Option<String> {
    CURRENT_TASK_EXECUTION_ID.with(|c| c.borrow().clone())
}

/// Execution-control handle for a task inside a packaged workflow.
///
/// Mirrors the engine's `TaskHandle` surface. Obtained by macro-generated code,
/// not constructed directly.
#[derive(Debug)]
pub struct TaskHandle {
    task_execution_id: String,
}

impl TaskHandle {
    /// Build a handle for the invocation running on this thread.
    ///
    /// Fails when the shell did not install an id — which would mean this
    /// package is being driven by a host too old to send one. Better a clear
    /// error at the first `defer_until` than a silent no-op wait.
    pub fn for_current_invocation() -> Result<Self, TaskError> {
        current_task_execution_id()
            .map(|task_execution_id| Self { task_execution_id })
            .ok_or_else(|| TaskError::ExecutionFailed {
                message: "task handle unavailable: the host did not supply a \
                          task-execution id for this invocation (host too old \
                          for CLOACI-T-0897?)"
                    .to_string(),
                task_id: String::new(),
                timestamp: cloacina_workflow::__private::chrono::Utc::now(),
            })
    }

    /// The task-execution id this handle acts on.
    pub fn task_execution_id(&self) -> &str {
        &self.task_execution_id
    }

    /// Release the concurrency slot while polling an external condition, then
    /// reclaim it — the packaged implementation of the engine's `defer_until`.
    ///
    /// The slot is genuinely released for the whole wait: another task can run
    /// in it. The condition runs here, in the plugin, so it can be any code the
    /// task author likes.
    ///
    /// # Errors
    ///
    /// Surfaces host-side failures as task errors — notably when the executor
    /// is shutting down and the slot can never be reclaimed, so the task fails
    /// instead of waiting forever.
    pub async fn defer_until<F, Fut>(
        &mut self,
        condition: F,
        poll_interval: Duration,
    ) -> Result<(), TaskError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let host = CloacinaHostClient::bound().map_err(|e| self.err("host unavailable", e))?;

        // Mark deferred BEFORE releasing, so an operator watching the row never
        // sees a task holding no slot while still reported Active.
        host.set_sub_status(&self.task_execution_id, &"Deferred".to_string())
            .map_err(|e| self.err("set sub_status Deferred", e))?;
        host.release_slot(&self.task_execution_id)
            .map_err(|e| self.err("release slot", e))?;

        // Poll here in the plugin — this is the part that cannot cross the
        // boundary, and the reason a callback channel was needed at all.
        loop {
            cloacina_workflow::__private::tokio::time::sleep(poll_interval).await;
            if condition().await {
                break;
            }
        }

        // Reclaim before resuming real work. Best-effort restore of Active
        // afterwards: if the status write fails the task is still correctly
        // running with a slot, so failing it here would be worse than a stale
        // sub_status.
        host.reclaim_slot(&self.task_execution_id)
            .map_err(|e| self.err("reclaim slot", e))?;
        let _ = host.set_sub_status(&self.task_execution_id, &"Active".to_string());

        Ok(())
    }

    fn err(&self, what: &str, e: impl std::fmt::Display) -> TaskError {
        TaskError::ExecutionFailed {
            message: format!("defer_until: {what} failed: {e}"),
            task_id: self.task_execution_id.clone(),
            timestamp: cloacina_workflow::__private::chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_installs_and_clears_the_id() {
        assert!(current_task_execution_id().is_none());
        {
            let _g = TaskExecutionIdGuard::set("abc");
            assert_eq!(current_task_execution_id().as_deref(), Some("abc"));
            assert_eq!(
                TaskHandle::for_current_invocation()
                    .unwrap()
                    .task_execution_id(),
                "abc"
            );
        }
        assert!(
            current_task_execution_id().is_none(),
            "guard must clear on drop so one invocation cannot leak into the next"
        );
    }

    /// Without an installed id the handle refuses to exist, rather than
    /// deferring against an empty task id and silently doing nothing.
    #[test]
    fn no_id_is_a_clear_error() {
        assert!(current_task_execution_id().is_none());
        let err = TaskHandle::for_current_invocation().expect_err("must fail");
        assert!(
            format!("{err}").contains("task handle unavailable"),
            "unexpected error: {err}"
        );
    }
}
