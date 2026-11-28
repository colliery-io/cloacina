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

//! A poll of a trigger, sent to an agent (CLOACINA-T-1077).
//!
//! On a server, a poll trigger's code runs on the tenant's agent, not in the
//! server (CLOACINA-A-0019). The server's scheduler decides a poll is due and
//! sends a [`PollWorkPacket`] on the delivery outbox. The agent loads the
//! package, calls the trigger and answers on `/v1/agent/result`, keyed by
//! [`PollWorkPacket::poll_id`], with one of:
//!
//! - `Success { context: {"decision": "fire", "context": {…}} }`: fire, with
//!   the context the trigger gave (an object, or absent for none);
//! - `Success { context: {"decision": "skip"} }`: skip;
//! - `Failure`: the trigger returned an error;
//! - `Refused`: the agent did not run the trigger.
//!
//! An agent that can run polls says so at registration with the capability
//! [`POLL_JOB_CAPABILITY`]. The server sends polls only to such an agent, so
//! an agent of a release before this one is never sent one.
//!
//! The same packet carries a poll of a Python polling accumulator
//! (CLOACINA-T-1156): [`PollWorkPacket::accumulator_name`] names it. The agent
//! calls the accumulator's poll function and answers with one of:
//!
//! - `Success { context: {"decision": "emit", "value": …} }`: the JSON value
//!   the poll function returned;
//! - `Success { context: {"decision": "skip"} }`: no change (the function
//!   returned `None`);
//! - `Failure` and `Refused` as for a trigger.
//!
//! An agent that can run them registers [`ACCUMULATOR_POLL_CAPABILITY`]; an
//! agent with only [`POLL_JOB_CAPABILITY`] is never sent one.

use serde::{Deserialize, Serialize};

use super::protocol::{AgentOutcome, ArtifactRef, FailureClassification, RefusalReason};
use crate::Context;
use cloacina_workflow::{TriggerError, TriggerResult};

/// Substrate push `kind` of a [`PollWorkPacket`].
pub const POLL_PACKET_KIND: &str = "agent_poll";

/// The capability an agent registers when it can run poll packets.
pub const POLL_JOB_CAPABILITY: &str = "job:poll";

/// The capability an agent registers when it can run polls of polling
/// accumulators (CLOACINA-T-1156).
pub const ACCUMULATOR_POLL_CAPABILITY: &str = "job:accumulator_poll";

/// One poll of one trigger, or of one polling accumulator, sent to an agent
/// of its tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollWorkPacket {
    pub protocol_version: u32,
    /// Rendezvous key: a fresh UUID per poll. The agent reports it as the
    /// `task_execution_id` of its `AgentResultRequest`.
    pub poll_id: String,
    /// The trigger's name in the package.
    pub trigger_name: String,
    /// The workflow the trigger fires. For the agent's log only.
    #[serde(default)]
    pub workflow_name: String,
    /// The package that declares the trigger.
    pub package_name: String,
    /// The package's build.
    pub artifact: ArtifactRef,
    /// How long the agent lets one poll run.
    pub timeout_seconds: u32,
    /// Tenant scope (same meaning as `WorkPacket::tenant_id`).
    pub tenant_id: Option<String>,
    /// `"rust"` (or absent) or `"python"`, as in `WorkPacket::language`.
    #[serde(default)]
    pub language: Option<String>,
    /// The server waits for the answer until this time (Unix milliseconds).
    /// An agent that gets the packet later does not run the poll: nobody
    /// waits for its answer, and a poll can consume what it reads. A packet
    /// that the server moves to another agent after an eviction is the case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_after_unix_ms: Option<i64>,
    /// A poll of this polling accumulator of the package, not of a trigger
    /// (CLOACINA-T-1156). `trigger_name` is then empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accumulator_name: Option<String>,
}

impl PollWorkPacket {
    /// Whether the server still waits for the answer at `now_unix_ms`.
    pub fn still_wanted(&self, now_unix_ms: i64) -> bool {
        self.not_after_unix_ms.is_none_or(|end| now_unix_ms <= end)
    }

    /// The trigger or the accumulator the poll is of, for logs.
    pub fn target_name(&self) -> &str {
        self.accumulator_name
            .as_deref()
            .unwrap_or(&self.trigger_name)
    }
}

/// The outcome an agent reports for one poll of a polling accumulator
/// (CLOACINA-T-1156): `Ok(Some(value))` is emit, `Ok(None)` no change, `Err`
/// the poll function's error.
pub fn accumulator_poll_outcome(result: Result<Option<serde_json::Value>, String>) -> AgentOutcome {
    match result {
        Ok(Some(value)) => AgentOutcome::Success {
            context: serde_json::json!({ "decision": "emit", "value": value }),
        },
        Ok(None) => AgentOutcome::Success {
            context: serde_json::json!({ "decision": "skip" }),
        },
        Err(message) => AgentOutcome::Failure {
            message,
            classification: FailureClassification::TaskError,
        },
    }
}

/// Read an agent's outcome for a poll of a polling accumulator:
/// `Ok(Some(value))` to emit, `Ok(None)` for no change.
pub fn accumulator_poll_from_outcome(
    outcome: AgentOutcome,
) -> Result<Option<serde_json::Value>, PollFailure> {
    match outcome {
        AgentOutcome::Success { context } => {
            match context.get("decision").and_then(|d| d.as_str()) {
                Some("skip") => Ok(None),
                Some("emit") => match context.get("value") {
                    Some(value) => Ok(Some(value.clone())),
                    None => Err(PollFailure::BadAnswer(format!(
                        "an emit with no value: {context}"
                    ))),
                },
                _ => Err(PollFailure::BadAnswer(format!(
                    "no decision \"emit\" or \"skip\" in {context}"
                ))),
            }
        }
        AgentOutcome::Failure { message, .. } => Err(PollFailure::TriggerFailed(message)),
        AgentOutcome::Refused { reason, message } => Err(PollFailure::Refused { reason, message }),
    }
}

/// The outcome an agent reports for one poll.
pub fn poll_outcome(result: Result<TriggerResult, TriggerError>) -> AgentOutcome {
    match result {
        Ok(TriggerResult::Skip) => AgentOutcome::Success {
            context: serde_json::json!({ "decision": "skip" }),
        },
        Ok(TriggerResult::Fire(context)) => {
            let context = context.map(|c| {
                serde_json::Value::Object(
                    c.data()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
            });
            let mut answer = serde_json::json!({ "decision": "fire" });
            if let Some(context) = context {
                answer["context"] = context;
            }
            AgentOutcome::Success { context: answer }
        }
        Err(e) => AgentOutcome::Failure {
            message: e.to_string(),
            classification: FailureClassification::TaskError,
        },
    }
}

/// Why a poll gave no answer the scheduler can use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollFailure {
    /// The trigger ran on the agent and returned an error.
    TriggerFailed(String),
    /// The agent did not run the trigger.
    Refused {
        reason: RefusalReason,
        message: String,
    },
    /// The agent's answer is not a poll answer.
    BadAnswer(String),
}

impl std::fmt::Display for PollFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PollFailure::TriggerFailed(m) => write!(f, "the trigger failed on the agent: {m}"),
            PollFailure::Refused { reason, message } => {
                write!(f, "the agent did not run the poll ({reason:?}): {message}")
            }
            PollFailure::BadAnswer(m) => write!(f, "the agent's answer is not a poll answer: {m}"),
        }
    }
}

/// Read an agent's outcome for a poll as fire (with its context) or skip.
pub fn poll_result_from_outcome(outcome: AgentOutcome) -> Result<TriggerResult, PollFailure> {
    match outcome {
        AgentOutcome::Success { context } => {
            match context.get("decision").and_then(|d| d.as_str()) {
                Some("skip") => Ok(TriggerResult::Skip),
                Some("fire") => match context.get("context") {
                    None | Some(serde_json::Value::Null) => Ok(TriggerResult::Fire(None)),
                    Some(serde_json::Value::Object(map)) => {
                        let mut ctx = Context::new();
                        for (k, v) in map {
                            ctx.insert(k.clone(), v.clone())
                                .map_err(|e| PollFailure::BadAnswer(e.to_string()))?;
                        }
                        Ok(TriggerResult::Fire(Some(ctx)))
                    }
                    Some(other) => Err(PollFailure::BadAnswer(format!(
                        "the context of a fire is not an object: {other}"
                    ))),
                },
                _ => Err(PollFailure::BadAnswer(format!(
                    "no decision \"fire\" or \"skip\" in {context}"
                ))),
            }
        }
        AgentOutcome::Failure { message, .. } => Err(PollFailure::TriggerFailed(message)),
        AgentOutcome::Refused { reason, message } => Err(PollFailure::Refused { reason, message }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet::AGENT_PROTOCOL_VERSION;

    fn packet(not_after: Option<i64>) -> PollWorkPacket {
        PollWorkPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            poll_id: "p1".into(),
            trigger_name: "watch_inbox".into(),
            workflow_name: "wf".into(),
            package_name: "pkg".into(),
            artifact: ArtifactRef {
                digest: "abc".into(),
                fetch_url: "/v1/agent/artifact/abc".into(),
                build_target_triple: "aarch64-macos".into(),
            },
            timeout_seconds: 30,
            tenant_id: Some("acme".into()),
            language: Some("python".into()),
            not_after_unix_ms: not_after,
            accumulator_name: None,
        }
    }

    /// A poll of a polling accumulator names it, and an older packet with no
    /// such field reads as a trigger poll.
    #[test]
    fn an_accumulator_poll_names_its_accumulator_on_the_wire() {
        let mut p = packet(None);
        assert!(
            !serde_json::to_string(&p)
                .unwrap()
                .contains("accumulator_name")
        );
        assert_eq!(p.target_name(), "watch_inbox");
        p.trigger_name = String::new();
        p.accumulator_name = Some("prices".into());
        let back: PollWorkPacket =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.accumulator_name.as_deref(), Some("prices"));
        assert_eq!(back.target_name(), "prices");
    }

    /// Emit with its value, no change, and the poll function's error cross
    /// the wire; a trigger's answer is not an accumulator's.
    #[test]
    fn an_accumulator_poll_answer_crosses_the_wire() {
        let value = serde_json::json!({"price": 7});
        assert_eq!(
            accumulator_poll_from_outcome(accumulator_poll_outcome(Ok(Some(value.clone()))))
                .unwrap(),
            Some(value)
        );
        assert_eq!(
            accumulator_poll_from_outcome(accumulator_poll_outcome(Ok(None))).unwrap(),
            None
        );
        let failed =
            accumulator_poll_from_outcome(accumulator_poll_outcome(Err("feed down".into())))
                .unwrap_err();
        assert!(matches!(&failed, PollFailure::TriggerFailed(m) if m == "feed down"));
        let fire = poll_outcome(Ok(TriggerResult::Fire(None)));
        assert!(matches!(
            accumulator_poll_from_outcome(fire),
            Err(PollFailure::BadAnswer(_))
        ));
    }

    #[test]
    fn a_poll_packet_round_trips_as_json() {
        let json = serde_json::to_string(&packet(Some(1_000))).unwrap();
        let back: PollWorkPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(back.poll_id, "p1");
        assert_eq!(back.trigger_name, "watch_inbox");
        assert_eq!(back.not_after_unix_ms, Some(1_000));
        assert_eq!(back.language.as_deref(), Some("python"));
    }

    #[test]
    fn a_packet_past_its_end_is_not_wanted() {
        assert!(packet(Some(1_000)).still_wanted(1_000));
        assert!(!packet(Some(1_000)).still_wanted(1_001));
        assert!(packet(None).still_wanted(i64::MAX));
    }

    #[test]
    fn fire_with_its_context_and_skip_cross_the_wire() {
        let mut ctx = Context::new();
        ctx.insert("file", serde_json::json!("a.csv")).unwrap();
        let fired = poll_result_from_outcome(poll_outcome(Ok(TriggerResult::Fire(Some(ctx)))))
            .unwrap()
            .into_context()
            .expect("a context");
        assert_eq!(fired.get("file"), Some(&serde_json::json!("a.csv")));

        let bare = poll_result_from_outcome(poll_outcome(Ok(TriggerResult::Fire(None)))).unwrap();
        assert!(bare.should_fire());
        assert!(bare.into_context().is_none());

        let skip = poll_result_from_outcome(poll_outcome(Ok(TriggerResult::Skip))).unwrap();
        assert!(!skip.should_fire());
    }

    #[test]
    fn an_error_a_refusal_and_a_bad_answer_are_failures() {
        let failed = poll_result_from_outcome(poll_outcome(Err(TriggerError::PollError {
            message: "inbox down".into(),
        })))
        .unwrap_err();
        assert!(
            matches!(&failed, PollFailure::TriggerFailed(m) if m.contains("inbox down")),
            "{failed:?}"
        );

        let refused = poll_result_from_outcome(AgentOutcome::Refused {
            reason: RefusalReason::Shutdown,
            message: "agent at max_concurrency".into(),
        })
        .unwrap_err();
        assert!(refused.to_string().contains("did not run the poll"));

        let bad = poll_result_from_outcome(AgentOutcome::Success {
            context: serde_json::json!({ "outputs": [] }),
        })
        .unwrap_err();
        assert!(matches!(bad, PollFailure::BadAnswer(_)));
    }
}
