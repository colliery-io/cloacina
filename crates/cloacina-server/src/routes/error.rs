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

//! Standardized API error responses.
//!
//! All REST API errors use [`ApiError`] for consistent, machine-readable
//! error responses with request correlation IDs.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Standardized API error response.
///
/// All error responses follow this format:
/// ```json
/// {
///   "error": "human-readable message",
///   "code": "machine_readable_code"
/// }
/// ```
///
/// The request ID is propagated via the `x-request-id` response header
/// (set by the request_id_middleware), not via the response body.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    /// Create a new API error.
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    // -- Common error constructors --

    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    /// 409 — the request conflicts with existing state, e.g. creating a named
    /// workflow instance whose name is already taken (CLOACI-T-0894).
    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }

    /// 500 for a failure whose detail must not reach the caller: the text of
    /// a database, pool or other internal error (CLOACINA-T-1035).
    ///
    /// The body holds `what` (a fixed text that names the operation) and
    /// nothing of `detail`. `detail` is logged at error level inside the
    /// request span, so the log line carries the request id that the
    /// `x-request-id` response header gives the caller.
    pub fn internal_logged(what: &str, detail: impl std::fmt::Display) -> Self {
        tracing::error!(error = %detail, "{what}");
        Self::internal(internal_message(what))
    }
}

/// The body text of [`ApiError::internal_logged`].
pub(crate) fn internal_message(what: &str) -> String {
    format!(
        "internal error: {what}; the server log has the detail under this response's x-request-id"
    )
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = cloacina_api_types::ErrorBody {
            error: self.message,
            code: self.code.to_string(),
        };
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CLOACINA-T-1035: the text of the cause never reaches the body.
    #[test]
    fn internal_logged_keeps_the_detail_out_of_the_body() {
        let detail = "duplicate key value violates unique constraint \"api_keys_pkey\"";
        let e = ApiError::internal_logged("failed to list tenants", detail);
        assert_eq!(e.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.code, "internal_error");
        assert!(e.message.contains("failed to list tenants"));
        assert!(e.message.contains("x-request-id"));
        assert!(!e.message.contains("duplicate key"));
        assert!(!e.message.contains("api_keys_pkey"));
    }
}
