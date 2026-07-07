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

//! cloacina-compiler library — entrypoint `run()` exposed so integration tests
//! and the binary main both share the same code path.

mod build;
mod config;
mod doc_parse;
mod health;
mod loopp;
mod param_parse;
pub mod sandbox;

pub use config::{BuildRlimits, CompilerConfig};

use std::sync::Arc;

use anyhow::{Context, Result};
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::database::Database;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Start the compiler service. Blocks until SIGINT / SIGTERM.
pub async fn run(config: CompilerConfig) -> Result<()> {
    let log_guard = install_logging(&config)?;

    // Install the Prometheus recorder before any metric emit fires.
    // Shared `/metrics` endpoint piggybacks on the existing health
    // listener (no separate port). CLOACI-I-0109 / T-0591.
    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .context("failed to install Prometheus metrics recorder")?;
    register_compiler_metrics();

    info!(
        bind = %config.bind,
        poll_interval = ?config.poll_interval,
        heartbeat_interval = ?config.heartbeat_interval,
        stale_threshold = ?config.stale_threshold,
        sweep_interval = ?config.sweep_interval,
        compiler_instance_id = %config.compiler_instance_id,
        "cloacina-compiler starting"
    );

    // CLOACI-T-0575: rlimits are Linux-only kernel-enforced. On non-Linux
    // dev hosts we still build and run, but the resource ceiling is
    // unenforced; warn once so the operator can't miss it.
    #[cfg(not(target_os = "linux"))]
    tracing::warn!(
        "non-Linux build host: --build-rlimit-* values are ignored \
         (setrlimit hook is Linux-only per ADR-0005). The wall-clock \
         timeout from --build-timeout-s is the only resource bound."
    );

    // Registry is shared between the build loop and the /v1/status endpoint.
    // CLOACI-T-0779: a tenant-scoped compiler binds to that tenant's Postgres
    // schema (build isolation — it claims/builds ONLY that tenant's packages).
    let database = match &config.tenant_schema {
        Some(schema) => {
            info!(tenant_schema = %schema, "compiler scoped to tenant schema (build isolation)");
            Database::try_new_with_schema(&config.database_url, "", 5, Some(schema))
                .map_err(|e| anyhow::anyhow!("failed to open schema for tenant '{schema}': {e}"))?
        }
        None => Database::new(&config.database_url, "", 5),
    };
    // The public/admin schema owns its migrations; tenant schemas are migrated by
    // the server at tenant creation, so a tenant-scoped compiler must NOT migrate
    // (avoids racing the server and confining each compiler to its own schema).
    if config.tenant_schema.is_none() {
        database
            .run_migrations()
            .await
            .map_err(|e| anyhow::anyhow!("migration failed: {e}"))?;
    }
    let storage = UnifiedRegistryStorage::new(database.clone());
    // CLOACI-T-0780: a per-target compiler needs a DAL handle for the scan-and-fill
    // (the registry move below consumes `database`).
    let db_for_target = database.clone();
    let registry = Arc::new(
        WorkflowRegistryImpl::new(storage, database)
            .map_err(|e| anyhow::anyhow!("failed to construct workflow registry: {e}"))?,
    );

    let shutdown = CancellationToken::new();
    let signal_shutdown = shutdown.clone();
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            info!("SIGINT — shutting down");
            signal_shutdown.cancel();
        }
    });

    // Local HTTP endpoint for status / health / metrics probes —
    // cloacinactl compiler status / health talk to this, Prometheus
    // scrapes /metrics from the same listener (T-0591).
    let http_shutdown = shutdown.clone();
    let http_registry = Arc::clone(&registry);
    let http_handle = tokio::spawn(health::serve(
        config.bind,
        http_registry,
        metrics_handle,
        http_shutdown,
    ));

    // Main loop: per-target scan-and-fill (CLOACI-T-0780) when --build-target is
    // set, else the primary claim → build → mark queue + sweeper loop.
    if config.build_target.is_some() {
        loopp::run_per_target(registry, db_for_target, config, shutdown.clone()).await?;
    } else {
        loopp::run(registry, config, shutdown.clone()).await?;
    }

    shutdown.cancel();
    if let Err(e) = http_handle.await {
        tracing::warn!(%e, "http task exited with error");
    }

    drop(log_guard);
    Ok(())
}

/// Register HELP/TYPE for every `cloacina_compiler_*` metric so promtool
/// validates a complete exposition. Emit sites live in `loopp.rs` and
/// `build.rs`. See CLOACI-I-0109 / T-0591.
fn register_compiler_metrics() {
    metrics::describe_counter!(
        "cloacina_compiler_builds_total",
        "Total cargo builds executed by the compiler. `status` ∈ \
         `ok`, `failed`, `timed_out`. Timeouts leave the row for the \
         stale-build sweeper rather than marking it terminally failed."
    );
    metrics::describe_histogram!(
        "cloacina_compiler_build_duration_seconds",
        "Wall-clock duration of `execute_build` — covers the cargo \
         subprocess from spawn through artifact persistence. \
         Independent of the result status."
    );
    metrics::describe_gauge!(
        "cloacina_compiler_queue_depth",
        "Build queue size by state. `state` ∈ `queued`, `building`. \
         SQL-derived — re-seeded every sweep tick from `compiled_data` \
         row counts (REC-06 pattern), so it cannot drift on crash."
    );
    metrics::describe_counter!(
        "cloacina_compiler_sweep_resets_total",
        "Stale builds reset to `pending` by the sweeper — one increment \
         per row reclaimed (paired with T-0522). Sustained non-zero \
         rate indicates worker crashes or hung builds."
    );
    metrics::describe_counter!(
        "cloacina_compiler_heartbeat_failures_total",
        "Heartbeat-update failures from the builder. Repeated failures \
         starve the row's `build_claimed_at` field and the sweeper will \
         eventually reclaim it."
    );
}

fn install_logging(config: &CompilerConfig) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let logs_dir = config.home.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("failed to create logs dir {}", logs_dir.display()))?;

    // Daily-rotated file appender with optional retention via
    // `max_log_files`. `log_retention_days == 0` disables pruning per
    // operator opt-out; otherwise the appender keeps the most recent N
    // files and prunes the oldest automatically. CLOACI-T-0592.
    let mut builder = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("compiler")
        .filename_suffix("log");
    if config.log_retention_days > 0 {
        builder = builder.max_log_files(config.log_retention_days as usize);
    }
    let file_appender = builder.build(&logs_dir).with_context(|| {
        format!(
            "failed to build rolling log appender in {}",
            logs_dir.display()
        )
    })?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = if config.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);
    let stderr_layer = fmt::layer().with_writer(std::io::stderr);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();

    Ok(guard)
}
