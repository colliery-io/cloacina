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
use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;

use crate::manifest::{PackageManifest, MANIFEST_FILENAME};

pub fn extract_manifest_from_package(package_path: &PathBuf) -> Result<PackageManifest> {
    // Open the .cloacina file (tar.gz)
    let file = fs::File::open(package_path)
        .with_context(|| format!("Failed to open package file: {:?}", package_path))?;

    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);

    // Look for manifest.json in the archive
    for entry in archive.entries()? {
        let mut entry = entry.context("Failed to read archive entry")?;
        let path = entry.path().context("Failed to get entry path")?;

        if path == std::path::Path::new(MANIFEST_FILENAME) {
            // Read manifest content
            let mut manifest_content = String::new();
            entry
                .read_to_string(&mut manifest_content)
                .context("Failed to read manifest.json content")?;

            // Parse JSON
            let manifest: PackageManifest =
                serde_json::from_str(&manifest_content).context("Failed to parse manifest.json")?;

            return Ok(manifest);
        }
    }

    bail!("manifest.json not found in package archive")
}

pub fn extract_library_from_package(
    package_path: &PathBuf,
    manifest: &PackageManifest,
    temp_dir: &tempfile::TempDir,
) -> Result<PathBuf> {
    // Open the .cloacina file (tar.gz)
    let file = fs::File::open(package_path)
        .with_context(|| format!("Failed to open package file: {:?}", package_path))?;

    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);

    // Look for the library file in the archive
    for entry in archive.entries()? {
        let mut entry = entry.context("Failed to read archive entry")?;
        let path = entry.path().context("Failed to get entry path")?;

        // Check if this matches the library filename
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let manifest_filename = std::path::Path::new(&manifest.library.filename)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if filename == manifest_filename || path.to_str() == Some(&manifest.library.filename) {
            // Extract to temporary directory
            let extract_path = temp_dir.path().join(filename);
            let mut output_file = fs::File::create(&extract_path).with_context(|| {
                format!(
                    "Failed to create extracted library file: {:?}",
                    extract_path
                )
            })?;

            std::io::copy(&mut entry, &mut output_file)
                .context("Failed to extract library file")?;

            return Ok(extract_path);
        }
    }

    bail!(
        "Library file '{}' not found in package archive",
        manifest.library.filename
    );
}
