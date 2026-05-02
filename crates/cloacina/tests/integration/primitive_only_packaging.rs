/*
 *  Copyright 2026 Colliery Software
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

//! T-0550 / I-0102 T-D — primitive-only packaging integration tests.
//!
//! Exercise the unified `cloacina::package!()` shell macro through real
//! cdylib loads via `fidius_host::loader::load_library`. The point isn't
//! to drive the full reconciler (that's covered by `packaging.rs` and
//! `fidius_validation.rs`) but to lock down the wire-format invariants
//! that T-A's shell + T-B's reactor-metadata extraction depend on.

use cloacina_workflow_plugin::{PackageTasksMetadata, ReactorPackageMetadata};

/// Find the pre-built debug dylib for a fixture under `examples/fixtures/`.
fn find_fixture_dylib(name: &str) -> Option<std::path::PathBuf> {
    let cargo_manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::PathBuf::from(&cargo_manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let project_path = workspace_root.join(format!("examples/fixtures/{}", name));
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    // Cdylib output uses the package name with `-` replaced by `_`.
    let lib_basename = name.replace('-', "_");
    for profile in &["debug", "release"] {
        let path = project_path
            .join("target")
            .join(profile)
            .join(format!("lib{}.{}", lib_basename, ext));
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn load_handle(name: &str) -> Option<fidius_host::PluginHandle> {
    let path = find_fixture_dylib(name)?;
    let loaded = fidius_host::loader::load_library(&path)
        .unwrap_or_else(|e| panic!("Failed to load {} dylib: {}", name, e));
    let plugin = loaded.plugins.into_iter().next()?;
    Some(fidius_host::PluginHandle::from_loaded(plugin))
}

#[test]
fn reactor_only_fixture_emits_reactor_metadata() {
    let Some(handle) = load_handle("reactor-only-rust") else {
        eprintln!("Skipping: reactor-only-rust not built");
        return;
    };

    // Method index 4 = get_reactor_metadata
    let reactors: Vec<ReactorPackageMetadata> = handle
        .call_method(4, &())
        .expect("get_reactor_metadata FFI call should succeed");
    assert_eq!(
        reactors.len(),
        1,
        "reactor-only fixture should declare exactly one reactor; got {:?}",
        reactors
    );
    let r = &reactors[0];
    assert_eq!(r.name, "shared_rx");
    assert_eq!(r.package_name, "reactor-only-rust");
    let acc_names: Vec<&str> = r.accumulators.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(acc_names, vec!["alpha", "beta"]);
    assert_eq!(r.reaction_mode, "when_any");
}

#[test]
fn reactor_only_fixture_emits_no_tasks() {
    let Some(handle) = load_handle("reactor-only-rust") else {
        eprintln!("Skipping: reactor-only-rust not built");
        return;
    };

    // Method index 0 = get_task_metadata
    let metadata: PackageTasksMetadata = handle
        .call_method(0, &())
        .expect("get_task_metadata FFI call should succeed");
    assert!(
        metadata.tasks.is_empty(),
        "reactor-only fixture should declare no tasks; got {} task(s)",
        metadata.tasks.len()
    );
    assert_eq!(metadata.package_name, "reactor-only-rust");
}

#[test]
fn reactor_subscriber_fixture_carries_string_name_binding() {
    let Some(handle) = load_handle("reactor-subscriber-rust") else {
        eprintln!("Skipping: reactor-subscriber-rust not built");
        return;
    };

    // Method index 2 = get_graph_metadata
    let graph: cloacina_workflow_plugin::GraphPackageMetadata = handle
        .call_method(2, &())
        .expect("get_graph_metadata FFI call should succeed");
    assert_eq!(graph.graph_name, "subscriber_graph");
    assert_eq!(
        graph.trigger_reactor.as_deref(),
        Some("shared_rx"),
        "subscriber's trigger_reactor must be the string-named reactor (cross-package binding)"
    );
}
