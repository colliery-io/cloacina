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

use cloacina_workflow_plugin::{
    InputInterfaceDescriptor, PackageTasksMetadata, ReactorPackageMetadata,
};

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

    // CLOACI-T-0896 (Rust-cdylib side): `alpha` is declared as a
    // `#[state_accumulator(capacity = 5)]` — the FFI metadata must now report its
    // REAL kind + config from the `AccumulatorEntry` inventory, not the old
    // hardcoded `passthrough`. `beta` has no decorated fn, so it falls back to
    // passthrough — proving both the lookup and the fallback.
    let alpha = r
        .accumulators
        .iter()
        .find(|a| a.name == "alpha")
        .expect("alpha accumulator present");
    assert_eq!(
        alpha.accumulator_type, "state",
        "alpha is #[state_accumulator] — expected kind 'state', got {:?}",
        alpha.accumulator_type
    );
    assert_eq!(
        alpha.config.get("capacity").map(String::as_str),
        Some("5"),
        "alpha should carry its authored capacity in config; got {:?}",
        alpha.config
    );
    let beta = r
        .accumulators
        .iter()
        .find(|a| a.name == "beta")
        .expect("beta accumulator present");
    assert_eq!(
        beta.accumulator_type, "passthrough",
        "beta has no decorated accumulator fn — should fall back to passthrough"
    );
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

// ===========================================================================
// CLOACI-I-0128 (T-0758) — accumulator/reactor input-interface derivation.
// ===========================================================================

/// The mixed fixture's reactor surface exposes a TYPED input slot for its
/// accumulator source `alpha`, because mixed-rust opts in by deriving
/// `schemars::JsonSchema` on the boundary type `AlphaIn` (`compute(alpha:
/// Option<&AlphaIn>)`). Verifies the opt-in typed path end-to-end through the
/// `get_input_interface` FFI entrypoint.
#[test]
fn mixed_fixture_exposes_typed_reactor_input_interface() {
    let Some(handle) = load_handle("mixed-rust") else {
        eprintln!("Skipping: mixed-rust not built");
        return;
    };

    // Method index 9 = get_input_interface (CLOACI-I-0128).
    let desc: InputInterfaceDescriptor = handle
        .call_method(cloacina_workflow_plugin::METHOD_GET_INPUT_INTERFACE, &())
        .expect("get_input_interface FFI call should succeed");

    let reactor = desc
        .entries
        .iter()
        .find(|e| e.surface_kind == "reactor" && e.surface_name == "mixed_reactor")
        .expect("expected a reactor entry for 'mixed_reactor'");

    let slots: Vec<serde_json::Value> =
        serde_json::from_str(&reactor.slots_json).expect("reactor slots_json is a JSON array");
    let alpha = slots
        .iter()
        .find(|s| s["name"] == "alpha")
        .expect("reactor exposes an 'alpha' source slot");

    // AlphaIn derives JsonSchema → a rich object schema (NOT the permissive {}).
    let props = alpha["schema"].get("properties");
    assert!(
        props.is_some(),
        "alpha should carry a typed object schema (AlphaIn derives JsonSchema), got: {}",
        alpha["schema"]
    );
    assert!(
        props.unwrap().get("value").is_some(),
        "AlphaIn schema should describe its `value` field, got: {}",
        alpha["schema"]
    );
}

// ===========================================================================
// T-0553 — trigger-only and mixed fixtures (T-D completion).
// ===========================================================================

#[test]
fn trigger_only_fixture_emits_cron_and_custom_metadata() {
    let Some(handle) = load_handle("trigger-only-rust") else {
        eprintln!("Skipping: trigger-only-rust not built");
        return;
    };

    // Method index 5 = get_trigger_metadata
    let triggers: Vec<cloacina_workflow_plugin::TriggerPackageMetadata> = handle
        .call_method(5, &())
        .expect("get_trigger_metadata FFI call should succeed");

    assert_eq!(
        triggers.len(),
        2,
        "trigger-only fixture should emit two triggers (one cron + one custom); got {:?}",
        triggers
            .iter()
            .map(|t| (&t.name, &t.cron_expression))
            .collect::<Vec<_>>()
    );

    let cron = triggers
        .iter()
        .find(|t| t.cron_expression.is_some())
        .expect("expected a cron-shaped trigger entry");
    assert_eq!(cron.name, "trigger_only_cron");
    assert_eq!(
        cron.cron_expression.as_deref(),
        Some("*/10 * * * * *"),
        "cron trigger should expose its expression via cron_expression()"
    );
    assert_eq!(cron.package_name, "trigger-only-rust");

    let custom = triggers
        .iter()
        .find(|t| t.cron_expression.is_none())
        .expect("expected a custom-poll trigger entry");
    assert_eq!(custom.name, "trigger_only_custom");
}

#[test]
fn trigger_only_fixture_emits_no_reactors_or_graph() {
    let Some(handle) = load_handle("trigger-only-rust") else {
        eprintln!("Skipping: trigger-only-rust not built");
        return;
    };

    let reactors: Vec<cloacina_workflow_plugin::ReactorPackageMetadata> = handle
        .call_method(4, &())
        .expect("get_reactor_metadata should succeed");
    assert!(reactors.is_empty(), "trigger-only fixture has no reactors");

    // get_graph_metadata returns NotSupported (Plugin error) when no graph.
    let graph_result: Result<cloacina_workflow_plugin::GraphPackageMetadata, _> =
        handle.call_method(2, &());
    assert!(
        graph_result.is_err(),
        "trigger-only fixture has no computation graph; get_graph_metadata should error"
    );
}

#[test]
fn mixed_fixture_exposes_all_primitives() {
    let Some(handle) = load_handle("mixed-rust") else {
        eprintln!("Skipping: mixed-rust not built");
        return;
    };

    // Reactor present.
    let reactors: Vec<cloacina_workflow_plugin::ReactorPackageMetadata> = handle
        .call_method(4, &())
        .expect("get_reactor_metadata should succeed");
    assert_eq!(reactors.len(), 1);
    assert_eq!(reactors[0].name, "mixed_reactor");

    // Trigger present.
    let triggers: Vec<cloacina_workflow_plugin::TriggerPackageMetadata> = handle
        .call_method(5, &())
        .expect("get_trigger_metadata should succeed");
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0].name, "mixed_trigger");
    assert!(
        triggers[0].cron_expression.is_none(),
        "mixed fixture's trigger is custom-poll, not cron"
    );

    // Graph present, reactor-bound.
    let graph: cloacina_workflow_plugin::GraphPackageMetadata = handle
        .call_method(2, &())
        .expect("get_graph_metadata should succeed");
    assert_eq!(graph.graph_name, "mixed_graph");
    assert_eq!(graph.trigger_reactor.as_deref(), Some("mixed_reactor"));

    // Workflow present + subscribes to the trigger.
    let task_meta: cloacina_workflow_plugin::PackageTasksMetadata = handle
        .call_method(0, &())
        .expect("get_task_metadata should succeed");
    assert_eq!(task_meta.workflow_name, "mixed_workflow");
    assert_eq!(
        task_meta.triggers,
        vec!["mixed_trigger".to_string()],
        "mixed_workflow declares triggers = [\"mixed_trigger\"]"
    );
    assert_eq!(task_meta.tasks.len(), 1);
    assert_eq!(task_meta.tasks[0].id, "mixed_step");
}
