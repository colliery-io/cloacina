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
# Workflow Secrets

A workflow declares the secrets it requires with `secrets(...)`; a run binds
each declared name to a concrete tenant secret with a `{"$secret": "name"}`
reference. The plaintext is resolved at execution through a side channel
(`context.secret(...)`) and is **never** written into the durable context —
tasks may only persist non-sensitive derived facts.

See the README for the gold-path run recipe.
*/

use cloacina_workflow::{task, workflow, Context, TaskError};

cloacina_workflow_plugin::package!();

/// Notify the on-call channel using an API token stored as a tenant secret.
///
/// `api_token` is a declared secret: runs must bind it to a real tenant
/// secret; the value never appears in the context, execution history, or
/// logs.
#[workflow(
    name = "notify_oncall",
    description = "Send an on-call notification using a tenant-stored API token",
    author = "Cloacina Demo Team",
    params(
        channel: String = "#ops",
    ),
    secrets(api_token)
)]
pub mod notify_oncall {
    use super::*;

    /// Resolve the bound secret and prove it WITHOUT leaking it: only
    /// non-sensitive derived facts (a boolean + the token length) are
    /// written back into the durable context.
    #[task]
    pub async fn resolve_token(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let token = context
            .secret_field("api_token", "token")
            .await
            .map_err(|e| TaskError::Unknown {
                task_id: "resolve_token".to_string(),
                message: format!("secret resolution failed: {e}"),
            })?;

        println!(
            "🔑 token resolved ({} bytes) — value stays out of the context",
            token.len()
        );
        context.insert("token_resolved", serde_json::json!(true))?;
        context.insert("token_len", serde_json::json!(token.len()))?;
        Ok(())
    }

    /// "Send" the notification to the configured channel.
    #[task(dependencies = ["resolve_token"], retry_attempts = 2)]
    pub async fn send_notification(
        context: &mut Context<serde_json::Value>,
    ) -> Result<(), TaskError> {
        let channel = context
            .get("channel")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "#ops".to_string());
        let resolved = context
            .get("token_resolved")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !resolved {
            return Err(TaskError::ValidationFailed {
                message: "token was not resolved".to_string(),
            });
        }
        println!("📣 notification sent to {channel} (authenticated with the resolved token)");
        context.insert(
            "notification",
            serde_json::json!({ "channel": channel, "sent": true }),
        )?;
        Ok(())
    }
}
