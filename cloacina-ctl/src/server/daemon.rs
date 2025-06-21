/*
 *  Copyright 2025 Colliery Software
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

use crate::config::validation::Validate;
use crate::config::{CloacinaConfig, ConfigLoader};
use crate::database::validate_backend_compatibility;
use anyhow::{Context, Result};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

pub async fn start_server(
    config_path: Option<PathBuf>,
    foreground: bool,
    database_url_override: Option<String>,
) -> Result<()> {
    println!("{} Starting Cloacina server...", "→".cyan().bold());

    // Load configuration
    let loader = ConfigLoader::new();
    let mut config = loader
        .load_config(config_path.as_deref())
        .context("Failed to load configuration")?;

    // Override database URL if provided
    if let Some(url) = database_url_override {
        config.database.url = url;
    }

    // Validate configuration
    config
        .validate()
        .context("Configuration validation failed")?;

    // Validate database backend compatibility (Phase 01 integration)
    validate_backend_compatibility(&config.database.url)
        .context("Database backend compatibility check failed")?;

    println!(
        "{} Configuration validated successfully",
        "✓".green().bold()
    );

    // Check if server is already running
    if is_server_running(&config)? {
        return Err(anyhow::anyhow!(
            "Server is already running (PID file exists: {})",
            config.server.pid_file.display()
        ));
    }

    // Create DefaultRunner configuration from our config
    let mut runner_config = DefaultRunnerConfig::default();
    runner_config.max_concurrent_tasks = config.execution.max_concurrent_tasks as usize;
    runner_config.task_timeout = Duration::from_secs(config.execution.task_timeout_secs);
    runner_config.executor_poll_interval =
        Duration::from_millis(config.execution.polling_interval_ms);
    runner_config.enable_cron_scheduling = config.cron.enabled;
    runner_config.cron_poll_interval = Duration::from_secs(config.cron.check_interval_secs);
    runner_config.enable_registry_reconciler = config.registry.enabled;

    // Create the DefaultRunner
    println!("{} Initializing DefaultRunner...", "→".cyan().bold());
    let runner = DefaultRunner::with_config(&config.database.url, runner_config)
        .await
        .context("Failed to create DefaultRunner")?;

    // Write PID file
    write_pid_file(&config)?;

    println!("{} Server initialized successfully", "✓".green().bold());

    if foreground {
        println!(
            "{} Running in foreground mode (Ctrl+C to stop)",
            "→".cyan().bold()
        );

        // Wait for shutdown signal using tokio::select like the cron example
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal");
            }
        }

        println!("\n{} Shutting down server...", "→".yellow().bold());
    } else {
        println!(
            "{} Server started in daemon mode (PID file: {})",
            "✓".green().bold(),
            config.server.pid_file.display()
        );

        // In daemon mode, run indefinitely until killed
        info!("Running in daemon mode...");

        // Set up signal handling for daemon mode too
        let mut signal_received = false;

        // Keep running until we receive a signal
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal in daemon mode");
                    signal_received = true;
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    // Keep the process alive
                    continue;
                }
            }
        }

        if signal_received {
            println!("\n{} Shutting down server...", "→".yellow().bold());
        }
    }

    // Graceful shutdown
    info!("Shutting down gracefully...");
    runner.shutdown().await?;
    cleanup_pid_file(&config)?;

    println!("{} Server stopped successfully", "✓".green().bold());
    Ok(())
}

pub async fn stop_server(force: bool, timeout: u64) -> Result<()> {
    println!("{} Stopping Cloacina server...", "→".cyan().bold());

    // Load default config to get PID file location
    let loader = ConfigLoader::new();
    let config = loader
        .load_config(None::<&std::path::Path>)
        .context("Failed to load configuration")?;

    // Check if server is running
    let pid = read_pid_file(&config)?;
    if pid.is_none() {
        println!(
            "{} Server is not running (no PID file found)",
            "⚠".yellow().bold()
        );
        return Ok(());
    }

    let pid = pid.unwrap();

    if force {
        println!("{} Force stopping server (SIGKILL)", "→".yellow().bold());
        kill_process(pid, true)?;
    } else {
        println!(
            "{} Gracefully stopping server (timeout: {}s)",
            "→".cyan().bold(),
            timeout
        );
        kill_process(pid, false)?;

        // Wait for process to exit
        let timeout_duration = Duration::from_secs(timeout);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout_duration {
            if !is_process_running(pid) {
                break;
            }
            sleep(Duration::from_millis(100)).await;
        }

        // Force kill if still running after timeout
        if is_process_running(pid) {
            println!(
                "{} Timeout exceeded, force stopping server",
                "⚠".yellow().bold()
            );
            kill_process(pid, true)?;
        }
    }

    // Clean up PID file
    cleanup_pid_file(&config)?;

    println!("{} Server stopped successfully", "✓".green().bold());
    Ok(())
}

pub async fn restart_server(config_path: Option<PathBuf>, force: bool, timeout: u64) -> Result<()> {
    println!("{} Restarting Cloacina server...", "→".cyan().bold());

    // Stop the server first
    stop_server(force, timeout).await?;

    // Small delay between stop and start
    sleep(Duration::from_millis(1000)).await;

    // Start the server again
    start_server(config_path, false, None).await?;

    println!("{} Server restarted successfully", "✓".green().bold());
    Ok(())
}

/// Check if server is already running
fn is_server_running(config: &CloacinaConfig) -> Result<bool> {
    if let Some(pid) = read_pid_file(config)? {
        Ok(is_process_running(pid))
    } else {
        Ok(false)
    }
}

/// Write current process PID to PID file
fn write_pid_file(config: &CloacinaConfig) -> Result<()> {
    let pid = std::process::id();

    // Create parent directories if they don't exist
    if let Some(parent) = config.server.pid_file.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create PID file directory: {}", parent.display())
        })?;
    }

    fs::write(&config.server.pid_file, pid.to_string()).with_context(|| {
        format!(
            "Failed to write PID file: {}",
            config.server.pid_file.display()
        )
    })?;

    Ok(())
}

/// Read PID from PID file
fn read_pid_file(config: &CloacinaConfig) -> Result<Option<u32>> {
    if !config.server.pid_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config.server.pid_file).with_context(|| {
        format!(
            "Failed to read PID file: {}",
            config.server.pid_file.display()
        )
    })?;

    let pid: u32 = content
        .trim()
        .parse()
        .with_context(|| format!("Invalid PID in PID file: {}", content.trim()))?;

    Ok(Some(pid))
}

/// Clean up PID file
fn cleanup_pid_file(config: &CloacinaConfig) -> Result<()> {
    if config.server.pid_file.exists() {
        fs::remove_file(&config.server.pid_file).with_context(|| {
            format!(
                "Failed to remove PID file: {}",
                config.server.pid_file.display()
            )
        })?;
    }
    Ok(())
}

/// Check if a process is running
fn is_process_running(pid: u32) -> bool {
    use sysinfo::{Pid, System};

    let system = System::new_all();
    system.process(Pid::from(pid as usize)).is_some()
}

/// Kill a process with SIGTERM or SIGKILL
fn kill_process(pid: u32, force: bool) -> Result<()> {
    use std::process::Command;

    let signal = if force { "KILL" } else { "TERM" };

    let output = Command::new("kill")
        .arg(format!("-{}", signal))
        .arg(pid.to_string())
        .output()
        .context("Failed to execute kill command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to kill process {}: {}",
            pid,
            stderr
        ));
    }

    Ok(())
}
