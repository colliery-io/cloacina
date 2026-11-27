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

use anyhow::{Result, bail};
use std::net::SocketAddr;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::GlobalOpts;
use crate::commands::config::{self, CloacinaConfig, ServerSection};
use crate::shared::pid;

/// Server settings that `server start` forwards to `cloacina-server`
/// (CLOACINA-T-1039). Each one is a flag here, the server's own env var, and a
/// key of `[server]` in `config.toml`. The order is: flag, env var,
/// `config.toml`. A setting that none of them gives is not forwarded, and the
/// server uses its own default.
///
/// The signing secret of the alert webhook is not here: it is the env var
/// `CLOACINA_ALERT_WEBHOOK_SECRET` only, and the server process inherits it.
#[derive(clap::Args, Debug, Default, Clone)]
pub struct ServerSettingArgs {
    /// Seconds the server waits at start for the database to accept a
    /// connection, with retries; `0` = one try. Overrides
    /// `[server].database_wait_timeout_s`.
    #[arg(long, env = "CLOACINA_DATABASE_WAIT_TIMEOUT_S")]
    pub database_wait_timeout_s: Option<u64>,

    /// Days to keep a finished run in the database; `0` keeps run history
    /// for ever. Overrides `[server].run_history_retention_days`.
    #[arg(long, env = "CLOACINA_RUN_HISTORY_RETENTION_DAYS")]
    pub run_history_retention_days: Option<u64>,

    /// Seconds to keep an acked `delivery_outbox` row; `0` keeps the rows for
    /// ever. Overrides `[server].outbox_acked_retention_s`.
    #[arg(long, env = "CLOACINA_OUTBOX_ACKED_RETENTION_S")]
    pub outbox_acked_retention_s: Option<u64>,

    /// Seconds between passes of the retention job. Overrides
    /// `[server].retention_interval_s`.
    #[arg(long, env = "CLOACINA_RETENTION_INTERVAL_S")]
    pub retention_interval_s: Option<u64>,

    /// Time limit (seconds) of a fleet task that sets none of its own, and of
    /// a graph firing on the fleet. Overrides `[server].fleet_task_timeout_s`.
    #[arg(long, env = "CLOACINA_FLEET_TASK_TIMEOUT_S")]
    pub fleet_task_timeout_s: Option<u64>,

    /// Longest time limit (seconds) a fleet task can get. Overrides
    /// `[server].fleet_task_timeout_max_s`.
    #[arg(long, env = "CLOACINA_FLEET_TASK_TIMEOUT_MAX_S")]
    pub fleet_task_timeout_max_s: Option<u64>,

    /// URL that gets a JSON POST when a run ends. Overrides
    /// `[server].alert_webhook_url`. The URL can hold a token, so it goes to
    /// the server as an env var, not as an argument.
    #[arg(long, env = "CLOACINA_ALERT_WEBHOOK_URL", hide_env_values = true)]
    pub alert_webhook_url: Option<String>,

    /// Run outcomes that send an alert, comma-separated: `failed`,
    /// `completed`, `cancelled`, or `all`. Overrides
    /// `[server].alert_webhook_events`.
    #[arg(long, env = "CLOACINA_ALERT_WEBHOOK_EVENTS", value_delimiter = ',')]
    pub alert_webhook_events: Vec<String>,

    /// Seconds allowed for one alert webhook request. Overrides
    /// `[server].alert_webhook_timeout_s`.
    #[arg(long, env = "CLOACINA_ALERT_WEBHOOK_TIMEOUT_S")]
    pub alert_webhook_timeout_s: Option<u64>,

    /// The most sends of one alert before it is dropped. Overrides
    /// `[server].alert_webhook_max_attempts`.
    #[arg(long, env = "CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS")]
    pub alert_webhook_max_attempts: Option<u32>,

    /// Public URL of the server, for the absolute links in an alert.
    /// Overrides `[server].public_base_url`.
    #[arg(long, env = "CLOACINA_PUBLIC_BASE_URL")]
    pub public_base_url: Option<String>,

    /// Template of the URL of a task's logs in your log stack, with the
    /// placeholders `{tenant}`, `{execution_id}`, `{task}` and `{attempt}`.
    /// The UI then shows a log link on each task. Overrides
    /// `[server].task_log_url_template`. CLOACINA-T-1024.
    #[arg(long, env = "CLOACINA_TASK_LOG_URL_TEMPLATE")]
    pub task_log_url_template: Option<String>,

    // CLOACINA-T-1106: the other flags of `cloacina-server`.
    /// Trusted organization id (UUID) of the server: with
    /// `--require-signatures`, a key of this organization is trusted for
    /// every tenant. Overrides `[server].verification_org_id`.
    #[arg(long, env = "CLOACINA_VERIFICATION_ORG_ID")]
    pub verification_org_id: Option<String>,

    /// Seconds between passes of the reconciler that syncs the workflow
    /// registry with the database. Overrides `[server].reconcile_interval_s`.
    #[arg(long)]
    pub reconcile_interval_s: Option<u64>,

    /// Warning limit on per-tenant runners; none is stopped at the limit.
    /// Overrides `[server].tenant_runner_cache_size`.
    #[arg(long, env = "CLOACINA_TENANT_RUNNER_CACHE_SIZE")]
    pub tenant_runner_cache_size: Option<usize>,

    /// Seconds a tenant delete waits for running workflows to finish.
    /// Overrides `[server].tenant_deletion_drain_timeout_s`.
    #[arg(long, env = "CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S")]
    pub tenant_deletion_drain_timeout_s: Option<u64>,

    /// Number of daily log files to keep under `--home`; `0` keeps them all.
    /// Overrides `[server].log_retention_days`.
    #[arg(long)]
    pub log_retention_days: Option<u64>,

    /// Format of the server's log lines on stderr: `text` or `json`.
    /// Overrides `[server].log_format`.
    #[arg(long, env = "CLOACINA_LOG_FORMAT")]
    pub log_format: Option<String>,

    /// Heartbeat interval (seconds) that the server gives fleet agents.
    /// Overrides `[server].agent_heartbeat_interval_s`.
    #[arg(long, env = "CLOACINA_AGENT_HEARTBEAT_INTERVAL_S")]
    pub agent_heartbeat_interval_s: Option<u32>,

    /// Missed heartbeats before a fleet agent is dead and its work is taken
    /// back. Overrides `[server].agent_liveness_misses`.
    #[arg(long, env = "CLOACINA_AGENT_LIVENESS_MISSES")]
    pub agent_liveness_misses: Option<u32>,

    /// Seconds a fleet task waits, per attempt, while agents cannot fetch its
    /// package artifact; `0` fails the attempt at the first failed fetch.
    /// Overrides `[server].fleet_artifact_fetch_wait_s`.
    #[arg(long, env = "CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S")]
    pub fleet_artifact_fetch_wait_s: Option<u64>,

    /// Where an agent runs a computation-graph firing: `inline` (in the
    /// agent process) or `worker` (in a worker process, with the limits of a
    /// task). Overrides `[server].fleet_graph_firings`. CLOACINA-T-1125.
    #[arg(long, env = "CLOACINA_FLEET_GRAPH_FIRINGS")]
    pub fleet_graph_firings: Option<String>,

    /// Size in bytes above which a task context is logged as a warning;
    /// `0` = off. Overrides `[server].context_warn_bytes`.
    #[arg(long, env = "CLOACINA_CONTEXT_WARN_BYTES")]
    pub context_warn_bytes: Option<u64>,

    /// Largest size in bytes of a task context; `0` = no limit. Overrides
    /// `[server].context_max_bytes`.
    #[arg(long, env = "CLOACINA_CONTEXT_MAX_BYTES")]
    pub context_max_bytes: Option<u64>,

    /// CORS allowed origins, comma-separated; CORS is off unless set.
    /// Overrides `[server].cors_allowed_origins`.
    #[arg(long, env = "CLOACINA_CORS_ALLOWED_ORIGINS", value_delimiter = ',')]
    pub cors_allowed_origins: Vec<String>,

    /// CORS allowed methods, comma-separated. Overrides
    /// `[server].cors_allowed_methods`.
    #[arg(long, env = "CLOACINA_CORS_ALLOWED_METHODS", value_delimiter = ',')]
    pub cors_allowed_methods: Vec<String>,

    /// CORS allowed request headers, comma-separated. Overrides
    /// `[server].cors_allowed_headers`.
    #[arg(long, env = "CLOACINA_CORS_ALLOWED_HEADERS", value_delimiter = ',')]
    pub cors_allowed_headers: Vec<String>,

    /// The largest package archive, in bytes, that an upload accepts.
    /// Overrides `[server].max_package_bytes`.
    #[arg(long, env = "CLOACINA_MAX_PACKAGE_BYTES")]
    pub max_package_bytes: Option<u64>,

    /// The largest size, in bytes, that a package archive may unpack to.
    /// Overrides `[server].max_unpacked_bytes`.
    #[arg(long, env = "CLOACINA_MAX_UNPACKED_BYTES")]
    pub max_unpacked_bytes: Option<u64>,

    /// Days an agent key that the Kubernetes actuator mints stays valid.
    /// Overrides `[server].agent_key_lifetime_days`. CLOACINA-T-1083.
    #[arg(long, env = "CLOACINA_AGENT_KEY_LIFETIME_DAYS")]
    pub agent_key_lifetime_days: Option<u32>,

    /// Days left on an agent key at which the Kubernetes actuator replaces
    /// it. Overrides `[server].agent_key_rotate_days`. CLOACINA-T-1083.
    #[arg(long, env = "CLOACINA_AGENT_KEY_ROTATE_DAYS")]
    pub agent_key_rotate_days: Option<u32>,
}

/// What `server start` adds to the `cloacina-server` command for the settings
/// in [`ServerSettingArgs`].
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ForwardedSettings {
    /// Arguments, in the order they are added.
    pub args: Vec<String>,
    /// Env vars set on the server process.
    pub env: Vec<(&'static str, String)>,
}

/// Resolve each setting (flag or env var first, then `[server]` in
/// `config.toml`) and give what to forward. A setting with no value gives
/// nothing.
pub(crate) fn forwarded_settings(
    cli: &ServerSettingArgs,
    config: &ServerSection,
) -> ForwardedSettings {
    let mut out = ForwardedSettings::default();
    let mut arg = |flag: &str, value: Option<String>| {
        if let Some(value) = value {
            out.args.push(flag.to_string());
            out.args.push(value);
        }
    };
    let number = |cli: Option<u64>, config: Option<u64>| cli.or(config).map(|n| n.to_string());

    arg(
        "--database-wait-timeout-s",
        number(cli.database_wait_timeout_s, config.database_wait_timeout_s),
    );
    arg(
        "--run-history-retention-days",
        number(
            cli.run_history_retention_days,
            config.run_history_retention_days,
        ),
    );
    arg(
        "--outbox-acked-retention-s",
        number(
            cli.outbox_acked_retention_s,
            config.outbox_acked_retention_s,
        ),
    );
    arg(
        "--retention-interval-s",
        number(cli.retention_interval_s, config.retention_interval_s),
    );
    arg(
        "--fleet-task-timeout-s",
        number(cli.fleet_task_timeout_s, config.fleet_task_timeout_s),
    );
    arg(
        "--fleet-task-timeout-max-s",
        number(
            cli.fleet_task_timeout_max_s,
            config.fleet_task_timeout_max_s,
        ),
    );
    let events = if cli.alert_webhook_events.is_empty() {
        &config.alert_webhook_events
    } else {
        &cli.alert_webhook_events
    };
    arg(
        "--alert-webhook-events",
        (!events.is_empty()).then(|| events.join(",")),
    );
    arg(
        "--alert-webhook-timeout-s",
        number(cli.alert_webhook_timeout_s, config.alert_webhook_timeout_s),
    );
    arg(
        "--alert-webhook-max-attempts",
        cli.alert_webhook_max_attempts
            .or(config.alert_webhook_max_attempts)
            .map(|n| n.to_string()),
    );
    arg(
        "--public-base-url",
        cli.public_base_url
            .clone()
            .or_else(|| config.public_base_url.clone()),
    );
    // CLOACINA-T-1024: an empty template is "not set": no log link.
    let not_blank = |t: &String| !t.trim().is_empty();
    arg(
        "--task-log-url-template",
        cli.task_log_url_template
            .clone()
            .filter(not_blank)
            .or_else(|| config.task_log_url_template.clone().filter(not_blank)),
    );

    // CLOACINA-T-1106: the other flags of `cloacina-server`.
    let text = |cli: &Option<String>, config: &Option<String>| {
        cli.clone()
            .filter(not_blank)
            .or_else(|| config.clone().filter(not_blank))
    };
    let list = |cli: &Vec<String>, config: &Vec<String>| {
        let values = if cli.is_empty() { config } else { cli };
        (!values.is_empty()).then(|| values.join(","))
    };
    arg(
        "--verification-org-id",
        text(&cli.verification_org_id, &config.verification_org_id),
    );
    arg(
        "--reconcile-interval-s",
        number(cli.reconcile_interval_s, config.reconcile_interval_s),
    );
    arg(
        "--tenant-runner-cache-size",
        cli.tenant_runner_cache_size
            .or(config.tenant_runner_cache_size)
            .map(|n| n.to_string()),
    );
    arg(
        "--tenant-deletion-drain-timeout-s",
        number(
            cli.tenant_deletion_drain_timeout_s,
            config.tenant_deletion_drain_timeout_s,
        ),
    );
    arg(
        "--log-retention-days",
        number(cli.log_retention_days, config.log_retention_days),
    );
    arg("--log-format", text(&cli.log_format, &config.log_format));
    arg(
        "--agent-heartbeat-interval-s",
        cli.agent_heartbeat_interval_s
            .or(config.agent_heartbeat_interval_s)
            .map(|n| n.to_string()),
    );
    arg(
        "--agent-liveness-misses",
        cli.agent_liveness_misses
            .or(config.agent_liveness_misses)
            .map(|n| n.to_string()),
    );
    arg(
        "--fleet-artifact-fetch-wait-s",
        number(
            cli.fleet_artifact_fetch_wait_s,
            config.fleet_artifact_fetch_wait_s,
        ),
    );
    arg(
        "--fleet-graph-firings",
        text(&cli.fleet_graph_firings, &config.fleet_graph_firings),
    );
    arg(
        "--context-warn-bytes",
        number(cli.context_warn_bytes, config.context_warn_bytes),
    );
    arg(
        "--context-max-bytes",
        number(cli.context_max_bytes, config.context_max_bytes),
    );
    arg(
        "--cors-allowed-origins",
        list(&cli.cors_allowed_origins, &config.cors_allowed_origins),
    );
    arg(
        "--cors-allowed-methods",
        list(&cli.cors_allowed_methods, &config.cors_allowed_methods),
    );
    arg(
        "--cors-allowed-headers",
        list(&cli.cors_allowed_headers, &config.cors_allowed_headers),
    );
    arg(
        "--max-package-bytes",
        number(cli.max_package_bytes, config.max_package_bytes),
    );
    arg(
        "--max-unpacked-bytes",
        number(cli.max_unpacked_bytes, config.max_unpacked_bytes),
    );
    arg(
        "--agent-key-lifetime-days",
        cli.agent_key_lifetime_days
            .or(config.agent_key_lifetime_days)
            .map(|n| n.to_string()),
    );
    arg(
        "--agent-key-rotate-days",
        cli.agent_key_rotate_days
            .or(config.agent_key_rotate_days)
            .map(|n| n.to_string()),
    );

    // The URL can hold a token: an env var of the server process, so it is
    // not in the process list.
    if let Some(url) = cli
        .alert_webhook_url
        .clone()
        .or_else(|| config.alert_webhook_url.clone())
    {
        out.env.push(("CLOACINA_ALERT_WEBHOOK_URL", url));
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    globals: &GlobalOpts,
    bind: SocketAddr,
    metrics_bind: Option<SocketAddr>,
    database_url: Option<String>,
    bootstrap_key: Option<String>,
    require_signatures: bool,
    default_executor: Option<String>,
    settings: ServerSettingArgs,
) -> Result<()> {
    #![allow(clippy::too_many_arguments)]
    let config_path = globals.home.join("config.toml");
    let db_url = config::resolve_database_url(database_url.as_deref(), &config_path)?;
    // CLOACI-T-0640: config.toml `[server].default_executor` is the preferred
    // surface; an explicit `--default-executor` overrides it. Forward the
    // resolved value so the server doesn't re-read config (cloacinactl is the
    // config-aware front door, same as `--database-url`).
    let default_executor =
        config::resolve_default_executor(default_executor.as_deref(), &config_path);

    // CLOACINA-T-1039: the database wait, retention, fleet time limit and
    // alert webhook settings, resolved the same way.
    let forwarded = forwarded_settings(&settings, &CloacinaConfig::load(&config_path).server);

    // Write PID file before exec — we're the process about to become server.
    pid::write(&globals.home.join("server.pid"))?;

    let mut cmd = server_command(
        &globals.home,
        bind,
        metrics_bind,
        &db_url,
        bootstrap_key,
        require_signatures,
        &default_executor,
    );
    if globals.verbose {
        cmd.arg("--verbose");
    }
    cmd.args(&forwarded.args);
    cmd.envs(forwarded.env.iter().map(|(name, value)| (*name, value)));

    // Replaces the current process. Only returns on error.
    let err = cmd.exec();
    bail!("failed to exec cloacina-server: {err}");
}

/// The `cloacina-server` command line for the resolved settings.
#[allow(clippy::too_many_arguments)]
fn server_command(
    home: &Path,
    bind: SocketAddr,
    metrics_bind: Option<SocketAddr>,
    db_url: &str,
    bootstrap_key: Option<String>,
    require_signatures: bool,
    default_executor: &str,
) -> Command {
    let mut cmd = Command::new("cloacina-server");
    cmd.arg("--home")
        .arg(home)
        .arg("--bind")
        .arg(bind.to_string())
        .arg("--database-url")
        .arg(db_url)
        .arg("--default-executor")
        .arg(default_executor);
    // CLOACINA-T-1032: the flag is passed on only when set, so the server's
    // own default (`/metrics` on `--bind`) is as before.
    if let Some(addr) = metrics_bind {
        cmd.arg("--metrics-bind").arg(addr.to_string());
    }
    if let Some(key) = bootstrap_key {
        cmd.arg("--bootstrap-key").arg(key);
    }
    if require_signatures {
        cmd.arg("--require-signatures");
    }
    cmd
}

// Silence "unused" warning when other imports are unused in certain features.
#[allow(dead_code)]
fn _config_type_check(_: CloacinaConfig) {}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tempfile::TempDir;

    /// The flags of `server start`, parsed with no env var in play.
    #[derive(Parser)]
    struct Flags {
        #[command(flatten)]
        settings: ServerSettingArgs,
    }

    fn section(toml: &str) -> ServerSection {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, toml).unwrap();
        CloacinaConfig::load(&path).server
    }

    const FULL_CONFIG: &str = r#"
[server]
database_wait_timeout_s = 120
run_history_retention_days = 30
outbox_acked_retention_s = 600
retention_interval_s = 60
fleet_task_timeout_s = 900
fleet_task_timeout_max_s = 7200
alert_webhook_url = "https://alerts.example.com/hook"
alert_webhook_events = ["failed", "cancelled"]
alert_webhook_timeout_s = 5
alert_webhook_max_attempts = 3
public_base_url = "https://cloacina.example.com"
verification_org_id = "6f1c2a7e-3b9d-4c55-9e0a-2d4b8f1e7a60"
reconcile_interval_s = 15
tenant_runner_cache_size = 512
tenant_deletion_drain_timeout_s = 45
log_retention_days = 7
log_format = "json"
agent_heartbeat_interval_s = 10
agent_liveness_misses = 4
fleet_artifact_fetch_wait_s = 120
fleet_graph_firings = "worker"
context_warn_bytes = 2097152
context_max_bytes = 33554432
cors_allowed_origins = ["https://ops.example.com", "https://ui.example.com"]
cors_allowed_methods = ["GET", "POST"]
cors_allowed_headers = ["authorization"]
max_package_bytes = 209715200
max_unpacked_bytes = 1073741824
agent_key_lifetime_days = 60
agent_key_rotate_days = 20
"#;

    /// The arguments forwarded for [`FULL_CONFIG`] after the settings of
    /// CLOACINA-T-1039, in order.
    const FULL_CONFIG_LATER_ARGS: [&str; 38] = [
        "--verification-org-id",
        "6f1c2a7e-3b9d-4c55-9e0a-2d4b8f1e7a60",
        "--reconcile-interval-s",
        "15",
        "--tenant-runner-cache-size",
        "512",
        "--tenant-deletion-drain-timeout-s",
        "45",
        "--log-retention-days",
        "7",
        "--log-format",
        "json",
        "--agent-heartbeat-interval-s",
        "10",
        "--agent-liveness-misses",
        "4",
        "--fleet-artifact-fetch-wait-s",
        "120",
        "--fleet-graph-firings",
        "worker",
        "--context-warn-bytes",
        "2097152",
        "--context-max-bytes",
        "33554432",
        "--cors-allowed-origins",
        "https://ops.example.com,https://ui.example.com",
        "--cors-allowed-methods",
        "GET,POST",
        "--cors-allowed-headers",
        "authorization",
        "--max-package-bytes",
        "209715200",
        "--max-unpacked-bytes",
        "1073741824",
        "--agent-key-lifetime-days",
        "60",
        "--agent-key-rotate-days",
        "20",
    ];

    #[test]
    fn nothing_set_forwards_nothing() {
        let forwarded =
            forwarded_settings(&ServerSettingArgs::default(), &ServerSection::default());
        assert_eq!(forwarded, ForwardedSettings::default());
    }

    #[test]
    fn config_toml_settings_are_forwarded() {
        let forwarded = forwarded_settings(&ServerSettingArgs::default(), &section(FULL_CONFIG));
        assert_eq!(
            forwarded.args[forwarded.args.len() - FULL_CONFIG_LATER_ARGS.len()..],
            FULL_CONFIG_LATER_ARGS
        );
        assert_eq!(
            forwarded.args[..forwarded.args.len() - FULL_CONFIG_LATER_ARGS.len()],
            [
                "--database-wait-timeout-s",
                "120",
                "--run-history-retention-days",
                "30",
                "--outbox-acked-retention-s",
                "600",
                "--retention-interval-s",
                "60",
                "--fleet-task-timeout-s",
                "900",
                "--fleet-task-timeout-max-s",
                "7200",
                "--alert-webhook-events",
                "failed,cancelled",
                "--alert-webhook-timeout-s",
                "5",
                "--alert-webhook-max-attempts",
                "3",
                "--public-base-url",
                "https://cloacina.example.com",
            ]
        );
        // The URL goes as an env var, never as an argument.
        assert_eq!(
            forwarded.env,
            [(
                "CLOACINA_ALERT_WEBHOOK_URL",
                "https://alerts.example.com/hook".to_string()
            )]
        );
        assert!(!forwarded.args.iter().any(|a| a.contains("alerts.example")));
    }

    #[test]
    fn a_flag_wins_over_config_toml() {
        let cli = ServerSettingArgs {
            run_history_retention_days: Some(0),
            fleet_task_timeout_s: Some(120),
            alert_webhook_url: Some("https://other.example.com/hook".to_string()),
            alert_webhook_events: vec!["all".to_string()],
            alert_webhook_max_attempts: Some(1),
            public_base_url: Some("https://ops.example.com".to_string()),
            ..ServerSettingArgs::default()
        };
        let forwarded = forwarded_settings(&cli, &section(FULL_CONFIG));
        let value_of = |flag: &str| {
            let at = forwarded.args.iter().position(|a| a == flag).unwrap();
            forwarded.args[at + 1].clone()
        };
        assert_eq!(value_of("--run-history-retention-days"), "0");
        assert_eq!(value_of("--fleet-task-timeout-s"), "120");
        assert_eq!(value_of("--alert-webhook-events"), "all");
        assert_eq!(value_of("--alert-webhook-max-attempts"), "1");
        assert_eq!(value_of("--public-base-url"), "https://ops.example.com");
        // Settings with no flag keep the value of config.toml.
        assert_eq!(value_of("--fleet-task-timeout-max-s"), "7200");
        assert_eq!(value_of("--retention-interval-s"), "60");
        assert_eq!(
            forwarded.env,
            [(
                "CLOACINA_ALERT_WEBHOOK_URL",
                "https://other.example.com/hook".to_string()
            )]
        );
    }

    #[test]
    fn one_config_key_forwards_one_setting() {
        let forwarded = forwarded_settings(
            &ServerSettingArgs::default(),
            &section("[server]\nfleet_task_timeout_s = 600\n"),
        );
        assert_eq!(forwarded.args, ["--fleet-task-timeout-s", "600"]);
        assert!(forwarded.env.is_empty());
    }

    #[test]
    fn flags_parse() {
        let flags = Flags::try_parse_from([
            "start",
            "--database-wait-timeout-s",
            "0",
            "--run-history-retention-days",
            "30",
            "--outbox-acked-retention-s",
            "600",
            "--retention-interval-s",
            "60",
            "--fleet-task-timeout-s",
            "900",
            "--fleet-task-timeout-max-s",
            "7200",
            "--alert-webhook-url",
            "https://alerts.example.com/hook",
            "--alert-webhook-events",
            "failed,cancelled",
            "--alert-webhook-timeout-s",
            "5",
            "--alert-webhook-max-attempts",
            "3",
            "--public-base-url",
            "https://cloacina.example.com",
        ])
        .unwrap()
        .settings;
        assert_eq!(flags.database_wait_timeout_s, Some(0));
        assert_eq!(flags.run_history_retention_days, Some(30));
        assert_eq!(flags.outbox_acked_retention_s, Some(600));
        assert_eq!(flags.retention_interval_s, Some(60));
        assert_eq!(flags.fleet_task_timeout_s, Some(900));
        assert_eq!(flags.fleet_task_timeout_max_s, Some(7200));
        assert_eq!(
            flags.alert_webhook_url.as_deref(),
            Some("https://alerts.example.com/hook")
        );
        assert_eq!(flags.alert_webhook_events, ["failed", "cancelled"]);
        assert_eq!(flags.alert_webhook_timeout_s, Some(5));
        assert_eq!(flags.alert_webhook_max_attempts, Some(3));
        assert_eq!(
            flags.public_base_url.as_deref(),
            Some("https://cloacina.example.com")
        );
    }

    fn args(metrics_bind: Option<SocketAddr>) -> Vec<String> {
        server_command(
            Path::new("/tmp/cloacina-home"),
            "127.0.0.1:8080".parse().unwrap(),
            metrics_bind,
            "postgres://u:p@h:5432/d",
            None,
            false,
            "default",
        )
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
    }

    #[test]
    fn metrics_bind_is_passed_to_the_server_when_set() {
        let args = args(Some("127.0.0.1:9090".parse().unwrap()));
        let at = args.iter().position(|a| a == "--metrics-bind").unwrap();
        assert_eq!(args[at + 1], "127.0.0.1:9090");
    }

    /// The whole command line: the base arguments, then the forwarded
    /// settings. No flag is there twice.
    #[test]
    fn no_flag_is_forwarded_twice() {
        let mut cmd = server_command(
            Path::new("/tmp/cloacina-home"),
            "127.0.0.1:8080".parse().unwrap(),
            Some("127.0.0.1:9090".parse().unwrap()),
            "postgres://u:p@h:5432/d",
            Some("key".to_string()),
            true,
            "fleet",
        );
        let forwarded = forwarded_settings(&ServerSettingArgs::default(), &section(FULL_CONFIG));
        cmd.args(&forwarded.args);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let flags: Vec<&String> = args.iter().filter(|a| a.starts_with("--")).collect();
        let unique: std::collections::BTreeSet<&String> = flags.iter().copied().collect();
        assert_eq!(flags.len(), unique.len(), "a flag is there twice: {args:?}");
        assert_eq!(flags.len(), 36);
        assert_eq!(flags[0], "--home");
        assert_eq!(flags[6], "--require-signatures");
        assert_eq!(flags[7], "--database-wait-timeout-s");
        assert_eq!(flags[17], "--verification-org-id");
        assert_eq!(flags[33], "--max-unpacked-bytes");
        assert_eq!(flags[35], "--agent-key-rotate-days");
    }

    /// CLOACINA-T-1106: each flag of [`ServerSettingArgs`] is forwarded under
    /// its own name when its `[server]` key is set. The alert webhook URL is
    /// the one that goes as an env var.
    #[test]
    fn every_setting_flag_is_forwarded_from_its_config_key() {
        use clap::CommandFactory;
        let own: std::collections::BTreeSet<String> = Flags::command()
            .get_arguments()
            .filter_map(|a| a.get_long().map(|l| format!("--{l}")))
            .filter(|f| f != "--alert-webhook-url")
            .collect();
        let forwarded = forwarded_settings(&ServerSettingArgs::default(), &section(FULL_CONFIG));
        let sent: std::collections::BTreeSet<String> = forwarded
            .args
            .iter()
            .filter(|a| a.starts_with("--"))
            .cloned()
            .collect();
        // FULL_CONFIG sets every key but task_log_url_template, which has its
        // own test.
        let mut expected = own.clone();
        expected.remove("--task-log-url-template");
        assert_eq!(sent, expected);
    }

    /// CLOACINA-T-1106: a flag of the new settings wins over its key, and an
    /// empty text value is "not set".
    #[test]
    fn later_settings_take_the_flag_first() {
        let cli = ServerSettingArgs {
            fleet_artifact_fetch_wait_s: Some(0),
            max_package_bytes: Some(1_000),
            log_format: Some("text".to_string()),
            cors_allowed_origins: vec!["*".to_string()],
            verification_org_id: Some(" ".to_string()),
            ..ServerSettingArgs::default()
        };
        let forwarded = forwarded_settings(&cli, &section(FULL_CONFIG));
        let value_of = |flag: &str| {
            let at = forwarded.args.iter().position(|a| a == flag).unwrap();
            forwarded.args[at + 1].clone()
        };
        assert_eq!(value_of("--fleet-artifact-fetch-wait-s"), "0");
        assert_eq!(value_of("--max-package-bytes"), "1000");
        assert_eq!(value_of("--log-format"), "text");
        assert_eq!(value_of("--cors-allowed-origins"), "*");
        assert_eq!(
            value_of("--verification-org-id"),
            "6f1c2a7e-3b9d-4c55-9e0a-2d4b8f1e7a60",
            "a blank flag falls back to config.toml"
        );
        assert_eq!(value_of("--max-unpacked-bytes"), "1073741824");
    }

    #[test]
    fn later_setting_flags_parse() {
        let flags = Flags::try_parse_from([
            "start",
            "--fleet-artifact-fetch-wait-s",
            "60",
            "--context-warn-bytes",
            "1",
            "--context-max-bytes",
            "2",
            "--max-package-bytes",
            "3",
            "--max-unpacked-bytes",
            "4",
            "--log-format",
            "json",
            "--cors-allowed-origins",
            "https://a.example.com,https://b.example.com",
            "--tenant-runner-cache-size",
            "64",
            "--agent-liveness-misses",
            "5",
            "--fleet-graph-firings",
            "worker",
            "--agent-key-lifetime-days",
            "45",
            "--agent-key-rotate-days",
            "15",
        ])
        .unwrap()
        .settings;
        assert_eq!(flags.agent_key_lifetime_days, Some(45));
        assert_eq!(flags.agent_key_rotate_days, Some(15));
        assert_eq!(flags.fleet_graph_firings.as_deref(), Some("worker"));
        assert_eq!(flags.fleet_artifact_fetch_wait_s, Some(60));
        assert_eq!(flags.context_warn_bytes, Some(1));
        assert_eq!(flags.context_max_bytes, Some(2));
        assert_eq!(flags.max_package_bytes, Some(3));
        assert_eq!(flags.max_unpacked_bytes, Some(4));
        assert_eq!(flags.log_format.as_deref(), Some("json"));
        assert_eq!(
            flags.cors_allowed_origins,
            ["https://a.example.com", "https://b.example.com"]
        );
        assert_eq!(flags.tenant_runner_cache_size, Some(64));
        assert_eq!(flags.agent_liveness_misses, Some(5));
    }

    /// CLOACINA-T-1024: the flag (or its env var), then `[server]` of
    /// `config.toml`; an empty value is "not set".
    #[test]
    fn task_log_url_template_is_passed_to_the_server_only_when_set() {
        let template = "https://logs.example.com/?run={execution_id}&task={task}";
        let from_config =
            "[server]\ntask_log_url_template = \"https://other.example.com/{task}\"\n";
        let forwarded = |flag: Option<&str>, config: &ServerSection| {
            let cli = ServerSettingArgs {
                task_log_url_template: flag.map(str::to_string),
                ..ServerSettingArgs::default()
            };
            forwarded_settings(&cli, config).args
        };

        assert_eq!(
            forwarded(Some(template), &ServerSection::default()),
            ["--task-log-url-template", template]
        );
        assert_eq!(
            forwarded(Some(template), &section(from_config)),
            ["--task-log-url-template", template],
            "the flag wins over config.toml"
        );
        assert_eq!(
            forwarded(None, &section(from_config)),
            [
                "--task-log-url-template",
                "https://other.example.com/{task}"
            ]
        );
        for unset in [None, Some(""), Some("  ")] {
            assert!(forwarded(unset, &ServerSection::default()).is_empty());
        }

        let flags = Flags::try_parse_from(["start", "--task-log-url-template", template])
            .unwrap()
            .settings;
        assert_eq!(flags.task_log_url_template.as_deref(), Some(template));
    }

    #[test]
    fn metrics_bind_is_left_out_when_not_set() {
        assert!(!args(None).iter().any(|a| a == "--metrics-bind"));
    }
}
