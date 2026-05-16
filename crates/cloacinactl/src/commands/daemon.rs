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
//! and registers polling-scheduler entries for FFI-declared custom-poll
//! triggers. Cron-shaped triggers are registered by
//! `RegistryReconciler::step_load_cron_triggers` at load time (T-0553), so
//! this module only handles the non-cron arm. Uses SQLite for state and the
//! filesystem for package storage.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use cloacina::registry::{
    FilesystemWorkflowRegistry, ReconcileResult, ReconcilerConfig, RegistryReconciler,
};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};

use super::config::CloacinaConfig;
use super::watcher::PackageWatcher;

/// Merge watch directories from multiple sources, deduplicating.
///
/// Priority: packages_dir (always first), then CLI dirs, then config dirs.
pub(crate) fn collect_watch_dirs(
    packages_dir: &Path,
    cli_dirs: &[PathBuf],
    config_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    let mut dirs = vec![packages_dir.to_path_buf()];
    for dir in cli_dirs.iter().chain(config_dirs.iter()) {
        if !dirs.contains(dir) {
            dirs.push(dir.clone());
        }
    }
    dirs
}

/// Diff watch directories and apply changes to the watcher.
///
/// Returns the new set of watch directories. Logs warnings on failure
/// but never errors — the daemon keeps running.
pub(crate) fn apply_watch_dir_changes(
    watcher: &mut PackageWatcher,
    current: &[PathBuf],
    new: &[PathBuf],
) {
    for dir in new {
        if !current.contains(dir) {
            if let Err(e) = watcher.watch_dir(dir) {
                warn!("Failed to watch new directory {}: {}", dir.display(), e);
            } else {
                info!("Added watch directory: {}", dir.display());
            }
        }
    }
    for dir in current {
        if !new.contains(dir) {
            if let Err(e) = watcher.unwatch_dir(dir) {
                warn!("Failed to unwatch directory {}: {}", dir.display(), e);
            } else {
                info!("Removed watch directory: {}", dir.display());
            }
        }
    }
}

/// Handle a reconciliation result: log changes/failures and register triggers.
pub(crate) async fn handle_reconcile(
    runner: &DefaultRunner,
    registry: &Arc<FilesystemWorkflowRegistry>,
    result: &ReconcileResult,
    label: &str,
) {
    if result.has_changes() {
        info!(
            "{}: {} loaded, {} unloaded",
            label,
            result.packages_loaded.len(),
            result.packages_unloaded.len()
        );
        register_triggers_from_reconcile(runner, registry, result).await;
    }
    if result.has_failures() {
        for (id, err) in &result.packages_failed {
            warn!("Package {} failed: {}", id, err);
        }
    }
}

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
    log_retention_days: u64,
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

    // Daily-rotated file appender with optional retention via
    // `max_log_files`. `log_retention_days == 0` disables pruning per
    // operator opt-out. CLOACI-T-0592.
    let mut log_builder = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("cloacina")
        .filename_suffix("log");
    if log_retention_days > 0 {
        log_builder = log_builder.max_log_files(log_retention_days as usize);
    }
    let file_appender = log_builder.build(&logs_dir).with_context(|| {
        format!(
            "Failed to build rolling log appender in {}",
            logs_dir.display()
        )
    })?;
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

    // 2. Load config file (if exists) — needed for runner and watcher settings
    let config_path = home.join("config.toml");
    let config = CloacinaConfig::load(&config_path);
    let daemon_cfg = &config.daemon;

    // Collect all watch directories (default + CLI + config)
    let all_watch_dirs =
        collect_watch_dirs(&packages_dir, &watch_dirs, &config.resolve_watch_dirs());

    info!(
        "Watch directories: {:?}",
        all_watch_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
    );

    // 3. Create/open SQLite database
    let db_path = home.join("cloacina.db");
    let db_url = format!("sqlite://{}?mode=rwc&_journal_mode=WAL", db_path.display());
    info!("Database: {}", db_path.display());

    // 4. Create DefaultRunner with SQLite backend and configured poll intervals
    let mut config_builder = DefaultRunnerConfig::builder()
        .cron_poll_interval(Duration::from_millis(poll_interval_ms))
        .trigger_base_poll_interval(Duration::from_millis(daemon_cfg.trigger_poll_interval_ms))
        .cron_recovery_interval(Duration::from_secs(daemon_cfg.cron_recovery_interval_s));
    if let Some(max_catchup) = daemon_cfg.cron_max_catchup {
        config_builder = config_builder.cron_max_catchup_executions(max_catchup as usize);
    }
    let runner_config = config_builder
        .build()
        .context("Invalid runner configuration")?;

    let runner = DefaultRunner::with_config(&db_url, runner_config)
        .await
        .context("Failed to create DefaultRunner")?;
    info!("DefaultRunner initialized with SQLite backend");

    // 5. Start health socket and pulse
    let health_state = Arc::new(super::health::SharedDaemonState::new());
    let (health_shutdown_tx, health_shutdown_rx) = watch::channel(false);

    let socket_path = home.join("daemon.sock");
    let health_socket_handle = tokio::spawn(super::health::run_health_socket(
        socket_path,
        runner.dal().clone(),
        health_state.clone(),
        "sqlite".to_string(),
        health_shutdown_rx.clone(),
    ));

    let health_pulse_handle = tokio::spawn(super::health::run_health_pulse(
        runner.dal().clone(),
        health_state.clone(),
        "sqlite".to_string(),
        Duration::from_secs(60),
        health_shutdown_rx,
    ));

    // 6. Create FilesystemWorkflowRegistry
    let registry = Arc::new(FilesystemWorkflowRegistry::new(all_watch_dirs.clone()));
    let registry_for_triggers = registry.clone();

    // 6. Create shutdown channel
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    // 7. Create RegistryReconciler (we drive it manually, not via its built-in loop)
    let reconciler_config = ReconcilerConfig {
        reconcile_interval: Duration::from_millis(poll_interval_ms),
        enable_startup_reconciliation: false, // We do it ourselves below
        ..ReconcilerConfig::default()
    };

    let reconciler = RegistryReconciler::new(registry, reconciler_config, shutdown_rx)
        .context("Failed to create RegistryReconciler")?;

    // 8. Perform initial reconciliation
    info!("Running initial reconciliation...");
    match reconciler.reconcile().await {
        Ok(result) => {
            info!(
                "Initial reconciliation: {} loaded, {} unloaded, {} failed",
                result.packages_loaded.len(),
                result.packages_unloaded.len(),
                result.packages_failed.len()
            );
            health_state.set_packages_loaded(result.packages_loaded.len());
            health_state
                .set_last_reconciliation(chrono::Utc::now())
                .await;
            register_triggers_from_reconcile(&runner, &registry_for_triggers, &result).await;
        }
        Err(e) => {
            warn!("Initial reconciliation failed: {}", e);
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
                        health_state.set_packages_loaded(result.packages_loaded.len());
                        health_state.set_last_reconciliation(chrono::Utc::now()).await;
                        handle_reconcile(&runner, &registry_for_triggers, &result, "Reconciliation").await;
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
                        health_state.set_packages_loaded(result.packages_loaded.len());
                        health_state.set_last_reconciliation(chrono::Utc::now()).await;
                        handle_reconcile(&runner, &registry_for_triggers, &result, "Periodic reconciliation").await;
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
                let new_watch_dirs = collect_watch_dirs(
                    &packages_dir,
                    &watch_dirs,
                    &new_config.resolve_watch_dirs(),
                );

                apply_watch_dir_changes(&mut watcher, &current_watch_dirs, &new_watch_dirs);
                current_watch_dirs = new_watch_dirs;

                // Trigger reconciliation to pick up packages in new dirs
                info!("Triggering reconciliation after config reload...");
                match reconciler.reconcile().await {
                    Ok(result) => {
                        health_state.set_packages_loaded(result.packages_loaded.len());
                        health_state.set_last_reconciliation(chrono::Utc::now()).await;
                        handle_reconcile(&runner, &registry_for_triggers, &result, "Post-reload reconciliation").await;
                    }
                    Err(e) => {
                        error!("Post-reload reconciliation failed: {}", e);
                    }
                }

                info!("Configuration reload complete.");
            }
        }
    }

    // Stop health socket and pulse
    let _ = health_shutdown_tx.send(true);
    health_socket_handle.abort();
    health_pulse_handle.abort();

    // Graceful shutdown with timeout and force-exit on second signal
    let shutdown_timeout = Duration::from_secs(daemon_cfg.shutdown_timeout_s);
    info!(
        "Draining in-flight workflows (timeout: {}s)...",
        shutdown_timeout.as_secs()
    );
    info!("Press Ctrl+C again to force exit immediately.");

    // Race: runner shutdown vs timeout vs second Ctrl+C
    tokio::select! {
        result = runner.shutdown() => {
            match result {
                Ok(()) => info!("All workflows drained successfully."),
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

        // Pull the loaded package's compiled cdylib bytes so we can call
        // `get_trigger_metadata` over FFI. Python packages have no cdylib;
        // their triggers register through the import-time path.
        let loaded = match registry
            .get_workflow(&metadata.package_name, &metadata.version)
            .await
        {
            Ok(Some(l)) => l,
            _ => continue,
        };
        let library_data = match &loaded.compiled_data {
            Some(bytes) => bytes,
            None => continue,
        };

        // T-0553 / I-0102: read trigger metadata from the cdylib's
        // `get_trigger_metadata` FFI method (T-0552 made this real).
        // Cron-shaped entries route to the cron scheduler (already
        // registered by the reconciler at load time); non-cron entries
        // route to the polling scheduler here.
        let trigger_metadata = match load_trigger_metadata(library_data) {
            Ok(m) => m,
            Err(e) => {
                debug!(
                    "Failed to extract trigger metadata for {}: {}",
                    metadata.package_name, e
                );
                continue;
            }
        };

        if trigger_metadata.is_empty() {
            continue;
        }

        // If every trigger is cron-shaped, the reconciler already handled
        // them via `CronWorkflowRegistrar`. Skip the per-entry walk.
        if trigger_metadata.iter().all(|t| t.cron_expression.is_some()) {
            continue;
        }

        for trigger in &trigger_metadata {
            if trigger.cron_expression.is_some() {
                // Cron trigger registration moved into the reconciler
                // via `CronWorkflowRegistrar` (T-0553 follow-up). The
                // DefaultRunner injects a `DalCronRegistrar` at startup,
                // so the schedule was already created by the time this
                // post-reconcile hook fires. Skip to avoid double-
                // registration.
                continue;
            }
            // Custom-poll trigger — register against the cron-style
            // scheduler so it polls on its declared interval. The
            // `runtime.get_trigger` lookup succeeds for both in-process
            // triggers (compiled in) and packaged cdylib triggers (the
            // reconciler's `step_load_custom_triggers` registers an
            // FfiTriggerImpl per declared trigger via the FFI bridge).
            match runner.runtime().get_trigger(&trigger.name) {
                Some(t) => match scheduler.register_trigger(t.as_ref(), &trigger.name).await {
                    Ok(_) => {
                        info!(
                            "Registered trigger schedule from package {}: '{}' (poll: {})",
                            metadata.package_name, trigger.name, trigger.poll_interval
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to register trigger schedule for '{}' in package {}: {}",
                            trigger.name, metadata.package_name, e
                        );
                    }
                },
                None => {
                    warn!(
                        "Trigger '{}' declared in FFI metadata for package {} but no Trigger \
                         impl found in runtime — was the cdylib loaded?",
                        trigger.name, metadata.package_name
                    );
                }
            }
        }
    }
}

/// T-0553: load a fidius cdylib and pull `get_trigger_metadata` (method
/// index 5). Used by the daemon's post-load auto-registration hook.
fn load_trigger_metadata(
    library_data: &[u8],
) -> Result<Vec<cloacina_workflow_plugin::TriggerPackageMetadata>, String> {
    let library_extension = if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    };
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("temp dir: {}", e))?;
    let temp_path = temp_dir.path().join(format!(
        "trigger_{}.{}",
        uuid::Uuid::new_v4(),
        library_extension
    ));
    std::fs::write(&temp_path, library_data).map_err(|e| format!("write dylib: {}", e))?;

    let loaded =
        fidius_host::loader::load_library(&temp_path).map_err(|e| format!("load dylib: {}", e))?;
    let plugin = loaded
        .plugins
        .into_iter()
        .next()
        .ok_or_else(|| "no plugins in library".to_string())?;
    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    cloacina::computation_graph::packaging_bridge::call_get_trigger_metadata(&handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn collect_watch_dirs_deduplicates() {
        let pkg = PathBuf::from("/home/user/.cloacina/packages");
        let cli = vec![PathBuf::from("/extra/dir1"), PathBuf::from("/extra/dir2")];
        let config = vec![
            PathBuf::from("/extra/dir2"), // duplicate of CLI
            PathBuf::from("/config/dir3"),
        ];

        let result = collect_watch_dirs(&pkg, &cli, &config);
        assert_eq!(
            result,
            vec![
                PathBuf::from("/home/user/.cloacina/packages"),
                PathBuf::from("/extra/dir1"),
                PathBuf::from("/extra/dir2"),
                PathBuf::from("/config/dir3"),
            ]
        );
    }

    #[test]
    fn collect_watch_dirs_packages_dir_always_first() {
        let pkg = PathBuf::from("/packages");
        let cli = vec![PathBuf::from("/packages")]; // same as packages_dir
        let config = vec![];

        let result = collect_watch_dirs(&pkg, &cli, &config);
        // packages_dir should appear only once
        assert_eq!(result, vec![PathBuf::from("/packages")]);
    }

    #[test]
    fn collect_watch_dirs_empty_sources() {
        let pkg = PathBuf::from("/packages");
        let result = collect_watch_dirs(&pkg, &[], &[]);
        assert_eq!(result, vec![PathBuf::from("/packages")]);
    }

    #[test]
    fn collect_watch_dirs_preserves_order() {
        let pkg = PathBuf::from("/pkg");
        let cli = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let config = vec![PathBuf::from("/c")];

        let result = collect_watch_dirs(&pkg, &cli, &config);
        assert_eq!(
            result,
            vec![
                PathBuf::from("/pkg"),
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/c"),
            ]
        );
    }
}
