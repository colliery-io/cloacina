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

use std::sync::Arc;
use std::time::Duration;

use cloacina::cloacina_computation_graph::{GraphError, GraphResult};
use cloacina::computation_graph::graph_executor::{GraphExecutor, GraphFireEvent};
use cloacina::computation_graph::packaging_bridge::input_cache_to_ffi_cache;
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::database::universal_types::UniversalUuid;
use cloacina::fleet::{
    host_target_triple, AgentOutcome, ArtifactRef, GraphWorkPacket, AGENT_PROTOCOL_VERSION,
    AGENT_RECIPIENT_PREFIX, GRAPH_PACKET_KIND,
};
use cloacina::models::delivery_outbox::NewDeliveryOutbox;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use tracing::{debug, info, warn};

use crate::agent_registry::AgentRegistry;
use crate::fleet_coordinator::FleetCoordinator;
use crate::fleet_executor::select_fleet_agent;

/// Max wall-clock to wait for an agent's graph result before reporting the
/// firing failed. Matches the task path's conservative cap.
const GRAPH_RESULT_WAIT_TIMEOUT: Duration = Duration::from_secs(300);

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
        }
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
        let (primary_digest, language) = self
            .dal
            .workflow_packages()
            .get_active_dispatch_for_package(&package_name, None)
            .await
            .map_err(|e| format!("artifact digest lookup failed: {e}"))?
            .ok_or_else(|| format!("no active artifact for package '{package_name}'"))?;

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
                .get_artifact_triples_for_package(&package_name)
                .await
                .unwrap_or_default();
            t.push(host_target_triple().to_string());
            Some(t)
        };
        let snapshot = self.agent_registry.snapshot();
        let agent =
            select_fleet_agent(&snapshot, &fire.tenant_id, &runnable_triples).ok_or_else(|| {
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
                .get_artifact_digest_for_target(&package_name, &agent_triple)
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
        let rx = self.coordinator.register_pending(firing_id);

        let packet = GraphWorkPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            firing_id: firing_id.0.to_string(),
            graph_name: fire.graph_name.clone(),
            cache,
            artifact: ArtifactRef {
                fetch_url: format!("/v1/agent/artifact/{}", digest),
                digest,
                build_target_triple: agent.target_triple.clone(),
            },
            timeout_seconds: 300,
            tenant_id: fire.tenant_id.clone(),
            language: Some(language),
        };
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
        if let Err(e) = self.outbox_dal.delivery_outbox().enqueue(row).await {
            self.coordinator.cancel(firing_id);
            return Err(format!("delivery_outbox enqueue: {e}"));
        }
        self.delivery_wake.wake();
        debug!(
            graph = %fire.graph_name,
            agent_id = %agent_id,
            firing_id = %firing_id,
            "fleet: graph firing enqueued + relay woken"
        );

        // 6. Await the agent's result. POST-dispatch failures do NOT fall
        //    back (the agent may have run — or still be running — the graph).
        match tokio::time::timeout(GRAPH_RESULT_WAIT_TIMEOUT, rx).await {
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
                Ok(GraphResult::error(GraphError::NodeExecution(format!(
                    "fleet graph result wait exceeded {}s (agent {}) — NOT re-running \
                     in-process to avoid a double execution",
                    GRAPH_RESULT_WAIT_TIMEOUT.as_secs(),
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
