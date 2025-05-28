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

use cloacina::{task, Context, Task, TaskError};

#[task(id = "test-task", dependencies = [])]
async fn test_task(_context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    Ok(())
}

#[test]
fn test_task_generation() {
    // This should verify that the task constructor was created
    let task = test_task_task();
    assert_eq!(task.id(), "test-task");
    assert_eq!(task.dependencies().len(), 0);
}
