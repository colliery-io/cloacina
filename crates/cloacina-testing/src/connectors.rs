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

//! Build a Sync connector guest crate to `wasm32-wasip2` and stage it as a
//! provider package a test can load with the connector host
//! (COLLIERY-I-0632 / CLOACINA-T-0979). The build runs at most once per
//! fixture per test process; `cargo` itself keeps it incremental across
//! processes. Moved from weir's WASM test kit.
//!
//! A `CARGO_TARGET_DIR` in the environment would move the guest's artifact
//! away from the path this reads; the build is run with it unset.

pub mod conformance;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use cloacina_constructor_contract::sync::{ConnectorRole, SyncMode};
use cloacina_constructor_contract::{
    CONNECTOR_INTERFACE, CONNECTOR_INTERFACE_VERSION, ConfigField, ConnectorMeta,
    ConstructorManifest, PrimitiveKind, ProviderManifest, ProviderRuntime,
};

static BUILT: OnceLock<Mutex<HashMap<PathBuf, Vec<u8>>>> = OnceLock::new();

/// Build the guest crate at `crate_dir` to a release `wasm32-wasip2` component
/// and return its bytes. Panics with the build output on failure.
pub fn build_component(crate_dir: &Path) -> Vec<u8> {
    let crate_dir = crate_dir
        .canonicalize()
        .unwrap_or_else(|e| panic!("connector crate {}: {e}", crate_dir.display()));
    let cache = BUILT.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(bytes) = cache.lock().unwrap().get(&crate_dir) {
        return bytes.clone();
    }
    let output = Command::new("cargo")
        .args(["build", "--lib", "--target", "wasm32-wasip2", "--release"])
        .env_remove("CARGO_TARGET_DIR")
        .current_dir(&crate_dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn cargo build in {}: {e}", crate_dir.display()));
    assert!(
        output.status.success(),
        "wasm build of {} failed:\n{}",
        crate_dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    let name = crate_name(&crate_dir).replace('-', "_");
    let path = crate_dir.join(format!("target/wasm32-wasip2/release/{name}.wasm"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    cache.lock().unwrap().insert(crate_dir, bytes.clone());
    bytes
}

/// The `[package] name` of the crate at `dir`.
pub fn crate_name(dir: &Path) -> String {
    let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).expect("Cargo.toml");
    manifest
        .lines()
        .map(str::trim)
        .find_map(|l| {
            l.strip_prefix("name = ")
                .map(|v| v.trim_matches('"').to_string())
        })
        .expect("[package] name")
}

/// One member of a staged connector provider.
pub struct Member<'a> {
    pub name: &'a str,
    pub kind: PrimitiveKind,
    pub roles: Vec<ConnectorRole>,
}

impl<'a> Member<'a> {
    pub fn source(name: &'a str) -> Self {
        Self {
            name,
            kind: PrimitiveKind::Source,
            roles: vec![ConnectorRole::Source],
        }
    }

    pub fn destination(name: &'a str) -> Self {
        Self {
            name,
            kind: PrimitiveKind::Destination,
            roles: vec![ConnectorRole::Destination],
        }
    }
}

/// Stage `bytes` as the connector provider package `pkg` under `root`, with
/// the given members, all over the one component. Returns the package dir.
pub fn stage_package(root: &Path, pkg: &str, bytes: &[u8], members: &[Member<'_>]) -> PathBuf {
    let dir = root.join(pkg);
    std::fs::create_dir_all(&dir).unwrap();
    let wasm = format!("{}.wasm", pkg.replace('-', "_"));
    std::fs::write(dir.join(&wasm), bytes).unwrap();
    std::fs::write(
        dir.join("package.toml"),
        format!(
            "[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\ninterface = \"{CONNECTOR_INTERFACE}\"\n\
             interface_version = {CONNECTOR_INTERFACE_VERSION}\nruntime = \"wasm\"\n\n\
             [metadata]\ncategory = \"constructor\"\n\n[wasm]\ncomponent = \"{wasm}\"\n"
        ),
    )
    .unwrap();
    let provider = ProviderManifest {
        name: pkg.into(),
        version: "0.1.0".into(),
        component: wasm,
        runtime: ProviderRuntime::Wasm,
        constructors: members
            .iter()
            .map(|m| ConstructorManifest {
                name: m.name.into(),
                version: "0.1.0".into(),
                primitive_kind: m.kind,
                interface: CONNECTOR_INTERFACE.into(),
                interface_version: CONNECTOR_INTERFACE_VERSION,
                params: vec![],
                config_fields: vec![ConfigField {
                    name: "json".into(),
                    ty: "String".into(),
                }],
                dependencies: vec![],
                description: None,
                author: None,
                connector: Some(ConnectorMeta {
                    roles: m.roles.clone(),
                    supported_sync_modes: vec![SyncMode::FullRefresh],
                    config_schema: "{\"type\":\"object\"}".into(),
                }),
            })
            .collect(),
    };
    std::fs::write(dir.join("provider.json"), provider.to_json().unwrap()).unwrap();
    dir
}

/// Build the guest crate at `crate_dir` and stage it under `root` as a
/// package named after the crate, with a `source` and a `destination` member.
pub fn stage_connector(root: &Path, crate_dir: &Path) -> PathBuf {
    let pkg = crate_name(crate_dir);
    let bytes = build_component(crate_dir);
    stage_package(
        root,
        &pkg,
        &bytes,
        &[Member::source("source"), Member::destination("destination")],
    )
}
