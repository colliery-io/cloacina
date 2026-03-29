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

//! Daemon mode — lightweight local scheduler.
//!
//! Watches directories for `.cloacina` packages, loads them via the reconciler,
//! and runs cron + trigger schedules. Uses SQLite for state, filesystem for
//! package storage.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info};

use cloacina::registry::{FilesystemWorkflowRegistry, ReconcilerConfig, RegistryReconciler};
use cloacina::runner::DefaultRunner;

/// Run the daemon.
///
/// This is the main entry point for `cloacinactl daemon`. It:
/// 1. Initializes the `~/.cloacina/` home directory
/// 2. Creates the SQLite database
/// 3. Creates the `DefaultRunner`
/// 4. Creates the `FilesystemWorkflowRegistry` for all watch directories
/// 5. Starts the `RegistryReconciler` (initial scan + background loop)
/// 6. Blocks until Ctrl+C
pub async fn run(home: PathBuf, watch_dirs: Vec<PathBuf>, poll_interval_ms: u64) -> Result<()> {
    // 1. Initialize home directory
    info!("Daemon home: {}", home.display());
    std::fs::create_dir_all(&home)
        .with_context(|| format!("Failed to create daemon home: {}", home.display()))?;

    let packages_dir = home.join("packages");
    std::fs::create_dir_all(&packages_dir)
        .with_context(|| format!("Failed to create packages dir: {}", packages_dir.display()))?;

    // Collect all watch directories (default + user-specified)
    let mut all_watch_dirs = vec![packages_dir.clone()];
    for dir in &watch_dirs {
        if *dir != packages_dir {
            all_watch_dirs.push(dir.clone());
        }
    }

    info!(
        "Watch directories: {:?}",
        all_watch_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
    );

    // 2. Create/open SQLite database
    let db_path = home.join("cloacina.db");
    let db_url = format!("sqlite://{}?mode=rwc&_journal_mode=WAL", db_path.display());
    info!("Database: {}", db_path.display());

    // 3. Create DefaultRunner with SQLite backend
    let runner = DefaultRunner::new(&db_url)
        .await
        .context("Failed to create DefaultRunner")?;
    info!("DefaultRunner initialized with SQLite backend");

    // 4. Create FilesystemWorkflowRegistry
    let registry = Arc::new(FilesystemWorkflowRegistry::new(all_watch_dirs.clone()));

    // 5. Create shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // 6. Create and start RegistryReconciler
    let reconciler_config = ReconcilerConfig {
        reconcile_interval: std::time::Duration::from_millis(poll_interval_ms),
        ..ReconcilerConfig::default()
    };

    let reconciler = RegistryReconciler::new(registry, reconciler_config, shutdown_rx)
        .context("Failed to create RegistryReconciler")?;

    // Perform initial reconciliation
    info!("Starting initial reconciliation...");
    let reconciler_handle = tokio::spawn(async move {
        if let Err(e) = reconciler.start_reconciliation_loop().await {
            error!("Reconciler loop failed: {}", e);
        }
    });

    info!("");
    info!("Daemon is running.");
    info!("  Home:       {}", home.display());
    info!(
        "  Watching:   {}",
        all_watch_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    info!("  Database:   {}", db_path.display());
    info!("  Poll:       {}ms", poll_interval_ms);
    info!("");
    info!("Drop .cloacina packages into the watch directories to load them.");
    info!("Press Ctrl+C to shut down.");
    info!("");

    // 7. Block until Ctrl+C
    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    info!("");
    info!("Shutting down...");

    // Signal shutdown to all components
    let _ = shutdown_tx.send(true);

    // Wait for reconciler to stop (with timeout)
    let shutdown_timeout = std::time::Duration::from_secs(10);
    match tokio::time::timeout(shutdown_timeout, reconciler_handle).await {
        Ok(Ok(())) => info!("Reconciler stopped cleanly"),
        Ok(Err(e)) => error!("Reconciler task panicked: {}", e),
        Err(_) => error!(
            "Reconciler shutdown timed out after {}s",
            shutdown_timeout.as_secs()
        ),
    }

    // Shutdown the runner
    runner
        .shutdown()
        .await
        .context("Failed to shutdown runner")?;

    info!("Daemon shutdown complete.");
    Ok(())
}
