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

use cloacina::task;

#[task(id = "test", dependencies = [])]
async fn test_task(
    _context: &mut cloacina::Context<serde_json::Value>,
) -> Result<(), cloacina::TaskError> {
    Ok(())
}

#[test]
fn test_macro_expansion() {
    // Just test that the macro expands without error
    let _task = test_task_task();
}

// CLOACINA-T-1014: `timeout_seconds` gives the task a time limit of its own,
// and `retry_attempts = 0` reaches `Task::retry_policy()`.
#[task(id = "long_task", timeout_seconds = 600, retry_attempts = 0)]
async fn long_task(
    _context: &mut cloacina::Context<serde_json::Value>,
) -> Result<(), cloacina::TaskError> {
    Ok(())
}

#[test]
fn test_task_time_limit_and_retry_attempts_from_the_macro() {
    use cloacina::Task;
    let task = long_task_task();
    assert_eq!(task.timeout(), Some(std::time::Duration::from_secs(600)));
    assert_eq!(task.retry_policy().max_attempts, 0);
    // A task that sets no limit has none of its own.
    assert_eq!(test_task_task().timeout(), None);
}
