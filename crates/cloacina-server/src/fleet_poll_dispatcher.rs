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

//! Polls of poll triggers, sent to the tenant's agents (CLOACINA-T-1077).
//!
//! A server that loads packages from their metadata runs no tenant code
//! (CLOACINA-A-0019), so a poll trigger's code runs on an agent of its
//! tenant. The scheduler of the tenant's runner decides a poll is due and
//! polls a `DispatchedTrigger`, which asks this dispatcher. The dispatcher:
//!
//! 1. reads the active build of the package in the tenant's schema;
//! 2. picks a live agent of the tenant, pool `default`, with a free slot, a
//!    runnable arch and the capability `job:poll` (an agent of a release
//!    before poll packets registers none and is never sent one);
//! 3. waits for the agent's report on the [`FleetCoordinator`] under a new
//!    poll id, and enqueues a [`PollWorkPacket`] on the delivery outbox;
//! 4. reads the report as fire (with the trigger's context) or skip.
//!
//! No agent, no answer in time, a refusal or the trigger's error is an error
//! of the poll: the scheduler records it on the schedule row and fires
//! nothing. A late report finds no wait and changes nothing. A packet that
//! reaches an agent after the server stopped waiting is not run by the agent
//! (`not_after_unix_ms`). The poll never runs in this process.
//!
//! A poll of a Python polling accumulator (CLOACINA-T-1156) takes the same
//! path: the accumulator on the reactor's owner replica asks at each
//! interval; the agent needs the capability `job:accumulator_poll` and
//! answers a JSON value or no change. With no answer the accumulator emits
//! nothing that interval; the reason is logged and counted.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use cloacina::cloacina_workflow::{TriggerError, TriggerResult};
use cloacina::dal::unified::FleetAgent;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::fleet::{
    ACCUMULATOR_POLL_CAPABILITY, AGENT_PROTOCOL_VERSION, AGENT_RECIPIENT_PREFIX, ArtifactRef,
    POLL_JOB_CAPABILITY, POLL_PACKET_KIND, PollFailure, PollWorkPacket,
    accumulator_poll_from_outcome, host_target_triple, poll_result_from_outcome,
};
use cloacina::models::delivery_outbox::NewDeliveryOutbox;
use cloacina::trigger::{AccumulatorPollRequest, PollDispatcher, PollRequest};
use tracing::{debug, info, warn};

use crate::agent_registry::{AgentRegistry, FleetLoad, TaskDemand, select_fleet_agent};
use crate::fleet_coordinator::FleetCoordinator;

/// How long the server waits for the answer past the poll's time limit: the
/// agent starts the clock after it took the packet and loaded the package,
/// and its report needs a moment to arrive.
pub const POLL_ANSWER_GRACE: Duration = Duration::from_secs(10);

/// The default of `--fleet-poll-timeout-s`: how long an agent lets one poll
/// run. The same as the in-process poll limit of the scheduler.
pub const DEFAULT_POLL_TIMEOUT_SECONDS: u64 = 30;

static POLL_TIMEOUT_SECONDS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(DEFAULT_POLL_TIMEOUT_SECONDS);

/// Set the time an agent lets one poll run (`--fleet-poll-timeout-s`).
/// Called once at start, before the runners are made. 0 is refused.
pub fn set_poll_timeout_seconds(seconds: u64) -> Result<(), String> {
    if seconds == 0 || seconds > u32::MAX as u64 {
        return Err(format!(
            "--fleet-poll-timeout-s must be between 1 and {} seconds, got {seconds}",
            u32::MAX
        ));
    }
    POLL_TIMEOUT_SECONDS.store(seconds, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// The time an agent lets one poll run, as set at start.
pub fn poll_timeout() -> Duration {
    Duration::from_secs(POLL_TIMEOUT_SECONDS.load(std::sync::atomic::Ordering::Relaxed))
}

/// Sends each poll of a tenant's poll triggers to an agent of the tenant.
pub struct FleetPollDispatcher {
    /// The tenant's DAL: the package builds.
    dal: cloacina::dal::DAL,
    /// The admin DAL: the delivery outbox is server-wide.
    outbox_dal: cloacina::dal::DAL,
    agent_registry: Arc<AgentRegistry>,
    coordinator: Arc<FleetCoordinator>,
    delivery_wake: cloacina::delivery::WakeHandle,
    /// How long an agent lets one poll run.
    poll_timeout: Duration,
}

/// Why a poll got no answer, as the label of
/// `cloacina_fleet_polls_missed_total`. Gives the message back.
fn missed_reason(reason: &'static str, message: String) -> String {
    metrics::counter!("cloacina_fleet_polls_missed_total", "reason" => reason).increment(1);
    metrics::counter!("cloacina_fleet_polls_total", "answer" => "missed").increment(1);
    message
}

/// The agents of a roster that registered `capability`.
fn capable_of(live: &[FleetAgent], capability: &str) -> Vec<FleetAgent> {
    live.iter()
        .filter(|a| a.capabilities.iter().any(|c| c == capability))
        .cloned()
        .collect()
}

/// The agents of a roster that can run poll packets of poll triggers.
#[cfg(test)]
pub(crate) fn poll_capable(live: &[FleetAgent]) -> Vec<FleetAgent> {
    capable_of(live, POLL_JOB_CAPABILITY)
}

/// The agents of a roster that can run polls of polling accumulators
/// (CLOACINA-T-1156).
#[cfg(test)]
pub(crate) fn accumulator_poll_capable(live: &[FleetAgent]) -> Vec<FleetAgent> {
    capable_of(live, ACCUMULATOR_POLL_CAPABILITY)
}

/// The reason of a poll trigger's poll that found no agent to take it.
#[cfg(test)]
pub(crate) fn no_agent_reason(request: &PollRequest, live: &[FleetAgent]) -> String {
    no_agent_reason_for(&PollTarget::Trigger(request), live)
}

fn no_agent_reason_for(target: &PollTarget<'_>, live: &[FleetAgent]) -> String {
    let tenant = target.tenant_id();
    let of_tenant = live
        .iter()
        .filter(|a| a.tenant_id.as_deref() == Some(tenant))
        .count();
    let capable = capable_of(live, target.capability())
        .iter()
        .filter(|a| a.tenant_id.as_deref() == Some(tenant))
        .count();
    let (kind, packets, keeps) = match target {
        PollTarget::Trigger(_) => (
            "poll triggers",
            "poll packets",
            "A poll trigger runs on the tenant's agents; the fleet keeps one agent for a tenant \
             with a poll trigger",
        ),
        PollTarget::Accumulator(_) => (
            "polling accumulators",
            "polls of polling accumulators",
            "A polling accumulator's poll function runs on the tenant's agents; keep one agent \
             running for it",
        ),
    };
    let why = if of_tenant == 0 {
        "the tenant has no live agent".to_string()
    } else if capable == 0 {
        format!(
            "none of the tenant's {of_tenant} live agent(s) can run {kind}: they are of a \
             release before {packets}; upgrade them"
        )
    } else {
        "every agent that can run it is full, or none has a build of the package for its arch"
            .to_string()
    };
    format!(
        "poll of {} (package '{}') not run: {why}. {keeps}",
        target.what(),
        target.package_name()
    )
}

/// The packet of one poll of a polling accumulator (CLOACINA-T-1156).
pub(crate) fn accumulator_poll_packet(
    poll_id: UniversalUuid,
    request: &AccumulatorPollRequest,
    agent_tenant: Option<String>,
    artifact: ArtifactRef,
    language: String,
    poll_timeout: Duration,
    not_after_unix_ms: i64,
) -> PollWorkPacket {
    PollWorkPacket {
        protocol_version: AGENT_PROTOCOL_VERSION,
        poll_id: poll_id.0.to_string(),
        trigger_name: String::new(),
        workflow_name: String::new(),
        package_name: request.package_name.clone(),
        artifact,
        timeout_seconds: poll_timeout.as_secs().clamp(1, u32::MAX as u64) as u32,
        tenant_id: agent_tenant,
        language: Some(language),
        not_after_unix_ms: Some(not_after_unix_ms),
        accumulator_name: Some(request.accumulator_name.clone()),
    }
}

/// The packet of one poll.
pub(crate) fn poll_work_packet(
    poll_id: UniversalUuid,
    request: &PollRequest,
    agent_tenant: Option<String>,
    artifact: ArtifactRef,
    language: String,
    poll_timeout: Duration,
    not_after_unix_ms: i64,
) -> PollWorkPacket {
    PollWorkPacket {
        protocol_version: AGENT_PROTOCOL_VERSION,
        poll_id: poll_id.0.to_string(),
        trigger_name: request.trigger_name.clone(),
        workflow_name: request.workflow_name.clone(),
        package_name: request.package_name.clone(),
        artifact,
        timeout_seconds: poll_timeout.as_secs().clamp(1, u32::MAX as u64) as u32,
        tenant_id: agent_tenant,
        language: Some(language),
        not_after_unix_ms: Some(not_after_unix_ms),
        accumulator_name: None,
    }
}

impl FleetPollDispatcher {
    pub fn new(
        dal: cloacina::dal::DAL,
        outbox_dal: cloacina::dal::DAL,
        agent_registry: Arc<AgentRegistry>,
        coordinator: Arc<FleetCoordinator>,
        delivery_wake: cloacina::delivery::WakeHandle,
        poll_timeout: Duration,
    ) -> Self {
        Self {
            dal,
            outbox_dal,
            agent_registry,
            coordinator,
            delivery_wake,
            poll_timeout: poll_timeout.max(Duration::from_secs(1)),
        }
    }

    /// How long the server waits for the agent's answer.
    fn answer_wait(&self) -> Duration {
        self.poll_timeout + POLL_ANSWER_GRACE
    }
}

/// What one poll is of: a trigger (CLOACINA-T-1077) or a polling accumulator
/// (CLOACINA-T-1156).
pub(crate) enum PollTarget<'a> {
    Trigger(&'a PollRequest),
    Accumulator(&'a AccumulatorPollRequest),
}

impl PollTarget<'_> {
    fn tenant_id(&self) -> &str {
        match self {
            PollTarget::Trigger(r) => &r.tenant_id,
            PollTarget::Accumulator(r) => &r.tenant_id,
        }
    }
    fn package_name(&self) -> &str {
        match self {
            PollTarget::Trigger(r) => &r.package_name,
            PollTarget::Accumulator(r) => &r.package_name,
        }
    }
    /// "trigger 'x'" or "polling accumulator 'x'", for the reasons.
    fn what(&self) -> String {
        match self {
            PollTarget::Trigger(r) => format!("trigger '{}'", r.trigger_name),
            PollTarget::Accumulator(r) => format!("polling accumulator '{}'", r.accumulator_name),
        }
    }
    /// The capability an agent needs to run it.
    fn capability(&self) -> &'static str {
        match self {
            PollTarget::Trigger(_) => POLL_JOB_CAPABILITY,
            PollTarget::Accumulator(_) => ACCUMULATOR_POLL_CAPABILITY,
        }
    }
}

#[async_trait]
impl PollDispatcher for FleetPollDispatcher {
    async fn poll(&self, request: &PollRequest) -> Result<TriggerResult, TriggerError> {
        let report = self
            .send_poll(&PollTarget::Trigger(request))
            .await
            .map_err(|message| TriggerError::PollError { message })?;
        match poll_result_from_outcome(report.outcome) {
            Ok(result) => {
                let answer = if result.should_fire() { "fire" } else { "skip" };
                metrics::counter!("cloacina_fleet_polls_total", "answer" => answer).increment(1);
                debug!(
                    trigger = %request.trigger_name,
                    agent_id = %report.agent_id,
                    duration_ms = report.duration_ms,
                    answer,
                    "fleet: poll answered"
                );
                Ok(result)
            }
            Err(failure) => Err(TriggerError::PollError {
                message: self.failure_reason(
                    &PollTarget::Trigger(request),
                    &report.agent_id,
                    failure,
                ),
            }),
        }
    }

    async fn poll_accumulator(
        &self,
        request: &AccumulatorPollRequest,
    ) -> Result<Option<serde_json::Value>, String> {
        let target = PollTarget::Accumulator(request);
        let report = self.send_poll(&target).await?;
        match accumulator_poll_from_outcome(report.outcome) {
            Ok(value) => {
                let answer = if value.is_some() { "emit" } else { "skip" };
                metrics::counter!("cloacina_fleet_polls_total", "answer" => answer).increment(1);
                debug!(
                    accumulator = %request.accumulator_name,
                    agent_id = %report.agent_id,
                    duration_ms = report.duration_ms,
                    answer,
                    "fleet: accumulator poll answered"
                );
                Ok(value)
            }
            Err(failure) => Err(self.failure_reason(&target, &report.agent_id, failure)),
        }
    }

    fn wait(&self) -> Duration {
        // The lookups and the enqueue before the answer wait take a moment.
        self.answer_wait() + Duration::from_secs(5)
    }
}

impl FleetPollDispatcher {
    /// The reason of an answer the server cannot use, counted.
    fn failure_reason(
        &self,
        target: &PollTarget<'_>,
        agent_id: &str,
        failure: PollFailure,
    ) -> String {
        let what = target.what();
        match failure {
            PollFailure::TriggerFailed(message) => {
                metrics::counter!("cloacina_fleet_polls_total", "answer" => "failed").increment(1);
                format!("{what} failed on agent {agent_id}: {message}")
            }
            failure @ PollFailure::Refused { .. } => missed_reason(
                "refused",
                format!("poll of {what} not run by agent {agent_id}: {failure}"),
            ),
            failure @ PollFailure::BadAnswer(_) => missed_reason(
                "bad_answer",
                format!("poll of {what} on agent {agent_id}: {failure}"),
            ),
        }
    }

    /// Send one poll to an agent of the tenant and wait for its report. `Err`:
    /// no report, with the reason, counted as missed.
    async fn send_poll(
        &self,
        target: &PollTarget<'_>,
    ) -> Result<cloacina::fleet::AgentResultRequest, String> {
        let what = target.what();
        let package_name = target.package_name().to_string();
        // 1. The active build of the package, in the tenant's schema.
        let active = match self
            .dal
            .workflow_packages()
            .get_active_dispatch(&package_name, None)
            .await
        {
            Ok(Some(active)) => active,
            Ok(None) => {
                return Err(missed_reason(
                    "no_build",
                    format!("poll of {what} not run: package '{package_name}' has no active build"),
                ));
            }
            Err(e) => {
                return Err(missed_reason(
                    "no_build",
                    format!(
                        "poll of {what} not run: the build of package '{package_name}' could \
                         not be read: {e}"
                    ),
                ));
            }
        };
        let interpreted = active.language.eq_ignore_ascii_case("python");
        let runnable_triples: Option<Vec<String>> = if interpreted {
            active.python_platforms.clone()
        } else {
            let mut t = self
                .dal
                .workflow_packages()
                .get_artifact_triples_for_package(&package_name, &active.version)
                .await
                .unwrap_or_default();
            t.push(host_target_triple());
            Some(t)
        };

        // 2. An agent of the tenant that can run this poll.
        let live = self.agent_registry.live_agents().await.map_err(|e| {
            missed_reason(
                "no_agent",
                format!("poll of {what} not run: the fleet roster could not be read: {e}"),
            )
        })?;
        let capable = capable_of(&live, target.capability());
        let tenant = Some(target.tenant_id().to_string());
        // CLOACINA-T-1086: only an agent that loads the package's plugin
        // interface version.
        if let Some(reason) = crate::agent_registry::plugin_interface_wait_reason(
            &capable,
            &tenant,
            cloacina_api_types::DEFAULT_AGENT_POOL,
            active.plugin_interface_version,
        ) {
            return Err(missed_reason(
                "no_agent",
                format!("poll of {what} not run: {reason}"),
            ));
        }
        let capable =
            crate::agent_registry::agents_loading(&capable, active.plugin_interface_version);
        let Some(agent) = select_fleet_agent(
            &capable,
            &tenant,
            &runnable_triples,
            // A poll names no pool and has no request: the pool `default`,
            // packed by free slots, as a graph firing.
            &TaskDemand::pool(cloacina_api_types::DEFAULT_AGENT_POOL),
            &FleetLoad::new(),
        ) else {
            return Err(missed_reason(
                "no_agent",
                no_agent_reason_for(target, &live),
            ));
        };
        let agent_id = agent.agent_id.clone();

        let digest = if interpreted {
            active.content_hash.clone()
        } else {
            match self
                .dal
                .workflow_packages()
                .get_artifact_digest_for_target(
                    &package_name,
                    &active.version,
                    &agent.target_triple,
                )
                .await
            {
                Ok(Some(arch_digest)) => arch_digest,
                _ => active.content_hash.clone(),
            }
        };
        let build_target_triple = if interpreted {
            agent.target_triple.clone()
        } else {
            host_target_triple()
        };

        // 3. Wait before the enqueue, so a fast agent cannot answer first.
        let poll_id = UniversalUuid::new_v4();
        let rx = self.coordinator.register_dispatch(
            poll_id,
            cloacina::security::ownership::DispatchTarget {
                tenant_id: agent.tenant_id.clone(),
                agent_id: agent_id.clone(),
                dispatch_id: None,
            },
        );
        let wait = self.answer_wait();
        let not_after = chrono::Utc::now()
            + chrono::Duration::from_std(wait).unwrap_or(chrono::Duration::seconds(60));
        let artifact = ArtifactRef {
            fetch_url: format!("/v1/agent/artifact/{digest}"),
            digest,
            build_target_triple,
        };
        let packet = match target {
            PollTarget::Trigger(request) => poll_work_packet(
                poll_id,
                request,
                agent.tenant_id.clone(),
                artifact,
                active.language.clone(),
                self.poll_timeout,
                not_after.timestamp_millis(),
            ),
            PollTarget::Accumulator(request) => accumulator_poll_packet(
                poll_id,
                request,
                agent.tenant_id.clone(),
                artifact,
                active.language.clone(),
                self.poll_timeout,
                not_after.timestamp_millis(),
            ),
        };
        let payload = match serde_json::to_vec(&packet) {
            Ok(p) => p,
            Err(e) => {
                self.coordinator.cancel(poll_id);
                return Err(missed_reason(
                    "enqueue",
                    format!("serialize the poll packet: {e}"),
                ));
            }
        };
        let row = NewDeliveryOutbox {
            recipient: format!("{AGENT_RECIPIENT_PREFIX}{agent_id}"),
            kind: POLL_PACKET_KIND.to_string(),
            tenant_id: agent.tenant_id.clone(),
            payload,
        };
        match self.outbox_dal.delivery_outbox().enqueue(row).await {
            Ok(row) => self.coordinator.packet_enqueued(poll_id, row.id),
            Err(e) => {
                self.coordinator.cancel(poll_id);
                return Err(missed_reason(
                    "enqueue",
                    format!("poll of {what} not run: the packet could not be queued: {e}"),
                ));
            }
        }
        self.delivery_wake.wake();
        debug!(
            target = %what,
            agent_id = %agent_id,
            poll_id = %poll_id,
            "fleet: poll enqueued"
        );

        // 4. The report.
        match tokio::time::timeout(wait, rx).await {
            Ok(Ok(report)) => Ok(report),
            Ok(Err(_canceled)) => Err(missed_reason(
                "no_answer",
                format!("poll of {what}: the wait for agent {agent_id} ended before it answered"),
            )),
            Err(_elapsed) => {
                self.coordinator.cancel(poll_id);
                info!(
                    target = %what,
                    agent_id = %agent_id,
                    poll_id = %poll_id,
                    wait_s = wait.as_secs(),
                    "fleet: no answer to a poll in time; a later answer changes nothing"
                );
                Err(missed_reason(
                    "no_answer",
                    format!(
                        "poll of {what}: no answer from agent {agent_id} in {} s; nothing came \
                         of it, and a later answer is dropped",
                        wait.as_secs()
                    ),
                ))
            }
        }
    }
}

/// Tenants that keep one agent because of a poll trigger (CLOACINA-T-1077).
///
/// The fleet control loop reads this each tick, on the leader only, when the
/// server loads packages from their metadata: a tenant whose schema has an
/// enabled, unpaused poll trigger gets a floor of one agent, so its polls
/// have an agent to run on. A tenant with none keeps the configured floor and
/// can scale to zero.
pub async fn tenants_with_poll_triggers(
    admin: &cloacina::database::Database,
    tenants: &crate::TenantDatabaseCache,
) -> Result<std::collections::HashSet<String>, String> {
    let mut out = std::collections::HashSet::new();
    // `public` lives in the admin schema.
    match cloacina::dal::DAL::new(admin.clone())
        .schedule()
        .has_active_poll_triggers()
        .await
    {
        Ok(true) => {
            out.insert("public".to_string());
        }
        Ok(false) => {}
        Err(e) => warn!(error = %e, "poll floor: the public schema could not be read"),
    }
    let schemas = cloacina::database::DatabaseAdmin::new(admin.clone())
        .list_tenant_schemas()
        .await
        .map_err(|e| e.to_string())?;
    for tenant in schemas {
        let db = match tenants.resolve(&tenant, admin).await {
            Ok(db) => db,
            Err(e) => {
                warn!(tenant = %tenant, error = %e, "poll floor: no database for the tenant; skipped");
                continue;
            }
        };
        match cloacina::dal::DAL::new(db)
            .schedule()
            .has_active_poll_triggers()
            .await
        {
            Ok(true) => {
                out.insert(tenant);
            }
            Ok(false) => {}
            Err(e) => {
                warn!(tenant = %tenant, error = %e, "poll floor: the tenant's schedules could not be read; skipped")
            }
        }
    }
    Ok(out)
}

/// The floor of a tenant's agent count (CLOACINA-T-1077): at least one for a
/// tenant with a poll trigger, else the configured floor.
pub fn tenant_floor(configured_floor: u32, has_poll_trigger: bool) -> u32 {
    if has_poll_trigger {
        configured_floor.max(1)
    } else {
        configured_floor
    }
}

/// The counts the actuator reconciles to, with the poll floor applied: each
/// tenant with a poll trigger gets at least one agent, also a tenant with no
/// desired count yet.
pub fn apply_poll_floor(
    mut desired: Vec<(String, u32)>,
    poll_tenants: &std::collections::HashSet<String>,
) -> Vec<(String, u32)> {
    for (tenant, want) in desired.iter_mut() {
        if poll_tenants.contains(tenant) {
            *want = (*want).max(1);
        }
    }
    let mut missing: Vec<&String> = poll_tenants
        .iter()
        .filter(|t| !desired.iter().any(|(d, _)| d == *t))
        .collect();
    missing.sort();
    for tenant in missing {
        desired.push((tenant.clone(), 1));
    }
    desired
}

/// Describe the metrics of the poll dispatch. Called once at server start.
pub fn describe_metrics() {
    metrics::describe_counter!(
        "cloacina_fleet_polls_total",
        "Polls of poll triggers (CLOACINA-T-1077) and of polling accumulators \
         (CLOACINA-T-1156) sent to agents, by `answer`: `fire` or `skip` (a trigger), `emit` \
         or `skip` (an accumulator), `failed` (the code returned an error on the agent) or \
         `missed` (no answer: see cloacina_fleet_polls_missed_total)."
    );
    metrics::describe_counter!(
        "cloacina_fleet_polls_missed_total",
        "Polls of poll triggers and polling accumulators that got no answer, by `reason`: \
         `no_build`, `no_agent`, `enqueue`, `no_answer`, `refused` or `bad_answer`. \
         Nothing fires and nothing is emitted; the next interval polls again."
    );
    metrics::describe_counter!(
        "cloacina_accumulator_polls_total",
        "Polls of Python polling accumulators sent to agents, as the accumulator saw them \
         (CLOACINA-T-1156), by `outcome`: `emit`, `skip` (no change), `missed` (no answer) \
         or `in_flight` (an interval skipped because the last poll still ran)."
    );
    metrics::describe_gauge!(
        "cloacina_fleet_poll_floor_tenants",
        "Tenants held at one agent or more because they have a poll trigger, as read on \
         the last tick of the fleet loop (CLOACINA-T-1077). Set only by the replica that \
         led the tick."
    );
    metrics::describe_counter!(
        "cloacina_trigger_polls_total",
        "Polls of poll triggers by the scheduler, by `outcome`: `fire`, `skip`, `error` (no \
         answer, or the trigger's error) or `in_flight` (an interval skipped because the \
         last poll still runs). CLOACINA-T-1077."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_registry::test_fleet_agent;

    fn request() -> PollRequest {
        PollRequest {
            tenant_id: "acme".into(),
            package_name: "inbox".into(),
            package_version: "1.0.0".into(),
            trigger_name: "watch_inbox".into(),
            workflow_name: "wf".into(),
        }
    }

    fn agent(id: &str, tenant: &str, polls: bool) -> FleetAgent {
        let mut a = test_fleet_agent(id, 4, Some(tenant));
        a.target_triple = host_target_triple();
        if polls {
            a.capabilities.push(POLL_JOB_CAPABILITY.to_string());
        }
        a
    }

    /// Only an agent that registered `job:poll`, of the poll's tenant, is
    /// picked; an older agent never gets a poll.
    #[test]
    fn a_poll_goes_to_an_agent_of_the_tenant_that_can_run_polls() {
        let live = vec![
            agent("old", "acme", false),
            agent("other-tenant", "globex", true),
            agent("new", "acme", true),
        ];
        let capable = poll_capable(&live);
        let picked = select_fleet_agent(
            &capable,
            &Some("acme".to_string()),
            &None,
            &TaskDemand::pool(cloacina_api_types::DEFAULT_AGENT_POOL),
            &FleetLoad::new(),
        )
        .expect("an agent");
        assert_eq!(picked.agent_id, "new");

        let only_old = vec![agent("old", "acme", false)];
        assert!(
            select_fleet_agent(
                &poll_capable(&only_old),
                &Some("acme".to_string()),
                &None,
                &TaskDemand::pool(cloacina_api_types::DEFAULT_AGENT_POOL),
                &FleetLoad::new(),
            )
            .is_none()
        );
    }

    #[test]
    fn the_no_agent_reason_says_what_is_missing() {
        let r = no_agent_reason(&request(), &[]);
        assert!(r.contains("the tenant has no live agent"), "{r}");
        assert!(r.contains("'watch_inbox'"), "{r}");
        let r = no_agent_reason(&request(), &[agent("old", "acme", false)]);
        assert!(r.contains("release before poll packets"), "{r}");
        let r = no_agent_reason(&request(), &[agent("new", "acme", true)]);
        assert!(r.contains("full"), "{r}");
    }

    #[test]
    fn the_packet_carries_the_trigger_the_limit_and_the_end_of_the_wait() {
        let id = UniversalUuid::new_v4();
        let packet = poll_work_packet(
            id,
            &request(),
            Some("acme".into()),
            ArtifactRef {
                digest: "abc".into(),
                fetch_url: "/v1/agent/artifact/abc".into(),
                build_target_triple: host_target_triple(),
            },
            "rust".into(),
            Duration::from_secs(30),
            1_234,
        );
        assert_eq!(packet.poll_id, id.0.to_string());
        assert_eq!(packet.trigger_name, "watch_inbox");
        assert_eq!(packet.package_name, "inbox");
        assert_eq!(packet.timeout_seconds, 30);
        assert_eq!(packet.not_after_unix_ms, Some(1_234));
        assert_eq!(packet.tenant_id.as_deref(), Some("acme"));
        let wire: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&packet).unwrap()).unwrap();
        assert_eq!(wire["poll_id"], id.0.to_string());
    }

    fn accumulator_request() -> AccumulatorPollRequest {
        AccumulatorPollRequest {
            tenant_id: "acme".into(),
            package_name: "prices".into(),
            package_version: "1.0.0".into(),
            accumulator_name: "ticker".into(),
        }
    }

    /// A poll of a polling accumulator goes only to an agent that registered
    /// `job:accumulator_poll` (CLOACINA-T-1156): an agent with `job:poll`
    /// alone (the release of poll triggers) never gets one.
    #[test]
    fn an_accumulator_poll_goes_only_to_an_agent_that_can_run_it() {
        let mut new = agent("new", "acme", true);
        new.capabilities
            .push(ACCUMULATOR_POLL_CAPABILITY.to_string());
        let live = vec![agent("triggers-only", "acme", true), new];
        let capable = accumulator_poll_capable(&live);
        let picked = select_fleet_agent(
            &capable,
            &Some("acme".to_string()),
            &None,
            &TaskDemand::pool(cloacina_api_types::DEFAULT_AGENT_POOL),
            &FleetLoad::new(),
        )
        .expect("an agent");
        assert_eq!(picked.agent_id, "new");

        let only_old = vec![agent("triggers-only", "acme", true)];
        assert!(accumulator_poll_capable(&only_old).is_empty());
        let request = accumulator_request();
        let r = no_agent_reason_for(&PollTarget::Accumulator(&request), &only_old);
        assert!(r.contains("polling accumulator 'ticker'"), "{r}");
        assert!(
            r.contains("release before polls of polling accumulators"),
            "{r}"
        );
    }

    #[test]
    fn the_accumulator_packet_names_the_accumulator_and_no_trigger() {
        let id = UniversalUuid::new_v4();
        let packet = accumulator_poll_packet(
            id,
            &accumulator_request(),
            Some("acme".into()),
            ArtifactRef {
                digest: "abc".into(),
                fetch_url: "/v1/agent/artifact/abc".into(),
                build_target_triple: host_target_triple(),
            },
            "python".into(),
            Duration::from_secs(30),
            1_234,
        );
        assert_eq!(packet.accumulator_name.as_deref(), Some("ticker"));
        assert!(packet.trigger_name.is_empty());
        assert_eq!(packet.package_name, "prices");
        assert_eq!(packet.timeout_seconds, 30);
        assert_eq!(packet.not_after_unix_ms, Some(1_234));
        assert_eq!(packet.language.as_deref(), Some("python"));
    }

    /// The floor keeps one agent for a tenant with a poll trigger, also one
    /// with no desired count yet, and leaves the others to scale to zero.
    #[test]
    fn a_tenant_with_a_poll_trigger_keeps_one_agent() {
        assert_eq!(tenant_floor(0, true), 1);
        assert_eq!(tenant_floor(0, false), 0);
        assert_eq!(tenant_floor(3, true), 3);

        let polls: std::collections::HashSet<String> =
            ["acme".to_string(), "initech".to_string()].into();
        let desired = apply_poll_floor(
            vec![
                ("acme".to_string(), 0),
                ("globex".to_string(), 0),
                ("big".to_string(), 5),
            ],
            &polls,
        );
        assert_eq!(
            desired,
            vec![
                ("acme".to_string(), 1),
                ("globex".to_string(), 0),
                ("big".to_string(), 5),
                ("initech".to_string(), 1),
            ]
        );
    }
}
