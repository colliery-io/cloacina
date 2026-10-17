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

//! A sync definition is a workflow instance of the Sync workflow; its fire is
//! a pass, not one execution (COLLIERY-I-0632 / CLOACINA-T-0978).
//!
//! [`SyncFireHook`] takes over every fire of the `sync` workflow, on its cron
//! or by hand: it prepares the pass, starts the first segment, hands that
//! execution back to the runner (the schedule's audit record points at it) and
//! runs the remaining segments on a spawned task. One pass per sync at a time:
//! a fire that finds the sync's pass still running is skipped with an error
//! the schedule's log keeps, the way weir skipped a run already active.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use cloacina::Context;
use cloacina::WorkflowExecutionError;
use cloacina::computation_graph::reactor_ownership::ReactorOwnership;
use cloacina::dal::DAL;
use cloacina::executor::WorkflowExecutor;
use cloacina::workflow_instance::{InstanceFireFuture, InstanceFireHook};
use serde_json::Value;

use crate::WORKFLOW_NAME;
use crate::dispatcher::{Dispatcher, SyncDefinition};

/// The instance fire hook for Sync. Install it on a runner with
/// [`crate::install`].
pub struct SyncFireHook {
    in_flight: Arc<Mutex<HashSet<String>>>,
    ownership: Option<Arc<dyn ReactorOwnership>>,
    tenant: Option<String>,
    /// The runner the hook is installed on, to hand back a handle to an
    /// execution that was already in flight. It holds the hook in turn; the
    /// cycle lives as long as the runner, which is the process.
    runner: Mutex<Option<cloacina::runner::DefaultRunner>>,
}

impl SyncFireHook {
    pub fn new(ownership: Option<Arc<dyn ReactorOwnership>>, tenant: Option<String>) -> Self {
        Self {
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            ownership,
            tenant,
            runner: Mutex::new(None),
        }
    }

    /// The runner this hook is installed on (set by [`crate::install`]).
    pub fn set_runner(&self, runner: cloacina::runner::DefaultRunner) {
        *self.runner.lock().unwrap() = Some(runner);
    }

    /// The syncs whose pass is running on this process right now.
    pub fn running(&self) -> Vec<String> {
        let mut v: Vec<String> = self.in_flight.lock().unwrap().iter().cloned().collect();
        v.sort();
        v
    }
}

fn failed(message: String) -> WorkflowExecutionError {
    WorkflowExecutionError::ExecutionFailed { message }
}

impl InstanceFireHook for SyncFireHook {
    fn fire(
        &self,
        executor: Arc<dyn WorkflowExecutor>,
        dal: Arc<DAL>,
        workflow_name: &str,
        context: Context<Value>,
    ) -> Option<InstanceFireFuture> {
        if workflow_name != WORKFLOW_NAME {
            return None;
        }
        let in_flight = self.in_flight.clone();
        let runner = self.runner.lock().unwrap().clone();
        let mut dispatcher = Dispatcher::new(executor, dal);
        if let Some(own) = &self.ownership {
            dispatcher = dispatcher.with_ownership(own.clone(), self.tenant.clone());
        }
        Some(Box::pin(async move {
            let def = SyncDefinition::new(context.data().clone().into_iter().collect());
            let sync = def
                .name()
                .ok_or_else(|| failed("sync fire without a `sync` param".to_string()))?
                .to_string();
            {
                let mut set = in_flight.lock().unwrap();
                if !set.insert(sync.clone()) {
                    return Err(failed(format!(
                        "sync `{sync}`: a pass is still running on this process; this fire is skipped"
                    )));
                }
            }
            let release = |in_flight: &Arc<Mutex<HashSet<String>>>, sync: &str| {
                in_flight.lock().unwrap().remove(sync);
            };

            // Never block the fire on a wait: the manual-run request and the cron
            // loop are both waiting on this future. Claim, then either hand back
            // the execution already in flight (a crash may have left one whose
            // stale claim the sweeper has yet to reset) or start the first
            // segment; the spawned task does any waiting.
            let prepared = match dispatcher.claim(&def).await {
                Ok(p) => p,
                Err(e) => {
                    release(&in_flight, &sync);
                    return Err(failed(e.to_string()));
                }
            };
            let outstanding = match dispatcher.outstanding(&sync).await {
                Ok(o) => o,
                Err(e) => {
                    dispatcher.abandon(prepared).await;
                    release(&in_flight, &sync);
                    return Err(failed(e.to_string()));
                }
            };
            let first = match (outstanding, runner) {
                (Some(id), Some(runner)) => {
                    tracing::info!(sync = %sync, execution_id = %id, "sync: fire resumes the execution already in flight");
                    cloacina::executor::WorkflowExecution::new(
                        id,
                        WORKFLOW_NAME.to_string(),
                        runner,
                    )
                }
                (Some(_), None) => {
                    // No runner to hand back a handle with: wait here instead.
                    dispatcher.abandon(prepared).await;
                    release(&in_flight, &sync);
                    return Err(failed(format!(
                        "sync `{sync}`: an execution is still in flight; fire again when it ends"
                    )));
                }
                (None, _) => match dispatcher.start_segment(&prepared).await {
                    Ok(h) => h,
                    Err(e) => {
                        dispatcher.abandon(prepared).await;
                        release(&in_flight, &sync);
                        return Err(failed(e.to_string()));
                    }
                },
            };
            let first_id = first.execution_id;
            let (_stop_tx, stop) = tokio::sync::watch::channel(false);
            let in_flight_task = in_flight.clone();
            let sync_task = sync.clone();
            tokio::spawn(async move {
                match dispatcher.finish_pass(prepared, first_id, stop).await {
                    Ok(outcome) => {
                        tracing::info!(sync = %sync_task, ?outcome, "sync: pass finished")
                    }
                    Err(e) => tracing::error!(sync = %sync_task, error = %e, "sync: pass failed"),
                }
                in_flight_task.lock().unwrap().remove(&sync_task);
            });
            Ok(first)
        }))
    }
}
