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

//! Error types and exit-code mapping per ADR-0003.

use std::fmt;

/// Typed CLI errors. Each variant maps deterministically to an exit code.
#[derive(Debug)]
pub enum CliError {
    /// 1 — user-facing mistake (bad flags, validation, malformed input).
    UserError(String),
    /// 2 — network / server unreachable / transport failure.
    Network(String),
    /// 3 — resource not found.
    NotFound { resource: String, key: String },
    /// 4 — auth failure (401/403).
    Auth(String),
    /// 5 — server rejected the operation (business-logic 4xx/5xx).
    ServerReject {
        status: u16,
        body: serde_json::Value,
    },
    /// 1 — underlying IO error surfaced to user.
    Io(std::io::Error),
    /// Raw wrapping for unclassified errors (maps to exit 1).
    Other(anyhow::Error),
}

impl CliError {
    /// Exit code for this error, per ADR-0003 §6.
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::UserError(_) => 1,
            CliError::Network(_) => 2,
            CliError::NotFound { .. } => 3,
            CliError::Auth(_) => 4,
            CliError::ServerReject { .. } => 5,
            CliError::Io(_) => 1,
            CliError::Other(_) => 1,
        }
    }

    /// Build a `CliError` from a reqwest error. Transport errors → Network.
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        CliError::Network(err.to_string())
    }

    /// Build a `CliError` from an HTTP response status + body.
    pub fn from_status(status: u16, body: serde_json::Value) -> Self {
        match status {
            401 | 403 => CliError::Auth(extract_message(&body)),
            404 => CliError::NotFound {
                resource: "resource".to_string(),
                key: extract_message(&body),
            },
            400 | 422 => CliError::UserError(extract_message(&body)),
            _ => CliError::ServerReject { status, body },
        }
    }
}

fn extract_message(body: &serde_json::Value) -> String {
    body.get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            body.get("message")
                .and_then(|m| m.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| body.to_string())
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::UserError(msg) => write!(f, "Error: {msg}"),
            CliError::Network(msg) => write!(f, "Error: network — {msg}"),
            CliError::NotFound { resource, key } => {
                write!(f, "Error: not found — {resource} '{key}'")
            }
            CliError::Auth(msg) => write!(f, "Error: authentication — {msg}"),
            CliError::ServerReject { status, body } => {
                write!(
                    f,
                    "Error: server (HTTP {status}) — {}",
                    extract_message(body)
                )
            }
            CliError::Io(e) => write!(f, "Error: io — {e}"),
            CliError::Other(e) => write!(f, "Error: {e:#}"),
        }
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

impl From<reqwest::Error> for CliError {
    fn from(e: reqwest::Error) -> Self {
        CliError::from_reqwest(e)
    }
}

impl From<anyhow::Error> for CliError {
    fn from(e: anyhow::Error) -> Self {
        CliError::Other(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_adr() {
        assert_eq!(CliError::UserError("x".into()).exit_code(), 1);
        assert_eq!(CliError::Network("x".into()).exit_code(), 2);
        assert_eq!(
            CliError::NotFound {
                resource: "r".into(),
                key: "k".into()
            }
            .exit_code(),
            3
        );
        assert_eq!(CliError::Auth("x".into()).exit_code(), 4);
        assert_eq!(
            CliError::ServerReject {
                status: 500,
                body: serde_json::Value::Null,
            }
            .exit_code(),
            5
        );
    }

    #[test]
    fn from_status_maps_correctly() {
        assert!(matches!(
            CliError::from_status(401, serde_json::Value::Null),
            CliError::Auth(_)
        ));
        assert!(matches!(
            CliError::from_status(404, serde_json::Value::Null),
            CliError::NotFound { .. }
        ));
        assert!(matches!(
            CliError::from_status(400, serde_json::Value::Null),
            CliError::UserError(_)
        ));
        assert!(matches!(
            CliError::from_status(500, serde_json::Value::Null),
            CliError::ServerReject { .. }
        ));
    }

    #[test]
    fn message_extraction_prefers_structured_error() {
        let body = serde_json::json!({"error": {"message": "bad input"}});
        let err = CliError::from_status(400, body);
        assert!(err.to_string().contains("bad input"));
    }
}
