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

//! Substrate delivery WebSocket consumer (`GET /v1/ws/delivery/{recipient}`).
//!
//! Protocol reference: the WebSocket Protocol page of the docs site;
//! JSON Schemas under `/schemas/ws/`. Delivery is at-least-once — this
//! stream dedups on row id and acks each frame after yielding it, so a
//! consumer crash before processing leaves the row unacked → redelivered.

use std::collections::HashSet;
use std::time::Duration;

use async_stream::try_stream;
use futures_util::{SinkExt, Stream, StreamExt};
use tokio_tungstenite::tungstenite::Message;

pub use cloacina_api_types::delivery::DELIVERY_PROTOCOL_VERSION;
use cloacina_api_types::delivery::{ClientMessage, ServerMessage};

use crate::error::ClientError;
use crate::Client;

/// One decoded delivery push.
#[derive(Debug, Clone)]
pub struct DeliveryPush {
    /// Outbox row id — dedup key, already acked by the stream.
    pub id: i64,
    /// Producer-defined payload discriminator (e.g. `execution_event`).
    pub kind: String,
    pub recipient: String,
    pub tenant_id: Option<String>,
    /// Decoded payload bytes (base64 on the wire).
    pub payload: Vec<u8>,
}

/// Options for [`Client::subscribe_delivery`].
#[derive(Debug, Clone)]
pub struct SubscribeOptions {
    /// Reconnect on abnormal closure (default true).
    pub reconnect: bool,
    /// Initial reconnect backoff (default 100ms, doubles up to max).
    pub reconnect_initial: Duration,
    /// Max reconnect backoff (default 30s).
    pub reconnect_max: Duration,
}

impl Default for SubscribeOptions {
    fn default() -> Self {
        Self {
            reconnect: true,
            reconnect_initial: Duration::from_millis(100),
            reconnect_max: Duration::from_secs(30),
        }
    }
}

fn ws_base(server: &str) -> Result<String, ClientError> {
    if let Some(rest) = server.strip_prefix("https://") {
        Ok(format!("wss://{rest}"))
    } else if let Some(rest) = server.strip_prefix("http://") {
        Ok(format!("ws://{rest}"))
    } else {
        Err(ClientError::Config(format!(
            "server must start with http:// or https:// (got {server})"
        )))
    }
}

pub(crate) fn subscribe_delivery(
    client: Client,
    recipient: String,
    options: SubscribeOptions,
) -> impl Stream<Item = Result<DeliveryPush, ClientError>> {
    try_stream! {
        let base = ws_base(client.server())?;
        let mut seen: HashSet<i64> = HashSet::new();
        let mut backoff = options.reconnect_initial;

        loop {
            // Tickets are single-use — mint a fresh one per connection.
            let ticket = client.create_ws_ticket().await?.ticket;
            let url = format!(
                "{base}/v1/ws/delivery/{}?token={}",
                urlencoding::encode(&recipient),
                urlencoding::encode(&ticket),
            );

            let (mut socket, _resp) = tokio_tungstenite::connect_async(&url)
                .await
                .map_err(|e| ClientError::Ws(format!("connect failed for {url}: {e}")))?;

            // Declare our protocol version; an incompatible server closes
            // with 4426, which we surface as a terminal error below.
            let hello = serde_json::to_string(&ClientMessage::Hello {
                protocol_version: DELIVERY_PROTOCOL_VERSION,
                since_id: None,
            })
            .expect("hello serializes");
            socket
                .send(Message::Text(hello.into()))
                .await
                .map_err(|e| ClientError::Ws(format!("hello send failed: {e}")))?;

            let mut close_code: Option<u16> = None;

            while let Some(msg) = socket.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => {
                        if !options.reconnect {
                            Err(ClientError::Ws(format!("recv error: {e}")))?;
                        }
                        break;
                    }
                };
                match msg {
                    Message::Text(text) => {
                        let frame: ServerMessage = match serde_json::from_str(&text) {
                            Ok(f) => f,
                            Err(_) => continue, // tolerate unknown frames
                        };
                        if let ServerMessage::Push { id, kind, recipient: r, tenant_id, .. } = &frame {
                            let payload = frame
                                .decode_push_payload()
                                .map_err(|e| ClientError::Ws(format!("bad push payload: {e}")))?;
                            let push = DeliveryPush {
                                id: *id,
                                kind: kind.clone(),
                                recipient: r.clone(),
                                tenant_id: tenant_id.clone(),
                                payload,
                            };
                            let fresh = seen.insert(push.id);
                            let ack_id = push.id;
                            if fresh {
                                yield push;
                            }
                            // Ack after yield: a consumer crash before
                            // processing leaves the row unacked.
                            let ack = serde_json::to_string(&ClientMessage::Ack {
                                protocol_version: DELIVERY_PROTOCOL_VERSION,
                                id: ack_id,
                            })
                            .expect("ack serializes");
                            if socket.send(Message::Text(ack.into())).await.is_err() {
                                break;
                            }
                        }
                        backoff = options.reconnect_initial;
                    }
                    Message::Close(frame) => {
                        close_code = frame.map(|f| f.code.into());
                        break;
                    }
                    _ => {}
                }
            }

            // 4426 = unsupported protocol_version — reconnecting cannot help.
            if close_code == Some(4426) {
                Err(ClientError::ProtocolVersion {
                    client_version: DELIVERY_PROTOCOL_VERSION,
                })?;
            }
            if !options.reconnect {
                break;
            }
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(options.reconnect_max);
        }
    }
}

pub(crate) fn follow_execution_events(
    client: Client,
    execution_id: String,
    options: SubscribeOptions,
) -> impl Stream<Item = Result<serde_json::Value, ClientError>> {
    try_stream! {
        let recipient = format!("exec_events:{execution_id}");
        let stream = subscribe_delivery(client, recipient, options);
        let mut stream = std::pin::pin!(stream);
        while let Some(push) = stream.next().await {
            let push = push?;
            let event: serde_json::Value = serde_json::from_slice(&push.payload)
                .map_err(|e| ClientError::Ws(format!("push payload is not JSON: {e}")))?;
            yield event;
        }
    }
}
