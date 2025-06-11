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
use anyhow::{Result, Context, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use semver::{Version, VersionReq};
use regex::Regex;
use std::collections::HashMap;
use tar::Builder;
use flate2::{Compression, write::GzEncoder};

const CLOACINA_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageManifest {
    pub package: PackageInfo,
    pub library: LibraryInfo,
    pub tasks: Vec<TaskInfo>,
    pub execution_order: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub abi_version: u32,
    pub cloacina_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibraryInfo {
    pub filename: String,
    pub symbols: Vec<String>,
    pub architecture: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskInfo {
    pub index: u32,
    pub id: String,
    pub dependencies: Vec<String>,
    pub description: String,
    pub source_location: String,
}

#[derive(Debug)]
pub struct CompileResult {
    pub so_path: PathBuf,
    pub manifest: PackageManifest,
}

#[derive(Deserialize, Debug)]
struct CargoToml {
    package: Option<PackageSection>,
    lib: Option<LibSection>,
    dependencies: Option<HashMap<String, DependencySpec>>,
}

#[derive(Deserialize, Debug)]
struct PackageSection {
    name: String,
    version: String,
    #[serde(rename = "rust-version")]
    rust_version: Option<String>,
}

#[derive(Deserialize, Debug)]
struct LibSection {
    #[serde(rename = "crate-type")]
    crate_type: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum DependencySpec {
    Simple(String),
    Detailed {
        version: Option<String>,
        #[allow(dead_code)]
        path: Option<String>,
        #[allow(dead_code)]
        git: Option<String>,
        #[allow(dead_code)]
        branch: Option<String>,
        #[allow(dead_code)]
        tag: Option<String>,
        #[allow(dead_code)]
        rev: Option<String>,
        #[allow(dead_code)]
        features: Option<Vec<String>>,
        #[allow(dead_code)]
        default_features: Option<bool>,
    },
}

#[derive(Parser)]
#[command(
    name = "cloacina-ctl",
    version,
    about = "Command-line interface for Cloacina workflow compilation and management",
    long_about = "A tool for compiling, packaging, inspecting, and debugging Cloacina workflows"
)]
struct Cli {
    /// Target triple for cross-compilation (e.g., x86_64-unknown-linux-gnu)
    #[arg(long, global = true)]
    target: Option<String>,
    
    /// Build profile (debug or release)
    #[arg(long, global = true, default_value = "release")]
    profile: String,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    quiet: bool,
    
    /// Control colored output [auto, always, never]
    #[arg(long, global = true, default_value = "auto")]
    color: String,
    
    /// Number of parallel jobs for compilation
    #[arg(short = 'j', long, global = true)]
    jobs: Option<u32>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a workflow Cargo project into a shared library
    Compile {
        /// Path to the workflow Cargo project directory
        project_path: PathBuf,
        
        /// Output .so file path
        #[arg(short, long)]
        output: PathBuf,
        
        /// Additional cargo build flags
        #[arg(long)]
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
        #[arg(long)]
        cargo_flags: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging level based on verbose/quiet flags
    init_logging(&cli);
    
    match cli.command {
        Commands::Compile {
            ref project_path,
            ref output,
            ref cargo_flags,
        } => {
            let _result = compile_workflow(
                project_path.clone(), 
                output.clone(), 
                cli.target.clone(), 
                cli.profile.clone(), 
                cargo_flags.clone(),
                &cli
            )?;
        }
        Commands::Package {
            ref project_path,
            ref output,
            ref cargo_flags,
        } => {
            package_workflow(
                project_path.clone(),
                output.clone(),
                cli.target.clone(),
                cli.profile.clone(),
                cargo_flags.clone(),
                &cli
            )?;
        }
    }
    
    Ok(())
}

fn init_logging(cli: &Cli) {
    let level = if cli.quiet {
        "error"
    } else if cli.verbose {
        "debug"
    } else {
        "info"
    };
    
    std::env::set_var("RUST_LOG", level);
    // Simple print-based logging for now
}

fn should_print(cli: &Cli, level: LogLevel) -> bool {
    match level {
        LogLevel::Error => true, // Always print errors
        LogLevel::Info => !cli.quiet,
        LogLevel::Debug => cli.verbose && !cli.quiet,
    }
}

#[derive(Debug)]
enum LogLevel {
    #[allow(dead_code)]
    Error,
    Info,
    Debug,
}

fn compile_workflow(
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
    let manifest = generate_manifest(&cargo_toml, &so_path, &target)?;
    
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

fn package_workflow(
    project_path: PathBuf,
    output: PathBuf,
    target: Option<String>,
    profile: String,
    cargo_flags: Vec<String>,
    cli: &Cli,
) -> Result<()> {
    if should_print(cli, LogLevel::Info) {
        println!("Packaging workflow project: {:?}", project_path);
    }
    
    // Step 1: Use compile_workflow to get .so and manifest
    let temp_so = tempfile::NamedTempFile::new()
        .context("Failed to create temporary file for .so")?;
    let temp_so_path = temp_so.path().to_path_buf();
    
    let compile_result = compile_workflow(
        project_path,
        temp_so_path,
        target,
        profile,
        cargo_flags,
        cli,
    )?;
    
    // Step 2: Create package archive
    create_package_archive(&compile_result, &output, cli)?;
    
    if should_print(cli, LogLevel::Info) {
        println!("Package created successfully: {:?}", output);
    }
    
    Ok(())
}

fn create_package_archive(
    compile_result: &CompileResult,
    output: &PathBuf,
    cli: &Cli,
) -> Result<()> {
    // Create the output tar.gz file
    let output_file = fs::File::create(output)
        .with_context(|| format!("Failed to create output file: {:?}", output))?;
    
    let gz_encoder = GzEncoder::new(output_file, Compression::default());
    let mut tar_builder = Builder::new(gz_encoder);
    
    if should_print(cli, LogLevel::Debug) {
        println!("Creating package archive...");
    }
    
    // Add manifest.json to archive
    let manifest_json = serde_json::to_string_pretty(&compile_result.manifest)
        .context("Failed to serialize manifest to JSON")?;
    
    let manifest_bytes = manifest_json.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_cksum();
    
    tar_builder.append_data(&mut header, "manifest.json", manifest_bytes)
        .context("Failed to add manifest.json to archive")?;
    
    if should_print(cli, LogLevel::Debug) {
        println!("Added manifest.json to archive");
    }
    
    // Add .so file to archive
    let so_filename = compile_result.so_path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid .so file path"))?;
    let archive_so_path = so_filename.to_string_lossy().to_string();
    
    tar_builder.append_file(&archive_so_path, &mut fs::File::open(&compile_result.so_path)?)
        .context("Failed to add .so file to archive")?;
    
    if should_print(cli, LogLevel::Debug) {
        println!("Added {} to archive", archive_so_path);
    }
    
    // Finalize the archive
    tar_builder.finish()
        .context("Failed to finalize package archive")?;
    
    Ok(())
}

fn validate_rust_crate_structure(project_path: &PathBuf) -> Result<()> {
    // Check if project path exists and is a directory
    if !project_path.exists() {
        bail!("Project path does not exist: {:?}", project_path);
    }
    
    if !project_path.is_dir() {
        bail!("Project path is not a directory: {:?}", project_path);
    }
    
    // Check for Cargo.toml - the only requirement for a valid Rust crate
    let cargo_toml_path = project_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        bail!("Cargo.toml not found in project directory: {:?}", project_path);
    }
    
    // Let cargo handle validation of the actual source structure during build
    
    Ok(())
}

fn validate_cargo_toml(project_path: &PathBuf) -> Result<CargoToml> {
    let cargo_toml_path = project_path.join("Cargo.toml");
    
    // Read and parse Cargo.toml
    let content = fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("Failed to read Cargo.toml at {:?}", cargo_toml_path))?;
    
    let cargo_toml: CargoToml = toml::from_str(&content)
        .with_context(|| format!("Failed to parse Cargo.toml at {:?}", cargo_toml_path))?;
    
    // Check for package section
    let package = cargo_toml.package.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing [package] section"))?;
    
    println!("Package: {} v{}", package.name, package.version);
    
    // Check for lib section with cdylib crate-type
    let lib = cargo_toml.lib.as_ref()
        .ok_or_else(|| anyhow::anyhow!(
            "Cargo.toml missing [lib] section. Add:\n\n[lib]\ncrate-type = [\"cdylib\"]\n"
        ))?;
    
    let crate_types = lib.crate_type.as_ref()
        .ok_or_else(|| anyhow::anyhow!(
            "Cargo.toml [lib] section missing crate-type. Add:\n\n[lib]\ncrate-type = [\"cdylib\"]\n"
        ))?;
    
    if !crate_types.contains(&"cdylib".to_string()) {
        bail!(
            "Cargo.toml [lib] crate-type must include \"cdylib\". Current: {:?}\n\n\
            Add or update:\n\n[lib]\ncrate-type = [\"cdylib\"]\n",
            crate_types
        );
    }
    
    println!("Found cdylib crate-type: {:?}", crate_types);
    
    Ok(cargo_toml)
}

fn validate_cloacina_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    let dependencies = cargo_toml.dependencies.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No dependencies found in Cargo.toml"))?;
    
    // Check for cloacina dependency
    let cloacina_dep = dependencies.get("cloacina")
        .ok_or_else(|| anyhow::anyhow!(
            "Missing 'cloacina' dependency. Add:\n\n[dependencies]\ncloacina = \"{}\"",
            CLOACINA_VERSION
        ))?;
    
    // Extract version requirement
    let version_req = match cloacina_dep {
        DependencySpec::Simple(version) => version.clone(),
        DependencySpec::Detailed { version, path, .. } => {
            match (version, path) {
                (Some(v), _) => v.clone(),
                (None, Some(_)) => {
                    // Path dependency - assume it's compatible with current version
                    println!("Using path dependency for cloacina (assuming compatible)");
                    format!(">= {}", CLOACINA_VERSION)
                },
                (None, None) => bail!("cloacina dependency must specify either version or path"),
            }
        }
    };
    
    // Parse current cloacina version
    let current_version = Version::parse(CLOACINA_VERSION)
        .with_context(|| format!("Failed to parse current cloacina version: {}", CLOACINA_VERSION))?;
    
    // Parse dependency version requirement
    let version_req = VersionReq::parse(&version_req)
        .with_context(|| format!("Failed to parse cloacina dependency version: {}", version_req))?;
    
    // Check if current version satisfies requirement
    if !version_req.matches(&current_version) {
        bail!(
            "cloacina version mismatch. Project requires: {}, but cloacina-ctl is version: {}",
            version_req,
            current_version
        );
    }
    
    // Additional semver compatibility check for runtime
    // A 0.2.x build will run on 0.3.0 runtime (forward compatible on minor versions)
    println!("cloacina dependency found: {} (compatible with {})", version_req, current_version);
    
    // Check for cloacina-macros dependency
    if let Some(macros_dep) = dependencies.get("cloacina-macros") {
        let macros_version = match macros_dep {
            DependencySpec::Simple(version) => version.clone(),
            DependencySpec::Detailed { version, .. } => {
                version.as_ref().unwrap_or(&"*".to_string()).clone()
            }
        };
        println!("cloacina-macros dependency found: {}", macros_version);
    }
    
    Ok(())
}

fn validate_packaged_workflow_presence(project_path: &PathBuf) -> Result<()> {
    let src_path = project_path.join("src");
    
    // Regex to find #[packaged_workflow] macro usage
    let packaged_workflow_regex = Regex::new(r"#\[packaged_workflow\]")
        .expect("Failed to compile regex");
    
    let mut found_macro = false;
    
    // Walk through all .rs files in src directory
    for entry in std::fs::read_dir(&src_path)
        .with_context(|| format!("Failed to read src directory: {:?}", src_path))? 
    {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;
            
            if packaged_workflow_regex.is_match(&content) {
                found_macro = true;
                println!("Found #[packaged_workflow] macro in: {:?}", path.file_name().unwrap());
                break;
            }
        }
    }
    
    if !found_macro {
        bail!(
            "No #[packaged_workflow] macro found in source files.\n\n\
            Make sure at least one module is annotated with #[packaged_workflow]:\n\n\
            #[packaged_workflow]\n\
            mod my_workflow {{\n\
                // workflow tasks here\n\
            }}"
        );
    }
    
    Ok(())
}

fn validate_rust_version_compatibility(cargo_toml: &CargoToml) -> Result<()> {
    // Get Rust version from rustc
    let rustc_output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .context("Failed to run rustc --version. Is Rust installed?")?;
    
    let rustc_version_str = String::from_utf8(rustc_output.stdout)
        .context("Failed to parse rustc version output")?;
    
    // Parse rustc version (e.g., "rustc 1.75.0 (82e1608df 2023-12-21)")
    let rustc_version_regex = Regex::new(r"rustc (\d+\.\d+\.\d+)")
        .expect("Failed to compile regex");
    
    let rustc_version = rustc_version_regex.captures(&rustc_version_str)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract version from rustc output"))?;
    
    println!("Current Rust version: {}", rustc_version);
    
    // Check if package specifies rust-version
    if let Some(package) = &cargo_toml.package {
        if let Some(required_rust_version) = &package.rust_version {
            // Compare versions
            let current = Version::parse(rustc_version)
                .context("Failed to parse current Rust version")?;
            let required = Version::parse(required_rust_version)
                .context("Failed to parse required Rust version")?;
            
            if current < required {
                bail!(
                    "Rust version mismatch. Project requires: {}, but current version is: {}",
                    required_rust_version,
                    rustc_version
                );
            }
            
            println!("Rust version {} satisfies requirement: {}", rustc_version, required_rust_version);
        }
    }
    
    Ok(())
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
    cmd.arg("build")
       .arg("--lib")
       .current_dir(project_path);
    
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
    
    let command_str = format!("cargo {}", 
        cmd.get_args().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(" "));
    
    if should_print(cli, LogLevel::Info) {
        println!("Running: {}", command_str);
    }
    
    // Execute cargo build
    let output = cmd.output()
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

fn generate_manifest(
    cargo_toml: &CargoToml,
    so_path: &PathBuf,
    target: &Option<String>,
) -> Result<PackageManifest> {
    let package = cargo_toml.package.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Missing package section in Cargo.toml"))?;
    
    // Extract architecture from target or use current platform
    let architecture = if let Some(target_triple) = target {
        target_triple.clone()
    } else {
        get_current_architecture()
    };
    
    // Get library filename
    let library_filename = so_path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid so_path"))?
        .to_string_lossy()
        .to_string();
    
    let manifest = PackageManifest {
        package: PackageInfo {
            name: package.name.clone(),
            version: package.version.clone(),
            description: format!("Packaged workflow: {}", package.name),
            abi_version: 1,
            cloacina_version: CLOACINA_VERSION.to_string(),
        },
        library: LibraryInfo {
            filename: library_filename,
            symbols: vec!["cloacina_execute_task".to_string()],
            architecture,
        },
        tasks: vec![], // TODO: Extract from source code
        execution_order: vec![], // TODO: Generate from task dependencies
    };
    
    Ok(manifest)
}

fn get_current_architecture() -> String {
    // Get the current target triple
    if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
        "x86_64-unknown-linux-gnu".to_string()
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "macos") {
        "x86_64-apple-darwin".to_string()
    } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
        "aarch64-apple-darwin".to_string()
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    fn create_test_project(cargo_toml_content: &str) -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Create Cargo.toml
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml_content)
            .expect("Failed to write Cargo.toml");
        
        // Create src directory
        fs::create_dir(temp_dir.path().join("src"))
            .expect("Failed to create src directory");
        
        // Create lib.rs
        fs::write(temp_dir.path().join("src/lib.rs"), "// Test library")
            .expect("Failed to write lib.rs");
        
        temp_dir
    }
    
    #[test]
    fn test_valid_cargo_toml_with_cdylib() {
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let result = validate_cargo_toml(&temp_dir.path().to_path_buf());
        
        assert!(result.is_ok(), "Should accept valid cdylib configuration: {:?}", result.err());
    }
    
    #[test]
    fn test_valid_cargo_toml_with_multiple_crate_types() {
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["lib", "cdylib"]
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let result = validate_cargo_toml(&temp_dir.path().to_path_buf());
        
        assert!(result.is_ok(), "Should accept cdylib among multiple crate types");
    }
    
    #[test]
    fn test_missing_lib_section() {
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let result = validate_cargo_toml(&temp_dir.path().to_path_buf());
        
        assert!(result.is_err(), "Should reject missing [lib] section");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("[lib]"), "Error should mention missing [lib] section");
    }
    
    #[test]
    fn test_missing_crate_type() {
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
name = "test_workflow"
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let result = validate_cargo_toml(&temp_dir.path().to_path_buf());
        
        assert!(result.is_err(), "Should reject missing crate-type");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("crate-type"), "Error should mention missing crate-type");
    }
    
    #[test]
    fn test_wrong_crate_type() {
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["lib"]
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let result = validate_cargo_toml(&temp_dir.path().to_path_buf());
        
        assert!(result.is_err(), "Should reject non-cdylib crate-type");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("cdylib"), "Error should mention required cdylib");
    }
    
    #[test]
    fn test_missing_package_section() {
        let cargo_toml = r#"
[lib]
crate-type = ["cdylib"]
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let result = validate_cargo_toml(&temp_dir.path().to_path_buf());
        
        assert!(result.is_err(), "Should reject missing [package] section");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("package"), "Error should mention missing [package] section");
    }
    
    #[test]
    fn test_rust_crate_structure_validation() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Test missing directory
        let nonexistent = temp_dir.path().join("nonexistent");
        let result = validate_rust_crate_structure(&nonexistent);
        assert!(result.is_err(), "Should reject nonexistent directory");
        
        // Test missing Cargo.toml
        let empty_dir = TempDir::new().expect("Failed to create temp directory");
        let result = validate_rust_crate_structure(&empty_dir.path().to_path_buf());
        assert!(result.is_err(), "Should reject directory without Cargo.toml");
        
        // Test valid structure (just needs Cargo.toml)
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"")
            .expect("Failed to write Cargo.toml");
        let result = validate_rust_crate_structure(&temp_dir.path().to_path_buf());
        assert!(result.is_ok(), "Should accept valid crate with Cargo.toml");
    }
    
    #[test]
    fn test_cloacina_compatibility_valid() {
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
cloacina = "0.2.0-alpha.5"
cloacina-macros = "0.2.0-alpha.5"
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let cargo_toml = validate_cargo_toml(&temp_dir.path().to_path_buf()).unwrap();
        let result = validate_cloacina_compatibility(&cargo_toml);
        
        assert!(result.is_ok(), "Should accept matching cloacina version: {:?}", result.err());
    }
    
    #[test]
    fn test_cloacina_compatibility_missing() {
        let cargo_toml = r#"
[package]
name = "test-workflow"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = "1.0"
"#;
        
        let temp_dir = create_test_project(cargo_toml);
        let cargo_toml = validate_cargo_toml(&temp_dir.path().to_path_buf()).unwrap();
        let result = validate_cloacina_compatibility(&cargo_toml);
        
        assert!(result.is_err(), "Should reject missing cloacina dependency");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Missing 'cloacina' dependency"));
    }
    
    #[test]
    fn test_packaged_workflow_presence() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Create Cargo.toml
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"")
            .expect("Failed to write Cargo.toml");
        
        // Create src directory
        fs::create_dir(temp_dir.path().join("src"))
            .expect("Failed to create src directory");
        
        // Create lib.rs with packaged_workflow macro
        fs::write(
            temp_dir.path().join("src/lib.rs"), 
            "#[packaged_workflow]\nmod my_workflow {}"
        ).expect("Failed to write lib.rs");
        
        let result = validate_packaged_workflow_presence(&temp_dir.path().to_path_buf());
        assert!(result.is_ok(), "Should find packaged_workflow macro");
    }
    
    #[test]
    fn test_packaged_workflow_missing() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        
        // Create Cargo.toml
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"")
            .expect("Failed to write Cargo.toml");
        
        // Create src directory
        fs::create_dir(temp_dir.path().join("src"))
            .expect("Failed to create src directory");
        
        // Create lib.rs without packaged_workflow macro
        fs::write(
            temp_dir.path().join("src/lib.rs"), 
            "// No macro here"
        ).expect("Failed to write lib.rs");
        
        let result = validate_packaged_workflow_presence(&temp_dir.path().to_path_buf());
        assert!(result.is_err(), "Should reject missing packaged_workflow macro");
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("#[packaged_workflow]"));
    }
}