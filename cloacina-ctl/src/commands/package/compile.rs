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

use crate::cli::Cli;
use crate::manifest::{generation::generate_manifest, CompileResult};
use crate::utils::{should_print, LogLevel};
use crate::validation::{
    validate_cargo_toml, validate_cloacina_compatibility, validate_packaged_workflow_presence,
    validate_rust_crate_structure, validate_rust_version_compatibility,
};

pub fn compile_workflow(
    project_path: PathBuf,
    output: PathBuf,
    target: Option<String>,
    profile: String,
    cargo_flags: Vec<String>,
    cli: &Cli,
) -> Result<CompileResult> {
    if should_print(cli, LogLevel::Info) {
        println!("Compiling workflow project: {:?}", project_path);
    }

    // Step 1: Validate it's a valid Rust crate
    validate_rust_crate_structure(&project_path)?;

    // Step 2: Validate Cargo.toml for cdylib requirement
    let cargo_toml = validate_cargo_toml(&project_path)?;

    // Step 3: Validate cloacina compatibility
    validate_cloacina_compatibility(&cargo_toml)?;

    // Step 4: Check for packaged_workflow macros
    validate_packaged_workflow_presence(&project_path)?;

    // Step 5: Validate Rust version compatibility
    validate_rust_version_compatibility(&cargo_toml)?;

    if should_print(cli, LogLevel::Info) {
        println!("All validations passed");
    }

    // Step 6: Execute cargo build
    let so_path = execute_cargo_build(&project_path, target.as_ref(), &profile, &cargo_flags, cli)?;

    // Step 7: Generate manifest data
    let manifest = generate_manifest(&cargo_toml, &so_path, &target, &project_path)?;

    // Step 8: Copy .so file to output location
    copy_output_file(&so_path, &output)?;

    if should_print(cli, LogLevel::Info) {
        println!("Compilation successful: {:?}", output);
    }

    Ok(CompileResult {
        so_path: output,
        manifest,
    })
}

fn execute_cargo_build(
    project_path: &PathBuf,
    target: Option<&String>,
    profile: &str,
    cargo_flags: &[String],
    cli: &Cli,
) -> Result<PathBuf> {
    println!("Building with cargo...");

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("build").arg("--lib").current_dir(project_path);

    // Add profile flag
    if profile == "release" {
        cmd.arg("--release");
    }

    // Add target flag if specified
    if let Some(target_triple) = target {
        cmd.arg("--target").arg(target_triple);
        println!("Cross-compiling for target: {}", target_triple);
    }

    // Add jobs flag if specified
    if let Some(jobs) = cli.jobs {
        cmd.arg("--jobs").arg(jobs.to_string());
        if should_print(cli, LogLevel::Debug) {
            println!("Using {} parallel jobs", jobs);
        }
    }

    // Add any additional cargo flags
    for flag in cargo_flags {
        cmd.arg(flag);
    }

    let command_str = format!(
        "cargo {}",
        cmd.get_args()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );

    if should_print(cli, LogLevel::Info) {
        println!("Running: {}", command_str);
    }

    // Execute cargo build
    let output = cmd
        .output()
        .context("Failed to execute cargo build. Is cargo installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        bail!(
            "Cargo build failed with exit code {:?}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
            output.status.code(),
            stdout,
            stderr
        );
    }

    println!("Cargo build completed successfully");

    // Find the resulting .so file
    find_compiled_library(project_path, target, profile)
}

fn find_compiled_library(
    project_path: &PathBuf,
    target: Option<&String>,
    profile: &str,
) -> Result<PathBuf> {
    // Determine the target directory structure
    let target_dir = project_path.join("target");

    let build_dir = if let Some(target_triple) = target {
        target_dir.join(target_triple).join(profile)
    } else {
        target_dir.join(profile)
    };

    if !build_dir.exists() {
        bail!("Build directory not found: {:?}", build_dir);
    }

    // Look for .so files (on Unix) or .dll files (on Windows)
    let extensions = if cfg!(target_os = "windows") {
        vec!["dll"]
    } else {
        vec!["so", "dylib"]
    };

    for extension in &extensions {
        for entry in std::fs::read_dir(&build_dir)
            .with_context(|| format!("Failed to read build directory: {:?}", build_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if ext == *extension {
                    println!("Found compiled library: {:?}", path);
                    return Ok(path);
                }
            }
        }
    }

    bail!(
        "No compiled library found in build directory: {:?}\n\
        Expected files with extensions: {:?}",
        build_dir,
        extensions
    );
}

fn copy_output_file(source: &PathBuf, destination: &PathBuf) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
    }

    std::fs::copy(source, destination)
        .with_context(|| format!("Failed to copy {:?} to {:?}", source, destination))?;

    println!("Copied library to: {:?}", destination);

    Ok(())
}
