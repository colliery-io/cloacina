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

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use clap::Parser;
use cloacina::fleet::{
    AGENT_PROTOCOL_VERSION, AGENT_RECIPIENT_PREFIX, AgentHeartbeatRequest, AgentHeartbeatResponse,
    AgentKeyReplenishRequest, AgentKeyReplenishResponse, AgentRegisterRequest,
    AgentRegisterResponse, AgentResultRequest, EphemeralKeyEntry, GRAPH_PACKET_KIND,
    GraphWorkPacket, RefusalReason, WORK_PACKET_KIND, WorkPacket, host_target_triple,
};
use cloacina::security::{AgentKeyPool, InMemorySecretResolver};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

/// CLOACI-T-0861 / D-5 — how many one-time ephemeral keys the agent advertises at
/// registration, and the low-water mark below which it proactively tops up. The
/// server consumes one key per secret-bearing dispatch, so this bounds how many
/// secret dispatches can be served before a replenish is needed.
const KEY_POOL_TARGET: usize = 32;
/// Proactively replenish when the local pool drops to (or below) this many keys,
/// independent of the server's own replenish signal.
const KEY_POOL_LOW_WATER: usize = 8;

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
    /// API key used for both REST and the WS ticket mint. Give it here, in
    /// `CLOACINA_API_KEY`, or in a file with `--api-key-file`. The agent takes
    /// the variable out of its environment at start (CLOACINA-T-1033).
    #[arg(long, env = API_KEY_ENV, default_value = "", hide_default_value = true)]
    api_key: String,
    /// File that holds the API key (surrounding whitespace is ignored). Use it
    /// in place of `--api-key` / `CLOACINA_API_KEY`, not with them.
    #[arg(long, env = "CLOACINA_API_KEY_FILE")]
    api_key_file: Option<std::path::PathBuf>,
    /// Optional caller-chosen agent id; server assigns one if omitted.
    #[arg(long)]
    agent_id: Option<String>,
    /// Max concurrent work packets this agent will accept.
    #[arg(long, env = "CLOACINA_AGENT_MAX_CONCURRENCY", default_value_t = 4)]
    max_concurrency: u32,
    /// Free-form capability tags advertised at registration.
    #[arg(long, value_delimiter = ',')]
    capabilities: Vec<String>,
    /// The agent pool this agent is in. Registered as the capability
    /// `pool:<name>`. Not set: the server counts the agent in `default`.
    #[arg(long, env = "CLOACINA_AGENT_POOL")]
    pool: Option<String>,
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

/// The environment variable the agent's key may arrive in.
const API_KEY_ENV: &str = "CLOACINA_API_KEY";

/// Take the agent's key out of the process environment (CLOACINA-T-1033).
///
/// Task code runs inside the agent process, so anything left in the
/// environment is readable by a task (`std::env`, Python's `os.environ`).
/// On Unix the value is first overwritten where it lies, because
/// `/proc/<pid>/environ` shows the start-time environment block and
/// `remove_var` only drops the pointer to it.
///
/// # Safety
///
/// No other thread may read or write the environment while this runs. `main`
/// calls it before the tokio runtime, the logger and the Python interpreter
/// exist, so the process has one thread.
unsafe fn remove_api_key_from_env() {
    #[cfg(unix)]
    {
        let name = std::ffi::CString::new(API_KEY_ENV).expect("no NUL in the name");
        // SAFETY: single-threaded (see above). `getenv` returns a pointer to
        // the NUL-terminated value inside the environment block, which is
        // writable memory of this process; the write stays inside the value.
        unsafe {
            let value = libc::getenv(name.as_ptr());
            if !value.is_null() {
                std::ptr::write_bytes(value, 0, libc::strlen(value));
            }
        }
    }
    // SAFETY: single-threaded (see above).
    unsafe { std::env::remove_var(API_KEY_ENV) };
}

/// Settle where the key comes from: `--api-key` / `CLOACINA_API_KEY`, or the
/// file named by `--api-key-file` / `CLOACINA_API_KEY_FILE`. The file is not
/// deleted: a restart of the agent reads it again.
fn resolve_api_key(args: &mut Args) -> Result<()> {
    match (&args.api_key_file, args.api_key.is_empty()) {
        (Some(_), false) => bail!(
            "give the API key one way: --api-key-file / CLOACINA_API_KEY_FILE, or --api-key / \
             CLOACINA_API_KEY"
        ),
        (Some(path), true) => {
            let key = std::fs::read_to_string(path)
                .with_context(|| format!("read the API key file {:?}", path))?;
            let key = key.trim();
            if key.is_empty() {
                bail!("the API key file {:?} is empty", path);
            }
            args.api_key = key.to_string();
        }
        (None, false) => {}
        (None, true) => bail!(
            "no API key: set CLOACINA_API_KEY, or name a file with --api-key-file / \
             CLOACINA_API_KEY_FILE"
        ),
    }
    Ok(())
}

fn main() -> Result<()> {
    // Before anything else, while this is the only thread: read the key and
    // take it out of the environment (CLOACINA-T-1033).
    let mut args = Args::parse();
    // SAFETY: no thread has been started yet.
    unsafe { remove_api_key_from_env() };
    resolve_api_key(&mut args)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build the tokio runtime")?
        .block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<()> {
    // rustls has both ring and aws-lc-rs compiled in; without a process
    // default, a wss:// connect panics choosing one.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

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

    // CLOACI-T-0861 / D-5 — process-wide one-time ephemeral key pool. Shared
    // across reconnect sessions and per-packet workers: a task still unwrapping
    // secrets when the WS drops must keep the private key it needs, and each
    // register/replenish mints fresh keys into this same pool.
    let key_pool = Arc::new(AsyncMutex::new(AgentKeyPool::new()));

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
            key_pool.clone(),
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
    key_pool: Arc<AsyncMutex<AgentKeyPool>>,
) -> Result<()> {
    // ── 0. CLOACI-T-0861 / D-5 — mint a fresh batch of one-time ephemeral keys
    //       for this registration and advertise their PUBLIC halves as a pool. The
    //       private halves stay in `key_pool` (shared across workers + reconnects);
    //       the server consumes one key per secret-bearing dispatch and stamps its
    //       `key_id` on the WorkPacket. A leaked key exposes at most ONE execution.
    let ephemeral_key_pool: Vec<EphemeralKeyEntry> =
        { key_pool.lock().await.mint(KEY_POOL_TARGET) };

    // ── 1. Register (fresh each session — the server may have restarted and
    //       forgotten us, in which case a stale agent_id would be unroutable).
    // CLOACINA-T-1021: the agent's pool is registered as the capability
    // `pool:<name>`. With no pool the server counts the agent in `default`.
    let mut capabilities = args.capabilities.clone();
    if let Some(pool) = args.pool.as_deref().filter(|p| !p.is_empty()) {
        capabilities.push(format!("pool:{pool}"));
    }
    let register_resp = register(
        http,
        server_base,
        &args.api_key,
        &args.agent_id,
        args.max_concurrency,
        target_triple,
        &capabilities,
        ephemeral_key_pool,
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

    // ── 2. Spawn the heartbeat task for THIS session. It also carries the D-5
    //       key-pool top-up: each heartbeat response may ask the agent to
    //       replenish, and the loop proactively tops up when the local pool runs
    //       low.
    let heartbeat = spawn_heartbeat_loop(
        http.clone(),
        server_base.to_string(),
        args.api_key.clone(),
        agent_id.clone(),
        args.max_concurrency,
        in_flight.clone(),
        Duration::from_secs(register_resp.heartbeat_interval_seconds.max(1) as u64),
        key_pool.clone(),
    );

    // ── 3+4. Mint a ticket, connect, and serve. Bundled so the heartbeat is
    //         aborted on any early exit (mint/connect failure or loop end).
    let session = async {
        let ticket = mint_ws_ticket(http, server_base, &args.api_key)
            .await
            .context("mint WS ticket")?;
        let ws_url = ws_url_for(server_base, &agent_id, &ticket)?;
        info!(ws_url = %without_query(&ws_url), "connecting to substrate delivery WS");
        let (stream, _resp) = tokio_tungstenite::connect_async(&ws_url)
            .await
            .with_context(|| format!("WS connect to {}", without_query(&ws_url)))?;
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
            key_pool.clone(),
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
    ephemeral_key_pool: Vec<EphemeralKeyEntry>,
) -> Result<AgentRegisterResponse> {
    let body = AgentRegisterRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.clone(),
        max_concurrency,
        target_triple: target_triple.to_string(),
        capabilities: capabilities.to_vec(),
        // CLOACI-T-0861 / D-5 — advertise the one-time key POOL. The private
        // halves stay in the process `key_pool` and never leave. The single-key
        // field is retired in favor of the pool.
        ephemeral_public_key: None,
        ephemeral_key_pool,
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

/// Why one POST of a report failed (CLOACINA-T-1015).
#[derive(Debug)]
enum PostError {
    /// The server will never accept this report: do not send it again.
    Permanent(String),
    /// The server could not be reached or could not take the report now.
    Transient(String),
}

/// Whether a response status means "this report can never be accepted". A
/// 4xx is the server's answer to the report itself: a protocol mismatch
/// (400), a key of another tenant (403), a bad key (401). 408 and 429 are the
/// two 4xx that ask the client to come back.
fn is_permanent_status(status: reqwest::StatusCode) -> bool {
    status.is_client_error()
        && status != reqwest::StatusCode::REQUEST_TIMEOUT
        && status != reqwest::StatusCode::TOO_MANY_REQUESTS
}

/// POST one report, one time, with a time limit on the whole request.
async fn post_result(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &AgentResultRequest,
    attempt_timeout: Duration,
) -> std::result::Result<(), PostError> {
    let resp = http
        .post(format!("{}/v1/agent/result", server))
        .bearer_auth(api_key)
        .timeout(attempt_timeout)
        .json(req)
        .send()
        .await
        .map_err(|e| PostError::Transient(e.to_string()))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().await.unwrap_or_default();
    let message = format!("HTTP {}: {}", status.as_u16(), short(&body));
    if is_permanent_status(status) {
        Err(PostError::Permanent(message))
    } else {
        Err(PostError::Transient(message))
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_heartbeat_loop(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    max_concurrency: u32,
    in_flight: Arc<AtomicU32>,
    interval: Duration,
    key_pool: Arc<AsyncMutex<AgentKeyPool>>,
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
                in_flight_push_ids: push_registry().in_work_ids(),
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
                    // CLOACI-T-0861 / D-5 — honor the server's replenish signal AND
                    // proactively top up when the local pool runs low.
                    let requested = resp
                        .json::<AgentHeartbeatResponse>()
                        .await
                        .map(|r| r.replenish_keys as usize)
                        .unwrap_or(0);
                    let local = { key_pool.lock().await.len() };
                    if requested > 0 || local <= KEY_POOL_LOW_WATER {
                        maybe_replenish_keys(&http, &server, &api_key, &agent_id, &key_pool).await;
                    }
                }
                Ok(resp) => warn!(status = %resp.status(), "heartbeat non-success"),
                Err(e) => warn!(error = %e, "heartbeat request failed"),
            }
        }
    })
}

/// CLOACI-T-0861 / D-5 — mint enough fresh one-time keys to bring the local pool
/// back up to [`KEY_POOL_TARGET`] and POST them to the server's key-pool top-up
/// endpoint. Best-effort: a failed top-up is retried on the next heartbeat.
async fn maybe_replenish_keys(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    agent_id: &str,
    key_pool: &Arc<AsyncMutex<AgentKeyPool>>,
) {
    let fresh: Vec<EphemeralKeyEntry> = {
        let mut pool = key_pool.lock().await;
        let deficit = KEY_POOL_TARGET.saturating_sub(pool.len());
        if deficit == 0 {
            return;
        }
        pool.mint(deficit)
    };
    let n = fresh.len();
    let body = AgentKeyReplenishRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.to_string(),
        keys: fresh,
    };
    match http
        .post(format!("{}/v1/agent/keys", server))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let accepted = resp
                .json::<AgentKeyReplenishResponse>()
                .await
                .map(|r| r.accepted)
                .unwrap_or(0);
            debug!(minted = n, accepted, "key pool replenished");
        }
        Ok(resp) => warn!(status = %resp.status(), "key replenish non-success"),
        Err(e) => warn!(error = %e, "key replenish request failed"),
    }
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
    key_pool: Arc<AsyncMutex<AgentKeyPool>>,
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
                        &key_pool,
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
    key_pool: &Arc<AsyncMutex<AgentKeyPool>>,
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
            // CLOACINA-T-1011: one push id starts one worker. The server pushes
            // an unacked row again after a reconnect or a sweep.
            let state = push_registry().begin(push_id);
            match state {
                PushState::InWork => {
                    debug!(push_id, "push already in work; ignoring the re-push");
                    return Ok(());
                }
                PushState::Done => {
                    debug!(push_id, "push already done; acking again");
                    let _ = ack_tx.send(Message::Text(make_ack(push_id)?.into()));
                    return Ok(());
                }
                PushState::New => {}
            }
            let mut guard = PushGuard::new(push_id);
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
                    guard.hand_over();
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
                guard.hand_over();
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
                guard.hand_over();
                finish_and_ack(ack_tx, push_id);
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
                guard.hand_over();
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
            guard.hand_over();
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
                key_pool.clone(),
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

/// How many finished push ids the agent remembers, to ack a late re-push
/// again without running it.
const DONE_PUSH_MEMORY: usize = 4096;

/// What the agent knows about one push id (CLOACINA-T-1011).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PushState {
    /// Not seen before: now marked as in work.
    New,
    /// A worker has this push in work. Do not start it again.
    InWork,
    /// This push finished and was acked. Ack it again, do not run it.
    Done,
}

/// The push ids this agent has in work and the ones it finished lately.
///
/// Delivery is at-least-once (CLOACINA-A-0006, CLOACINA-A-0016): the server pushes an outbox
/// row again after a reconnect or a sweep. The agent acks a row only when the
/// work ends, so a long task sees its own packet again. This registry makes
/// the agent an idempotent recipient: one push id starts one worker.
#[derive(Default)]
struct PushRegistry {
    in_work: std::collections::HashSet<i64>,
    done: std::collections::HashSet<i64>,
    done_order: std::collections::VecDeque<i64>,
}

impl PushRegistry {
    /// Record the arrival of a push. `New` marks the id as in work.
    fn begin(&mut self, id: i64) -> PushState {
        if self.in_work.contains(&id) {
            return PushState::InWork;
        }
        if self.done.contains(&id) {
            return PushState::Done;
        }
        self.in_work.insert(id);
        PushState::New
    }

    /// The work of this push ended and its ack is on the way.
    fn finish(&mut self, id: i64) {
        self.in_work.remove(&id);
        if self.done.insert(id) {
            self.done_order.push_back(id);
            while self.done_order.len() > DONE_PUSH_MEMORY {
                if let Some(old) = self.done_order.pop_front() {
                    self.done.remove(&old);
                }
            }
        }
    }

    /// The push could not be handled (a bad frame): forget it, so a later
    /// push of the same id is handled again.
    fn abandon(&mut self, id: i64) {
        self.in_work.remove(&id);
    }

    fn in_work_ids(&self) -> Vec<i64> {
        let mut ids: Vec<i64> = self.in_work.iter().copied().collect();
        ids.sort_unstable();
        ids
    }
}

/// Process-wide, so it outlives a WebSocket reconnect: the server pushes the
/// open rows again on each new connection.
fn push_registry() -> std::sync::MutexGuard<'static, PushRegistry> {
    static REGISTRY: std::sync::OnceLock<std::sync::Mutex<PushRegistry>> =
        std::sync::OnceLock::new();
    REGISTRY
        .get_or_init(|| std::sync::Mutex::new(PushRegistry::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Removes a push id from the registry when the frame handler fails before a
/// worker takes the push over.
struct PushGuard {
    id: i64,
    armed: bool,
}

impl PushGuard {
    fn new(id: i64) -> Self {
        Self { id, armed: true }
    }
    /// A worker (or an ack path) owns the push id now.
    fn hand_over(&mut self) {
        self.armed = false;
    }
}

impl Drop for PushGuard {
    fn drop(&mut self) {
        if self.armed {
            push_registry().abandon(self.id);
        }
    }
}

/// Mark the push as finished and queue its ack.
fn finish_and_ack(ack_tx: &mpsc::UnboundedSender<Message>, push_id: i64) {
    push_registry().finish(push_id);
    if let Ok(ack) = make_ack(push_id) {
        let _ = ack_tx.send(Message::Text(ack.into()));
    }
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
    key_pool: Arc<AsyncMutex<AgentKeyPool>>,
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
            &key_pool,
        )
        .await;
        let duration_ms = start.elapsed().as_millis() as u64;
        let req = AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id: agent_id.clone(),
            task_execution_id: packet.task_execution_id.clone(),
            attempt: packet.attempt,
            dispatch_id: packet.dispatch_id.clone(),
            duration_ms,
            outcome,
        };
        // CLOACINA-T-1015: the ack waits for the report (see `report_outcome`).
        let server_gives_up_at = server_wait_end(start, packet.timeout_seconds);
        report_outcome(&http, &server, &api_key, &req, server_gives_up_at).await;
        finish_and_ack(&ack_tx, push_id);
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
            dispatch_id: None,
            duration_ms,
            outcome,
        };
        let server_gives_up_at = server_wait_end(start, packet.timeout_seconds);
        report_outcome(&http, &server, &api_key, &req, server_gives_up_at).await;
        finish_and_ack(&ack_tx, push_id);
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
            dispatch_id: None,
            duration_ms: 0,
            outcome: cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::Shutdown,
                message,
            },
        };
        let server_gives_up_at = server_wait_end(Instant::now(), packet.timeout_seconds);
        report_outcome(&http, &server, &api_key, &req, server_gives_up_at).await;
        finish_and_ack(&ack_tx, push_id);
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
                dispatch_id: packet.dispatch_id.clone(),
                duration_ms: 0,
                outcome: cloacina::fleet::AgentOutcome::Refused { reason, message },
            };
            let server_gives_up_at = server_wait_end(Instant::now(), packet.timeout_seconds);
            report_outcome(&http, &server, &api_key, &req, server_gives_up_at).await;
        }
        finish_and_ack(&ack_tx, push_id);
    });
}

/// How the agent retries the POST of a report (CLOACINA-T-1015).
#[derive(Debug, Clone, Copy)]
struct ReportPolicy {
    /// Time limit of one POST.
    attempt_timeout: Duration,
    /// Wait before the second try; doubled after each failure.
    backoff_min: Duration,
    /// The longest wait between two tries.
    backoff_max: Duration,
    /// The most POSTs for one report.
    max_attempts: u32,
    /// The agent keeps trying at least this long after the first failure,
    /// also when the server's wait should be over by then. A late report
    /// costs nothing (the server drops it), and the server may wait longer
    /// than the packet says.
    min_window: Duration,
}

impl Default for ReportPolicy {
    fn default() -> Self {
        Self {
            attempt_timeout: Duration::from_secs(15),
            backoff_min: Duration::from_secs(1),
            backoff_max: Duration::from_secs(15),
            max_attempts: 12,
            min_window: Duration::from_secs(30),
        }
    }
}

/// What became of a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportDelivery {
    /// The server took it.
    Delivered,
    /// The server refused it for good (a 4xx). Not sent again.
    Rejected,
    /// Every try failed and the retries are used up.
    GaveUp,
}

/// When the server most likely stops waiting for the report of a packet: the
/// packet's own time limit after the agent got it. The server waits a fixed
/// time from the dispatch, and the packet's `timeout_seconds` is that time or
/// less. This is the agent's estimate; [`ReportPolicy::min_window`] is the
/// floor under it.
fn server_wait_end(received_at: Instant, timeout_seconds: u32) -> Instant {
    received_at + Duration::from_secs(u64::from(timeout_seconds))
}

/// POST a report until the server takes it (CLOACINA-T-1015).
///
/// A failed POST used to be logged and the packet acked: a server restart, a
/// database outage or a network error lost the result of work that had ended,
/// and the task ran again after the server's result wait.
///
/// Now a failure that can pass (network error, time limit, 5xx, 408, 429) is
/// tried again with a doubling backoff. The tries stop at
/// [`ReportPolicy::max_attempts`], or when the next try would start after
/// `server_gives_up_at` (and after the minimum window): past that point the
/// server has run its own retry and a report changes nothing. A 4xx that
/// rejects the report itself is not tried again.
///
/// The caller acks the packet after this returns, whatever the result. With
/// the retries used up the ack is still right: the server's result wait ends,
/// and its retry policy decides the next run. An unacked row would be pushed
/// to this agent again and run again for a dispatch the server gave up on, on
/// top of the server's own retry. While the tries go on the push id is still
/// in work, so the heartbeat keeps its lease and a re-push is ignored.
async fn report_outcome(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &AgentResultRequest,
    server_gives_up_at: Instant,
) -> ReportDelivery {
    report_with_policy(
        http,
        server,
        api_key,
        req,
        server_gives_up_at,
        &ReportPolicy::default(),
    )
    .await
}

async fn report_with_policy(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &AgentResultRequest,
    server_gives_up_at: Instant,
    policy: &ReportPolicy,
) -> ReportDelivery {
    let mut delay = policy.backoff_min;
    let mut stop_at: Option<Instant> = None;
    let mut attempt = 1u32;
    loop {
        let error = match post_result(http, server, api_key, req, policy.attempt_timeout).await {
            Ok(()) => {
                debug!(
                    task_id = %req.task_execution_id,
                    outcome = %summarize_outcome(&req.outcome),
                    attempt,
                    "result reported"
                );
                return ReportDelivery::Delivered;
            }
            Err(PostError::Permanent(e)) => {
                error!(
                    task_id = %req.task_execution_id,
                    error = %e,
                    "the server rejected the result; not sent again"
                );
                return ReportDelivery::Rejected;
            }
            Err(PostError::Transient(e)) => e,
        };
        let now = Instant::now();
        let stop_at = *stop_at.get_or_insert(server_gives_up_at.max(now + policy.min_window));
        if attempt >= policy.max_attempts || now + delay > stop_at {
            error!(
                task_id = %req.task_execution_id,
                attempts = attempt,
                error = %error,
                "result not delivered and the retries are used up; \
                 the server runs the task again after its result wait"
            );
            return ReportDelivery::GaveUp;
        }
        warn!(
            task_id = %req.task_execution_id,
            attempt,
            retry_in_ms = delay.as_millis() as u64,
            error = %error,
            "result POST failed; trying again"
        );
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(policy.backoff_max);
        attempt += 1;
    }
}

/// The start of a response body, short enough for one log line.
fn short(s: &str) -> String {
    s.chars().take(300).collect()
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

fn loaded_runtimes()
-> &'static tokio::sync::Mutex<std::collections::HashMap<String, Arc<cloacina::Runtime>>> {
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

    // CLOACI-T-0908: name our arch so the server hands us OUR per-arch build of
    // any native provider (falling back to the primary when none exists).
    let url = format!(
        "{}/v1/agent/providers/{}?target_triple={}",
        server,
        digest,
        cloacina::fleet::protocol::host_target_triple()
    );
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
    use cloacina::TaskNamespace;
    use cloacina::registry::loader::grants::GrantSpec;
    use cloacina::registry::loader::load_constructor_node;

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
    key_pool: &Arc<AsyncMutex<AgentKeyPool>>,
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

    // 5b. CLOACI-T-0861 / D-5 — unwrap the HPKE-wrapped secrets with the ONE-TIME
    //     pooled private key the server named in `secret_key_id`. We `take` that
    //     key out of the pool (one-time use → discarded after this) and unwrap
    //     into an in-memory resolver attached to the Context, so the task body
    //     reads them via `ctx.secret(name)`. The plaintext lives only in this
    //     resolver, only for the run; it is never persisted or logged (NFR-001/
    //     NFR-003). The AAD is keyed by `task_execution_id`, matching the wrap.
    if !packet.wrapped_secrets.is_empty() {
        let Some(key_id) = packet.secret_key_id.as_deref() else {
            // Secrets present but no key id — a malformed dispatch. Fail closed
            // rather than run with missing secrets.
            return cloacina::fleet::AgentOutcome::Failure {
                message: "work packet carries wrapped_secrets but no secret_key_id".to_string(),
                classification: cloacina::fleet::FailureClassification::Validation,
            };
        };
        // One-time discard: remove the key from the pool. An unknown key_id (e.g.
        // the agent restarted after the server consumed the key) is a clean
        // failure — never an execution with missing secrets.
        let private_key = { key_pool.lock().await.take(key_id) };
        let Some(private_key) = private_key else {
            return cloacina::fleet::AgentOutcome::Failure {
                message: format!(
                    "unknown or already-used one-time secret key '{}' — cannot unwrap secrets \
                     (agent may have restarted since the key was advertised)",
                    key_id
                ),
                classification: cloacina::fleet::FailureClassification::Validation,
            };
        };
        match InMemorySecretResolver::from_wrapped(
            &private_key,
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

/// The URL with its query removed, for logs and error text: the query of the
/// delivery WebSocket URL is `?token=<ticket>`, a credential (CLOACINA-T-1040).
fn without_query(url: &str) -> &str {
    url.split_once('?').map_or(url, |(base, _)| base)
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
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

    // ── CLOACINA-T-1015: the result POST is retried ──────────────────────

    /// A local HTTP server that answers each request with the next status of
    /// `statuses` (the last one repeats) and counts the requests.
    async fn result_server(statuses: Vec<u16>) -> (String, Arc<AtomicU32>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let hits = Arc::new(AtomicU32::new(0));
        let counter = hits.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                let n = counter.fetch_add(1, Ordering::SeqCst) as usize;
                let status = statuses[n.min(statuses.len() - 1)];
                tokio::spawn(async move {
                    // Read the head and the body the head announces.
                    let mut buf = Vec::new();
                    let mut chunk = [0u8; 4096];
                    loop {
                        let Ok(read) = sock.read(&mut chunk).await else {
                            return;
                        };
                        if read == 0 {
                            break;
                        }
                        buf.extend_from_slice(&chunk[..read]);
                        let text = String::from_utf8_lossy(&buf).to_ascii_lowercase();
                        if let Some(head_end) = text.find("\r\n\r\n") {
                            let want = text
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:"))
                                .and_then(|v| v.trim().parse::<usize>().ok())
                                .unwrap_or(0);
                            if buf.len() >= head_end + 4 + want {
                                break;
                            }
                        }
                    }
                    let body = r#"{"protocol_version":1}"#;
                    let resp = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n\
                         content-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = sock.write_all(resp.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        (format!("http://{addr}"), hits)
    }

    fn a_report() -> AgentResultRequest {
        AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id: "agent-a".into(),
            task_execution_id: "00000000-0000-0000-0000-000000000001".into(),
            attempt: 1,
            dispatch_id: Some("d1".into()),
            duration_ms: 1,
            outcome: cloacina::fleet::AgentOutcome::Success {
                context: serde_json::json!({}),
            },
        }
    }

    /// The real policy with the waits cut to milliseconds.
    fn fast_policy() -> ReportPolicy {
        ReportPolicy {
            attempt_timeout: Duration::from_secs(5),
            backoff_min: Duration::from_millis(5),
            backoff_max: Duration::from_millis(20),
            max_attempts: 5,
            min_window: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn result_post_is_retried_until_the_server_takes_it() {
        let (server, hits) = result_server(vec![503, 500, 200]).await;
        let http = reqwest::Client::new();
        let got = report_with_policy(
            &http,
            &server,
            "key",
            &a_report(),
            Instant::now() + Duration::from_secs(60),
            &fast_policy(),
        )
        .await;
        assert_eq!(got, ReportDelivery::Delivered);
        assert_eq!(hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn result_post_is_retried_when_the_server_is_not_reachable_yet() {
        // Nothing listens on the port at first: a connection error.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let http = reqwest::Client::new();
        let got = report_with_policy(
            &http,
            &format!("http://{addr}"),
            "key",
            &a_report(),
            Instant::now() + Duration::from_secs(60),
            &ReportPolicy {
                max_attempts: 3,
                ..fast_policy()
            },
        )
        .await;
        assert_eq!(got, ReportDelivery::GaveUp, "3 tries, then it gives up");
    }

    #[tokio::test]
    async fn result_post_rejected_for_good_is_sent_one_time() {
        for status in [400u16, 401, 403] {
            let (server, hits) = result_server(vec![status]).await;
            let http = reqwest::Client::new();
            let got = report_with_policy(
                &http,
                &server,
                "key",
                &a_report(),
                Instant::now() + Duration::from_secs(60),
                &fast_policy(),
            )
            .await;
            assert_eq!(got, ReportDelivery::Rejected, "status {status}");
            assert_eq!(hits.load(Ordering::SeqCst), 1, "status {status}");
        }
    }

    #[tokio::test]
    async fn result_post_stops_at_the_attempt_limit() {
        let (server, hits) = result_server(vec![503]).await;
        let http = reqwest::Client::new();
        let got = report_with_policy(
            &http,
            &server,
            "key",
            &a_report(),
            Instant::now() + Duration::from_secs(60),
            &fast_policy(),
        )
        .await;
        assert_eq!(got, ReportDelivery::GaveUp);
        assert_eq!(hits.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn result_post_stops_when_the_server_has_given_up() {
        // The server's wait is over and the minimum window is shorter than
        // the next backoff: one try, no more.
        let (server, hits) = result_server(vec![503]).await;
        let http = reqwest::Client::new();
        let got = report_with_policy(
            &http,
            &server,
            "key",
            &a_report(),
            Instant::now(),
            &ReportPolicy {
                backoff_min: Duration::from_millis(50),
                min_window: Duration::from_millis(10),
                ..fast_policy()
            },
        )
        .await;
        assert_eq!(got, ReportDelivery::GaveUp);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn result_post_keeps_trying_for_the_minimum_window_after_the_wait_ended() {
        let (server, hits) = result_server(vec![503, 503, 200]).await;
        let http = reqwest::Client::new();
        let got = report_with_policy(
            &http,
            &server,
            "key",
            &a_report(),
            Instant::now(),
            &fast_policy(),
        )
        .await;
        assert_eq!(got, ReportDelivery::Delivered);
        assert_eq!(hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn the_ack_of_a_packet_waits_for_its_report() {
        // The first POST fails; the real policy waits 1 s before the second.
        let (server, hits) = result_server(vec![503, 200]).await;
        let (ack_tx, mut ack_rx) = mpsc::unbounded_channel();
        spawn_refusal(
            reqwest::Client::new(),
            server,
            "key".into(),
            "agent-a".into(),
            Ok(pkt("any-triple")),
            RefusalReason::Shutdown,
            "agent at max_concurrency".into(),
            ack_tx,
            991_015,
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(
            ack_rx.try_recv().is_err(),
            "no ack while the report is not delivered"
        );
        let ack = tokio::time::timeout(Duration::from_secs(10), ack_rx.recv())
            .await
            .expect("the ack comes after the report")
            .expect("ack channel open");
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        match ack {
            Message::Text(t) => assert!(t.contains("991015"), "ack names the push id: {t}"),
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[test]
    fn only_a_4xx_that_rejects_the_report_is_permanent() {
        use reqwest::StatusCode;
        for code in [400u16, 401, 403, 404, 409, 422] {
            assert!(
                is_permanent_status(StatusCode::from_u16(code).unwrap()),
                "{code}"
            );
        }
        for code in [408u16, 429, 500, 502, 503, 504] {
            assert!(
                !is_permanent_status(StatusCode::from_u16(code).unwrap()),
                "{code}"
            );
        }
    }

    #[test]
    fn default_report_policy_is_bounded() {
        let p = ReportPolicy::default();
        // The waits between the tries: 1, 2, 4, 8, then 15 s each.
        let mut delay = p.backoff_min;
        let mut total = Duration::ZERO;
        for _ in 1..p.max_attempts {
            total += delay;
            delay = (delay * 2).min(p.backoff_max);
        }
        assert_eq!(total, Duration::from_secs(120));
        assert!(server_wait_end(Instant::now(), 300) > Instant::now() + Duration::from_secs(299));
    }

    #[test]
    fn push_registry_starts_one_worker_per_push_id() {
        // CLOACINA-T-1011: a re-push of a packet in work is not a new start.
        let mut r = PushRegistry::default();
        assert_eq!(r.begin(7), PushState::New);
        assert_eq!(r.begin(7), PushState::InWork);
        assert_eq!(r.in_work_ids(), vec![7]);
        r.finish(7);
        assert!(r.in_work_ids().is_empty());
        assert_eq!(r.begin(7), PushState::Done);
    }

    #[test]
    fn push_registry_abandon_lets_the_push_start_again() {
        let mut r = PushRegistry::default();
        assert_eq!(r.begin(3), PushState::New);
        r.abandon(3);
        assert_eq!(r.begin(3), PushState::New);
    }

    #[test]
    fn push_registry_forgets_the_oldest_finished_ids() {
        let mut r = PushRegistry::default();
        for id in 0..(DONE_PUSH_MEMORY as i64 + 1) {
            assert_eq!(r.begin(id), PushState::New);
            r.finish(id);
        }
        assert_eq!(r.done.len(), DONE_PUSH_MEMORY);
        assert_eq!(r.begin(1), PushState::Done);
        // Id 0 was the oldest and is forgotten.
        assert_eq!(r.begin(0), PushState::New);
    }

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
            secret_key_id: None,
            dispatch_id: None,
        }
    }

    fn empty_key_pool() -> Arc<AsyncMutex<AgentKeyPool>> {
        Arc::new(AsyncMutex::new(AgentKeyPool::new()))
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
            &empty_key_pool(),
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
            &empty_key_pool(),
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
    fn without_query_removes_the_ticket_from_the_ws_url() {
        let url = ws_url_for("https://api.example.com:8443", "abc-123", "tk-secret").unwrap();
        let logged = without_query(&url);
        assert_eq!(
            logged,
            "wss://api.example.com:8443/v1/ws/delivery/agent%3Aabc-123"
        );
        assert!(!logged.contains("token"));
        assert!(!logged.contains("tk-secret"));
        // A URL with no query is unchanged.
        assert_eq!(
            without_query("ws://localhost:8080/v1/x"),
            "ws://localhost:8080/v1/x"
        );
    }

    #[test]
    fn ws_url_for_rejects_unsupported_scheme() {
        assert!(ws_url_for("ftp://x", "y", "z").is_err());
    }

    // ── CLOACINA-T-1033: the agent's key leaves the environment ──────────

    /// The tests below change the process environment; they take this lock so
    /// that only one of them does at a time.
    static KEY_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn parse_args(extra: &[&str]) -> Args {
        let mut argv = vec!["cloacina-agent", "--server", "http://server.invalid"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv).expect("arguments parse")
    }

    #[test]
    fn api_key_is_gone_from_the_environment_after_startup_handling() {
        let _guard = KEY_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: the lock keeps the other tests of this group away, and no
        // other test of this crate reads or writes this variable.
        unsafe { std::env::set_var(API_KEY_ENV, "clk_test_environment_form") };

        // The same three steps as `main`.
        let mut args = parse_args(&[]);
        unsafe { remove_api_key_from_env() };
        resolve_api_key(&mut args).expect("the key came from the environment");

        assert_eq!(args.api_key, "clk_test_environment_form");
        assert!(std::env::var_os(API_KEY_ENV).is_none());
        assert!(
            std::env::vars_os().all(|(_, v)| v != "clk_test_environment_form"),
            "no variable holds the key"
        );
    }

    #[test]
    fn api_key_is_read_from_a_file_and_the_file_stays() {
        let _guard = KEY_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::remove_var(API_KEY_ENV) };
        let dir = tempdir();
        let path = dir.join("api-key");
        std::fs::write(&path, "clk_test_file_form\n").unwrap();

        let mut args = parse_args(&["--api-key-file", path.to_str().unwrap()]);
        unsafe { remove_api_key_from_env() };
        resolve_api_key(&mut args).expect("the key came from the file");

        assert_eq!(args.api_key, "clk_test_file_form");
        assert!(path.exists(), "a restart reads the file again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn api_key_must_come_from_exactly_one_place() {
        let _guard = KEY_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::remove_var(API_KEY_ENV) };
        let dir = tempdir();
        let path = dir.join("api-key");

        // Neither.
        let mut args = parse_args(&[]);
        let err = resolve_api_key(&mut args).unwrap_err().to_string();
        assert!(err.contains("no API key"), "{err}");

        // Both.
        std::fs::write(&path, "clk_test_file_form").unwrap();
        let mut args = parse_args(&[
            "--api-key",
            "clk_test_flag_form",
            "--api-key-file",
            path.to_str().unwrap(),
        ]);
        let err = resolve_api_key(&mut args).unwrap_err().to_string();
        assert!(err.contains("one way"), "{err}");

        // An empty file.
        std::fs::write(&path, "\n").unwrap();
        let mut args = parse_args(&["--api-key-file", path.to_str().unwrap()]);
        let err = resolve_api_key(&mut args).unwrap_err().to_string();
        assert!(err.contains("is empty"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `/proc/self/environ` shows the environment the process started with,
    /// so the check needs a process that started with the key: this test runs
    /// itself again as a child with the key set, and the child does the
    /// start-up handling and reads its own `/proc/self/environ`.
    #[cfg(target_os = "linux")]
    #[test]
    fn start_time_environment_no_longer_shows_the_key() {
        const CHILD: &str = "CLOACINA_AGENT_TEST_ENVIRON_CHILD";
        const VALUE: &str = "clk_test_start_time_environment";
        if std::env::var_os(CHILD).is_none() {
            let out = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "tests::start_time_environment_no_longer_shows_the_key",
                    "--test-threads=1",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .env(API_KEY_ENV, VALUE)
                .output()
                .expect("run the child");
            assert!(
                out.status.success(),
                "child failed: {}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            return;
        }
        let holds_key = |bytes: &[u8]| bytes.windows(VALUE.len()).any(|w| w == VALUE.as_bytes());
        let before = std::fs::read("/proc/self/environ").unwrap();
        assert!(
            holds_key(&before),
            "control: the child started with the key"
        );
        // SAFETY: the child runs this one test; nothing else touches the
        // environment.
        unsafe { remove_api_key_from_env() };
        let after = std::fs::read("/proc/self/environ").unwrap();
        assert!(!holds_key(&after), "the key is still in /proc/self/environ");
    }
}
