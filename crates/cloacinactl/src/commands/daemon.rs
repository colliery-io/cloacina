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

//! `cloacinactl daemon` — lightweight local scheduler with SQLite.
//!
//! Headless background process that watches a directory for `.cloacina` packages,
//! registers them, and runs them on cron schedules. No HTTP API, no Postgres.

use anyhow::{Context, Result};
use cloacina::dal::{UnifiedRegistryStorage, DAL};
use cloacina::database::universal_types::UniversalUuid;
use cloacina::registry::{WorkflowRegistry, WorkflowRegistryImpl};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::Database;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::sync::watch;
use tracing::{error, info};

/// Default data directory.
fn default_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

fn default_db_path() -> PathBuf {
    default_data_dir().join("daemon.db")
}

fn default_storage_path() -> PathBuf {
    default_data_dir().join("storage")
}

/// Arguments for the `daemon` subcommand.
#[derive(Debug, clap::Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: Option<DaemonCommands>,

    /// Directory to watch for .cloacina packages.
    #[arg(long)]
    pub packages: Option<PathBuf>,

    /// SQLite database path.
    #[arg(long, default_value_os_t = default_db_path())]
    pub db: PathBuf,

    /// Filesystem registry storage path.
    #[arg(long, default_value_os_t = default_storage_path())]
    pub storage: PathBuf,

    /// How often to scan for new packages (seconds).
    #[arg(long, default_value = "5")]
    pub poll_interval: u64,
}

#[derive(Debug, clap::Subcommand)]
pub enum DaemonCommands {
    /// Show registered workflows, schedules, and recent executions.
    Status(StatusArgs),

    /// Manage cron schedules.
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommands,
    },

    /// Register a .cloacina package manually.
    Register(RegisterArgs),
}

#[derive(Debug, clap::Args)]
pub struct StatusArgs {
    /// SQLite database path.
    #[arg(long, default_value_os_t = default_db_path())]
    pub db: PathBuf,

    /// Filesystem registry storage path.
    #[arg(long, default_value_os_t = default_storage_path())]
    pub storage: PathBuf,
}

#[derive(Debug, clap::Subcommand)]
pub enum ScheduleCommands {
    /// Create a cron schedule for a workflow.
    Set(ScheduleSetArgs),
    /// List all cron schedules.
    List(ScheduleListArgs),
    /// Delete a cron schedule.
    Delete(ScheduleDeleteArgs),
}

#[derive(Debug, clap::Args)]
pub struct ScheduleSetArgs {
    /// Workflow name to schedule.
    pub workflow_name: String,
    /// Cron expression (e.g. "0 9 * * *").
    #[arg(long)]
    pub cron: String,
    /// Timezone (default: UTC).
    #[arg(long, default_value = "UTC")]
    pub timezone: String,
    /// SQLite database path.
    #[arg(long, default_value_os_t = default_db_path())]
    pub db: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ScheduleListArgs {
    /// SQLite database path.
    #[arg(long, default_value_os_t = default_db_path())]
    pub db: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct ScheduleDeleteArgs {
    /// Schedule ID to delete.
    pub id: String,
    /// SQLite database path.
    #[arg(long, default_value_os_t = default_db_path())]
    pub db: PathBuf,
}

#[derive(Debug, clap::Args)]
pub struct RegisterArgs {
    /// Path to .cloacina package file.
    pub package: PathBuf,
    /// SQLite database path.
    #[arg(long, default_value_os_t = default_db_path())]
    pub db: PathBuf,
    /// Filesystem registry storage path.
    #[arg(long, default_value_os_t = default_storage_path())]
    pub storage: PathBuf,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Entry point — dispatches to run/status/schedule/register.
pub async fn dispatch(args: &DaemonArgs) -> Result<()> {
    match &args.command {
        Some(DaemonCommands::Status(status_args)) => status(status_args).await,
        Some(DaemonCommands::Schedule { command }) => match command {
            ScheduleCommands::Set(a) => schedule_set(a).await,
            ScheduleCommands::List(a) => schedule_list(a).await,
            ScheduleCommands::Delete(a) => schedule_delete(a).await,
        },
        Some(DaemonCommands::Register(a)) => register(a).await,
        None => run(args).await,
    }
}

// ---------------------------------------------------------------------------
// daemon run — main loop
// ---------------------------------------------------------------------------

/// Run the daemon: start DefaultRunner with SQLite, spawn directory scanner, wait for shutdown.
async fn run(args: &DaemonArgs) -> Result<()> {
    let packages_dir = args
        .packages
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--packages <dir> is required when running the daemon"))?;

    // Create data directories
    std::fs::create_dir_all(&args.db.parent().unwrap_or(&args.db))?;
    std::fs::create_dir_all(&args.storage)?;
    std::fs::create_dir_all(packages_dir)?;

    let sqlite_url = format!("sqlite://{}", args.db.display());

    info!(
        db = %args.db.display(),
        storage = %args.storage.display(),
        packages = %packages_dir.display(),
        poll_interval = args.poll_interval,
        "Starting cloacina daemon"
    );

    // Build runner config
    let config = DefaultRunnerConfig::builder()
        .enable_registry_reconciler(true)
        .registry_storage_backend("filesystem")
        .registry_storage_path(Some(args.storage.clone()))
        .registry_reconcile_interval(Duration::from_secs(5))
        .registry_enable_startup_reconciliation(true)
        .enable_cron_scheduling(true)
        .cron_enable_recovery(true)
        .cron_poll_interval(Duration::from_secs(2))
        .db_pool_size(4)
        .build();

    let runner = DefaultRunner::with_config(&sqlite_url, config)
        .await
        .context("Failed to start daemon runner")?;

    info!("Daemon runner started, background services active");

    // Shutdown channel for the scanner
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn directory scanner
    let scanner_db = runner.database().clone();
    let scanner_storage_path = args.storage.clone();
    let scanner_packages_dir = packages_dir.clone();
    let scanner_interval = Duration::from_secs(args.poll_interval);

    let scanner_handle = tokio::spawn(async move {
        directory_scanner(
            scanner_packages_dir,
            scanner_db,
            scanner_storage_path,
            scanner_interval,
            shutdown_rx,
        )
        .await;
    });

    // Wait for shutdown signal
    shutdown_signal().await;
    info!("Shutdown signal received");

    // Stop scanner
    let _ = shutdown_tx.send(true);
    let _ = scanner_handle.await;

    // Stop runner
    runner
        .shutdown()
        .await
        .context("Error during runner shutdown")?;

    info!("Daemon stopped cleanly");
    Ok(())
}

/// Wait for SIGTERM or Ctrl+C.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

// ---------------------------------------------------------------------------
// Directory scanner
// ---------------------------------------------------------------------------

/// Polls a directory for .cloacina files, registers new ones, unregisters removed ones.
async fn directory_scanner(
    packages_dir: PathBuf,
    database: Database,
    storage_path: PathBuf,
    interval: Duration,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut known_files: HashMap<String, SystemTime> = HashMap::new();

    info!(dir = %packages_dir.display(), "Directory scanner started");

    loop {
        // Scan
        if let Err(e) = scan_once(&packages_dir, &database, &storage_path, &mut known_files).await {
            error!("Directory scan error: {}", e);
        }

        // Wait for next interval or shutdown
        tokio::select! {
            _ = tokio::time::sleep(interval) => {},
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("Directory scanner shutting down");
                    return;
                }
            }
        }
    }
}

/// Single scan pass: detect added/removed .cloacina files.
async fn scan_once(
    packages_dir: &PathBuf,
    database: &Database,
    storage_path: &PathBuf,
    known_files: &mut HashMap<String, SystemTime>,
) -> Result<()> {
    // List current .cloacina files
    let mut current_files: HashMap<String, SystemTime> = HashMap::new();

    let entries = std::fs::read_dir(packages_dir).context("Failed to read packages directory")?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("cloacina") {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    let filename = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    current_files.insert(filename, modified);
                }
            }
        }
    }

    // Detect new or modified files
    for (filename, modified) in &current_files {
        let is_new = match known_files.get(filename) {
            None => true,
            Some(prev_modified) => prev_modified != modified,
        };

        if is_new {
            let path = packages_dir.join(filename);
            info!(file = %filename, "Registering package");

            match register_package(database, storage_path, &path).await {
                Ok(id) => {
                    info!(file = %filename, id = %id, "Package registered successfully");
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("already exists") {
                        info!(file = %filename, "Package already registered, skipping");
                    } else {
                        error!(file = %filename, error = %e, "Failed to register package");
                    }
                }
            }
        }
    }

    // Detect removed files
    let removed: Vec<String> = known_files
        .keys()
        .filter(|k| !current_files.contains_key(*k))
        .cloned()
        .collect();

    for filename in &removed {
        info!(file = %filename, "Package file removed from directory");
        // Note: we don't auto-unregister here — the package stays in the DB.
        // Use `daemon schedule delete` to manage lifecycle explicitly.
    }

    *known_files = current_files;
    Ok(())
}

/// Register a single .cloacina package file.
async fn register_package(
    database: &Database,
    storage_path: &PathBuf,
    package_path: &PathBuf,
) -> Result<uuid::Uuid> {
    let data = std::fs::read(package_path)
        .with_context(|| format!("Failed to read {}", package_path.display()))?;

    let storage = cloacina::dal::FilesystemRegistryStorage::new(storage_path)
        .map_err(|e| anyhow::anyhow!("Failed to create filesystem storage: {}", e))?;

    let mut registry = WorkflowRegistryImpl::new(storage, database.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create registry: {}", e))?;

    let package_id = registry
        .register_workflow(data)
        .await
        .map_err(|e| anyhow::anyhow!("Registration failed: {}", e))?;

    Ok(package_id)
}

// ---------------------------------------------------------------------------
// daemon status
// ---------------------------------------------------------------------------

async fn status(args: &StatusArgs) -> Result<()> {
    let sqlite_url = format!("sqlite://{}", args.db.display());
    let database = Database::try_new_with_schema(&sqlite_url, "", 2, None)
        .context("Failed to open database")?;
    database
        .run_migrations()
        .await
        .map_err(|e| anyhow::anyhow!("Migration error: {}", e))?;

    let dal = DAL::new(database.clone());

    // Registered packages
    let storage = cloacina::dal::FilesystemRegistryStorage::new(&args.storage)
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {}", e))?;
    let registry = WorkflowRegistryImpl::new(storage, database.clone())
        .map_err(|e| anyhow::anyhow!("Failed to create registry: {}", e))?;

    let packages = registry
        .list_workflows()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list packages: {}", e))?;

    println!("Registered Workflows ({}):", packages.len());
    if packages.is_empty() {
        println!("  (none)");
    }
    for p in &packages {
        println!(
            "  {} v{} — {} tasks [{}]",
            p.package_name,
            p.version,
            p.tasks.len(),
            p.id
        );
    }

    // Cron schedules
    let schedules = dal
        .cron_schedule()
        .list(false, 100, 0)
        .await
        .unwrap_or_default();

    println!("\nCron Schedules ({}):", schedules.len());
    if schedules.is_empty() {
        println!("  (none)");
    }
    for s in &schedules {
        let enabled = if s.enabled.0 { "enabled" } else { "disabled" };
        println!(
            "  {} — \"{}\" ({}) — next: {} [{}]",
            s.workflow_name,
            s.cron_expression,
            enabled,
            s.next_run_at.0.format("%Y-%m-%d %H:%M:%S UTC"),
            s.id.0
        );
    }

    // Recent executions
    let stats = dal
        .cron_execution()
        .get_execution_stats(chrono::Utc::now() - chrono::Duration::try_hours(24).unwrap())
        .await;

    if let Ok(stats) = stats {
        println!("\nExecution Stats (last 24h):");
        println!("  Total:      {}", stats.total_executions);
        println!("  Successful: {}", stats.successful_executions);
        println!(
            "  Failed:     {}",
            stats.total_executions - stats.successful_executions
        );
        if stats.total_executions > 0 {
            let rate = stats.successful_executions as f64 / stats.total_executions as f64 * 100.0;
            println!("  Success:    {:.1}%", rate);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// daemon register
// ---------------------------------------------------------------------------

async fn register(args: &RegisterArgs) -> Result<()> {
    let sqlite_url = format!("sqlite://{}", args.db.display());
    let database = Database::try_new_with_schema(&sqlite_url, "", 2, None)
        .context("Failed to open database")?;
    database
        .run_migrations()
        .await
        .map_err(|e| anyhow::anyhow!("Migration error: {}", e))?;

    std::fs::create_dir_all(&args.storage)?;

    let id = register_package(&database, &args.storage, &args.package).await?;
    println!("Package registered: {}", id);
    Ok(())
}

// ---------------------------------------------------------------------------
// daemon schedule set/list/delete
// ---------------------------------------------------------------------------

async fn schedule_set(args: &ScheduleSetArgs) -> Result<()> {
    let sqlite_url = format!("sqlite://{}", args.db.display());

    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .enable_registry_reconciler(false)
        .db_pool_size(2)
        .build();

    let runner = DefaultRunner::with_config(&sqlite_url, config)
        .await
        .context("Failed to start runner")?;

    let schedule_id = runner
        .register_cron_workflow(&args.workflow_name, &args.cron, &args.timezone)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    runner.shutdown().await.ok();

    println!(
        "Schedule created: {} — \"{}\" ({}) [{}]",
        args.workflow_name, args.cron, args.timezone, schedule_id.0
    );
    Ok(())
}

async fn schedule_list(args: &ScheduleListArgs) -> Result<()> {
    let sqlite_url = format!("sqlite://{}", args.db.display());
    let database = Database::try_new_with_schema(&sqlite_url, "", 2, None)
        .context("Failed to open database")?;
    database
        .run_migrations()
        .await
        .map_err(|e| anyhow::anyhow!("Migration error: {}", e))?;

    let dal = DAL::new(database);
    let schedules = dal
        .cron_schedule()
        .list(false, 100, 0)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list schedules: {}", e))?;

    if schedules.is_empty() {
        println!("No cron schedules.");
        return Ok(());
    }

    for s in &schedules {
        let enabled = if s.enabled.0 { "enabled" } else { "disabled" };
        println!(
            "{} — \"{}\" {} ({}) — next: {}",
            s.id.0,
            s.cron_expression,
            s.workflow_name,
            enabled,
            s.next_run_at.0.format("%Y-%m-%d %H:%M:%S UTC"),
        );
    }

    Ok(())
}

async fn schedule_delete(args: &ScheduleDeleteArgs) -> Result<()> {
    let sqlite_url = format!("sqlite://{}", args.db.display());
    let database = Database::try_new_with_schema(&sqlite_url, "", 2, None)
        .context("Failed to open database")?;
    database
        .run_migrations()
        .await
        .map_err(|e| anyhow::anyhow!("Migration error: {}", e))?;

    let uuid = uuid::Uuid::parse_str(&args.id).context("Invalid UUID")?;
    let dal = DAL::new(database);
    dal.cron_schedule()
        .delete(UniversalUuid(uuid))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to delete schedule: {}", e))?;

    println!("Schedule deleted: {}", args.id);
    Ok(())
}
