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

//! Implementation of `cloacinactl package` subcommands.

use anyhow::{anyhow, Context, Result};
use cloacina::security::{
    verify_package_offline, DbKeyManager, DbPackageSigner, DetachedSignature, PackageSigner,
    SignatureSource,
};
use pyo3::prelude::*;
use std::path::{Path, PathBuf};
use tracing::info;

use super::{connect_db, parse_uuid, read_master_key};

/// Build a .cloacina package.
///
/// For Python projects (detected by `pyproject.toml` with `[tool.cloaca]`),
/// uses the pure Rust builder — no `pip install cloaca` required.
/// For Rust projects, delegates to cargo compilation.
pub async fn build(output: &str, targets: &[String], dry_run: bool, verbose: bool) -> Result<()> {
    let project_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Detect Python project: pyproject.toml with [tool.cloaca]
    let pyproject_path = project_dir.join("pyproject.toml");
    let is_python = if pyproject_path.exists() {
        let content =
            std::fs::read_to_string(&pyproject_path).context("Failed to read pyproject.toml")?;
        let table: toml::Table = content.parse().context("Failed to parse pyproject.toml")?;
        table
            .get("tool")
            .and_then(|v| v.as_table())
            .and_then(|t| t.get("cloaca"))
            .is_some()
    } else {
        false
    };

    if is_python {
        build_python(output, targets, dry_run, verbose).await
    } else {
        build_rust(output, targets, dry_run, verbose).await
    }
}

/// Build a Python .cloacina package using the pure Rust builder.
async fn build_python(
    output: &str,
    targets: &[String],
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    let project_dir = std::env::current_dir()?;

    if dry_run {
        let content = std::fs::read_to_string(project_dir.join("pyproject.toml"))?;
        let table: toml::Table = content.parse()?;
        let project = table.get("project").and_then(|v| v.as_table());
        let name = project
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let version = project
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        info!("Dry run — would build Python package:");
        info!("  Name:    {}", name);
        info!("  Version: {}", version);
        info!("  Output:  {}", output);
        return Ok(());
    }

    let config = cloacina::packaging::PythonBuildConfig {
        project_dir,
        output_dir: std::path::PathBuf::from(output),
        targets: targets.to_vec(),
        verbose,
    };

    let result = cloacina::packaging::build_python_package(&config)
        .context("Failed to build Python package")?;

    info!(
        "Built {}-{}.cloacina ({})",
        result.package_name, result.version, result.fingerprint
    );

    // Validate with Python-specific validator
    let package_data = std::fs::read(&result.archive_path)?;
    let manifest = cloacina::registry::loader::python_loader::peek_manifest(&package_data)
        .map_err(|e| anyhow!("Failed to read manifest from built package: {}", e))?;

    let validator = cloacina::registry::loader::validator::PackageValidator::new()
        .map_err(|e| anyhow!("Failed to create validator: {}", e))?;
    let validation = validator.validate_python_package(&package_data, &manifest);

    if !validation.is_valid {
        anyhow::bail!(
            "Package validation failed:\n  - {}",
            validation.errors.join("\n  - ")
        );
    }

    info!("Package validation passed");
    Ok(())
}

/// Build a Rust .cloacina package (existing path via PyO3/cargo).
async fn build_rust(output: &str, targets: &[String], dry_run: bool, verbose: bool) -> Result<()> {
    let output = output.to_string();
    let targets = targets.to_vec();

    Python::with_gil(|py| {
        let mut args: Vec<String> = vec!["-o".to_string(), output.clone()];

        for target in &targets {
            args.push("--target".to_string());
            args.push(target.clone());
        }

        if dry_run {
            args.push("--dry-run".to_string());
        }

        if verbose {
            args.push("--verbose".to_string());
        }

        let build_mod = py.import("cloaca.cli.build").map_err(|e| {
            anyhow!(
                "Failed to import cloaca.cli.build: {}\n\
                 Is the cloaca package installed in this Python environment?",
                e
            )
        })?;

        let build_cmd = build_mod
            .getattr("build")
            .map_err(|e| anyhow!("Failed to get build command from cloaca.cli.build: {}", e))?;

        let py_args = pyo3::types::PyList::new(py, &args)
            .map_err(|e| anyhow!("Failed to create Python args list: {}", e))?;

        let kwargs = pyo3::types::PyDict::new(py);
        kwargs
            .set_item("standalone_mode", false)
            .map_err(|e| anyhow!("Failed to set standalone_mode: {}", e))?;

        build_cmd
            .call((py_args,), Some(&kwargs))
            .map_err(|e| anyhow!("cloaca build failed: {}", e))?;

        Ok::<(), anyhow::Error>(())
    })?;

    if !dry_run {
        validate_output_packages(&output).await?;
    }

    Ok(())
}

/// Find and validate all .cloacina packages in the output directory.
async fn validate_output_packages(output_dir: &str) -> Result<()> {
    use cloacina::registry::loader::validator::PackageValidator;

    let output_path = std::path::Path::new(output_dir);

    // Collect .cloacina files from the output directory
    let packages: Vec<_> = if output_path.is_file()
        && output_path.extension().and_then(|e| e.to_str()) == Some("cloacina")
    {
        vec![output_path.to_path_buf()]
    } else if output_path.is_dir() {
        std::fs::read_dir(output_path)
            .context("Failed to read output directory")?
            .flatten()
            .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("cloacina"))
            .map(|e| e.path())
            .collect()
    } else {
        return Ok(()); // Nothing to validate
    };

    if packages.is_empty() {
        return Ok(());
    }

    let validator =
        PackageValidator::new().map_err(|e| anyhow!("Failed to create validator: {}", e))?;

    for package_path in &packages {
        info!("Validating {}...", package_path.display());

        let package_data = std::fs::read(package_path)
            .with_context(|| format!("Failed to read {}", package_path.display()))?;

        let result = validator
            .validate_package(&package_data, None)
            .await
            .map_err(|e| anyhow!("Validation error: {}", e))?;

        if !result.is_valid {
            let errors = result.errors.join("\n  - ");
            anyhow::bail!(
                "Package validation failed for {}:\n  - {}\n\n\
                 See https://docs.cloacina.dev/explanation/packaged-workflow-validation/ \
                 for troubleshooting guidance.",
                package_path.display(),
                errors
            );
        }

        for warning in &result.warnings {
            info!("Validation warning: {}", warning);
        }

        info!(
            "Package {} validated (FFI smoke test OK)",
            package_path.display()
        );
    }

    Ok(())
}

/// Sign a package and write a detached .sig file.
pub async fn sign(database_url: &str, package: &str, key_id: &str, store: bool) -> Result<()> {
    let package_path = Path::new(package);
    anyhow::ensure!(package_path.exists(), "Package file not found: {}", package);

    let key_uuid = parse_uuid(key_id).context("Invalid key ID")?;
    let master_key = read_master_key()?;
    let dal = connect_db(database_url)?;

    let signer = DbPackageSigner::new(dal);
    let sig_info = signer
        .sign_package_with_db_key(package_path, key_uuid, &master_key, store)
        .await
        .context("Failed to sign package")?;

    // Write detached signature
    let sig_path = PathBuf::from(format!("{}.sig", package));
    let detached = DetachedSignature::from_signature_info(&sig_info);
    detached
        .write_to_file(&sig_path)
        .context("Failed to write signature file")?;

    info!(
        "Package signed successfully\n  Package: {}\n  Signature: {}\n  Key fingerprint: {}\n  Package hash: {}",
        package,
        sig_path.display(),
        sig_info.key_fingerprint,
        sig_info.package_hash
    );

    if store {
        info!("Signature stored in database");
    }

    Ok(())
}

/// Verify a package signature.
pub async fn verify(
    database_url: Option<&str>,
    org_id: Option<&str>,
    package: &str,
    signature_path: Option<&str>,
    public_key_path: Option<&str>,
) -> Result<()> {
    let package_path = Path::new(package);
    anyhow::ensure!(package_path.exists(), "Package file not found: {}", package);

    // Offline mode: use public key file directly
    if let Some(pk_path) = public_key_path {
        let sig_path = signature_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{}.sig", package)));

        anyhow::ensure!(
            sig_path.exists(),
            "Signature file not found: {}",
            sig_path.display()
        );

        let pem = std::fs::read_to_string(pk_path)
            .with_context(|| format!("Failed to read public key file: {}", pk_path))?;

        let public_key = DbKeyManager::decode_public_key_pem(&pem).map_err(|e| anyhow!("{}", e))?;

        let result = verify_package_offline(package_path, &sig_path, &public_key)
            .context("Verification failed")?;

        info!(
            "Verification succeeded (offline)\n  Package hash: {}\n  Signer: {}",
            result.package_hash, result.signer_fingerprint
        );
        return Ok(());
    }

    // Online mode: use database
    let database_url = database_url.context(
        "Database URL is required for online verification. Use --public-key for offline mode.",
    )?;
    let org_id = org_id.context("Organization ID is required for online verification")?;
    let org_uuid = parse_uuid(org_id).context("Invalid organization ID")?;

    let dal = connect_db(database_url)?;
    let signer = DbPackageSigner::new(dal.clone());
    let key_manager = DbKeyManager::new(dal);

    let sig_source = match signature_path {
        Some(path) => SignatureSource::DetachedFile {
            path: PathBuf::from(path),
        },
        None => SignatureSource::Auto,
    };

    let result = cloacina::security::verify_package(
        package_path,
        org_uuid,
        sig_source,
        &signer,
        &key_manager,
    )
    .await
    .context("Verification failed")?;

    info!(
        "Verification succeeded\n  Package hash: {}\n  Signer: {}\n  Signer name: {}",
        result.package_hash,
        result.signer_fingerprint,
        result.signer_name.as_deref().unwrap_or("(unknown)")
    );

    Ok(())
}

/// Inspect a detached signature file.
pub fn inspect(signature_path: &str) -> Result<()> {
    let sig = DetachedSignature::read_from_file(Path::new(signature_path))
        .context("Failed to read signature file")?;

    println!("Signature File: {}", signature_path);
    println!("  Format version: {}", sig.version);
    println!("  Algorithm:      {}", sig.algorithm);
    println!("  Package hash:   {}", sig.package_hash);
    println!("  Key fingerprint: {}", sig.key_fingerprint);
    println!("  Signed at:      {}", sig.signed_at);

    Ok(())
}
