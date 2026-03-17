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

//! Prometheus metrics endpoint.

use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

/// GET /metrics — Prometheus-compatible scrape endpoint.
/// Public (no auth required).
pub async fn metrics() -> impl IntoResponse {
    let handle = match crate::observability::prometheus_handle() {
        Some(h) => h,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "Metrics not configured").into_response();
        }
    };

    let metrics_text = handle.render();
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics_text,
    )
        .into_response()
}
