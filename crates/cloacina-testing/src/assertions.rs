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

//! Assertion helpers for test results.

use crate::result::TestResult;

impl TestResult {
    /// Asserts that all tasks completed successfully.
    ///
    /// # Panics
    ///
    /// Panics with a list of non-completed tasks if any task failed or was skipped.
    pub fn assert_all_completed(&self) {
        let failures: Vec<_> = self
            .task_outcomes
            .iter()
            .filter(|(_, outcome)| !outcome.is_completed())
            .map(|(id, outcome)| format!("  '{}': {}", id, outcome))
            .collect();

        if !failures.is_empty() {
            panic!(
                "assertion failed: expected all tasks to be Completed, but found:\n{}",
                failures.join("\n")
            );
        }
    }

    /// Asserts that a specific task completed successfully.
    ///
    /// # Panics
    ///
    /// Panics if the task is not found or did not complete.
    pub fn assert_task_completed(&self, task_id: &str) {
        let outcome = self.task_outcomes.get(task_id).unwrap_or_else(|| {
            panic!(
                "assertion failed: task '{}' not found in results. Available: {:?}",
                task_id,
                self.task_outcomes.keys().collect::<Vec<_>>()
            )
        });

        if !outcome.is_completed() {
            panic!(
                "assertion failed: expected task '{}' to be Completed, but was {}",
                task_id, outcome
            );
        }
    }

    /// Asserts that a specific task failed.
    ///
    /// # Panics
    ///
    /// Panics if the task is not found or did not fail.
    pub fn assert_task_failed(&self, task_id: &str) {
        let outcome = self.task_outcomes.get(task_id).unwrap_or_else(|| {
            panic!(
                "assertion failed: task '{}' not found in results. Available: {:?}",
                task_id,
                self.task_outcomes.keys().collect::<Vec<_>>()
            )
        });

        if !outcome.is_failed() {
            panic!(
                "assertion failed: expected task '{}' to be Failed, but was {}",
                task_id, outcome
            );
        }
    }

    /// Asserts that a specific task was skipped.
    ///
    /// # Panics
    ///
    /// Panics if the task is not found or was not skipped.
    pub fn assert_task_skipped(&self, task_id: &str) {
        let outcome = self.task_outcomes.get(task_id).unwrap_or_else(|| {
            panic!(
                "assertion failed: task '{}' not found in results. Available: {:?}",
                task_id,
                self.task_outcomes.keys().collect::<Vec<_>>()
            )
        });

        if !outcome.is_skipped() {
            panic!(
                "assertion failed: expected task '{}' to be Skipped, but was {}",
                task_id, outcome
            );
        }
    }
}
