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

use cloacina_ctl::manifest::*;
use serde_json;
use std::collections::HashMap;

#[test]
fn test_package_manifest_serialization() {
    let manifest = PackageManifest {
        package: PackageInfo {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            description: "Test package description".to_string(),
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
    };

    // Test serialization
    let json = serde_json::to_string_pretty(&manifest).expect("Failed to serialize manifest");
    assert!(json.contains("test-package"));
    assert!(json.contains("libtest.so"));
    assert!(json.contains("task1"));
    assert!(json.contains("task2"));

    // Test deserialization
    let deserialized: PackageManifest =
        serde_json::from_str(&json).expect("Failed to deserialize manifest");

    assert_eq!(deserialized.package.name, manifest.package.name);
    assert_eq!(deserialized.package.version, manifest.package.version);
    assert_eq!(deserialized.library.filename, manifest.library.filename);
    assert_eq!(deserialized.tasks.len(), manifest.tasks.len());
    assert_eq!(deserialized.tasks[0].id, manifest.tasks[0].id);
    assert_eq!(
        deserialized.tasks[1].dependencies,
        manifest.tasks[1].dependencies
    );
}

#[test]
fn test_package_manifest_with_graph_data() {
    let graph_data = cloacina::WorkflowGraphData {
        nodes: vec![cloacina::GraphNode {
            id: "task1".to_string(),
            data: cloacina::TaskNode {
                id: "task1".to_string(),
                name: "Task 1".to_string(),
                description: Some("First task".to_string()),
                source_location: Some("src/lib.rs:10".to_string()),
                metadata: HashMap::new(),
            },
        }],
        edges: vec![],
        metadata: cloacina::GraphMetadata {
            task_count: 1,
            edge_count: 0,
            has_cycles: false,
            depth_levels: 1,
            root_tasks: vec!["task1".to_string()],
            leaf_tasks: vec!["task1".to_string()],
        },
    };

    let manifest = PackageManifest {
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
        tasks: vec![TaskInfo {
            index: 0,
            id: "task1".to_string(),
            dependencies: vec![],
            description: "First task".to_string(),
            source_location: "src/lib.rs:10".to_string(),
        }],
        execution_order: vec!["task1".to_string()],
        graph: Some(graph_data.clone()),
    };

    // Test serialization with graph data
    let json = serde_json::to_string_pretty(&manifest).expect("Failed to serialize manifest");
    assert!(json.contains("graph"));
    assert!(json.contains("nodes"));
    assert!(json.contains("edges"));
    assert!(json.contains("metadata"));

    // Test deserialization
    let deserialized: PackageManifest =
        serde_json::from_str(&json).expect("Failed to deserialize manifest");

    assert!(deserialized.graph.is_some());
    let graph = deserialized.graph.unwrap();
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].id, "task1");
    assert_eq!(graph.metadata.task_count, 1);
}

#[test]
fn test_task_info_validation() {
    let task = TaskInfo {
        index: 0,
        id: "test_task".to_string(),
        dependencies: vec!["dep1".to_string(), "dep2".to_string()],
        description: "Test task description".to_string(),
        source_location: "src/lib.rs:42".to_string(),
    };

    // Test basic properties
    assert_eq!(task.index, 0);
    assert_eq!(task.id, "test_task");
    assert_eq!(task.dependencies.len(), 2);
    assert!(task.dependencies.contains(&"dep1".to_string()));
    assert!(task.dependencies.contains(&"dep2".to_string()));
    assert!(!task.description.is_empty());
    assert!(!task.source_location.is_empty());
}

#[test]
fn test_library_info_validation() {
    let library = LibraryInfo {
        filename: "libworkflow.so".to_string(),
        symbols: vec![
            "cloacina_execute_task".to_string(),
            "cloacina_get_task_metadata".to_string(),
        ],
        architecture: "aarch64-apple-darwin".to_string(),
    };

    assert_eq!(library.filename, "libworkflow.so");
    assert_eq!(library.symbols.len(), 2);
    assert!(library
        .symbols
        .contains(&"cloacina_execute_task".to_string()));
    assert_eq!(library.architecture, "aarch64-apple-darwin");
}

#[test]
fn test_package_info_validation() {
    let package = PackageInfo {
        name: "my-workflow".to_string(),
        version: "2.1.3".to_string(),
        description: "A complex workflow package".to_string(),
        cloacina_version: "0.2.0-alpha.5".to_string(),
    };

    assert_eq!(package.name, "my-workflow");
    assert_eq!(package.version, "2.1.3");
    assert!(!package.description.is_empty());
    assert!(package.cloacina_version.starts_with("0.2.0"));
}

#[test]
fn test_compile_result() {
    use std::path::PathBuf;

    let manifest = PackageManifest {
        package: PackageInfo {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            cloacina_version: "0.2.0".to_string(),
        },
        library: LibraryInfo {
            filename: "libtest.so".to_string(),
            symbols: vec!["cloacina_execute_task".to_string()],
            architecture: "x86_64-unknown-linux-gnu".to_string(),
        },
        tasks: vec![],
        execution_order: vec![],
        graph: None,
    };

    let compile_result = CompileResult {
        so_path: PathBuf::from("/tmp/libtest.so"),
        manifest: manifest.clone(),
    };

    assert_eq!(compile_result.so_path, PathBuf::from("/tmp/libtest.so"));
    assert_eq!(compile_result.manifest.package.name, manifest.package.name);
}

#[test]
fn test_manifest_constants() {
    assert_eq!(MANIFEST_FILENAME, "manifest.json");
    assert_eq!(EXECUTE_TASK_SYMBOL, "cloacina_execute_task");
}

#[test]
fn test_manifest_with_empty_tasks() {
    let manifest = PackageManifest {
        package: PackageInfo {
            name: "empty-package".to_string(),
            version: "1.0.0".to_string(),
            description: "Package with no tasks".to_string(),
            cloacina_version: "0.2.0".to_string(),
        },
        library: LibraryInfo {
            filename: "libempty.so".to_string(),
            symbols: vec!["cloacina_execute_task".to_string()],
            architecture: "x86_64-unknown-linux-gnu".to_string(),
        },
        tasks: vec![],
        execution_order: vec![],
        graph: None,
    };

    let json = serde_json::to_string(&manifest).expect("Failed to serialize empty manifest");
    let deserialized: PackageManifest =
        serde_json::from_str(&json).expect("Failed to deserialize empty manifest");

    assert!(deserialized.tasks.is_empty());
    assert!(deserialized.execution_order.is_empty());
    assert!(deserialized.graph.is_none());
}

#[test]
fn test_manifest_json_structure() {
    let manifest = PackageManifest {
        package: PackageInfo {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            cloacina_version: "0.2.0".to_string(),
        },
        library: LibraryInfo {
            filename: "libtest.so".to_string(),
            symbols: vec!["cloacina_execute_task".to_string()],
            architecture: "x86_64-unknown-linux-gnu".to_string(),
        },
        tasks: vec![],
        execution_order: vec![],
        graph: None,
    };

    let json_value: serde_json::Value =
        serde_json::to_value(&manifest).expect("Failed to convert to JSON value");

    // Verify JSON structure
    assert!(json_value.is_object());
    assert!(json_value["package"].is_object());
    assert!(json_value["library"].is_object());
    assert!(json_value["tasks"].is_array());
    assert!(json_value["execution_order"].is_array());

    // Verify package fields
    assert_eq!(json_value["package"]["name"], "test");
    assert_eq!(json_value["package"]["version"], "1.0.0");

    // Verify library fields
    assert_eq!(json_value["library"]["filename"], "libtest.so");
    assert!(json_value["library"]["symbols"].is_array());
}
