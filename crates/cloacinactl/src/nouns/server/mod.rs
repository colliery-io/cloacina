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

//! `cloacinactl server <verb>` — cloacina-server HTTP API verbs.

use anyhow::Result;
use clap::{Args, Subcommand};
use std::net::SocketAddr;

use crate::GlobalOpts;

pub mod health;
pub mod start;
pub mod status;
pub mod stop;

#[derive(Args)]
pub struct ServerCmd {
    #[command(subcommand)]
    verb: ServerVerb,
}

#[derive(Subcommand)]
enum ServerVerb {
    /// Start cloacina-server (execs the binary).
    Start {
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: SocketAddr,

        #[arg(long, env = "DATABASE_URL")]
        database_url: Option<String>,

        #[arg(long, env = "CLOACINA_BOOTSTRAP_KEY")]
        bootstrap_key: Option<String>,

        #[arg(long, env = "CLOACINA_REQUIRE_SIGNATURES")]
        require_signatures: bool,
    },
    /// Stop a locally-running server via PID file + SIGTERM.
    Stop {
        /// Send SIGKILL instead of SIGTERM.
        #[arg(long)]
        force: bool,
    },
    /// Rich status via HTTP.
    Status,
    /// Terse HTTP /health probe — exit 0 if up, 2 otherwise.
    Health,
}

impl ServerCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<()> {
        match self.verb {
            ServerVerb::Start {
                bind,
                database_url,
                bootstrap_key,
                require_signatures,
            } => {
                start::run(
                    globals,
                    bind,
                    database_url,
                    bootstrap_key,
                    require_signatures,
                )
                .await
            }
            ServerVerb::Stop { force } => stop::run(globals, force).await,
            ServerVerb::Status => status::run(globals).await,
            ServerVerb::Health => health::run(globals).await,
        }
    }
}
