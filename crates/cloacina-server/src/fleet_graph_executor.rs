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

//! Fleet dispatch for computation-graph firings (CLOACI-T-0722).
//!
//! Mirrors [`crate::fleet_executor::FleetExecutor`] for the reactor's
//! whole-graph fire: resolve the graph's owning package + artifact digest,
//! pick a live agent, ship a [`GraphWorkPacket`] over the substrate outbox,
//! and await the agent's result through the same uuid→result coordinator
//! rendezvous the task path uses (`/v1/agent/result` is a pure forward).
//!
//! **Fallback policy** (the reactor's hot path must never wedge):
//! - PRE-dispatch failures — no owning package (embedded graph), a Python
//!   package (interpreted CGs stay in-process in v1), no eligible agent,
//!   snapshot conversion or enqueue errors — run the firing **in-process**
//!   via the closure every [`GraphFireEvent`] carries, with a warning.
//! - POST-dispatch failures — the agent reported an error, or the result
//!   wait timed out — surface as a [`GraphResult`] error WITHOUT a local
//!   re-run: the agent may have executed (or still be executing) the graph,
//!   and double-running a firing is worse than reporting it failed.
//!
//! CLOACINA-T-1125: `--fleet-graph-firings` says where the agent runs a
//! firing. `inline` (the default): in the agent process. `worker`: in a
//! worker process, held to the memory share of a task with no request and to
//! the packet's time limit. When the result wait ends with no report the
//! server sends a cancel that names the firing id, and the agent kills that
//! worker (an inline firing is not stopped).

use std::sync::Arc;

use cloacina::cloacina_computation_graph::{GraphError, GraphResult};
use cloacina::computation_graph::graph_executor::{GraphExecutor, GraphFireEvent};
use cloacina::computation_graph::packaging_bridge::input_cache_to_ffi_cache;
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::fleet::{
    AGENT_PROTOCOL_VERSION, AGENT_RECIPIENT_PREFIX, AgentOutcome, ArtifactRef, GRAPH_PACKET_KIND,
    GraphWorkPacket, host_target_triple,
};
use cloacina::models::delivery_outbox::NewDeliveryOutbox;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use tracing::{debug, info, warn};

use crate::agent_registry::{AgentRegistry, FleetLoad, TaskDemand, select_fleet_agent};
use crate::fleet_coordinator::FleetCoordinator;

use crate::fleet_executor::{FleetTaskTimeLimits, TaskTimeLimit, send_cancel};

/// Where an agent runs a graph firing (CLOACINA-T-1125):
/// `--fleet-graph-firings` / `CLOACINA_FLEET_GRAPH_FIRINGS`.
///
/// `inline` (the default) runs the firing in the agent process: no worker
/// start and no package load per firing, and no memory limit, kill or
/// cancel. `worker` runs each firing in a worker process, with the limits,
/// time limit and cancel of a task. Measured on a debug build, a small Rust
/// graph firing took about 210 ms in a worker and 0.2 ms inline, and a
/// reactor waits for each firing before the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum GraphFiringExecution {
    #[default]
    Inline,
    Worker,
}

impl GraphFiringExecution {
    /// The `inline` field of the graph packet.
    pub fn inline(self) -> bool {
        self == GraphFiringExecution::Inline
    }
}

pub struct FleetGraphExecutor {
    dal: cloacina::dal::DAL,
    /// Admin-schema DAL for the delivery_outbox enqueue (delivery is
    /// server-global — see `FleetExecutor::outbox_dal`).
    outbox_dal: cloacina::dal::DAL,
    agent_registry: Arc<AgentRegistry>,
    coordinator: Arc<FleetCoordinator>,
    delivery_wake: cloacina::delivery::WakeHandle,
    /// Registry handle for resolving a graph's owning package by surface
    /// name at fire time (`find_package_for_surface("reactor", …)`).
    registry: WorkflowRegistryImpl<UnifiedRegistryStorage>,
    /// The time limit of a firing (CLOACINA-T-1039): the server's default
    /// fleet task time limit. A graph has no limit of its own, so every
    /// firing gets the default. The packet carries it, and the server waits
    /// for the result that long plus the grace of the task path.
    time_limit: TaskTimeLimit,
    /// Where the agent runs a firing (CLOACINA-T-1125).
    firing_execution: GraphFiringExecution,
}

/// The time limit of a graph firing under the server's fleet time limits: the
/// default limit, the one a task with no limit of its own gets.
fn graph_time_limit(limits: &FleetTaskTimeLimits) -> TaskTimeLimit {
    limits.limit_for(None)
}

/// The packet of one firing. `time_limit` is what the agent is told and what
/// the server's result wait is built from.
#[allow(clippy::too_many_arguments)]
fn graph_work_packet(
    firing_id: UniversalUuid,
    graph_name: &str,
    tenant_id: Option<String>,
    cache: std::collections::HashMap<String, String>,
    digest: String,
    build_target_triple: String,
    language: String,
    time_limit: TaskTimeLimit,
    firing_execution: GraphFiringExecution,
) -> GraphWorkPacket {
    GraphWorkPacket {
        protocol_version: AGENT_PROTOCOL_VERSION,
        firing_id: firing_id.0.to_string(),
        graph_name: graph_name.to_string(),
        cache,
        artifact: ArtifactRef {
            fetch_url: format!("/v1/agent/artifact/{}", digest),
            digest,
            build_target_triple,
        },
        timeout_seconds: time_limit.seconds,
        tenant_id,
        language: Some(language),
        // CLOACI-T-0861 / D-5 — DOCUMENTED SEAM (graph-FFI path). Unlike the
        // per-task path (fully wired in `fleet_executor.rs` step 3d), a
        // whole-graph firing has no per-task `Context` seam on the agent
        // (`process_graph_packet` executes via FFI `execute_graph(cache)` with
        // no `Context::secret` accessor to attach a resolver to). The one-time
        // key POOL, consume, and wrap mechanism are all in place — wiring this
        // path needs an agent-side graph secrets accessor first. Until then a
        // graph firing carries no secrets. AAD would use `firing_id`.
        wrapped_secrets: Vec::new(),
        secret_key_id: None,
        // CLOACINA-T-1125: the server setting decides where the agent runs
        // the firing; a graph has no inline mark of its own.
        inline: firing_execution.inline(),
    }
}

impl FleetGraphExecutor {
    pub fn new(
        dal: cloacina::dal::DAL,
        outbox_dal: cloacina::dal::DAL,
        agent_registry: Arc<AgentRegistry>,
        coordinator: Arc<FleetCoordinator>,
        delivery_wake: cloacina::delivery::WakeHandle,
        registry: WorkflowRegistryImpl<UnifiedRegistryStorage>,
    ) -> Self {
        Self {
            dal,
            outbox_dal,
            agent_registry,
            coordinator,
            delivery_wake,
            registry,
            time_limit: graph_time_limit(&FleetTaskTimeLimits::default()),
            firing_execution: GraphFiringExecution::default(),
        }
    }

    /// Set where the agent runs a firing (`--fleet-graph-firings`).
    pub fn with_firing_execution(mut self, firing_execution: GraphFiringExecution) -> Self {
        self.firing_execution = firing_execution;
        self
    }

    /// Set the fleet time limits (`--fleet-task-timeout-s`,
    /// `--fleet-task-timeout-max-s`). A firing gets the default limit.
    pub fn with_task_time_limits(mut self, time_limits: FleetTaskTimeLimits) -> Self {
        self.time_limit = graph_time_limit(&time_limits);
        self
    }

    /// Attempt fleet dispatch. `Err(reason)` = pre-dispatch failure — the
    /// caller falls back to in-process execution.
    async fn try_dispatch(&self, fire: &GraphFireEvent) -> Result<GraphResult, String> {
        // 1. Resolve the graph's owning package: by its reactor surface name
        //    first, then by any of the firing's accumulator sources (some
        //    packages — notably Python CGs — declare accumulator surfaces but
        //    not a reactor surface; same fallback the health API uses).
        let mut package = self
            .registry
            .find_package_for_surface("reactor", &fire.graph_name)
            .await
            .map_err(|e| format!("surface→package lookup failed: {e}"))?;
        if package.is_none() {
            for source in fire.snapshot.sources() {
                if let Ok(Some(p)) = self
                    .registry
                    .find_package_for_surface("accumulator", source.as_str())
                    .await
                {
                    package = Some(p);
                    break;
                }
            }
        }
        let package_name = package.ok_or_else(|| {
            format!(
                "graph '{}' has no owning package (embedded graph)",
                fire.graph_name
            )
        })?;

        // 2. Resolve the active artifact. Interpreted (Python) CGs stay
        //    in-process in v1 — their compiled graph fn is a live PyObject,
        //    not a shippable artifact.
        //    CLOACINA-T-1018: a graph fire has no workflow execution, so it
        //    always runs on the active version; the per-target lookups below
        //    are scoped to that version.
        let active = self
            .dal
            .workflow_packages()
            .get_active_dispatch(&package_name, None)
            .await
            .map_err(|e| format!("artifact digest lookup failed: {e}"))?
            .ok_or_else(|| format!("no active artifact for package '{package_name}'"))?;
        let (primary_digest, language, package_version, package_interface) = (
            active.content_hash,
            active.language,
            active.version,
            active.plugin_interface_version,
        );

        // 3. Select a live agent with capacity + a runnable arch in the
        //    graph's tenant. An interpreted (Python) package runs from source
        //    on ANY arch (CLOACI-T-0841); a compiled one needs the host
        //    primary ∪ this package's per-target builds.
        let interpreted = language.eq_ignore_ascii_case("python");
        let runnable_triples: Option<Vec<String>> = if interpreted {
            None
        } else {
            let mut t = self
                .dal
                .workflow_packages()
                .get_artifact_triples_for_package(&package_name, &package_version)
                .await
                .unwrap_or_default();
            t.push(host_target_triple().to_string());
            Some(t)
        };
        // CLOACI-T-0916: read the DB-backed live roster, so agents registered
        // via any replica are eligible (the packet rides the delivery outbox
        // to whichever replica holds the agent's socket).
        let snapshot = self
            .agent_registry
            .live_agents()
            .await
            .map_err(|e| format!("fleet roster read failed: {e}"))?;
        // CLOACINA-T-1086: only an agent that loads the package's plugin
        // interface version.
        if let Some(reason) = crate::agent_registry::plugin_interface_wait_reason(
            &snapshot,
            &fire.tenant_id,
            cloacina_api_types::DEFAULT_AGENT_POOL,
            package_interface,
        ) {
            return Err(reason);
        }
        let snapshot = crate::agent_registry::agents_loading(&snapshot, package_interface);
        let agent = select_fleet_agent(
            &snapshot,
            &fire.tenant_id,
            &runnable_triples,
            // CLOACINA-T-1027: a graph firing names no pool and has no
            // request. It goes to the pool `default` and is packed by the
            // agent's free slots; it is not in the load of placed tasks.
            &TaskDemand::pool(cloacina_api_types::DEFAULT_AGENT_POOL),
            &FleetLoad::new(),
        )
        .ok_or_else(|| {
            format!(
                "no live agent with capacity + runnable arch in tenant {:?}",
                fire.tenant_id
            )
        })?;
        let agent_id = agent.agent_id.clone();
        let agent_triple = agent.target_triple.clone();

        // Per-target artifact when one exists for the agent's arch, else the
        // primary/host build (selection already guaranteed runnability). An
        // interpreted package always ships its primary (source) digest.
        let (digest, _triple) = if interpreted {
            (primary_digest, agent_triple)
        } else {
            match self
                .dal
                .workflow_packages()
                .get_artifact_digest_for_target(&package_name, &package_version, &agent_triple)
                .await
            {
                Ok(Some(arch_digest)) => (arch_digest, agent_triple),
                _ => (primary_digest, host_target_triple().to_string()),
            }
        };

        // 4. Convert the snapshot into the FFI/wire cache shape — the same
        //    conversion the in-process FFI path performs.
        let cache = input_cache_to_ffi_cache(&fire.snapshot)
            .map_err(|e| format!("snapshot conversion failed: {e}"))?;

        // 5. Rendezvous BEFORE enqueue (a fast agent must not race the receiver).
        let firing_id = UniversalUuid::new_v4();
        // CLOACINA-T-1050: a firing has no dispatch id (the firing id names
        // one dispatch). The wait takes a report only from this tenant and
        // this agent, as a task dispatch does.
        let rx = self.coordinator.register_dispatch(
            firing_id,
            cloacina::security::ownership::DispatchTarget {
                tenant_id: fire.tenant_id.clone(),
                agent_id: agent_id.clone(),
                dispatch_id: None,
            },
        );

        let packet = graph_work_packet(
            firing_id,
            &fire.graph_name,
            fire.tenant_id.clone(),
            cache,
            digest,
            agent.target_triple.clone(),
            language,
            self.time_limit,
            self.firing_execution,
        );
        let payload = serde_json::to_vec(&packet).map_err(|e| {
            self.coordinator.cancel(firing_id);
            format!("serialize GraphWorkPacket: {e}")
        })?;

        let row = NewDeliveryOutbox {
            recipient: format!("{}{}", AGENT_RECIPIENT_PREFIX, agent_id),
            kind: GRAPH_PACKET_KIND.to_string(),
            tenant_id: fire.tenant_id.clone(),
            payload,
        };
        let packet_row_id = match self.outbox_dal.delivery_outbox().enqueue(row).await {
            Ok(row) => {
                self.coordinator.packet_enqueued(firing_id, row.id);
                row.id
            }
            Err(e) => {
                self.coordinator.cancel(firing_id);
                return Err(format!("delivery_outbox enqueue: {e}"));
            }
        };
        self.delivery_wake.wake();
        debug!(
            graph = %fire.graph_name,
            agent_id = %agent_id,
            firing_id = %firing_id,
            "fleet: graph firing enqueued + relay woken"
        );

        // 6. Await the agent's result. POST-dispatch failures do NOT fall
        //    back (the agent may have run — or still be running — the graph).
        //    The wait is the firing's time limit plus the grace of the task
        //    path (CLOACINA-T-1039).
        let result_wait = self.time_limit.result_wait();
        match tokio::time::timeout(result_wait, rx).await {
            Ok(Ok(result)) => match result.outcome {
                AgentOutcome::Success { context } => {
                    // The agent reports terminal outputs as a JSON array under
                    // "outputs" (may be empty — the reactor only logs them).
                    let outputs_json: Vec<serde_json::Value> = context
                        .get("outputs")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let outputs: Vec<Box<dyn std::any::Any + Send>> = outputs_json
                        .iter()
                        .cloned()
                        .map(|v| Box::new(v) as Box<dyn std::any::Any + Send>)
                        .collect();
                    info!(
                        graph = %fire.graph_name,
                        agent_id = %result.agent_id,
                        duration_ms = result.duration_ms,
                        "fleet: graph firing completed on agent"
                    );
                    Ok(GraphResult::completed_with_json(outputs, outputs_json))
                }
                AgentOutcome::Failure {
                    message,
                    classification: _,
                } => Ok(GraphResult::error(GraphError::NodeExecution(format!(
                    "graph firing failed on agent {}: {}",
                    result.agent_id, message
                )))),
                AgentOutcome::Refused { reason, message } => {
                    // The agent did NOT run the graph — safe to fall back.
                    Err(format!(
                        "agent {} refused the graph firing ({reason:?}): {message}",
                        result.agent_id
                    ))
                }
            },
            Ok(Err(_canceled)) => Ok(GraphResult::error(GraphError::NodeExecution(
                "fleet graph rendezvous canceled before the agent reported".to_string(),
            ))),
            Err(_elapsed) => {
                self.coordinator.cancel(firing_id);
                // CLOACINA-T-1125: the server gives up on this firing. Ask the
                // agent that has the packet to stop its worker; a firing's
                // cancel names its firing id.
                let firing = firing_id.0.to_string();
                send_cancel(
                    &self.outbox_dal,
                    &self.delivery_wake,
                    &firing,
                    Some(packet_row_id),
                    &agent_id,
                    fire.tenant_id.clone(),
                    &firing,
                    "the server's wait for the graph firing's result ended",
                )
                .await;
                Ok(GraphResult::error(GraphError::NodeExecution(format!(
                    "fleet graph result wait exceeded {}s (agent {}) — NOT re-running \
                     in-process to avoid a double execution",
                    result_wait.as_secs(),
                    agent_id
                ))))
            }
        }
    }
}

#[async_trait::async_trait]
impl GraphExecutor for FleetGraphExecutor {
    async fn execute(&self, fire: GraphFireEvent) -> GraphResult {
        match self.try_dispatch(&fire).await {
            Ok(result) => result,
            Err(reason) => {
                warn!(
                    graph = %fire.graph_name,
                    reason = %reason,
                    "fleet: graph dispatch not possible — running firing in-process"
                );
                (fire.in_process)(fire.snapshot).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet_executor::RESULT_WAIT_GRACE;
    use std::time::Duration;

    fn packet_with(limits: &FleetTaskTimeLimits) -> GraphWorkPacket {
        packet_run(limits, GraphFiringExecution::default())
    }

    fn packet_run(
        limits: &FleetTaskTimeLimits,
        firing_execution: GraphFiringExecution,
    ) -> GraphWorkPacket {
        graph_work_packet(
            UniversalUuid::new_v4(),
            "pricing",
            Some("acme".to_string()),
            std::collections::HashMap::new(),
            "abc123".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
            "rust".to_string(),
            graph_time_limit(limits),
            firing_execution,
        )
    }

    /// CLOACINA-T-1125: by default the agent runs a firing inline; the
    /// setting `worker` puts it in a worker process.
    #[test]
    fn a_firing_is_inline_by_default_and_in_a_worker_when_set() {
        let limits = FleetTaskTimeLimits::default();
        assert_eq!(
            GraphFiringExecution::default(),
            GraphFiringExecution::Inline
        );
        let default = packet_with(&limits);
        assert!(default.inline);
        let wire: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&default).unwrap()).unwrap();
        assert_eq!(wire["inline"], true);
        assert!(!packet_run(&limits, GraphFiringExecution::Worker).inline);
    }

    #[test]
    fn graph_packet_carries_the_configured_default_limit() {
        let limits = FleetTaskTimeLimits::from_seconds(900, 3600).unwrap();
        let packet = packet_with(&limits);
        assert_eq!(packet.timeout_seconds, 900);
        assert_eq!(packet.graph_name, "pricing");
        assert_eq!(packet.artifact.fetch_url, "/v1/agent/artifact/abc123");
        // The wire form the agent reads holds the same value.
        let wire: serde_json::Value =
            serde_json::from_slice(&serde_json::to_vec(&packet).unwrap()).unwrap();
        assert_eq!(wire["timeout_seconds"], 900);
    }

    #[test]
    fn graph_packet_with_no_setting_carries_the_server_default() {
        let packet = packet_with(&FleetTaskTimeLimits::default());
        assert_eq!(
            u64::from(packet.timeout_seconds),
            FleetTaskTimeLimits::DEFAULT_SECONDS
        );
    }

    #[test]
    fn the_result_wait_is_the_limit_plus_the_grace() {
        let limits = FleetTaskTimeLimits::from_seconds(900, 3600).unwrap();
        assert_eq!(
            graph_time_limit(&limits).result_wait(),
            Duration::from_secs(900) + RESULT_WAIT_GRACE
        );
    }
}
