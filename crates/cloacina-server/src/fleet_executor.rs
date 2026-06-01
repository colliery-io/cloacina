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

//! Server-side `TaskExecutor` backend that pushes work to remote agents over
//! the substrate, awaits their report, and reconciles via the shared
//! [`cloacina::executor::TaskResultHandler`] so the thread executor and the
//! fleet executor produce **identical** state writes (CLOACI-T-0633).
//!
//! Real (T-0633 Tier B):
//! - **Context inlining**: the merged dependency context is resolved via the
//!   shared [`cloacina::executor::TaskContextBuilder`] — byte-for-byte the
//!   same logic the thread executor uses, so a fleet-run task sees exactly the
//!   input context an in-process run would.
//! - **Artifact ref**: resolved from `workflow_packages` (active = success +
//!   non-superseded) for the task's package, scoped to the agent's tenant.
//!
//! Remaining stub:
//! - **Capacity-aware selection**: currently picks the first
//!   `available_capacity > 0` agent. A later pass sorts by available_capacity
//!   and filters by tenant.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::dispatcher::{
    DispatchError, ExecutionResult, ExecutorMetrics, TaskExecutor, TaskReadyEvent,
};
use cloacina::error::{ExecutorError, TaskError};
use cloacina::executor::types::ClaimedTask;
use cloacina::executor::{TaskContextBuilder, TaskResultHandler};
use cloacina::fleet::{
    host_target_triple, AgentOutcome, AgentResultRequest, ArtifactRef, WorkPacket,
    AGENT_PROTOCOL_VERSION, AGENT_RECIPIENT_PREFIX, WORK_PACKET_KIND,
};
use cloacina::models::delivery_outbox::NewDeliveryOutbox;
use cloacina::retry::RetryPolicy;
use cloacina::Context;
use tracing::{debug, info, warn};

use crate::agent_registry::AgentRegistry;
use crate::fleet_coordinator::FleetCoordinator;

/// Default ceiling on how long the executor will wait for an agent to report
/// back. T-0633 Tier B will derive this from the task's `RetryPolicy`/
/// `ExecutorConfig::task_timeout`; v1 uses a single conservative cap so the
/// dispatcher slot doesn't pin forever on a silent agent.
const RESULT_WAIT_TIMEOUT: Duration = Duration::from_secs(300);

/// Baseline advertised capacity so an empty fleet still reports a value the
/// dispatcher can throttle against.
const MIN_ADVERTISED_CAPACITY: usize = 0;

pub struct FleetExecutor {
    dal: cloacina::dal::DAL,
    agent_registry: Arc<AgentRegistry>,
    coordinator: Arc<FleetCoordinator>,
    delivery_wake: cloacina::delivery::WakeHandle,
    result_handler: TaskResultHandler,
    /// Server-side `Runtime` (shared with the runner) used purely to
    /// introspect a task's `dependencies()` so the work packet can inline the
    /// merged dependency context. The agent — not the server — executes the
    /// cdylib; the server only needs the loaded task's dependency list.
    runtime: Arc<cloacina::Runtime>,
    /// Shared dependency-context resolver (T-0633) — identical logic to the
    /// thread executor's, so a fleet-run task sees the exact same input
    /// context it would running in-process.
    context_builder: TaskContextBuilder,
    /// Name surfaced via `TaskExecutor::name()`; defaults to `"fleet"` and
    /// matches the dispatcher-routing executor key operators target.
    name: String,
}

impl FleetExecutor {
    pub fn new(
        dal: cloacina::dal::DAL,
        agent_registry: Arc<AgentRegistry>,
        coordinator: Arc<FleetCoordinator>,
        delivery_wake: cloacina::delivery::WakeHandle,
        result_handler: TaskResultHandler,
        runtime: Arc<cloacina::Runtime>,
    ) -> Self {
        let context_builder = TaskContextBuilder::new(dal.clone());
        Self {
            dal,
            agent_registry,
            coordinator,
            delivery_wake,
            result_handler,
            runtime,
            context_builder,
            name: "fleet".to_string(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Route a pre-dispatch failure (namespace parse, task-not-loaded,
    /// context-build failure, artifact-not-found) through the shared
    /// `TaskResultHandler` so the fleet's failure/retry bookkeeping is
    /// identical to the thread path.
    async fn reconcile_error(
        &self,
        event: &TaskReadyEvent,
        claimed_task: &ClaimedTask,
        retry_policy: &RetryPolicy,
        duration: Duration,
        message: String,
    ) -> ExecutionResult {
        let err = ExecutorError::TaskExecution(TaskError::ExecutionFailed {
            message,
            task_id: event.task_name.clone(),
            timestamp: chrono::Utc::now(),
        });
        self.result_handler
            .handle_outcome(event, claimed_task, Err(err), retry_policy, duration)
            .await
    }
}

#[async_trait]
impl TaskExecutor for FleetExecutor {
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
        let start = Instant::now();
        let claimed_task = ClaimedTask {
            task_execution_id: event.task_execution_id,
            workflow_execution_id: event.workflow_execution_id,
            task_name: event.task_name.clone(),
            attempt: event.attempt,
        };
        // T-0633 follow-on: derive the real RetryPolicy from the loaded task.
        // v1 uses the engine default — retry per the standard policy on failure.
        let retry_policy = RetryPolicy::default();

        // ── 1. Parse the task namespace first — its tenant is the authoritative
        //       work tenant (NOT the agent's), and we need it to select a
        //       tenant-matching agent in step 2.
        let namespace = match cloacina::parse_namespace(&event.task_name) {
            Ok(ns) => ns,
            Err(e) => {
                return Ok(self
                    .reconcile_error(
                        &event,
                        &claimed_task,
                        &retry_policy,
                        start.elapsed(),
                        format!("parse_namespace({}): {}", event.task_name, e),
                    )
                    .await);
            }
        };
        // The namespace tenant is "public" for non-tenant-scoped work; map that
        // onto the agent roster's `Option<String>` tenant (None == public) so
        // selection + reclaim agree on what "same tenant" means.
        let task_tenant: Option<String> = if namespace.tenant_id == "public" {
            None
        } else {
            Some(namespace.tenant_id.clone())
        };

        // ── 2. Select a live agent: same tenant as the task, with capacity,
        //       greedy on most-free-capacity (so load spreads). Tenant isolation
        //       (REQ-008): an agent only ever receives work in its tenant scope.
        let snapshot = self.agent_registry.snapshot();
        let Some(agent) = snapshot
            .iter()
            .filter(|a| a.available_capacity > 0 && a.tenant_id == task_tenant)
            .max_by_key(|a| a.available_capacity)
        else {
            warn!(
                task_id = %event.task_execution_id,
                tenant = ?task_tenant,
                "fleet: no live agent with capacity in the task's tenant"
            );
            return Ok(self
                .reconcile_error(
                    &event,
                    &claimed_task,
                    &retry_policy,
                    start.elapsed(),
                    format!("no available fleet agent in tenant {:?}", task_tenant),
                )
                .await);
        };
        let agent_id = agent.agent_id.clone();
        let tenant_id = agent.tenant_id.clone();

        // ── 3. Resolve the task's dependencies from the server Runtime, then
        //       build the merged dependency context via the SHARED
        //       TaskContextBuilder so the agent receives exactly the input
        //       context a thread run would produce.
        let dependencies: Vec<cloacina::task::TaskNamespace> =
            match self.runtime.get_task(&namespace) {
                Some(task) => task.dependencies().to_vec(),
                None => {
                    return Ok(self
                        .reconcile_error(
                            &event,
                            &claimed_task,
                            &retry_policy,
                            start.elapsed(),
                            format!(
                                "task `{}` not loaded in server runtime — cannot resolve \
                                 dependency context for fleet dispatch",
                                event.task_name
                            ),
                        )
                        .await);
                }
            };
        let context = match self
            .context_builder
            .build(&claimed_task, &dependencies)
            .await
        {
            Ok(c) => c,
            Err(e) => {
                return Ok(self
                    .reconcile_error(
                        &event,
                        &claimed_task,
                        &retry_policy,
                        start.elapsed(),
                        format!("dependency context build failed: {}", e),
                    )
                    .await);
            }
        };
        let context_value = context_to_json(&context);

        // ── 3. Resolve the artifact digest from workflow_packages for the
        //       task's package within the agent's tenant scope. The success +
        //       non-superseded filters are load-bearing (see DAL doc) — a
        //       wrong row would route a stale/unbuilt cdylib to the agent.
        let digest = match self
            .dal
            .workflow_packages()
            .get_active_content_hash_for_package(&namespace.package_name, tenant_id.as_deref())
            .await
        {
            Ok(Some(h)) => h,
            Ok(None) => {
                return Ok(self
                    .reconcile_error(
                        &event,
                        &claimed_task,
                        &retry_policy,
                        start.elapsed(),
                        format!(
                            "no active (success, non-superseded) artifact for package `{}` \
                             in tenant {:?}",
                            namespace.package_name, tenant_id
                        ),
                    )
                    .await);
            }
            Err(e) => {
                return Ok(self
                    .reconcile_error(
                        &event,
                        &claimed_task,
                        &retry_policy,
                        start.elapsed(),
                        format!("artifact digest lookup failed: {}", e),
                    )
                    .await);
            }
        };

        // ── 4. Build the work packet with real context + real artifact ref.
        let packet = WorkPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            task_execution_id: event.task_execution_id.0.to_string(),
            workflow_execution_id: event.workflow_execution_id.0.to_string(),
            task_name: event.task_name.clone(),
            attempt: event.attempt,
            context: context_value,
            artifact: ArtifactRef {
                fetch_url: format!("/v1/agent/artifact/{}", digest),
                digest,
                build_target_triple: host_target_triple(),
            },
            timeout_seconds: 300,
            tenant_id: tenant_id.clone(),
        };
        let payload_bytes = match serde_json::to_vec(&packet) {
            Ok(b) => b,
            Err(e) => {
                return Err(DispatchError::ExecutionFailed(format!(
                    "serialize WorkPacket: {}",
                    e
                )));
            }
        };

        // ── 5. Register the rendezvous BEFORE enqueueing so a fast agent
        //       reporting back can't race past the receiver.
        let rx = self.coordinator.register_pending(event.task_execution_id);

        // ── 6. Enqueue into the substrate outbox + wake the relay.
        let recipient = format!("{}{}", AGENT_RECIPIENT_PREFIX, agent_id);
        let row = NewDeliveryOutbox {
            recipient,
            kind: WORK_PACKET_KIND.to_string(),
            tenant_id: tenant_id.clone(),
            payload: payload_bytes,
        };
        if let Err(e) = self.dal.delivery_outbox().enqueue(row).await {
            self.coordinator.cancel(event.task_execution_id);
            warn!(
                task_id = %event.task_execution_id,
                agent_id = %agent_id,
                error = %e,
                "fleet: delivery_outbox enqueue failed"
            );
            return Ok(self
                .reconcile_error(
                    &event,
                    &claimed_task,
                    &retry_policy,
                    start.elapsed(),
                    format!("delivery_outbox enqueue: {}", e),
                )
                .await);
        }
        self.delivery_wake.wake();
        debug!(
            task_id = %event.task_execution_id,
            agent_id = %agent_id,
            "fleet: work packet enqueued + relay woken"
        );

        // ── 7. Await the agent's result.
        let result_req: AgentResultRequest =
            match tokio::time::timeout(RESULT_WAIT_TIMEOUT, rx).await {
                Ok(Ok(req)) => req,
                Ok(Err(_)) => {
                    // Sender dropped — coordinator cleared (e.g. server shutdown).
                    warn!(
                        task_id = %event.task_execution_id,
                        "fleet: result rendezvous canceled before agent reported"
                    );
                    return Ok(self
                        .reconcile_error(
                            &event,
                            &claimed_task,
                            &retry_policy,
                            start.elapsed(),
                            "fleet result rendezvous canceled".to_string(),
                        )
                        .await);
                }
                Err(_) => {
                    // Timeout waiting for the agent. Cancel the rendezvous so a
                    // late report is reported as orphan + dropped.
                    self.coordinator.cancel(event.task_execution_id);
                    warn!(
                        task_id = %event.task_execution_id,
                        agent_id = %agent_id,
                        elapsed_s = ?RESULT_WAIT_TIMEOUT.as_secs(),
                        "fleet: agent result wait exceeded server-side timeout"
                    );
                    return Ok(self
                        .result_handler
                        .handle_outcome(
                            &event,
                            &claimed_task,
                            Err(ExecutorError::TaskTimeout),
                            &retry_policy,
                            start.elapsed(),
                        )
                        .await);
                }
            };

        // ── 8. Map AgentOutcome → Result<Context, ExecutorError>.
        let outcome: Result<Context<serde_json::Value>, ExecutorError> = match result_req.outcome {
            AgentOutcome::Success { context } => match value_to_context(context) {
                Ok(c) => Ok(c),
                Err(e) => Err(ExecutorError::TaskExecution(TaskError::ExecutionFailed {
                    message: format!("invalid Success.context from agent: {}", e),
                    task_id: event.task_name.clone(),
                    timestamp: chrono::Utc::now(),
                })),
            },
            AgentOutcome::Failure {
                message,
                classification,
            } => Err(ExecutorError::TaskExecution(TaskError::ExecutionFailed {
                message: format!("agent failure ({:?}): {}", classification, message),
                task_id: event.task_name.clone(),
                timestamp: chrono::Utc::now(),
            })),
            AgentOutcome::Refused { reason, message } => {
                // Refusals are transient (target-triple mismatch, artifact
                // fetch failure, agent shutdown → reschedule). Map onto
                // TaskExecution; the retry policy decides whether to retry.
                Err(ExecutorError::TaskExecution(TaskError::ExecutionFailed {
                    message: format!("agent refused ({:?}): {}", reason, message),
                    task_id: event.task_name.clone(),
                    timestamp: chrono::Utc::now(),
                }))
            }
        };

        // ── 9. Reconcile through the shared handler (T-0630). This is the
        //       whole point of the fleet design: agent and thread paths
        //       converge here so state writes / retry / context-persist are
        //       identical by construction.
        info!(
            task_id = %event.task_execution_id,
            agent_id = %agent_id,
            ok = outcome.is_ok(),
            "fleet: agent reported; reconciling via shared TaskResultHandler"
        );
        Ok(self
            .result_handler
            .handle_outcome(
                &event,
                &claimed_task,
                outcome,
                &retry_policy,
                start.elapsed(),
            )
            .await)
    }

    fn has_capacity(&self) -> bool {
        self.agent_registry
            .snapshot()
            .iter()
            .any(|a| a.available_capacity > 0)
    }

    fn metrics(&self) -> ExecutorMetrics {
        let snap = self.agent_registry.snapshot();
        let total_max: u32 = snap.iter().map(|a| a.max_concurrency).sum();
        let total_in_flight: u32 = snap.iter().map(|a| a.in_flight).sum();
        ExecutorMetrics {
            active_tasks: total_in_flight as usize,
            max_concurrent: (total_max as usize).max(MIN_ADVERTISED_CAPACITY),
            total_executed: 0,
            total_failed: 0,
            avg_duration_ms: 0,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Convert a JSON Object (or Null) into a `Context<serde_json::Value>` to feed
/// the shared `TaskResultHandler` Success path.
fn value_to_context(value: serde_json::Value) -> Result<Context<serde_json::Value>, anyhow::Error> {
    use anyhow::{anyhow, bail};
    let mut ctx = Context::<serde_json::Value>::new();
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                ctx.insert(k.as_str(), v)
                    .map_err(|e| anyhow!("insert {}: {}", k, e))?;
            }
            Ok(ctx)
        }
        serde_json::Value::Null => Ok(ctx),
        other => bail!(
            "AgentOutcome::Success.context must be a JSON object or null, got {}",
            kind_of(&other)
        ),
    }
}

/// Materialize a `Context<serde_json::Value>` into a JSON object for the work
/// packet's `context` field (inverse of `value_to_context`).
fn context_to_json(ctx: &Context<serde_json::Value>) -> serde_json::Value {
    serde_json::Value::Object(
        ctx.data()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

fn kind_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// Silence unused warning on UniversalUuid if later changes drop a use.
const _: fn(UniversalUuid) = |_| {};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_to_context_round_trips_an_object() {
        let v = serde_json::json!({"a": 1, "b": "two"});
        let ctx = value_to_context(v.clone()).unwrap();
        assert_eq!(ctx.get("a"), Some(&serde_json::json!(1)));
        assert_eq!(ctx.get("b"), Some(&serde_json::json!("two")));
    }

    #[test]
    fn value_to_context_accepts_null_as_empty() {
        let ctx = value_to_context(serde_json::Value::Null).unwrap();
        assert!(ctx.data().is_empty());
    }

    #[test]
    fn value_to_context_rejects_array() {
        let v = serde_json::json!([1, 2, 3]);
        assert!(value_to_context(v).is_err());
    }

    #[test]
    fn context_to_json_round_trips_through_value_to_context() {
        let v = serde_json::json!({"a": 1, "b": "two"});
        let ctx = value_to_context(v.clone()).unwrap();
        assert_eq!(context_to_json(&ctx), v);
    }
}
