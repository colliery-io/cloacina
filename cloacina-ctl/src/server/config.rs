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

use crate::cli::ConfigCommands;
use crate::config::validation::Validate;
use crate::config::{defaults, ConfigLoader};
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

pub async fn handle_config_command(command: ConfigCommands) -> Result<()> {
    match command {
        ConfigCommands::Generate { output, force } => generate_config(output, force).await,
        ConfigCommands::Validate { config } => validate_config(config).await,
        ConfigCommands::Show { config } => show_config(config).await,
    }
}

async fn generate_config(output: Option<PathBuf>, force: bool) -> Result<()> {
    let output_path = output.unwrap_or_else(|| PathBuf::from("cloacina.toml"));

    // Check if file exists and force flag
    if output_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "Configuration file '{}' already exists. Use --force to overwrite.",
            output_path.display()
        ));
    }

    // Generate configuration content
    let content = defaults::generate_default_config_toml()
        .context("Failed to generate TOML configuration")?;

    // Write to file
    fs::write(&output_path, content)
        .with_context(|| format!("Failed to write configuration to {}", output_path.display()))?;

    println!(
        "{} Generated default configuration: {}",
        "✓".green().bold(),
        output_path.display().to_string().cyan()
    );

    Ok(())
}

async fn validate_config(config_path: Option<PathBuf>) -> Result<()> {
    let loader = ConfigLoader::new();

    // Load the configuration
    let config = loader
        .load_config(config_path.as_deref())
        .context("Failed to load configuration")?;

    // Validate the configuration
    match config.validate() {
        Ok(()) => {
            println!("{} Configuration is valid", "✓".green().bold());
            Ok(())
        }
        Err(validation_error) => {
            println!("{} Configuration validation failed:", "✗".red().bold());
            println!("  {}", validation_error.to_string().red());
            std::process::exit(1);
        }
    }
}

async fn show_config(config_path: Option<PathBuf>) -> Result<()> {
    let loader = ConfigLoader::new();

    // Load and resolve the configuration
    let config = loader
        .load_config(config_path.as_deref())
        .context("Failed to load configuration")?;

    // Validate before showing
    if let Err(validation_error) = config.validate() {
        println!(
            "{} Warning: Configuration has validation errors:",
            "⚠".yellow().bold()
        );
        println!("  {}", validation_error.to_string().yellow());
        println!();
    }

    // Format and display as TOML
    let output =
        toml::to_string_pretty(&config).context("Failed to serialize configuration as TOML")?;

    println!("{}", output);
    Ok(())
}
