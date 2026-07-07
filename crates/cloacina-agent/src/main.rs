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
use cloacina::crypto::envelope::{generate_ephemeral_keypair, EphemeralKeypair};
use cloacina::fleet::{
    host_target_triple, AgentHeartbeatRequest, AgentRegisterRequest, AgentRegisterResponse,
    AgentResultRequest, AgentResultResponse, GraphWorkPacket, RefusalReason, WorkPacket,
    AGENT_PROTOCOL_VERSION, AGENT_RECIPIENT_PREFIX, GRAPH_PACKET_KIND, WORK_PACKET_KIND,
};
use cloacina::security::InMemorySecretResolver;
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

/// Backoff bounds for the reconnect loop.
const RECONNECT_BACKOFF_MIN: Duration = Duration::from_secs(1);
const RECONNECT_BACKOFF_MAX: Duration = Duration::from_secs(30);

async fn run(args: Args) -> Result<()> {
    let server_base = args.server.trim_end_matches('/').to_string();
    let http = reqwest::Client::builder()
        .user_agent(concat!("cloacina-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("build HTTP client")?;

    // Install the PyO3 Python runtime so the agent can load Python-packaged
    // workflows (not just Rust cdylibs). Idempotent. (CLOACI-T-0716)
    cloacina_python::install();

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

    // Process-wide in-flight counter — survives across reconnect sessions so a
    // task still running when the WS drops is still counted on reconnect.
    let in_flight = Arc::new(AtomicU32::new(0));

    // ── Reconnect loop ─────────────────────────────────────────────
    // A WS drop (or a server restart, which also forgets our registration) must
    // NOT take the agent down — it re-registers and reconnects with capped
    // exponential backoff. A session that actually connected resets the backoff;
    // repeated failures (server still down) back off up to RECONNECT_BACKOFF_MAX.
    let mut backoff = RECONNECT_BACKOFF_MIN;
    loop {
        match connect_and_serve(
            &args,
            &http,
            &server_base,
            &target_triple,
            in_flight.clone(),
            cache_dir.clone(),
        )
        .await
        {
            Ok(()) => {
                // We had a live session that then ended — reconnect promptly.
                info!("substrate WS session ended — reconnecting");
                backoff = RECONNECT_BACKOFF_MIN;
                tokio::time::sleep(RECONNECT_BACKOFF_MIN).await;
            }
            Err(e) => {
                warn!(
                    error = %e,
                    backoff_seconds = backoff.as_secs(),
                    "agent session failed to establish — backing off before retry"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(RECONNECT_BACKOFF_MAX);
            }
        }
    }
}

/// One register→connect→serve session. Returns `Ok(())` once a live WS session
/// has ended (caller reconnects immediately) and `Err` if the session could not
/// be established (caller backs off). The per-session heartbeat task is aborted
/// on every exit path so it never outlives its registration.
async fn connect_and_serve(
    args: &Args,
    http: &reqwest::Client,
    server_base: &str,
    target_triple: &str,
    in_flight: Arc<AtomicU32>,
    cache_dir: Arc<std::path::PathBuf>,
) -> Result<()> {
    // ── 0. CLOACI-T-0861 — mint a fresh ephemeral X25519 keypair for THIS
    //       session. The public key is advertised at register; the private key is
    //       held (in an Arc, shared across per-packet workers) to unwrap the
    //       server's HPKE-wrapped secrets. Regenerated on every session/reconnect,
    //       so a leaked key exposes at most one connection's secret resolutions.
    //       (Granularity caveat vs. D-5 per-execution: see `security::fleet_secret`.)
    let keypair = Arc::new(generate_ephemeral_keypair());
    let ephemeral_public_key = Some(BASE64.encode(&keypair.public_key_bytes));

    // ── 1. Register (fresh each session — the server may have restarted and
    //       forgotten us, in which case a stale agent_id would be unroutable).
    let register_resp = register(
        http,
        server_base,
        &args.api_key,
        &args.agent_id,
        args.max_concurrency,
        target_triple,
        &args.capabilities,
        ephemeral_public_key,
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

    // ── 2. Spawn the heartbeat task for THIS session.
    let heartbeat = spawn_heartbeat_loop(
        http.clone(),
        server_base.to_string(),
        args.api_key.clone(),
        agent_id.clone(),
        args.max_concurrency,
        in_flight.clone(),
        Duration::from_secs(register_resp.heartbeat_interval_seconds.max(1) as u64),
    );

    // ── 3+4. Mint a ticket, connect, and serve. Bundled so the heartbeat is
    //         aborted on any early exit (mint/connect failure or loop end).
    let session = async {
        let ticket = mint_ws_ticket(http, server_base, &args.api_key)
            .await
            .context("mint WS ticket")?;
        let ws_url = ws_url_for(server_base, &agent_id, &ticket)?;
        info!(ws_url = %ws_url, "connecting to substrate delivery WS");
        let (stream, _resp) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .with_context(|| format!("WS connect to {}", ws_url))?;
        info!(agent_id = %agent_id, "substrate WS connected");
        receive_loop(
            stream,
            http.clone(),
            server_base.to_string(),
            args.api_key.clone(),
            agent_id.clone(),
            target_triple.to_string(),
            args.max_concurrency,
            in_flight.clone(),
            cache_dir.clone(),
            keypair.clone(),
        )
        .await
    }
    .await;

    heartbeat.abort();
    session
}

// ────────────────────────────────────────────────────────────────────
// HTTP helpers
// ────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn register(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    agent_id: &Option<String>,
    max_concurrency: u32,
    target_triple: &str,
    capabilities: &[String],
    ephemeral_public_key: Option<String>,
) -> Result<AgentRegisterResponse> {
    let body = AgentRegisterRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.clone(),
        max_concurrency,
        target_triple: target_triple.to_string(),
        capabilities: capabilities.to_vec(),
        // CLOACI-T-0861 — advertise the ephemeral X25519 public key so the server
        // can HPKE-wrap secrets to this agent. The private half stays in memory
        // for the session (`keypair` in `run_session`) and never leaves.
        ephemeral_public_key,
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
) -> tokio::task::JoinHandle<()> {
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
    })
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
    keypair: Arc<EphemeralKeypair>,
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
                        &keypair,
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
    keypair: &Arc<EphemeralKeypair>,
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
            // CLOACI-T-0722: whole-graph firings ride their own packet kind.
            if kind == GRAPH_PACKET_KIND {
                let cur = in_flight.load(Ordering::SeqCst);
                let packet = decode_graph_packet(&env).context("decode graph packet")?;
                if cur >= max_concurrency {
                    warn!(
                        in_flight = cur,
                        max_concurrency, "saturated; refusing graph firing"
                    );
                    spawn_graph_refusal(
                        http.clone(),
                        server.to_string(),
                        api_key.to_string(),
                        agent_id.to_string(),
                        packet,
                        "agent at max_concurrency".to_string(),
                        ack_tx.clone(),
                        push_id,
                    );
                    return Ok(());
                }
                spawn_graph_worker(
                    http.clone(),
                    server.to_string(),
                    api_key.to_string(),
                    agent_id.to_string(),
                    packet,
                    in_flight.clone(),
                    ack_tx.clone(),
                    push_id,
                    cache_dir.clone(),
                );
                return Ok(());
            }
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
                keypair.clone(),
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

fn decode_graph_packet(env: &serde_json::Value) -> Result<GraphWorkPacket> {
    let b64 = env
        .get("payload_b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("push frame missing `payload_b64`"))?;
    let bytes = BASE64
        .decode(b64)
        .with_context(|| "decode base64 payload")?;
    serde_json::from_slice::<GraphWorkPacket>(&bytes)
        .with_context(|| "deserialize GraphWorkPacket JSON")
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
    keypair: Arc<EphemeralKeypair>,
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
            &keypair,
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

/// CLOACI-T-0841: digests whose Python source has been imported in this
/// process — the import's side effect (graph executors in the process-global
/// registry) persists, so one import per digest suffices. A version upgrade
/// arrives as a NEW digest; the loader's T-0840 eviction makes its re-import
/// actually re-execute the module.
fn imported_py_graph_digests() -> &'static tokio::sync::Mutex<std::collections::HashSet<String>> {
    static CACHE: std::sync::OnceLock<tokio::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashSet::new()))
}

/// Execute one PYTHON whole-graph firing (CLOACI-T-0841): fetch the source
/// archive by digest, stage + import it via the agent's Python runtime (the
/// same path Python task packets use — providers staged, T-0840 module
/// eviction applied), then look up the named graph executor and run it with
/// the packet's cache rebuilt into the in-process `InputCache` wire shape.
async fn execute_python_graph(
    packet: &GraphWorkPacket,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
) -> cloacina::fleet::AgentOutcome {
    let digest = packet.artifact.digest.clone();

    // One import per digest; the graph executors it registers are global.
    {
        let mut imported = imported_py_graph_digests().lock().await;
        if !imported.contains(&digest) {
            let archive = match fetch_source_archive(http, server, api_key, &digest).await {
                Ok(a) => a,
                Err(e) => {
                    return cloacina::fleet::AgentOutcome::Refused {
                        reason: RefusalReason::ArtifactFetchFailed,
                        message: format!("python graph source fetch failed: {}", e),
                    };
                }
            };
            if let Err(e) = stage_agent_providers(http, server, api_key, &digest, cache_dir).await {
                return cloacina::fleet::AgentOutcome::Refused {
                    reason: RefusalReason::RuntimeLoadFailed,
                    message: e,
                };
            }
            let staging = cache_dir.join(format!("pysrc-{}", digest));
            if let Err(e) = std::fs::create_dir_all(&staging) {
                return cloacina::fleet::AgentOutcome::Refused {
                    reason: RefusalReason::RuntimeLoadFailed,
                    message: format!("create python staging dir {:?}: {}", staging, e),
                };
            }
            let Some(py) = cloacina::python_runtime::python_runtime() else {
                return cloacina::fleet::AgentOutcome::Refused {
                    reason: RefusalReason::RuntimeLoadFailed,
                    message: "no Python runtime installed in agent".to_string(),
                };
            };
            let tenant = packet
                .tenant_id
                .clone()
                .unwrap_or_else(|| "public".to_string());
            // Throwaway Runtime: we only need the import's registration side
            // effects (graph executors land in the process-global registry).
            let runtime_for_load = Arc::new(cloacina::Runtime::empty());
            let result = tokio::task::spawn_blocking(move || {
                py.load_workflow_package(&archive, &staging, &tenant, &runtime_for_load)
                    .map(|_| ())
            })
            .await;
            match result {
                Ok(Ok(())) => {
                    imported.insert(digest.clone());
                    debug!(digest = %digest, "python graph package imported");
                }
                Ok(Err(e)) => {
                    return cloacina::fleet::AgentOutcome::Refused {
                        reason: RefusalReason::RuntimeLoadFailed,
                        message: format!("python graph load: {}", e),
                    };
                }
                Err(e) => {
                    return cloacina::fleet::AgentOutcome::Refused {
                        reason: RefusalReason::RuntimeLoadFailed,
                        message: format!("python graph load panicked: {}", e),
                    };
                }
            }
        }
    }

    // The packet's graph_name is the REACTOR name (the reactor is the firing
    // unit); executors register under their CG name. Try a direct CG-name hit
    // (self-reactor graphs), then fan out to all subscriber graphs of the
    // reactor — the agent-side analog of the scheduler's subscriber dispatcher.
    let executors = match cloacina_python::computation_graph::get_graph_executor(&packet.graph_name)
    {
        Some(ex) => vec![ex],
        None => {
            cloacina_python::computation_graph::get_graph_executors_for_reactor(&packet.graph_name)
        }
    };
    if executors.is_empty() {
        return cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!(
                "no graph executor registered for '{}' (as CG name or reactor) after \
                 importing digest {} — package/graph mismatch",
                packet.graph_name, digest
            ),
        };
    }

    // Rebuild the InputCache in the exact in-process wire shape: each entry
    // is bincode(Vec<u8>) of the raw event JSON — byte-identical to what the
    // server-side accumulators store, so executor behavior matches a local
    // firing exactly.
    let mut cache = cloacina::computation_graph::types::InputCache::new();
    for (source, json_str) in &packet.cache {
        let frame = match bincode::serialize(&json_str.clone().into_bytes()) {
            Ok(f) => f,
            Err(e) => {
                return cloacina::fleet::AgentOutcome::Failure {
                    message: format!("rebuild cache entry '{}': {}", source, e),
                    classification: cloacina::fleet::FailureClassification::TaskError,
                };
            }
        };
        cache.update(
            cloacina::cloacina_computation_graph::SourceName::new(source.clone()),
            frame,
        );
    }

    // Execute every subscriber graph (usually one). Any error fails the
    // firing — the same aggregate the in-process subscriber dispatcher
    // reports. The Python executor holds the GIL inside its own
    // spawn_blocking.
    for executor in executors {
        let graph_name = executor.name.clone();
        match executor.execute(&cache).await {
            cloacina::cloacina_computation_graph::GraphResult::Completed { .. } => {
                debug!(
                    graph = %graph_name,
                    reactor = %packet.graph_name,
                    "python graph firing executed on agent"
                );
            }
            cloacina::cloacina_computation_graph::GraphResult::Error(e) => {
                return cloacina::fleet::AgentOutcome::Failure {
                    message: format!("python graph '{}' execution failed: {}", graph_name, e),
                    classification: cloacina::fleet::FailureClassification::TaskError,
                };
            }
        }
    }
    // Python graph outputs are PyObjects the reactor discards — report
    // success with no outputs (parity with in-process).
    cloacina::fleet::AgentOutcome::Success {
        context: serde_json::json!({ "outputs": [] }),
    }
}

/// CLOACI-T-0722: per-digest cache of loaded GRAPH plugins, mirroring the
/// task-side `loaded_runtimes()` — one dlopen per package, reused across
/// firings for the process lifetime.
fn loaded_graphs() -> &'static tokio::sync::Mutex<
    std::collections::HashMap<
        String,
        Arc<cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin>,
    >,
> {
    static CACHE: std::sync::OnceLock<
        tokio::sync::Mutex<
            std::collections::HashMap<
                String,
                Arc<cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin>,
            >,
        >,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()))
}

/// CLOACI-T-0722: spawn a worker that executes one whole-graph firing —
/// fetch the cdylib by digest, `execute_graph(cache)` via FFI, report the
/// outcome under the packet's `firing_id` (the server's rendezvous key).
#[allow(clippy::too_many_arguments)]
fn spawn_graph_worker(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    packet: GraphWorkPacket,
    in_flight: Arc<AtomicU32>,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
    cache_dir: Arc<std::path::PathBuf>,
) {
    in_flight.fetch_add(1, Ordering::SeqCst);
    tokio::spawn(async move {
        let start = Instant::now();
        let outcome =
            process_graph_packet(&packet, &http, &server, &api_key, cache_dir.as_path()).await;
        let duration_ms = start.elapsed().as_millis() as u64;
        let req = AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id: agent_id.clone(),
            task_execution_id: packet.firing_id.clone(),
            attempt: 1,
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

/// CLOACI-T-0722: refuse a graph firing pre-execution (saturation). The
/// server treats a refusal as "the agent did NOT run it" and falls back
/// in-process.
#[allow(clippy::too_many_arguments)]
fn spawn_graph_refusal(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    packet: GraphWorkPacket,
    message: String,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
) {
    tokio::spawn(async move {
        let req = AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id,
            task_execution_id: packet.firing_id.clone(),
            attempt: 1,
            duration_ms: 0,
            outcome: cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::Shutdown,
                message,
            },
        };
        report_outcome(&http, &server, &api_key, &req).await;
        if let Ok(ack) = make_ack(push_id) {
            let _ = ack_tx.send(Message::Text(ack.into()));
        }
    });
}

/// Execute one whole-graph firing (CLOACI-T-0722): fetch + cache the cdylib
/// by digest, load it once per digest, call the plugin's `execute_graph`
/// with the packet's pre-converted FFI cache, and map the result.
async fn process_graph_packet(
    packet: &GraphWorkPacket,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
) -> cloacina::fleet::AgentOutcome {
    // CLOACI-T-0841: a Python-packaged CG executes from SOURCE — import the
    // module (registering its graph executors) and run the named executor.
    if packet.language.as_deref() == Some("python") {
        return execute_python_graph(packet, http, server, api_key, cache_dir).await;
    }
    let digest = packet.artifact.digest.clone();

    let plugin = {
        let mut cache = loaded_graphs().lock().await;
        if let Some(p) = cache.get(&digest) {
            p.clone()
        } else {
            let artifact_path =
                match fetch_and_cache_artifact(http, server, api_key, &packet.artifact, cache_dir)
                    .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        return cloacina::fleet::AgentOutcome::Refused {
                            reason: RefusalReason::ArtifactFetchFailed,
                            message: format!("graph artifact fetch failed: {}", e),
                        };
                    }
                };
            let bytes = match std::fs::read(&artifact_path) {
                Ok(b) => b,
                Err(e) => {
                    return cloacina::fleet::AgentOutcome::Refused {
                        reason: RefusalReason::ArtifactFetchFailed,
                        message: format!("read cached graph artifact: {}", e),
                    };
                }
            };
            let loaded =
                match cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin::load(&bytes)
                {
                    Ok(p) => Arc::new(p),
                    Err(e) => {
                        return cloacina::fleet::AgentOutcome::Refused {
                            reason: RefusalReason::RuntimeLoadFailed,
                            message: format!("load graph plugin: {}", e),
                        };
                    }
                };
            cache.insert(digest.clone(), loaded.clone());
            loaded
        }
    };

    let request = cloacina::cloacina_workflow_plugin::GraphExecutionRequest {
        cache: packet.cache.clone(),
    };
    let plugin_for_call = plugin.clone();
    let result = tokio::task::spawn_blocking(move || plugin_for_call.execute_graph(request)).await;

    match result {
        Ok(Ok(ffi_result)) => {
            if ffi_result.success {
                let outputs: Vec<serde_json::Value> = ffi_result
                    .terminal_outputs_json
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
                    .collect();
                debug!(
                    graph = %packet.graph_name,
                    outputs = outputs.len(),
                    "graph firing executed on agent"
                );
                cloacina::fleet::AgentOutcome::Success {
                    context: serde_json::json!({ "outputs": outputs }),
                }
            } else {
                cloacina::fleet::AgentOutcome::Failure {
                    message: ffi_result
                        .error
                        .unwrap_or_else(|| "unknown FFI graph execution error".to_string()),
                    classification: cloacina::fleet::FailureClassification::TaskError,
                }
            }
        }
        Ok(Err(e)) => cloacina::fleet::AgentOutcome::Failure {
            message: format!("execute_graph FFI call failed: {}", e),
            classification: cloacina::fleet::FailureClassification::TaskError,
        },
        Err(join_err) => cloacina::fleet::AgentOutcome::Failure {
            message: format!("execute_graph panicked: {}", join_err),
            classification: cloacina::fleet::FailureClassification::TaskError,
        },
    }
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

/// Process-wide cache of loaded per-package runtimes, keyed by artifact digest
/// (CLOACI-T-0716). A package is loaded once — `dlopen` for Rust, PyO3 import
/// for Python — and the populated `Runtime` is reused for every subsequent
/// packet of that package. Required for Python (re-importing a module in a live
/// interpreter is a no-op, so the `@task` decorators wouldn't re-run) and a
/// useful efficiency win for Rust.
static LOADED_RUNTIMES: std::sync::OnceLock<
    tokio::sync::Mutex<std::collections::HashMap<String, Arc<cloacina::Runtime>>>,
> = std::sync::OnceLock::new();

fn loaded_runtimes(
) -> &'static tokio::sync::Mutex<std::collections::HashMap<String, Arc<cloacina::Runtime>>> {
    LOADED_RUNTIMES.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Fetch the package SOURCE archive (the uploaded `.cloacina`) by digest from
/// `GET /v1/agent/source/{digest}` — used for Python packages, which have no
/// cdylib (CLOACI-T-0716).
async fn fetch_source_archive(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    digest: &str,
) -> Result<Vec<u8>> {
    let url = format!("{}/v1/agent/source/{}", server, digest);
    let resp = http
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.bytes().await?.to_vec())
}

/// Load a Rust cdylib package into `runtime`: fetch + cache the artifact by
/// digest, then register its tasks via the fidius-backed `TaskRegistrar`.
async fn load_rust_cdylib(
    packet: &WorkPacket,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
    runtime: &Arc<cloacina::Runtime>,
) -> std::result::Result<(), cloacina::fleet::AgentOutcome> {
    let artifact_path =
        fetch_and_cache_artifact(http, server, api_key, &packet.artifact, cache_dir)
            .await
            .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::ArtifactFetchFailed,
                message: format!("artifact fetch failed: {}", e),
            })?;
    let cdylib_bytes =
        std::fs::read(&artifact_path).map_err(|e| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::ArtifactFetchFailed,
            message: format!("read cached artifact {:?}: {}", artifact_path, e),
        })?;
    let registrar = cloacina::registry::loader::TaskRegistrar::new().map_err(|e| {
        cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!("task registrar init: {}", e),
        }
    })?;
    let package_id = format!("agent_pkg_{}", packet.artifact.digest);
    let metadata = synthetic_package_metadata(&packet.artifact.digest);
    let registered = registrar
        .register_package_tasks(
            &package_id,
            &cdylib_bytes,
            &metadata,
            packet.tenant_id.as_deref(),
            runtime,
        )
        .await
        .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!("register_package_tasks: {}", e),
        })?;
    debug!(
        package_id = %package_id,
        registered = ?registered.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
        "cdylib registered"
    );

    // CLOACI-T-0838: resolve the package's `constructor!` node declarations —
    // the agent twin of the reconciler's Step 5b. Fail closed both ways: decls
    // with no bundled providers refuse the load (hermetic guarantee), and a
    // node that can't resolve refuses rather than surfacing later as a mystery
    // "task not registered".
    let loader = cloacina::registry::loader::package_loader::PackageLoader::new().map_err(|e| {
        cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!("package loader init: {}", e),
        }
    })?;
    let decls = loader
        .extract_constructor_metadata(&cdylib_bytes)
        .await
        .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!("extract constructor metadata: {}", e),
        })?;
    if !decls.is_empty() {
        let staged =
            stage_agent_providers(http, server, api_key, &packet.artifact.digest, cache_dir)
                .await
                .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
                    reason: RefusalReason::RuntimeLoadFailed,
                    message: e,
                })?;
        if staged == 0 {
            return Err(cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: format!(
                    "package declares {} constructor node(s) but the server has NO bundled \
                     providers for it — rebuild the package so the compiler bundles each \
                     `from` provider (CLOACI-T-0836)",
                    decls.len()
                ),
            });
        }
        // Namespace components come from the dispatched task's namespace so the
        // registered nodes match what the scheduler will look up.
        let ns = cloacina::parse_namespace(&packet.task_name).map_err(|e| {
            cloacina::fleet::AgentOutcome::Failure {
                message: format!("parse_namespace({}): {}", packet.task_name, e),
                classification: cloacina::fleet::FailureClassification::Validation,
            }
        })?;
        resolve_agent_constructor_nodes(decls, &ns.tenant_id, &ns.package_name, runtime)
            .await
            .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: e,
            })?;
    }
    Ok(())
}

/// CLOACI-T-0838: fetch the package's bundled CONSTRUCTOR PROVIDERS from the
/// server (`GET /v1/agent/providers/{digest}` — the fleet twin of the
/// reconciler reading `package_providers`), unpack them under `cache_dir`, and
/// point the process-global provider search path at them. Returns how many
/// providers were staged. Zero providers CLEARS the search path (hermeticity:
/// this package must not resolve against a previously-loaded package's
/// bundle). The staged tree is left in place — loaded constructor nodes re-read
/// their wasm components from it for the process lifetime.
async fn stage_agent_providers(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    digest: &str,
    cache_dir: &std::path::Path,
) -> std::result::Result<usize, String> {
    use base64::Engine as _;

    #[derive(serde::Deserialize)]
    struct ProviderEntry {
        name: String,
        data: String,
    }
    #[derive(serde::Deserialize)]
    struct ProvidersResponse {
        providers: Vec<ProviderEntry>,
    }

    let url = format!("{}/v1/agent/providers/{}", server, digest);
    let resp = http
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| format!("providers fetch: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("providers fetch: HTTP {}", resp.status()));
    }
    let body: ProvidersResponse = resp
        .json()
        .await
        .map_err(|e| format!("providers decode: {}", e))?;

    if body.providers.is_empty() {
        cloacina::registry::loader::clear_provider_search_path();
        return Ok(0);
    }

    let providers_root = cache_dir.join(format!("providers-{}", digest));
    std::fs::create_dir_all(&providers_root)
        .map_err(|e| format!("create providers dir {:?}: {}", providers_root, e))?;
    for entry in &body.providers {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&entry.data)
            .map_err(|e| format!("provider '{}' base64: {}", entry.name, e))?;
        let archive_path = providers_root.join(format!("{}.cloacina", entry.name));
        std::fs::write(&archive_path, bytes)
            .map_err(|e| format!("stage provider '{}': {}", entry.name, e))?;
        cloacina::registry::loader::unpack_provider_archive(&archive_path, &providers_root, &[])
            .map_err(|e| format!("unpack provider '{}': {}", entry.name, e))?;
    }
    cloacina::registry::loader::set_provider_search_path(&providers_root);
    Ok(body.providers.len())
}

/// CLOACI-T-0838: resolve each declared `constructor!` node against the staged
/// provider bundles and register it into the per-package runtime — the agent
/// twin of the reconciler's Step 5b (`step_load_constructor_nodes`). The
/// namespace components (tenant/package/workflow) come from the declaration +
/// the dispatched task's namespace, matching what the scheduler dispatches.
async fn resolve_agent_constructor_nodes(
    decls: Vec<cloacina::cloacina_workflow_plugin::ConstructorPackageMetadata>,
    tenant: &str,
    package_name: &str,
    runtime: &Arc<cloacina::Runtime>,
) -> std::result::Result<(), String> {
    use cloacina::registry::loader::grants::GrantSpec;
    use cloacina::registry::loader::load_constructor_node;
    use cloacina::TaskNamespace;

    for decl in decls {
        let namespace = TaskNamespace::new(tenant, package_name, &decl.workflow, &decl.id);
        let dependencies: Vec<TaskNamespace> = decl
            .dependencies
            .iter()
            .map(|dep| TaskNamespace::new(tenant, package_name, &decl.workflow, dep))
            .collect();
        let grants = GrantSpec::from_pairs(decl.grants.clone());
        // Config values crossed the FFI wire JSON-encoded (bincode can't carry
        // `serde_json::Value`); parse each back before binding.
        let config: Vec<(String, serde_json::Value)> = decl
            .config
            .iter()
            .map(|(k, raw)| {
                serde_json::from_str(raw)
                    .map(|v| (k.clone(), v))
                    .map_err(|e| {
                        format!(
                            "constructor node '{}': config value for '{}' is not valid JSON \
                         ({raw}): {e}",
                            decl.id, k
                        )
                    })
            })
            .collect::<Result<_, _>>()?;

        let (node_id, from, constructor) = (decl.id.clone(), decl.from.clone(), decl.constructor);
        // wasmtime compilation is blocking work; hop off the async executor.
        let node = tokio::task::spawn_blocking(move || {
            load_constructor_node(&node_id, &from, &constructor, config, dependencies, grants)
        })
        .await
        .map_err(|e| format!("constructor node '{}' load join: {}", decl.id, e))?
        .map_err(|e| {
            format!(
                "resolve constructor node '{}' (from = '{}'): {}",
                decl.id, decl.from, e
            )
        })?;

        runtime.register_task(namespace.clone(), move || node.clone());
        debug!(namespace = %namespace, "registered packaged constructor node (agent)");
    }
    Ok(())
}

/// Load a Python package into `runtime`: fetch the source archive, stage it,
/// and import the `workflow/` + `vendor/` tree via the installed `PythonRuntime`
/// (PyO3). The staging dir lives under `cache_dir` and is left in place — the
/// imported module's `sys.path` entries point into it for the process lifetime
/// (CLOACI-T-0716).
async fn load_python_package(
    packet: &WorkPacket,
    digest: &str,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
    runtime: &Arc<cloacina::Runtime>,
) -> std::result::Result<(), cloacina::fleet::AgentOutcome> {
    let archive = fetch_source_archive(http, server, api_key, digest)
        .await
        .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::ArtifactFetchFailed,
            message: format!("python source fetch failed: {}", e),
        })?;

    // CLOACI-T-0838: stage the package's bundled constructor providers BEFORE
    // the module import, so `cloaca.constructor(...)` calls resolve against
    // them during load (the agent twin of the reconciler's Python branch).
    // Zero providers clears the search path — an undeclared ref then fails the
    // import with a clear "no such provider" instead of leaking to a
    // previously-loaded package's bundle.
    stage_agent_providers(http, server, api_key, digest, cache_dir)
        .await
        .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: e,
        })?;

    let staging = cache_dir.join(format!("pysrc-{}", digest));
    std::fs::create_dir_all(&staging).map_err(|e| cloacina::fleet::AgentOutcome::Refused {
        reason: RefusalReason::RuntimeLoadFailed,
        message: format!("create python staging dir {:?}: {}", staging, e),
    })?;

    let py = cloacina::python_runtime::python_runtime().ok_or_else(|| {
        cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: "no Python runtime installed in agent".to_string(),
        }
    })?;

    // The tenant must match what the task lookup uses, which is the tenant
    // component of the packet's namespaced task_name (e.g. `public::pkg::wf::t`).
    // The public tenant rides as `tenant_id = None` in the packet, so deriving
    // from task_name (rather than `tenant_id.unwrap_or_default()` → "") is what
    // makes the Python tasks register under `public::…` instead of `::…`.
    let tenant = cloacina::parse_namespace(&packet.task_name)
        .map(|ns| ns.tenant_id)
        .unwrap_or_else(|_| packet.tenant_id.clone().unwrap_or_default());
    let runtime_for_load = runtime.clone();
    // load_workflow_package is synchronous + holds the GIL → run off the async
    // worker. We discard the LoadedPythonWorkflow (it may hold non-Send PyObjects)
    // and only care that the tasks registered into `runtime`.
    let result = tokio::task::spawn_blocking(move || {
        py.load_workflow_package(&archive, &staging, &tenant, &runtime_for_load)
            .map(|_| ())
    })
    .await;

    match result {
        Ok(Ok(())) => {
            debug!(digest = %digest, "python package imported");
            Ok(())
        }
        Ok(Err(e)) => Err(cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!("python load: {}", e),
        }),
        Err(e) => Err(cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!("python load task panicked: {}", e),
        }),
    }
}

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
#[allow(clippy::too_many_arguments)]
async fn process_work_packet(
    packet: &WorkPacket,
    agent_triple: &str,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
    keypair: &EphemeralKeypair,
) -> cloacina::fleet::AgentOutcome {
    // 1-3. Resolve (or build) the per-package runtime, cached by digest.
    //
    // Loading is done once per package and the populated Runtime is reused
    // across packets: re-importing a Python module in a live interpreter is a
    // no-op (so the `@task` decorators wouldn't re-run and the new Runtime would
    // be empty), and for Rust it avoids a redundant dlopen per task.
    let digest = packet.artifact.digest.clone();
    let is_python = packet.language.as_deref() == Some("python");

    // OQ-6 fail-closed for Rust cdylibs (built for a specific target). Python is
    // interpreted, so the artifact triple is irrelevant — skip the gate.
    if !is_python && packet.artifact.build_target_triple != agent_triple {
        return cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::TargetTripleMismatch,
            message: format!(
                "agent target triple `{}` does not match artifact build target `{}`",
                agent_triple, packet.artifact.build_target_triple
            ),
        };
    }

    let runtime = {
        // The lock is held across a (one-time, per-digest) load so concurrent
        // packets for the same package don't double-load; cache hits are a quick
        // map lookup + Arc clone.
        let mut cache = loaded_runtimes().lock().await;
        if let Some(rt) = cache.get(&digest) {
            rt.clone()
        } else {
            let rt = Arc::new(cloacina::Runtime::new());
            let loaded = if is_python {
                load_python_package(packet, &digest, http, server, api_key, cache_dir, &rt).await
            } else {
                load_rust_cdylib(packet, http, server, api_key, cache_dir, &rt).await
            };
            if let Err(out) = loaded {
                return out;
            }
            cache.insert(digest.clone(), rt.clone());
            rt
        }
    };

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
                "task `{}` not registered after loading package (registered: {:?})",
                packet.task_name,
                runtime
                    .task_namespaces()
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
            ),
            classification: cloacina::fleet::FailureClassification::Validation,
        };
    };

    // 5. Build context from the work packet.
    let mut context = match build_context(&packet.context) {
        Ok(c) => c,
        Err(e) => {
            return cloacina::fleet::AgentOutcome::Failure {
                message: format!("build context: {}", e),
                classification: cloacina::fleet::FailureClassification::Validation,
            };
        }
    };

    // 5b. CLOACI-T-0861 — unwrap the HPKE-wrapped secrets with our ephemeral
    //     private key into an in-memory resolver and attach it to the Context, so
    //     the task body reads them via `ctx.secret(name)`. The plaintext lives
    //     only in this resolver, only for the run; it is never persisted or logged
    //     by the agent (NFR-001/NFR-003). The AAD is keyed by the packet's
    //     `task_execution_id`, matching how the server wrapped it.
    if !packet.wrapped_secrets.is_empty() {
        match InMemorySecretResolver::from_wrapped(
            &keypair.private,
            &packet.wrapped_secrets,
            &packet.task_execution_id,
        ) {
            Ok(resolver) => context.set_secret_resolver(resolver.into_arc()),
            Err(e) => {
                // Names only — `FleetSecretError` never renders a secret value.
                return cloacina::fleet::AgentOutcome::Failure {
                    message: format!("unwrap secrets: {}", e),
                    classification: cloacina::fleet::FailureClassification::Validation,
                };
            }
        }
    }

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
        // Agent synthesizes this for dynamic dispatch; no declared input
        // interface here (CLOACI-I-0128).
        declared_params: vec![],
        declared_surfaces: vec![],
        task_docs: Default::default(),
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
            language: Some("rust".into()),
            wrapped_secrets: Vec::new(),
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
            &generate_ephemeral_keypair(),
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
            &generate_ephemeral_keypair(),
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
