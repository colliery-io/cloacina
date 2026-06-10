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

    /// LRU cap on cached per-tenant `DefaultRunner` instances (CLOACI-T-0580).
    /// Each cached runner has its own scheduler loop, executor pool, and DB
    /// connection pool; bump for high-cardinality SaaS deployments, drop for
    /// memory-tight ones. Default 256.
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

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Print the OpenAPI 3.1 document for the REST API to stdout and exit.
    /// No database or network access. The committed copy lives at
    /// `docs/static/openapi.json`; `angreal docs spec-check` diffs the two.
    EmitOpenapi,
}

fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[tokio::main]
async fn main() -> Result<()> {
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

    cloacina_server::run(
        cli.home,
        cli.bind,
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
        cloacina_server::CorsConfig {
            allowed_origins: cli.cors_allowed_origins,
            allowed_methods: cli.cors_allowed_methods,
            allowed_headers: cli.cors_allowed_headers,
        },
    )
    .await
}
