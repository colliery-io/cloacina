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

    /// 401 / 403 — missing, invalid, or under-privileged API key, or a
    /// refusal such as the signature check of an upload.
    #[error("authentication: {0}")]
    Auth(ApiErrorDetail),

    /// 404 — the addressed resource does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// 400 / 422 — the server rejected the request as malformed.
    #[error("invalid request: {0}")]
    InvalidRequest(ApiErrorDetail),

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
            401 | 403 => ClientError::Auth(ApiErrorDetail::from_body(&body)),
            404 => ClientError::NotFound(extract_message(&body)),
            400 | 422 => ClientError::InvalidRequest(ApiErrorDetail::from_body(&body)),
            _ => ClientError::Server { status, body },
        }
    }

    /// Machine-readable `code` from the canonical error body, when present:
    /// for `Auth`, `InvalidRequest` and `Server`.
    pub fn code(&self) -> Option<&str> {
        match self {
            ClientError::Server { body, .. } => body.get("code").and_then(|c| c.as_str()),
            ClientError::Auth(detail) | ClientError::InvalidRequest(detail) => {
                detail.code.as_deref()
            }
            _ => None,
        }
    }
}

/// The error envelope of a 400, 401, 403 or 422 (CLOACINA-T-1141): the
/// server's message and its machine-readable `code`. It displays as the
/// message. Read the code to tell refusals apart; the message text can
/// change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiErrorDetail {
    /// The human-readable message (`error` of the body), or the raw body
    /// when it has no string `error`.
    pub message: String,
    /// The `code` of the body, when it has a string one.
    pub code: Option<String>,
}

impl ApiErrorDetail {
    /// Read the canonical `{error, code}` body.
    pub fn from_body(body: &Value) -> Self {
        Self {
            message: extract_message(body),
            code: body.get("code").and_then(|c| c.as_str()).map(String::from),
        }
    }

    /// The message.
    pub fn as_str(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ApiErrorDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// CLOACINA-T-1141: a 400 and a 403 keep the envelope's code and message.
    #[test]
    fn a_400_and_a_403_keep_the_code_and_the_message() {
        let refused = ClientError::from_status(
            403,
            serde_json::json!({"code": "package_tampered", "error": "package contents do not match the signed hash"}),
        );
        assert_eq!(refused.code(), Some("package_tampered"));
        let ClientError::Auth(detail) = &refused else {
            panic!("a 403 is Auth: {refused:?}");
        };
        assert_eq!(
            detail.message,
            "package contents do not match the signed hash"
        );
        assert_eq!(
            refused.to_string(),
            "authentication: package contents do not match the signed hash"
        );

        let malformed = ClientError::from_status(
            400,
            serde_json::json!({"code": "malformed_signature", "error": "the 'signature' field is not a signature"}),
        );
        assert_eq!(malformed.code(), Some("malformed_signature"));
        let ClientError::InvalidRequest(detail) = &malformed else {
            panic!("a 400 is InvalidRequest: {malformed:?}");
        };
        assert_eq!(detail.as_str(), "the 'signature' field is not a signature");
        assert_eq!(
            malformed.to_string(),
            "invalid request: the 'signature' field is not a signature"
        );
    }

    /// A body with no code gives no code; one with no string `error` keeps
    /// its raw text as the message.
    #[test]
    fn a_body_without_a_code_gives_none() {
        let err = ClientError::from_status(401, serde_json::json!({"error": "no key"}));
        assert_eq!(err.code(), None);
        assert_eq!(err.to_string(), "authentication: no key");

        let err = ClientError::from_status(422, serde_json::json!({"detail": "x"}));
        assert_eq!(err.code(), None);
        let ClientError::InvalidRequest(detail) = &err else {
            panic!("a 422 is InvalidRequest: {err:?}");
        };
        assert_eq!(detail.message, r#"{"detail":"x"}"#);

        let err = ClientError::from_status(404, serde_json::json!({"code": "x", "error": "y"}));
        assert_eq!(err.code(), None, "a 404 keeps the message only");
    }
}
