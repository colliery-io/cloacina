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

//! `cloacinactl execution <verb>`.

use clap::{Args, Subcommand};

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::GlobalOpts;

#[derive(Args)]
pub struct ExecutionCmd {
    #[command(subcommand)]
    verb: ExecutionVerb,
}

#[derive(Subcommand)]
enum ExecutionVerb {
    /// Recent executions.
    List {
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u32,
    },
    /// Current state of a single execution.
    Status { id: String },
    /// Event trail for an execution.
    Events {
        id: String,
        /// Follow live events (SSE) until Ctrl-C.
        #[arg(long)]
        follow: bool,
        /// Only events since this duration ago (e.g. "5m").
        #[arg(long)]
        since: Option<String>,
    },
}

impl ExecutionCmd {
    pub async fn run(self, globals: &GlobalOpts) -> Result<(), CliError> {
        let config = CloacinaConfig::load(&globals.home.join("config.toml"));
        let ctx = ClientContext::resolve(globals, &config).map_err(CliError::Other)?;
        let output = ctx.output;
        let client = CliClient::new(ctx)?;
        let tenant = client.ctx().tenant_segment().to_string();
        match self.verb {
            ExecutionVerb::List {
                workflow,
                status,
                limit,
            } => {
                let mut query = format!("?limit={limit}");
                if let Some(w) = workflow {
                    query.push_str(&format!("&workflow={w}"));
                }
                if let Some(s) = status {
                    query.push_str(&format!("&status={s}"));
                }
                let body: serde_json::Value = client
                    .get(&format!("/tenants/{tenant}/executions{query}"))
                    .await?;
                render::list(&body, output)
            }
            ExecutionVerb::Status { id } => {
                let body: serde_json::Value = client
                    .get(&format!("/tenants/{tenant}/executions/{id}"))
                    .await?;
                render::object(&body, output)
            }
            ExecutionVerb::Events { id, follow, since } => {
                if follow {
                    return Err(CliError::UserError(
                        "--follow streaming is tracked under spec Open Items; not in v1".into(),
                    ));
                }
                let mut path = format!("/tenants/{tenant}/executions/{id}/events");
                if let Some(s) = since {
                    path.push_str(&format!("?since={s}"));
                }
                let body: serde_json::Value = client.get(&path).await?;
                render::list(&body, output)
            }
        }
    }
}
