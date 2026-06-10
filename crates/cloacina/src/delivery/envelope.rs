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

//! Substrate WS envelope — re-exported from `cloacina-api-types`.
//!
//! The envelope types moved to `cloacina-api-types` in T-0642 (CLOACI-I-0113)
//! so SDK clients can consume them without linking the engine. This module
//! keeps the original `cloacina::delivery::envelope` paths working and holds
//! the one helper that needs the diesel-backed outbox model.

pub use cloacina_api_types::delivery::{
    ClientMessage, EnvelopeError, ServerMessage, DELIVERY_PROTOCOL_VERSION,
};

use crate::models::delivery_outbox::DeliveryOutbox;

/// Build a `push` frame from an outbox row, base64-encoding the payload.
///
/// (Was `ServerMessage::push_from_row` before the type moved crates — an
/// inherent impl is no longer possible on the foreign type.)
pub fn push_from_row(row: &DeliveryOutbox) -> ServerMessage {
    ServerMessage::push(
        row.id,
        &row.kind,
        &row.recipient,
        row.tenant_id.clone(),
        &row.payload,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::universal_types::UniversalTimestamp;

    #[test]
    fn push_from_row_round_trips_payload() {
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
        let msg = push_from_row(&row);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"push\""));
        let back: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.decode_push_payload().unwrap(), row.payload);
    }
}
