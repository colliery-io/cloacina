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

//! Cloacina execution agent (CLOACI-I-0114 / task T-0632).
//!
//! A DB-less binary that connects to a `cloacina-server`, registers as part
//! of the fleet, listens for work packets pushed over the substrate WS,
//! enforces the OQ-6 target-triple fail-closed check, and reports each
//! outcome via REST.
//!
//! **Tier A** (this binary, today): the protocol skeleton end-to-end. Every
//! accepted work packet currently returns `AgentOutcome::Refused` — either
//! `TargetTripleMismatch` (real OQ-6 enforcement) or `RuntimeLoadFailed`
//! (Tier-B carry-forward marker). The substrate path, register/heartbeat
//! flow, and result reconciliation contract are all real.
//!
//! **Tier B** (T-0632 carry-forward, landing alongside T-0633's end-to-end
//! contract): fetch the cdylib via `GET /v1/agent/artifact/{digest}`, cache
//! by digest, `dlopen` via fidius `PluginHandle`, resolve the task in the
//! `cloacina` inventory, build a `Context` from `WorkPacket.context`,
//! execute, classify the outcome, return `Success`/`Failure`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context as _, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use clap::Parser;
use cloacina::fleet::{
    host_target_triple, AgentHeartbeatRequest, AgentRegisterRequest, AgentRegisterResponse,
    AgentResultRequest, AgentResultResponse, RefusalReason, WorkPacket, AGENT_PROTOCOL_VERSION,
    AGENT_RECIPIENT_PREFIX, WORK_PACKET_KIND,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

#[derive(Parser, Debug)]
#[command(
    name = "cloacina-agent",
    about = "DB-less execution agent for the Cloacina fleet (CLOACI-I-0114).",
    version
)]
struct Args {
    /// Server base URL (e.g. `http://localhost:8080` or `https://api.example.com`).
    #[arg(long, env = "CLOACINA_SERVER")]
    server: String,
    /// API key used for both REST and the WS ticket mint.
    #[arg(long, env = "CLOACINA_API_KEY")]
    api_key: String,
    /// Optional caller-chosen agent id; server assigns one if omitted.
    #[arg(long)]
    agent_id: Option<String>,
    /// Max concurrent work packets this agent will accept.
    #[arg(long, default_value_t = 4)]
    max_concurrency: u32,
    /// Free-form capability tags advertised at registration.
    #[arg(long, value_delimiter = ',')]
    capabilities: Vec<String>,
    /// Override the host target triple the agent advertises (rarely needed —
    /// useful for testing the OQ-6 fail-closed path on a homogeneous host).
    #[arg(long)]
    target_triple_override: Option<String>,
    /// Directory used to cache fetched artifacts by digest. Defaults to
    /// `<TMPDIR>/cloacina-agent-cache`. Reuses cached cdylibs across work
    /// packets to skip the REST fetch on cache hit.
    #[arg(long, env = "CLOACINA_AGENT_CACHE_DIR")]
    cache_dir: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    if let Err(e) = run(args).await {
        error!(error = ?e, "agent exited with error");
        std::process::exit(1);
    }
    Ok(())
}

async fn run(args: Args) -> Result<()> {
    let server_base = args.server.trim_end_matches('/').to_string();
    let http = reqwest::Client::builder()
        .user_agent(concat!("cloacina-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("build HTTP client")?;

    let target_triple = args
        .target_triple_override
        .clone()
        .unwrap_or_else(host_target_triple);

    let cache_dir = args
        .cache_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("cloacina-agent-cache"));
    std::fs::create_dir_all(&cache_dir)
        .with_context(|| format!("create cache dir {:?}", cache_dir))?;
    info!(cache_dir = ?cache_dir, "artifact cache directory");
    let cache_dir = Arc::new(cache_dir);

    // ── 1. Register ────────────────────────────────────────────────
    let register_resp = register(
        &http,
        &server_base,
        &args.api_key,
        &args.agent_id,
        args.max_concurrency,
        &target_triple,
        &args.capabilities,
    )
    .await
    .context("agent register failed")?;
    let agent_id = register_resp.agent_id.clone();
    info!(
        agent_id = %agent_id,
        target_triple = %target_triple,
        max_concurrency = args.max_concurrency,
        heartbeat_interval_seconds = register_resp.heartbeat_interval_seconds,
        "agent registered with server"
    );

    // ── 2. Spawn heartbeat task ────────────────────────────────────
    let in_flight = Arc::new(AtomicU32::new(0));
    spawn_heartbeat_loop(
        http.clone(),
        server_base.clone(),
        args.api_key.clone(),
        agent_id.clone(),
        args.max_concurrency,
        in_flight.clone(),
        Duration::from_secs(register_resp.heartbeat_interval_seconds.max(1) as u64),
    );

    // ── 3. Mint a single-use WS ticket + connect the substrate WS ──
    let ticket = mint_ws_ticket(&http, &server_base, &args.api_key)
        .await
        .context("mint WS ticket")?;
    let ws_url = ws_url_for(&server_base, &agent_id, &ticket)?;
    info!(ws_url = %ws_url, "connecting to substrate delivery WS");
    let (stream, _resp) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .with_context(|| format!("WS connect to {}", ws_url))?;

    // ── 4. Receive + process loop ──────────────────────────────────
    receive_loop(
        stream,
        http,
        server_base,
        args.api_key,
        agent_id,
        target_triple,
        args.max_concurrency,
        in_flight,
        cache_dir,
    )
    .await
}

// ────────────────────────────────────────────────────────────────────
// HTTP helpers
// ────────────────────────────────────────────────────────────────────

async fn register(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    agent_id: &Option<String>,
    max_concurrency: u32,
    target_triple: &str,
    capabilities: &[String],
) -> Result<AgentRegisterResponse> {
    let body = AgentRegisterRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.clone(),
        max_concurrency,
        target_triple: target_triple.to_string(),
        capabilities: capabilities.to_vec(),
    };
    let resp = http
        .post(format!("{}/v1/agent/register", server))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json().await?)
}

async fn mint_ws_ticket(http: &reqwest::Client, server: &str, api_key: &str) -> Result<String> {
    let resp: serde_json::Value = http
        .post(format!("{}/v1/auth/ws-ticket", server))
        .bearer_auth(api_key)
        .json(&serde_json::json!({}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    resp.get("ticket")
        .and_then(|t| t.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow!("ws-ticket response missing `ticket`: {}", resp))
}

async fn post_result(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &AgentResultRequest,
) -> Result<AgentResultResponse> {
    let resp = http
        .post(format!("{}/v1/agent/result", server))
        .bearer_auth(api_key)
        .json(req)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.json().await?)
}

fn spawn_heartbeat_loop(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    max_concurrency: u32,
    in_flight: Arc<AtomicU32>,
    interval: Duration,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let n = in_flight.load(Ordering::SeqCst);
            let body = AgentHeartbeatRequest {
                protocol_version: AGENT_PROTOCOL_VERSION,
                agent_id: agent_id.clone(),
                in_flight: n,
                available_capacity: max_concurrency.saturating_sub(n),
            };
            match http
                .post(format!("{}/v1/agent/heartbeat", server))
                .bearer_auth(&api_key)
                .json(&body)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    debug!(in_flight = n, "heartbeat ok");
                }
                Ok(resp) => warn!(status = %resp.status(), "heartbeat non-success"),
                Err(e) => warn!(error = %e, "heartbeat request failed"),
            }
        }
    });
}

// ────────────────────────────────────────────────────────────────────
// WS loop + per-packet processing
// ────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn receive_loop<S>(
    mut stream: tokio_tungstenite::WebSocketStream<S>,
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    target_triple: String,
    max_concurrency: u32,
    in_flight: Arc<AtomicU32>,
    cache_dir: Arc<std::path::PathBuf>,
) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    // mpsc so per-packet tasks send acks back to the single WS sender without
    // contending for a mutex on the stream sink.
    let (ack_tx, mut ack_rx) = mpsc::unbounded_channel::<Message>();

    loop {
        tokio::select! {
            // Outgoing acks from per-packet tasks.
            ack = ack_rx.recv() => match ack {
                Some(msg) => {
                    if let Err(e) = stream.send(msg).await {
                        warn!(error = %e, "WS send failed");
                        break;
                    }
                }
                None => break, // all senders dropped (shouldn't happen while we hold one)
            },

            // Incoming frames from the server.
            msg = stream.next() => match msg {
                Some(Ok(Message::Text(text))) => {
                    if let Err(e) = handle_text_frame(
                        text.to_string(),
                        &http,
                        &server,
                        &api_key,
                        &agent_id,
                        &target_triple,
                        &in_flight,
                        max_concurrency,
                        &ack_tx,
                        &cache_dir,
                    ).await {
                        warn!(error = ?e, "frame handling error");
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    info!("substrate WS closed by server");
                    break;
                }
                Some(Ok(_)) => {}, // ping/pong/binary — substrate uses text frames
                Some(Err(e)) => {
                    warn!(error = %e, "WS recv error");
                    break;
                }
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_text_frame(
    text: String,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    agent_id: &str,
    target_triple: &str,
    in_flight: &Arc<AtomicU32>,
    max_concurrency: u32,
    ack_tx: &mpsc::UnboundedSender<Message>,
    cache_dir: &Arc<std::path::PathBuf>,
) -> Result<()> {
    let env: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("envelope JSON parse: {}", truncate(&text, 256)))?;
    let envelope_type = env.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match envelope_type {
        "welcome" => {
            debug!("substrate welcome frame");
        }
        "push" => {
            let push_id = env
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow!("push frame missing numeric `id`"))?;
            let kind = env.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            if kind != WORK_PACKET_KIND {
                warn!(kind = %kind, push_id, "non-work push received; acking + ignoring");
                let _ = ack_tx.send(Message::Text(make_ack(push_id)?.into()));
                return Ok(());
            }

            // Capacity check: if we're saturated, refuse with Shutdown-ish.
            // (v1: a real fleet would refuse via the substrate so the relay
            // re-pushes to a less-loaded peer; here we accept + refuse so
            // the contract is observable.)
            let cur = in_flight.load(Ordering::SeqCst);
            if cur >= max_concurrency {
                warn!(
                    in_flight = cur,
                    max_concurrency, "saturated; refusing packet"
                );
                let packet = decode_packet(&env);
                spawn_refusal(
                    http.clone(),
                    server.to_string(),
                    api_key.to_string(),
                    agent_id.to_string(),
                    packet,
                    RefusalReason::Shutdown,
                    "agent at max_concurrency".to_string(),
                    ack_tx.clone(),
                    push_id,
                );
                return Ok(());
            }

            let packet = decode_packet(&env).context("decode work packet")?;
            spawn_packet_worker(
                http.clone(),
                server.to_string(),
                api_key.to_string(),
                agent_id.to_string(),
                target_triple.to_string(),
                packet,
                in_flight.clone(),
                ack_tx.clone(),
                push_id,
                cache_dir.clone(),
            );
        }
        other => {
            debug!(other, "ignoring unknown frame type");
        }
    }
    Ok(())
}

fn decode_packet(env: &serde_json::Value) -> Result<WorkPacket> {
    let b64 = env
        .get("payload_b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("push frame missing `payload_b64`"))?;
    let bytes = BASE64
        .decode(b64)
        .with_context(|| "decode base64 payload")?;
    serde_json::from_slice::<WorkPacket>(&bytes).with_context(|| "deserialize WorkPacket JSON")
}

fn make_ack(id: i64) -> Result<String> {
    let v = serde_json::json!({
        "type": "ack",
        "protocol_version": cloacina::delivery::DELIVERY_PROTOCOL_VERSION,
        "id": id,
    });
    Ok(serde_json::to_string(&v)?)
}

/// Spawn a worker that runs the full Tier-B path (triple check → fetch+cache
/// artifact → dlopen via fidius → resolve task → execute with context →
/// classify), POSTs the result, and acks.
#[allow(clippy::too_many_arguments)]
fn spawn_packet_worker(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    target_triple: String,
    packet: WorkPacket,
    in_flight: Arc<AtomicU32>,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
    cache_dir: Arc<std::path::PathBuf>,
) {
    in_flight.fetch_add(1, Ordering::SeqCst);
    tokio::spawn(async move {
        let start = Instant::now();
        let outcome = process_work_packet(
            &packet,
            &target_triple,
            &http,
            &server,
            &api_key,
            cache_dir.as_path(),
        )
        .await;
        let duration_ms = start.elapsed().as_millis() as u64;
        let req = AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id: agent_id.clone(),
            task_execution_id: packet.task_execution_id.clone(),
            attempt: packet.attempt,
            duration_ms,
            outcome,
        };
        report_outcome(&http, &server, &api_key, &req).await;
        if let Ok(ack) = make_ack(push_id) {
            let _ = ack_tx.send(Message::Text(ack.into()));
        }
        in_flight.fetch_sub(1, Ordering::SeqCst);
    });
}

/// Spawn a worker that POSTs a refusal + acks. Same shape as `spawn_packet_worker`
/// but doesn't bump in-flight (we're refusing pre-execution).
#[allow(clippy::too_many_arguments)]
fn spawn_refusal(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    packet: Result<WorkPacket>,
    reason: RefusalReason,
    message: String,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
) {
    tokio::spawn(async move {
        if let Ok(packet) = packet {
            let req = AgentResultRequest {
                protocol_version: AGENT_PROTOCOL_VERSION,
                agent_id,
                task_execution_id: packet.task_execution_id.clone(),
                attempt: packet.attempt,
                duration_ms: 0,
                outcome: cloacina::fleet::AgentOutcome::Refused { reason, message },
            };
            report_outcome(&http, &server, &api_key, &req).await;
        }
        if let Ok(ack) = make_ack(push_id) {
            let _ = ack_tx.send(Message::Text(ack.into()));
        }
    });
}

async fn report_outcome(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &AgentResultRequest,
) {
    match post_result(http, server, api_key, req).await {
        Ok(_) => debug!(
            task_id = %req.task_execution_id,
            outcome = %summarize_outcome(&req.outcome),
            "result reported"
        ),
        Err(e) => warn!(
            task_id = %req.task_execution_id,
            error = %e,
            "result POST failed (will be retried by FleetExecutor reconciliation when T-0633 lands)"
        ),
    }
}

fn summarize_outcome(o: &cloacina::fleet::AgentOutcome) -> String {
    match o {
        cloacina::fleet::AgentOutcome::Success { .. } => "success".into(),
        cloacina::fleet::AgentOutcome::Failure { classification, .. } => {
            format!("failure({:?})", classification)
        }
        cloacina::fleet::AgentOutcome::Refused { reason, .. } => format!("refused({:?})", reason),
    }
}

// ────────────────────────────────────────────────────────────────────
// Work-packet processing — OQ-6 + Tier-A stub.
// ────────────────────────────────────────────────────────────────────

/// The agent's full execution path for one packet (Tier B, T-0632).
///
/// 1. **OQ-6 fail-closed** target-triple check — refuse if the artifact was
///    built for a different target before any `dlopen` attempt.
/// 2. **Fetch + cache** the cdylib by digest (`GET /v1/agent/artifact/{digest}`,
///    cached on disk under `cache_dir/<digest>.<ext>` to skip the REST hop
///    on warm hits). Failure → `Refused { ArtifactFetchFailed }`.
/// 3. **Register the cdylib's tasks** into a per-packet `cloacina::Runtime`
///    via `TaskRegistrar::register_package_tasks` (fidius `PluginHandle`
///    under the hood). Failure → `Refused { RuntimeLoadFailed }`.
/// 4. **Resolve the task** from the registered namespaces; missing →
///    `Failure { Validation }`.
/// 5. **Build a `Context<serde_json::Value>`** from the inlined `WorkPacket.context`.
/// 6. **Execute the task** under the packet's timeout. Map the outcome onto
///    `AgentOutcome::Success { context }` / `Failure {..}` / timeout.
async fn process_work_packet(
    packet: &WorkPacket,
    agent_triple: &str,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
) -> cloacina::fleet::AgentOutcome {
    // 1. OQ-6 fail-closed.
    if packet.artifact.build_target_triple != agent_triple {
        return cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::TargetTripleMismatch,
            message: format!(
                "agent target triple `{}` does not match artifact build target `{}`",
                agent_triple, packet.artifact.build_target_triple
            ),
        };
    }

    // 2. Fetch + cache (no-op if cached).
    let artifact_path =
        match fetch_and_cache_artifact(http, server, api_key, &packet.artifact, cache_dir).await {
            Ok(p) => p,
            Err(e) => {
                return cloacina::fleet::AgentOutcome::Refused {
                    reason: RefusalReason::ArtifactFetchFailed,
                    message: format!("artifact fetch failed: {}", e),
                };
            }
        };
    let cdylib_bytes = match std::fs::read(&artifact_path) {
        Ok(b) => b,
        Err(e) => {
            return cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::ArtifactFetchFailed,
                message: format!("read cached artifact {:?}: {}", artifact_path, e),
            };
        }
    };

    // 3. Register the cdylib's tasks into a per-packet Runtime.
    let runtime = Arc::new(cloacina::Runtime::new());
    let registrar = match cloacina::registry::loader::TaskRegistrar::new() {
        Ok(r) => r,
        Err(e) => {
            return cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: format!("task registrar init: {}", e),
            };
        }
    };
    let package_id = format!("agent_pkg_{}", packet.artifact.digest);
    let metadata = synthetic_package_metadata(&packet.artifact.digest);
    let registered_namespaces = match registrar
        .register_package_tasks(
            &package_id,
            &cdylib_bytes,
            &metadata,
            packet.tenant_id.as_deref(),
            &runtime,
        )
        .await
    {
        Ok(ns) => ns,
        Err(e) => {
            return cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: format!("register_package_tasks: {}", e),
            };
        }
    };
    debug!(
        package_id = %package_id,
        registered = ?registered_namespaces.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
        "cdylib registered"
    );

    // 4. Resolve the task by namespace string.
    let namespace = match cloacina::parse_namespace(&packet.task_name) {
        Ok(ns) => ns,
        Err(e) => {
            return cloacina::fleet::AgentOutcome::Failure {
                message: format!("parse_namespace({}): {}", packet.task_name, e),
                classification: cloacina::fleet::FailureClassification::Validation,
            };
        }
    };
    let Some(task) = runtime.get_task(&namespace) else {
        return cloacina::fleet::AgentOutcome::Failure {
            message: format!(
                "task `{}` not registered after loading cdylib (registered: {:?})",
                packet.task_name,
                registered_namespaces
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
            ),
            classification: cloacina::fleet::FailureClassification::Validation,
        };
    };

    // 5. Build context from the work packet.
    let context = match build_context(&packet.context) {
        Ok(c) => c,
        Err(e) => {
            return cloacina::fleet::AgentOutcome::Failure {
                message: format!("build context: {}", e),
                classification: cloacina::fleet::FailureClassification::Validation,
            };
        }
    };

    // 6. Execute under the packet's timeout.
    let timeout = std::time::Duration::from_secs(packet.timeout_seconds.max(1) as u64);
    match tokio::time::timeout(timeout, task.execute(context)).await {
        Ok(Ok(output)) => {
            let output_value = context_to_value(&output);
            cloacina::fleet::AgentOutcome::Success {
                context: output_value,
            }
        }
        Ok(Err(e)) => cloacina::fleet::AgentOutcome::Failure {
            message: e.to_string(),
            classification: cloacina::fleet::FailureClassification::TaskError,
        },
        Err(_) => cloacina::fleet::AgentOutcome::Failure {
            message: format!("task exceeded timeout of {}s", packet.timeout_seconds),
            classification: cloacina::fleet::FailureClassification::Timeout,
        },
    }
}

/// GET the artifact from `/v1/agent/artifact/{digest}` and cache it under
/// `cache_dir/<digest>.<ext>` keyed by the content-addressed digest. Returns
/// the path. Cache hit short-circuits the REST call.
async fn fetch_and_cache_artifact(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    artifact: &cloacina::fleet::ArtifactRef,
    cache_dir: &std::path::Path,
) -> Result<std::path::PathBuf> {
    let ext = library_extension();
    let cached = cache_dir.join(format!("{}.{}", artifact.digest, ext));
    if cached.exists() {
        debug!(path = ?cached, digest = %artifact.digest, "artifact cache hit");
        return Ok(cached);
    }

    let url = if artifact.fetch_url.starts_with("http") {
        artifact.fetch_url.clone()
    } else {
        format!(
            "{}{}",
            server.trim_end_matches('/'),
            if artifact.fetch_url.starts_with('/') {
                artifact.fetch_url.clone()
            } else {
                format!("/{}", artifact.fetch_url)
            }
        )
    };
    debug!(url = %url, digest = %artifact.digest, "fetching artifact");

    let bytes = http
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .with_context(|| format!("GET {}", url))?
        .error_for_status()
        .with_context(|| format!("HTTP error from {}", url))?
        .bytes()
        .await
        .with_context(|| format!("read body from {}", url))?;

    std::fs::create_dir_all(cache_dir)
        .with_context(|| format!("create cache dir {:?}", cache_dir))?;
    // Atomic write via tmp + rename so a concurrent reader never sees a
    // half-written file under the digest path.
    let tmp = cache_dir.join(format!("{}.{}.tmp", artifact.digest, ext));
    std::fs::write(&tmp, &bytes).with_context(|| format!("write tmp {:?}", tmp))?;
    std::fs::rename(&tmp, &cached).with_context(|| format!("rename {:?} -> {:?}", tmp, cached))?;
    debug!(path = ?cached, bytes = bytes.len(), "artifact cached");
    Ok(cached)
}

fn library_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

/// Build a minimal `PackageMetadata` for `TaskRegistrar::register_package_tasks`.
/// Note: the registrar wants the loader-side `package_loader::PackageMetadata`
/// (distinct from the higher-level `registry::types::PackageMetadata`), and
/// marks the parameter `_metadata` in its body — only a cleanly-constructed
/// value is needed.
fn synthetic_package_metadata(
    digest: &str,
) -> cloacina::registry::loader::package_loader::PackageMetadata {
    use cloacina::registry::loader::package_loader::PackageMetadata;
    PackageMetadata {
        package_name: format!("agent_dynamic_{}", digest),
        workflow_name: format!("agent_dynamic_{}", digest),
        version: "0.0.0".into(),
        description: None,
        author: None,
        tasks: vec![],
        graph_data: None,
        architecture: host_target_triple(),
        symbols: vec![],
        workflow_triggers: vec![],
    }
}

/// Convert the work packet's `serde_json::Value` (expected to be an Object or
/// Null) into a `Context<serde_json::Value>`.
fn build_context(
    value: &serde_json::Value,
) -> Result<cloacina::Context<serde_json::Value>, anyhow::Error> {
    let mut ctx = cloacina::Context::<serde_json::Value>::new();
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                ctx.insert(k.as_str(), v.clone())
                    .map_err(|e| anyhow!("insert {}: {}", k, e))?;
            }
            Ok(ctx)
        }
        serde_json::Value::Null => Ok(ctx),
        other => bail!(
            "WorkPacket.context must be a JSON object or null, got {}",
            kind_of(other)
        ),
    }
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

/// Materialize a `Context<serde_json::Value>` back into a JSON object so the
/// server's `FleetExecutor` reconciliation can pass it through to the shared
/// `TaskResultHandler::handle_outcome` Success branch.
fn context_to_value(ctx: &cloacina::Context<serde_json::Value>) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = ctx
        .data()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    serde_json::Value::Object(map)
}

// ────────────────────────────────────────────────────────────────────
// URL construction.
// ────────────────────────────────────────────────────────────────────

fn ws_url_for(server: &str, agent_id: &str, ticket: &str) -> Result<String> {
    let trimmed = server.trim_end_matches('/');
    let ws_base = if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("wss://{}", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("ws://{}", rest)
    } else {
        bail!("server must start with http:// or https://: {}", server);
    };
    let recipient = format!("{}{}", AGENT_RECIPIENT_PREFIX, agent_id);
    // `:` is unreserved in RFC 3986 path segments but encode for resilience.
    let recipient_enc = recipient.replace(':', "%3A");
    Ok(format!(
        "{}/v1/ws/delivery/{}?token={}",
        ws_base, recipient_enc, ticket
    ))
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

// Silence the unused `Serialize` import if `serde_json` no-ops above ever change.
const _: fn() = || {
    fn _assert_serialize<T: Serialize>() {}
    _assert_serialize::<AgentResultRequest>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina::fleet::{AgentOutcome, ArtifactRef};

    fn pkt(triple: &str) -> WorkPacket {
        WorkPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            task_execution_id: "t1".into(),
            workflow_execution_id: "w1".into(),
            task_name: "ns::task".into(),
            attempt: 1,
            context: serde_json::json!({}),
            artifact: ArtifactRef {
                digest: "deadbeef".into(),
                fetch_url: "/v1/agent/artifact/deadbeef".into(),
                build_target_triple: triple.into(),
            },
            timeout_seconds: 60,
            tenant_id: None,
        }
    }

    // The target_triple mismatch path is tested at the helper level — the
    // OQ-6 short-circuit is the very first thing `process_work_packet` does,
    // so we exercise the same branch by calling it with a never-reachable
    // server URL (the mismatch must short-circuit before any fetch attempt).
    #[tokio::test]
    async fn target_triple_mismatch_refuses_pre_fetch() {
        let agent_triple = "aarch64-apple-darwin";
        let packet = pkt("x86_64-unknown-linux-gnu");
        let http = reqwest::Client::new();
        let tmp = tempdir();
        // Server URL is deliberately bogus — if the triple check didn't
        // short-circuit, the fetch would error out and we'd see
        // `ArtifactFetchFailed`, not `TargetTripleMismatch`.
        let outcome = process_work_packet(
            &packet,
            agent_triple,
            &http,
            "http://nowhere.invalid:1",
            "ignored-key",
            tmp.as_path(),
        )
        .await;
        assert!(matches!(
            outcome,
            AgentOutcome::Refused {
                reason: RefusalReason::TargetTripleMismatch,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn artifact_fetch_failure_refuses() {
        // Triple matches (so we get past OQ-6) but server URL is unreachable.
        let agent_triple = host_target_triple();
        let packet = pkt(&agent_triple);
        let http = reqwest::Client::new();
        let tmp = tempdir();
        let outcome = process_work_packet(
            &packet,
            &agent_triple,
            &http,
            "http://127.0.0.1:1",
            "ignored",
            tmp.as_path(),
        )
        .await;
        assert!(matches!(
            outcome,
            AgentOutcome::Refused {
                reason: RefusalReason::ArtifactFetchFailed,
                ..
            }
        ));
    }

    fn tempdir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("cloacina-agent-test-{}", rand_suffix()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn rand_suffix() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string()
    }

    #[test]
    fn build_context_accepts_object_or_null() {
        let v = serde_json::json!({"k1": 1, "k2": "x"});
        let ctx = build_context(&v).unwrap();
        assert_eq!(ctx.get("k1"), Some(&serde_json::json!(1)));
        let null_ctx = build_context(&serde_json::Value::Null).unwrap();
        assert!(null_ctx.data().is_empty());
    }

    #[test]
    fn build_context_rejects_non_object_non_null() {
        let v = serde_json::json!([1, 2, 3]);
        assert!(build_context(&v).is_err());
    }

    #[test]
    fn context_to_value_round_trips_through_build_context() {
        let v = serde_json::json!({"a": 1, "b": "two"});
        let ctx = build_context(&v).unwrap();
        let back = context_to_value(&ctx);
        assert_eq!(back, v);
    }

    #[test]
    fn ws_url_for_handles_https_and_http() {
        let url = ws_url_for("https://api.example.com:8443", "abc-123", "tk").unwrap();
        assert_eq!(
            url,
            "wss://api.example.com:8443/v1/ws/delivery/agent%3Aabc-123?token=tk"
        );
        let url = ws_url_for("http://localhost:8080/", "abc", "tk").unwrap();
        assert_eq!(
            url,
            "ws://localhost:8080/v1/ws/delivery/agent%3Aabc?token=tk"
        );
    }

    #[test]
    fn ws_url_for_rejects_unsupported_scheme() {
        assert!(ws_url_for("ftp://x", "y", "z").is_err());
    }
}
