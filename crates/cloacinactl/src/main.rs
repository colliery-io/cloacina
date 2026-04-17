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

//! cloacinactl — Command-line interface for the Cloacina task orchestration engine.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod commands;

/// cloacinactl — Cloacina task orchestration engine
#[derive(Parser)]
#[command(name = "cloacinactl")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Cloacina home directory
    #[arg(long, global = true, default_value_os_t = default_home())]
    home: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the daemon — a lightweight local scheduler that watches directories
    /// for .cloacina packages and runs their cron schedules and triggers
    Daemon {
        /// Directories to watch for .cloacina packages (repeatable).
        /// The default packages directory (~/.cloacina/packages/) is always watched.
        #[arg(long = "watch-dir")]
        watch_dirs: Vec<PathBuf>,

        /// Reconciler poll interval in milliseconds (file watcher handles immediate detection)
        #[arg(long, default_value = "500")]
        poll_interval: u64,
    },

    /// Manage configuration (get/set/list values in ~/.cloacina/config.toml)
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Administrative commands for managing the Cloacina system
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },

    /// Show the status of the running daemon
    Status,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Config key (e.g., "database_url", "daemon.poll_interval_ms")
        key: String,
    },

    /// Set a configuration value
    Set {
        /// Config key (e.g., "database_url", "daemon.poll_interval_ms")
        key: String,

        /// Value to set
        value: String,
    },

    /// List all configuration values
    List,
}

#[derive(Subcommand)]
enum AdminCommands {
    /// Clean up old execution events from the database
    CleanupEvents {
        /// Database URL (overrides config file and DATABASE_URL env var)
        #[arg(long, env = "DATABASE_URL")]
        database_url: Option<String>,

        /// Delete events older than this duration (e.g., "90d", "30d", "7d", "24h")
        #[arg(long, default_value = "90d")]
        older_than: String,

        /// Preview what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
    },
}

/// Default home directory (~/.cloacina/).
fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.home.join("config.toml");

    match cli.command {
        Commands::Daemon {
            watch_dirs,
            poll_interval,
        } => {
            // Daemon sets up its own logging (file + stderr)
            commands::daemon::run(cli.home, watch_dirs, poll_interval, cli.verbose).await?;
        }

        Commands::Config { command } => match command {
            ConfigCommands::Get { key } => {
                commands::config::run_get(&config_path, &key)?;
            }
            ConfigCommands::Set { key, value } => {
                commands::config::run_set(&config_path, &key, &value)?;
            }
            ConfigCommands::List => {
                commands::config::run_list(&config_path)?;
            }
        },

        Commands::Status => {
            commands::status::run(cli.home).await?;
        }

        Commands::Admin { command } => {
            // Standard stderr logging for admin commands
            let filter = if cli.verbose {
                EnvFilter::new("debug")
            } else {
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
            };
            tracing_subscriber::registry()
                .with(fmt::layer())
                .with(filter)
                .init();

            match command {
                AdminCommands::CleanupEvents {
                    database_url,
                    older_than,
                    dry_run,
                } => {
                    let db_url = commands::config::resolve_database_url(
                        database_url.as_deref(),
                        &config_path,
                    )?;
                    commands::cleanup_events::run(&db_url, &older_than, dry_run).await?;
                }
            }
        }
    }

    Ok(())
}
