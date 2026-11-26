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
use crate::commands::config::{self, CloacinaConfig};
use crate::shared::pid;

pub async fn run(
    globals: &GlobalOpts,
    bind: SocketAddr,
    metrics_bind: Option<SocketAddr>,
    database_url: Option<String>,
    bootstrap_key: Option<String>,
    require_signatures: bool,
    default_executor: Option<String>,
) -> Result<()> {
    let config_path = globals.home.join("config.toml");
    let db_url = config::resolve_database_url(database_url.as_deref(), &config_path)?;
    // CLOACI-T-0640: config.toml `[server].default_executor` is the preferred
    // surface; an explicit `--default-executor` overrides it. Forward the
    // resolved value so the server doesn't re-read config (cloacinactl is the
    // config-aware front door, same as `--database-url`).
    let default_executor =
        config::resolve_default_executor(default_executor.as_deref(), &config_path);

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

    // Replaces the current process. Only returns on error.
    let err = cmd.exec();
    bail!("failed to exec cloacina-server: {err}");
}

/// The `cloacina-server` command line for the resolved settings.
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

    #[test]
    fn metrics_bind_is_left_out_when_not_set() {
        assert!(!args(None).iter().any(|a| a == "--metrics-bind"));
    }
}
