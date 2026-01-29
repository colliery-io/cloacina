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

//! Concurrency slot token for executor resource management.
//!
//! `SlotToken` wraps a semaphore permit and provides a clean interface for
//! releasing and reclaiming concurrency slots. This abstraction decouples
//! the TaskHandle from tokio's specific permit types, allowing future
//! extensions like weighted slots or cross-executor management.

use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::ExecutorError;

/// A token representing a held concurrency slot in the executor.
///
/// When a task is executing, it holds a `SlotToken` that reserves one of the
/// executor's concurrency slots. The token can be temporarily released (e.g.,
/// during deferred polling) and later reclaimed.
///
/// # Lifecycle
///
/// 1. Created by the executor when a task begins execution (permit acquired)
/// 2. Held for the duration of normal task execution
/// 3. Optionally released during `defer_until` to free the slot
/// 4. Reclaimed when the deferred condition is met
/// 5. Dropped when the task completes (permit returned to semaphore)
pub struct SlotToken {
    permit: Option<OwnedSemaphorePermit>,
    semaphore: Arc<Semaphore>,
}

impl SlotToken {
    /// Creates a new SlotToken from an already-acquired permit.
    pub(crate) fn new(permit: OwnedSemaphorePermit, semaphore: Arc<Semaphore>) -> Self {
        Self {
            permit: Some(permit),
            semaphore,
        }
    }

    /// Release the concurrency slot, freeing it for other tasks.
    ///
    /// This drops the semaphore permit, making the slot available immediately.
    /// After calling `release()`, the token is in a released state and
    /// `reclaim()` must be called before the task resumes real work.
    ///
    /// Returns `true` if a permit was released, `false` if already released.
    pub fn release(&mut self) -> bool {
        self.permit.take().is_some()
    }

    /// Reclaim a concurrency slot after it was released.
    ///
    /// This acquires a new semaphore permit. If all slots are occupied,
    /// this will wait until one becomes available.
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed (executor shutting down).
    pub async fn reclaim(&mut self) -> Result<(), ExecutorError> {
        if self.permit.is_some() {
            // Already holding a permit, nothing to do
            return Ok(());
        }

        let permit = self.semaphore.clone().acquire_owned().await.map_err(|_| {
            ExecutorError::TaskExecution(crate::TaskError::ExecutionFailed {
                message: "semaphore closed during slot reclaim".into(),
                task_id: String::new(),
                timestamp: chrono::Utc::now(),
            })
        })?;

        self.permit = Some(permit);
        Ok(())
    }

    /// Returns whether the token currently holds a concurrency slot.
    pub fn is_held(&self) -> bool {
        self.permit.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_slot_token_release_frees_permit() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let mut token = SlotToken::new(permit, semaphore.clone());

        assert!(token.is_held());
        assert_eq!(semaphore.available_permits(), 0);

        // Release should free the slot
        assert!(token.release());
        assert!(!token.is_held());
        assert_eq!(semaphore.available_permits(), 1);

        // Double release returns false
        assert!(!token.release());
    }

    #[tokio::test]
    async fn test_slot_token_reclaim_reacquires_permit() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let mut token = SlotToken::new(permit, semaphore.clone());

        token.release();
        assert_eq!(semaphore.available_permits(), 1);

        token.reclaim().await.unwrap();
        assert!(token.is_held());
        assert_eq!(semaphore.available_permits(), 0);
    }

    #[tokio::test]
    async fn test_slot_token_reclaim_when_already_held_is_noop() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let mut token = SlotToken::new(permit, semaphore.clone());

        // Reclaim when already held should be a no-op
        token.reclaim().await.unwrap();
        assert!(token.is_held());
        assert_eq!(semaphore.available_permits(), 0);
    }

    #[tokio::test]
    async fn test_slot_token_drop_releases_permit() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        {
            let _token = SlotToken::new(permit, semaphore.clone());
            assert_eq!(semaphore.available_permits(), 0);
        }
        // Token dropped — permit returned
        assert_eq!(semaphore.available_permits(), 1);
    }

    #[tokio::test]
    async fn test_slot_token_reclaim_waits_for_availability() {
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let mut token = SlotToken::new(permit, semaphore.clone());

        // Release our slot
        token.release();

        // Another task grabs the slot
        let _other_permit = semaphore.clone().acquire_owned().await.unwrap();
        assert_eq!(semaphore.available_permits(), 0);

        // Reclaim should wait — spawn it and drop the other permit
        let sem_clone = semaphore.clone();
        let handle = tokio::spawn(async move {
            token.reclaim().await.unwrap();
            assert!(token.is_held());
            token
        });

        // Give the reclaim task a moment to start waiting
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(sem_clone.available_permits(), 0);

        // Release the other permit — reclaim should complete
        drop(_other_permit);

        let token = handle.await.unwrap();
        assert!(token.is_held());
        assert_eq!(sem_clone.available_permits(), 0);
    }
}
