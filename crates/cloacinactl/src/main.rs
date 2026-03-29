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

//! Cloacina CLI - Command-line interface for the Cloacina task orchestration engine.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod commands;

/// Cloacina - A resilient task execution and orchestration engine
#[derive(Parser)]
#[command(name = "cloacina")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Database URL (can also be set via DATABASE_URL environment variable)
    #[arg(long, env = "DATABASE_URL", global = true)]
    database_url: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the daemon — a lightweight local scheduler that watches directories
    /// for .cloacina packages and runs their cron schedules and triggers
    Daemon {
        /// Daemon home directory for database, logs, and state
        #[arg(long, default_value_os_t = default_home())]
        home: PathBuf,

        /// Directories to watch for .cloacina packages (repeatable).
        /// The default packages directory (~/.cloacina/packages/) is always watched.
        #[arg(long = "watch-dir")]
        watch_dirs: Vec<PathBuf>,

        /// Reconciler poll interval in milliseconds
        #[arg(long, default_value = "50")]
        poll_interval: u64,
    },

    /// Administrative commands for managing the Cloacina system
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },
}

/// Default daemon home directory (~/.cloacina/).
fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[derive(Subcommand)]
enum AdminCommands {
    /// Clean up old execution events from the database
    CleanupEvents {
        /// Delete events older than this duration (e.g., "90d", "30d", "7d", "24h")
        #[arg(long, default_value = "90d")]
        older_than: String,

        /// Preview what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon {
            home,
            watch_dirs,
            poll_interval,
        } => {
            // Daemon sets up its own logging (file + stderr)
            commands::daemon::run(home, watch_dirs, poll_interval, cli.verbose).await?;
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
                    older_than,
                    dry_run,
                } => {
                    let database_url = cli
                    .database_url
                    .context("Database URL is required. Set --database-url or DATABASE_URL environment variable")?;

                    commands::cleanup_events::run(&database_url, &older_than, dry_run).await?;
                }
            }
        }
    }

    Ok(())
}
