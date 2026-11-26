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

//! cloacina-server — HTTP API for Cloacina. Extracted from cloacinactl's serve
//! command in T-0510 (CLOACI-I-0098).

use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use uuid::Uuid;

/// cloacina-server — HTTP API for Cloacina, backed by Postgres.
#[derive(Parser)]
#[command(name = "cloacina-server")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Cloacina home directory
    #[arg(long, default_value_os_t = default_home())]
    home: PathBuf,

    /// Address to bind the HTTP server to
    #[arg(long, default_value = "127.0.0.1:8080")]
    bind: SocketAddr,

    /// Address of a separate listener for `/metrics` (for example
    /// `0.0.0.0:9090`). When set, `/metrics` is served only there and the main
    /// listener answers 404 for it. When not set, `/metrics` stays on `--bind`
    /// with no authentication. CLOACINA-T-1025.
    #[arg(long, env = "CLOACINA_METRICS_BIND")]
    metrics_bind: Option<SocketAddr>,

    /// Database URL (overrides DATABASE_URL env var). Required to run the
    /// server; not needed for `emit-openapi`.
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Bootstrap API key (used instead of auto-generating on first startup).
    #[arg(long, env = "CLOACINA_BOOTSTRAP_KEY")]
    bootstrap_key: Option<String>,

    /// Require package signatures for workflow uploads.
    #[arg(long, env = "CLOACINA_REQUIRE_SIGNATURES")]
    require_signatures: bool,

    /// Trusted organization id (UUID) used to verify package signatures.
    /// Required when `--require-signatures` is set; otherwise startup fails fast.
    /// CLOACI-I-0103 / T-0567.
    #[arg(long, env = "CLOACINA_VERIFICATION_ORG_ID")]
    verification_org_id: Option<Uuid>,

    /// Interval (seconds) between reconciler passes that sync the in-runner
    /// workflow registry with the DB. Default matches the cloacina runtime
    /// default; override upward for quiet prod, downward for fast e2e.
    #[arg(long)]
    reconcile_interval_s: Option<u64>,

    /// Warning limit on per-tenant `DefaultRunner` instances (CLOACI-T-0580,
    /// CLOACINA-T-1031). Every tenant has a runner from server start until
    /// the tenant is deleted; none is stopped at this limit. Above it the
    /// server logs a warning. Default 256.
    #[arg(long, env = "CLOACINA_TENANT_RUNNER_CACHE_SIZE", default_value_t = 256)]
    tenant_runner_cache_size: usize,

    /// Max seconds to wait for in-flight workflows to drain during tenant
    /// teardown (CLOACI-T-0581). Past this, the runner is hard-evicted —
    /// any task that ignored cooperative cancellation will error on its
    /// next DB write once the schema is dropped in step 4. Default 30s.
    #[arg(
        long,
        env = "CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S",
        default_value_t = 30
    )]
    tenant_deletion_drain_timeout_s: u64,

    /// Number of daily-rotated log files to retain on disk. `0` disables
    /// pruning entirely (unbounded — explicit opt-out). Default 14 days.
    /// CLOACI-I-0109 / T-0592.
    #[arg(long, default_value_t = 14)]
    log_retention_days: u64,

    /// Days to keep a finished workflow run (status Completed, Failed or
    /// Cancelled) in the database, counted from when it finished. A
    /// background job then deletes the run with its task executions, task
    /// metadata, contexts, execution events, recovery events and schedule
    /// executions. Runs that are not finished are never deleted. `0` (the
    /// default) keeps run history for ever: an operator must set a period to
    /// turn the deletes on. Not related to
    /// `--log-retention-days`, which is about log files. CLOACINA-T-1023.
    #[arg(long, env = "CLOACINA_RUN_HISTORY_RETENTION_DAYS", default_value_t = 0)]
    run_history_retention_days: u64,

    /// Seconds to keep a `delivery_outbox` row after it was acked. `0` turns
    /// this off (keep for ever). Default 3600. CLOACINA-T-1023.
    #[arg(
        long,
        env = "CLOACINA_OUTBOX_ACKED_RETENTION_S",
        default_value_t = 3600
    )]
    outbox_acked_retention_s: u64,

    /// Seconds between passes of the retention job. Default 300.
    /// CLOACINA-T-1023.
    #[arg(long, env = "CLOACINA_RETENTION_INTERVAL_S", default_value_t = 300)]
    retention_interval_s: u64,

    /// Executor every task is dispatched to (CLOACI-T-0640). `default` (the
    /// in-process thread executor) unless set to another registered key —
    /// notably `fleet` to send all work to the execution-agent fleet. The key
    /// must match a registered executor or the server fails fast at startup.
    /// Operators normally set this via `[server].default_executor` in
    /// `config.toml` (forwarded by `cloacinactl server start`); this flag/env
    /// is the override for direct `cloacina-server` runs.
    #[arg(long, env = "CLOACINA_DEFAULT_EXECUTOR", default_value = "default")]
    default_executor: String,

    /// Heartbeat interval (seconds) the server advertises to fleet agents and
    /// uses as its liveness sweep cadence. Lower = faster dead-agent detection
    /// + in-flight reclaim, at the cost of more heartbeat traffic. Default 15.
    /// CLOACI-T-0639.
    #[arg(
        long,
        env = "CLOACINA_AGENT_HEARTBEAT_INTERVAL_S",
        default_value_t = cloacina::fleet::DEFAULT_HEARTBEAT_INTERVAL_SECONDS
    )]
    agent_heartbeat_interval_s: u32,

    /// Consecutive missed heartbeats before the server marks a fleet agent dead
    /// and reclaims its in-flight work. Effective dead-after = interval ×
    /// misses (default 15s × 3 = 45s). Lower = more aggressive failover, higher
    /// chance of evicting a briefly-slow agent. Default 3. CLOACI-T-0639.
    #[arg(long, env = "CLOACINA_AGENT_LIVENESS_MISSES", default_value_t = 3)]
    agent_liveness_misses: u32,

    /// Time limit (seconds) of a fleet task that sets none of its own. The
    /// agent stops waiting for the task after this long and the attempt fails
    /// as a timeout. A task sets its own limit with
    /// `#[task(timeout_seconds = N)]` or `@task(timeout_seconds=N)`.
    /// Default 300. CLOACINA-T-1014.
    #[arg(
        long,
        env = "CLOACINA_FLEET_TASK_TIMEOUT_S",
        default_value_t = cloacina_server::fleet_executor::FleetTaskTimeLimits::DEFAULT_SECONDS
    )]
    fleet_task_timeout_s: u64,

    /// Longest time limit (seconds) a fleet task can get. A task that asks
    /// for more gets this value, with a warning in the log. Must not be below
    /// `--fleet-task-timeout-s`. Default 86400 (24 hours). CLOACINA-T-1014.
    #[arg(
        long,
        env = "CLOACINA_FLEET_TASK_TIMEOUT_MAX_S",
        default_value_t = cloacina_server::fleet_executor::FleetTaskTimeLimits::DEFAULT_MAX_SECONDS
    )]
    fleet_task_timeout_max_s: u64,

    /// CORS allowed origins, comma-separated (e.g.
    /// `https://ops.example.com,https://ui.example.com`, or `*` for any).
    /// CORS is DISABLED unless this is set — browser consumers require an
    /// explicit opt-in. CLOACI-T-0643 / REQ-009.
    #[arg(
        long,
        env = "CLOACINA_CORS_ALLOWED_ORIGINS",
        value_delimiter = ',',
        num_args = 0..
    )]
    cors_allowed_origins: Vec<String>,

    /// CORS allowed methods, comma-separated. Defaults to
    /// GET,POST,DELETE,OPTIONS when CORS is enabled.
    #[arg(
        long,
        env = "CLOACINA_CORS_ALLOWED_METHODS",
        value_delimiter = ',',
        num_args = 0..
    )]
    cors_allowed_methods: Vec<String>,

    /// CORS allowed request headers, comma-separated. Defaults to
    /// authorization,content-type when CORS is enabled.
    #[arg(
        long,
        env = "CLOACINA_CORS_ALLOWED_HEADERS",
        value_delimiter = ',',
        num_args = 0..
    )]
    cors_allowed_headers: Vec<String>,

    /// Alert webhook (CLOACINA-T-1024): a URL that gets a JSON POST when a
    /// run ends. Not set = off. The URL can hold a token, so it is never
    /// logged. To sign each request, set `CLOACINA_ALERT_WEBHOOK_SECRET`
    /// (environment only): the header `X-Cloacina-Signature` then holds
    /// `t=<unix seconds>,v1=<hex HMAC-SHA256 of "<t>.<body>">`.
    #[arg(long, env = "CLOACINA_ALERT_WEBHOOK_URL", hide_env_values = true)]
    alert_webhook_url: Option<String>,

    /// Which run outcomes send an alert, comma-separated: `failed`,
    /// `completed`, `cancelled`, or `all`. Default `failed`.
    #[arg(
        long,
        env = "CLOACINA_ALERT_WEBHOOK_EVENTS",
        value_delimiter = ',',
        num_args = 0..
    )]
    alert_webhook_events: Vec<String>,

    /// Seconds allowed for one alert webhook request. Default 10.
    #[arg(long, env = "CLOACINA_ALERT_WEBHOOK_TIMEOUT_S", default_value_t = 10)]
    alert_webhook_timeout_s: u64,

    /// The most sends of one alert before it is dropped. The wait between
    /// sends doubles from 30 seconds to 15 minutes. Default 8.
    #[arg(long, env = "CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS", default_value_t = 8)]
    alert_webhook_max_attempts: u32,

    /// The public URL of this server (for example
    /// `https://cloacina.example.com`). Used for the absolute links in an
    /// alert. Not set = the alert holds paths only.
    #[arg(long, env = "CLOACINA_PUBLIC_BASE_URL")]
    public_base_url: Option<String>,

    /// Seconds to wait at start for the database to accept a connection.
    /// The server tries again with a growing delay (0.5 s, doubling, at most
    /// 5 s) and logs each wait. After this time it exits with an error.
    /// `0` means one try and no wait. The migrations run after the first
    /// connection. CLOACINA-T-1035.
    #[arg(long, env = "CLOACINA_DATABASE_WAIT_TIMEOUT_S", default_value_t = 60)]
    database_wait_timeout_s: u64,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Print the OpenAPI 3.1 document for the REST API to stdout and exit.
    /// No database or network access. The committed copy lives at
    /// `docs/public/openapi.json`; `angreal docs spec-check` diffs the two.
    EmitOpenapi,
}

/// Vendored in place of the `dirs` crate (T-0942) — home-dir lookup was the
/// only thing we used it for.
fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let var = "USERPROFILE";
    #[cfg(not(windows))]
    let var = "HOME";
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn default_home() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[tokio::main]
async fn main() -> Result<()> {
    // CLOACINA-T-1030: before any TLS client is built (kube, OIDC, webhook).
    cloacina_server::install_default_crypto_provider();

    let cli = Cli::parse();

    if let Some(Command::EmitOpenapi) = cli.command {
        // Spec emission needs no DB, no logging setup, no runtime.
        println!("{}", cloacina_server::openapi::openapi_json());
        return Ok(());
    }

    let database_url = cli.database_url.ok_or_else(|| {
        anyhow::anyhow!(
            "a database URL is required to run the server: \
             pass --database-url or set DATABASE_URL"
        )
    })?;

    // CLOACINA-T-1024: the alert webhook. An empty URL means off, like no URL.
    if let Some(url) = cli
        .alert_webhook_url
        .as_deref()
        .filter(|u| !u.trim().is_empty())
    {
        let config = cloacina_server::alerts::AlertWebhookConfig::new(
            url,
            &cli.alert_webhook_events,
            std::env::var("CLOACINA_ALERT_WEBHOOK_SECRET").ok(),
            std::time::Duration::from_secs(cli.alert_webhook_timeout_s),
            cli.alert_webhook_max_attempts,
            cli.public_base_url.clone(),
        )
        .map_err(anyhow::Error::msg)?;
        cloacina_server::alerts::install(config);
    }

    // Fail at start on limits the server cannot run with (CLOACINA-T-1014).
    let fleet_task_time_limits =
        cloacina_server::fleet_executor::FleetTaskTimeLimits::from_seconds(
            cli.fleet_task_timeout_s,
            cli.fleet_task_timeout_max_s,
        )
        .map_err(|e| anyhow::anyhow!(e))?;

    cloacina_server::run(
        cli.home,
        cli.bind,
        cli.metrics_bind,
        database_url,
        cli.verbose,
        cli.bootstrap_key,
        cli.require_signatures,
        cli.verification_org_id,
        cli.reconcile_interval_s.map(std::time::Duration::from_secs),
        cli.tenant_runner_cache_size,
        std::time::Duration::from_secs(cli.tenant_deletion_drain_timeout_s),
        cli.log_retention_days,
        cli.default_executor,
        cli.agent_heartbeat_interval_s,
        cli.agent_liveness_misses,
        fleet_task_time_limits,
        cloacina_server::CorsConfig {
            allowed_origins: cli.cors_allowed_origins,
            allowed_methods: cli.cors_allowed_methods,
            allowed_headers: cli.cors_allowed_headers,
        },
        cloacina_server::retention::RetentionConfig::from_flags(
            cli.run_history_retention_days,
            cli.outbox_acked_retention_s,
            cli.retention_interval_s,
        ),
        std::time::Duration::from_secs(cli.database_wait_timeout_s),
    )
    .await
}
