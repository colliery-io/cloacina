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

use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::archive::{extract_library_from_package, extract_manifest_from_package};
use crate::cli::{Cli, DebugAction};
use crate::library::{execute_task_from_library, resolve_task_name};
use crate::manifest::PackageManifest;
use crate::utils::{process_environment_variables, should_print, LogLevel};

pub fn debug_package(package_path: PathBuf, action: &DebugAction, cli: &Cli) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        println!("Debug package: {:?}", package_path);
    }

    // Step 1: Validate package file exists
    if !package_path.exists() {
        bail!("Package file does not exist: {:?}", package_path);
    }

    if !package_path.is_file() {
        bail!("Package path is not a file: {:?}", package_path);
    }

    // Step 2: Extract manifest.json from package
    let manifest = extract_manifest_from_package(&package_path)?;

    // Step 3: Handle subcommands
    match action {
        DebugAction::List => {
            debug_list_tasks(&manifest, cli)?;
        }
        DebugAction::Execute {
            task,
            context,
            env_vars,
            env_file,
            include_env,
            env_prefix,
        } => {
            debug_execute_task(
                &package_path,
                &manifest,
                task,
                context,
                env_vars,
                env_file,
                include_env,
                env_prefix,
                cli,
            )?;
        }
    }

    Ok(())
}

fn debug_list_tasks(manifest: &PackageManifest, cli: &Cli) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        if manifest.tasks.is_empty() {
            println!("No tasks defined in this package.");
            return Ok(());
        }

        println!("Available Tasks:");
        for task in &manifest.tasks {
            let status = if task.dependencies.is_empty() {
                "ready to run"
            } else {
                "requires dependencies"
            };

            println!("  {}: {} ({})", task.index, task.id, status);
            if !task.source_location.is_empty() {
                println!("     Source: {}", task.source_location);
            }
            if !task.dependencies.is_empty() {
                println!("     Dependencies: {:?}", task.dependencies);
            }
            if !task.description.is_empty() {
                println!("     Description: {}", task.description);
            }
            println!();
        }

        if !manifest.execution_order.is_empty() {
            println!(
                "Suggested Execution Order: {}",
                manifest.execution_order.join(" → ")
            );
        }
    }

    Ok(())
}

fn debug_execute_task(
    package_path: &PathBuf,
    manifest: &PackageManifest,
    task_identifier: &str,
    context_json: &str,
    env_vars: &[String],
    env_file: &Option<PathBuf>,
    include_env: &bool,
    env_prefix: &Option<String>,
    cli: &Cli,
) -> Result<()> {
    // Step 1: Parse and validate context JSON
    let mut context_value: serde_json::Value = serde_json::from_str(context_json)
        .with_context(|| format!("Invalid JSON context: {}", context_json))?;

    // Step 1a: Process environment variables
    process_environment_variables(
        &mut context_value,
        env_vars,
        env_file,
        include_env,
        env_prefix,
    )?;

    // Step 2: Resolve task name (convert index to name if needed)
    let task_name = resolve_task_name(manifest, task_identifier)?;

    // Convert the potentially modified context back to JSON string
    let final_context_json =
        serde_json::to_string(&context_value).context("Failed to serialize modified context")?;

    if should_print(cli, LogLevel::Info) {
        println!("Executing task: {}", task_name);
        println!("Context: {}", final_context_json);
    }

    // Step 3: Extract .so file from package
    let temp_dir = tempfile::TempDir::new().context("Failed to create temporary directory")?;

    let library_path = extract_library_from_package(package_path, manifest, &temp_dir)?;

    // Step 4: Load library and execute task
    execute_task_from_library(&library_path, &task_name, &final_context_json, cli)?;

    Ok(())
}
