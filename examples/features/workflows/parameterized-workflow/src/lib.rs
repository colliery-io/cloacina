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

/*!
# Parameterized Workflow

One workflow template, many differently-configured runs. `params(...)` on
`#[workflow]` declares the configurable surface; the server validates every
run's provided values against it (types, required-ness, defaults) and
delivers the bound values to tasks as top-level context keys.

See the README for the gold-path run recipe.
*/

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

/// A file-sync template: where to read, where to write, how to transfer.
///
/// `source` and `dst` are required (no default); `mode` and `max_files`
/// fall back to their defaults when a run doesn't bind them.
#[workflow(
    name = "sync_file",
    description = "Sync files from a source to a destination — parameterized template",
    author = "Cloacina Demo Team",
    params(
        source: String,
        dst: String,
        mode: String = "copy",
        max_files: i64 = 100,
    )
)]
pub mod sync_file {
    use super::*;

    /// Read the bound params off the context and turn them into a plan.
    /// Bound values arrive as flat top-level context keys.
    #[task]
    pub async fn plan_sync(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let source = context
            .get("source")
            .and_then(|v| v.as_str().map(String::from))
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "missing bound param: source".to_string(),
            })?;
        let dst = context
            .get("dst")
            .and_then(|v| v.as_str().map(String::from))
            .ok_or_else(|| TaskError::ValidationFailed {
                message: "missing bound param: dst".to_string(),
            })?;
        let mode = context
            .get("mode")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "copy".to_string());
        let max_files = context
            .get("max_files")
            .and_then(|v| v.as_i64())
            .unwrap_or(100);

        if mode != "copy" && mode != "move" {
            return Err(TaskError::ValidationFailed {
                message: format!("mode must be 'copy' or 'move', got '{mode}'"),
            });
        }

        println!("📋 plan: {mode} up to {max_files} files  {source} → {dst}");
        context.insert(
            "sync_plan",
            serde_json::json!({
                "source": source,
                "dst": dst,
                "mode": mode,
                "max_files": max_files,
            }),
        )?;
        Ok(())
    }

    /// Simulate the transfer the plan describes.
    #[task(dependencies = ["plan_sync"], retry_attempts = 2)]
    pub async fn execute_sync(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let plan =
            context
                .get("sync_plan")
                .cloned()
                .ok_or_else(|| TaskError::ValidationFailed {
                    message: "missing sync_plan".to_string(),
                })?;
        let max_files = plan.get("max_files").and_then(|v| v.as_i64()).unwrap_or(0);
        // Simulate: "transfer" a deterministic number of files under the cap.
        let transferred = max_files.min(42);
        println!("🚚 transferred {transferred} file(s)");
        context.insert(
            "sync_result",
            serde_json::json!({ "transferred": transferred, "plan": plan }),
        )?;
        Ok(())
    }

    /// Report what this particular parameterization did.
    #[task(dependencies = ["execute_sync"])]
    pub async fn report(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let result =
            context
                .get("sync_result")
                .cloned()
                .ok_or_else(|| TaskError::ValidationFailed {
                    message: "missing sync_result".to_string(),
                })?;
        println!("✅ sync complete: {result}");
        context.insert("sync_report", result)?;
        Ok(())
    }
}
