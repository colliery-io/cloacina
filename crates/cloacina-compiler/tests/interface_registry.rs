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

//! Packages across a plugin interface change, built with crates from a
//! registry (CLOACINA-T-1085).
//!
//! Two packages of the `cloacinactl package new` shape, each in service with
//! a library built for the plugin interface of the old minor:
//!
//! - one pins the old minor (`"0.12"`, with a `Cargo.lock`), so its rebuild
//!   from kept source resolves the old minor's crates from crates.io again:
//!   it is listed as needing a new publish, with both interface versions;
//! - one pins the release under test and reaches its crates through a
//!   `[patch.crates-io]` path override in its own `Cargo.toml`: its rebuild
//!   has this release's interface and it runs with no publish.
//!
//! The compiler runs as in production: no `--dev-workspace`. The test needs
//! crates.io and builds the old minor's crates, so it is ignored by default
//! and runs in the nightly lane:
//!
//! ```text
//! cargo test -p cloacina-compiler --features sqlite --test interface_registry -- --ignored
//! ```
//!
//! `CLOACINA_OLD_MINOR` names the old minor (default `0.12`, plugin
//! interface 7).

#![cfg(all(unix, feature = "sqlite"))]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::fleet::PLUGIN_INTERFACE_VERSION;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina_compiler::{BuildRlimits, CompilerConfig, interface_pass};

type Registry = WorkflowRegistryImpl<UnifiedRegistryStorage>;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn old_minor() -> String {
    std::env::var("CLOACINA_OLD_MINOR").unwrap_or_else(|_| "0.12".to_string())
}

/// The cargo target folder of the packages these tests build.
fn target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("t1085-registry-target")
}

fn library_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

/// The `[patch.crates-io]` table that sends every crate of this workspace to
/// its path: the build under test, as a user's build would reach it once it
/// is published.
fn patch_table() -> String {
    let crates = workspace_root().join("crates");
    let mut lines = vec!["[patch.crates-io]".to_string()];
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&crates)
        .expect("crates folder")
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    for dir in entries {
        let Ok(raw) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
            continue;
        };
        let Ok(doc) = raw.parse::<toml::Table>() else {
            continue;
        };
        if let Some(name) = doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        {
            lines.push(format!(
                "{name} = {{ path = {:?} }}",
                dir.display().to_string()
            ));
        }
    }
    lines.join("\n")
}

/// Write a package of the `cloacinactl package new` shape into `dir`, with
/// its `cloacina-*` crates pinned to `pin`, and `extra` at the end of its
/// `Cargo.toml`.
fn write_package(dir: &Path, name: &str, pin: &str, extra: &str) {
    let module = name.replace('-', "_");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = ["packaged"]
packaged = []

[dependencies]
cloacina-workflow = {{ version = "{pin}", features = ["packaged", "macros"] }}
cloacina-workflow-plugin = "{pin}"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"

[workspace]

{extra}
"#
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("package.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
interface = "cloacina-workflow-plugin"
interface_version = 1
extension = "cloacina"

[metadata]
language = "rust"
workflow_name = "{module}"
description = "{name} workflow"
"#
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("src/lib.rs"),
        format!(
            r#"use cloacina_workflow::{{task, workflow}};
use cloacina_workflow::{{Context, TaskError}};

cloacina_workflow_plugin::package!();

#[workflow(name = "{module}", description = "{name} workflow")]
pub mod {module}_wf {{
    use super::*;

    #[task(id = "hello", dependencies = [])]
    pub async fn hello(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {{
        context.insert("hello", serde_json::json!("world"))?;
        Ok(())
    }}
}}
"#
        ),
    )
    .unwrap();
}

fn cargo(dir: &Path, args: &[&str]) {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target_dir())
        .status()
        .expect("spawn cargo");
    assert!(
        status.success(),
        "cargo {args:?} in {} failed",
        dir.display()
    );
}

/// Build the package in `dir` with cargo, as a compiler of the old minor
/// did, and return its library.
fn build_library(dir: &Path, name: &str) -> Vec<u8> {
    cargo(dir, &["build", "--lib"]);
    let path = target_dir().join("debug").join(format!(
        "lib{}.{}",
        name.replace('-', "_"),
        library_extension()
    ));
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn pack_dir(dir: &Path) -> Vec<u8> {
    let out = tempfile::TempDir::new().expect("tempdir");
    let archive = out.path().join("package.cloacina");
    fidius_core::package::pack_package(dir, Some(&archive)).expect("pack the package");
    std::fs::read(&archive).expect("read the archive")
}

async fn sqlite_registry() -> (Registry, cloacina::dal::DAL) {
    let url = format!(
        "file:t1085_registry_{}?mode=memory&cache=shared",
        uuid::Uuid::new_v4()
    );
    let db = cloacina::database::Database::new(&url, "", 5);
    db.run_migrations().await.expect("migrations");
    let storage = UnifiedRegistryStorage::new(db.clone());
    let dal = cloacina::dal::DAL::new(db.clone());
    (
        WorkflowRegistryImpl::new(storage, db).expect("registry"),
        dal,
    )
}

fn config(home: &Path) -> CompilerConfig {
    CompilerConfig {
        home: home.to_path_buf(),
        bind: "127.0.0.1:0".parse().unwrap(),
        metrics_bind: None,
        database_url: "unused-in-this-test".to_string(),
        tenant_schema: None,
        build_target: None,
        build_target_package: None,
        verbose: false,
        poll_interval: Duration::from_secs(1),
        heartbeat_interval: Duration::from_secs(5),
        stale_threshold: Duration::from_secs(30),
        sweep_interval: Duration::from_secs(15),
        cargo_flags: vec!["build".to_string(), "--lib".to_string()],
        tmp_root: Some(home.join("build-tmp")),
        cargo_target_dir: Some(target_dir()),
        build_timeout: Duration::from_secs(3600),
        vendor_dir: None,
        // No `--dev-workspace`: crates resolve as a user's build resolves them.
        dev_workspace: None,
        build_rlimits: BuildRlimits {
            cpu_s: 3600,
            mem_bytes: 16 * 1024 * 1024 * 1024,
            files: 4096,
            procs: 4096,
        },
        compiler_instance_id: cloacina::UniversalUuid::new_v4(),
        log_retention_days: 7,
        python: Default::default(),
        describe_timeout: Duration::from_secs(120),
        describe_program: Some(PathBuf::from(env!("CARGO_BIN_EXE_cloacina-compiler"))),
    }
}

/// Put a package in service with `library` as its build, the way a compiler
/// of the old minor recorded it: no interface on the row.
async fn in_service(registry: &mut Registry, archive: Vec<u8>, library: Vec<u8>) -> uuid::Uuid {
    let id = registry
        .register_workflow_package(archive)
        .await
        .expect("register the package");
    let claim = registry
        .claim_next_build()
        .await
        .expect("claim")
        .expect("a pending build");
    assert_eq!(claim.id, id);
    registry
        .mark_build_success(id, library)
        .await
        .expect("record the build");
    id
}

#[tokio::test]
#[ignore = "needs crates.io; the nightly lane runs it"]
async fn a_package_pinned_to_the_old_minor_is_listed_and_one_pinned_to_this_release_runs() {
    let home = tempfile::TempDir::new().unwrap();
    let stage = tempfile::TempDir::new().unwrap();
    let old = old_minor();
    let this_release = env!("CARGO_PKG_VERSION");

    // Pinned to the old minor, with a lock file: crates.io only.
    let old_pin = stage.path().join("old-pin-rust");
    write_package(&old_pin, "old-pin-rust", &old, "");
    cargo(&old_pin, &["generate-lockfile"]);
    let old_library = build_library(&old_pin, "old-pin-rust");

    // Pinned to this release, reached through the path override.
    let new_pin = stage.path().join("new-pin-rust");
    write_package(&new_pin, "new-pin-rust", this_release, &patch_table());

    let (mut registry, dal) = sqlite_registry().await;
    let old_id = in_service(&mut registry, pack_dir(&old_pin), old_library.clone()).await;
    // Its stored build is a library of the old interface too.
    let new_id = in_service(&mut registry, pack_dir(&new_pin), old_library).await;

    let config = config(home.path());
    assert_eq!(interface_pass(&registry, &config).await, 2);

    // The old pin: listed, with both versions and the step; in service.
    let listed = registry.list_packages_needing_new_build().await.unwrap();
    assert_eq!(listed.len(), 1, "{listed:?}");
    let entry = &listed[0];
    eprintln!("listed: {entry:?}");
    assert_eq!(entry.id, old_id);
    let built_for = entry.built_for.expect("the old interface is named");
    assert_ne!(built_for, PLUGIN_INTERFACE_VERSION);
    assert!(
        entry
            .reason
            .contains(&format!("built for plugin interface {built_for}")),
        "{}",
        entry.reason
    );
    assert!(
        entry.reason.contains(&format!(
            "loads plugin interface {PLUGIN_INTERFACE_VERSION}"
        )),
        "{}",
        entry.reason
    );
    assert!(
        entry.reason.contains("publish the package again"),
        "{}",
        entry.reason
    );
    let dispatch = dal
        .workflow_packages()
        .get_active_dispatch("old-pin-rust", None)
        .await
        .unwrap()
        .expect("in service");
    assert_eq!(dispatch.plugin_interface_version, Some(built_for));
    assert!(dispatch.needs_new_build.is_some());

    // The pin to this release: rebuilt with this interface, not listed.
    let status = registry.interface_status(new_id).await.unwrap().unwrap();
    assert_eq!(
        status.plugin_interface_version,
        Some(PLUGIN_INTERFACE_VERSION)
    );
    assert_eq!(status.needs_new_build, None);

    // Neither is rebuilt again.
    assert_eq!(interface_pass(&registry, &config).await, 0);
}
