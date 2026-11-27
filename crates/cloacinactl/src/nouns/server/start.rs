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
    task_log_url_template: Option<String>,
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
        task_log_url_template,
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
    task_log_url_template: Option<String>,
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
    // CLOACINA-T-1024: passed on only when set; not set = no log link.
    if let Some(template) = task_log_url_template.filter(|t| !t.trim().is_empty()) {
        cmd.arg("--task-log-url-template").arg(template);
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
"#;

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
            forwarded.args,
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
        args_with(metrics_bind, None)
    }

    fn args_with(metrics_bind: Option<SocketAddr>, log_url: Option<&str>) -> Vec<String> {
        server_command(
            Path::new("/tmp/cloacina-home"),
            "127.0.0.1:8080".parse().unwrap(),
            metrics_bind,
            log_url.map(str::to_string),
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
        assert_eq!(flags.len(), 17);
        assert_eq!(flags[0], "--home");
        assert_eq!(flags[6], "--require-signatures");
        assert_eq!(flags[7], "--database-wait-timeout-s");
    }

    #[test]
    fn task_log_url_template_is_passed_to_the_server_only_when_set() {
        let template = "https://logs.example.com/?run={execution_id}&task={task}";
        let args = args_with(None, Some(template));
        let at = args
            .iter()
            .position(|a| a == "--task-log-url-template")
            .unwrap();
        assert_eq!(args[at + 1], template);

        for unset in [None, Some(""), Some("  ")] {
            assert!(
                !args_with(None, unset)
                    .iter()
                    .any(|a| a == "--task-log-url-template")
            );
        }
    }

    #[test]
    fn metrics_bind_is_left_out_when_not_set() {
        assert!(!args(None).iter().any(|a| a == "--metrics-bind"));
    }
}
