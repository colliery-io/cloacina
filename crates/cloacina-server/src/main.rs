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

    /// Require package signatures for workflow uploads. The signer must be a
    /// trusted key of the tenant, or of `--verification-org-id` if one is set.
    #[arg(long, env = "CLOACINA_REQUIRE_SIGNATURES")]
    require_signatures: bool,

    /// Server-wide trusted organization id (UUID). With `--require-signatures`,
    /// an upload is checked against the trusted keys of its tenant first, then
    /// against the keys of this organization: a key in it is trusted for every
    /// tenant. Optional: with none, each tenant trusts only its own keys
    /// (CLOACINA-A-0018).
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

    /// Format of the server's log lines on stderr: `text`, or `json` (one
    /// JSON object per line, for a log stack). The log file under `--home`
    /// is JSON in both. In `json`, a line written while a task runs in the
    /// server has the task's ids under `span` (CLOACINA-T-1024).
    #[arg(long, env = "CLOACINA_LOG_FORMAT", default_value_t)]
    log_format: cloacina::task_logs::LogFormat,

    /// Template of the URL of a task's logs in your log stack
    /// (CLOACINA-T-1024). The placeholders `{tenant}`, `{execution_id}`,
    /// `{task}` and `{attempt}` are filled in for each task, URL-encoded.
    /// The task API then gives `log_url`, and the UI shows a link. Not set =
    /// no link.
    #[arg(long, env = "CLOACINA_TASK_LOG_URL_TEMPLATE")]
    task_log_url_template: Option<String>,

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

    /// Seconds to keep a `delivery_outbox` row after it was acked. A
    /// background job then deletes the row. `0` (the default) keeps acked
    /// rows for ever: an operator must set a period to turn the deletes on,
    /// and until then `delivery_outbox` grows with each delivered message.
    /// CLOACINA-T-1023, CLOACINA-T-1092.
    #[arg(long, env = "CLOACINA_OUTBOX_ACKED_RETENTION_S", default_value_t = 0)]
    outbox_acked_retention_s: u64,

    /// Bytes of the end of each fleet task attempt's output (stdout and
    /// stderr, as one stream) that the agent keeps and the server stores
    /// with the result. Longer output is cut at its start and stored with a
    /// "cut" flag and its total size. `0` keeps no output. Default 262144
    /// (256 KiB). CLOACINA-T-1091.
    #[arg(
        long,
        env = "CLOACINA_TASK_OUTPUT_MAX_BYTES",
        default_value_t = cloacina::fleet::DEFAULT_TASK_OUTPUT_MAX_BYTES
    )]
    task_output_max_bytes: u64,

    /// Days to keep the stored output of a task attempt, counted from when
    /// it was stored. A background job then deletes it, whether
    /// `--run-history-retention-days` is set or not. `0` keeps it for ever.
    /// Default 30. CLOACINA-T-1091.
    #[arg(
        long,
        env = "CLOACINA_TASK_OUTPUT_RETENTION_DAYS",
        default_value_t = cloacina_server::retention::DEFAULT_TASK_OUTPUT_RETENTION_DAYS
    )]
    task_output_retention_days: u64,

    /// Seconds between passes of the retention job. Default 300.
    /// CLOACINA-T-1023.
    #[arg(long, env = "CLOACINA_RETENTION_INTERVAL_S", default_value_t = 300)]
    retention_interval_s: u64,

    /// Executor every task is dispatched to (CLOACI-T-0640). Not set: `fleet`
    /// (the execution-agent fleet), or `default` (the in-process thread
    /// executor) with `--single-trust`. `default` needs `--single-trust`: the
    /// server does not start with it otherwise (CLOACINA-T-1075). The key must
    /// match a registered executor or the server fails fast at startup.
    /// `cloacinactl server start` forwards `[server].default_executor` of
    /// `config.toml` when it is set.
    #[arg(long, env = "CLOACINA_DEFAULT_EXECUTOR")]
    default_executor: Option<String>,

    /// Single trust (CLOACINA-T-1075): every tenant of this server is trusted
    /// as much as the server itself. Turns on the two things that run tenant
    /// code in the server process: packages are loaded in process (their
    /// libraries opened, their Python modules imported), and
    /// `--default-executor default` is allowed. Off by default: the server
    /// loads each package from the metadata document its compiler stored and
    /// sends every task to the agent fleet. The server logs a warning at start
    /// when it is on.
    #[arg(long, env = "CLOACINA_SINGLE_TRUST")]
    single_trust: bool,

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

    /// Where an execution agent runs a computation-graph firing: `inline`
    /// (in the agent process: no worker start per firing, and no memory
    /// limit, kill or cancel) or `worker` (a worker process with the limits,
    /// time limit and cancel of a task). Default `inline`. CLOACINA-T-1125.
    #[arg(
        long,
        env = "CLOACINA_FLEET_GRAPH_FIRINGS",
        value_enum,
        default_value_t = cloacina_server::fleet_graph_executor::GraphFiringExecution::Inline
    )]
    fleet_graph_firings: cloacina_server::fleet_graph_executor::GraphFiringExecution,

    /// How long (seconds) a fleet task waits, per attempt, while agents cannot
    /// fetch its package artifact. The time counts from the first refused
    /// fetch of the attempt; the task is dispatched again after a delay (2 s,
    /// growing to 30 s) and uses no retry attempt. After the limit the
    /// attempt fails with an error that names the artifact fetch. 0: the
    /// first failed fetch fails the attempt. Default 300. CLOACINA-T-1064.
    #[arg(
        long,
        env = "CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S",
        default_value_t = cloacina_server::fleet_executor::DEFAULT_ARTIFACT_FETCH_WAIT_SECONDS
    )]
    fleet_artifact_fetch_wait_s: u64,

    /// Time limit (seconds) of one poll of a poll trigger on an agent. The
    /// agent stops the poll after this long; the server waits this plus 10 s
    /// for the answer, then records the poll as missed and fires nothing.
    /// Default 30. CLOACINA-T-1077.
    #[arg(
        long,
        env = "CLOACINA_FLEET_POLL_TIMEOUT_S",
        default_value_t = cloacina_server::fleet_poll_dispatcher::DEFAULT_POLL_TIMEOUT_SECONDS
    )]
    fleet_poll_timeout_s: u64,

    /// The ageing interval of waiting work, in whole seconds. For each
    /// interval a task has waited, its priority in the order of waiting work
    /// improves by one, never past 0. `0` switches ageing off. Default 120.
    /// CLOACINA-T-1045, CLOACINA-T-1123.
    #[arg(
        long,
        env = "CLOACINA_PRIORITY_AGING_INTERVAL_SECONDS",
        default_value_t = cloacina_api_types::DEFAULT_PRIORITY_AGING_INTERVAL.as_secs()
    )]
    priority_aging_interval_s: u64,

    /// Size in bytes above which the context a task returns, or the context
    /// built for a task, is logged as a warning and counted in
    /// `cloacina_context_size_warnings_total`. 0 turns the warning off.
    /// Default 1048576 (1 MiB). CLOACINA-T-1047.
    #[arg(
        long,
        env = "CLOACINA_CONTEXT_WARN_BYTES",
        default_value_t = cloacina::executor::context_size::DEFAULT_CONTEXT_WARN_BYTES
    )]
    context_warn_bytes: u64,

    /// Largest size in bytes of a task context. A task whose result context
    /// or input context is larger fails with the size, the limit and the
    /// largest keys in the error, and is not retried. One setting for the
    /// whole server. 0 turns the limit off. Default 16777216 (16 MiB).
    /// CLOACINA-T-1047.
    #[arg(
        long,
        env = "CLOACINA_CONTEXT_MAX_BYTES",
        default_value_t = cloacina::executor::context_size::DEFAULT_CONTEXT_MAX_BYTES
    )]
    context_max_bytes: u64,

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

    /// The largest package archive, in bytes, that an upload accepts. A
    /// larger upload is refused with status 413 and an error that names the
    /// size and this limit. Default: 104857600 (100 MiB). A Python package
    /// carries its vendored dependencies, one folder per platform.
    /// CLOACINA-T-1048.
    #[arg(
        long,
        env = "CLOACINA_MAX_PACKAGE_BYTES",
        default_value_t = cloacina_server::upload_limit::DEFAULT_MAX_PACKAGE_BYTES
    )]
    max_package_bytes: u64,

    /// The largest size, in bytes, that a package archive may unpack to.
    /// The server refuses an
    /// upload that unpacks to more, and so does its package loader. Default: 524288000 (500 MiB). The limits of 10,000 entries and
    /// of 10 times the archive size stay. CLOACINA-T-1048.
    #[arg(
        long,
        env = "CLOACINA_MAX_UNPACKED_BYTES",
        default_value_t = cloacina::cloacina_workflow_plugin::unpack_limit::DEFAULT_MAX_UNPACKED_BYTES
    )]
    max_unpacked_bytes: u64,

    /// Days an agent key that the Kubernetes actuator mints stays valid.
    /// After it the key is refused with 401 `key_expired`. Default 30.
    /// Docker actuator keys do not expire. CLOACINA-T-1083.
    #[arg(
        long,
        env = "CLOACINA_AGENT_KEY_LIFETIME_DAYS",
        default_value_t = cloacina_server::actuator::AgentKeyExpiry::DEFAULT_LIFETIME_DAYS
    )]
    agent_key_lifetime_days: u32,

    /// Days left on an agent key at which the Kubernetes actuator replaces
    /// it: a new key in the Secret, and the pods roll onto it. The old key is
    /// revoked when every pool's rollout is complete. Must be at least 1 and
    /// less than `--agent-key-lifetime-days`. Default 10. CLOACINA-T-1083.
    #[arg(
        long,
        env = "CLOACINA_AGENT_KEY_ROTATE_DAYS",
        default_value_t = cloacina_server::actuator::AgentKeyExpiry::DEFAULT_ROTATE_DAYS
    )]
    agent_key_rotate_days: u32,

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

    // The package upload limit (CLOACINA-T-1048).
    cloacina_server::upload_limit::set_max_package_bytes(cli.max_package_bytes)
        .map_err(|e| anyhow::anyhow!(e))?;
    cloacina::cloacina_workflow_plugin::unpack_limit::set_max_unpacked_bytes(
        cli.max_unpacked_bytes,
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    // CLOACINA-T-1083: the expiry and rotation of the agent keys.
    cloacina_server::actuator::set_agent_key_expiry(
        cloacina_server::actuator::AgentKeyExpiry::from_days(
            cli.agent_key_lifetime_days,
            cli.agent_key_rotate_days,
        )
        .map_err(|e| anyhow::anyhow!(e))?,
    );

    // CLOACINA-T-1024: the console log format and the task log link.
    cloacina_server::task_logs::set_log_format(cli.log_format);
    if let Some(template) = cli
        .task_log_url_template
        .as_deref()
        .filter(|t| !t.trim().is_empty())
    {
        let template = cloacina_server::task_logs::TaskLogUrlTemplate::parse(template)
            .map_err(|e| anyhow::anyhow!("--task-log-url-template: {e}"))?;
        cloacina_server::task_logs::install_url_template(template);
    }

    // Fail at start on limits the server cannot run with (CLOACINA-T-1014).
    let fleet_task_time_limits =
        cloacina_server::fleet_executor::FleetTaskTimeLimits::from_seconds(
            cli.fleet_task_timeout_s,
            cli.fleet_task_timeout_max_s,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
    // CLOACINA-T-1077: the time limit of one poll on an agent.
    cloacina_server::fleet_poll_dispatcher::set_poll_timeout_seconds(cli.fleet_poll_timeout_s)
        .map_err(|e| anyhow::anyhow!(e))?;
    // CLOACINA-T-1123: the ageing interval of waiting work.
    cloacina_server::set_priority_aging_interval_seconds(cli.priority_aging_interval_s)
        .map_err(|e| anyhow::anyhow!(e))?;
    // CLOACINA-T-1091: the end of each attempt's output that is kept.
    cloacina_server::task_output::set_max_bytes(cli.task_output_max_bytes);

    // CLOACINA-T-1075: where tenant code runs. A mode that would run tenant
    // tasks in this process without single trust stops the start.
    let execution_mode = cloacina_server::execution_mode::ExecutionMode::resolve(
        cli.single_trust,
        cli.default_executor.as_deref(),
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
        execution_mode,
        cli.agent_heartbeat_interval_s,
        cli.agent_liveness_misses,
        fleet_task_time_limits,
        cli.fleet_graph_firings,
        std::time::Duration::from_secs(cli.fleet_artifact_fetch_wait_s),
        cloacina::executor::ContextSizeLimits {
            warn_bytes: cli.context_warn_bytes,
            max_bytes: cli.context_max_bytes,
        },
        cloacina_server::CorsConfig {
            allowed_origins: cli.cors_allowed_origins,
            allowed_methods: cli.cors_allowed_methods,
            allowed_headers: cli.cors_allowed_headers,
        },
        cloacina_server::retention::RetentionConfig::from_flags(
            cli.run_history_retention_days,
            cli.outbox_acked_retention_s,
            cli.task_output_retention_days,
            cli.retention_interval_s,
        ),
        std::time::Duration::from_secs(cli.database_wait_timeout_s),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;
    use cloacina_server::retention::RetentionConfig;

    fn retention_of(args: &[&str]) -> RetentionConfig {
        let cli = Cli::try_parse_from(args).unwrap();
        RetentionConfig::from_flags(
            cli.run_history_retention_days,
            cli.outbox_acked_retention_s,
            cli.task_output_retention_days,
            cli.retention_interval_s,
        )
    }

    // CLOACINA-T-1092: with no retention flag no run history and no acked
    // outbox row is deleted. CLOACINA-T-1091: task output goes after 30
    // days, so the retention job starts; with that set to 0 it does not.
    #[test]
    fn retention_flags_are_off_by_default_but_for_task_output() {
        let cli = Cli::try_parse_from(["cloacina-server"]).unwrap();
        assert_eq!(cli.outbox_acked_retention_s, 0);
        assert_eq!(cli.run_history_retention_days, 0);
        assert_eq!(cli.task_output_retention_days, 30);
        assert_eq!(cli.task_output_max_bytes, 256 * 1024);

        let cfg = retention_of(&["cloacina-server"]);
        assert_eq!(cfg, RetentionConfig::default());
        assert_eq!(cfg.outbox_acked, None);
        assert_eq!(cfg.run_history, None);
        assert!(cfg.enabled());

        let cfg = retention_of(&["cloacina-server", "--task-output-retention-days", "0"]);
        assert_eq!(cfg.task_output, None);
        assert!(!cfg.enabled());
    }

    #[test]
    fn outbox_acked_retention_flag_turns_the_cleanup_on() {
        let cfg = retention_of(&["cloacina-server", "--outbox-acked-retention-s", "3600"]);
        assert_eq!(
            cfg.outbox_acked,
            Some(std::time::Duration::from_secs(3_600))
        );
        assert!(cfg.enabled());
    }

    // CLOACINA-T-1075: single trust is off and the executor is not set by
    // default, so the server needs the fleet; the flag turns single trust on.
    #[test]
    fn single_trust_is_off_by_default_and_the_executor_unset() {
        use cloacina_server::execution_mode::ExecutionMode;
        let cli = Cli::try_parse_from(["cloacina-server"]).unwrap();
        assert!(!cli.single_trust);
        assert_eq!(cli.default_executor, None);
        let mode =
            ExecutionMode::resolve(cli.single_trust, cli.default_executor.as_deref()).unwrap();
        assert_eq!(mode.default_executor, "fleet");

        let cli = Cli::try_parse_from([
            "cloacina-server",
            "--single-trust",
            "--default-executor",
            "default",
        ])
        .unwrap();
        assert!(cli.single_trust);
        let mode =
            ExecutionMode::resolve(cli.single_trust, cli.default_executor.as_deref()).unwrap();
        assert_eq!(mode.default_executor, "default");

        let cli =
            Cli::try_parse_from(["cloacina-server", "--default-executor", "default"]).unwrap();
        assert!(ExecutionMode::resolve(cli.single_trust, cli.default_executor.as_deref()).is_err());
    }

    // CLOACINA-T-1048: the unpack limit is a setting next to the upload limit.
    #[test]
    fn max_unpacked_bytes_flag_defaults_to_500_mib_and_is_read() {
        let cli = Cli::try_parse_from(["cloacina-server"]).unwrap();
        assert_eq!(cli.max_unpacked_bytes, 524_288_000);
        assert_eq!(cli.max_package_bytes, 104_857_600);
        let cli =
            Cli::try_parse_from(["cloacina-server", "--max-unpacked-bytes", "1073741824"]).unwrap();
        assert_eq!(cli.max_unpacked_bytes, 1_073_741_824);
    }

    // CLOACINA-T-1125: graph firings run inline on the agent unless set to
    // `worker`; any other value stops the server at start.
    #[test]
    fn fleet_graph_firings_default_to_inline_and_take_worker() {
        use cloacina_server::fleet_graph_executor::GraphFiringExecution;
        let cli = Cli::try_parse_from(["cloacina-server"]).unwrap();
        assert_eq!(cli.fleet_graph_firings, GraphFiringExecution::Inline);
        let cli =
            Cli::try_parse_from(["cloacina-server", "--fleet-graph-firings", "worker"]).unwrap();
        assert_eq!(cli.fleet_graph_firings, GraphFiringExecution::Worker);
        let cli =
            Cli::try_parse_from(["cloacina-server", "--fleet-graph-firings", "inline"]).unwrap();
        assert_eq!(cli.fleet_graph_firings, GraphFiringExecution::Inline);
    }

    // CLOACINA-T-1083: agent keys live 30 days and are rotated with 10 left.
    #[test]
    fn agent_key_expiry_flags_default_to_30_and_10_days() {
        let cli = Cli::try_parse_from(["cloacina-server"]).unwrap();
        assert_eq!(cli.agent_key_lifetime_days, 30);
        assert_eq!(cli.agent_key_rotate_days, 10);
        let cli = Cli::try_parse_from([
            "cloacina-server",
            "--agent-key-lifetime-days",
            "90",
            "--agent-key-rotate-days",
            "30",
        ])
        .unwrap();
        let rule = cloacina_server::actuator::AgentKeyExpiry::from_days(
            cli.agent_key_lifetime_days,
            cli.agent_key_rotate_days,
        )
        .unwrap();
        assert_eq!(rule.lifetime, chrono::Duration::days(90));
        assert_eq!(rule.rotate_before, chrono::Duration::days(30));
    }

    #[test]
    fn a_bad_fleet_graph_firings_value_is_refused_at_start() {
        let err = Cli::try_parse_from(["cloacina-server", "--fleet-graph-firings", "workers"])
            .err()
            .expect("refused");
        let text = err.to_string();
        assert!(text.contains("--fleet-graph-firings"), "{text}");
        assert!(text.contains("inline") && text.contains("worker"), "{text}");
    }

    /// CLOACINA-T-1077: the time limit of a poll on an agent.
    #[test]
    fn the_poll_time_limit_defaults_to_30_seconds_and_refuses_0() {
        let cli = Cli::try_parse_from(["cloacina-server"]).unwrap();
        assert_eq!(cli.fleet_poll_timeout_s, 30);
        let cli = Cli::try_parse_from(["cloacina-server", "--fleet-poll-timeout-s", "5"]).unwrap();
        assert_eq!(cli.fleet_poll_timeout_s, 5);
        assert!(cloacina_server::fleet_poll_dispatcher::set_poll_timeout_seconds(0).is_err());
    }

    /// CLOACINA-T-1123: the ageing interval is a flag, 120 s by default; 0
    /// switches ageing off.
    #[test]
    fn the_priority_aging_interval_defaults_to_120_seconds_and_takes_0() {
        let cli = Cli::try_parse_from(["cloacina-server"]).unwrap();
        assert_eq!(cli.priority_aging_interval_s, 120);
        let cli =
            Cli::try_parse_from(["cloacina-server", "--priority-aging-interval-s", "0"]).unwrap();
        assert_eq!(cli.priority_aging_interval_s, 0);
        let cli =
            Cli::try_parse_from(["cloacina-server", "--priority-aging-interval-s", "300"]).unwrap();
        assert_eq!(cli.priority_aging_interval_s, 300);
        assert!(
            Cli::try_parse_from(["cloacina-server", "--priority-aging-interval-s", "2m"]).is_err()
        );
    }
}
