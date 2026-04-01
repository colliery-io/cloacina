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

//! Validation tests for the fidius plugin integration.
//!
//! These tests verify that fidius's safety guarantees (ABI drift detection,
//! wire format validation, graceful rejection of non-fidius dylibs) work
//! correctly in the cloacina context.

use cloacina_workflow_plugin::{PackageTasksMetadata, TaskExecutionRequest, TaskExecutionResult};

/// Find the pre-built debug dylib for the packaged-workflows example.
fn find_packaged_workflow_dylib() -> Option<std::path::PathBuf> {
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::PathBuf::from(&cargo_manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let project_path = workspace_root.join("examples/features/packaged-workflows");
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    // Prefer debug (matches test binary wire format)
    for profile in &["debug", "release"] {
        let path = project_path
            .join("target")
            .join(profile)
            .join(format!("libpackaged_workflow_example.{}", ext));
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Create a temporary file that is NOT a fidius plugin.
fn create_non_fidius_dylib() -> tempfile::NamedTempFile {
    let mut temp = tempfile::Builder::new()
        .suffix(if cfg!(target_os = "macos") {
            ".dylib"
        } else {
            ".so"
        })
        .tempfile()
        .expect("Failed to create temp file");

    // Write some garbage bytes — not a valid dylib at all
    use std::io::Write;
    temp.write_all(b"This is not a valid dynamic library")
        .expect("Failed to write");
    temp
}

#[test]
fn test_non_fidius_dylib_rejected_gracefully() {
    let fake_dylib = create_non_fidius_dylib();

    let result = fidius_host::loader::load_library(fake_dylib.path());

    assert!(result.is_err(), "Loading a non-fidius file should fail");
    let err = result.err().unwrap();
    let err_str = err.to_string();
    // Should get a load error, not a SIGSEGV
    assert!(
        !err_str.is_empty(),
        "Error should have a descriptive message"
    );
}

#[test]
fn test_metadata_fidelity() {
    let dylib_path = match find_packaged_workflow_dylib() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: packaged-workflows example not built");
            return;
        }
    };

    let loaded =
        fidius_host::loader::load_library(&dylib_path).expect("Failed to load plugin library");

    assert!(
        !loaded.plugins.is_empty(),
        "Library should contain at least one plugin"
    );

    let plugin = loaded.plugins.into_iter().next().unwrap();
    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    // Method index 0 = get_task_metadata (zero-arg)
    let metadata: PackageTasksMetadata = handle
        .call_method(0, &())
        .expect("Failed to call get_task_metadata");

    // Verify workflow name matches what the #[workflow] macro embedded
    assert_eq!(metadata.workflow_name, "analytics_workflow");
    assert_eq!(metadata.package_name, "packaged-workflow-example");
    assert!(metadata.package_description.is_some());
    assert!(metadata.package_author.is_some());
    assert!(metadata.workflow_fingerprint.is_some());

    // Verify tasks
    assert!(!metadata.tasks.is_empty(), "Should have at least one task");

    // Check first task has expected fields
    let first_task = &metadata.tasks[0];
    assert!(!first_task.id.is_empty(), "Task ID should not be empty");
    assert!(
        first_task.namespaced_id_template.contains("::"),
        "Namespaced ID should contain ::"
    );
    assert!(
        !first_task.source_location.is_empty(),
        "Source location should not be empty"
    );

    // Verify task IDs from the analytics_workflow example
    let task_ids: Vec<&str> = metadata.tasks.iter().map(|t| t.id.as_str()).collect();
    assert!(
        task_ids.contains(&"extract_data"),
        "Should have extract_data task, got: {:?}",
        task_ids
    );
}

#[test]
fn test_task_execution_fidelity() {
    let dylib_path = match find_packaged_workflow_dylib() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: packaged-workflows example not built");
            return;
        }
    };

    let loaded =
        fidius_host::loader::load_library(&dylib_path).expect("Failed to load plugin library");

    let plugin = loaded.plugins.into_iter().next().unwrap();
    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    // Execute a task with a known context
    let request = TaskExecutionRequest {
        task_name: "extract_data".to_string(),
        context_json: "{}".to_string(),
    };

    // Method index 1 = execute_task (fidius 0.0.5 tuple encoding: single-arg = (T,))
    let result: TaskExecutionResult = handle
        .call_method(1, &(request,))
        .expect("Failed to call execute_task");

    assert!(result.success, "Task should succeed");
    assert!(
        result.context_json.is_some(),
        "Should return updated context"
    );
    assert!(result.error.is_none(), "Should have no error");

    // Verify the context was actually modified by the task
    let ctx: serde_json::Value =
        serde_json::from_str(result.context_json.as_ref().unwrap()).expect("Invalid JSON");
    assert!(
        ctx.get("extracted_records").is_some(),
        "Context should contain extracted_records after extract_data task"
    );
}

#[test]
fn test_unknown_task_returns_error() {
    let dylib_path = match find_packaged_workflow_dylib() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: packaged-workflows example not built");
            return;
        }
    };

    let loaded =
        fidius_host::loader::load_library(&dylib_path).expect("Failed to load plugin library");

    let plugin = loaded.plugins.into_iter().next().unwrap();
    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    let request = TaskExecutionRequest {
        task_name: "nonexistent_task".to_string(),
        context_json: "{}".to_string(),
    };

    // fidius 0.0.5 tuple encoding: single-arg = (T,)
    let result: TaskExecutionResult = handle
        .call_method(1, &(request,))
        .expect("FFI call should succeed even for unknown task");

    assert!(!result.success, "Unknown task should not succeed");
    assert!(
        result.error.is_some(),
        "Should have error message for unknown task"
    );
    assert!(
        result.error.as_ref().unwrap().contains("Unknown task"),
        "Error should mention unknown task"
    );
}

#[test]
fn test_plugin_info_populated() {
    let dylib_path = match find_packaged_workflow_dylib() {
        Some(p) => p,
        None => {
            eprintln!("Skipping: packaged-workflows example not built");
            return;
        }
    };

    let loaded =
        fidius_host::loader::load_library(&dylib_path).expect("Failed to load plugin library");

    assert_eq!(loaded.plugins.len(), 1, "Should have exactly one plugin");

    let plugin = &loaded.plugins[0];
    assert_eq!(
        plugin.info.interface_name, "CloacinaPlugin",
        "Interface name should be CloacinaPlugin"
    );
    assert!(
        plugin.info.interface_hash != 0,
        "Interface hash should be non-zero"
    );
    assert_eq!(
        plugin.info.interface_version, 1,
        "Interface version should be 1"
    );
    assert_eq!(
        plugin.method_count, 2,
        "Should have 2 methods (get_task_metadata, execute_task)"
    );
}
