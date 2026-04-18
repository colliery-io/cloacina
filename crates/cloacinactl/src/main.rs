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
//!
//! Structure: strict noun-verb (`<noun> <verb>`). Runtime services (`daemon`,
//! `server`) expose `start`/`stop`/`status`/`health`. Client nouns (`package`,
//! `workflow`, etc.) land in later tasks of I-0098. `status` at the top level
//! is a documented exception — a composite view over daemon + server.

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;

mod commands;
mod nouns;
mod shared;

use shared::error::CliError;

use nouns::{daemon, package, server};

/// cloacinactl — Cloacina task orchestration engine
#[derive(Parser)]
#[command(name = "cloacinactl")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    globals: GlobalOpts,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Clone)]
pub struct GlobalOpts {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Cloacina home directory
    #[arg(long, global = true, default_value_os_t = default_home())]
    pub home: PathBuf,

    /// Named profile from `~/.cloacina/config.toml` (profile resolution lands in T-0512).
    #[arg(long, global = true)]
    pub profile: Option<String>,

    /// Override the profile's server URL.
    #[arg(long, global = true)]
    pub server: Option<String>,

    /// Override the profile's API key (accepts `env:VAR` or `file:PATH`).
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// Tenant to target (required for admin keys on tenant-scoped commands).
    #[arg(long, global = true)]
    pub tenant: Option<String>,

    /// Shortcut for `-o json`.
    #[arg(long, global = true)]
    pub json: bool,

    /// Output format (default table).
    #[arg(short = 'o', long = "output", global = true, value_enum)]
    pub output: Option<OutputFormat>,

    /// Disable ANSI colors.
    #[arg(long, global = true)]
    pub no_color: bool,
}

#[derive(Copy, Clone, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Yaml,
    /// One ID per line — useful for piping.
    Id,
}

impl GlobalOpts {
    pub fn effective_output(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            self.output.unwrap_or_default()
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Daemon — lightweight local scheduler
    Daemon(daemon::DaemonCmd),

    /// Server — cloacina-server HTTP API
    Server(server::ServerCmd),

    /// Package — build, pack, upload, and inspect .cloacina archives
    Package(package::PackageCmd),

    /// Composite status: daemon + server side by side.
    Status,

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
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Config key (e.g., "database_url", "daemon.poll_interval_ms")
        key: String,
    },

    /// Set a configuration value
    Set { key: String, value: String },

    /// List all configuration values
    List,

    /// Manage server-targeting profiles.
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// Upsert a profile by name.
    Set {
        /// Profile name.
        name: String,
        /// Server URL.
        server: String,
        /// API key (raw, `env:VAR`, or `file:PATH`).
        #[arg(long)]
        api_key: String,
        /// Also set this profile as the default.
        #[arg(long)]
        default: bool,
    },
    /// List profiles; default marked with `*`.
    List,
    /// Set the default profile.
    Use { name: String },
    /// Remove a profile; clears default_profile if it was the default.
    Delete { name: String },
}

#[derive(Subcommand)]
enum AdminCommands {
    /// Clean up old execution events from the database
    CleanupEvents {
        #[arg(long, env = "DATABASE_URL")]
        database_url: Option<String>,

        #[arg(long, default_value = "90d")]
        older_than: String,

        #[arg(long)]
        dry_run: bool,
    },
}

fn default_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloacina")
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::from(0),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

async fn run() -> std::result::Result<(), CliError> {
    let cli = Cli::parse();
    let config_path = cli.globals.home.join("config.toml");

    (match cli.command {
        Commands::Daemon(cmd) => cmd.run(&cli.globals).await,
        Commands::Server(cmd) => cmd.run(&cli.globals).await,
        Commands::Package(cmd) => return cmd.run(&cli.globals).await,
        Commands::Status => nouns::top_level_status(&cli.globals).await,

        Commands::Config { command } => match command {
            ConfigCommands::Get { key } => commands::config::run_get(&config_path, &key),
            ConfigCommands::Set { key, value } => {
                commands::config::run_set(&config_path, &key, &value)
            }
            ConfigCommands::List => commands::config::run_list(&config_path),
            ConfigCommands::Profile { command } => match command {
                ProfileCommands::Set {
                    name,
                    server,
                    api_key,
                    default,
                } => commands::config::run_profile_set(
                    &config_path,
                    &name,
                    &server,
                    &api_key,
                    default,
                ),
                ProfileCommands::List => commands::config::run_profile_list(&config_path),
                ProfileCommands::Use { name } => {
                    commands::config::run_profile_use(&config_path, &name)
                }
                ProfileCommands::Delete { name } => {
                    commands::config::run_profile_delete(&config_path, &name)
                }
            },
        },

        Commands::Admin { command } => {
            use tracing_subscriber::{fmt, prelude::*, EnvFilter};
            let filter = if cli.globals.verbose {
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
                    commands::cleanup_events::run(&db_url, &older_than, dry_run).await
                }
            }
        }
    })
    .map_err(CliError::from)
}
