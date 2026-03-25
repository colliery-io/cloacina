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

//! Scheduler Performance Benchmark Harness
//!
//! Exercises the full scheduling feature set against a real database:
//! - Trigger scheduler (including cron)
//! - Continuous scheduler
//! - Task execution engine
//! - Hybrid (all schedulers simultaneously)

mod metrics;
mod reporting;
mod scenarios;

use clap::{Parser, Subcommand};
use std::time::Duration;

#[derive(Parser)]
#[command(
    name = "scheduler-bench",
    about = "Cloacina scheduler performance benchmarks"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Test duration
    #[arg(long, default_value = "60s", global = true)]
    duration: String,

    /// Output format
    #[arg(long, default_value = "table", global = true)]
    output: OutputFormat,

    /// Database URL (defaults to SQLite in-memory)
    #[arg(long, global = true)]
    database_url: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Smoke test — single workflow, proves harness works
    Smoke,
    /// Trigger + cron scheduler benchmarks
    Trigger {
        #[arg(long)]
        scenario: Option<String>,
    },
    /// Continuous scheduler benchmarks
    Continuous {
        #[arg(long)]
        scenario: Option<String>,
    },
    /// Task execution engine benchmarks
    Execution {
        #[arg(long)]
        scenario: Option<String>,
    },
    /// Hybrid — all schedulers running simultaneously
    Hybrid {
        #[arg(long)]
        scenario: Option<String>,
    },
    /// Run all benchmarks with default scenarios
    All,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

fn parse_duration(s: &str) -> Duration {
    let s = s.trim();
    if s.ends_with("ms") {
        Duration::from_millis(s[..s.len() - 2].parse().unwrap_or(60000))
    } else if s.ends_with('s') {
        Duration::from_secs(s[..s.len() - 1].parse().unwrap_or(60))
    } else if s.ends_with('m') {
        Duration::from_secs(s[..s.len() - 1].parse::<u64>().unwrap_or(1) * 60)
    } else {
        Duration::from_secs(s.parse().unwrap_or(60))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let duration = parse_duration(&cli.duration);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive("off".parse().unwrap()),
        )
        .init();

    let db_url = cli.database_url.unwrap_or_else(|| {
        format!(
            "file:bench_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().to_string().replace('-', "")
        )
    });

    println!("Scheduler Performance Benchmark");
    println!(
        "  Database: {}",
        if db_url.contains("memory") {
            "SQLite (in-memory)"
        } else {
            &db_url
        }
    );
    println!("  Duration: {:?}", duration);
    println!();

    let db = cloacina::Database::try_new_with_schema(&db_url, "", 4, None)?;
    db.run_migrations().await?;

    match cli.command {
        Commands::Smoke => {
            let results = scenarios::smoke::run(&db, duration).await?;
            reporting::print_results("Smoke", &results, &cli.output);
        }
        Commands::Trigger { scenario } => {
            let results = scenarios::trigger::run(&db, duration, scenario.as_deref()).await?;
            reporting::print_results("Trigger", &results, &cli.output);
        }
        Commands::Continuous { scenario } => {
            let results = scenarios::continuous::run(&db, duration, scenario.as_deref()).await?;
            reporting::print_results("Continuous", &results, &cli.output);
        }
        Commands::Execution { scenario } => {
            let results = scenarios::execution::run(&db, duration, scenario.as_deref()).await?;
            reporting::print_results("Execution", &results, &cli.output);
        }
        Commands::Hybrid { scenario } => {
            let results = scenarios::hybrid::run(&db, duration, scenario.as_deref()).await?;
            reporting::print_results("Hybrid", &results, &cli.output);
        }
        Commands::All => {
            for (name, runner) in [
                ("Smoke", scenarios::smoke::run(&db, duration).await),
                (
                    "Trigger",
                    scenarios::trigger::run(&db, duration, None).await,
                ),
                (
                    "Continuous",
                    scenarios::continuous::run(&db, duration, None).await,
                ),
                (
                    "Execution",
                    scenarios::execution::run(&db, duration, None).await,
                ),
                ("Hybrid", scenarios::hybrid::run(&db, duration, None).await),
            ] {
                match runner {
                    Ok(results) => reporting::print_results(name, &results, &cli.output),
                    Err(e) => eprintln!("  {} FAILED: {}", name, e),
                }
            }
        }
    }

    Ok(())
}
