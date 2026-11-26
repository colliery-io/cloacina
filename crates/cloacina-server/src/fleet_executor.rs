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
    AGENT_PROTOCOL_VERSION, AGENT_RECIPIENT_PREFIX, AgentOutcome, AgentResultRequest, ArtifactRef,
    FailureClassification, WORK_PACKET_KIND, WorkPacket, WrappedSecret, host_target_triple,
};
use cloacina::models::delivery_outbox::NewDeliveryOutbox;
use cloacina::retry::RetryPolicy;
use cloacina::security::{decode_pool_public_key, resolve_and_wrap_secrets, secret_ref_names};
use cloacina::task::TaskNamespace;
use cloacina::{Context, SecretResolver};
use tracing::{debug, info, warn};

use crate::agent_registry::{AgentRegistry, select_fleet_agent};
use crate::fleet_coordinator::FleetCoordinator;

/// How long the server waits for the agent's report past the task's time
/// limit (CLOACINA-T-1014). The agent starts its clock when the task starts,
/// after it took the packet and loaded the package, and its report needs a
/// moment to arrive. Without the grace the server would give up on a task at
/// the same moment the agent reports its timeout.
pub(crate) const RESULT_WAIT_GRACE: Duration = Duration::from_secs(30);

/// The time limits of a fleet task (CLOACINA-T-1014).
///
/// A task with a time limit of its own (`#[task(timeout_seconds = N)]`,
/// `@task(timeout_seconds=N)`) gets that limit; any other task gets
/// `default`. No task gets more than `max`. The limit goes to the agent in the
/// work packet, and the server waits for the result that long plus a short
/// grace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetTaskTimeLimits {
    /// The limit of a task that sets none.
    pub default: Duration,
    /// The longest limit a task can get.
    pub max: Duration,
}

impl FleetTaskTimeLimits {
    /// Seconds a task with no limit of its own gets.
    pub const DEFAULT_SECONDS: u64 = 300;
    /// The longest limit a task can ask for, in seconds (24 hours).
    pub const DEFAULT_MAX_SECONDS: u64 = 86_400;

    /// Build the limits from the server's settings. Fails on a value the
    /// server cannot run with, so a bad setting stops the start and does not
    /// show up later as tasks that time out at once.
    pub fn from_seconds(default_seconds: u64, max_seconds: u64) -> Result<Self, String> {
        if default_seconds == 0 || max_seconds == 0 {
            return Err("the fleet task time limits must be greater than 0 seconds".to_string());
        }
        if max_seconds > u32::MAX as u64 {
            return Err(format!(
                "the maximum fleet task time limit ({max_seconds} s) is above {} s",
                u32::MAX
            ));
        }
        if default_seconds > max_seconds {
            return Err(format!(
                "the default fleet task time limit ({default_seconds} s) is above the \
                 maximum ({max_seconds} s)"
            ));
        }
        Ok(Self {
            default: Duration::from_secs(default_seconds),
            max: Duration::from_secs(max_seconds),
        })
    }

    /// The limit one task gets: its own or the default, capped at the maximum,
    /// in whole seconds (a part of a second counts as one second).
    pub(crate) fn limit_for(&self, own: Option<Duration>) -> TaskTimeLimit {
        let asked = own.unwrap_or(self.default);
        let capped = asked > self.max;
        let limit = if capped { self.max } else { asked };
        let mut seconds = limit.as_secs();
        if limit.subsec_nanos() > 0 {
            seconds += 1;
        }
        let seconds = seconds.clamp(1, u32::MAX as u64) as u32;
        TaskTimeLimit {
            seconds,
            capped_from: capped.then_some(asked),
        }
    }
}

impl Default for FleetTaskTimeLimits {
    fn default() -> Self {
        Self {
            default: Duration::from_secs(Self::DEFAULT_SECONDS),
            max: Duration::from_secs(Self::DEFAULT_MAX_SECONDS),
        }
    }
}

/// The time limit of one dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaskTimeLimit {
    /// What the work packet carries and the agent enforces.
    pub(crate) seconds: u32,
    /// The limit the task asked for, when it was above the maximum.
    pub(crate) capped_from: Option<Duration>,
}

impl TaskTimeLimit {
    /// How long the server waits for the agent's report.
    pub(crate) fn result_wait(&self) -> Duration {
        Duration::from_secs(self.seconds as u64) + RESULT_WAIT_GRACE
    }
}

/// The error the shared result handler gets for an agent's failure report.
///
/// The task's own error text is kept inside the message, because a retry
/// condition (`ErrorPattern`, `TransientOnly`) is a test on that text. A task
/// that ran past its time limit on the agent is a timeout, the same as on the
/// in-process executor.
fn agent_failure_error(
    task_name: &str,
    message: &str,
    classification: FailureClassification,
) -> ExecutorError {
    if classification == FailureClassification::Timeout {
        return ExecutorError::TaskTimeout;
    }
    ExecutorError::TaskExecution(TaskError::ExecutionFailed {
        message: format!("agent failure ({:?}): {}", classification, message),
        task_id: task_name.to_string(),
        timestamp: chrono::Utc::now(),
    })
}

/// How long a task waits before the next dispatch try when no agent could
/// take it (CLOACINA-T-1012). Short enough that a freed slot or a new agent is
/// used soon; long enough that a queue of waiting tasks does not read the
/// roster on every scheduler tick.
const NO_CAPACITY_RETRY_DELAY: Duration = Duration::from_secs(2);

/// Baseline advertised capacity so an empty fleet still reports a value the
/// dispatcher can throttle against.
const MIN_ADVERTISED_CAPACITY: usize = 0;

/// How often the executor refreshes its task claim while waiting for the agent
/// to report. Must stay well under the stale-claim sweeper threshold (60s) so a
/// long-running fleet task isn't reclaimed mid-flight and re-dispatched.
const FLEET_CLAIM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);

/// CLOACI-T-0861 / I-0133 (D-2/D-5) — builds the grant-gated secret resolver a
/// fleet dispatch uses to resolve + HPKE-wrap a task's secrets.
///
/// This is the seam between the fleet dispatch path (which is fully wired: extract
/// the task's `$secret` names, consume a one-time pooled key, wrap, attach) and
/// the tenant secrets subsystem (the tenant→`org_id` mapping, the server KEK, and
/// the package's `ResolvedGrants` → `SecretAllow`). A concrete impl belongs to
/// the (not-yet-built) server secrets subsystem; when absent, secret-referencing
/// fleet tasks fail closed at dispatch (never a plaintext leak).
#[async_trait]
pub trait FleetSecretResolverFactory: Send + Sync {
    /// Build a fail-closed, grant-gated [`SecretResolver`] scoped to `tenant` +
    /// `package`. `None` ⇒ this deployment cannot resolve secrets for that scope
    /// (secrets not configured, or no grants) — the dispatch then fails closed.
    async fn resolver_for(&self, tenant: &str, package: &str) -> Option<Arc<dyn SecretResolver>>;
}

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
    /// CLOACI-T-0861 / D-5 — builds the grant-gated resolver used to resolve +
    /// HPKE-wrap a task's secrets to a one-time pooled agent key. `None` on a
    /// deployment with no secrets subsystem wired (the common case today): a task
    /// that references secrets then fails closed at dispatch. See
    /// [`FleetSecretResolverFactory`].
    secret_resolver_factory: Option<Arc<dyn FleetSecretResolverFactory>>,
    /// CLOACINA-T-1014: the default and the maximum time limit of a task.
    time_limits: FleetTaskTimeLimits,
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
        let instance_id = UniversalUuid::new_v4();
        Self {
            dal,
            outbox_dal,
            agent_registry,
            coordinator,
            delivery_wake,
            // CLOACINA-T-1013, CLOACINA-A-0017: the terminal writes of a result apply only while
            // this executor holds the claim on the task. A result that comes
            // back after the claim moved (a stale-claim sweep, another
            // replica) changes no state.
            result_handler: result_handler.with_runner_id(instance_id),
            runtime,
            context_builder,
            name: "fleet".to_string(),
            instance_id,
            secret_resolver_factory: None,
            time_limits: FleetTaskTimeLimits::default(),
        }
    }

    /// CLOACINA-T-1014: set the default and the maximum time limit of a task
    /// (the server's `--fleet-task-timeout-s` and `--fleet-task-timeout-max-s`).
    pub fn with_task_time_limits(mut self, time_limits: FleetTaskTimeLimits) -> Self {
        self.time_limits = time_limits;
        self
    }

    /// CLOACI-T-0861 / D-5 — attach the grant-gated secret resolver factory used
    /// to deliver per-execution fleet secrets. Wire this once the server secrets
    /// subsystem (tenant→`org_id`, KEK, grants) exists; until then the executor
    /// runs with `None` and secret-referencing fleet tasks fail closed.
    pub fn with_secret_resolver_factory(
        mut self,
        factory: Arc<dyn FleetSecretResolverFactory>,
    ) -> Self {
        self.secret_resolver_factory = Some(factory);
        self
    }

    /// CLOACI-T-0861 / D-5 — resolve + HPKE-wrap this task's `$secret`-referenced
    /// secrets to a ONE-TIME pooled key consumed from the selected agent.
    ///
    /// Returns `(wrapped_secrets, Some(key_id))` on success. Fails closed (Err,
    /// which the caller turns into a clean dispatch failure — NEVER plaintext,
    /// NEVER a reused key) when: no resolver factory is configured, no
    /// grant/resolver exists for the scope, or the agent's one-time key pool is
    /// exhausted. All secrets in one dispatch wrap to the SAME key.
    async fn wrap_task_secrets(
        &self,
        namespace: &TaskNamespace,
        agent_id: &str,
        execution_id: &str,
        secret_names: &[String],
    ) -> Result<(Vec<WrappedSecret>, Option<String>), String> {
        let Some(factory) = self.secret_resolver_factory.as_ref() else {
            return Err(format!(
                "task references {} secret(s) {:?} but this server has no secret \
                 resolver configured (CLOACINA_SECRET_KEK unset / secrets subsystem \
                 not wired) — refusing to dispatch (no plaintext fallback)",
                secret_names.len(),
                secret_names
            ));
        };
        let Some(resolver) = factory
            .resolver_for(&namespace.tenant_id, &namespace.package_name)
            .await
        else {
            return Err(format!(
                "no secret grants/resolver for tenant '{}' package '{}'",
                namespace.tenant_id, namespace.package_name
            ));
        };
        // Consume a ONE-TIME pooled key from the selected agent (D-5). Exhaustion
        // fails the dispatch cleanly — the server never reuses a key or sends
        // plaintext. The heartbeat replenish signal refills the pool for next time.
        let Some(key) = self.agent_registry.consume_secret_key(agent_id) else {
            return Err(format!(
                "agent '{}' one-time secret-key pool is exhausted — refusing to \
                 dispatch secrets (never reuse a key, never send plaintext)",
                agent_id
            ));
        };
        let pubkey = decode_pool_public_key(&key)
            .map_err(|e| format!("decode consumed pool public key: {e}"))?;
        let wrapped =
            resolve_and_wrap_secrets(resolver.as_ref(), secret_names, execution_id, &pubkey)
                .await
                .map_err(|e| format!("resolve+wrap secrets: {e}"))?;
        Ok((wrapped, Some(key.key_id)))
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Put the task back in the queue for a later dispatch try, with no use of
    /// a retry attempt (CLOACINA-T-1012). For "no agent can take it now": no
    /// live agent of the tenant has a free slot, or the chosen agent was full
    /// when the packet arrived. That is a wait for capacity, not a failure of
    /// the task.
    async fn defer_for_capacity(&self, event: &TaskReadyEvent, why: &str) -> ExecutionResult {
        metrics::counter!("cloacina_fleet_dispatch_deferred_total").increment(1);
        let not_before = cloacina::database::universal_types::UniversalTimestamp(
            chrono::Utc::now()
                + chrono::Duration::from_std(NO_CAPACITY_RETRY_DELAY)
                    .unwrap_or_else(|_| chrono::Duration::seconds(2)),
        );
        match self
            .dal
            .task_execution()
            .defer_dispatch(event.task_execution_id, not_before)
            .await
        {
            Ok(_) => debug!(
                task_id = %event.task_execution_id,
                why,
                "fleet: no capacity now; the task waits and keeps its attempt"
            ),
            Err(e) => warn!(
                task_id = %event.task_execution_id,
                error = %e,
                "fleet: could not delay the next dispatch try; the task stays Ready"
            ),
        }
        ExecutionResult::skipped(event.task_execution_id)
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
                // CLOACINA-T-1013: no dispatch without a claim. Without it two
                // invocations can send the same task to agents. The task stays
                // Ready and the scheduler dispatches it again on a later tick.
                warn!(
                    task_id = %task_execution_id,
                    error = %e,
                    "fleet: claim failed — not dispatching; the task stays Ready"
                );
                return Ok(ExecutionResult::skipped(task_execution_id));
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
        // (the result wait is the task's time limit plus RESULT_WAIT_GRACE).
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
                            // No task to ask: the name does not parse.
                            &RetryPolicy::default(),
                            start.elapsed(),
                            format!("parse_namespace({}): {}", event.task_name, e),
                        )
                        .await);
                }
            };

            // CLOACINA-T-1014: the task's own retry policy and time limit come
            // from the task loaded in the server runtime, the same object the
            // in-process executor asks. A task that is not loaded fails in
            // step 3; until then it has the default policy.
            let loaded_task = self.runtime.get_task(&namespace);
            let retry_policy = loaded_task
                .as_ref()
                .map(|task| task.retry_policy())
                .unwrap_or_default();
            let time_limit = self
                .time_limits
                .limit_for(loaded_task.as_ref().and_then(|task| task.timeout()));
            if let Some(asked) = time_limit.capped_from {
                warn!(
                    task = %event.task_name,
                    asked_s = asked.as_secs(),
                    max_s = time_limit.seconds,
                    "fleet: the task's time limit is above the server maximum; \
                     the maximum applies"
                );
            }
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
            //
            // CLOACINA-T-1018: the lookup goes through the execution's version
            // pin. The first task of an execution pins the active version; each
            // later task gets that version, also after a newer one is active.
            let (primary_digest, language, package_version) = match self
                .dal
                .workflow_packages()
                .resolve_dispatch_for_execution(
                    event.workflow_execution_id,
                    &namespace.package_name,
                    None,
                )
                .await
            {
                Ok(Some(pkg)) => (pkg.content_hash, pkg.language, pkg.version),
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
                    .get_artifact_triples_for_package(&namespace.package_name, &package_version)
                    .await
                    .unwrap_or_default();
                t.push(host_target_triple().to_string());
                Some(t)
            };
            // CLOACI-T-0916: selection reads the DB-backed roster
            // (heartbeat-recency-filtered), so agents registered/heartbeating
            // via ANY replica are eligible. Connection locality is a non-issue:
            // the work packet rides the delivery outbox and is pushed by
            // whichever replica holds the agent's delivery WebSocket.
            let snapshot = match self.agent_registry.live_agents().await {
                Ok(s) => s,
                Err(e) => {
                    return Ok(self
                        .reconcile_error(
                            &event,
                            &claimed_task,
                            &retry_policy,
                            start.elapsed(),
                            format!("fleet roster read failed: {}", e),
                        )
                        .await);
                }
            };
            let Some(agent) = select_fleet_agent(
                &snapshot,
                &task_tenant,
                &runnable_triples,
                cloacina_api_types::DEFAULT_AGENT_POOL,
            ) else {
                // CLOACINA-T-1012: a wait for capacity, not a failure. The task
                // stays Ready and keeps its attempt count.
                debug!(
                    task_id = %event.task_execution_id,
                    tenant = ?task_tenant,
                    "fleet: no live agent with capacity + a runnable arch in the task's tenant"
                );
                return Ok(self
                    .defer_for_capacity(&event, "no agent of the tenant has a free slot")
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
            let dependencies: Vec<cloacina::task::TaskNamespace> = match loaded_task.as_ref() {
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
                    .get_artifact_digest_for_target(
                        &namespace.package_name,
                        &package_version,
                        &agent_triple,
                    )
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

            // ── 3d. CLOACI-T-0861 / D-5 — if the task references secrets (via the
            //        T-0859 `{"$secret"}` alias map carried in the merged context),
            //        resolve + HPKE-wrap them to a ONE-TIME pooled key consumed
            //        from the selected agent. Fail the dispatch CLEANLY on any
            //        problem (no resolver, no grant, exhausted pool) — never a
            //        plaintext fallback, never a reused key.
            let secret_names = secret_ref_names(&context_value);
            let (wrapped_secrets, secret_key_id) = if secret_names.is_empty() {
                (Vec::new(), None)
            } else {
                match self
                    .wrap_task_secrets(
                        &namespace,
                        &agent_id,
                        &event.task_execution_id.0.to_string(),
                        &secret_names,
                    )
                    .await
                {
                    Ok(pair) => pair,
                    Err(msg) => {
                        return Ok(self
                            .reconcile_error(
                                &event,
                                &claimed_task,
                                &retry_policy,
                                start.elapsed(),
                                format!("fleet secret delivery failed: {msg}"),
                            )
                            .await);
                    }
                }
            };

            // ── 4. Build the work packet with real context + real artifact ref.
            // CLOACINA-T-1011: name this dispatch, so only its own report
            // resolves the wait registered in step 5.
            let dispatch_id = uuid::Uuid::new_v4().to_string();
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
                // CLOACINA-T-1014: the task's own limit or the server default,
                // capped at the server maximum. The agent enforces it.
                timeout_seconds: time_limit.seconds,
                tenant_id: tenant_id.clone(),
                language: Some(language),
                // CLOACI-T-0861 / D-5 — secrets resolved + HPKE-wrapped to a
                // one-time pooled key (see step 3d). `secret_key_id` names the
                // consumed pool key the agent unwraps ONCE with, then discards.
                // Both empty/None when the task references no secrets.
                wrapped_secrets,
                secret_key_id,
                dispatch_id: Some(dispatch_id.clone()),
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
            let rx = self
                .coordinator
                .register_dispatch(event.task_execution_id, dispatch_id);

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

            // ── 7. Await the agent's result: the task's time limit plus a
            //       short grace (CLOACINA-T-1014).
            let result_wait = time_limit.result_wait();
            let result_req: AgentResultRequest = match tokio::time::timeout(result_wait, rx).await {
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
                        elapsed_s = ?result_wait.as_secs(),
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
            // CLOACINA-T-1012: `Shutdown` is the agent's "I am full or going
            // away" refusal. The task did not run: wait for capacity, with no
            // use of a retry attempt.
            if matches!(
                result_req.outcome,
                AgentOutcome::Refused {
                    reason: cloacina::fleet::RefusalReason::Shutdown,
                    ..
                }
            ) {
                return Ok(self
                    .defer_for_capacity(&event, "the agent refused: full or shutting down")
                    .await);
            }
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
                    } => Err(agent_failure_error(
                        &event.task_name,
                        &message,
                        classification,
                    )),
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
        // Sync trait method — served from the replica-local cache of the last
        // DB roster read (refreshed on every roster touch + each sweeper
        // tick). Selection itself always re-reads the DB.
        self.agent_registry
            .cached_agents()
            .iter()
            .any(|a| a.available_capacity > 0)
    }

    fn metrics(&self) -> ExecutorMetrics {
        let snap = self.agent_registry.cached_agents();
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

// Agent selection lives in `crate::agent_registry::select_fleet_agent`
// (CLOACI-T-0916): it operates on the DB-backed live roster shared by this
// executor and the fleet graph executor.

// Silence unused warning on UniversalUuid if later changes drop a use.
const _: fn(UniversalUuid) = |_| {};

#[cfg(test)]
mod tests {
    use super::*;

    // ── CLOACINA-T-1014: time limits ─────────────────────────────────────────

    #[test]
    fn a_task_with_a_ten_minute_limit_gets_ten_minutes_in_the_packet_and_the_wait() {
        let limits = FleetTaskTimeLimits::default();
        let limit = limits.limit_for(Some(Duration::from_secs(600)));
        assert_eq!(limit.seconds, 600, "the packet carries the task's limit");
        assert_eq!(limit.capped_from, None);
        assert_eq!(
            limit.result_wait(),
            Duration::from_secs(600) + RESULT_WAIT_GRACE,
            "the server waits the limit plus the grace, not a fixed 300 s"
        );
    }

    #[test]
    fn a_task_with_no_limit_gets_the_server_default() {
        let limits = FleetTaskTimeLimits::from_seconds(120, 3600).unwrap();
        let limit = limits.limit_for(None);
        assert_eq!(limit.seconds, 120);
        assert_eq!(limit.capped_from, None);
        assert_eq!(FleetTaskTimeLimits::default().limit_for(None).seconds, 300);
    }

    #[test]
    fn a_task_limit_above_the_maximum_gets_the_maximum() {
        let limits = FleetTaskTimeLimits::from_seconds(300, 900).unwrap();
        let limit = limits.limit_for(Some(Duration::from_secs(7200)));
        assert_eq!(limit.seconds, 900);
        assert_eq!(limit.capped_from, Some(Duration::from_secs(7200)));
    }

    #[test]
    fn a_part_of_a_second_counts_as_one_second() {
        let limits = FleetTaskTimeLimits::default();
        assert_eq!(
            limits.limit_for(Some(Duration::from_millis(200))).seconds,
            1
        );
        assert_eq!(
            limits.limit_for(Some(Duration::from_millis(1500))).seconds,
            2
        );
    }

    #[test]
    fn bad_time_limit_settings_are_refused() {
        assert!(FleetTaskTimeLimits::from_seconds(0, 10).is_err());
        assert!(FleetTaskTimeLimits::from_seconds(10, 0).is_err());
        assert!(FleetTaskTimeLimits::from_seconds(600, 300).is_err());
        assert!(FleetTaskTimeLimits::from_seconds(300, u32::MAX as u64 + 1).is_err());
        assert_eq!(
            FleetTaskTimeLimits::from_seconds(
                FleetTaskTimeLimits::DEFAULT_SECONDS,
                FleetTaskTimeLimits::DEFAULT_MAX_SECONDS
            )
            .unwrap(),
            FleetTaskTimeLimits::default()
        );
    }

    // ── CLOACINA-T-1014: the agent's error reaches the retry conditions ─────

    /// What the agent reports for a failed task: `TaskError::to_string()`.
    fn agent_message(task_text: &str) -> String {
        TaskError::ExecutionFailed {
            message: task_text.to_string(),
            task_id: "t".to_string(),
            timestamp: chrono::Utc::now(),
        }
        .to_string()
    }

    fn policy(max_attempts: i32, condition: cloacina::retry::RetryCondition) -> RetryPolicy {
        RetryPolicy::builder()
            .max_attempts(max_attempts)
            .retry_condition(condition)
            .build()
    }

    #[test]
    fn an_error_pattern_matches_the_task_text_of_an_agent_failure() {
        use cloacina::retry::RetryCondition;
        let error = agent_failure_error(
            "public::pkg::wf::t",
            &agent_message("upstream said: Rate Limit exceeded"),
            FailureClassification::TaskError,
        );
        assert!(
            error.to_string().contains("Rate Limit exceeded"),
            "the task's own text is in the error: {error}"
        );
        let matching = policy(
            3,
            RetryCondition::ErrorPattern {
                patterns: vec!["rate limit".to_string()],
            },
        );
        assert!(TaskResultHandler::retry_allowed(&matching, 1, &error));
        let other = policy(
            3,
            RetryCondition::ErrorPattern {
                patterns: vec!["disk full".to_string()],
            },
        );
        assert!(!TaskResultHandler::retry_allowed(&other, 1, &error));
    }

    #[test]
    fn transient_only_reads_the_task_text_of_an_agent_failure() {
        use cloacina::retry::RetryCondition;
        let transient = agent_failure_error(
            "public::pkg::wf::t",
            &agent_message("connection reset by peer"),
            FailureClassification::TaskError,
        );
        let permanent = agent_failure_error(
            "public::pkg::wf::t",
            &agent_message("invalid input row 7"),
            FailureClassification::TaskError,
        );
        let p = policy(3, RetryCondition::TransientOnly);
        assert!(TaskResultHandler::retry_allowed(&p, 1, &transient));
        assert!(!TaskResultHandler::retry_allowed(&p, 1, &permanent));
    }

    #[test]
    fn a_timeout_on_the_agent_is_a_task_timeout() {
        use cloacina::retry::RetryCondition;
        let error = agent_failure_error(
            "public::pkg::wf::t",
            "task exceeded timeout of 600s",
            FailureClassification::Timeout,
        );
        assert!(matches!(error, ExecutorError::TaskTimeout));
        assert!(TaskResultHandler::retry_allowed(
            &policy(3, RetryCondition::TransientOnly),
            1,
            &error
        ));
    }

    // ── CLOACINA-T-1014: the task's retry policy decides, not the default ───

    #[test]
    fn retry_attempts_zero_gives_one_attempt() {
        use cloacina::retry::RetryCondition;
        let error = agent_failure_error(
            "public::pkg::wf::t",
            &agent_message("boom"),
            FailureClassification::TaskError,
        );
        // The first run of a task is attempt 1.
        assert!(!TaskResultHandler::retry_allowed(
            &policy(0, RetryCondition::AllErrors),
            1,
            &error
        ));
        assert!(!TaskResultHandler::retry_allowed(
            &policy(3, RetryCondition::Never),
            1,
            &error
        ));
        // The default policy, which the fleet used for every task before,
        // retries the same failure.
        assert!(TaskResultHandler::retry_allowed(
            &RetryPolicy::default(),
            1,
            &error
        ));
    }

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

    // ── D-5 one-time key pool: registry consume → wrap → agent unwrap ────────

    /// End-to-end at the SERVER boundary: the server consumes a one-time pooled
    /// key from the agent's replica-local pool, HPKE-wraps a secret to it, and
    /// the agent unwraps with the matching pooled private key into
    /// `Context::secret`. Only ciphertext + key_id cross the wire; the pool is
    /// one-time (a second dispatch spends a different key; exhaustion then
    /// fails cleanly).
    #[tokio::test]
    async fn fleet_pool_wrap_unwrap_end_to_end_and_exhaustion() {
        use cloacina::security::{AgentKeyPool, InMemorySecretResolver, resolve_and_wrap_secrets};
        use std::collections::BTreeMap;

        // Agent mints a 1-key pool; this replica's registry stores the public
        // entries (CLOACI-T-0916: the pool is replica-local; the roster row
        // itself lives in the DB and is not touched by the pool paths).
        let mut agent_pool = AgentKeyPool::new();
        let entries = agent_pool.mint(1);
        let registry = AgentRegistry::new(
            cloacina::database::Database::new(
                "postgres://cloacina:cloacina@localhost:15432/cloacina",
                "cloacina",
                1,
            ),
            std::time::Duration::from_secs(60),
        );
        assert_eq!(registry.replenish_secret_keys("a1", entries), 1);

        // The "gated resolver" the factory would return — here an in-memory one
        // holding the resolved plaintext (stands in for SecretStoreResolver).
        let mut fields = BTreeMap::new();
        fields.insert("password".to_string(), "hunter2".to_string());
        let resolver = InMemorySecretResolver::new(std::collections::HashMap::from([(
            "db".to_string(),
            fields,
        )]));

        // Server side: consume ONE pooled key, wrap to it.
        let key = registry
            .consume_secret_key("a1")
            .expect("a pooled key to consume");
        let pubkey = cloacina::security::decode_pool_public_key(&key).unwrap();
        let wrapped = resolve_and_wrap_secrets(&resolver, &["db".to_string()], "exec-7", &pubkey)
            .await
            .unwrap();
        assert!(!wrapped[0].ciphertext_b64.contains("hunter2"));

        // Agent side: take the matching one-time private key, unwrap, serve.
        let priv_key = agent_pool.take(&key.key_id).expect("agent holds the key");
        let agent_resolver =
            InMemorySecretResolver::from_wrapped(&priv_key, &wrapped, "exec-7").unwrap();
        let ctx =
            Context::<serde_json::Value>::new().with_secret_resolver(agent_resolver.into_arc());
        assert_eq!(
            ctx.secret("db").await.unwrap().get("password").unwrap(),
            "hunter2"
        );

        // One-time: the pool is now exhausted — a second dispatch gets no key
        // (the executor would fail closed, never reuse/plaintext).
        assert!(registry.consume_secret_key("a1").is_none());
    }

    // Agent selection / tenant isolation tests (CLOACI-T-0817) moved to
    // `crate::agent_registry` alongside `select_fleet_agent` (CLOACI-T-0916).
}
