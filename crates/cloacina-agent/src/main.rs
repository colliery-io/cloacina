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
//!
//! **Worker processes** (CLOACINA-T-1028): the agent fetches the package, and
//! a child process of its own loads it and runs the one task (see
//! [`worker`]). The agent holds each worker to its memory limit and its time
//! limit (see [`memory`]). A task marked `inline` runs in the agent process.

mod memory;
mod worker;

mod poll;

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
    AgentRegisterResponse, AgentResultRequest, AgentUndecodableRequest, CANCEL_PACKET_KIND,
    CancelPacket, EphemeralKeyEntry, GRAPH_PACKET_KIND, GraphWorkPacket, POLL_JOB_CAPABILITY,
    POLL_PACKET_KIND, PollWorkPacket, RefusalReason, WORK_PACKET_KIND, WorkPacket,
    host_target_triple,
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
    /// Total CPU this agent offers to tasks, a Kubernetes quantity (`4`,
    /// `500m`). Reported at registration; the server places tasks by their
    /// CPU requests against it. Not set: the agent reports none and the
    /// server packs it by `--max-concurrency` only. The agent does not cap
    /// a task's CPU by it.
    #[arg(long, env = "CLOACINA_AGENT_CPU")]
    cpu: Option<String>,
    /// Total memory of this agent, a Kubernetes quantity (`32Gi`, `512Mi`):
    /// the memory limit of its pod. The agent keeps `--reserved-memory` of
    /// it and reports the rest at registration as what it offers to tasks.
    /// A task's memory request is its limit; a task with no request gets
    /// the offered memory divided by `--max-concurrency`. Not set: the agent
    /// reports none, and uses the limit of its own cgroup, if it has one,
    /// for the task limits.
    #[arg(long, env = "CLOACINA_AGENT_MEMORY")]
    memory: Option<String>,
    /// The agent's own share of its memory, a Kubernetes quantity, or `0`.
    /// Taken off `--memory` before the rest is offered to tasks
    /// (CLOACINA-T-1028).
    #[arg(
        long,
        env = "CLOACINA_AGENT_RESERVED_MEMORY",
        default_value = cloacina::fleet::DEFAULT_AGENT_RESERVED_MEMORY
    )]
    reserved_memory: String,
    /// The address-space ceiling of a task's worker process, as a multiple
    /// of the task's memory limit (64 GiB is added). A backstop behind the
    /// memory watchdog, set with `setrlimit` on Linux. `0`: no ceiling.
    #[arg(
        long,
        env = "CLOACINA_AGENT_TASK_ADDRESS_SPACE_FACTOR",
        default_value_t = worker::DEFAULT_ADDRESS_SPACE_FACTOR
    )]
    task_address_space_factor: u64,
    /// The most idle warm workers the agent keeps (CLOACINA-T-1126): worker
    /// processes that keep a package version loaded for the next job of the
    /// same package version, tenant and memory limit. Default: the value of
    /// `--max-concurrency`. `0`: none; each job gets a new worker.
    #[arg(long, env = "CLOACINA_AGENT_WARM_WORKERS")]
    warm_workers: Option<usize>,
    /// Seconds a warm worker may stay idle before the agent ends it.
    #[arg(
        long,
        env = "CLOACINA_AGENT_WARM_WORKER_IDLE_SECONDS",
        default_value_t = worker::DEFAULT_WARM_IDLE_SECONDS
    )]
    warm_worker_idle_seconds: u64,
    /// The most jobs one worker process runs. The agent ends it after the
    /// last one.
    #[arg(
        long,
        env = "CLOACINA_AGENT_WARM_WORKER_MAX_JOBS",
        default_value_t = worker::DEFAULT_WARM_MAX_JOBS
    )]
    warm_worker_max_jobs: u32,
    /// A worker that holds more than this share of its jobs' memory limit
    /// after a job, in percent, is ended and not used again.
    #[arg(
        long,
        env = "CLOACINA_AGENT_WARM_WORKER_MAX_MEMORY_PERCENT",
        default_value_t = worker::DEFAULT_WARM_MAX_MEMORY_PERCENT
    )]
    warm_worker_max_memory_percent: u64,
    /// Override the host target triple the agent advertises (rarely needed —
    /// useful for testing the OQ-6 fail-closed path on a homogeneous host).
    #[arg(long)]
    target_triple_override: Option<String>,
    /// Directory used to cache fetched artifacts by digest. Defaults to
    /// `<TMPDIR>/cloacina-agent-cache`. Reuses cached cdylibs across work
    /// packets to skip the REST fetch on cache hit.
    #[arg(long, env = "CLOACINA_AGENT_CACHE_DIR")]
    cache_dir: Option<std::path::PathBuf>,
    /// The largest size, in bytes, that a package archive may unpack to.
    /// The agent refuses to
    /// load a package or a provider archive that unpacks to more. Default: 524288000 (500 MiB). The limits of 10,000 entries and
    /// of 10 times the archive size stay. CLOACINA-T-1048.
    #[arg(
        long,
        env = "CLOACINA_MAX_UNPACKED_BYTES",
        default_value_t = cloacina::cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES
    )]
    max_unpacked_bytes: u64,
    /// Format of the agent's log lines on stdout: `text`, or `json` (one JSON
    /// object per line, for a log stack). In `json`, a line written while a
    /// task runs has the task's ids under `span` (CLOACINA-T-1024).
    #[arg(long, env = "CLOACINA_AGENT_LOG_FORMAT", default_value_t)]
    log_format: cloacina::task_logs::LogFormat,
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
    // CLOACINA-T-1028: the agent starts itself as `cloacina-agent __worker`
    // to run one task. A worker takes no flags; its job comes on stdin.
    #[cfg(unix)]
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new(worker::WORKER_ARG)) {
        worker::worker_main();
    }

    // Before anything else, while this is the only thread: read the key and
    // take it out of the environment (CLOACINA-T-1033).
    let mut args = Args::parse();
    // SAFETY: no thread has been started yet.
    unsafe { remove_api_key_from_env() };
    resolve_api_key(&mut args)?;
    cloacina::cloacina_workflow_plugin::unpack_limit::set_max_unpacked_bytes(
        args.max_unpacked_bytes,
    )
    .map_err(|e| anyhow::anyhow!(e))?;

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
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    match args.log_format {
        cloacina::task_logs::LogFormat::Text => {
            tracing_subscriber::fmt().with_env_filter(filter).init()
        }
        cloacina::task_logs::LogFormat::Json => tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init(),
    }

    if let Err(e) = run(args).await {
        error!(error = ?e, "agent exited with error");
        std::process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
mod log_format_tests {
    use super::*;
    use cloacina::task_logs::LogFormat;

    fn parse(extra: &[&str]) -> Result<Args, clap::Error> {
        let mut argv = vec!["cloacina-agent", "--server", "http://localhost:8080"];
        argv.extend_from_slice(extra);
        Args::try_parse_from(argv)
    }

    #[test]
    fn the_log_format_is_text_unless_json_is_asked_for() {
        assert_eq!(parse(&[]).unwrap().log_format, LogFormat::Text);
        assert_eq!(
            parse(&["--log-format", "json"]).unwrap().log_format,
            LogFormat::Json
        );
        assert!(parse(&["--log-format", "yaml"]).is_err());
    }

    #[test]
    fn warm_workers_default_to_the_slots_and_zero_turns_them_off() {
        let warm = warm_config(&parse(&["--max-concurrency", "6"]).unwrap());
        assert_eq!(warm.max_idle, 6);
        assert_eq!(warm.idle_timeout, Duration::from_secs(60));
        assert_eq!(warm.max_jobs, 100);
        assert_eq!(warm.max_memory_percent, 50);
        let off = warm_config(&parse(&["--warm-workers", "0"]).unwrap());
        assert_eq!(off.max_idle, 0);
        let set = warm_config(
            &parse(&[
                "--warm-workers",
                "2",
                "--warm-worker-idle-seconds",
                "5",
                "--warm-worker-max-jobs",
                "10",
                "--warm-worker-max-memory-percent",
                "75",
            ])
            .unwrap(),
        );
        assert_eq!(
            (
                set.max_idle,
                set.idle_timeout,
                set.max_jobs,
                set.max_memory_percent
            ),
            (2, Duration::from_secs(5), 10, 75)
        );
    }
}

/// Backoff bounds for the reconnect loop.
const RECONNECT_BACKOFF_MIN: Duration = Duration::from_secs(1);
const RECONNECT_BACKOFF_MAX: Duration = Duration::from_secs(30);

/// The totals the agent reports at registration (CLOACINA-T-1027): CPU in
/// millicores and memory in bytes, from `--cpu` and `--memory`. A flag that
/// is not set, or is empty (an env var set to nothing), reports none. A value
/// that is not a Kubernetes quantity above zero stops the agent at start.
fn agent_totals(cpu: Option<&str>, memory: Option<&str>) -> Result<(Option<u64>, Option<u64>)> {
    fn set(value: Option<&str>) -> Option<&str> {
        value.map(str::trim).filter(|v| !v.is_empty())
    }
    let cpu_millis = set(cpu)
        .map(cloacina::fleet::parse_cpu_millis)
        .transpose()
        .map_err(|e| anyhow::anyhow!("--cpu / CLOACINA_AGENT_CPU: {e}"))?;
    let memory_bytes = set(memory)
        .map(cloacina::fleet::parse_memory_bytes)
        .transpose()
        .map_err(|e| anyhow::anyhow!("--memory / CLOACINA_AGENT_MEMORY: {e}"))?;
    Ok((cpu_millis, memory_bytes))
}

/// The agent's own share of its memory, from `--reserved-memory`
/// (CLOACINA-T-1028): a Kubernetes quantity, or `0` for none.
fn reserved_memory(raw: &str) -> Result<u64> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "0" {
        return Ok(0);
    }
    cloacina::fleet::parse_memory_bytes(raw)
        .map_err(|e| anyhow::anyhow!("--reserved-memory / CLOACINA_AGENT_RESERVED_MEMORY: {e}"))
}

/// What the agent offers to tasks: its memory less its own share. An agent
/// whose share is all of its memory cannot run a task and does not start.
fn offered_memory(total: Option<u64>, reserved: u64) -> Result<Option<u64>> {
    match total {
        None => Ok(None),
        Some(total) if total > reserved => Ok(Some(total - reserved)),
        Some(total) => bail!(
            "the agent's memory ({}) is not above its own share ({}): raise --memory / \
             CLOACINA_AGENT_MEMORY or lower --reserved-memory / CLOACINA_AGENT_RESERVED_MEMORY",
            memory::format_bytes(total),
            memory::format_bytes(reserved)
        ),
    }
}

/// Settle how this agent holds its workers (CLOACINA-T-1028), and say it in
/// the log one time.
fn worker_settings_from(
    args: &Args,
    cache_dir: &std::path::Path,
) -> Result<worker::WorkerSettings> {
    let (_, memory_flag) = agent_totals(args.cpu.as_deref(), args.memory.as_deref())?;
    let reserved = reserved_memory(&args.reserved_memory)?;
    offered_memory(memory_flag, reserved)?;
    let agent_memory = memory::AgentMemory::detect(memory_flag);
    // The task limits come from `--memory`, or from the agent's own cgroup
    // when the flag is not given. A cgroup limit at or under the agent's own
    // share leaves nothing to divide: no default limit then.
    let offered = agent_memory
        .total()
        .and_then(|total| total.checked_sub(reserved))
        .filter(|offered| *offered > 0);
    let default_memory_limit = worker::default_memory_share(offered, args.max_concurrency);
    let child_cgroups = memory::probe_child_cgroup();
    let warm = warm_config(args);
    info!(
        warm_workers = warm.max_idle,
        warm_worker_idle_s = warm.idle_timeout.as_secs(),
        warm_worker_max_jobs = warm.max_jobs,
        warm_worker_max_memory_percent = warm.max_memory_percent,
        "warm workers"
    );
    info!(
        agent_memory = ?agent_memory.total().map(memory::format_bytes),
        reserved = %memory::format_bytes(reserved),
        default_task_memory_limit = ?default_memory_limit.map(memory::format_bytes),
        memory_check_interval_ms = memory::CHECK_INTERVAL.as_millis() as u64,
        address_space_factor = args.task_address_space_factor,
        child_cgroups = %match &child_cgroups {
            Ok(()) => "can be created (not used: the watchdog holds the limits)".to_string(),
            Err(why) => format!("cannot be created ({why}); the watchdog holds the limits"),
        },
        "tasks run in worker processes"
    );
    Ok(worker::WorkerSettings {
        watchdog: memory::Watchdog::new(agent_memory),
        default_memory_limit,
        address_space_factor: args.task_address_space_factor,
        log_format: args.log_format,
        max_unpacked_bytes: args.max_unpacked_bytes,
        warm: worker::WarmPool::new(warm),
        // Agents on one host can share a cache directory: each has its own
        // worker directories.
        ..worker::WorkerSettings::unlimited(cache_dir.join(format!("work-{}", std::process::id())))
    })
}

/// How this agent keeps warm workers (CLOACINA-T-1126), from its flags.
fn warm_config(args: &Args) -> worker::WarmConfig {
    worker::WarmConfig {
        max_idle: args
            .warm_workers
            .unwrap_or(args.max_concurrency.max(1) as usize),
        idle_timeout: Duration::from_secs(args.warm_worker_idle_seconds),
        max_jobs: args.warm_worker_max_jobs,
        max_memory_percent: args.warm_worker_max_memory_percent,
    }
}

async fn run(args: Args) -> Result<()> {
    // CLOACINA-T-1027: a bad total stops the start, before any connection.
    agent_totals(args.cpu.as_deref(), args.memory.as_deref())?;
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
    // CLOACINA-T-1028: how workers are started and held. A bad setting stops
    // the start. Worker directories that an agent with this process id left
    // behind (a container that started again) are removed.
    let settings = worker_settings_from(&args, &cache_dir)?;
    let _ = std::fs::remove_dir_all(&settings.work_root);
    worker::install_settings(settings);
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
    //
    // CLOACINA-T-1050: the agent keeps one id for the life of the process. The
    // first register gets it (the `--agent-id` given, or one the server
    // assigns); every later session registers with it again. The server takes
    // a result only from a registered agent, so a task that is still running
    // when the socket drops must report under an id that stays registered.
    let mut session_agent_id: Option<String> = args.agent_id.clone();
    let mut backoff = RECONNECT_BACKOFF_MIN;
    loop {
        match connect_and_serve(
            &args,
            &mut session_agent_id,
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
#[allow(clippy::too_many_arguments)]
async fn connect_and_serve(
    args: &Args,
    session_agent_id: &mut Option<String>,
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

    // ── 1. Register (each session — the server may have restarted and
    //       forgotten us). The id is the one of the earlier sessions of this
    //       process, if there was one: the server lets the key that holds an
    //       agent id register it again.
    // CLOACINA-T-1021: the agent's pool is registered as the capability
    // `pool:<name>`. With no pool the server counts the agent in `default`.
    let mut capabilities = args.capabilities.clone();
    if let Some(pool) = args.pool.as_deref().filter(|p| !p.is_empty()) {
        capabilities.push(format!("pool:{pool}"));
    }
    // CLOACINA-T-1077: this agent runs poll packets. The server sends a poll
    // only to an agent that says so.
    if !capabilities.iter().any(|c| c == POLL_JOB_CAPABILITY) {
        capabilities.push(POLL_JOB_CAPABILITY.to_string());
    }
    // CLOACINA-T-1027: the agent's total CPU and memory, when it was given
    // them. The server places tasks by their requests against these.
    // CLOACINA-T-1028: the memory it reports is what it offers to tasks, its
    // total less its own share.
    let (cpu_millis, memory_total) = agent_totals(args.cpu.as_deref(), args.memory.as_deref())?;
    let memory_bytes = offered_memory(memory_total, reserved_memory(&args.reserved_memory)?)?;
    let register_resp = match register(
        http,
        server_base,
        &args.api_key,
        session_agent_id,
        args.max_concurrency,
        target_triple,
        &capabilities,
        ephemeral_key_pool,
        cpu_millis,
        memory_bytes,
    )
    .await
    {
        Ok(resp) => resp,
        Err(e) => {
            // 409: another key holds the id now (the row of this agent was
            // removed while it was away, and the id was registered again).
            // An id the server assigned is dropped, and the next try gets a
            // new one. An id the operator chose is kept: the operator decides.
            if args.agent_id.is_none() && is_agent_id_taken(&e) {
                *session_agent_id = None;
            }
            return Err(e).context("agent register failed");
        }
    };
    let agent_id = register_resp.agent_id.clone();
    *session_agent_id = Some(agent_id.clone());
    info!(
        agent_id = %agent_id,
        target_triple = %target_triple,
        max_concurrency = args.max_concurrency,
        cpu_millis = ?cpu_millis,
        memory_bytes = ?memory_bytes,
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

/// Whether a register failed because another key holds the agent id (the
/// server answers 409, CLOACINA-T-1051).
fn is_agent_id_taken(err: &anyhow::Error) -> bool {
    err.downcast_ref::<reqwest::Error>()
        .and_then(|e| e.status())
        .is_some_and(|status| status == reqwest::StatusCode::CONFLICT)
}

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
    cpu_millis: Option<u64>,
    memory_bytes: Option<u64>,
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
        cpu_millis,
        memory_bytes,
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

/// What the agent sends to the server about one packet: the result of its
/// work, or the news that it could not decode the packet.
trait PacketReport: Serialize {
    /// The route that takes it.
    const PATH: &'static str;
    /// What it is about, for the log.
    fn subject(&self) -> String;
}

impl PacketReport for AgentResultRequest {
    const PATH: &'static str = "/v1/agent/result";
    fn subject(&self) -> String {
        format!(
            "task {} ({})",
            self.task_execution_id,
            summarize_outcome(&self.outcome)
        )
    }
}

impl PacketReport for AgentUndecodableRequest {
    const PATH: &'static str = "/v1/agent/undecodable";
    fn subject(&self) -> String {
        format!("undecodable push {}", self.push_id)
    }
}

/// POST one report, one time, with a time limit on the whole request.
async fn post_result<R: PacketReport>(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &R,
    attempt_timeout: Duration,
) -> std::result::Result<(), PostError> {
    let resp = http
        .post(format!("{}{}", server, R::PATH))
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
            // CLOACINA-T-1126: the worker counts since the last heartbeat the
            // server took; put back when this one is not taken.
            let warm = worker::settings().warm.clone();
            let body = AgentHeartbeatRequest {
                protocol_version: AGENT_PROTOCOL_VERSION,
                agent_id: agent_id.clone(),
                in_flight: n,
                available_capacity: max_concurrency.saturating_sub(n),
                in_flight_push_ids: push_registry().in_work_ids(),
                worker_counts: warm.take_counts(),
            };
            let sent = http
                .post(format!("{}/v1/agent/heartbeat", server))
                .bearer_auth(&api_key)
                .json(&body)
                .send()
                .await;
            if !matches!(&sent, Ok(resp) if resp.status().is_success())
                && let Some(counts) = &body.worker_counts
            {
                warm.restore_counts(counts);
            }
            match sent {
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
                // CLOACINA-T-1058: a packet that does not decode is reported
                // by its push id, whatever the load of the agent.
                let packet = match decode_graph_packet(&env).context("decode graph packet") {
                    Ok(packet) => packet,
                    Err(e) => {
                        guard.hand_over();
                        spawn_undecodable(
                            http.clone(),
                            server.to_string(),
                            api_key.to_string(),
                            agent_id.to_string(),
                            format!("{e:#}"),
                            ack_tx.clone(),
                            push_id,
                        );
                        return Ok(());
                    }
                };
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
                    worker::settings(),
                );
                return Ok(());
            }
            // CLOACINA-T-1028: the server gave up on a dispatch. Stop its
            // worker, and ack the cancel. Nothing is reported: the server
            // waits for no result of that dispatch.
            if kind == CANCEL_PACKET_KIND {
                guard.hand_over();
                match decode_payload::<CancelPacket>(&env) {
                    Ok(cancel) => {
                        let stopped = worker::dispatch_registry().cancel(&cancel.dispatch_id);
                        info!(
                            push_id,
                            task_id = %cancel.task_execution_id,
                            dispatch_id = %cancel.dispatch_id,
                            reason = %cancel.reason,
                            stopped,
                            "the server cancelled a dispatch"
                        );
                    }
                    Err(e) => warn!(push_id, error = %format!("{e:#}"), "cannot decode a cancel"),
                }
                finish_and_ack(ack_tx, push_id);
                return Ok(());
            }
            // CLOACINA-T-1077: a poll of a poll trigger.
            if kind == POLL_PACKET_KIND {
                let packet = match poll::decode_poll_packet(&env).context("decode poll packet") {
                    Ok(packet) => packet,
                    Err(e) => {
                        guard.hand_over();
                        spawn_undecodable(
                            http.clone(),
                            server.to_string(),
                            api_key.to_string(),
                            agent_id.to_string(),
                            format!("{e:#}"),
                            ack_tx.clone(),
                            push_id,
                        );
                        return Ok(());
                    }
                };
                guard.hand_over();
                let cur = in_flight.load(Ordering::SeqCst);
                if cur >= max_concurrency {
                    warn!(in_flight = cur, max_concurrency, "saturated; refusing poll");
                    spawn_poll_report(
                        http.clone(),
                        server.to_string(),
                        api_key.to_string(),
                        agent_id.to_string(),
                        packet,
                        None,
                        ack_tx.clone(),
                        push_id,
                    );
                    return Ok(());
                }
                spawn_poll_worker(
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
                    worker::settings(),
                );
                return Ok(());
            }
            if kind != WORK_PACKET_KIND {
                warn!(kind = %kind, push_id, "non-work push received; acking + ignoring");
                guard.hand_over();
                finish_and_ack(ack_tx, push_id);
                return Ok(());
            }

            let packet = match decode_packet(&env).context("decode work packet") {
                Ok(packet) => packet,
                Err(e) => {
                    guard.hand_over();
                    spawn_undecodable(
                        http.clone(),
                        server.to_string(),
                        api_key.to_string(),
                        agent_id.to_string(),
                        format!("{e:#}"),
                        ack_tx.clone(),
                        push_id,
                    );
                    return Ok(());
                }
            };

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

/// The JSON payload of a push frame, as `T`.
fn decode_payload<T: serde::de::DeserializeOwned>(env: &serde_json::Value) -> Result<T> {
    let b64 = env
        .get("payload_b64")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("push frame missing `payload_b64`"))?;
    let bytes = BASE64
        .decode(b64)
        .with_context(|| "decode base64 payload")?;
    serde_json::from_slice::<T>(&bytes).with_context(|| "deserialize the payload JSON")
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
        // CLOACINA-T-1028: this returns when the task's worker process has
        // exited (or, for an inline task, when its future ended). The slot
        // is counted free after that, below.
        let end = process_work_packet(
            &packet,
            &target_triple,
            &http,
            &server,
            &api_key,
            cache_dir.as_path(),
            &key_pool,
            &worker::settings(),
        )
        .await;
        let duration_ms = start.elapsed().as_millis() as u64;
        match end {
            worker::PacketEnd::Report(outcome) => {
                let req = AgentResultRequest {
                    protocol_version: AGENT_PROTOCOL_VERSION,
                    agent_id: agent_id.clone(),
                    task_execution_id: packet.task_execution_id.clone(),
                    attempt: packet.attempt,
                    dispatch_id: packet.dispatch_id.clone(),
                    duration_ms,
                    outcome,
                };
                // CLOACINA-T-1015: the ack waits for the report (see
                // `report_outcome`).
                let server_gives_up_at = server_wait_end(start, packet.timeout_seconds);
                report_outcome(&http, &server, &api_key, &req, server_gives_up_at).await;
            }
            // The server gave up on this dispatch and waits for no report.
            worker::PacketEnd::Cancelled => debug!(
                task_id = %packet.task_execution_id,
                dispatch_id = ?packet.dispatch_id,
                "dispatch cancelled by the server; no report"
            ),
        }
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

/// Import a PYTHON graph package (CLOACI-T-0841) from the files the agent
/// fetched (CLOACINA-T-1125): set the provider search path, unpack into
/// `staging`, import. One import per digest in a process; the graph
/// executors it registers are global.
async fn load_python_graph(
    packet: &GraphWorkPacket,
    package: &worker::PreparedPackage,
    staging: &std::path::Path,
) -> std::result::Result<(), cloacina::fleet::AgentOutcome> {
    let digest = package.digest.clone();
    let refused = |reason: RefusalReason, message: String| cloacina::fleet::AgentOutcome::Refused {
        reason,
        message,
    };
    {
        let mut imported = imported_py_graph_digests().lock().await;
        if !imported.contains(&digest) {
            let archive_path = package.source_archive.as_ref().ok_or_else(|| {
                refused(
                    RefusalReason::ArtifactFetchFailed,
                    "the package has no fetched source archive".to_string(),
                )
            })?;
            let archive = std::fs::read(archive_path).map_err(|e| {
                refused(
                    RefusalReason::ArtifactFetchFailed,
                    format!("read the cached python graph source {archive_path:?}: {e}"),
                )
            })?;
            use_providers(&package.providers)
                .map_err(|e| refused(RefusalReason::RuntimeLoadFailed, e))?;
            let staging = staging.to_path_buf();
            if let Err(e) = std::fs::create_dir_all(&staging) {
                return Err(refused(
                    RefusalReason::RuntimeLoadFailed,
                    format!("create python staging dir {:?}: {}", staging, e),
                ));
            }
            let vendored_set = match &package.vendored_set {
                Some(path) => Some(std::fs::read(path).map_err(|e| {
                    refused(
                        RefusalReason::ArtifactFetchFailed,
                        format!("read the cached python vendored dependencies {path:?}: {e}"),
                    )
                })?),
                None => None,
            };
            let Some(py) = cloacina::python_runtime::python_runtime() else {
                return Err(refused(
                    RefusalReason::RuntimeLoadFailed,
                    "no Python runtime installed in agent".to_string(),
                ));
            };
            let tenant = packet
                .tenant_id
                .clone()
                .unwrap_or_else(|| "public".to_string());
            // Throwaway Runtime: we only need the import's registration side
            // effects (graph executors land in the process-global registry).
            let runtime_for_load = Arc::new(cloacina::Runtime::empty());
            let result = tokio::task::spawn_blocking(move || {
                py.load_workflow_package(
                    &archive,
                    vendored_set.as_deref(),
                    &staging,
                    &tenant,
                    &runtime_for_load,
                )
                .map(|_| ())
            })
            .await;
            match result {
                Ok(Ok(())) => {
                    imported.insert(digest.clone());
                    debug!(digest = %digest, "python graph package imported");
                }
                Ok(Err(e)) => {
                    return Err(refused(
                        RefusalReason::RuntimeLoadFailed,
                        format!("python graph load: {}", e),
                    ));
                }
                Err(e) => {
                    return Err(refused(
                        RefusalReason::RuntimeLoadFailed,
                        format!("python graph load panicked: {}", e),
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Run one PYTHON graph firing on an imported package (CLOACI-T-0841): look
/// up the graph executors of the packet's reactor and run each with the
/// packet's cache rebuilt into the in-process `InputCache` wire shape.
async fn fire_python_graph(packet: &GraphWorkPacket) -> cloacina::fleet::AgentOutcome {
    let digest = &packet.artifact.digest;
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

/// CLOACI-T-0722: spawn the work of one whole-graph firing, report the
/// outcome under the packet's `firing_id` (the server's rendezvous key), and
/// ack. CLOACINA-T-1125: the firing runs in a worker process, held to the
/// limits of a task; the slot is counted free after the worker has exited.
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
    settings: Arc<worker::WorkerSettings>,
) {
    in_flight.fetch_add(1, Ordering::SeqCst);
    tokio::spawn(async move {
        let start = Instant::now();
        let end = process_graph_packet(
            &packet,
            &http,
            &server,
            &api_key,
            cache_dir.as_path(),
            &settings,
        )
        .await;
        match end {
            worker::PacketEnd::Report(outcome) => {
                let req = graph_report(
                    &agent_id,
                    &packet,
                    start.elapsed().as_millis() as u64,
                    outcome,
                );
                let server_gives_up_at = server_wait_end(start, packet.timeout_seconds);
                report_outcome(&http, &server, &api_key, &req, server_gives_up_at).await;
            }
            // The server gave up on this firing and waits for no report.
            worker::PacketEnd::Cancelled => debug!(
                firing_id = %packet.firing_id,
                graph = %packet.graph_name,
                "graph firing cancelled by the server; no report"
            ),
        }
        finish_and_ack(&ack_tx, push_id);
        in_flight.fetch_sub(1, Ordering::SeqCst);
    });
}

/// The report of a graph firing: keyed by the firing id, attempt 1, no
/// dispatch id, as before CLOACINA-T-1125.
fn graph_report(
    agent_id: &str,
    packet: &GraphWorkPacket,
    duration_ms: u64,
    outcome: cloacina::fleet::AgentOutcome,
) -> AgentResultRequest {
    AgentResultRequest {
        protocol_version: AGENT_PROTOCOL_VERSION,
        agent_id: agent_id.to_string(),
        task_execution_id: packet.firing_id.clone(),
        attempt: 1,
        dispatch_id: None,
        duration_ms,
        outcome,
    }
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

/// CLOACINA-T-1077: report a poll's outcome under its poll id and ack the
/// push. `None`: a refusal of a full agent; the poll did not run.
#[allow(clippy::too_many_arguments)]
fn spawn_poll_report(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    packet: PollWorkPacket,
    outcome: Option<(cloacina::fleet::AgentOutcome, u64)>,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
) {
    tokio::spawn(async move {
        let (outcome, duration_ms) = outcome.unwrap_or_else(|| {
            (
                cloacina::fleet::AgentOutcome::Refused {
                    reason: RefusalReason::Shutdown,
                    message: "agent at max_concurrency".to_string(),
                },
                0,
            )
        });
        let req = AgentResultRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id,
            task_execution_id: packet.poll_id.clone(),
            attempt: 1,
            dispatch_id: None,
            duration_ms,
            outcome,
        };
        let server_gives_up_at = server_wait_end(Instant::now(), packet.timeout_seconds);
        report_outcome(&http, &server, &api_key, &req, server_gives_up_at).await;
        finish_and_ack(&ack_tx, push_id);
    });
}

/// CLOACINA-T-1077: run one poll of a poll trigger and report fire or skip.
/// A poll counts as one packet in flight while it runs. CLOACINA-T-1143: the
/// poll runs in a worker process; the slot is counted free after the worker
/// has exited or gone back to the warm pool.
#[allow(clippy::too_many_arguments)]
fn spawn_poll_worker(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    target_triple: String,
    packet: PollWorkPacket,
    in_flight: Arc<AtomicU32>,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
    cache_dir: Arc<std::path::PathBuf>,
    settings: Arc<worker::WorkerSettings>,
) {
    in_flight.fetch_add(1, Ordering::SeqCst);
    tokio::spawn(async move {
        let start = Instant::now();
        let end = process_poll_packet(
            &packet,
            &target_triple,
            &http,
            &server,
            &api_key,
            cache_dir.as_path(),
            &settings,
        )
        .await;
        let outcome = match end {
            Some(worker::PacketEnd::Report(outcome)) => Some(outcome),
            // The server sends no cancel for a poll; an id it named anyway
            // ends the poll with no report, as for a task.
            Some(worker::PacketEnd::Cancelled) => {
                debug!(poll_id = %packet.poll_id, "poll cancelled; no report");
                finish_and_ack(&ack_tx, push_id);
                in_flight.fetch_sub(1, Ordering::SeqCst);
                return;
            }
            None => None,
        };
        match outcome {
            Some(outcome) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                debug!(
                    trigger = %packet.trigger_name,
                    poll_id = %packet.poll_id,
                    outcome = %summarize_outcome(&outcome),
                    duration_ms,
                    "poll ran"
                );
                spawn_poll_report(
                    http,
                    server,
                    api_key,
                    agent_id,
                    packet,
                    Some((outcome, duration_ms)),
                    ack_tx,
                    push_id,
                );
            }
            None => {
                info!(
                    trigger = %packet.trigger_name,
                    poll_id = %packet.poll_id,
                    "poll packet arrived after the server stopped waiting; not run"
                );
                finish_and_ack(&ack_tx, push_id);
            }
        }
        in_flight.fetch_sub(1, Ordering::SeqCst);
    });
}

/// Process-wide cache of the Rust poll triggers this process opened, keyed
/// by build digest and trigger name (CLOACINA-T-1077). The library is opened
/// once per build in a process. CLOACINA-T-1143: the process is the poll's
/// worker, so a trigger keeps its state between the polls of one warm worker.
type PollTriggerCache = tokio::sync::Mutex<
    std::collections::HashMap<(String, String), Arc<dyn cloacina::cloacina_workflow::Trigger>>,
>;

fn loaded_poll_triggers() -> &'static PollTriggerCache {
    static CACHE: std::sync::OnceLock<PollTriggerCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()))
}

/// CLOACINA-T-1077, CLOACINA-T-1143: one poll on the agent. The agent
/// checks the packet and fetches the package files, one time per digest; a
/// worker process loads the package and runs the poll ([`run_poll_job`]).
/// `None`: the packet came after the server stopped waiting, and is not run.
async fn process_poll_packet(
    packet: &PollWorkPacket,
    agent_triple: &str,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
    settings: &worker::WorkerSettings,
) -> Option<worker::PacketEnd> {
    use worker::PacketEnd::Report;
    if !poll::still_wanted(packet) {
        return None;
    }
    if let Some(refusal) = poll::wrong_arch(packet, agent_triple) {
        return Some(Report(refusal));
    }
    let package = match prepare_package(
        &packet.artifact,
        packet.language.as_deref(),
        http,
        server,
        api_key,
        cache_dir,
    )
    .await
    {
        Ok(package) => package,
        Err(outcome) => return Some(Report(outcome)),
    };
    let job = worker::PollJob {
        packet: packet.clone(),
        // A worker gets a directory of its own (set at its start).
        work_dir: cache_dir.join(format!("pysrc-{}", package.digest)),
        package,
        log_format: settings.log_format.to_string(),
        max_unpacked_bytes: settings.max_unpacked_bytes,
        #[cfg(test)]
        test_action: None,
    };
    Some(worker::run_poll_in_worker(job, settings).await)
}

/// Load the package of a poll and run the poll (CLOACINA-T-1143), in the
/// poll's worker process.
///
/// `started` is called when the trigger is loaded. `time_limit` is given
/// when the poll runs in the agent process: its future is dropped at the
/// limit. A worker is given none: the agent holds the clock and kills it.
pub(crate) async fn run_poll_job(
    job: worker::PollJob,
    time_limit: Option<Duration>,
    started: impl FnOnce(),
) -> cloacina::fleet::AgentOutcome {
    let worker::PollJob {
        packet,
        package,
        work_dir,
        #[cfg(test)]
        test_action,
        ..
    } = job;

    #[cfg(test)]
    if let Some(action) = test_action {
        started();
        return worker::tests::run_test_action(action, None).await;
    }

    let trigger = match load_poll_trigger(&packet, &package, &work_dir).await {
        Ok(trigger) => trigger,
        Err(outcome) => return outcome,
    };
    started();
    poll::run_poll(trigger, time_limit, packet.timeout_seconds).await
}

/// The trigger of a poll. A Python package is imported into the per-build
/// runtime of this process (a poll and a task of one build share it when
/// they run in one process); a Rust build's library is opened once per build
/// and trigger in this process.
async fn load_poll_trigger(
    packet: &PollWorkPacket,
    package: &worker::PreparedPackage,
    work_dir: &std::path::Path,
) -> std::result::Result<Arc<dyn cloacina::cloacina_workflow::Trigger>, cloacina::fleet::AgentOutcome>
{
    let digest = package.digest.clone();
    if packet.language.as_deref() == Some("python") {
        let load = poll::load_packet_for(packet);
        let runtime = {
            let mut cache = loaded_runtimes().lock().await;
            if let Some(rt) = cache.get(&digest) {
                rt.clone()
            } else {
                let rt = Arc::new(cloacina::Runtime::new());
                load_python_package(&load, package, work_dir, &rt).await?;
                cache.insert(digest.clone(), rt.clone());
                rt
            }
        };
        return runtime
            .get_trigger(&packet.trigger_name)
            .ok_or_else(|| poll::no_such_trigger(packet));
    }

    let key = (digest, packet.trigger_name.clone());
    let mut cache = loaded_poll_triggers().lock().await;
    if let Some(trigger) = cache.get(&key) {
        return Ok(trigger.clone());
    }
    let path = package
        .library
        .clone()
        .ok_or_else(|| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::ArtifactFetchFailed,
            message: "the poll's package has no fetched library".to_string(),
        })?;
    let name = packet.trigger_name.clone();
    let opened = tokio::task::spawn_blocking(move || {
        cloacina::registry::loader::ffi_trigger::FfiTriggerImpl::open(
            &path,
            &name,
            Duration::from_secs(60),
            false,
        )
    })
    .await;
    let trigger: Arc<dyn cloacina::cloacina_workflow::Trigger> = match opened {
        Ok(Ok(trigger)) => Arc::new(trigger),
        Ok(Err(e)) => {
            return Err(cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: e,
            });
        }
        Err(e) => {
            return Err(cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: format!("open the library: {e}"),
            });
        }
    };
    cache.insert(key, trigger.clone());
    Ok(trigger)
}

/// One whole-graph firing on the agent (CLOACI-T-0722). The agent fetches
/// the package files, one time per digest; CLOACINA-T-1125: a worker process
/// loads the package and runs the firing ([`run_graph_job`]), or the agent
/// process for a packet marked `inline`.
async fn process_graph_packet(
    packet: &GraphWorkPacket,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
    settings: &worker::WorkerSettings,
) -> worker::PacketEnd {
    // A firing whose cancel came before its packet is not started.
    if worker::dispatch_registry().is_cancelled(&packet.firing_id) {
        return worker::PacketEnd::Cancelled;
    }
    let package = match prepare_package(
        &packet.artifact,
        packet.language.as_deref(),
        http,
        server,
        api_key,
        cache_dir,
    )
    .await
    {
        Ok(package) => package,
        Err(outcome) => return worker::PacketEnd::Report(outcome),
    };
    let job = worker::GraphJob {
        packet: packet.clone(),
        // An inline firing keeps its Python package unpacked for the life of
        // the agent. A worker gets a directory of its own.
        work_dir: cache_dir.join(format!("pysrc-{}", package.digest)),
        package,
        log_format: settings.log_format.to_string(),
        max_unpacked_bytes: settings.max_unpacked_bytes,
        #[cfg(test)]
        test_action: None,
    };
    worker::run_graph_here_or_in_worker(job, settings).await
}

/// Load the package of a graph firing and run it (CLOACINA-T-1125): in a
/// worker process, or in the agent process for a packet marked `inline`.
///
/// `started` is called when the package is loaded. `time_limit` is given
/// for an inline firing: its future is dropped at the limit. A worker is
/// given none: the agent holds the clock and kills the process.
pub(crate) async fn run_graph_job(
    job: worker::GraphJob,
    time_limit: Option<Duration>,
    started: impl FnOnce(),
) -> cloacina::fleet::AgentOutcome {
    let worker::GraphJob {
        packet,
        package,
        work_dir,
        #[cfg(test)]
        test_action,
        ..
    } = job;

    #[cfg(test)]
    if let Some(action) = test_action {
        started();
        return worker::tests::run_test_action(action, None).await;
    }

    // CLOACI-T-0841: a Python-packaged CG runs from SOURCE: import the
    // module (registering its graph executors), then run them.
    let is_python = packet.language.as_deref() == Some("python");
    let plugin = if is_python {
        if let Err(out) = load_python_graph(&packet, &package, &work_dir).await {
            return out;
        }
        None
    } else {
        match load_rust_graph(&package).await {
            Ok(plugin) => Some(plugin),
            Err(out) => return out,
        }
    };
    started();
    let fire = async {
        match plugin {
            Some(plugin) => fire_rust_graph(plugin, &packet).await,
            None => fire_python_graph(&packet).await,
        }
    };
    match time_limit {
        Some(limit) => tokio::time::timeout(limit, fire).await.unwrap_or_else(|_| {
            cloacina::fleet::AgentOutcome::Failure {
                message: format!(
                    "graph firing exceeded timeout of {}s",
                    packet.timeout_seconds
                ),
                classification: cloacina::fleet::FailureClassification::Timeout,
            }
        }),
        None => fire.await,
    }
}

/// Open the library of a Rust graph package, one time per digest in a
/// process (CLOACI-T-0722).
async fn load_rust_graph(
    package: &worker::PreparedPackage,
) -> std::result::Result<
    Arc<cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin>,
    cloacina::fleet::AgentOutcome,
> {
    let mut cache = loaded_graphs().lock().await;
    if let Some(p) = cache.get(&package.digest) {
        return Ok(p.clone());
    }
    let artifact_path =
        package
            .library
            .as_ref()
            .ok_or_else(|| cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::ArtifactFetchFailed,
                message: "the graph package has no fetched library".to_string(),
            })?;
    let bytes =
        std::fs::read(artifact_path).map_err(|e| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::ArtifactFetchFailed,
            message: format!("read cached graph artifact: {}", e),
        })?;
    let loaded = cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin::load(&bytes)
        .map(Arc::new)
        .map_err(|e| cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::RuntimeLoadFailed,
            message: format!("load graph plugin: {}", e),
        })?;
    cache.insert(package.digest.clone(), loaded.clone());
    Ok(loaded)
}

/// Call the plugin's `execute_graph` with the packet's pre-converted FFI
/// cache, and map the result (CLOACI-T-0722).
async fn fire_rust_graph(
    plugin: Arc<cloacina::computation_graph::packaging_bridge::LoadedGraphPlugin>,
    packet: &GraphWorkPacket,
) -> cloacina::fleet::AgentOutcome {
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
    packet: WorkPacket,
    reason: RefusalReason,
    message: String,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
) {
    tokio::spawn(async move {
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
        finish_and_ack(&ack_tx, push_id);
    });
}

/// Tell the server that this agent cannot decode a push, then ack it
/// (CLOACINA-T-1058).
///
/// The agent could not read the packet, so it has no task id for a report:
/// it names the push id, and the server finds the task in its own outbox
/// row and fails the dispatch. Before, a full agent acked such a push with
/// no report, and an agent with a free slot neither acked nor reported: the
/// task waited out its time limit either way.
///
/// The message is retried as a report is. The packet's time limit is not
/// known, so the tries go on for [`ReportPolicy::min_window`] at least. The
/// push is acked after that, whatever the result, for the reasons
/// [`report_outcome`] gives. A server from before the route answers 404: the
/// push is acked and the task waits out its time limit, as before.
fn spawn_undecodable(
    http: reqwest::Client,
    server: String,
    api_key: String,
    agent_id: String,
    error: String,
    ack_tx: mpsc::UnboundedSender<Message>,
    push_id: i64,
) {
    error!(
        push_id,
        error = %error,
        "cannot decode a pushed packet (a server of another version?); telling the server"
    );
    tokio::spawn(async move {
        let req = AgentUndecodableRequest {
            protocol_version: AGENT_PROTOCOL_VERSION,
            agent_id,
            push_id,
            error,
            agent_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        };
        report_outcome(&http, &server, &api_key, &req, Instant::now()).await;
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
async fn report_outcome<R: PacketReport>(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &R,
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

async fn report_with_policy<R: PacketReport>(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    req: &R,
    server_gives_up_at: Instant,
    policy: &ReportPolicy,
) -> ReportDelivery {
    let subject = req.subject();
    let mut delay = policy.backoff_min;
    let mut stop_at: Option<Instant> = None;
    let mut attempt = 1u32;
    loop {
        let error = match post_result(http, server, api_key, req, policy.attempt_timeout).await {
            Ok(()) => {
                debug!(report = %subject, attempt, "result reported");
                return ReportDelivery::Delivered;
            }
            Err(PostError::Permanent(e)) => {
                error!(
                    report = %subject,
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
                report = %subject,
                attempts = attempt,
                error = %error,
                "result not delivered and the retries are used up; \
                 the server runs the task again after its result wait"
            );
            return ReportDelivery::GaveUp;
        }
        warn!(
            report = %subject,
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

/// Fetch the dependencies the compiler vendored for a Python package and this
/// agent's platform, from `GET /v1/agent/python-vendor/{digest}`
/// (CLOACINA-T-1073). `Ok(None)`: the server holds no set for this platform
/// (204), or it is a server from before the route (404). The loader then
/// refuses a package that pins dependencies and ships none.
async fn fetch_vendored_set(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    digest: &str,
) -> Result<Option<Vec<u8>>> {
    let platform = cloacina::cloacina_workflow_plugin::python_env::host_platform();
    // A platform is `<arch>-<os>`: letters, digits, `_` and `-`.
    let url = format!(
        "{}/v1/agent/python-vendor/{}?platform={}",
        server, digest, platform
    );
    let resp = http.get(&url).bearer_auth(api_key).send().await?;
    match resp.status() {
        reqwest::StatusCode::NO_CONTENT | reqwest::StatusCode::NOT_FOUND => Ok(None),
        _ => Ok(Some(resp.error_for_status()?.bytes().await?.to_vec())),
    }
}

/// What the agent fetched for each digest (CLOACINA-T-1028). The agent does
/// the requests to the server; a worker process reads the files.
fn prepared_packages()
-> &'static tokio::sync::Mutex<std::collections::HashMap<String, worker::PreparedPackage>> {
    static CACHE: std::sync::OnceLock<
        tokio::sync::Mutex<std::collections::HashMap<String, worker::PreparedPackage>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Write `bytes` to `path` through a temporary file and a rename, so a reader
/// never sees half a file.
fn write_cache_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = std::path::PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Fetch what a task or a graph firing of this package needs into the cache
/// directory, one time per digest: the library (Rust), or the source archive
/// and the vendored dependencies (Python), and the bundled providers.
async fn prepare_package(
    artifact: &cloacina::fleet::ArtifactRef,
    language: Option<&str>,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
) -> std::result::Result<worker::PreparedPackage, cloacina::fleet::AgentOutcome> {
    use cloacina::fleet::AgentOutcome::Refused;
    let digest = artifact.digest.clone();
    let is_python = language == Some("python");
    // The lock is held across the one-time fetch, so two packets of one
    // package do not fetch it twice.
    let mut prepared = prepared_packages().lock().await;
    if let Some(package) = prepared.get(&digest) {
        return Ok(package.clone());
    }
    let fetch_failed = |message: String| Refused {
        reason: RefusalReason::ArtifactFetchFailed,
        message,
    };
    let mut package = worker::PreparedPackage {
        digest: digest.clone(),
        library: None,
        source_archive: None,
        vendored_set: None,
        providers: worker::Providers::None,
    };
    if is_python {
        let archive_path = cache_dir.join(format!("pysrc-{digest}.cloacina"));
        if !archive_path.exists() {
            let archive = fetch_source_archive(http, server, api_key, &digest)
                .await
                .map_err(|e| fetch_failed(format!("python source fetch failed: {e}")))?;
            write_cache_file(&archive_path, &archive)
                .map_err(|e| fetch_failed(format!("write {archive_path:?}: {e}")))?;
        }
        package.source_archive = Some(archive_path);
        // CLOACINA-T-1073: the dependencies the compiler vendored for this
        // platform, when the server holds a set.
        let vendored = fetch_vendored_set(http, server, api_key, &digest)
            .await
            .map_err(|e| fetch_failed(format!("python vendored dependencies fetch failed: {e}")))?;
        if let Some(set) = vendored {
            let set_path = cache_dir.join(format!("pyvendor-{digest}.bin"));
            write_cache_file(&set_path, &set)
                .map_err(|e| fetch_failed(format!("write {set_path:?}: {e}")))?;
            package.vendored_set = Some(set_path);
        }
    } else {
        let library = fetch_and_cache_artifact(http, server, api_key, artifact, cache_dir)
            .await
            .map_err(|e| fetch_failed(format!("artifact fetch failed: {e}")))?;
        package.library = Some(library);
    }
    // CLOACI-T-0838: the bundled constructor providers. A Python package
    // needs the answer before its import; a Rust package only when it
    // declares constructor nodes, which the load finds out.
    package.providers = match fetch_agent_providers(http, server, api_key, &digest, cache_dir).await
    {
        Ok(Some(root)) => worker::Providers::Staged(root),
        Ok(None) => worker::Providers::None,
        Err(e) if is_python => {
            return Err(Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: e,
            });
        }
        Err(e) => worker::Providers::Failed(e),
    };
    prepared.insert(digest, package.clone());
    Ok(package)
}

/// Point the provider search path of this process at the package's bundled
/// providers. No providers clears it (hermeticity: a package must not
/// resolve against the bundle of a package loaded before).
fn use_providers(providers: &worker::Providers) -> std::result::Result<usize, String> {
    match providers {
        worker::Providers::Staged(root) => {
            cloacina::registry::loader::set_provider_search_path(root);
            Ok(1)
        }
        worker::Providers::None => {
            cloacina::registry::loader::clear_provider_search_path();
            Ok(0)
        }
        worker::Providers::Failed(e) => Err(e.clone()),
    }
}

/// Load a Rust cdylib package into `runtime` from the library file the agent
/// fetched: register its tasks via the fidius-backed `TaskRegistrar`.
async fn load_rust_cdylib(
    packet: &WorkPacket,
    package: &worker::PreparedPackage,
    runtime: &Arc<cloacina::Runtime>,
) -> std::result::Result<(), cloacina::fleet::AgentOutcome> {
    let artifact_path =
        package
            .library
            .as_ref()
            .ok_or_else(|| cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::ArtifactFetchFailed,
                message: "the package has no fetched library".to_string(),
            })?;
    let cdylib_bytes =
        std::fs::read(artifact_path).map_err(|e| cloacina::fleet::AgentOutcome::Refused {
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
        let staged = use_providers(&package.providers).map_err(|e| {
            cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::RuntimeLoadFailed,
                message: e,
            }
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
/// reconciler reading `package_providers`), and unpack them under
/// `cache_dir`. The staged tree is left in place — loaded constructor nodes
/// re-read their wasm components from it for the process lifetime. The
/// search path of the process is not changed: the process that loads the
/// package sets it ([`use_providers`]). `None`: the server has no providers
/// for the package.
async fn fetch_agent_providers(
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    digest: &str,
    cache_dir: &std::path::Path,
) -> std::result::Result<Option<std::path::PathBuf>, String> {
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
        return Ok(None);
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
    Ok(Some(providers_root))
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
    package: &worker::PreparedPackage,
    staging: &std::path::Path,
    runtime: &Arc<cloacina::Runtime>,
) -> std::result::Result<(), cloacina::fleet::AgentOutcome> {
    let digest = package.digest.as_str();
    let read_failed = |what: &str, path: &std::path::Path, e: std::io::Error| {
        cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::ArtifactFetchFailed,
            message: format!("read the cached {what} {path:?}: {e}"),
        }
    };
    let archive_path =
        package
            .source_archive
            .as_ref()
            .ok_or_else(|| cloacina::fleet::AgentOutcome::Refused {
                reason: RefusalReason::ArtifactFetchFailed,
                message: "the package has no fetched source archive".to_string(),
            })?;
    let archive = std::fs::read(archive_path)
        .map_err(|e| read_failed("python source archive", archive_path, e))?;

    // CLOACI-T-0838: stage the package's bundled constructor providers BEFORE
    // the module import, so `cloaca.constructor(...)` calls resolve against
    // them during load (the agent twin of the reconciler's Python branch).
    // Zero providers clears the search path — an undeclared ref then fails the
    // import with a clear "no such provider" instead of leaking to a
    // previously-loaded package's bundle.
    use_providers(&package.providers).map_err(|e| cloacina::fleet::AgentOutcome::Refused {
        reason: RefusalReason::RuntimeLoadFailed,
        message: e,
    })?;

    // CLOACINA-T-1073: the dependencies the compiler vendored for this
    // platform, when the server holds a set.
    let vendored_set = match &package.vendored_set {
        Some(path) => Some(
            std::fs::read(path)
                .map_err(|e| read_failed("python vendored dependencies", path, e))?,
        ),
        None => None,
    };

    let staging = staging.to_path_buf();
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
        py.load_workflow_package(
            &archive,
            vendored_set.as_deref(),
            &staging,
            &tenant,
            &runtime_for_load,
        )
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
///
/// CLOACINA-T-1028: steps 1, 2 and the secret unwrap run here, in the agent.
/// Steps 3 to 6 are [`run_job`], which runs in a worker process of the task,
/// or in the agent process for a task marked `inline`.
#[allow(clippy::too_many_arguments)]
async fn process_work_packet(
    packet: &WorkPacket,
    agent_triple: &str,
    http: &reqwest::Client,
    server: &str,
    api_key: &str,
    cache_dir: &std::path::Path,
    key_pool: &Arc<AsyncMutex<AgentKeyPool>>,
    settings: &worker::WorkerSettings,
) -> worker::PacketEnd {
    use worker::PacketEnd::Report;
    let digest = packet.artifact.digest.clone();
    let is_python = packet.language.as_deref() == Some("python");

    // A packet that arrives after the server cancelled its dispatch is not
    // started.
    if let Some(id) = packet.dispatch_id.as_deref()
        && worker::dispatch_registry().is_cancelled(id)
    {
        return worker::PacketEnd::Cancelled;
    }

    // OQ-6 fail-closed for Rust cdylibs (built for a specific target). Python is
    // interpreted, so the artifact triple is irrelevant — skip the gate.
    if !is_python && packet.artifact.build_target_triple != agent_triple {
        return Report(cloacina::fleet::AgentOutcome::Refused {
            reason: RefusalReason::TargetTripleMismatch,
            message: format!(
                "agent target triple `{}` does not match artifact build target `{}`",
                agent_triple, packet.artifact.build_target_triple
            ),
        });
    }

    // 2. Fetch the package files, one time per digest.
    let package = match prepare_package(
        &packet.artifact,
        packet.language.as_deref(),
        http,
        server,
        api_key,
        cache_dir,
    )
    .await
    {
        Ok(package) => package,
        Err(outcome) => return Report(outcome),
    };

    // 5b. CLOACI-T-0861 / D-5 — unwrap the HPKE-wrapped secrets with the ONE-TIME
    //     pooled private key the server named in `secret_key_id`. We `take` that
    //     key out of the pool (one-time use → discarded after this) and unwrap
    //     here, in the agent: the key pool never leaves this process. The plain
    //     values go to the task's worker over its stdin pipe (CLOACINA-T-1028),
    //     where an in-memory resolver serves `ctx.secret(name)`. They are never
    //     persisted or logged (NFR-001/NFR-003). The AAD is keyed by
    //     `task_execution_id`, matching the wrap.
    let mut secrets = std::collections::HashMap::new();
    if !packet.wrapped_secrets.is_empty() {
        let Some(key_id) = packet.secret_key_id.as_deref() else {
            // Secrets present but no key id — a malformed dispatch. Fail closed
            // rather than run with missing secrets.
            return Report(cloacina::fleet::AgentOutcome::Failure {
                message: "work packet carries wrapped_secrets but no secret_key_id".to_string(),
                classification: cloacina::fleet::FailureClassification::Validation,
            });
        };
        // One-time discard: remove the key from the pool. An unknown key_id (e.g.
        // the agent restarted after the server consumed the key) is a clean
        // failure — never an execution with missing secrets.
        let private_key = { key_pool.lock().await.take(key_id) };
        let Some(private_key) = private_key else {
            return Report(cloacina::fleet::AgentOutcome::Failure {
                message: format!(
                    "unknown or already-used one-time secret key '{}' — cannot unwrap secrets \
                     (agent may have restarted since the key was advertised)",
                    key_id
                ),
                classification: cloacina::fleet::FailureClassification::Validation,
            });
        };
        match InMemorySecretResolver::from_wrapped(
            &private_key,
            &packet.wrapped_secrets,
            &packet.task_execution_id,
        ) {
            Ok(resolver) => secrets = resolver.into_secrets(),
            Err(e) => {
                // Names only — `FleetSecretError` never renders a secret value.
                return Report(cloacina::fleet::AgentOutcome::Failure {
                    message: format!("unwrap secrets: {}", e),
                    classification: cloacina::fleet::FailureClassification::Validation,
                });
            }
        }
    }

    // The job: the packet without its wrapped secrets, and the plain values.
    let mut job_packet = packet.clone();
    job_packet.wrapped_secrets = Vec::new();
    job_packet.secret_key_id = None;
    let job = worker::WorkerJob {
        packet: job_packet,
        secrets,
        package,
        // An inline task keeps its package unpacked for the life of the
        // agent. A worker gets a directory of its own (set at its start).
        work_dir: cache_dir.join(format!("pysrc-{digest}")),
        log_format: settings.log_format.to_string(),
        max_unpacked_bytes: settings.max_unpacked_bytes,
        #[cfg(test)]
        test_action: None,
    };
    worker::run_here_or_in_worker(job, settings).await
}

/// Load the job's package and run its one task (CLOACINA-T-1028): in a
/// worker process, or in the agent process for a task marked `inline`.
///
/// `started` is called when the package is loaded and the task is about to
/// run. `time_limit` is given for an inline task: its future is dropped at
/// the limit (a blocking body runs on). A worker is given none: the agent
/// holds the clock and kills the process.
pub(crate) async fn run_job(
    job: worker::WorkerJob,
    time_limit: Option<Duration>,
    started: impl FnOnce(),
) -> cloacina::fleet::AgentOutcome {
    let worker::WorkerJob {
        packet,
        secrets,
        package,
        work_dir,
        #[cfg(test)]
        test_action,
        ..
    } = job;
    let packet = &packet;
    // CLOACINA-T-1081: the packet carries only the secrets the task's package
    // declares; any other name the task asks for is refused.
    let secrets = InMemorySecretResolver::new(secrets)
        .refusing_unsent()
        .into_arc();

    #[cfg(test)]
    if let Some(action) = test_action {
        started();
        return worker::tests::run_test_action(action, Some(secrets)).await;
    }

    // 3. Resolve (or build) the per-package runtime, cached by digest.
    //
    // In the agent process (inline tasks) the load is done once per package
    // and the populated Runtime is reused: re-importing a Python module in a
    // live interpreter is a no-op (so the `@task` decorators wouldn't re-run
    // and the new Runtime would be empty), and for Rust it avoids a redundant
    // dlopen per task. A worker process loads its one package one time.
    let digest = package.digest.clone();
    let is_python = packet.language.as_deref() == Some("python");
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
                load_python_package(packet, &package, &work_dir, &rt).await
            } else {
                load_rust_cdylib(packet, &package, &rt).await
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

    // 5b. The task's secrets, which the agent unwrapped: an in-memory resolver
    //     attached to the Context, so the task body reads them via
    //     `ctx.secret(name)`. The plain values live only in this resolver,
    //     only for the run. CLOACINA-T-1081: a name the packet does not carry
    //     is refused.
    context.set_secret_resolver(secrets);

    // 6. Execute inside the task's span (CLOACINA-T-1024): a log line written
    //    while the task runs carries the tenant, workflow, run, task, attempt
    //    and dispatch. An inline task runs under the packet's time limit; a
    //    worker is held to it by the agent.
    use tracing::Instrument as _;
    let task_span = cloacina::task_logs::task_span(
        packet.tenant_id.as_deref().unwrap_or("public"),
        &packet.task_name,
        &packet.workflow_execution_id,
        packet.attempt,
        packet.dispatch_id.as_deref(),
    );
    started();
    let run = async {
        match time_limit {
            Some(limit) => tokio::time::timeout(limit, task.execute(context)).await,
            None => Ok(task.execute(context).await),
        }
    };
    match run.instrument(task_span).await {
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
        python_platforms: None,
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

    /// CLOACINA-T-1051: the server answers 409 to a register for an agent id
    /// that another key holds. The agent tells that from any other failure,
    /// so it can drop an id the server assigned and get a new one.
    #[tokio::test]
    async fn a_refused_agent_id_is_told_from_other_register_failures() {
        async fn try_register(status: u16) -> anyhow::Error {
            let (server, _hits) = result_server(vec![status]).await;
            register(
                &reqwest::Client::new(),
                &server,
                "key",
                &Some("agent-a".to_string()),
                1,
                "aarch64-apple-darwin",
                &[],
                Vec::new(),
                None,
                None,
            )
            .await
            .expect_err("the register fails")
        }
        assert!(is_agent_id_taken(&try_register(409).await));
        assert!(!is_agent_id_taken(&try_register(403).await));
        assert!(!is_agent_id_taken(&try_register(500).await));
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
            pkt("any-triple"),
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

    // ── CLOACINA-T-1058: a packet the agent cannot decode ────────────────

    /// A payload that is a work packet of another shape: it has the task's
    /// ids, and lacks fields this agent needs (`artifact`,
    /// `timeout_seconds`), as a packet from a newer or older server can.
    fn other_shape_payload() -> String {
        let payload = serde_json::json!({
            "protocol_version": AGENT_PROTOCOL_VERSION,
            "task_execution_id": "00000000-0000-0000-0000-000000001058",
            "firing_id": "00000000-0000-0000-0000-000000001058",
            "attempt": 1,
            "dispatch_id": "d1",
        });
        BASE64.encode(serde_json::to_vec(&payload).unwrap())
    }

    /// A local HTTP server that takes one request, answers 200, and hands
    /// the request (head and body) to the test.
    async fn capture_server() -> (String, tokio::sync::oneshot::Receiver<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
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
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
                 content-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.shutdown().await;
            let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
        });
        (format!("http://{addr}"), rx)
    }

    /// Give the agent one push frame it cannot decode. Returns the request
    /// the server got (if one came) and whether the agent acked the push.
    async fn deliver_undecodable(
        push_id: i64,
        kind: &str,
        payload_b64: Option<&str>,
        in_flight: u32,
        max_concurrency: u32,
    ) -> (Option<String>, bool) {
        let (server, request) = capture_server().await;
        let (ack_tx, mut ack_rx) = mpsc::unbounded_channel();
        let mut frame = serde_json::json!({"type": "push", "id": push_id, "kind": kind});
        if let Some(payload) = payload_b64 {
            frame["payload_b64"] = serde_json::json!(payload);
        }
        let _ = handle_text_frame(
            frame.to_string(),
            &reqwest::Client::new(),
            &server,
            "key",
            "agent-a",
            "any-triple",
            &Arc::new(AtomicU32::new(in_flight)),
            max_concurrency,
            &ack_tx,
            &Arc::new(tempdir()),
            &empty_key_pool(),
        )
        .await;
        // The message and the ack come from a spawned task.
        let request = tokio::time::timeout(Duration::from_secs(2), request)
            .await
            .ok()
            .and_then(|r| r.ok());
        let acked = match tokio::time::timeout(Duration::from_secs(2), ack_rx.recv()).await {
            Ok(Some(Message::Text(t))) => t.contains(&push_id.to_string()),
            _ => false,
        };
        (request, acked)
    }

    /// The server is told of the push by its id, on the undecodable route,
    /// and the push is acked after that.
    fn assert_told_and_acked(push_id: i64, got: (Option<String>, bool)) {
        let (request, acked) = got;
        let request = request.unwrap_or_else(|| {
            panic!("the server must learn of the packet: acked={acked}, reports=0")
        });
        assert!(
            request.starts_with("POST /v1/agent/undecodable "),
            "the route: {request}"
        );
        assert!(
            request.contains(&format!("\"push_id\":{push_id}")),
            "the message names the push id: {request}"
        );
        assert!(request.contains("\"agent_id\":\"agent-a\""), "{request}");
        assert!(acked, "the push is acked after the message");
        assert!(
            !push_registry().in_work_ids().contains(&push_id),
            "the push is not in work any more"
        );
    }

    #[tokio::test]
    async fn a_full_agent_reports_a_packet_it_cannot_decode() {
        let id = 1_058_001;
        let payload = other_shape_payload();
        let got = deliver_undecodable(id, WORK_PACKET_KIND, Some(&payload), 1, 1).await;
        assert_told_and_acked(id, got);
    }

    #[tokio::test]
    async fn an_agent_with_a_free_slot_reports_a_packet_it_cannot_decode() {
        let id = 1_058_002;
        let payload = other_shape_payload();
        let got = deliver_undecodable(id, WORK_PACKET_KIND, Some(&payload), 0, 1).await;
        assert_told_and_acked(id, got);
    }

    #[tokio::test]
    async fn an_agent_reports_a_graph_packet_it_cannot_decode() {
        let id = 1_058_003;
        let payload = other_shape_payload();
        let got = deliver_undecodable(id, GRAPH_PACKET_KIND, Some(&payload), 0, 1).await;
        assert_told_and_acked(id, got);
    }

    /// The payload carries no task id at all: not base64, not JSON, missing.
    #[tokio::test]
    async fn an_agent_reports_a_packet_with_no_readable_payload() {
        let not_json = BASE64.encode(b"\x00\xffnot json");
        let payloads = [Some("%%% not base64 %%%"), Some(not_json.as_str()), None];
        for (n, payload) in payloads.into_iter().enumerate() {
            let id = 1_058_010 + n as i64;
            let got = deliver_undecodable(id, WORK_PACKET_KIND, payload, 0, 1).await;
            assert_told_and_acked(id, got);
        }
    }

    /// A server from before the route answers 404: the agent sends the
    /// message one time and acks the push.
    #[tokio::test]
    async fn an_agent_acks_an_undecodable_packet_when_the_server_has_no_route() {
        let (server, hits) = result_server(vec![404]).await;
        let (ack_tx, mut ack_rx) = mpsc::unbounded_channel();
        spawn_undecodable(
            reqwest::Client::new(),
            server,
            "key".into(),
            "agent-a".into(),
            "deserialize WorkPacket JSON".into(),
            ack_tx,
            1_058_020,
        );
        let ack = tokio::time::timeout(Duration::from_secs(5), ack_rx.recv())
            .await
            .expect("the ack comes")
            .expect("ack channel open");
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        match ack {
            Message::Text(t) => assert!(t.contains("1058020"), "ack names the push id: {t}"),
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    /// CLOACINA-T-1125: a graph firing runs in a worker process, and its
    /// report reaches the server in the shape it had before: keyed by the
    /// firing id, attempt 1, no dispatch id, the outputs under `outputs`.
    /// The push is acked after the report, and the slot is free again.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_graph_firing_reports_to_the_server_as_before() {
        let settings = Arc::new(worker::tests::test_settings(
            "graph-report",
            memory::Watchdog::with_interval(
                memory::AgentMemory::Unknown,
                Duration::from_millis(50),
            ),
        ));
        std::fs::create_dir_all(&settings.work_root).unwrap();
        let package = worker::tests::python_graph_package(&settings.work_root, "t1125_report");
        let pid_file = settings.work_root.join("report.pid");
        let mut packet =
            worker::tests::python_graph_job("graph_report", "t1125_report", &package, &pid_file)
                .packet;
        packet.artifact.digest = package.digest.clone();
        // The package files are fetched already: no request but the report.
        prepared_packages()
            .lock()
            .await
            .insert(package.digest.clone(), package.clone());

        let (server, request) = capture_server().await;
        let (ack_tx, mut ack_rx) = mpsc::unbounded_channel();
        let in_flight = Arc::new(AtomicU32::new(0));
        spawn_graph_worker(
            reqwest::Client::new(),
            server,
            "key".into(),
            "agent-a".into(),
            packet,
            in_flight.clone(),
            ack_tx,
            1_125_001,
            Arc::new(settings.work_root.clone()),
            settings.clone(),
        );
        let request = tokio::time::timeout(Duration::from_secs(60), request)
            .await
            .expect("the report comes")
            .expect("one request");
        assert!(
            request.starts_with("POST /v1/agent/result "),
            "the route: {request}"
        );
        let body: serde_json::Value =
            serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body["task_execution_id"], "firing-graph_report");
        assert_eq!(body["attempt"], 1);
        assert_eq!(body["agent_id"], "agent-a");
        assert!(body.get("dispatch_id").is_none(), "{body}");
        assert_eq!(
            body["outcome"],
            serde_json::to_value(AgentOutcome::Success {
                context: serde_json::json!({ "outputs": [] }),
            })
            .unwrap()
        );
        let ack = tokio::time::timeout(Duration::from_secs(10), ack_rx.recv())
            .await
            .expect("the ack comes")
            .expect("ack channel open");
        assert!(matches!(ack, Message::Text(t) if t.contains("1125001")));
        let pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_ne!(pid, std::process::id(), "a worker process ran the firing");
        for _ in 0..100 {
            if in_flight.load(Ordering::SeqCst) == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(in_flight.load(Ordering::SeqCst), 0, "the slot is free");
    }

    #[test]
    fn graph_report_has_the_shape_of_before() {
        let packet = worker::tests::graph_job("shape", None).packet;
        let req = graph_report(
            "agent-a",
            &packet,
            7,
            AgentOutcome::Success {
                context: serde_json::json!({ "outputs": [1] }),
            },
        );
        assert_eq!(req.task_execution_id, packet.firing_id);
        assert_eq!(req.attempt, 1);
        assert_eq!(req.dispatch_id, None);
        assert_eq!(req.duration_ms, 7);
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
            pool: None,
            cpu_millis: None,
            memory_bytes: None,
            inline: false,
        }
    }

    // CLOACINA-T-1027: the totals an agent reports come from `--cpu` and
    // `--memory` (or their env vars), in the Kubernetes forms.
    #[test]
    fn agent_totals_are_read_from_the_flags() {
        assert_eq!(
            agent_totals(Some("4"), Some("32Gi")).unwrap(),
            (Some(4000), Some(32 << 30))
        );
        assert_eq!(
            agent_totals(Some("500m"), Some("512Mi")).unwrap(),
            (Some(500), Some(512 << 20))
        );
        // The managed Deployment and a hand start can give one and not the other.
        assert_eq!(
            agent_totals(None, Some("1Gi")).unwrap(),
            (None, Some(1 << 30))
        );
    }

    #[test]
    fn an_agent_with_no_totals_reports_none() {
        assert_eq!(agent_totals(None, None).unwrap(), (None, None));
        // An env var set to nothing is "not set".
        assert_eq!(agent_totals(Some(""), Some("  ")).unwrap(), (None, None));
    }

    #[test]
    fn a_bad_total_stops_the_agent_and_names_the_flag() {
        let err = agent_totals(Some("lots"), None).unwrap_err().to_string();
        assert!(err.contains("--cpu"), "{err}");
        let err = agent_totals(None, Some("32GB")).unwrap_err().to_string();
        assert!(err.contains("--memory"), "{err}");
        let err = agent_totals(Some("0"), None).unwrap_err().to_string();
        assert!(err.contains("zero"), "{err}");
    }

    #[test]
    fn the_total_flags_parse_from_the_command_line() {
        let args = Args::try_parse_from([
            "cloacina-agent",
            "--server",
            "http://localhost:8080",
            "--api-key",
            "k",
            "--pool",
            "large",
            "--cpu",
            "8",
            "--memory",
            "64Gi",
        ])
        .expect("the flags parse");
        assert_eq!(args.cpu.as_deref(), Some("8"));
        assert_eq!(args.memory.as_deref(), Some("64Gi"));
        assert_eq!(
            agent_totals(args.cpu.as_deref(), args.memory.as_deref()).unwrap(),
            (Some(8000), Some(64u64 << 30))
        );
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
            &worker::settings(),
        )
        .await;
        assert!(matches!(
            outcome,
            worker::PacketEnd::Report(AgentOutcome::Refused {
                reason: RefusalReason::TargetTripleMismatch,
                ..
            })
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
            &worker::settings(),
        )
        .await;
        assert!(matches!(
            outcome,
            worker::PacketEnd::Report(AgentOutcome::Refused {
                reason: RefusalReason::ArtifactFetchFailed,
                ..
            })
        ));
    }

    // ── CLOACINA-T-1028: worker processes, limits, cancel ────────────────

    /// What the agent offers to tasks is its memory less its own share.
    #[test]
    fn the_agent_offers_its_memory_less_its_own_share() {
        let reserved = reserved_memory("128Mi").unwrap();
        assert_eq!(reserved, 128 << 20);
        assert_eq!(
            offered_memory(Some(32 << 30), reserved).unwrap(),
            Some((32u64 << 30) - (128 << 20))
        );
        assert_eq!(offered_memory(None, reserved).unwrap(), None);
        // `0` keeps nothing back.
        assert_eq!(reserved_memory("0").unwrap(), 0);
        assert_eq!(offered_memory(Some(1 << 30), 0).unwrap(), Some(1 << 30));
        // The default of the flag is the shared default.
        assert_eq!(
            reserved_memory(&parse_args(&[]).reserved_memory).unwrap(),
            cloacina::fleet::DEFAULT_AGENT_RESERVED_MEMORY_BYTES
        );
    }

    #[test]
    fn an_agent_with_no_memory_left_for_tasks_does_not_start() {
        let err = offered_memory(Some(100 << 20), 128 << 20)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--reserved-memory"), "{err}");
        assert!(err.contains("100Mi") && err.contains("128Mi"), "{err}");
        let err = reserved_memory("lots").unwrap_err().to_string();
        assert!(err.contains("--reserved-memory"), "{err}");
    }

    /// The agent's settings give a task with no request its share: the
    /// offered memory over the slots.
    #[test]
    fn worker_settings_give_a_task_with_no_request_its_share() {
        let args = parse_args(&[
            "--memory",
            "1Gi",
            "--reserved-memory",
            "224Mi",
            "--max-concurrency",
            "4",
            "--task-address-space-factor",
            "0",
        ]);
        let settings = worker_settings_from(&args, &tempdir()).unwrap();
        // 1Gi less 224Mi is 800Mi; over 4 slots, 200Mi. On a host whose own
        // cgroup has a lower limit than 1Gi the share is smaller.
        let share = settings.default_memory_limit.expect("a share");
        assert!(share <= 200 << 20, "{share}");
        assert_eq!(settings.address_space_factor, 0);
        assert_eq!(settings.args, vec![worker::WORKER_ARG.to_string()]);

        let bad = parse_args(&["--memory", "64Mi"]);
        assert!(worker_settings_from(&bad, &tempdir()).is_err());
    }

    /// A cancel push stops nothing that does not run, is remembered, and is
    /// acked; no report goes to the server for it.
    #[tokio::test]
    async fn a_cancel_push_is_acked_and_remembered() {
        let payload = serde_json::to_vec(&CancelPacket {
            protocol_version: AGENT_PROTOCOL_VERSION,
            task_execution_id: "00000000-0000-0000-0000-000000001028".into(),
            dispatch_id: "dispatch-of-the-cancel-push-test".into(),
            reason: "the result wait ended".into(),
        })
        .unwrap();
        let frame = serde_json::json!({
            "type": "push",
            "id": 1_028_001,
            "kind": CANCEL_PACKET_KIND,
            "payload_b64": BASE64.encode(payload),
        });
        let (ack_tx, mut ack_rx) = mpsc::unbounded_channel();
        handle_text_frame(
            frame.to_string(),
            &reqwest::Client::new(),
            "http://127.0.0.1:1",
            "key",
            "agent-a",
            "any-triple",
            &Arc::new(AtomicU32::new(0)),
            1,
            &ack_tx,
            &Arc::new(tempdir()),
            &empty_key_pool(),
        )
        .await
        .unwrap();
        match ack_rx.try_recv() {
            Ok(Message::Text(t)) => assert!(t.contains("1028001"), "{t}"),
            other => panic!("the cancel is acked at once: {other:?}"),
        }
        assert!(worker::dispatch_registry().is_cancelled("dispatch-of-the-cancel-push-test"));

        // The packet of that dispatch arrives after: it is not started.
        let mut late = pkt(&host_target_triple());
        late.dispatch_id = Some("dispatch-of-the-cancel-push-test".into());
        let end = process_work_packet(
            &late,
            &host_target_triple(),
            &reqwest::Client::new(),
            "http://127.0.0.1:1",
            "key",
            tempdir().as_path(),
            &empty_key_pool(),
            &worker::settings(),
        )
        .await;
        assert!(matches!(end, worker::PacketEnd::Cancelled), "{end:?}");
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

    // ── CLOACINA-T-1048: the unpack limit is a setting ───────────────────

    #[test]
    fn max_unpacked_bytes_flag_defaults_to_500_mib_and_is_read() {
        assert_eq!(parse_args(&[]).max_unpacked_bytes, 524_288_000);
        assert_eq!(
            parse_args(&["--max-unpacked-bytes", "1073741824"]).max_unpacked_bytes,
            1_073_741_824
        );
    }
}
