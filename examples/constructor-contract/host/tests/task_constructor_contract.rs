// CLOACI-T-0822 constructor contract — host load + invoke + manifest round-trip.
//
// Two proofs:
//   1. `manifest_round_trips_and_carries_primitive_kind` — the cloacina constructor
//      MANIFEST the macros emit serializes into the package and reads back
//      identically, exposing the `primitive_kind` the loader (T-0823) switches
//      on to pick the descriptor.
//   2. `configured_task_constructor_loads_and_invokes` — a fidius-macro-authored
//      SYNC `TaskConstructor` (wasm32-wasip2 component) is loaded *configured*
//      (`load_wasm_configured`) and invoked with a serialized task context; the
//      constructor reads `name`, writes `result`, and hands the context back.
//
// The host re-declares the SAME `TaskConstructor` interface the fixture implements
// (with `crate = "fidius_core"`) so the macro emits a matching
// `TaskConstructor_WASM_DESCRIPTOR` (companion module `__fidius_TaskConstructor`).

#![allow(unexpected_cfgs)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use fidius_host::PluginHost;
use constructor_contract::{InputSlot, ConstructorManifest, PrimitiveKind, TaskInvocation, TaskOutcome};
use serde::Serialize;

/// Per-instance config the host binds once (mirrors the fixture's `Config`).
#[derive(Serialize)]
struct Config {
    prefix: String,
}

// SAME interface as the fixture → matching macro-generated descriptor + hash.
// `crate = "fidius_core"` is the host-side variant of the guest's
// `crate = "fidius_guest"` declaration.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait TaskConstructor: Send + Sync {
    fn execute(&self, invocation_json: String) -> String;
}

/// The manifest the `#[task(constructor)]` macro WOULD emit for the fixture. In the
/// real integration this is generated at the existing metadata-emission seam
/// (alongside `PackageTasksMetadata`) and written into the package; here we
/// construct it by hand to prove the schema + round-trip.
fn fixture_manifest() -> ConstructorManifest {
    ConstructorManifest {
        name: "greet".into(),
        version: "0.1.0".into(),
        primitive_kind: PrimitiveKind::Task,
        interface: "task-constructor".into(),
        interface_version: 1,
        params: vec![InputSlot::required(
            "name",
            serde_json::json!({"type": "string"}),
        )],
        dependencies: vec![],
        description: Some("Prefixes the context `name` into `result`.".into()),
        author: Some("CLOACI-T-0822".into()),
    }
}

/// Build the constructor fixture to a wasm component once, return its bytes.
fn component() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let fixture =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../task-constructor-fixture");
        let status = Command::new("cargo")
            .args(["build", "--target", "wasm32-wasip2", "--release"])
            .current_dir(&fixture)
            .status()
            .expect("cargo build --target wasm32-wasip2");
        assert!(status.success(), "task-constructor-fixture wasm build failed");
        std::fs::read(
            fixture.join("target/wasm32-wasip2/release/task_constructor_fixture.wasm"),
        )
        .unwrap()
    })
}

/// Stage the component + package.toml + the constructor manifest sidecar.
fn stage(root: &Path) {
    let dir = root.join("task-constructor-pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("task_constructor_fixture.wasm"), component()).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nname = \"task-constructor-pkg\"\nversion = \"0.1.0\"\n\
         interface = \"task-constructor\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
         [metadata]\ncategory = \"constructor\"\n\n\
         [wasm]\ncomponent = \"task_constructor_fixture.wasm\"\n",
    )
    .unwrap();
    // The cloacina constructor manifest travels as a sidecar (what the loader reads).
    std::fs::write(
        dir.join("constructor.json"),
        fixture_manifest().to_json().unwrap(),
    )
    .unwrap();
}

#[test]
fn manifest_round_trips_and_carries_primitive_kind() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    // Read the manifest back the way the loader (T-0823) would.
    let raw = std::fs::read_to_string(
        tmp.path().join("task-constructor-pkg").join("constructor.json"),
    )
    .unwrap();
    let read = ConstructorManifest::from_json(&raw).unwrap();

    assert_eq!(read, fixture_manifest(), "manifest must round-trip exactly");
    assert_eq!(
        read.primitive_kind,
        PrimitiveKind::Task,
        "loader switches on primitive_kind to pick the descriptor"
    );
    assert_eq!(read.interface, "task-constructor");
    assert_eq!(read.interface_version, 1);
    assert_eq!(read.params.len(), 1);
    assert_eq!(read.params[0].name, "name");
    assert!(read.params[0].required);
}

#[test]
fn configured_task_constructor_loads_and_invokes() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());

    // Loader flow: read manifest → confirm it's a Task → load via TaskConstructor.
    let raw = std::fs::read_to_string(
        tmp.path().join("task-constructor-pkg").join("constructor.json"),
    )
    .unwrap();
    let manifest = ConstructorManifest::from_json(&raw).unwrap();
    assert_eq!(manifest.primitive_kind, PrimitiveKind::Task);

    let host = PluginHost::builder()
        .search_path(tmp.path())
        .build()
        .unwrap();

    let handle = host
        .load_wasm_configured(
            "task-constructor-pkg",
            &__fidius_TaskConstructor::TaskConstructor_WASM_DESCRIPTOR,
            &Config {
                prefix: "hello, ".into(),
            },
        )
        .expect("load_wasm_configured");

    // Host async wrapper would do this serialize/spawn_blocking call:
    let invocation = TaskInvocation {
        context_json: serde_json::json!({ "name": "world" }).to_string(),
    };
    let inv_json = serde_json::to_string(&invocation).unwrap();

    // execute is method 0; single arg → tuple (T,); config already bound.
    let out_json: String = handle.call_method(0, &(inv_json,)).unwrap();
    let outcome: TaskOutcome = serde_json::from_str(&out_json).unwrap();

    assert!(outcome.success, "outcome: {outcome:?}");
    let ctx: serde_json::Value =
        serde_json::from_str(&outcome.context_json.unwrap()).unwrap();
    assert_eq!(ctx["result"], "hello, world");
    // Original context key is preserved.
    assert_eq!(ctx["name"], "world");
}

#[test]
fn missing_required_input_surfaces_as_failed_outcome() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());
    let host = PluginHost::builder()
        .search_path(tmp.path())
        .build()
        .unwrap();

    let handle = host
        .load_wasm_configured(
            "task-constructor-pkg",
            &__fidius_TaskConstructor::TaskConstructor_WASM_DESCRIPTOR,
            &Config {
                prefix: "x".into(),
            },
        )
        .unwrap();

    // Context without the required `name` → a clean failed outcome, not a trap.
    let invocation = TaskInvocation {
        context_json: serde_json::json!({ "other": 1 }).to_string(),
    };
    let inv_json = serde_json::to_string(&invocation).unwrap();
    let out_json: String = handle.call_method(0, &(inv_json,)).unwrap();
    let outcome: TaskOutcome = serde_json::from_str(&out_json).unwrap();

    assert!(!outcome.success);
    assert!(outcome.error.unwrap().contains("name"));
}
