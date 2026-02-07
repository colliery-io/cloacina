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
    /// Administrative commands for managing the Cloacina system
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },
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

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    match cli.command {
        Commands::Admin { command } => match command {
            AdminCommands::CleanupEvents {
                older_than,
                dry_run,
            } => {
                let database_url = cli
                    .database_url
                    .context("Database URL is required. Set --database-url or DATABASE_URL environment variable")?;

                commands::cleanup_events::run(&database_url, &older_than, dry_run).await?;
            }
        },
    }

    Ok(())
}
