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

//! cloacinactl - Control tool for the Cloacina task orchestration engine.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod commands;

/// cloacinactl - Control tool for the Cloacina task orchestration engine
#[derive(Parser)]
#[command(name = "cloacinactl")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Database URL (can also be set via DATABASE_URL environment variable)
    #[arg(long, env = "DATABASE_URL", global = true)]
    database_url: Option<String>,

    /// Organization ID for multi-tenant operations
    #[arg(long, env = "CLOACINA_ORG_ID", global = true)]
    org_id: Option<String>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Package signing and verification
    Package {
        #[command(subcommand)]
        command: PackageCommands,
    },

    /// Signing key management
    Key {
        #[command(subcommand)]
        command: KeyCommands,
    },

    /// Administrative operations
    Admin {
        #[command(subcommand)]
        command: AdminCommands,
    },
}

#[derive(Subcommand)]
enum PackageCommands {
    /// Build a .cloacina package from a Python project (delegates to cloaca)
    Build {
        /// Output directory for the package
        #[arg(short, long, default_value = ".")]
        output: String,

        /// Target platform(s) (default: current platform)
        #[arg(long)]
        target: Vec<String>,

        /// Show what would be built without building
        #[arg(long)]
        dry_run: bool,
    },

    /// Sign a package with a signing key
    Sign {
        /// Path to the package file to sign
        package: String,

        /// Signing key ID
        #[arg(long)]
        key_id: String,

        /// Store the signature in the database (in addition to the .sig file)
        #[arg(long)]
        store: bool,
    },

    /// Verify a package signature
    Verify {
        /// Path to the package file to verify
        package: String,

        /// Path to a detached signature file (default: <package>.sig)
        #[arg(long)]
        signature: Option<String>,

        /// Verify offline using a public key file instead of database
        #[arg(long)]
        public_key: Option<String>,
    },

    /// Inspect a detached signature file
    Inspect {
        /// Path to the signature file
        signature: String,
    },
}

#[derive(Subcommand)]
enum KeyCommands {
    /// Generate a new signing keypair
    Generate {
        /// Human-readable name for the key
        #[arg(long)]
        name: String,
    },

    /// List signing keys
    List,

    /// Export a public key for distribution
    Export {
        /// Signing key ID to export
        key_id: String,

        /// Output format: pem (default) or raw
        #[arg(long, default_value = "pem")]
        format: String,
    },

    /// Revoke a signing key
    Revoke {
        /// Signing key ID to revoke
        key_id: String,
    },

    /// Manage trusted public keys for verification
    Trust {
        #[command(subcommand)]
        command: TrustCommands,
    },
}

#[derive(Subcommand)]
enum TrustCommands {
    /// Add a trusted public key from a PEM file
    Add {
        /// Path to the PEM public key file
        key_file: String,

        /// Optional name for the trusted key
        #[arg(long)]
        name: Option<String>,
    },

    /// List trusted public keys
    List,

    /// Revoke a trusted public key
    Revoke {
        /// Trusted key ID to revoke
        key_id: String,
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
        Commands::Package { command } => {
            let database_url = cli.database_url.as_deref();
            let org_id = &cli.org_id;

            match command {
                PackageCommands::Build {
                    output,
                    target,
                    dry_run,
                } => {
                    commands::package::build(&output, &target, dry_run, cli.verbose).await?;
                }
                PackageCommands::Sign {
                    package,
                    key_id,
                    store,
                } => {
                    let db_url = database_url.context(
                        "Database URL is required. Set --database-url or DATABASE_URL env var",
                    )?;
                    commands::package::sign(db_url, &package, &key_id, store).await?;
                }
                PackageCommands::Verify {
                    package,
                    signature,
                    public_key,
                } => {
                    commands::package::verify(
                        database_url,
                        org_id.as_deref(),
                        &package,
                        signature.as_deref(),
                        public_key.as_deref(),
                    )
                    .await?;
                }
                PackageCommands::Inspect { signature } => {
                    commands::package::inspect(&signature)?;
                }
            }
        }

        Commands::Key { command } => {
            let database_url = cli
                .database_url
                .as_deref()
                .context("Database URL is required. Set --database-url or DATABASE_URL env var")?;
            let org_id = cli
                .org_id
                .as_deref()
                .context("Organization ID is required. Set --org-id or CLOACINA_ORG_ID env var")?;

            match command {
                KeyCommands::Generate { name } => {
                    commands::key::generate(database_url, org_id, &name).await?;
                }
                KeyCommands::List => {
                    commands::key::list(database_url, org_id).await?;
                }
                KeyCommands::Export { key_id, format } => {
                    commands::key::export(database_url, &key_id, &format).await?;
                }
                KeyCommands::Revoke { key_id } => {
                    commands::key::revoke(database_url, &key_id).await?;
                }
                KeyCommands::Trust { command } => match command {
                    TrustCommands::Add { key_file, name } => {
                        commands::key_trust::add(database_url, org_id, &key_file, name.as_deref())
                            .await?;
                    }
                    TrustCommands::List => {
                        commands::key_trust::list(database_url, org_id).await?;
                    }
                    TrustCommands::Revoke { key_id } => {
                        commands::key_trust::revoke(database_url, &key_id).await?;
                    }
                },
            }
        }

        Commands::Admin { command } => match command {
            AdminCommands::CleanupEvents {
                older_than,
                dry_run,
            } => {
                let database_url = cli.database_url.context(
                    "Database URL is required. Set --database-url or DATABASE_URL env var",
                )?;
                commands::cleanup_events::run(&database_url, &older_than, dry_run).await?;
            }
        },
    }

    Ok(())
}
