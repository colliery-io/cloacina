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

//! Host-side `Trigger` adapter that dispatches `poll()` through a
//! packaged cdylib via fidius FFI.
//!
//! Background: `Runtime::seed_from_inventory` only sees `inventory::iter`
//! entries that were submitted by code linked against the SAME
//! compilation of `cloacina-workflow-plugin`. Independently-compiled
//! cdylibs (every fixture under `examples/fixtures/*` and most user
//! workflows) are separate workspaces with their own `cloacina-workflow-
//! plugin` build, so their inventory submissions land in a private
//! linker section the host can't enumerate. Instead of giving up on
//! workflow-trigger subscriptions for those packages, the reconciler
//! builds an `FfiTriggerImpl` per FFI-declared trigger: the host-side
//! impl looks like a normal `cloacina_workflow::Trigger`, but its
//! `poll()` calls method index 6 (`invoke_trigger_poll`) on the
//! cdylib's plugin handle, which walks the cdylib's local inventory
//! and runs the user's `poll()` body on the cdylib's own tokio
//! runtime.

use cloacina_workflow::{Context, Trigger, TriggerError, TriggerResult};
use cloacina_workflow_plugin::{
    TriggerInvokeRequest, TriggerInvokeResult, METHOD_INVOKE_TRIGGER_POLL,
};
use std::sync::Arc;
use std::time::Duration;

/// Host-side `Trigger` impl that proxies to a packaged cdylib through
/// fidius. Cached metadata (`name`, `poll_interval`, `allow_concurrent`,
/// `cron_expression`) comes from `get_trigger_metadata` at registration
/// time, so the synchronous accessors don't cross the FFI boundary —
/// only `poll()` does.
pub struct FfiTriggerImpl {
    handle: Arc<fidius_host::PluginHandle>,
    name: String,
    poll_interval: Duration,
    allow_concurrent: bool,
    cron_expression: Option<String>,
}

impl FfiTriggerImpl {
    pub fn new(
        handle: Arc<fidius_host::PluginHandle>,
        name: String,
        poll_interval: Duration,
        allow_concurrent: bool,
        cron_expression: Option<String>,
    ) -> Self {
        Self {
            handle,
            name,
            poll_interval,
            allow_concurrent,
            cron_expression,
        }
    }
}

impl std::fmt::Debug for FfiTriggerImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfiTriggerImpl")
            .field("name", &self.name)
            .field("poll_interval", &self.poll_interval)
            .field("allow_concurrent", &self.allow_concurrent)
            .field("cron_expression", &self.cron_expression)
            .finish()
    }
}

#[async_trait::async_trait]
impl Trigger for FfiTriggerImpl {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.allow_concurrent
    }

    fn cron_expression(&self) -> Option<String> {
        self.cron_expression.clone()
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let handle = self.handle.clone();
        let request = TriggerInvokeRequest {
            trigger_name: self.name.clone(),
        };
        // call_method is sync; bounce to a blocking thread so the host's
        // async runtime stays responsive while the cdylib's tokio runtime
        // drives user poll() code.
        let result: Result<TriggerInvokeResult, fidius_host::CallError> =
            tokio::task::spawn_blocking(move || {
                handle.call_method(METHOD_INVOKE_TRIGGER_POLL, &request)
            })
            .await
            .map_err(|e| TriggerError::PollError {
                message: format!("FFI trigger poll spawn_blocking failed: {}", e),
            })?;

        let r = result.map_err(|e| TriggerError::PollError {
            message: format!("FFI trigger poll call failed: {:?}", e),
        })?;

        if let Some(err) = r.error {
            return Err(TriggerError::PollError { message: err });
        }

        if r.fire {
            let ctx = match r.context_json {
                Some(json) => {
                    Some(
                        Context::from_json(json).map_err(|e| TriggerError::PollError {
                            message: format!("FFI trigger context parse failed: {}", e),
                        })?,
                    )
                }
                None => None,
            };
            Ok(TriggerResult::Fire(ctx))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}
