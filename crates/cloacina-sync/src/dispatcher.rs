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

/// Runs passes of syncs on one executor.
#[derive(Clone)]
pub struct Dispatcher {
    executor: Arc<dyn WorkflowExecutor>,
    dal: Arc<DAL>,
    ownership: Option<Arc<dyn ReactorOwnership>>,
    tenant: Option<String>,
    /// Ceiling on one execution's wait; `None` waits as long as it takes.
    segment_timeout: Option<Duration>,
}

impl Dispatcher {
    /// A dispatcher on an executor and its database. With no ownership, for
    /// an embedded runner or a single-replica server, the one process runs
    /// every sync.
    pub fn new(executor: Arc<dyn WorkflowExecutor>, dal: Arc<DAL>) -> Self {
        Self {
            executor,
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
        stop: tokio::sync::watch::Receiver<bool>,
    ) -> Result<PassOutcome, DispatchError> {
        let prepared = self.prepare(def).await?;
        if *stop.borrow() {
            self.release(&prepared).await;
            return Ok(PassOutcome::Stopped { segments: 0 });
        }
        let first = match self.start_segment(&prepared).await {
            Ok(h) => h,
            Err(e) => {
                self.release(&prepared).await;
                return Err(e);
            }
        };
        self.finish_pass(prepared, first.execution_id, stop).await
    }

    /// The first half of a pass: parse the definition, take the sync's lock,
    /// and wait for any outstanding execution. Followed by
    /// [`Self::start_segment`] and [`Self::finish_pass`]; a fire hook uses the
    /// halves so it can hand the first execution back while the pass goes on.
    pub async fn prepare(&self, def: &SyncDefinition) -> Result<PreparedPass, DispatchError> {
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
        let prepared = PreparedPass {
            sync,
            params,
            template,
            id,
        };
        if let Err(e) = self.finish_outstanding(&prepared.sync).await {
            self.release(&prepared).await;
            return Err(e);
        }
        Ok(prepared)
    }

    /// Start the next segment: one execution of the Sync workflow at the
    /// committed state. Returns its handle.
    pub async fn start_segment(
        &self,
        prepared: &PreparedPass,
    ) -> Result<WorkflowExecution, DispatchError> {
        if let Some(own) = &self.ownership {
            let believed = own.believed_owned().await;
            if !believed
                .iter()
                .any(|b| b.name == prepared.sync && b.domain == prepared.id.domain)
            {
                return Err(DispatchError::NotOwner(prepared.sync.clone()));
            }
        }
        let state = self
            .committed_state(&prepared.sync, &prepared.params)
            .await?;
        let mut ctx = prepared.template.clone_data();
        let state_json =
            serde_json::to_string(&state).map_err(|e| DispatchError::Params(e.to_string()))?;
        ctx.remove(keys::STATE);
        ctx.insert(keys::STATE, Value::String(state_json))
            .map_err(|e| DispatchError::Params(e.to_string()))?;
        let handle = self
            .executor
            .execute_async(WORKFLOW_NAME, ctx)
            .await
            .map_err(|e| DispatchError::Start {
                sync: prepared.sync.clone(),
                error: e.to_string(),
            })?;
        tracing::info!(sync = %prepared.sync, execution_id = %handle.execution_id, cursor = ?state.cursor, "sync: segment started");
        Ok(handle)
    }

    /// The rest of a pass after its first segment was started: wait for it,
    /// then segments until exhausted or stopped. Releases the lock at the end.
    pub async fn finish_pass(
        &self,
        prepared: PreparedPass,
        first: uuid::Uuid,
        mut stop: tokio::sync::watch::Receiver<bool>,
    ) -> Result<PassOutcome, DispatchError> {
        let outcome = self.segments_after(&prepared, first, &mut stop).await;
        self.release(&prepared).await;
        outcome
    }

    async fn segments_after(
        &self,
        prepared: &PreparedPass,
        first: uuid::Uuid,
        stop: &mut tokio::sync::watch::Receiver<bool>,
    ) -> Result<PassOutcome, DispatchError> {
        let sync = prepared.sync.as_str();
        let mut in_flight = first;
        let mut segments = 0u64;
        loop {
            self.wait(sync, in_flight).await?;
            segments += 1;
            let (_, exhausted) = self.committed_row(sync, &prepared.params).await?;
            if exhausted {
                tracing::info!(sync, segments, "sync: pass done, source exhausted");
                return Ok(PassOutcome::Exhausted { segments });
            }
            if *stop.borrow() {
                return Ok(PassOutcome::Stopped { segments });
            }
            in_flight = self.start_segment(prepared).await?.execution_id;
        }
    }

    async fn release(&self, prepared: &PreparedPass) {
        if let Some(own) = &self.ownership
            && let Err(e) = own.release(&prepared.id).await
        {
            tracing::warn!(sync = %prepared.sync, error = %e, "sync: ownership release failed");
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
            // An outstanding execution that fails is this pass's failure too:
            // its segment is not committed, and the next pass will redo it.
            self.wait(sync, ex.id.0).await?;
        }
        Ok(())
    }

    /// Wait for one execution to reach a terminal status, polling the executor
    /// the way the engine's own execution handle does.
    async fn wait(&self, sync: &str, execution_id: uuid::Uuid) -> Result<(), DispatchError> {
        let started = std::time::Instant::now();
        loop {
            if let Some(limit) = self.segment_timeout
                && started.elapsed() > limit
            {
                return Err(DispatchError::Wait {
                    sync: sync.to_string(),
                    execution_id: UniversalUuid(execution_id),
                    error: format!("segment exceeded {}s", limit.as_secs()),
                });
            }
            let status = self
                .executor
                .get_execution_status(execution_id)
                .await
                .map_err(|e| DispatchError::Wait {
                    sync: sync.to_string(),
                    execution_id: UniversalUuid(execution_id),
                    error: e.to_string(),
                })?;
            match status {
                WorkflowStatus::Completed => return Ok(()),
                WorkflowStatus::Failed | WorkflowStatus::Cancelled => {
                    let error = self
                        .dal
                        .workflow_execution()
                        .get_by_id(UniversalUuid(execution_id))
                        .await
                        .ok()
                        .and_then(|r| r.error_details)
                        .unwrap_or_default();
                    return Err(DispatchError::SegmentFailed {
                        sync: sync.to_string(),
                        execution_id: UniversalUuid(execution_id),
                        status,
                        error,
                    });
                }
                WorkflowStatus::Pending | WorkflowStatus::Running | WorkflowStatus::Paused => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}

/// A pass between [`Dispatcher::prepare`] and [`Dispatcher::finish_pass`]:
/// the parsed definition and the ownership id it holds.
pub struct PreparedPass {
    pub sync: String,
    pub params: SyncParams,
    template: Context<Value>,
    id: ReactorId,
}
