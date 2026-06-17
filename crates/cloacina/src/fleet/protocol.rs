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

//! Wire types for the execution-agent fleet protocol (CLOACI-T-0631).
//!
//! ## Direction split
//!
//! - **Server → agent**: rides the substrate (S-0012 / T-0626 / T-0627).
//!   `cloacina::delivery::ServerMessage::Push` with `kind == WORK_PACKET_KIND`
//!   carries a JSON-serialized [`WorkPacket`]. Agents subscribe by connecting
//!   to `/v1/ws/delivery/{AGENT_RECIPIENT_PREFIX}{agent_id}` and acking each
//!   pushed packet on receipt with the substrate `ClientMessage::Ack`.
//! - **Agent → server**: REST POSTs ([`AgentRegisterRequest`],
//!   [`AgentHeartbeatRequest`], [`AgentResultRequest`]). The substrate
//!   envelope is intentionally narrow — extending its `ClientMessage` with
//!   fleet-specific frames would couple substrate generality to fleet
//!   internals.
//!
//! ## Idempotency + at-least-once
//!
//! Inherited from the substrate. Agents must tolerate seeing the same
//! [`WorkPacket`] more than once (after a disconnect/reconnect, after a
//! sweeper redelivery). The server's `FleetExecutor` reconciliation handles
//! result-frame idempotency by way of the shared
//! [`crate::executor::TaskResultHandler`] (T-0630).
//!
//! ## OQ-6: target-triple fail-closed
//!
//! Every [`ArtifactRef`] carries the cdylib's `build_target_triple`; every
//! [`AgentRegisterRequest`] carries the agent's own `target_triple`. The
//! agent refuses (returns [`AgentOutcome::Refused`] with
//! [`RefusalReason::TargetTripleMismatch`]) rather than attempting a
//! cross-target `dlopen` that would crash mysteriously. v1 fleet is
//! homogeneous; this is the guard that makes a misconfiguration loud.

use serde::{Deserialize, Serialize};

/// Wire-protocol version for the agent fleet. Bumped on backwards-incompatible
/// changes. Every frame carries it so peers can negotiate (or refuse).
pub const AGENT_PROTOCOL_VERSION: u32 = 1;

/// Substrate envelope `kind` value used when a `ServerMessage::Push` carries
/// a [`WorkPacket`]. Constants here keep the magic string out of call sites.
pub const WORK_PACKET_KIND: &str = "agent_work";

/// Substrate recipient prefix for agent connections. The full recipient
/// string for an agent is `format!("{}{}", AGENT_RECIPIENT_PREFIX, agent_id)`.
pub const AGENT_RECIPIENT_PREFIX: &str = "agent:";

/// Default suggested heartbeat interval the server returns to a registering
/// agent if no operator override is configured.
pub const DEFAULT_HEARTBEAT_INTERVAL_SECONDS: u32 = 15;

/// Best-effort host target triple. v1 simplification: `<arch>-<os>` derived
/// from `std::env::consts` — doesn't distinguish glibc vs musl, etc. Both
/// the server (when stamping `ArtifactRef::build_target_triple`) and the
/// agent (when reporting its own `target_triple` and doing the fail-closed
/// comparison) use this same function so the OQ-6 check is exact-string.
/// Per-artifact full-triple tracking is future work.
pub fn host_target_triple() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

// ─────────────────────────────────────────────────────────────────────────
// Registration: POST /v1/agent/register
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegisterRequest {
    pub protocol_version: u32,
    /// Caller-chosen agent id (e.g. hostname + pid hash). If `None` the server
    /// assigns a fresh one and returns it in [`AgentRegisterResponse::agent_id`].
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Maximum concurrent tasks this agent will accept.
    pub max_concurrency: u32,
    /// Target triple the agent is running on (e.g. `aarch64-apple-darwin`).
    /// OQ-6 fail-closed: the `FleetExecutor` only assigns work whose
    /// `ArtifactRef::build_target_triple` matches this.
    pub target_triple: String,
    /// Free-form capability tags the `FleetExecutor` can route on
    /// (e.g. `gpu`, `large_memory`).
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegisterResponse {
    pub protocol_version: u32,
    /// The id the `FleetExecutor` will use to address this agent. The agent
    /// MUST connect to the substrate WS at
    /// `/v1/ws/delivery/{AGENT_RECIPIENT_PREFIX}{agent_id}`.
    pub agent_id: String,
    /// Server-suggested heartbeat cadence. The agent should heartbeat at
    /// least this often; the server marks an agent dead after a small
    /// multiple of missed intervals.
    pub heartbeat_interval_seconds: u32,
}

// ─────────────────────────────────────────────────────────────────────────
// Heartbeat: POST /v1/agent/heartbeat
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeatRequest {
    pub protocol_version: u32,
    pub agent_id: String,
    /// Number of work packets currently in flight on this agent.
    pub in_flight: u32,
    /// Currently-available capacity (`max_concurrency - in_flight`). The
    /// `FleetExecutor` uses this for selection.
    pub available_capacity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeatResponse {
    pub protocol_version: u32,
}

// ─────────────────────────────────────────────────────────────────────────
// Work packet: substrate Push payload (server → agent over WS)
// ─────────────────────────────────────────────────────────────────────────

/// Fully self-contained work packet — everything a DB-less agent needs to
/// run one task without ever touching the database.
///
/// Serialized as JSON into the substrate `Push.payload_b64`. On Postgres the
/// outbox row that carries this is enqueued in the same transaction as the
/// state change that produced the work (by the `FleetExecutor`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkPacket {
    pub protocol_version: u32,
    pub task_execution_id: String,
    pub workflow_execution_id: String,
    pub task_name: String,
    pub attempt: i32,
    /// Merged dependency context the task closure consumes — eagerly
    /// resolved by the server because the agent has no DAL. (For very large
    /// contexts a future variant may swap inline JSON for a context-fetch
    /// REST reference; OQ-1.)
    pub context: serde_json::Value,
    /// Pointer to the cdylib artifact the agent must `dlopen`.
    pub artifact: ArtifactRef,
    /// Per-task execution timeout.
    pub timeout_seconds: u32,
    /// Tenant scope. The agent's authenticated context must match this to
    /// even receive the packet; included here so the agent can pass it into
    /// the runtime when constructing the task's execution scope.
    pub tenant_id: Option<String>,
    /// Package language, so the agent loads it the right way: `"rust"` (or
    /// absent, for older servers) → `dlopen` the cdylib at `artifact`;
    /// `"python"` → fetch the source archive and import it via PyO3. Defaults
    /// to `"rust"` when missing so a packet from a pre-CLOACI-T-0716 server is
    /// still handled as before. (CLOACI-T-0716)
    #[serde(default)]
    pub language: Option<String>,
}

/// Reference to a workflow artifact (cdylib) the agent must fetch + load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Content-addressed digest. Matches `workflow_packages.content_hash`.
    pub digest: String,
    /// REST URL the agent fetches from (relative or absolute). Typically
    /// `/v1/agent/artifact/{digest}` on the server.
    pub fetch_url: String,
    /// Target triple the cdylib was built for (OQ-6 fail-closed). The agent
    /// MUST compare to its own `target_triple` and refuse on mismatch rather
    /// than attempt `dlopen`.
    pub build_target_triple: String,
}

// ─────────────────────────────────────────────────────────────────────────
// Result report: POST /v1/agent/result
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResultRequest {
    pub protocol_version: u32,
    pub agent_id: String,
    pub task_execution_id: String,
    /// Echoed from the original work packet so the server can reject stale
    /// reports (an agent reporting attempt N on a row already retried to N+1).
    pub attempt: i32,
    pub duration_ms: u64,
    pub outcome: AgentOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResultResponse {
    pub protocol_version: u32,
}

/// Outcome of one work packet as reported by the agent. The server's
/// `FleetExecutor` reconciliation maps these onto the shared
/// `crate::executor::TaskResultHandler::handle_outcome` Result variant,
/// guaranteeing the thread and fleet executors agree on status / retry /
/// context-persist semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentOutcome {
    /// Task closure returned successfully with a produced output context.
    Success { context: serde_json::Value },
    /// Task closure returned an error.
    Failure {
        message: String,
        classification: FailureClassification,
    },
    /// Agent refused to run the work packet (pre-execution). Server should
    /// treat as transient and reschedule onto a different agent.
    Refused {
        reason: RefusalReason,
        message: String,
    },
}

/// Bounded classification of a task-level failure so the server's retry
/// decision is consistent with what `TaskResultHandler::is_transient_error`
/// would have decided locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClassification {
    /// Task code returned an error (analog of `ExecutorError::TaskExecution`).
    /// Retried only if the task's `RetryPolicy` says so.
    TaskError,
    /// Timeout, network, or other clearly-transient failure — preferred for retry.
    Transient,
    /// Invalid input / context / configuration. No retry.
    Validation,
    /// Task ran past its `timeout_seconds`.
    Timeout,
}

/// Why the agent refused to even run the packet. Treated as transient by the
/// server (reschedule to a different agent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    /// `agent.target_triple != artifact.build_target_triple` (OQ-6 fail-closed).
    TargetTripleMismatch,
    /// Artifact REST fetch failed (server unreachable, 404, 5xx, IO error).
    ArtifactFetchFailed,
    /// `dlopen` / runtime load failed (corrupted cdylib, missing symbol).
    RuntimeLoadFailed,
    /// Agent is draining or shutting down.
    Shutdown,
    /// Server tried to route a packet whose tenant the agent isn't authorized for.
    TenantMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_packet_round_trips_as_json() {
        let p = WorkPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            task_execution_id: "t1".into(),
            workflow_execution_id: "w1".into(),
            task_name: "ns::task".into(),
            attempt: 1,
            context: serde_json::json!({"k": 42}),
            artifact: ArtifactRef {
                digest: "deadbeef".into(),
                fetch_url: "/v1/agent/artifact/deadbeef".into(),
                build_target_triple: "aarch64-apple-darwin".into(),
            },
            timeout_seconds: 60,
            tenant_id: Some("t1".into()),
            language: Some("rust".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: WorkPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(back.task_execution_id, "t1");
        assert_eq!(back.artifact.build_target_triple, "aarch64-apple-darwin");
        assert_eq!(back.context, serde_json::json!({"k": 42}));
    }

    #[test]
    fn outcome_variants_round_trip_with_snake_case_tags() {
        let success = AgentOutcome::Success {
            context: serde_json::json!({}),
        };
        let json = serde_json::to_string(&success).unwrap();
        assert!(json.contains("\"kind\":\"success\""));
        let back: AgentOutcome = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, AgentOutcome::Success { .. }));

        let failure = AgentOutcome::Failure {
            message: "oops".into(),
            classification: FailureClassification::Transient,
        };
        let json = serde_json::to_string(&failure).unwrap();
        assert!(json.contains("\"kind\":\"failure\""));
        assert!(json.contains("\"classification\":\"transient\""));
        let back: AgentOutcome = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back,
            AgentOutcome::Failure {
                classification: FailureClassification::Transient,
                ..
            }
        ));

        let refused = AgentOutcome::Refused {
            reason: RefusalReason::TargetTripleMismatch,
            message: "expected x86_64, got aarch64".into(),
        };
        let json = serde_json::to_string(&refused).unwrap();
        assert!(json.contains("\"reason\":\"target_triple_mismatch\""));
        let back: AgentOutcome = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back,
            AgentOutcome::Refused {
                reason: RefusalReason::TargetTripleMismatch,
                ..
            }
        ));
    }

    #[test]
    fn agent_recipient_prefix_is_stable() {
        // T-0633's FleetExecutor + the agent both depend on this constant.
        assert_eq!(AGENT_RECIPIENT_PREFIX, "agent:");
        let recipient = format!("{}{}", AGENT_RECIPIENT_PREFIX, "abc-123");
        assert_eq!(recipient, "agent:abc-123");
    }

    #[test]
    fn register_request_agent_id_defaults_to_none() {
        let json = r#"{"protocol_version":1,"max_concurrency":4,"target_triple":"x86_64-unknown-linux-gnu"}"#;
        let req: AgentRegisterRequest = serde_json::from_str(json).unwrap();
        assert!(req.agent_id.is_none());
        assert!(req.capabilities.is_empty());
    }
}
