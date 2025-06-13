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

use clap::Parser;
use cloacina_ctl::archive::*;
use cloacina_ctl::cli::Cli;
use cloacina_ctl::manifest::{LibraryInfo, PackageInfo, PackageManifest, TaskInfo};
use std::fs;
use tempfile::TempDir;

fn create_test_cli() -> Cli {
    Cli::try_parse_from(vec!["cloacina-ctl", "inspect", "/test/path"])
        .expect("Failed to parse test CLI")
}

fn create_test_manifest() -> PackageManifest {
    PackageManifest {
        package: PackageInfo {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            description: "Test package".to_string(),
            cloacina_version: "0.2.0".to_string(),
        },
        library: LibraryInfo {
            filename: "libtest.so".to_string(),
            symbols: vec!["cloacina_execute_task".to_string()],
            architecture: "x86_64-unknown-linux-gnu".to_string(),
        },
        tasks: vec![
            TaskInfo {
                index: 0,
                id: "task1".to_string(),
                dependencies: vec![],
                description: "First task".to_string(),
                source_location: "src/lib.rs:10".to_string(),
            },
            TaskInfo {
                index: 1,
                id: "task2".to_string(),
                dependencies: vec!["task1".to_string()],
                description: "Second task".to_string(),
                source_location: "src/lib.rs:20".to_string(),
            },
        ],
        execution_order: vec!["task1".to_string(), "task2".to_string()],
        graph: None,
    }
}

#[test]
fn test_create_archive() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let library_path = temp_dir.path().join("libtest.so");
    let output_path = temp_dir.path().join("test.cloacina");

    // Create a dummy library file
    fs::write(&library_path, b"dummy library content").expect("Failed to write library file");

    let manifest = create_test_manifest();
    let compile_result = cloacina_ctl::manifest::CompileResult {
        so_path: library_path,
        manifest,
    };

    // Create the archive
    let result = create_package_archive(&compile_result, &output_path, &create_test_cli());
    assert!(result.is_ok(), "Archive creation should succeed");
    assert!(output_path.exists(), "Archive file should be created");

    // Verify archive size is reasonable
    let metadata = fs::metadata(&output_path).expect("Failed to get archive metadata");
    assert!(metadata.len() > 0, "Archive should not be empty");
}

#[test]
fn test_extract_manifest() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let library_path = temp_dir.path().join("libtest.so");
    let archive_path = temp_dir.path().join("test.cloacina");

    // Create library and archive
    fs::write(&library_path, b"dummy library content").expect("Failed to write library file");

    let original_manifest = create_test_manifest();
    let compile_result = cloacina_ctl::manifest::CompileResult {
        so_path: library_path,
        manifest: original_manifest.clone(),
    };

    create_package_archive(&compile_result, &archive_path, &create_test_cli())
        .expect("Failed to create archive");

    // Extract and verify manifest
    let extracted_manifest =
        extract_manifest_from_package(&archive_path).expect("Failed to extract manifest");

    assert_eq!(
        extracted_manifest.package.name,
        original_manifest.package.name
    );
    assert_eq!(
        extracted_manifest.package.version,
        original_manifest.package.version
    );
    assert_eq!(
        extracted_manifest.tasks.len(),
        original_manifest.tasks.len()
    );
    assert_eq!(extracted_manifest.tasks[0].id, "task1");
    assert_eq!(extracted_manifest.tasks[1].id, "task2");
}

#[test]
fn test_extract_library() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let library_path = temp_dir.path().join("libtest.so");
    let archive_path = temp_dir.path().join("test.cloacina");
    let extract_dir = TempDir::new().expect("Failed to create extract directory");

    let library_content = b"dummy library content with some data";
    fs::write(&library_path, library_content).expect("Failed to write library file");

    let manifest = create_test_manifest();
    let compile_result = cloacina_ctl::manifest::CompileResult {
        so_path: library_path,
        manifest: manifest.clone(),
    };

    create_package_archive(&compile_result, &archive_path, &create_test_cli())
        .expect("Failed to create archive");

    // Extract library
    let extracted_path = extract_library_from_package(&archive_path, &manifest, &extract_dir)
        .expect("Failed to extract library");

    assert!(extracted_path.exists(), "Extracted library should exist");

    let extracted_content = fs::read(&extracted_path).expect("Failed to read extracted library");
    assert_eq!(
        extracted_content, library_content,
        "Library content should match"
    );
}

#[test]
fn test_extract_nonexistent_archive() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let nonexistent_path = temp_dir.path().join("nonexistent.cloacina");

    let result = extract_manifest_from_package(&nonexistent_path);
    assert!(result.is_err(), "Should fail for nonexistent archive");
}

#[test]
fn test_extract_invalid_archive() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let invalid_path = temp_dir.path().join("invalid.cloacina");

    // Create an invalid archive (not a tar.gz)
    fs::write(&invalid_path, b"invalid archive content").expect("Failed to write invalid file");

    let result = extract_manifest_from_package(&invalid_path);
    assert!(result.is_err(), "Should fail for invalid archive");
}

#[test]
fn test_archive_without_manifest() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let archive_path = temp_dir.path().join("no_manifest.cloacina");

    // Create a valid tar.gz but without manifest.json
    let file = fs::File::create(&archive_path).expect("Failed to create file");
    let gz_encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar_builder = tar::Builder::new(gz_encoder);

    // Add a dummy file instead of manifest
    let dummy_data = b"dummy file";
    let mut header = tar::Header::new_gnu();
    header.set_size(dummy_data.len() as u64);
    header.set_cksum();
    tar_builder
        .append_data(&mut header, "dummy.txt", &dummy_data[..])
        .expect("Failed to add dummy file");

    tar_builder.finish().expect("Failed to finish archive");
    drop(tar_builder);

    let result = extract_manifest_from_package(&archive_path);
    assert!(result.is_err(), "Should fail when manifest.json is missing");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("manifest.json not found"),
        "Error should mention missing manifest"
    );
}
