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

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "cloacina-ctl",
    version,
    about = "Command-line interface for Cloacina workflow compilation and management",
    long_about = "A tool for compiling, packaging, inspecting, and debugging Cloacina workflows"
)]
pub struct Cli {
    /// Target triple for cross-compilation (e.g., x86_64-unknown-linux-gnu)
    #[arg(long, global = true)]
    pub target: Option<String>,

    /// Build profile (debug or release)
    #[arg(long, global = true, default_value = "release")]
    pub profile: String,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Control colored output [auto, always, never]
    #[arg(long, global = true, default_value = "auto")]
    pub color: String,

    /// Number of parallel jobs for compilation
    #[arg(short = 'j', long, global = true)]
    pub jobs: Option<u32>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum DebugAction {
    /// List all available tasks in the package
    List,
    /// Execute a specific task by index or name
    Execute {
        /// Task index (0, 1, 2...) or task name
        task: String,

        /// JSON context to pass to the task
        #[arg(long, default_value = "{}")]
        context: String,

        /// Environment variables to set (KEY=VALUE format)
        #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
        env_vars: Vec<String>,

        /// Load environment variables from .env file
        #[arg(long = "env-file")]
        env_file: Option<PathBuf>,

        /// Include current environment variables in context
        #[arg(long = "include-env")]
        include_env: bool,

        /// Prefix for environment variables to include (e.g., "CLOACINA_")
        #[arg(long = "env-prefix", requires = "include_env")]
        env_prefix: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum Commands {
    /// Compile a workflow Cargo project into a shared library
    Compile {
        /// Path to the workflow Cargo project directory
        project_path: PathBuf,

        /// Output .so file path
        #[arg(short, long)]
        output: PathBuf,

        /// Additional cargo build flags
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_flags: Vec<String>,
    },
    /// Package a workflow Cargo project into a .cloacina archive
    Package {
        /// Path to the workflow Cargo project directory
        project_path: PathBuf,

        /// Output .cloacina package file path
        #[arg(short, long)]
        output: PathBuf,

        /// Additional cargo build flags
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_flags: Vec<String>,
    },
    /// Inspect a .cloacina package and display its contents
    Inspect {
        /// Path to the .cloacina package file
        package_path: PathBuf,

        /// Output format (human or json)
        #[arg(long, default_value = "human")]
        format: String,
    },
    /// Visualize workflow task dependencies as ASCII diagram
    Visualize {
        /// Path to the .cloacina package file
        package_path: PathBuf,

        /// Show detailed task information
        #[arg(long)]
        details: bool,

        /// Layout style (horizontal, compact)
        #[arg(long, default_value = "horizontal")]
        layout: String,

        /// Output format (ascii, dot)
        #[arg(long, default_value = "ascii")]
        format: String,
    },
    /// Debug and execute tasks from a .cloacina package
    Debug {
        /// Path to the .cloacina package file
        package_path: PathBuf,

        #[command(subcommand)]
        action: DebugAction,
    },
}
