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

//! Host-Managed Registry Tests
//!
//! Tests for the new host-managed registry approach using the get_task_metadata() FFI function.

use std::ffi::CStr;

// Import the module to test FFI functions
use simple_packaged_demo::data_processing;

#[test]
fn test_get_task_metadata_basic() {
    // Call the new get_task_metadata() FFI function
    let metadata_ptr = unsafe { data_processing::get_task_metadata() };
    assert!(
        !metadata_ptr.is_null(),
        "get_task_metadata() should not return null"
    );

    // Dereference to get the metadata collection
    let metadata = unsafe { &*metadata_ptr };

    // Verify basic properties
    assert_eq!(metadata.task_count, 3, "Should have 3 tasks");
    assert!(!metadata.tasks.is_null(), "Tasks array should not be null");
    assert!(
        !metadata.workflow_name.is_null(),
        "Workflow name should not be null"
    );
    assert!(
        !metadata.package_name.is_null(),
        "Package name should not be null"
    );

    // Verify workflow and package names
    let workflow_name = unsafe { CStr::from_ptr(metadata.workflow_name) };
    let package_name = unsafe { CStr::from_ptr(metadata.package_name) };

    assert_eq!(workflow_name.to_str().unwrap(), "data_processing");
    assert_eq!(package_name.to_str().unwrap(), "simple_demo");
}

#[test]
fn test_get_task_metadata_task_details() {
    // Get metadata
    let metadata_ptr = unsafe { data_processing::get_task_metadata() };
    let metadata = unsafe { &*metadata_ptr };

    // Iterate through tasks and verify their properties
    let tasks_slice =
        unsafe { std::slice::from_raw_parts(metadata.tasks, metadata.task_count as usize) };

    let mut task_names = Vec::new();
    for task in tasks_slice {
        assert!(!task.local_id.is_null(), "Task local_id should not be null");
        assert!(
            !task.namespaced_id_template.is_null(),
            "Task namespaced_id_template should not be null"
        );
        assert!(
            !task.dependencies_json.is_null(),
            "Task dependencies_json should not be null"
        );
        assert!(
            !task.constructor_fn_name.is_null(),
            "Task constructor_fn_name should not be null"
        );
        assert!(
            !task.description.is_null(),
            "Task description should not be null"
        );

        // Get task name
        let task_name = unsafe { CStr::from_ptr(task.local_id) }.to_str().unwrap();
        task_names.push(task_name);

        // Verify namespaced ID template format
        let namespaced_template = unsafe { CStr::from_ptr(task.namespaced_id_template) }
            .to_str()
            .unwrap();
        assert!(namespaced_template.contains("{tenant}::simple_demo::data_processing::"));
        assert!(namespaced_template.ends_with(task_name));

        // Verify dependencies JSON is valid
        let deps_json = unsafe { CStr::from_ptr(task.dependencies_json) }
            .to_str()
            .unwrap();
        assert!(
            deps_json.starts_with('[') && deps_json.ends_with(']'),
            "Dependencies should be JSON array"
        );

        // Verify constructor function name format
        let constructor_name = unsafe { CStr::from_ptr(task.constructor_fn_name) }
            .to_str()
            .unwrap();
        assert!(
            constructor_name.ends_with("_task"),
            "Constructor should end with '_task'"
        );

        println!("Task: {} -> Constructor: {}", task_name, constructor_name);
    }

    // Verify we have the expected tasks
    task_names.sort();
    assert_eq!(
        task_names,
        vec!["collect_data", "generate_report", "process_data"]
    );
}

#[test]
fn test_task_metadata_memory_safety() {
    // Test multiple calls to ensure static data is stable
    let metadata_ptr1 = unsafe { data_processing::get_task_metadata() };
    let metadata_ptr2 = unsafe { data_processing::get_task_metadata() };

    // Should return the same pointer (static data)
    assert_eq!(
        metadata_ptr1, metadata_ptr2,
        "Multiple calls should return same static data"
    );

    let metadata1 = unsafe { &*metadata_ptr1 };
    let metadata2 = unsafe { &*metadata_ptr2 };

    // Verify both references have same data
    assert_eq!(metadata1.task_count, metadata2.task_count);
    assert_eq!(metadata1.tasks, metadata2.tasks);
    assert_eq!(metadata1.workflow_name, metadata2.workflow_name);
    assert_eq!(metadata1.package_name, metadata2.package_name);
}

#[test]
fn test_end_to_end_host_managed_workflow() {
    // Test the complete flow: metadata -> host registration -> workflow creation

    // Step 1: Get task metadata from package (like reconciler would)
    let metadata_ptr = unsafe { data_processing::get_task_metadata() };
    let metadata = unsafe { &*metadata_ptr };

    // Step 2: Register tasks in host registry (like reconciler would)
    register_tasks_from_metadata(metadata, "test_tenant");

    // Step 3: Create workflow via FFI (like reconciler would)
    let tenant_id = std::ffi::CString::new("test_tenant").unwrap();
    let workflow_id = std::ffi::CString::new("data_processing").unwrap();

    let workflow_ptr = unsafe {
        data_processing::cloacina_create_workflow(tenant_id.as_ptr(), workflow_id.as_ptr())
    };

    assert!(!workflow_ptr.is_null());

    let workflow = unsafe { Box::from_raw(workflow_ptr as *mut cloacina::workflow::Workflow) };
    assert_eq!(workflow.get_task_ids().len(), 3);

    println!(
        "End-to-end test successful: {} tasks registered and workflow created",
        workflow.get_task_ids().len()
    );
}

/// Helper to register tasks from metadata (simulates reconciler behavior)
fn register_tasks_from_metadata(
    metadata: &data_processing::TaskMetadataCollection,
    tenant_id: &str,
) {
    use std::ffi::CStr;

    let workflow_name = unsafe { CStr::from_ptr(metadata.workflow_name) }
        .to_str()
        .unwrap();
    let package_name = unsafe { CStr::from_ptr(metadata.package_name) }
        .to_str()
        .unwrap();

    let tasks_slice =
        unsafe { std::slice::from_raw_parts(metadata.tasks, metadata.task_count as usize) };

    for task in tasks_slice {
        let task_id = unsafe { CStr::from_ptr(task.local_id) }.to_str().unwrap();
        let constructor_name = unsafe { CStr::from_ptr(task.constructor_fn_name) }
            .to_str()
            .unwrap();

        let namespace =
            cloacina::TaskNamespace::new(tenant_id, package_name, workflow_name, task_id);

        // Create and register task constructor
        let task_constructor: Box<dyn Fn() -> std::sync::Arc<dyn cloacina::Task> + Send + Sync> =
            match constructor_name {
                "collect_data_task" => Box::new(|| {
                    std::sync::Arc::new(data_processing::collect_data_task())
                        as std::sync::Arc<dyn cloacina::Task>
                }),
                "process_data_task" => Box::new(|| {
                    std::sync::Arc::new(data_processing::process_data_task())
                        as std::sync::Arc<dyn cloacina::Task>
                }),
                "generate_report_task" => Box::new(|| {
                    std::sync::Arc::new(data_processing::generate_report_task())
                        as std::sync::Arc<dyn cloacina::Task>
                }),
                _ => panic!("Unknown constructor: {}", constructor_name),
            };

        cloacina::register_task_constructor(namespace, task_constructor);
    }
}
