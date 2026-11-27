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

//! Execution-agent fleet API types (CLOACI-I-0124 / WS-0b).

use serde::{Deserialize, Serialize};

/// One registered execution agent in the in-memory fleet roster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentInfo {
    pub agent_id: String,
    pub target_triple: String,
    pub max_concurrency: u32,
    pub in_flight: u32,
    pub available_capacity: u32,
    /// Seconds since this agent's last heartbeat — the liveness signal an
    /// operator reads (the underlying record stores a monotonic `Instant`,
    /// not a wall-clock time).
    pub seconds_since_heartbeat: u64,
    pub capabilities: Vec<String>,
    /// Tenant scope the agent registered under, if any.
    pub tenant_id: Option<String>,
}

/// The name of the pool every tenant has. An agent that registers no pool is
/// in it, and a task that names no pool runs in it.
pub const DEFAULT_AGENT_POOL: &str = "default";

/// Prefix of the capability an agent registers to say which pool it is in
/// (`pool:<name>`).
pub const AGENT_POOL_CAPABILITY_PREFIX: &str = "pool:";

/// The pool an agent is in, from the capabilities it registered: the first
/// `pool:<name>` capability, or [`DEFAULT_AGENT_POOL`] when it has none.
pub fn agent_pool_of(capabilities: &[String]) -> &str {
    capabilities
        .iter()
        .find_map(|c| c.strip_prefix(AGENT_POOL_CAPABILITY_PREFIX))
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_AGENT_POOL)
}

/// Settings of one agent pool of a tenant (CLOACINA-T-1021).
///
/// Each field is optional. A field that is absent means the server-wide
/// default (`CLOACINA_AGENT_*` on the server). The settings apply to every
/// managed agent of the pool: a pool has one size of agent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct AgentPoolSettings {
    /// CPU request of the agent container, a Kubernetes quantity (`500m`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_request: Option<String>,
    /// Memory request of the agent container, a Kubernetes quantity (`8Gi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_request: Option<String>,
    /// CPU limit of the agent container, a Kubernetes quantity (`4`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_limit: Option<String>,
    /// Memory limit of the agent container, a Kubernetes quantity (`32Gi`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<String>,
    /// How many tasks one agent runs at the same time (the agent's
    /// `--max-concurrency`). Absent: the agent's own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<u32>,
    /// Agent image. Absent: the server-wide `CLOACINA_AGENT_IMAGE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Name of the Kubernetes ServiceAccount the agent pods run as. The
    /// actuator creates it in the tenant namespace. Absent: the namespace's
    /// `default` ServiceAccount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_account_name: Option<String>,
    /// Annotations the actuator puts on that ServiceAccount (for a cloud
    /// workload identity). Needs `service_account_name`.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub service_account_annotations: std::collections::BTreeMap<String, String>,
    /// Labels added to the pool's pods (the pod template of its Deployment).
    /// Some identity systems select pods by a label. The labels the actuator
    /// sets itself are refused.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub pod_labels: std::collections::BTreeMap<String, String>,
    /// Annotations added to the pool's pods (the pod template of its
    /// Deployment).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub pod_annotations: std::collections::BTreeMap<String, String>,
    /// The pod's `automountServiceAccountToken`. Absent means `false`: the
    /// pod mounts no ServiceAccount token. The agent does not call the
    /// Kubernetes API. Set `true` for a pool whose identity system needs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automount_service_account_token: Option<bool>,
    /// Extra egress rules for the pool's pods, added to the default (cluster
    /// DNS and the server only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_egress: Vec<AgentEgressRule>,
    /// Fewest agents of the pool. Not for the pool `default` (its floor is
    /// the server-wide `CLOACINA_AUTOSCALE_FLOOR`). A pool other than
    /// `default` is not autoscaled yet: set it equal to `max_agents`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_agents: Option<u32>,
    /// Most agents of the pool. Not for the pool `default` (its max is the
    /// tenant's agent limit, `/v1/tenants/{tenant_id}/limits`). A pool other
    /// than `default` runs exactly this many agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_agents: Option<u32>,
}

/// One extra egress rule: a CIDR block the agent pods may reach, on the listed
/// ports. No ports means every port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct AgentEgressRule {
    /// IPv4 or IPv6 CIDR block, with no host bits set (`198.51.100.0/24`).
    pub cidr: String,
    /// Ports the rule allows. Empty: every port.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<AgentEgressPort>,
}

/// One port of an [`AgentEgressRule`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct AgentEgressPort {
    /// Port number, 1 to 65535.
    pub port: u16,
    /// `TCP` (the default), `UDP` or `SCTP`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
}

/// One agent pool of a tenant, as the pool routes return it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AgentPoolInfo {
    pub tenant_id: String,
    /// Pool name. Every tenant has `default`.
    pub pool: String,
    /// The pool's settings as stored. A field that is absent means the
    /// server-wide default.
    pub settings: AgentPoolSettings,
    /// How many agents the server keeps for the pool: the tenant's desired
    /// count for `default`, the pool's `max_agents` for another pool.
    pub desired_agents: u32,
    /// How many agents of the pool are registered and alive now.
    pub live_agents: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn agent_pool_of_reads_the_pool_capability() {
        assert_eq!(agent_pool_of(&caps(&[])), "default");
        assert_eq!(agent_pool_of(&caps(&["python", "gpu"])), "default");
        assert_eq!(agent_pool_of(&caps(&["python", "pool:big"])), "big");
        assert_eq!(agent_pool_of(&caps(&["pool:default"])), "default");
        // An empty name is no pool; the first pool capability wins.
        assert_eq!(agent_pool_of(&caps(&["pool:"])), "default");
        assert_eq!(agent_pool_of(&caps(&["pool:a", "pool:b"])), "a");
        // Only the exact prefix counts.
        assert_eq!(agent_pool_of(&caps(&["pools:big", "spool:big"])), "default");
    }

    #[test]
    fn pool_settings_serialize_only_what_is_set() {
        assert_eq!(
            serde_json::to_string(&AgentPoolSettings::default()).unwrap(),
            "{}"
        );
        let s: AgentPoolSettings =
            serde_json::from_str(r#"{"memory_limit":"32Gi","max_agents":2}"#).unwrap();
        assert_eq!(s.memory_limit.as_deref(), Some("32Gi"));
        assert_eq!(s.max_agents, Some(2));
        // A misspelt field is refused, not ignored.
        assert!(serde_json::from_str::<AgentPoolSettings>(r#"{"memory_limt":"32Gi"}"#).is_err());
    }
}
