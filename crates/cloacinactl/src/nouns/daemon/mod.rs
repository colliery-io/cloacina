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

//! `cloacinactl daemon <verb>` — local scheduler verbs.

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

use crate::GlobalOpts;

pub mod health;
pub mod start;
pub mod status;
pub mod stop;

#[derive(Args)]
pub struct DaemonCmd {
    #[command(subcommand)]
    verb: DaemonVerb,
}

#[derive(Subcommand)]
enum DaemonVerb {
    /// Run the daemon in the foreground.
    Start {
        /// Directories to watch for .cloacina packages (repeatable).
        #[arg(long = "watch-dir")]
        watch_dirs: Vec<PathBuf>,

        /// Reconciler poll interval in milliseconds.
        #[arg(long, default_value = "500")]
        poll_interval: u64,
    },
    /// Stop a running daemon via PID file + SIGTERM.
    Stop {
        /// Send SIGKILL instead of SIGTERM.
        #[arg(long)]
        force: bool,
    },
    /// Rich status via the daemon's Unix socket.
    Status,
    /// Terse probe — exit 0 if up, 2 otherwise.
    Health,
}

impl DaemonCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<()> {
        match self.verb {
            DaemonVerb::Start {
                watch_dirs,
                poll_interval,
            } => start::run(globals, watch_dirs, poll_interval).await,
            DaemonVerb::Stop { force } => stop::run(globals, force).await,
            DaemonVerb::Status => status::run(globals).await,
            DaemonVerb::Health => health::run(globals).await,
        }
    }
}
