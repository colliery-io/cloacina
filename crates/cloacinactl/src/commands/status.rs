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

//! `cloacinactl status` — queries the daemon health socket and displays status.

use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;

use super::health::DaemonHealth;

/// Connect to the daemon's Unix socket and display health status.
pub async fn run(home: PathBuf) -> Result<()> {
    let socket_path = home.join("daemon.sock");

    if !socket_path.exists() {
        println!(
            "Daemon is not running (no socket at {})",
            socket_path.display()
        );
        return Ok(());
    }

    let mut stream = UnixStream::connect(&socket_path)
        .await
        .context("Failed to connect to daemon health socket — is the daemon running?")?;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .context("Failed to read health response")?;

    let health: DaemonHealth =
        serde_json::from_slice(&buf).context("Failed to parse health response")?;

    display_health(&health);
    Ok(())
}

fn display_health(health: &DaemonHealth) {
    let status_indicator = match health.status.as_str() {
        "healthy" => "OK",
        "degraded" => "DEGRADED",
        "unhealthy" => "UNHEALTHY",
        other => other,
    };

    println!("Status:       {}", status_indicator);
    println!("PID:          {}", health.pid);
    println!("Uptime:       {}", format_duration(health.uptime_seconds));
    println!(
        "Database:     {} ({})",
        health.database.backend,
        if health.database.connected {
            "connected"
        } else {
            "DISCONNECTED"
        }
    );
    println!("Pipelines:    {} active", health.active_pipelines);
    println!("Packages:     {} loaded", health.reconciler.packages_loaded);
    if let Some(ref last) = health.reconciler.last_run_at {
        println!("Last reconcile: {}", last);
    }
}

fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}
