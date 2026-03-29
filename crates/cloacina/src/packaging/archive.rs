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

use anyhow::{Context, Result};
use flate2::{write::GzEncoder, Compression};
use std::fs;
use std::path::PathBuf;
use tar::Builder;

use super::types::{CompileResult, MANIFEST_FILENAME};

/// Create a package archive from compilation results.
///
/// The archive contains:
/// - `manifest.json` — Manifest (unified format for Rust and Python)
/// - The compiled dynamic library at the path specified in `manifest.rust.library_path`
pub fn create_package_archive(compile_result: &CompileResult, output: &PathBuf) -> Result<()> {
    // Create the output tar.gz file
    let output_file = fs::File::create(output)
        .with_context(|| format!("Failed to create output file: {:?}", output))?;

    let gz_encoder = GzEncoder::new(output_file, Compression::default());
    let mut tar_builder = Builder::new(gz_encoder);

    // Add manifest.json to archive (Manifest format)
    let manifest_json = serde_json::to_string_pretty(&compile_result.manifest)
        .context("Failed to serialize manifest to JSON")?;

    let manifest_bytes = manifest_json.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_cksum();

    tar_builder
        .append_data(&mut header, MANIFEST_FILENAME, manifest_bytes)
        .context("Failed to add manifest.json to archive")?;

    // Add .so/.dylib file to archive using the path from the manifest
    let library_path = compile_result
        .manifest
        .rust
        .as_ref()
        .map(|r| r.library_path.as_str())
        .unwrap_or("lib/library.so");

    tar_builder
        .append_file(library_path, &mut fs::File::open(&compile_result.so_path)?)
        .context("Failed to add library file to archive")?;

    // Finalize the archive
    tar_builder
        .finish()
        .context("Failed to finalize package archive")?;

    Ok(())
}
