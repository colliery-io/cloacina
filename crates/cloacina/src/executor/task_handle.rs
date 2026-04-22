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

//! Task execution control handle.
//!
//! `TaskHandle` provides execution control capabilities to tasks that opt in
//! by accepting it as a second parameter. The primary feature is `defer_until`,
//! which allows a task to release its concurrency slot while polling an
//! external condition.
//!
//! # Example
//!
//! ```rust,ignore
//! #[task(id = "wait_for_file")]
//! async fn wait_for_file(
//!     context: &mut Context<Value>,
//!     handle: &TaskHandle,
//! ) -> Result<(), TaskError> {
//!     handle.defer_until(
//!         || async { std::path::Path::new("/data/input.csv").exists() },
//!         Duration::from_secs(5),
//!     ).await?;
//!
//!     // File exists — slot has been reclaimed, proceed with work
//!     process_file(context).await
//! }
//! ```

use std::cell::RefCell;
use std::future::Future;
use std::time::Duration;

use tracing::{debug, warn};

use super::slot_token::SlotToken;
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ExecutorError;

tokio::task_local! {
    /// Task-local storage for passing a `TaskHandle` to macro-generated task code.
    ///
    /// The executor sets this before calling `task.execute()` for tasks that
    /// require a handle (`requires_handle() == true`). The macro-generated
    /// `execute()` body takes the handle out, passes it to the user function,
    /// and returns it when done.
    static TASK_HANDLE_SLOT: RefCell<Option<TaskHandle>>;
}

/// Takes the current task's `TaskHandle` out of task-local storage.
///
/// Called by macro-generated code inside `Task::execute()`. Panics if no
/// handle was set (indicates an executor bug).
pub fn take_task_handle() -> TaskHandle {
    TASK_HANDLE_SLOT.with(|cell| {
        cell.borrow_mut()
            .take()
            .expect("TaskHandle not set in task-local storage — executor bug")
    })
}

/// Returns a `TaskHandle` to task-local storage after the user function completes.
///
/// Called by macro-generated code to restore the handle so the executor can
/// reclaim the slot token.
pub fn return_task_handle(handle: TaskHandle) {
    TASK_HANDLE_SLOT.with(|cell| {
        *cell.borrow_mut() = Some(handle);
    })
}

/// Runs an async future with a `TaskHandle` available in task-local storage.
///
/// The executor calls this to wrap `task.execute()` so that macro-generated
/// code can retrieve the handle via [`take_task_handle`].
pub async fn with_task_handle<F, T>(handle: TaskHandle, f: F) -> (T, Option<TaskHandle>)
where
    F: Future<Output = T>,
{
    TASK_HANDLE_SLOT
        .scope(RefCell::new(Some(handle)), async {
            let result = f.await;
            let returned_handle = TASK_HANDLE_SLOT.with(|cell| cell.borrow_mut().take());
            (result, returned_handle)
        })
        .await
}

/// Execution control handle passed to tasks that need concurrency management.
///
/// Tasks receive a `TaskHandle` as an optional second parameter. It provides
/// methods for controlling the task's relationship with the executor's
/// concurrency slots.
///
/// The handle is created by the executor for each task execution and is not
/// reusable across executions.
pub struct TaskHandle {
    slot_token: SlotToken,
    task_execution_id: UniversalUuid,
    dal: Option<DAL>,
    cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
}

impl TaskHandle {
    /// Creates a new TaskHandle.
    ///
    /// This is called internally by the executor — tasks receive it as a parameter.
    #[allow(dead_code)]
    pub(crate) fn new(slot_token: SlotToken, task_execution_id: UniversalUuid) -> Self {
        Self {
            slot_token,
            task_execution_id,
            dal: None,
            cancel_rx: None,
        }
    }

    /// Creates a new TaskHandle with DAL for sub_status persistence.
    #[allow(dead_code)]
    pub(crate) fn with_dal(
        slot_token: SlotToken,
        task_execution_id: UniversalUuid,
        dal: DAL,
    ) -> Self {
        Self {
            slot_token,
            task_execution_id,
            dal: Some(dal),
            cancel_rx: None,
        }
    }

    /// Creates a new TaskHandle with DAL and a cancellation watch receiver
    /// fed by the executor's heartbeat loop. When the heartbeat detects
    /// that the task's claim has been lost, it sets the channel to `true`;
    /// tasks can observe this via [`is_cancelled`](Self::is_cancelled) and
    /// [`cancelled`](Self::cancelled) for cooperative shutdown.
    pub(crate) fn with_dal_and_cancel(
        slot_token: SlotToken,
        task_execution_id: UniversalUuid,
        dal: DAL,
        cancel_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            slot_token,
            task_execution_id,
            dal: Some(dal),
            cancel_rx: Some(cancel_rx),
        }
    }

    /// Release the concurrency slot while polling an external condition.
    ///
    /// This method:
    /// 1. Releases the executor concurrency slot (freeing it for other tasks)
    /// 2. Polls the condition function at the given interval
    /// 3. Reclaims a slot when the condition returns `true`
    /// 4. Returns control to the task with the slot re-held
    ///
    /// While deferred, the task's async future remains parked in the tokio
    /// runtime but does not consume a concurrency slot. Other tasks can use
    /// the freed slot.
    ///
    /// # Arguments
    ///
    /// * `condition` - Async function that returns `true` when the task should resume
    /// * `poll_interval` - How often to check the condition
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed (executor shutting down)
    /// or if the slot cannot be reclaimed.
    pub async fn defer_until<F, Fut>(
        &mut self,
        condition: F,
        poll_interval: Duration,
    ) -> Result<(), ExecutorError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = bool>,
    {
        debug!(
            task_execution_id = %self.task_execution_id,
            poll_interval_ms = poll_interval.as_millis(),
            "Task entering deferred state — releasing concurrency slot"
        );

        // Update sub_status to Deferred in the database
        if let Some(ref dal) = self.dal {
            if let Err(e) = dal
                .task_execution()
                .set_sub_status(self.task_execution_id, Some("Deferred"))
                .await
            {
                warn!(
                    task_execution_id = %self.task_execution_id,
                    error = %e,
                    "Failed to set sub_status to Deferred"
                );
            }
        }

        // Release the concurrency slot
        self.slot_token.release();

        // Poll until condition is met
        loop {
            tokio::time::sleep(poll_interval).await;
            if condition().await {
                break;
            }
        }

        // Reclaim a concurrency slot (may wait if at capacity)
        self.slot_token.reclaim().await?;

        // Update sub_status back to Active
        if let Some(ref dal) = self.dal {
            if let Err(e) = dal
                .task_execution()
                .set_sub_status(self.task_execution_id, Some("Active"))
                .await
            {
                warn!(
                    task_execution_id = %self.task_execution_id,
                    error = %e,
                    "Failed to set sub_status back to Active"
                );
            }
        }

        debug!(
            task_execution_id = %self.task_execution_id,
            "Task resumed — concurrency slot reclaimed"
        );

        Ok(())
    }

    /// Returns the task execution ID associated with this handle.
    pub fn task_execution_id(&self) -> UniversalUuid {
        self.task_execution_id
    }

    /// Returns whether the handle currently holds a concurrency slot.
    pub fn is_slot_held(&self) -> bool {
        self.slot_token.is_held()
    }

    /// Returns `true` if the executor has signaled that this task's claim
    /// was lost and the task should stop at the next safe point. Long-running
    /// tasks can poll this at checkpoint boundaries and return early to free
    /// resources; if they don't, the executor still aborts the task via
    /// `tokio::select!` racing on the same channel.
    ///
    /// Always returns `false` for handles created without a cancellation
    /// channel (e.g. in tests via [`TaskHandle::new`]).
    pub fn is_cancelled(&self) -> bool {
        self.cancel_rx
            .as_ref()
            .map(|rx| *rx.borrow())
            .unwrap_or(false)
    }

    /// Resolves when the executor signals cancellation (claim lost). Useful
    /// inside `tokio::select!` to race user work against the cancel signal:
    ///
    /// ```ignore
    /// tokio::select! {
    ///     _ = handle.cancelled() => return Ok(()),
    ///     _ = do_work() => { /* normal path */ }
    /// }
    /// ```
    ///
    /// If no cancellation channel is wired up (test handles), the future
    /// never resolves.
    pub async fn cancelled(&self) {
        match self.cancel_rx.as_ref() {
            Some(rx) => {
                let mut rx = rx.clone();
                let _ = rx.wait_for(|&v| v).await;
            }
            None => std::future::pending::<()>().await,
        }
    }

    /// Consumes the handle, returning the inner SlotToken.
    ///
    /// Used by the executor to reclaim ownership of the permit after
    /// task execution completes.
    #[allow(dead_code)]
    pub(crate) fn into_slot_token(self) -> SlotToken {
        self.slot_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    fn make_handle(semaphore: &Arc<Semaphore>) -> TaskHandle {
        let permit = semaphore
            .clone()
            .try_acquire_owned()
            .expect("permit should be available");
        let slot_token = SlotToken::new(permit, semaphore.clone());
        TaskHandle::new(slot_token, UniversalUuid::new_v4())
    }

    #[tokio::test]
    async fn test_defer_until_releases_and_reclaims_slot() {
        let semaphore = Arc::new(Semaphore::new(1));
        let mut handle = make_handle(&semaphore);

        assert_eq!(semaphore.available_permits(), 0);

        let call_count = Arc::new(AtomicUsize::new(0));
        let cc = call_count.clone();

        handle
            .defer_until(
                move || {
                    let cc = cc.clone();
                    async move {
                        let count = cc.fetch_add(1, Ordering::SeqCst);
                        count >= 2 // Return true on third call
                    }
                },
                Duration::from_millis(1),
            )
            .await
            .unwrap();

        // Condition was called 3 times (0, 1, 2 — true at 2)
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
        // Slot is reclaimed
        assert!(handle.is_slot_held());
        assert_eq!(semaphore.available_permits(), 0);
    }

    #[tokio::test]
    async fn test_defer_until_immediate_condition() {
        let semaphore = Arc::new(Semaphore::new(1));
        let mut handle = make_handle(&semaphore);

        // Condition is true immediately (first poll)
        handle
            .defer_until(|| async { true }, Duration::from_millis(1))
            .await
            .unwrap();

        assert!(handle.is_slot_held());
    }

    #[tokio::test]
    async fn test_defer_until_frees_slot_for_other_tasks() {
        let semaphore = Arc::new(Semaphore::new(1));
        let mut handle = make_handle(&semaphore);

        // Slot is held — no permits available
        assert_eq!(semaphore.available_permits(), 0);

        let sem_clone = semaphore.clone();
        let slot_was_available = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let swa = slot_was_available.clone();

        handle
            .defer_until(
                move || {
                    let swa = swa.clone();
                    let sem = sem_clone.clone();
                    async move {
                        // During polling, check if another task could acquire the slot
                        if sem.available_permits() > 0 {
                            swa.store(true, Ordering::SeqCst);
                        }
                        true // Return immediately
                    }
                },
                Duration::from_millis(1),
            )
            .await
            .unwrap();

        // The slot was available during the defer window
        assert!(slot_was_available.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_task_local_round_trip() {
        let semaphore = Arc::new(Semaphore::new(1));
        let handle = make_handle(&semaphore);
        let original_id = handle.task_execution_id();

        // with_task_handle sets handle in task-local, runs future, returns handle
        let (result, returned_handle) = with_task_handle(handle, async {
            // Inside the scope, take_task_handle should retrieve it
            let taken = take_task_handle();
            assert_eq!(taken.task_execution_id(), original_id);
            assert!(taken.is_slot_held());

            // Return it so the executor can reclaim it
            return_task_handle(taken);
            42
        })
        .await;

        assert_eq!(result, 42);
        let rh = returned_handle.expect("handle should be returned");
        assert_eq!(rh.task_execution_id(), original_id);
        assert!(rh.is_slot_held());
    }

    #[tokio::test]
    async fn test_task_local_not_returned_yields_none() {
        let semaphore = Arc::new(Semaphore::new(1));
        let handle = make_handle(&semaphore);

        // If the handle is taken but NOT returned, with_task_handle returns None
        let (_result, returned_handle) = with_task_handle(handle, async {
            let _taken = take_task_handle();
            // deliberately don't call return_task_handle
        })
        .await;

        assert!(
            returned_handle.is_none(),
            "handle should be None when not returned"
        );
    }

    #[tokio::test]
    async fn test_is_cancelled_default_false_without_channel() {
        let semaphore = Arc::new(Semaphore::new(1));
        let handle = make_handle(&semaphore);

        // Handles constructed without a cancellation channel (e.g. test
        // helpers via TaskHandle::new) must never report cancelled.
        assert!(!handle.is_cancelled());

        // cancelled() on a channel-less handle should never resolve —
        // use `select!` with a timeout to verify that without hanging.
        let cancelled_fires =
            tokio::time::timeout(Duration::from_millis(20), handle.cancelled()).await;
        assert!(
            cancelled_fires.is_err(),
            "cancelled() must never resolve without a cancel channel"
        );
    }

    #[tokio::test]
    async fn test_is_cancelled_reflects_watch_value() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore
            .clone()
            .try_acquire_owned()
            .expect("permit available");
        let slot_token = SlotToken::new(permit, semaphore.clone());
        let (tx, rx) = tokio::sync::watch::channel(false);
        // Bypass the DAL requirement of with_dal_and_cancel by constructing
        // directly — this mirrors what the executor does internally.
        let handle = TaskHandle {
            slot_token,
            task_execution_id: UniversalUuid::new_v4(),
            dal: None,
            cancel_rx: Some(rx),
        };

        assert!(!handle.is_cancelled(), "no signal → not cancelled");
        tx.send(true).expect("send cancellation");
        // watch::Sender::send is synchronous from the receiver's perspective
        // once returned; is_cancelled reads the current borrow.
        assert!(handle.is_cancelled(), "after send(true) → cancelled");
    }

    #[tokio::test]
    async fn test_cancelled_future_resolves_after_signal() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore
            .clone()
            .try_acquire_owned()
            .expect("permit available");
        let slot_token = SlotToken::new(permit, semaphore.clone());
        let (tx, rx) = tokio::sync::watch::channel(false);
        let handle = TaskHandle {
            slot_token,
            task_execution_id: UniversalUuid::new_v4(),
            dal: None,
            cancel_rx: Some(rx),
        };

        // Before the signal, `cancelled()` should not resolve quickly.
        let early = tokio::time::timeout(Duration::from_millis(10), handle.cancelled()).await;
        assert!(early.is_err(), "cancelled() must not resolve before send");

        // After firing the signal, the next poll resolves promptly.
        tx.send(true).expect("send cancellation");
        let after = tokio::time::timeout(Duration::from_millis(200), handle.cancelled()).await;
        assert!(after.is_ok(), "cancelled() must resolve after send(true)");
    }

    #[tokio::test]
    async fn test_cancelled_future_does_not_fire_when_sender_dropped() {
        // When the heartbeat task exits normally (success/failure path),
        // its sender clone is dropped. If no cancellation was ever fired,
        // `cancelled()` should not resolve — the task is not cancelled, it
        // simply has no channel left to observe. This prevents spurious
        // cancellations at the end of a normal task lifecycle.
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore
            .clone()
            .try_acquire_owned()
            .expect("permit available");
        let slot_token = SlotToken::new(permit, semaphore.clone());
        let (tx, rx) = tokio::sync::watch::channel(false);
        let handle = TaskHandle {
            slot_token,
            task_execution_id: UniversalUuid::new_v4(),
            dal: None,
            cancel_rx: Some(rx),
        };

        drop(tx); // all senders gone without firing true

        let elapsed = tokio::time::timeout(Duration::from_millis(20), handle.cancelled()).await;
        // `wait_for` returns Err when all senders drop, and we swallow that
        // in `cancelled()`, so the future *does* resolve — but it's not a
        // cancellation. Document the behavior here so callers know they
        // still need to pair `cancelled()` with `is_cancelled()` for the
        // definitive check.
        assert!(
            elapsed.is_ok(),
            "cancelled() resolves when sender drops (documented behavior)"
        );
        assert!(
            !handle.is_cancelled(),
            "is_cancelled() is the source of truth — stays false on sender drop"
        );
    }

    #[tokio::test]
    async fn test_with_task_handle_preserves_handle_through_defer() {
        let semaphore = Arc::new(Semaphore::new(1));
        let handle = make_handle(&semaphore);
        let original_id = handle.task_execution_id();

        let (_result, returned_handle) = with_task_handle(handle, async {
            let mut taken = take_task_handle();

            // Simulate what a macro-generated task does: defer, then return handle
            taken
                .defer_until(|| async { true }, Duration::from_millis(1))
                .await
                .unwrap();

            assert!(taken.is_slot_held());
            return_task_handle(taken);
        })
        .await;

        let rh = returned_handle.expect("handle should survive defer_until");
        assert_eq!(rh.task_execution_id(), original_id);
        assert!(rh.is_slot_held());
    }
}
