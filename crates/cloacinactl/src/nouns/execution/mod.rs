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
use futures_util::StreamExt;

use crate::commands::config::CloacinaConfig;
use crate::shared::client::CliClient;
use crate::shared::client_ctx::ClientContext;
use crate::shared::error::CliError;
use crate::shared::render;
use crate::{GlobalOpts, OutputFormat};

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
        /// Maximum number of rows to return (server-side cap: 1000).
        /// CLOACI-T-0596 / API-10.
        #[arg(long, default_value = "50")]
        limit: u32,
        /// Offset into the result set for pagination.
        #[arg(long, default_value = "0")]
        offset: u32,
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
                offset,
            } => {
                let mut query = format!("?limit={limit}&offset={offset}");
                if let Some(w) = workflow {
                    query.push_str(&format!("&workflow={w}"));
                }
                if let Some(s) = status {
                    query.push_str(&format!("&status={s}"));
                }
                let body: serde_json::Value = client
                    .get(&format!("/v1/tenants/{tenant}/executions{query}"))
                    .await?;
                render::list(&body, output)
            }
            ExecutionVerb::Status { id } => {
                let body: serde_json::Value = client
                    .get(&format!("/v1/tenants/{tenant}/executions/{id}"))
                    .await?;
                render::object(&body, output)
            }
            ExecutionVerb::Events { id, follow, since } => {
                if follow {
                    // CLOACI-T-0629: live event streaming over the interservice
                    // communication substrate (S-0012). Mint a single-use WS
                    // ticket, connect to the `delivery_outbox`-backed WS
                    // endpoint addressed at `exec_events:<execution_id>`,
                    // decode incoming `Push` frames, print each event, ack.
                    if since.is_some() {
                        return Err(CliError::UserError(
                            "--since cannot be combined with --follow yet (cursor support \
                             is OQ-C future work); pass --since on a non-follow call to get \
                             the historical snapshot, then run --follow for the live tail."
                                .into(),
                        ));
                    }
                    return follow_execution_events(client.as_ref(), &id, output).await;
                }
                let mut path = format!("/v1/tenants/{tenant}/executions/{id}/events");
                if let Some(s) = since {
                    path.push_str(&format!("?since={s}"));
                }
                let body: serde_json::Value = client.get(&path).await?;
                render::list(&body, output)
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// CLOACI-T-0629: --follow over the interservice communication substrate.
// ────────────────────────────────────────────────────────────────────────────

/// Stream the execution's events until the connection closes or Ctrl-C,
/// printing each in the user's chosen output format.
///
/// T-0646: the WS mechanics (ticket mint, connect, hello, push decode, ack)
/// moved into the published `cloacina-client` crate — the CLI consumes the
/// same stream third parties do. `reconnect: false` preserves the original
/// CLI behavior of exiting when the server closes the connection.
async fn follow_execution_events(
    client: &CliClient,
    exec_id: &str,
    output: OutputFormat,
) -> Result<(), CliError> {
    let options = cloacina_client::SubscribeOptions {
        reconnect: false,
        ..Default::default()
    };
    let stream = client
        .inner()
        .follow_execution_events_with(exec_id, options);
    let mut stream = std::pin::pin!(stream);
    while let Some(event) = stream.next().await {
        let event = event.map_err(CliError::from)?;
        render::object(&event, output)?;
    }
    Ok(())
}
