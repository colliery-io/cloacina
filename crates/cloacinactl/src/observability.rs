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

//! Observability setup: Prometheus metrics recorder and OpenTelemetry tracing stubs.

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;
use tracing::info;

static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Initialize the Prometheus metrics recorder.
/// Call once at startup before any metrics are recorded.
/// Returns `Some(handle)` on success, `None` if already initialized.
pub fn init_prometheus() -> Option<PrometheusHandle> {
    let builder = PrometheusBuilder::new();
    match builder.install_recorder() {
        Ok(handle) => {
            let cloned = handle.clone();
            // If another thread beat us, that's fine — the OnceLock keeps the first.
            PROMETHEUS_HANDLE.set(handle).ok();
            Some(cloned)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to install Prometheus recorder (may already be installed): {}",
                e
            );
            None
        }
    }
}

/// Get the Prometheus handle for rendering metrics.
pub fn prometheus_handle() -> Option<&'static PrometheusHandle> {
    PROMETHEUS_HANDLE.get()
}

/// Record some initial/static metrics at startup.
pub fn record_static_metrics(max_concurrent_tasks: usize) {
    metrics::gauge!("cloacina_workers_capacity").set(max_concurrent_tasks as f64);
}

/// Initialize OpenTelemetry tracing (stub).
///
/// Currently logs the configured endpoint. Actual OTLP exporter integration
/// is deferred until the OpenTelemetry Rust crate ecosystem stabilizes.
///
/// TODO: Add `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`,
/// and `tracing-opentelemetry` dependencies and wire up the OTLP exporter.
pub fn init_opentelemetry(endpoint: &str, service_name: &str) {
    if endpoint.is_empty() {
        return;
    }
    info!(
        endpoint = %endpoint,
        service_name = %service_name,
        "OpenTelemetry OTLP endpoint configured (stub — exporter not yet wired)"
    );
}
