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

use crate::agent_registry::{AgentRecord, AgentRegistry};
use crate::fleet_coordinator::FleetCoordinator;

/// Default ceiling on how long the executor will wait for an agent to report
/// back. T-0633 Tier B will derive this from the task's `RetryPolicy`/
/// `ExecutorConfig::task_timeout`; v1 uses a single conservative cap so the
/// dispatcher slot doesn't pin forever on a silent agent.
const RESULT_WAIT_TIMEOUT: Duration = Duration::from_secs(300);

/// Baseline advertised capacity so an empty fleet still reports a value the
/// dispatcher can throttle against.
const MIN_ADVERTISED_CAPACITY: usize = 0;

/// How often the executor refreshes its task claim while waiting for the agent
/// to report. Must stay well under the stale-claim sweeper threshold (60s) so a
/// long-running fleet task isn't reclaimed mid-flight and re-dispatched.
const FLEET_CLAIM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);

pub struct FleetExecutor {
    dal: cloacina::dal::DAL,
    /// CLOACI-T-0781: the ADMIN (public-schema) DAL, used ONLY for the
    /// delivery_outbox enqueue. Delivery is server-global — one relay + one WS
    /// sink drain the admin schema's outbox and the admin NOTIFY wakes them — so
    /// a per-tenant runner's WorkPackets must land in the admin outbox (tenant-
    /// tagged) to reach the agents. For the global runner this equals `dal`.
    outbox_dal: cloacina::dal::DAL,
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
    /// Per-executor identity for the atomic task claim. The scheduler
    /// over-selects Ready tasks (and for the `public` tenant the global +
    /// per-tenant runners both poll the same rows), relying on the executor to
    /// dedupe via `claim_for_runner` — the same mechanism ThreadTaskExecutor
    /// uses. Without it a fleet task outliving one poll is re-dispatched.
    instance_id: UniversalUuid,
}

impl FleetExecutor {
    pub fn new(
        dal: cloacina::dal::DAL,
        outbox_dal: cloacina::dal::DAL,
        agent_registry: Arc<AgentRegistry>,
        coordinator: Arc<FleetCoordinator>,
        delivery_wake: cloacina::delivery::WakeHandle,
        result_handler: TaskResultHandler,
        runtime: Arc<cloacina::Runtime>,
    ) -> Self {
        let context_builder = TaskContextBuilder::new(dal.clone());
        Self {
            dal,
            outbox_dal,
            agent_registry,
            coordinator,
            delivery_wake,
            result_handler,
            runtime,
            context_builder,
            name: "fleet".to_string(),
            instance_id: UniversalUuid::new_v4(),
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
        let task_execution_id = event.task_execution_id;

        // ── Atomically claim the task before any dispatch work. The scheduler
        //    over-selects Ready tasks (`get_ready_for_retry` is SELECT-only) and
        //    relies on the executor to dedupe — exactly as ThreadTaskExecutor
        //    does. For the `public` tenant the global + per-tenant runners both
        //    poll the same rows; and any fleet task outliving one ~100ms poll
        //    would otherwise be re-dispatched every tick, each dispatch
        //    clobbering the rendezvous → "result rendezvous canceled". The claim
        //    makes exactly one invocation own the task.
        use cloacina::dal::unified::task_execution::RunnerClaimResult;
        let claimed = match self
            .dal
            .task_execution()
            .claim_for_runner(task_execution_id, self.instance_id)
            .await
        {
            Ok(RunnerClaimResult::Claimed) => true,
            Ok(RunnerClaimResult::AlreadyClaimed) => {
                debug!(
                    task_id = %task_execution_id,
                    "fleet: task already claimed by another runner — skipping"
                );
                return Ok(ExecutionResult::skipped(task_execution_id));
            }
            Err(e) => {
                warn!(
                    task_id = %task_execution_id,
                    error = %e,
                    "fleet: claim failed — proceeding without claim"
                );
                false
            }
        };

        // Surface the workflow execution as Running while the agent works it.
        // The fleet path otherwise leaves it Pending until the agent reports
        // (the claim sets claimed_by, not status; nothing else marks the
        // execution Running on dispatch), so `execution status` would read
        // Pending for the whole run. We only reach here once per task (re-
        // dispatches short-circuit at AlreadyClaimed above), and the scheduler
        // only dispatches tasks for active (Pending/Running) executions, so this
        // is always a Pending→Running (or idempotent Running→Running) move.
        if let Err(e) = self
            .dal
            .workflow_execution()
            .update_status(event.workflow_execution_id, "Running")
            .await
        {
            warn!(
                workflow_id = %event.workflow_execution_id,
                error = %e,
                "fleet: failed to mark workflow execution Running"
            );
        }

        // Heartbeat the claim for the lifetime of the dispatch so the stale-claim
        // sweeper (60s) doesn't reclaim a task while the agent is still running it
        // (the result wait can be up to RESULT_WAIT_TIMEOUT).
        let heartbeat = if claimed {
            let dal = self.dal.clone();
            let runner_id = self.instance_id;
            Some(tokio::spawn(async move {
                let mut ticker = tokio::time::interval(FLEET_CLAIM_HEARTBEAT_INTERVAL);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                loop {
                    ticker.tick().await;
                    if let Err(e) = dal
                        .task_execution()
                        .heartbeat(task_execution_id, runner_id)
                        .await
                    {
                        warn!(task_id = %task_execution_id, error = %e, "fleet: claim heartbeat error");
                    }
                }
            }))
        } else {
            None
        };

        // Run the dispatch body in an inner async block so the claim is released
        // on EVERY exit path (the body has many early `return`s) without
        // duplicating the release. `return` inside the block returns the block's
        // value, not the function's.
        let result: Result<ExecutionResult, DispatchError> = async {
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
            // CLOACI-T-0817: "public" is now a first-class named tenant in the
            // WORK/AGENT realm — public agents register as `Some("public")` (the
            // actuator/demo mint a public-tenant-scoped key, never the bootstrap
            // `None` key). So the namespace tenant maps directly onto the agent
            // roster's `Option<String>` tenant: "public" -> `Some("public")`,
            // every named tenant -> `Some(<tenant>)`. The old `None == public`
            // duality is retired (that `None` belonged to the admin/bootstrap key,
            // which is a different realm entirely).
            let task_tenant: Option<String> = Some(namespace.tenant_id.clone());

            // ── 2. Select a live agent: same tenant as the task, with capacity,
            //       AND an arch this package has a cdylib for. Greedy on most-free-
            //       capacity (so load spreads). Tenant isolation (REQ-008): an agent
            //       only ever receives work in its tenant scope.
            //
            //       CLOACI-T-0780: the runnable arches are the host primary (built
            //       for host_target_triple()) ∪ this package's per-target builds.
            //       Filtering selection by this means we never hand an agent a
            //       package it has no cdylib for — so it can't fail-closed refuse,
            //       which would otherwise churn retries and log noise. A genuinely
            //       wrong-arch agent simply isn't eligible; the task waits for one
            //       that is (the no-eligible-agent path below), exactly as it would
            //       if no agent had capacity.
            // Resolve the package's PRIMARY artifact (digest + language) up front —
            // the language decides arch eligibility. CLOACI-T-0781: `self.dal` is
            // schema-scoped and tenant-schema packages carry tenant_id = NULL, so
            // look up by None (IS NULL).
            let (primary_digest, language) = match self
                .dal
                .workflow_packages()
                .get_active_dispatch_for_package(&namespace.package_name, None)
                .await
            {
                Ok(Some(info)) => info,
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
                                namespace.package_name, task_tenant
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

            // CLOACI-T-0780: an INTERPRETED package (e.g. Python) has no arch-specific
            // cdylib — it runs from its source on ANY agent. A COMPILED package runs
            // only where a cdylib exists for that arch: the host primary ∪ its
            // per-target builds. `None` = "any arch is fine" (interpreted).
            let interpreted = language.eq_ignore_ascii_case("python");
            let runnable_triples: Option<Vec<String>> = if interpreted {
                None
            } else {
                let mut t = self
                    .dal
                    .workflow_packages()
                    .get_artifact_triples_for_package(&namespace.package_name)
                    .await
                    .unwrap_or_default();
                t.push(host_target_triple().to_string());
                Some(t)
            };
            let snapshot = self.agent_registry.snapshot();
            let Some(agent) = select_fleet_agent(&snapshot, &task_tenant, &runnable_triples) else {
                warn!(
                    task_id = %event.task_execution_id,
                    tenant = ?task_tenant,
                    "fleet: no live agent with capacity + a runnable arch in the task's tenant"
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
            // CLOACI-T-0780: the selected agent's target triple — dispatch hands it
            // the cdylib built for THIS arch when one exists (else the primary).
            let agent_triple = agent.target_triple.clone();

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

            // ── 3b. CLOACI-T-0780 (multi-arch): resolve the cdylib digest + the
            //        triple to stamp. An INTERPRETED package runs on the selected
            //        agent via its interpreter regardless of arch — stamp the agent's
            //        OWN triple so the fail-closed guard is a no-op. A COMPILED
            //        package gets the cdylib built for the agent's arch when one
            //        exists, else the primary/host build (which only a host-arch agent
            //        matches — and selection already guaranteed the agent is runnable).
            let (digest, build_target_triple) = if interpreted {
                (primary_digest, agent_triple.clone())
            } else {
                match self
                    .dal
                    .workflow_packages()
                    .get_artifact_digest_for_target(&namespace.package_name, &agent_triple)
                    .await
                {
                    Ok(Some(arch_digest)) => {
                        tracing::debug!(
                            package = %namespace.package_name,
                            triple = %agent_triple,
                            digest = %arch_digest,
                            "fleet: dispatching per-target artifact (CLOACI-T-0780)"
                        );
                        (arch_digest, agent_triple.clone())
                    }
                    _ => (primary_digest, host_target_triple()),
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
                    build_target_triple,
                },
                timeout_seconds: 300,
                tenant_id: tenant_id.clone(),
                language: Some(language),
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
            if let Err(e) = self.outbox_dal.delivery_outbox().enqueue(row).await {
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

            // Stamp started_at now that the packet is committed to a specific
            // agent — the fleet's execution-start moment (the slot is taken).
            // The agent is DB-less and the claim only records the owner, not
            // started_at, so without this the per-task timeline (Gantt) has no
            // real start offset. Idempotent (no-op if already set) + best-effort.
            if let Err(e) = self
                .dal
                .task_execution()
                .mark_started(event.task_execution_id)
                .await
            {
                warn!(
                    task_id = %event.task_execution_id,
                    error = %e,
                    "fleet: failed to stamp task started_at"
                );
            }
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
            // CLOACI-T-0780: a refusal is an expected fail-closed reschedule, not a
            // failure — keep its reconcile log quiet (debug), unlike a real failure.
            let was_refused = matches!(result_req.outcome, AgentOutcome::Refused { .. });
            let outcome: Result<Context<serde_json::Value>, ExecutorError> =
                match result_req.outcome {
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
            if was_refused {
                tracing::debug!(
                    task_id = %event.task_execution_id,
                    agent_id = %agent_id,
                    "fleet: agent refused; rescheduling (fail-closed guard)"
                );
            } else {
                info!(
                    task_id = %event.task_execution_id,
                    agent_id = %agent_id,
                    ok = outcome.is_ok(),
                    "fleet: agent reported; reconciling via shared TaskResultHandler"
                );
            }
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
        .await;

        // Stop heartbeating and release the claim on EVERY exit path so retries
        // + dead-agent reclaim can re-claim the row. No-op if we never claimed.
        if let Some(h) = heartbeat {
            h.abort();
        }
        if claimed {
            if let Err(e) = self
                .dal
                .task_execution()
                .release_runner_claim(task_execution_id)
                .await
            {
                warn!(task_id = %task_execution_id, error = %e, "fleet: release claim failed");
            }
        }
        result
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

/// Select a live fleet agent for a task: same tenant as the work, with spare
/// capacity, AND a target triple this package has a cdylib for. Greedy on
/// most-free-capacity so load spreads.
///
/// Tenant isolation (REQ-008) lives here: `a.tenant_id == *task_tenant` is the
/// only cross-tenant gate, so an agent only ever receives work in its own tenant
/// scope. CLOACI-T-0817: "public" is a real tenant — public work carries
/// `Some("public")` and matches only `Some("public")` agents; a named tenant's
/// work matches only that tenant's agents. `runnable_triples == None` means
/// "any arch is fine" (interpreted package, e.g. Python).
fn select_fleet_agent<'a>(
    snapshot: &'a [AgentRecord],
    task_tenant: &Option<String>,
    runnable_triples: &Option<Vec<String>>,
) -> Option<&'a AgentRecord> {
    snapshot
        .iter()
        .filter(|a| {
            a.available_capacity > 0
                && &a.tenant_id == task_tenant
                && runnable_triples
                    .as_ref()
                    .map_or(true, |ts| ts.iter().any(|t| t == &a.target_triple))
        })
        .max_by_key(|a| a.available_capacity)
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

    // ── Agent selection / tenant isolation (CLOACI-T-0817) ──────────────────

    fn agent(id: &str, cap: u32, tenant: Option<&str>) -> AgentRecord {
        AgentRecord {
            agent_id: id.to_string(),
            max_concurrency: cap,
            in_flight: 0,
            available_capacity: cap,
            target_triple: "aarch64-apple-darwin".to_string(),
            capabilities: vec![],
            last_heartbeat: std::time::Instant::now(),
            tenant_id: tenant.map(str::to_string),
        }
    }

    /// CLOACI-T-0817: public-namespace work (`task_tenant = Some("public")`,
    /// which is what the `task_tenant` mapping now produces for the "public"
    /// namespace) selects a `Some("public")` agent — and never a named tenant's
    /// agent nor the bootstrap/admin `None` agent.
    #[test]
    fn public_work_selects_a_public_agent_only() {
        let roster = vec![
            agent("acme-1", 16, Some("acme")),
            agent("public-1", 8, Some("public")),
            agent("admin-1", 32, None), // bootstrap/admin-keyed agent
        ];
        let chosen = select_fleet_agent(&roster, &Some("public".to_string()), &None)
            .expect("a public agent must be selectable for public work");
        assert_eq!(chosen.agent_id, "public-1");
    }

    /// A named tenant's work matches only that tenant's agents — isolation is
    /// preserved in both directions: public agents are not eligible for `acme`
    /// work, and vice versa.
    #[test]
    fn named_tenant_work_is_isolated_from_public_and_others() {
        let roster = vec![
            agent("public-1", 32, Some("public")),
            agent("acme-1", 4, Some("acme")),
            agent("beta-1", 64, Some("beta")),
        ];
        // acme work -> only the acme agent, even though others have far more
        // free capacity (the greedy selector must not cross the tenant gate).
        let chosen = select_fleet_agent(&roster, &Some("acme".to_string()), &None)
            .expect("acme work must select the acme agent");
        assert_eq!(chosen.agent_id, "acme-1");

        // A tenant with no agent in the roster gets nothing — never a fallback
        // to public or another tenant.
        assert!(
            select_fleet_agent(&roster, &Some("gamma".to_string()), &None).is_none(),
            "work for a tenant with no agents must not leak onto another tenant"
        );
    }

    /// Public work must NOT fall back to a bootstrap/admin `None`-tenant agent
    /// now that the `None == public` duality is retired.
    #[test]
    fn public_work_does_not_select_a_none_tenant_agent() {
        let roster = vec![agent("admin-1", 32, None)];
        assert!(
            select_fleet_agent(&roster, &Some("public".to_string()), &None).is_none(),
            "a None-tenant (bootstrap/admin) agent must not serve public work"
        );
    }
}
