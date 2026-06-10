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

//! Client error model — generalized from `cloacinactl`'s `CliError`
//! (T-0646). The CLI maps these back onto its ADR-0003 exit codes.

use serde_json::Value;

/// Errors from the cloacina-server client.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Transport-level failure: connect, TLS, timeout, DNS.
    #[error("network: {0}")]
    Transport(String),

    /// 401 / 403 — missing, invalid, or under-privileged API key.
    #[error("authentication: {0}")]
    Auth(String),

    /// 404 — the addressed resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// 400 / 422 — the server rejected the request as malformed.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Any other non-2xx — business-logic rejection or server fault.
    /// `body` is the raw response JSON (canonically `{error, code}`).
    #[error("server rejected (HTTP {status}): {}", extract_message(.body))]
    Server { status: u16, body: Value },

    /// Configuration problems (profile resolution, key schemes).
    #[error("configuration: {0}")]
    Config(String),

    /// WebSocket / delivery-protocol failures.
    #[error("websocket: {0}")]
    Ws(String),

    /// The server rejected our delivery protocol version (close 4426).
    /// Reconnecting cannot help — upgrade the client.
    #[error("server does not speak delivery protocol v{client_version} (close 4426)")]
    ProtocolVersion { client_version: u32 },
}

impl ClientError {
    pub(crate) fn from_reqwest(err: reqwest::Error) -> Self {
        ClientError::Transport(err.to_string())
    }

    /// Map an HTTP status + canonical `{error, code}` body to a variant.
    pub fn from_status(status: u16, body: Value) -> Self {
        match status {
            401 | 403 => ClientError::Auth(extract_message(&body)),
            404 => ClientError::NotFound(extract_message(&body)),
            400 | 422 => ClientError::InvalidRequest(extract_message(&body)),
            _ => ClientError::Server { status, body },
        }
    }

    /// Machine-readable `code` from the canonical error body, when present.
    pub fn code(&self) -> Option<&str> {
        match self {
            ClientError::Server { body, .. } => body.get("code").and_then(|c| c.as_str()),
            _ => None,
        }
    }
}

/// CLOACI-T-0595 / API-06: the canonical `ApiError` envelope is
/// `{ code, error }`; `error` carries the human-readable message. A body
/// without a string `error` renders raw so the unexpected shape is visible.
pub(crate) fn extract_message(body: &Value) -> String {
    body.get("error")
        .and_then(|m| m.as_str())
        .map(String::from)
        .unwrap_or_else(|| body.to_string())
}
