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

//! cloacina-compiler — standalone build service for Cloacina packages.
//!
//! Polls the DB for packages in `build_status = pending`, compiles them, and
//! writes the resulting cdylib bytes back into `workflow_packages.compiled_data`.
//! Reconcilers on `cloacina-server` / `cloacinactl daemon` then load the bytes
//! directly — no runtime toolchain required. See CLOACI-I-0097 + ADR-0004.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;

use cloacina_compiler::{run, CompilerConfig};

/// cloacina-compiler — DB-queue-driven build service.
#[derive(Parser)]
#[command(name = "cloacina-compiler")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Cloacina home directory (logs + tmp scratch space).
    #[arg(long, default_value_os_t = default_home())]
    home: PathBuf,

    /// Address to bind the local /health + /v1/status endpoint to.
    #[arg(long, default_value = "127.0.0.1:9000")]
    bind: SocketAddr,

    /// Database URL (overrides CLOACINA_DATABASE_URL / DATABASE_URL env vars).
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Poll interval for new pending rows (milliseconds).
    #[arg(long, default_value_t = 2000)]
    poll_interval_ms: u64,

    /// Heartbeat interval while building (seconds).
    #[arg(long, default_value_t = 10)]
    heartbeat_interval_s: u64,

    /// Threshold past which a stuck `building` row is reset to `pending`
    /// (seconds). Should be at least 3× heartbeat interval.
    #[arg(long, default_value_t = 60)]
    stale_threshold_s: u64,

    /// Sweeper loop interval (seconds).
    #[arg(long, default_value_t = 30)]
    sweep_interval_s: u64,

    /// Extra cargo build flags. Default: --release --lib. Repeatable.
    #[arg(long = "cargo-flag")]
    cargo_flags: Vec<String>,
}

fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cargo_flags = if cli.cargo_flags.is_empty() {
        vec![
            "build".to_string(),
            "--release".to_string(),
            "--lib".to_string(),
        ]
    } else {
        cli.cargo_flags
    };

    let config = CompilerConfig {
        home: cli.home,
        bind: cli.bind,
        database_url: cli.database_url,
        verbose: cli.verbose,
        poll_interval: Duration::from_millis(cli.poll_interval_ms),
        heartbeat_interval: Duration::from_secs(cli.heartbeat_interval_s),
        stale_threshold: Duration::from_secs(cli.stale_threshold_s),
        sweep_interval: Duration::from_secs(cli.sweep_interval_s),
        cargo_flags,
        tmp_root: None,
    };

    run(config).await
}
