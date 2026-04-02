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
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cloacina::registry::{
    FilesystemWorkflowRegistry, ReconcileResult, ReconcilerConfig, RegistryReconciler,
};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

use super::config::CloacinaConfig;
use super::watcher::PackageWatcher;

/// Run the daemon.
///
/// This is the main entry point for `cloacinactl daemon`. It:
/// 1. Initializes the `~/.cloacina/` home directory
/// 2. Creates the SQLite database
/// 3. Creates the `DefaultRunner`
/// 4. Creates the `FilesystemWorkflowRegistry` for all watch directories
/// 5. Starts the `RegistryReconciler` (initial scan + background loop)
/// 6. Blocks until Ctrl+C
pub async fn run(
    home: PathBuf,
    watch_dirs: Vec<PathBuf>,
    poll_interval_ms: u64,
    verbose: bool,
) -> Result<()> {
    // 1. Initialize home directory and logging
    std::fs::create_dir_all(&home)
        .with_context(|| format!("Failed to create daemon home: {}", home.display()))?;

    let logs_dir = home.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .with_context(|| format!("Failed to create logs dir: {}", logs_dir.display()))?;

    // Set up dual logging: JSON to file + human-readable to stderr
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let file_appender = rolling::daily(&logs_dir, "cloacina.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(fmt::layer().json().with_writer(non_blocking))
        .init();

    info!("Daemon home: {}", home.display());
    info!("Logging to: {}", logs_dir.display());

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

    // 2. Load config file (if exists) — needed for runner and watcher settings
    let config_path = home.join("config.toml");
    let config = CloacinaConfig::load(&config_path);
    let daemon_cfg = &config.daemon;

    // 3. Create/open SQLite database
    let db_path = home.join("cloacina.db");
    let db_url = format!("sqlite://{}?mode=rwc&_journal_mode=WAL", db_path.display());
    info!("Database: {}", db_path.display());

    // 4. Create DefaultRunner with SQLite backend and configured poll intervals
    let runner_config = DefaultRunnerConfig::builder()
        .cron_poll_interval(Duration::from_millis(poll_interval_ms))
        .cron_max_catchup_executions(daemon_cfg.cron_max_catchup.unwrap_or(u64::MAX) as usize)
        .trigger_base_poll_interval(Duration::from_millis(daemon_cfg.trigger_poll_interval_ms))
        .cron_recovery_interval(Duration::from_secs(daemon_cfg.cron_recovery_interval_s))
        .build();

    let runner = DefaultRunner::with_config(&db_url, runner_config)
        .await
        .context("Failed to create DefaultRunner")?;
    info!("DefaultRunner initialized with SQLite backend");

    // 4. Create FilesystemWorkflowRegistry
    let registry = Arc::new(FilesystemWorkflowRegistry::new(all_watch_dirs.clone()));
    let registry_for_triggers = registry.clone();

    // 5. Create shutdown channel
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    // 6. Create RegistryReconciler (we drive it manually, not via its built-in loop)
    let reconciler_config = ReconcilerConfig {
        reconcile_interval: Duration::from_millis(poll_interval_ms),
        enable_startup_reconciliation: false, // We do it ourselves below
        ..ReconcilerConfig::default()
    };

    let reconciler = RegistryReconciler::new(registry, reconciler_config, shutdown_rx)
        .context("Failed to create RegistryReconciler")?;

    // 7. Perform initial reconciliation
    info!("Running initial reconciliation...");
    match reconciler.reconcile().await {
        Ok(result) => {
            info!(
                "Initial reconciliation: {} loaded, {} unloaded, {} failed",
                result.packages_loaded.len(),
                result.packages_unloaded.len(),
                result.packages_failed.len()
            );
            // Register triggers from newly loaded packages
            register_triggers_from_reconcile(&runner, &registry_for_triggers, &result).await;
        }
        Err(e) => {
            warn!("Initial reconciliation failed: {}", e);
        }
    }

    // 8. Merge config file watch dirs with CLI watch dirs
    let config_watch_dirs = config.resolve_watch_dirs();
    for dir in &config_watch_dirs {
        if !all_watch_dirs.contains(dir) {
            all_watch_dirs.push(dir.clone());
        }
    }

    // 9. Start filesystem watcher
    let debounce = Duration::from_millis(daemon_cfg.watcher_debounce_ms);
    let (mut watcher, mut reconcile_rx) =
        PackageWatcher::new(&all_watch_dirs, debounce).context("Failed to start file watcher")?;

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

    // 10. Set up signal handlers
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("Failed to register SIGTERM handler")?;
    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .context("Failed to register SIGHUP handler")?;

    // Track current watch dirs for diffing on reload
    let mut current_watch_dirs = all_watch_dirs.clone();

    // 11. Event loop: react to filesystem changes or periodic reconciliation
    let poll_interval = Duration::from_millis(poll_interval_ms);
    let mut periodic = tokio::time::interval(poll_interval);
    periodic.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            // Filesystem watcher detected a change
            Some(_signal) = reconcile_rx.recv() => {
                debug!("Filesystem change detected — reconciling");
                match reconciler.reconcile().await {
                    Ok(result) => {
                        if result.has_changes() {
                            info!(
                                "Reconciliation: {} loaded, {} unloaded",
                                result.packages_loaded.len(),
                                result.packages_unloaded.len()
                            );
                            register_triggers_from_reconcile(&runner, &registry_for_triggers, &result).await;
                        }
                        if result.has_failures() {
                            for (id, err) in &result.packages_failed {
                                warn!("Package {} failed: {}", id, err);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Reconciliation failed: {}", e);
                    }
                }
            }

            // Periodic reconciliation as a fallback
            _ = periodic.tick() => {
                debug!("Periodic reconciliation tick");
                match reconciler.reconcile().await {
                    Ok(result) => {
                        if result.has_changes() {
                            info!(
                                "Periodic reconciliation: {} loaded, {} unloaded",
                                result.packages_loaded.len(),
                                result.packages_unloaded.len()
                            );
                            register_triggers_from_reconcile(&runner, &registry_for_triggers, &result).await;
                        }
                    }
                    Err(e) => {
                        error!("Periodic reconciliation failed: {}", e);
                    }
                }
            }

            // SIGINT (Ctrl+C)
            _ = tokio::signal::ctrl_c() => {
                info!("");
                info!("Received SIGINT — shutting down...");
                break;
            }

            // SIGTERM
            _ = sigterm.recv() => {
                info!("Received SIGTERM — shutting down...");
                break;
            }

            // SIGHUP — reload configuration
            _ = sighup.recv() => {
                info!("Received SIGHUP — reloading configuration...");
                let new_config = CloacinaConfig::load(&config_path);
                let new_watch_dirs = {
                    let mut dirs = vec![packages_dir.clone()];
                    // CLI dirs
                    for dir in &watch_dirs {
                        if !dirs.contains(dir) {
                            dirs.push(dir.clone());
                        }
                    }
                    // Config file dirs
                    for dir in new_config.resolve_watch_dirs() {
                        if !dirs.contains(&dir) {
                            dirs.push(dir.clone());
                        }
                    }
                    dirs
                };

                // Diff watch dirs: add new, remove old
                for dir in &new_watch_dirs {
                    if !current_watch_dirs.contains(dir) {
                        if let Err(e) = watcher.watch_dir(dir) {
                            warn!("Failed to watch new directory {}: {}", dir.display(), e);
                        } else {
                            info!("Added watch directory: {}", dir.display());
                        }
                    }
                }
                for dir in &current_watch_dirs {
                    if !new_watch_dirs.contains(dir) {
                        if let Err(e) = watcher.unwatch_dir(dir) {
                            warn!("Failed to unwatch directory {}: {}", dir.display(), e);
                        } else {
                            info!("Removed watch directory: {}", dir.display());
                        }
                    }
                }
                current_watch_dirs = new_watch_dirs;

                // Trigger reconciliation to pick up packages in new dirs
                info!("Triggering reconciliation after config reload...");
                match reconciler.reconcile().await {
                    Ok(result) => {
                        if result.has_changes() {
                            info!(
                                "Post-reload reconciliation: {} loaded, {} unloaded",
                                result.packages_loaded.len(),
                                result.packages_unloaded.len()
                            );
                            register_triggers_from_reconcile(&runner, &registry_for_triggers, &result).await;
                        }
                    }
                    Err(e) => {
                        error!("Post-reload reconciliation failed: {}", e);
                    }
                }

                info!("Configuration reload complete.");
            }
        }
    }

    // Graceful shutdown with timeout and force-exit on second signal
    let shutdown_timeout = Duration::from_secs(daemon_cfg.shutdown_timeout_s);
    info!(
        "Draining in-flight pipelines (timeout: {}s)...",
        shutdown_timeout.as_secs()
    );
    info!("Press Ctrl+C again to force exit immediately.");

    // Race: runner shutdown vs timeout vs second Ctrl+C
    tokio::select! {
        result = runner.shutdown() => {
            match result {
                Ok(()) => info!("All pipelines drained successfully."),
                Err(e) => error!("Runner shutdown error: {}", e),
            }
        }
        _ = tokio::time::sleep(shutdown_timeout) => {
            error!("Shutdown timed out after {}s — forcing exit.", shutdown_timeout.as_secs());
        }
        _ = tokio::signal::ctrl_c() => {
            warn!("Second SIGINT received — forcing immediate exit.");
            std::process::exit(1);
        }
    }

    info!("Daemon shutdown complete.");
    Ok(())
}

/// After reconciliation loads new packages, register their triggers with the
/// trigger scheduler so they get polled and create TriggerSchedule DB records.
async fn register_triggers_from_reconcile(
    runner: &DefaultRunner,
    registry: &Arc<FilesystemWorkflowRegistry>,
    result: &ReconcileResult,
) {
    use cloacina::registry::traits::WorkflowRegistry;

    if result.packages_loaded.is_empty() {
        return;
    }

    let scheduler = match runner.unified_scheduler().await {
        Some(s) => s,
        None => {
            debug!("Unified scheduler not available — skipping trigger registration");
            return;
        }
    };

    // For each newly loaded package, check if it has triggers in its manifest
    let workflows = match registry.list_workflows().await {
        Ok(w) => w,
        Err(e) => {
            warn!("Failed to list workflows for trigger registration: {}", e);
            return;
        }
    };

    for package_id in &result.packages_loaded {
        // Find the workflow metadata for this package
        let metadata = match workflows.iter().find(|w| w.id == *package_id) {
            Some(m) => m,
            None => continue,
        };

        // Load the package data to read the manifest for trigger definitions
        let loaded = match registry
            .get_workflow(&metadata.package_name, &metadata.version)
            .await
        {
            Ok(Some(l)) => l,
            _ => continue,
        };

        // Unpack the source archive to a temp dir and read package.toml
        let tmp = match tempfile::TempDir::new() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let archive_path = tmp.path().join("pkg.cloacina");
        if std::fs::write(&archive_path, &loaded.package_data).is_err() {
            continue;
        }
        let extract_dir = tmp.path().join("source");
        if std::fs::create_dir_all(&extract_dir).is_err() {
            continue;
        }
        let source_dir = match fidius_core::package::unpack_package(&archive_path, &extract_dir) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let cloacina_manifest = match fidius_core::package::load_manifest::<
            cloacina_workflow_plugin::CloacinaMetadata,
        >(&source_dir)
        {
            Ok(m) => m,
            Err(_) => continue,
        };

        for trigger_def in &cloacina_manifest.metadata.triggers {
            if let Some(cron_expr) = &trigger_def.cron_expression {
                // Cron trigger — register via the unified schedule API
                match runner
                    .register_cron_workflow(&trigger_def.workflow, cron_expr, "UTC")
                    .await
                {
                    Ok(schedule_id) => {
                        info!(
                            "Registered cron schedule: '{}' -> workflow '{}' (cron: {}, id: {})",
                            trigger_def.name, trigger_def.workflow, cron_expr, schedule_id
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to create cron schedule for '{}': {}",
                            trigger_def.name, e
                        );
                    }
                }
            } else {
                // Custom poll trigger — look for registered Trigger impl
                if let Some(trigger) = cloacina::trigger::get_trigger(&trigger_def.name) {
                    match scheduler
                        .register_trigger(trigger.as_ref(), &trigger_def.workflow)
                        .await
                    {
                        Ok(_schedule) => {
                            info!(
                                "Registered trigger schedule: '{}' -> workflow '{}' (poll: {})",
                                trigger_def.name, trigger_def.workflow, trigger_def.poll_interval
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Failed to register trigger schedule for '{}': {}",
                                trigger_def.name, e
                            );
                        }
                    }
                } else {
                    warn!(
                        "Trigger '{}' declared in package.toml but no Trigger impl found in registry",
                        trigger_def.name
                    );
                }
            }
        }
    }
}
