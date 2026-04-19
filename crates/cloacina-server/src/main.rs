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

    /// Database URL (overrides DATABASE_URL env var)
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    /// Bootstrap API key (used instead of auto-generating on first startup).
    #[arg(long, env = "CLOACINA_BOOTSTRAP_KEY")]
    bootstrap_key: Option<String>,

    /// Require package signatures for workflow uploads.
    #[arg(long, env = "CLOACINA_REQUIRE_SIGNATURES")]
    require_signatures: bool,

    /// Interval (seconds) between reconciler passes that sync the in-runner
    /// workflow registry with the DB. Default matches the cloacina runtime
    /// default; override upward for quiet prod, downward for fast e2e.
    #[arg(long)]
    reconcile_interval_s: Option<u64>,
}

fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    cloacina_server::run(
        cli.home,
        cli.bind,
        cli.database_url,
        cli.verbose,
        cli.bootstrap_key,
        cli.require_signatures,
        cli.reconcile_interval_s.map(std::time::Duration::from_secs),
    )
    .await
}
