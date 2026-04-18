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

mod config;
mod health;
mod loopp;

pub use config::CompilerConfig;

use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Start the compiler service. Blocks until SIGINT / SIGTERM.
pub async fn run(config: CompilerConfig) -> Result<()> {
    let log_guard = install_logging(&config)?;

    info!(
        bind = %config.bind,
        poll_interval = ?config.poll_interval,
        heartbeat_interval = ?config.heartbeat_interval,
        stale_threshold = ?config.stale_threshold,
        sweep_interval = ?config.sweep_interval,
        "cloacina-compiler starting"
    );

    let shutdown = CancellationToken::new();
    let signal_shutdown = shutdown.clone();
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            info!("SIGINT — shutting down");
            signal_shutdown.cancel();
        }
    });

    // Local HTTP endpoint for status / health probes (T-0525 will consume).
    let http_shutdown = shutdown.clone();
    let http_handle = tokio::spawn(health::serve(config.bind, http_shutdown));

    // Main queue + sweeper loop (T-0521 / T-0522 fill in the real build logic).
    loopp::run(config, shutdown.clone()).await?;

    shutdown.cancel();
    if let Err(e) = http_handle.await {
        tracing::warn!(%e, "http task exited with error");
    }

    drop(log_guard);
    Ok(())
}

fn install_logging(config: &CompilerConfig) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let logs_dir = config.home.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("failed to create logs dir {}", logs_dir.display()))?;

    let file_appender = rolling::daily(&logs_dir, "compiler.log");
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
