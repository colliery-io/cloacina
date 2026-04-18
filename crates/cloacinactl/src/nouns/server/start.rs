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

use anyhow::{bail, Result};
use std::net::SocketAddr;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::commands::config::{self, CloacinaConfig};
use crate::shared::pid;
use crate::GlobalOpts;

pub async fn run(
    globals: &GlobalOpts,
    bind: SocketAddr,
    database_url: Option<String>,
    bootstrap_key: Option<String>,
    require_signatures: bool,
) -> Result<()> {
    let config_path = globals.home.join("config.toml");
    let db_url = config::resolve_database_url(database_url.as_deref(), &config_path)?;

    // Write PID file before exec — we're the process about to become server.
    pid::write(&globals.home.join("server.pid"))?;

    let mut cmd = Command::new("cloacina-server");
    cmd.arg("--home")
        .arg(&globals.home)
        .arg("--bind")
        .arg(bind.to_string())
        .arg("--database-url")
        .arg(&db_url);
    if globals.verbose {
        cmd.arg("--verbose");
    }
    if let Some(key) = bootstrap_key {
        cmd.arg("--bootstrap-key").arg(key);
    }
    if require_signatures {
        cmd.arg("--require-signatures");
    }

    // Replaces the current process. Only returns on error.
    let err = cmd.exec();
    bail!("failed to exec cloacina-server: {err}");
}

// Silence "unused" warning when other imports are unused in certain features.
#[allow(dead_code)]
fn _config_type_check(_: CloacinaConfig) {}
