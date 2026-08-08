/*
 *  Copyright 2026 Colliery Software
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

// Fixture for the named-workflow-instance e2e lane (CLOACI-T-0927).
//
// Declares one REQUIRED param (`region`, no default) and one DEFAULTED param
// (`batch_size`). The required one is what makes create-time validation
// observable: creating an instance without it must be rejected by the server.
//
// The task echoes both params back into the context under distinct keys. The
// bound params themselves already arrive as top-level context keys via the
// fire-time merge, but echoing them proves the TASK actually saw the bound
// values rather than the harness merely reading back what it wrote.

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

#[workflow(
    name = "instance_params_workflow",
    description = "named-instance e2e fixture — required + defaulted params",
    author = "instance-e2e",
    params(
        region: String,
        batch_size: u32 = 100,
    )
)]
pub mod instance_params_workflow {
    use super::*;

    #[task(retry_attempts = 0)]
    pub async fn echo_params(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        // Read the params as the workflow author would — off the context, where
        // the fire-time merge put them.
        let region = context
            .get("region")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| TaskError::ExecutionFailed {
                message: "required param 'region' missing from context".to_string(),
                task_id: "echo_params".to_string(),
                timestamp: chrono::Utc::now(),
            })?;
        let batch_size = context
            .get("batch_size")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| TaskError::ExecutionFailed {
                message: "param 'batch_size' missing from context".to_string(),
                task_id: "echo_params".to_string(),
                timestamp: chrono::Utc::now(),
            })?;

        context.insert("observed_region", serde_json::json!(region))?;
        context.insert("observed_batch_size", serde_json::json!(batch_size))?;
        Ok(())
    }
}
