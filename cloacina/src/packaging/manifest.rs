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

use anyhow::{format_err, Context, Result};
use regex::Regex;
use sha2::Digest;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::str;

use super::types::{
    CargoToml, LibraryInfo, PackageInfo, PackageManifest, TaskInfo, CLOACINA_VERSION,
    EXECUTE_TASK_SYMBOL,
};

/// Generate a package manifest from Cargo.toml and compiled library
pub fn generate_manifest(
    cargo_toml: &CargoToml,
    so_path: &PathBuf,
    target: &Option<String>,
    project_path: &PathBuf,
) -> Result<PackageManifest> {
    let package = cargo_toml
        .package
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing package section in Cargo.toml"))?;

    // Use the provided target triple, or query rustc for host target if none specified
    let architecture = match target {
        Some(t) => {
            validate_target(t)?;
            t.clone()
        }
        None => get_target()?,
    };

    // Get library filename
    let library_filename = so_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid so_path"))?
        .to_string_lossy()
        .to_string();

    // Get file metadata
    let metadata = fs::metadata(so_path)
        .with_context(|| format!("Failed to get metadata for: {:?}", so_path))?;
    let file_size = metadata.len();

    // Calculate checksum
    let checksum = calculate_file_checksum(so_path)?;

    // Extract tasks from source code
    let tasks = extract_tasks_from_source(project_path)?;

    Ok(PackageManifest {
        version: "1.0".to_string(),
        package: PackageInfo {
            name: package.name.clone(),
            version: package.version.clone(),
            description: package.description.clone(),
            authors: package.authors.clone(),
            keywords: package.keywords.clone(),
        },
        library: LibraryInfo {
            filename: library_filename,
            architecture,
            size: file_size,
            checksum,
        },
        tasks,
        cloacina_version: CLOACINA_VERSION.to_string(),
    })
}

fn calculate_file_checksum(file_path: &PathBuf) -> Result<String> {
    use std::io::Read;

    let mut file = fs::File::open(file_path)
        .with_context(|| format!("Failed to open file for checksum: {:?}", file_path))?;

    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .context("Failed to read file for checksum")?;

        if bytes_read == 0 {
            break;
        }

        use sha2::Digest;
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_tasks_from_source(project_path: &PathBuf) -> Result<Vec<TaskInfo>> {
    let src_dir = project_path.join("src");
    let lib_rs = src_dir.join("lib.rs");
    let main_rs = src_dir.join("main.rs");

    // Check lib.rs first, then main.rs if lib.rs doesn't exist
    let source_file = if lib_rs.exists() {
        lib_rs
    } else if main_rs.exists() {
        main_rs
    } else {
        return Ok(Vec::new()); // No tasks if no source file
    };

    let source_content = fs::read_to_string(&source_file)
        .with_context(|| format!("Failed to read source file: {:?}", source_file))?;

    extract_tasks_from_content(&source_content)
}

fn extract_tasks_from_content(content: &str) -> Result<Vec<TaskInfo>> {
    let mut tasks = Vec::new();

    // Look for #[packaged_workflow] followed by function definitions
    let workflow_regex =
        Regex::new(r#"#\[packaged_workflow(?:\([^)]*\))?\]\s*(?:pub\s+)?fn\s+(\w+)"#)
            .context("Failed to compile regex for workflow extraction")?;

    for captures in workflow_regex.captures_iter(content) {
        if let Some(function_name) = captures.get(1) {
            let task_name = function_name.as_str().to_string();

            tasks.push(TaskInfo {
                name: task_name.clone(),
                description: None, // Could be extracted from doc comments in the future
                symbol: EXECUTE_TASK_SYMBOL.to_string(),
            });
        }
    }

    Ok(tasks)
}

fn validate_target(target: &str) -> Result<()> {
    // Basic format check - should have at least 2-4 components separated by hyphens
    let parts: Vec<&str> = target.split('-').collect();
    if parts.len() < 2 || parts.len() > 4 {
        return Err(anyhow::anyhow!(
            "Invalid target triple format: {}. Expected format like 'x86_64-unknown-linux-gnu'",
            target
        ));
    }

    // Check if rustc supports this target
    let output = Command::new("rustc")
        .arg("--print")
        .arg("target-list")
        .output()
        .context("Failed to get supported targets from rustc")?;

    let supported_targets =
        str::from_utf8(&output.stdout).context("rustc target list output is not valid UTF-8")?;

    if !supported_targets.lines().any(|line| line.trim() == target) {
        return Err(anyhow::anyhow!(
            "Target '{}' is not supported by rustc. Run 'rustc --print target-list' to see supported targets.",
            target
        ));
    }

    Ok(())
}

fn get_target() -> Result<String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .context("Failed to run rustc to get the host target")?;
    let output = str::from_utf8(&output.stdout).context("`rustc -vV` didn't return utf8 output")?;

    let field = "host: ";
    let host = output
        .lines()
        .find(|l| l.starts_with(field))
        .map(|l| &l[field.len()..])
        .ok_or_else(|| {
            format_err!(
                "`rustc -vV` didn't have a line for `{}`, got:\n{}",
                field.trim(),
                output
            )
        })?
        .to_string();
    Ok(host)
}
