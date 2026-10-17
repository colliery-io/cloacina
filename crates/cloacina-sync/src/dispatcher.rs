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

//! The dispatcher (COLLIERY-I-0632 / CLOACINA-T-0977, CLOACINA-A-0013
//! amendment 2): one pass of a sync as a series of segments.
//!
//! The dispatcher reads the committed stream state, creates one execution of
//! the Sync workflow with the sync's params and that state, waits for it, reads
//! the state the write task committed, and goes again until the source reports
//! its end. One execution is in flight per sync, ever: that is what makes a
//! sync's executions serial and in order, and what keeps the dispatcher out of
//! the cursor's way. The dispatcher never writes the cursor; the write task's
//! completion transaction does.
//!
//! **Resume.** An execution that was in flight when the last dispatcher died
//! may have written its segment without the completion commit landing yet. So
//! a pass first waits for every outstanding execution of this sync, then opens
//! the next segment at whatever state those commits left.
//!
//! **Ownership.** On a server, one replica runs a sync's loop: the dispatcher
//! claims the sync on the ownership session (CLOACINA-A-0012) in the `Sync`
//! domain, and releases it when the pass ends. Loss of the claim stops the loop
//! before any new execution is created. Embedded and SQLite pass no ownership
//! and run as the single process they are.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use cloacina::Context;
use cloacina::computation_graph::reactor_ownership::{ReactorId, ReactorOwnership};
use cloacina::dal::DAL;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::executor::{WorkflowExecution, WorkflowExecutor, WorkflowStatus};
use cloacina::runner::DefaultRunner;
use cloacina_constructor_contract::sync::StreamState;
use serde_json::Value;

use crate::WORKFLOW_NAME;
use crate::params::{SyncParams, keys};

/// A sync as the dispatcher runs it: the instance params of the Sync
/// workflow, by key, with `sync` naming the sync.
#[derive(Debug, Clone)]
pub struct SyncDefinition {
    pub params: HashMap<String, Value>,
}

impl SyncDefinition {
    /// From the flat params of a workflow instance.
    pub fn new(params: HashMap<String, Value>) -> Self {
        Self { params }
    }

    /// The sync's name, the `sync` param.
    pub fn name(&self) -> Option<&str> {
        self.params.get(keys::SYNC).and_then(Value::as_str)
    }

    fn context(&self) -> Result<Context<Value>, DispatchError> {
        let mut ctx = Context::new();
        for (k, v) in &self.params {
            ctx.insert(k.clone(), v.clone())
                .map_err(|e| DispatchError::Params(e.to_string()))?;
        }
        Ok(ctx)
    }
}

/// How a pass ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassOutcome {
    /// The source reported its end. `segments` executions ran in this pass.
    Exhausted { segments: u64 },
    /// The caller asked the loop to stop between segments.
    Stopped { segments: u64 },
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("sync definition: {0}")]
    Params(String),
    #[error("another replica owns sync `{0}`")]
    NotOwner(String),
    #[error("ownership claim for sync `{sync}` failed: {error}")]
    Ownership { sync: String, error: String },
    #[error("database: {0}")]
    Database(String),
    #[error("execution {execution_id} of sync `{sync}` ended {status:?}: {error}")]
    SegmentFailed {
        sync: String,
        execution_id: UniversalUuid,
        status: WorkflowStatus,
        error: String,
    },
    #[error("start execution for sync `{sync}`: {error}")]
    Start { sync: String, error: String },
    #[error("wait for execution {execution_id} of sync `{sync}`: {error}")]
    Wait {
        sync: String,
        execution_id: UniversalUuid,
        error: String,
    },
}

/// Runs passes of syncs on one runner.
pub struct Dispatcher {
    runner: DefaultRunner,
    dal: DAL,
    ownership: Option<Arc<dyn ReactorOwnership>>,
    tenant: Option<String>,
    /// Ceiling on one execution's wait; `None` waits as long as it takes.
    segment_timeout: Option<Duration>,
}

impl Dispatcher {
    /// A dispatcher on a runner, for an embedded runner or a single-replica
    /// server: no ownership, the one process runs every sync.
    pub fn new(runner: DefaultRunner) -> Self {
        let dal = DAL::new(runner.database().clone());
        Self {
            runner,
            dal,
            ownership: None,
            tenant: None,
            segment_timeout: None,
        }
    }

    /// Take the sync's lock on this ownership session before a pass, in the
    /// given tenant, and release it after. For a server with replicas.
    pub fn with_ownership(
        mut self,
        ownership: Arc<dyn ReactorOwnership>,
        tenant: Option<String>,
    ) -> Self {
        self.ownership = Some(ownership);
        self.tenant = tenant;
        self
    }

    /// A ceiling on how long one segment's execution may take.
    pub fn with_segment_timeout(mut self, timeout: Duration) -> Self {
        self.segment_timeout = Some(timeout);
        self
    }

    /// Run one pass of `def`: segments until the source is exhausted, or until
    /// `stop` turns true between segments.
    pub async fn run_pass(
        &self,
        def: &SyncDefinition,
        mut stop: tokio::sync::watch::Receiver<bool>,
    ) -> Result<PassOutcome, DispatchError> {
        let sync = def
            .name()
            .ok_or_else(|| DispatchError::Params(format!("param `{}` is missing", keys::SYNC)))?
            .to_string();
        let template = def.context()?;
        let params = SyncParams::from_context(&template)
            .map_err(|e| DispatchError::Params(e.to_string()))?;

        let id = ReactorId::sync(self.tenant.clone(), sync.clone());
        if let Some(own) = &self.ownership {
            match own.claim(&id).await {
                Ok(true) => {}
                Ok(false) => return Err(DispatchError::NotOwner(sync)),
                Err(error) => return Err(DispatchError::Ownership { sync, error }),
            }
        }
        let outcome = self.run_owned(&sync, &params, &template, &mut stop).await;
        if let Some(own) = &self.ownership {
            if let Err(e) = own.release(&id).await {
                tracing::warn!(sync = %sync, error = %e, "sync: ownership release failed");
            }
        }
        outcome
    }

    async fn run_owned(
        &self,
        sync: &str,
        params: &SyncParams,
        template: &Context<Value>,
        stop: &mut tokio::sync::watch::Receiver<bool>,
    ) -> Result<PassOutcome, DispatchError> {
        self.finish_outstanding(sync).await?;

        let mut segments = 0u64;
        loop {
            if *stop.borrow() {
                return Ok(PassOutcome::Stopped { segments });
            }
            if let Some(own) = &self.ownership {
                let believed = own.believed_owned().await;
                if !believed.iter().any(|b| b.name == sync) {
                    return Err(DispatchError::NotOwner(sync.to_string()));
                }
            }

            let state = self.committed_state(sync, params).await?;
            let mut ctx = template.clone_data();
            let state_json =
                serde_json::to_string(&state).map_err(|e| DispatchError::Params(e.to_string()))?;
            ctx.remove(keys::STATE);
            ctx.insert(keys::STATE, Value::String(state_json))
                .map_err(|e| DispatchError::Params(e.to_string()))?;

            let handle = self
                .runner
                .execute_async(WORKFLOW_NAME, ctx)
                .await
                .map_err(|e| DispatchError::Start {
                    sync: sync.to_string(),
                    error: e.to_string(),
                })?;
            let execution_id = UniversalUuid(handle.execution_id);
            tracing::info!(sync, %execution_id, segment = segments + 1, cursor = ?state.cursor, "sync: segment started");
            self.wait(sync, handle).await?;
            segments += 1;

            let (_, exhausted) = self.committed_row(sync, params).await?;
            if exhausted {
                tracing::info!(sync, segments, "sync: pass done, source exhausted");
                return Ok(PassOutcome::Exhausted { segments });
            }
        }
    }

    /// The committed state of the sync's stream, or a fresh read.
    async fn committed_state(
        &self,
        sync: &str,
        p: &SyncParams,
    ) -> Result<StreamState, DispatchError> {
        Ok(self.committed_row(sync, p).await?.0)
    }

    async fn committed_row(
        &self,
        sync: &str,
        p: &SyncParams,
    ) -> Result<(StreamState, bool), DispatchError> {
        let row = self
            .dal
            .sync_stream_state()
            .get(sync, &p.stream.stream, &p.partition.id)
            .await
            .map_err(|e| DispatchError::Database(e.to_string()))?;
        Ok(match row {
            None => (StreamState::default(), false),
            Some(r) => (
                StreamState {
                    cursor: r.cursor,
                    opaque: r.opaque.0,
                },
                r.exhausted.0,
            ),
        })
    }

    /// Wait for every outstanding execution of this sync: one may have written
    /// its segment without its completion landing yet.
    async fn finish_outstanding(&self, sync: &str) -> Result<(), DispatchError> {
        let active = self
            .dal
            .workflow_execution()
            .get_active_executions()
            .await
            .map_err(|e| DispatchError::Database(e.to_string()))?;
        for ex in active
            .into_iter()
            .filter(|e| e.workflow_name == WORKFLOW_NAME)
        {
            let Some(context_id) = ex.context_id else {
                continue;
            };
            let ctx = match self.dal.context().read::<Value>(context_id).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            if ctx.get(keys::SYNC).and_then(Value::as_str) != Some(sync) {
                continue;
            }
            tracing::info!(sync, execution_id = %ex.id, "sync: waiting for an outstanding execution before the next segment");
            let handle =
                WorkflowExecution::new(ex.id.0, WORKFLOW_NAME.to_string(), self.runner.clone());
            // An outstanding execution that fails is this pass's failure too:
            // its segment is not committed, and the next pass will redo it.
            self.wait(sync, handle).await?;
        }
        Ok(())
    }

    async fn wait(&self, sync: &str, handle: WorkflowExecution) -> Result<(), DispatchError> {
        let execution_id = UniversalUuid(handle.execution_id);
        let result = handle
            .wait_for_completion_with_timeout(self.segment_timeout)
            .await
            .map_err(|e| DispatchError::Wait {
                sync: sync.to_string(),
                execution_id,
                error: e.to_string(),
            })?;
        if result.status != WorkflowStatus::Completed {
            return Err(DispatchError::SegmentFailed {
                sync: sync.to_string(),
                execution_id,
                status: result.status,
                error: result.error_message.unwrap_or_default(),
            });
        }
        Ok(())
    }
}
