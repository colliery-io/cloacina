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

//! Shared WebSocket envelope for the interservice communication substrate
//! (CLOACI-I-0115 / S-0012, task T-0627).
//!
//! Replaces the bespoke per-endpoint framing in `cloacina-server/src/routes/ws.rs`
//! with a single versioned envelope consumed by every substrate WS consumer
//! (CLI execution-events in T-0629, fleet agent protocol in T-0631 / I-0114).
//!
//! ## Wire format
//!
//! JSON text frames in both directions; payloads are **base64-encoded bytes**
//! inside the JSON. The envelope is a serde-tagged enum (`type` field):
//!
//! - Server → recipient: [`ServerMessage`] (`welcome`, `push`).
//! - Recipient → server: [`ClientMessage`] (`hello`, `ack`).
//!
//! ## Idempotency contract
//!
//! Delivery is **at-least-once** (NFR-1.1.2): a recipient may receive the same
//! `push` more than once across disconnect/reconnect cycles. Recipients must
//! be idempotent on row `id`.
//!
//! ## Resync model
//!
//! On reconnect, the server resets every `delivered`-state row for the
//! authenticated `(recipient, tenant)` back to `pending` and wakes the relay,
//! which re-pushes them through the normal sink path. No separate resync
//! frames are needed — the recipient just sees a sequence of `push` messages
//! after `welcome`. The `Hello.since_id` field is advisory (an optimization
//! hint for future cursor-based catch-up; v1 ignores it).

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Wire-protocol version for the substrate envelope. Bumped on
/// backwards-incompatible changes; carried on every frame so peers can
/// negotiate or refuse.
pub const DELIVERY_PROTOCOL_VERSION: u32 = 1;

/// Server → recipient frames.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Sent first on a new connection. Echoes the negotiated protocol version
    /// and (informationally) the highest row id known to the server at
    /// connect time so the recipient can size its dedup window.
    Welcome {
        protocol_version: u32,
        max_known_id: i64,
    },
    /// A delivery-outbox row addressed to this recipient. `payload_b64` is
    /// base64-encoded raw bytes from `delivery_outbox.payload`.
    Push {
        protocol_version: u32,
        id: i64,
        kind: String,
        recipient: String,
        tenant_id: Option<String>,
        payload_b64: String,
    },
}

impl ServerMessage {
    /// Helper: build a `push` from an outbox row, base64-encoding the payload.
    pub fn push_from_row(row: &crate::models::delivery_outbox::DeliveryOutbox) -> Self {
        ServerMessage::Push {
            protocol_version: DELIVERY_PROTOCOL_VERSION,
            id: row.id,
            kind: row.kind.clone(),
            recipient: row.recipient.clone(),
            tenant_id: row.tenant_id.clone(),
            payload_b64: BASE64.encode(&row.payload),
        }
    }

    /// Decode the `payload_b64` of a [`ServerMessage::Push`].
    /// Returns `Err` for non-`Push` variants or invalid base64.
    pub fn decode_push_payload(&self) -> Result<Vec<u8>, EnvelopeError> {
        match self {
            ServerMessage::Push { payload_b64, .. } => BASE64
                .decode(payload_b64)
                .map_err(|e| EnvelopeError::Base64(e.to_string())),
            _ => Err(EnvelopeError::WrongVariant("Push")),
        }
    }
}

/// Recipient → server frames.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Optional connect-time advisory. `since_id` is a cursor hint the server
    /// may use to skip already-acked rows in a future cursor-based catch-up;
    /// v1 ignores it (the server resets `delivered` → `pending` and replays
    /// via the relay).
    Hello {
        protocol_version: u32,
        since_id: Option<i64>,
    },
    /// Recipient confirms it has processed row `id`. Idempotent.
    Ack { protocol_version: u32, id: i64 },
}

/// Errors decoding/encoding substrate envelope frames.
#[derive(Debug, thiserror::Error)]
pub enum EnvelopeError {
    #[error("expected envelope variant {0}")]
    WrongVariant(&'static str),
    #[error("invalid base64 payload: {0}")]
    Base64(String),
    #[error("invalid JSON envelope: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported protocol_version: got {got}, supports {supported}")]
    UnsupportedVersion { got: u32, supported: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::UniversalTimestamp;
    use crate::models::delivery_outbox::DeliveryOutbox;

    #[test]
    fn welcome_round_trips_as_json() {
        let msg = ServerMessage::Welcome {
            protocol_version: DELIVERY_PROTOCOL_VERSION,
            max_known_id: 42,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"welcome\""));
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back, msg);
    }

    #[test]
    fn push_round_trips_with_base64_payload() {
        let row = DeliveryOutbox {
            id: 7,
            recipient: "agent:abc".to_string(),
            kind: "work".to_string(),
            tenant_id: Some("t1".to_string()),
            payload: b"\x00\xffhello".to_vec(),
            delivery_state: "pending".to_string(),
            delivery_attempts: 0,
            created_at: UniversalTimestamp::now(),
            delivered_at: None,
            acked_at: None,
        };
        let msg = ServerMessage::push_from_row(&row);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"push\""));
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.decode_push_payload().unwrap(), row.payload);
    }

    #[test]
    fn ack_and_hello_round_trip() {
        let ack = ClientMessage::Ack {
            protocol_version: DELIVERY_PROTOCOL_VERSION,
            id: 99,
        };
        let json = serde_json::to_string(&ack).unwrap();
        assert!(json.contains("\"type\":\"ack\""));
        assert_eq!(serde_json::from_str::<ClientMessage>(&json).unwrap(), ack);

        let hello = ClientMessage::Hello {
            protocol_version: DELIVERY_PROTOCOL_VERSION,
            since_id: Some(10),
        };
        let json = serde_json::to_string(&hello).unwrap();
        assert!(json.contains("\"type\":\"hello\""));
        assert_eq!(serde_json::from_str::<ClientMessage>(&json).unwrap(), hello);

        // `since_id: None` should serialize as `null` and round-trip.
        let hello_none = ClientMessage::Hello {
            protocol_version: DELIVERY_PROTOCOL_VERSION,
            since_id: None,
        };
        let json = serde_json::to_string(&hello_none).unwrap();
        assert_eq!(
            serde_json::from_str::<ClientMessage>(&json).unwrap(),
            hello_none
        );
    }

    #[test]
    fn decode_push_payload_rejects_wrong_variant() {
        let msg = ServerMessage::Welcome {
            protocol_version: DELIVERY_PROTOCOL_VERSION,
            max_known_id: 0,
        };
        assert!(matches!(
            msg.decode_push_payload(),
            Err(EnvelopeError::WrongVariant("Push"))
        ));
    }
}
