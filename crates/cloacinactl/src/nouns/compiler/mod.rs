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

//! `cloacinactl compiler <verb>` — cloacina-compiler lifecycle + probes.

use anyhow::Result;
use clap::{Args, Subcommand};
use std::net::SocketAddr;

use crate::GlobalOpts;

pub mod health;
pub mod start;
pub mod status;
pub mod stop;

#[derive(Args)]
pub struct CompilerCmd {
    #[command(subcommand)]
    verb: CompilerVerb,
}

#[derive(Subcommand)]
enum CompilerVerb {
    /// Start cloacina-compiler (execs the binary).
    Start {
        #[arg(long, default_value = "127.0.0.1:9000")]
        bind: SocketAddr,

        #[arg(long, env = "DATABASE_URL")]
        database_url: Option<String>,

        /// Poll interval for new pending rows (milliseconds).
        #[arg(long)]
        poll_interval_ms: Option<u64>,

        /// Heartbeat interval while building (seconds).
        #[arg(long)]
        heartbeat_interval_s: Option<u64>,

        /// Threshold past which a stuck `building` row is reset (seconds).
        #[arg(long)]
        stale_threshold_s: Option<u64>,

        /// Sweeper loop interval (seconds).
        #[arg(long)]
        sweep_interval_s: Option<u64>,
    },
    /// Stop a locally-running compiler via PID file + SIGTERM.
    Stop {
        /// Send SIGKILL instead of SIGTERM.
        #[arg(long)]
        force: bool,
    },
    /// Rich status via HTTP /v1/status.
    Status,
    /// Terse HTTP /health probe — exit 0 if up, 2 otherwise.
    Health,
}

impl CompilerCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<()> {
        match self.verb {
            CompilerVerb::Start {
                bind,
                database_url,
                poll_interval_ms,
                heartbeat_interval_s,
                stale_threshold_s,
                sweep_interval_s,
            } => {
                start::run(
                    globals,
                    bind,
                    database_url,
                    poll_interval_ms,
                    heartbeat_interval_s,
                    stale_threshold_s,
                    sweep_interval_s,
                )
                .await
            }
            CompilerVerb::Stop { force } => stop::run(globals, force).await,
            CompilerVerb::Status => status::run(globals).await,
            CompilerVerb::Health => health::run(globals).await,
        }
    }
}
