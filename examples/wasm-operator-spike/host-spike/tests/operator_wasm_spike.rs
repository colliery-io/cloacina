// CLOACI-T-0821 WASM-operator spike — host load + invoke.
//
// Proves a fidius-macro-authored SYNCHRONOUS operator, compiled to a
// wasm32-wasip2 component, is loaded *configured* (`load_wasm_configured`)
// and invoked across the sandbox boundary, and that N differently-configured
// instances coexist.
//
// The host re-declares the SAME interface the fixture implements (with
// `crate = "fidius_core"`) so the macro emits a matching
// `MinimalOperator_WASM_DESCRIPTOR` (companion module `__fidius_MinimalOperator`).
// The descriptor's `interface_export` is what the host links against; the
// package.toml `interface` field is metadata only.

#![allow(unexpected_cfgs)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use fidius_host::PluginHost;
use serde::Serialize;

#[derive(Serialize)]
struct Config {
    op: String,
}

// SAME interface as the fixture → matching macro-generated descriptor + hash.
#[fidius_macro::plugin_interface(version = 1, buffer = PluginAllocated, crate = "fidius_core")]
pub trait MinimalOperator: Send + Sync {
    fn apply(&self, input: String) -> String;
}

/// Build the operator fixture to a wasm component once, return its bytes.
fn component() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../operator-fixture");
        let status = Command::new("cargo")
            .args(["build", "--target", "wasm32-wasip2", "--release"])
            .current_dir(&fixture)
            .status()
            .expect("cargo build --target wasm32-wasip2");
        assert!(status.success(), "operator-fixture wasm build failed");
        std::fs::read(
            fixture.join("target/wasm32-wasip2/release/wasm_operator_fixture.wasm"),
        )
        .unwrap()
    })
}

/// Stage the component + a package.toml where `find_wasm_package` resolves it.
fn stage(root: &std::path::Path) {
    let dir = root.join("wasm-operator-pkg");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("wasm_operator_fixture.wasm"), component()).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        "[package]\nname = \"wasm-operator-pkg\"\nversion = \"0.1.0\"\n\
         interface = \"minimal-operator\"\ninterface_version = 1\nruntime = \"wasm\"\n\n\
         [metadata]\ncategory = \"test\"\n\n\
         [wasm]\ncomponent = \"wasm_operator_fixture.wasm\"\n",
    )
    .unwrap();
}

#[test]
fn configured_operator_loads_and_invokes() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());
    let host = PluginHost::builder()
        .search_path(tmp.path())
        .build()
        .unwrap();

    let handle = host
        .load_wasm_configured(
            "wasm-operator-pkg",
            &__fidius_MinimalOperator::MinimalOperator_WASM_DESCRIPTOR,
            &Config { op: "prefix".into() },
        )
        .expect("load_wasm_configured");

    // apply is method 0; single arg → tuple (T,); config already bound.
    let out: String = handle.call_method(0, &("test".to_string(),)).unwrap();
    assert_eq!(out, "prefix: test");

    // Config persists across calls on the same configured instance.
    let out2: String = handle.call_method(0, &("again".to_string(),)).unwrap();
    assert_eq!(out2, "prefix: again");
}

#[test]
fn n_differently_configured_instances_coexist() {
    let tmp = tempfile::TempDir::new().unwrap();
    stage(tmp.path());
    let host = PluginHost::builder()
        .search_path(tmp.path())
        .build()
        .unwrap();

    let a = host
        .load_wasm_configured(
            "wasm-operator-pkg",
            &__fidius_MinimalOperator::MinimalOperator_WASM_DESCRIPTOR,
            &Config { op: "prefix".into() },
        )
        .unwrap();
    let b = host
        .load_wasm_configured(
            "wasm-operator-pkg",
            &__fidius_MinimalOperator::MinimalOperator_WASM_DESCRIPTOR,
            &Config { op: "suffix".into() },
        )
        .unwrap();

    let oa: String = a.call_method(0, &("x".to_string(),)).unwrap();
    let ob: String = b.call_method(0, &("y".to_string(),)).unwrap();
    assert_eq!(oa, "prefix: x");
    assert_eq!(ob, "suffix: y");
}
