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
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

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

/// Mint a single-use WebSocket ticket, connect to the substrate delivery
/// endpoint addressed at `exec_events:<exec_id>`, and stream events until the
/// connection closes or Ctrl-C. Each `push` frame's payload is decoded as the
/// JSON event the producer enqueued in T-0629 Phase A, printed in the user's
/// chosen output format, and acknowledged so the row moves to `acked`.
async fn follow_execution_events(
    client: &CliClient,
    exec_id: &str,
    output: OutputFormat,
) -> Result<(), CliError> {
    // 1. Mint a ws-ticket via the existing REST endpoint.
    let ticket_resp: serde_json::Value = client
        .post::<_, serde_json::Value>("/v1/auth/ws-ticket", &serde_json::json!({}))
        .await?;
    let ticket = ticket_resp
        .get("ticket")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            CliError::Other(anyhow::anyhow!(
                "ws-ticket response missing `ticket` field: {}",
                ticket_resp
            ))
        })?;

    // 2. Build the wss/ws URL from the configured server.
    let url = ws_url_for(&client.ctx().server, exec_id, ticket)?;

    // 3. Connect.
    let (mut stream, _resp) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("WS connect failed for {}: {}", url, e)))?;

    // 4. Receive loop: parse Push frames, print payload, send Ack.
    while let Some(msg) = stream.next().await {
        let msg = msg.map_err(|e| CliError::Other(anyhow::anyhow!("WS recv error: {}", e)))?;
        match msg {
            Message::Text(text) => {
                let parsed: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("warning: invalid frame from server: {e}");
                        continue;
                    }
                };
                match parsed.get("type").and_then(|t| t.as_str()) {
                    Some("welcome") => {} // informational, no output
                    Some("push") => {
                        if let Some(event) = decode_push_payload(&parsed) {
                            render::object(&event, output)?;
                        }
                        if let Some(id) = parsed.get("id").and_then(|v| v.as_i64()) {
                            let ack = serde_json::json!({
                                "type": "ack",
                                "protocol_version": 1,
                                "id": id,
                            });
                            let ack_text = serde_json::to_string(&ack).map_err(|e| {
                                CliError::Other(anyhow::anyhow!("ack serialize: {}", e))
                            })?;
                            stream
                                .send(Message::Text(ack_text.into()))
                                .await
                                .map_err(|e| {
                                    CliError::Other(anyhow::anyhow!("ack send failed: {}", e))
                                })?;
                        }
                    }
                    other => {
                        eprintln!("warning: ignoring unknown frame type: {:?}", other);
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            Message::Binary(_) => {
                eprintln!("warning: substrate sends text frames; ignoring binary");
            }
        }
    }
    Ok(())
}

/// Decode `payload_b64` from a `push` envelope into the producer-side JSON event.
fn decode_push_payload(frame: &serde_json::Value) -> Option<serde_json::Value> {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    let b64 = frame.get("payload_b64")?.as_str()?;
    let bytes = BASE64.decode(b64).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Build a `ws://` or `wss://` URL for the substrate delivery endpoint of a
/// specific execution. Recipient string convention is `exec_events:<exec_id>`.
fn ws_url_for(server: &str, exec_id: &str, ticket: &str) -> Result<String, CliError> {
    let trimmed = server.trim_end_matches('/');
    let ws_base = if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("wss://{}", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("ws://{}", rest)
    } else {
        return Err(CliError::Other(anyhow::anyhow!(
            "server must start with http:// or https:// (got {})",
            server
        )));
    };
    // `:` is unreserved in path segments per RFC 3986; encode anyway for
    // resilience against intermediaries that mishandle it.
    let recipient = format!("exec_events%3A{}", exec_id);
    Ok(format!(
        "{}/v1/ws/delivery/{}?token={}",
        ws_base, recipient, ticket
    ))
}

#[cfg(test)]
mod ws_url_tests {
    use super::ws_url_for;

    #[test]
    fn https_becomes_wss() {
        let url = ws_url_for("https://api.example.com:8443", "abc-123", "tk-1").unwrap();
        assert_eq!(
            url,
            "wss://api.example.com:8443/v1/ws/delivery/exec_events%3Aabc-123?token=tk-1"
        );
    }

    #[test]
    fn http_becomes_ws() {
        let url = ws_url_for("http://localhost:8080/", "abc", "tk").unwrap();
        assert_eq!(
            url,
            "ws://localhost:8080/v1/ws/delivery/exec_events%3Aabc?token=tk"
        );
    }

    #[test]
    fn unsupported_scheme_errors() {
        let err = ws_url_for("ftp://x", "y", "z").unwrap_err();
        assert!(format!("{:?}", err).contains("http"));
    }
}
