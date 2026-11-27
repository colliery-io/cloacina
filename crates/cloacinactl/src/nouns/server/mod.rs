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

        /// Address of a separate listener for `/metrics`. When set, `/metrics`
        /// is served only there and `--bind` answers 404 for it.
        #[arg(long, env = "CLOACINA_METRICS_BIND")]
        metrics_bind: Option<SocketAddr>,

        #[arg(long, env = "DATABASE_URL")]
        database_url: Option<String>,

        #[arg(long, env = "CLOACINA_BOOTSTRAP_KEY")]
        bootstrap_key: Option<String>,

        #[arg(long, env = "CLOACINA_REQUIRE_SIGNATURES")]
        require_signatures: bool,

        /// Executor every task is dispatched to (CLOACI-T-0640): `fleet`
        /// (execution-agent fleet) or `default` (in-process thread executor,
        /// needs single trust). Overrides `[server].default_executor` in
        /// config.toml when set. Not set: the server picks.
        #[arg(long, env = "CLOACINA_DEFAULT_EXECUTOR")]
        default_executor: Option<String>,

        /// Single trust (CLOACINA-T-1075): tenant code runs in the server
        /// process. Forwarded as `--single-trust`. Also on with
        /// `[server].single_trust = true` in config.toml.
        #[arg(long, env = "CLOACINA_SINGLE_TRUST")]
        single_trust: bool,

        #[command(flatten)]
        settings: Box<start::ServerSettingArgs>,
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
                metrics_bind,
                database_url,
                bootstrap_key,
                require_signatures,
                default_executor,
                single_trust,
                settings,
            } => {
                start::run(
                    globals,
                    bind,
                    metrics_bind,
                    database_url,
                    bootstrap_key,
                    require_signatures,
                    default_executor,
                    single_trust,
                    *settings,
                )
                .await
            }
            ServerVerb::Stop { force } => stop::run(globals, force).await,
            ServerVerb::Status => status::run(globals).await,
            ServerVerb::Health => health::run(globals).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The source of the flags of `cloacina-server`.
    const SERVER_MAIN: &str = include_str!("../../../../cloacina-server/src/main.rs");

    /// The long flags of `cloacinactl server start`, its own and the two
    /// global flags it passes on (`--home`, `--verbose`).
    fn server_start_flags() -> BTreeSet<String> {
        let cmd = ServerCmd::augment_args(clap::Command::new("server"));
        let start = cmd
            .find_subcommand("start")
            .expect("server has a start verb");
        let mut flags: BTreeSet<String> = start
            .get_arguments()
            .filter_map(|a| a.get_long().map(|l| format!("--{l}")))
            .collect();
        flags.insert("--home".to_string());
        flags.insert("--verbose".to_string());
        flags
    }

    /// CLOACINA-T-1106: every flag of `cloacina-server` is a flag of
    /// `cloacinactl server start`. A new server setting fails this test until
    /// `server start` takes it (a field of `ServerSettingArgs`, a key of
    /// `[server]`, a line of `forwarded_settings`).
    #[test]
    fn server_start_has_every_flag_of_cloacina_server() {
        let server = crate::shared::flag_list::long_flags(SERVER_MAIN, "Cli");
        assert!(
            server.len() > 30,
            "the server's flags were not read: {server:?}"
        );
        let start = server_start_flags();
        let missing: Vec<&String> = server.difference(&start).collect();
        assert!(
            missing.is_empty(),
            "flags of cloacina-server that `cloacinactl server start` lacks: {missing:?}"
        );
        // And `server start` has no flag that the server does not know.
        let unknown: Vec<&String> = start.difference(&server).collect();
        assert!(
            unknown.is_empty(),
            "flags of `server start` that cloacina-server lacks: {unknown:?}"
        );
    }
}
