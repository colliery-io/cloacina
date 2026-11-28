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

//! Poll packets (CLOACINA-T-1077).
//!
//! On a server, a poll trigger's code runs on an agent of its tenant. The
//! server sends one [`PollWorkPacket`] per poll; the agent loads the package,
//! calls the trigger and answers fire (with the trigger's context) or skip,
//! on `/v1/agent/result` under the packet's poll id.
//!
//! CLOACINA-T-1143: a poll is a job of a worker process, beside a task and a
//! graph firing (`worker::PollJob`):
//! - in the agent process (`process_poll_packet` in `main.rs`): the checks of
//!   this module ([`still_wanted`], [`wrong_arch`]) and the fetch of the
//!   package;
//! - in the worker (`run_poll_job`): the load of the trigger and
//!   [`run_poll`]. The agent holds the worker to the poll's time limit and to
//!   the agent's default memory share, and kills its process group at either.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use cloacina::cloacina_workflow::Trigger;
use cloacina::fleet::{
    AgentOutcome, FailureClassification, PollWorkPacket, RefusalReason, WorkPacket, poll_outcome,
};

/// Read the packet of an `agent_poll` push.
pub(crate) fn decode_poll_packet(env: &serde_json::Value) -> Result<PollWorkPacket> {
    let b64 = env
        .get("payload_b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("push frame missing `payload_b64`"))?;
    let bytes = BASE64
        .decode(b64)
        .with_context(|| "decode base64 payload")?;
    serde_json::from_slice::<PollWorkPacket>(&bytes)
        .with_context(|| "deserialize PollWorkPacket JSON")
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Whether the server still waits for the answer of `packet` now. A packet
/// it no longer waits for is not run and gets no report: a poll can consume
/// what it reads, and its answer would be dropped.
pub(crate) fn still_wanted(packet: &PollWorkPacket) -> bool {
    packet.still_wanted(now_unix_ms())
}

/// The work packet the package loaders of the task path take, for the
/// package of a poll. The task name gives the loaders the tenant and the
/// package; no task of that name is looked up.
pub(crate) fn load_packet_for(packet: &PollWorkPacket) -> WorkPacket {
    let tenant = packet
        .tenant_id
        .clone()
        .unwrap_or_else(|| "public".to_string());
    WorkPacket {
        protocol_version: packet.protocol_version,
        task_execution_id: packet.poll_id.clone(),
        workflow_execution_id: String::new(),
        task_name: format!(
            "{tenant}::{}::{}::{}",
            packet.package_name,
            if packet.workflow_name.is_empty() {
                "poll"
            } else {
                packet.workflow_name.as_str()
            },
            packet.trigger_name
        ),
        attempt: 1,
        context: serde_json::json!({}),
        artifact: packet.artifact.clone(),
        timeout_seconds: packet.timeout_seconds,
        tenant_id: packet.tenant_id.clone(),
        language: packet.language.clone(),
        wrapped_secrets: Vec::new(),
        secret_key_id: None,
        dispatch_id: None,
        pool: None,
        cpu_millis: None,
        memory_bytes: None,
        inline: false,
        output_max_bytes: None,
    }
}

/// The refusal of a trigger the package does not declare.
pub(crate) fn no_such_trigger(packet: &PollWorkPacket) -> AgentOutcome {
    AgentOutcome::Failure {
        message: format!(
            "the package '{}' (build {}) has no poll trigger '{}'",
            packet.package_name, packet.artifact.digest, packet.trigger_name
        ),
        classification: FailureClassification::Validation,
    }
}

/// The refusal of a Rust build for another arch.
pub(crate) fn wrong_arch(packet: &PollWorkPacket, agent_triple: &str) -> Option<AgentOutcome> {
    let python = packet.language.as_deref() == Some("python");
    (!python && packet.artifact.build_target_triple != agent_triple).then(|| {
        AgentOutcome::Refused {
            reason: RefusalReason::TargetTripleMismatch,
            message: format!(
                "agent target triple `{}` does not match artifact build target `{}`",
                agent_triple, packet.artifact.build_target_triple
            ),
        }
    })
}

/// Call the trigger and give the outcome. With `time_limit` (a poll in the
/// agent process) the poll is dropped at the limit; in a worker the agent
/// holds the clock and kills the worker, with the same text.
pub(crate) async fn run_poll(
    trigger: Arc<dyn Trigger>,
    time_limit: Option<Duration>,
    timeout_seconds: u32,
) -> AgentOutcome {
    let Some(limit) = time_limit else {
        return poll_outcome(trigger.poll().await);
    };
    match tokio::time::timeout(limit, trigger.poll()).await {
        Ok(result) => poll_outcome(result),
        Err(_) => AgentOutcome::Failure {
            message: format!("poll exceeded timeout of {timeout_seconds}s"),
            classification: FailureClassification::Timeout,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina::cloacina_workflow::{TriggerError, TriggerResult};
    use cloacina::fleet::{AGENT_PROTOCOL_VERSION, ArtifactRef};

    fn packet(not_after: Option<i64>) -> PollWorkPacket {
        PollWorkPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            poll_id: "6f0e9a52-3c1d-4a8e-9b5e-0d6a7f1c2b3e".into(),
            trigger_name: "watch_inbox".into(),
            workflow_name: "wf".into(),
            package_name: "inbox".into(),
            artifact: ArtifactRef {
                digest: "abc".into(),
                fetch_url: "/v1/agent/artifact/abc".into(),
                build_target_triple: "x86_64-linux".into(),
            },
            timeout_seconds: 1,
            tenant_id: Some("acme".into()),
            language: Some("rust".into()),
            not_after_unix_ms: not_after,
        }
    }

    #[test]
    fn a_poll_packet_decodes_from_its_push() {
        let p = packet(None);
        let env = serde_json::json!({
            "type": "push",
            "id": 7,
            "kind": cloacina::fleet::POLL_PACKET_KIND,
            "payload_b64": BASE64.encode(serde_json::to_vec(&p).unwrap()),
        });
        let back = decode_poll_packet(&env).unwrap();
        assert_eq!(back.poll_id, p.poll_id);
        assert!(decode_poll_packet(&serde_json::json!({"type": "push"})).is_err());
    }

    #[test]
    fn a_packet_past_the_servers_wait_is_not_run() {
        assert!(!still_wanted(&packet(Some(now_unix_ms() - 1_000))));
        assert!(still_wanted(&packet(Some(now_unix_ms() + 60_000))));
        assert!(still_wanted(&packet(None)));
    }

    #[test]
    fn the_loaders_get_the_tenant_and_the_package_of_the_poll() {
        let load = load_packet_for(&packet(None));
        let ns = cloacina::parse_namespace(&load.task_name).unwrap();
        assert_eq!(ns.tenant_id, "acme");
        assert_eq!(ns.package_name, "inbox");
        assert_eq!(load.artifact.digest, "abc");
        let mut public = packet(None);
        public.tenant_id = None;
        let ns = cloacina::parse_namespace(&load_packet_for(&public).task_name).unwrap();
        assert_eq!(ns.tenant_id, "public");
    }

    #[test]
    fn a_rust_build_of_another_arch_is_refused_and_python_runs_anywhere() {
        let p = packet(None);
        assert!(matches!(
            wrong_arch(&p, "aarch64-macos"),
            Some(AgentOutcome::Refused {
                reason: RefusalReason::TargetTripleMismatch,
                ..
            })
        ));
        assert!(wrong_arch(&p, "x86_64-linux").is_none());
        let mut py = packet(None);
        py.language = Some("python".into());
        assert!(wrong_arch(&py, "aarch64-macos").is_none());
    }

    #[derive(Debug)]
    struct Slow;

    #[async_trait::async_trait]
    impl Trigger for Slow {
        fn name(&self) -> &str {
            "slow"
        }
        fn poll_interval(&self) -> Duration {
            Duration::from_secs(1)
        }
        fn allow_concurrent(&self) -> bool {
            false
        }
        async fn poll(&self) -> Result<TriggerResult, TriggerError> {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok(TriggerResult::Skip)
        }
    }

    #[tokio::test]
    async fn a_poll_past_its_time_limit_is_a_timeout() {
        let outcome = run_poll(Arc::new(Slow), Some(Duration::from_secs(1)), 1).await;
        assert!(matches!(
            &outcome,
            AgentOutcome::Failure {
                classification: FailureClassification::Timeout,
                message,
            } if message == "poll exceeded timeout of 1s"
        ));
    }
}
