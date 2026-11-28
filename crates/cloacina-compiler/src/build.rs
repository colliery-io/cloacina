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

//! Build execution: unpack source → cargo (or language-appropriate) build →
//! return the compiled cdylib bytes. Called per-claim from the compiler's
//! main loop.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use cloacina::UniversalUuid;
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::packaging::build_child::{
    ChildPolicy, ChildRlimits, ChildStep, build_child_command, kill_child_group,
};
use cloacina::registry::package_description::PackageDescription;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use cloacina::security::audit;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tracing::{debug, info, warn};

use crate::config::CompilerConfig;
use crate::describe::DescribeInput;

/// Result of a single build attempt.
pub enum BuildOutcome {
    Success {
        /// Compiled cdylib bytes (empty for pure-Python packages).
        artifact: Vec<u8>,
        /// CLOACI-T-0752: per-task "what & why" docs parsed from the package
        /// source, keyed by task local id. Empty when nothing was documented.
        task_docs:
            std::collections::HashMap<String, cloacina::registry::loader::package_loader::TaskDocs>,
        /// CLOACI-T-0760: declared workflow params parsed from the package
        /// source (Python `cloaca.workflow_params(...)`). Empty for Rust (its
        /// params come from the FFI input-interface entrypoint) and for Python
        /// packages that declare none.
        declared_params: Vec<cloacina::input_interface::InputSlot>,
        /// CLOACI-T-0770: declared CG boundary surfaces parsed from the package
        /// source (Python `cloaca.boundary_schema(...)` on accumulators). Empty
        /// for Rust (surfaces come from the FFI input-interface) and for Python
        /// packages with no boundary schemas.
        declared_surfaces: Vec<cloacina_api_types::DeclaredSurface>,
        /// CLOACINA-T-1074: the full metadata document that the describe
        /// child wrote, with the source parse above already in it. `None`
        /// for a per-target build (`--build-target`), which fills a library
        /// for one architecture and does not describe.
        document: Option<Box<PackageDescription>>,
    },
    Failed(String),
    /// The cargo subprocess exceeded `CompilerConfig::build_timeout` and was
    /// killed. The build row's heartbeat stops and the stale-build sweeper
    /// resets it to `pending` on its next tick — do NOT call
    /// `mark_build_failed` for this outcome (the row is reclaimed, not
    /// terminally failed). CLOACI-T-0573 / OPS-10.
    TimedOut {
        elapsed: Duration,
    },
}

/// Internal build-step error. `cargo_build` returns this so `run_build` can
/// distinguish "build failed cleanly" from "build was killed for taking too
/// long" before lowering to `BuildOutcome`. Carries cargo exit info for
/// T-0576's audit `finished` event.
#[derive(Debug)]
enum BuildError {
    Failed {
        reason: String,
        /// `Some(code)` if cargo exited normally; `None` for pre-spawn
        /// errors (target_dir mkdir, spawn failure) and signal-terminated
        /// cargo exits.
        exit_status: Option<i32>,
        /// `Some("SIGKILL")` etc. when cargo was signal-terminated.
        exit_signal: Option<String>,
    },
    TimedOut {
        elapsed: Duration,
    },
}

impl BuildError {
    /// Pre-spawn failure: no cargo subprocess ever ran, so exit fields
    /// are `None`. Used for filesystem / registry / unpack errors.
    fn internal(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
            exit_status: None,
            exit_signal: None,
        }
    }
}

/// Successful cargo invocation. Carries the artifact plus exit info for
/// T-0576's audit `finished` event.
#[derive(Debug)]
struct CargoBuildSuccess {
    artifact: Vec<u8>,
    /// Cargo exits with `Some(0)` on success; we still capture it so the
    /// audit event has a uniform shape.
    exit_status: Option<i32>,
}

/// Execute a build for the given package id.
///
/// Fetches source bytes from the registry, unpacks them, runs the
/// language-appropriate build step, and returns the produced cdylib bytes
/// (empty for pure-Python packages) or an error tail.
pub async fn execute_build(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) -> BuildOutcome {
    match run_build(registry, package_id, config).await {
        Ok((artifact, task_docs, declared_params, declared_surfaces, document)) => {
            BuildOutcome::Success {
                artifact,
                task_docs,
                declared_params,
                declared_surfaces,
                document: document.map(Box::new),
            }
        }
        Err(BuildError::Failed { reason, .. }) => BuildOutcome::Failed(reason),
        Err(BuildError::TimedOut { elapsed }) => BuildOutcome::TimedOut { elapsed },
    }
}

/// SHA-256 of a file's bytes, hex-encoded. `Ok(None)` if the file doesn't
/// exist. Used by T-0576's audit emit to record source-tree provenance.
fn sha256_hex_if_present(path: &Path) -> std::io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(Some(hex::encode(digest)))
}

/// Translate a signal number to a name for the audit `exit_signal` field.
/// Only covers the signals the compiler actually sees (SIGKILL from
/// timeout, SIGSEGV/SIGABRT from rlimit overshoots, SIGTERM from
/// operator shutdown). Returns the number as a string for unrecognized
/// signals.
#[cfg(unix)]
fn signal_name(num: i32) -> String {
    match num {
        1 => "SIGHUP".into(),
        2 => "SIGINT".into(),
        6 => "SIGABRT".into(),
        9 => "SIGKILL".into(),
        11 => "SIGSEGV".into(),
        15 => "SIGTERM".into(),
        24 => "SIGXCPU".into(),
        other => format!("signal({other})"),
    }
}

type TaskDocsMap =
    std::collections::HashMap<String, cloacina::registry::loader::package_loader::TaskDocs>;

/// What a finished build gives: the library, the source parse (docs, params,
/// surfaces) and the metadata document.
type BuildParts = (
    Vec<u8>,
    TaskDocsMap,
    Vec<cloacina::input_interface::InputSlot>,
    Vec<cloacina_api_types::DeclaredSurface>,
    Option<PackageDescription>,
);

/// Describe a built package in a child process (CLOACINA-T-1074).
///
/// A Rust package is described from `library`, the built cdylib. A Python
/// package (`library` is empty) is described from `archive`, the uploaded
/// package, with the dependencies the registry holds for the compiler's own
/// platform and the package's bundled providers.
///
/// Used by the build, and by the pass over packages built before the
/// document existed, or of an older format. `source_dir` is the unpacked
/// package. An `Err` is the reason, as the publisher reads it.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn describe_stored(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_name: &str,
    version: &str,
    library: &[u8],
    archive: &[u8],
    source_dir: &Path,
    work: &Path,
    config: &CompilerConfig,
) -> Result<PackageDescription, String> {
    use cloacina::registry::WorkflowRegistry;

    let providers = registry
        .get_package_providers(package_name, version)
        .await
        .map_err(|e| format!("failed to read the bundled providers: {e}"))?;
    let mut document = if !library.is_empty() {
        crate::describe::describe(DescribeInput::Rust { library }, work, config).await?
    } else {
        let platform = cloacina_workflow_plugin::python_env::host_platform();
        let vendored_set = registry
            .get_python_vendor(package_name, version, &platform)
            .await
            .map_err(|e| format!("failed to read the vendored dependencies for {platform}: {e}"))?;
        crate::describe::describe(
            DescribeInput::Python {
                archive,
                vendored_set: vendored_set.as_deref(),
                providers: &providers,
            },
            work,
            config,
        )
        .await?
    };
    record_in_process(&mut document, &providers, source_dir, work)?;
    Ok(document)
}

/// Write into the document what the server decides a load by (format 2,
/// CLOACINA-T-1078): each bundled provider with its runtime, read from its
/// `provider.json`, and what of the package would run tenant code in the
/// process that loads it. Data only: no provider is opened, and the
/// package's code does not run here.
fn record_in_process(
    document: &mut PackageDescription,
    providers: &[(String, Vec<u8>)],
    source_dir: &Path,
    work: &Path,
) -> Result<(), String> {
    document.providers = provider_runtimes(providers, work)?;
    let manifest = cloacina_workflow_plugin::manifest::load_resolved_manifest(source_dir)
        .map_err(|e| format!("load package.toml: {e}"))?;
    document.in_process = cloacina::registry::in_process::in_process_reasons(
        document,
        &manifest.metadata.accumulators,
    );
    Ok(())
}

/// The bundled providers with their runtimes, from their manifests.
fn provider_runtimes(
    providers: &[(String, Vec<u8>)],
    work: &Path,
) -> Result<Vec<cloacina::registry::in_process::ProviderDescription>, String> {
    if providers.is_empty() {
        return Ok(Vec::new());
    }
    let root = work.join(format!("provider-manifests-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).map_err(|e| format!("create {}: {e}", root.display()))?;
    let read = || -> Result<Vec<(String, String)>, String> {
        for (index, (name, bytes)) in providers.iter().enumerate() {
            let path = root.join(format!("{index}.cloacina"));
            std::fs::write(&path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
            cloacina::registry::loader::unpack_provider_archive(&path, &root, &[])
                .map_err(|e| format!("unpack the bundled provider '{name}': {e}"))?;
        }
        cloacina::registry::loader::constructor_loader::provider_runtimes(&root)
            .map_err(|e| format!("read the bundled providers' manifests: {e}"))
    };
    let result = read();
    let _ = std::fs::remove_dir_all(&root);
    Ok(result?
        .into_iter()
        .map(
            |(name, runtime)| cloacina::registry::in_process::ProviderDescription { name, runtime },
        )
        .collect())
}

/// What the compiler reads from the unpacked package source, without running
/// it: the docs of each task, the declared params and secrets, and the
/// declared surfaces.
fn parse_source(
    source_dir: &Path,
    language: &str,
) -> (
    TaskDocsMap,
    Vec<cloacina::input_interface::InputSlot>,
    Vec<cloacina_api_types::DeclaredSurface>,
) {
    // CLOACI-T-0752: parse "what & why" docs from the unpacked source while we
    // still have it. Best-effort and independent of the cargo build.
    let task_docs = crate::doc_parse::parse_task_docs(source_dir, language);

    // CLOACI-T-0760: parse declared workflow params from the source too. For
    // Python this is the *only* source of declared params (no cdylib FFI); for
    // Rust it's empty (the FFI input-interface entrypoint is authoritative).
    let mut declared_params = crate::param_parse::parse_workflow_params(source_dir, language);

    // CLOACI-T-0859: parse declared workflow SECRETS (Python
    // `cloaca.workflow_secrets(...)`) and carry them as encrypted slots in the
    // same declared-params list, so the manifest records which secrets the
    // workflow requires. Empty for Rust (its secrets ride the FFI interface).
    declared_params.extend(crate::param_parse::parse_workflow_secrets(
        source_dir, language,
    ));

    // CLOACI-T-0770: parse declared CG boundary surfaces (Python
    // `cloaca.boundary_schema(...)`) from the same source. Empty for Rust.
    let declared_surfaces = crate::param_parse::parse_boundary_schemas(source_dir, language);

    (task_docs, declared_params, declared_surfaces)
}

/// Describe a package that is already built and in service
/// (CLOACINA-T-1074): one built before the metadata document existed. No
/// build runs. The stored library (or, for a Python package, the stored
/// archive) is described in a child process, and the source parse is added
/// as a build adds it. An `Err` is the reason the package cannot be
/// described.
pub(crate) async fn describe_existing(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) -> Result<PackageDescription, String> {
    let (meta, source_bytes) = registry
        .get_source_for_build(package_id)
        .await
        .map_err(|e| format!("failed to load the package source: {e}"))?
        .ok_or_else(|| "the package is no longer in the registry".to_string())?;
    let library = registry
        .get_compiled_data_by_id(package_id)
        .await
        .map_err(|e| format!("failed to load the built library: {e}"))?
        .unwrap_or_default();

    let tmp_root = config.tmp_root_or_default();
    std::fs::create_dir_all(&tmp_root)
        .map_err(|e| format!("failed to ensure tmp_root {}: {e}", tmp_root.display()))?;
    let work = TempDir::new_in(&tmp_root)
        .map_err(|e| format!("failed to create a scratch folder: {e}"))?;

    let archive_path = work.path().join("pkg.cloacina");
    std::fs::write(&archive_path, &source_bytes)
        .map_err(|e| format!("failed to stage the archive: {e}"))?;
    let extract_dir = work.path().join("source");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| format!("failed to prepare the extract folder: {e}"))?;
    let source_dir = tokio::task::spawn_blocking(move || {
        cloacina_workflow_plugin::unpack_limit::unpack_package(&archive_path, &extract_dir)
    })
    .await
    .map_err(|e| format!("unpack task panicked: {e}"))?
    .map_err(|e| format!("unpack of the package archive failed: {e}"))?;
    let manifest = load_manifest(&source_dir)?;
    let language = manifest_language(&manifest);
    if language != "python" && library.is_empty() {
        return Err("the package row holds no built library".to_string());
    }

    let mut document = describe_stored(
        registry,
        &meta.package_name,
        &meta.version,
        if language == "python" { &[] } else { &library },
        &source_bytes,
        &source_dir,
        work.path(),
        config,
    )
    .await?;
    let (task_docs, declared_params, declared_surfaces) = parse_source(&source_dir, &language);
    document.overlay_source_parse(&task_docs, &declared_params, &declared_surfaces);
    Ok(document)
}

/// Open a stored Rust library in the describe child, to see whether it has
/// this build's plugin interface (CLOACINA-T-1085). `Ok` when it opens and
/// describes; an `Err` is the child's reason, which names both interface
/// versions when the interface is the cause.
pub(crate) async fn open_stored_library(
    library: &[u8],
    config: &CompilerConfig,
) -> Result<(), String> {
    let tmp_root = config.tmp_root_or_default();
    std::fs::create_dir_all(&tmp_root)
        .map_err(|e| format!("failed to ensure tmp_root {}: {e}", tmp_root.display()))?;
    let work = TempDir::new_in(&tmp_root)
        .map_err(|e| format!("failed to create a scratch folder: {e}"))?;
    crate::describe::describe(DescribeInput::Rust { library }, work.path(), config)
        .await
        .map(|_| ())
}

async fn run_build(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) -> Result<BuildParts, BuildError> {
    let build_started_at = std::time::Instant::now();
    let build_claim_id = UniversalUuid(package_id);

    let (meta, source_bytes) = registry
        .get_source_for_build(package_id)
        .await
        .map_err(|e| BuildError::internal(format!("failed to load source for {package_id}: {e}")))?
        .ok_or_else(|| {
            BuildError::internal(format!(
                "package {package_id} disappeared between claim and build"
            ))
        })?;

    let tmp_root = config.tmp_root_or_default();
    std::fs::create_dir_all(&tmp_root).map_err(|e| {
        BuildError::internal(format!(
            "failed to ensure tmp_root {}: {e}",
            tmp_root.display()
        ))
    })?;

    let work = TempDir::new_in(&tmp_root)
        .map_err(|e| BuildError::internal(format!("failed to create build tmpdir: {e}")))?;

    let archive_path = work.path().join("pkg.cloacina");
    std::fs::write(&archive_path, &source_bytes)
        .map_err(|e| BuildError::internal(format!("failed to stage archive: {e}")))?;

    let extract_dir = work.path().join("source");
    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| BuildError::internal(format!("failed to prepare extract dir: {e}")))?;

    let archive_path_inner = archive_path.clone();
    let extract_dir_inner = extract_dir.clone();
    let source_dir = tokio::task::spawn_blocking(move || {
        cloacina_workflow_plugin::unpack_limit::unpack_package(
            &archive_path_inner,
            &extract_dir_inner,
        )
    })
    .await
    .map_err(|e| BuildError::internal(format!("unpack task panicked: {e}")))?
    .map_err(|e| BuildError::internal(format!("unpack of the package archive failed: {e}")))?;

    let manifest = load_manifest(&source_dir).map_err(BuildError::internal)?;
    let language = manifest_language(&manifest);

    let (task_docs, declared_params, declared_surfaces) = parse_source(&source_dir, &language);

    info!(
        %package_id,
        package_name = %meta.package_name,
        version = %meta.version,
        language = %language,
        "starting build"
    );

    // CLOACI-T-0576: hash Cargo.toml / Cargo.lock for the audit `started`
    // event so operators have content-addressable provenance for every
    // build. Missing files are tolerated (Python packages have no
    // Cargo.toml; some packages have no Cargo.lock).
    let cargo_toml_hash = sha256_hex_if_present(&source_dir.join("Cargo.toml"))
        .map_err(|e| BuildError::internal(format!("failed to hash Cargo.toml: {e}")))?
        .unwrap_or_else(|| "<absent>".to_string());
    let cargo_lock_hash = sha256_hex_if_present(&source_dir.join("Cargo.lock"))
        .map_err(|e| BuildError::internal(format!("failed to hash Cargo.lock: {e}")))?;

    audit::log_compiler_build_started(
        build_claim_id,
        &meta.package_name,
        &meta.version,
        &cargo_toml_hash,
        cargo_lock_hash.as_deref(),
        config.compiler_instance_id,
    );

    let result = match language.as_str() {
        "python" => {
            // CLOACINA-T-1073: no cargo build. The dependencies of the lock
            // file are vendored, one stored set per platform; a package that
            // ships `vendor/` is used as it is.
            debug!("Python package: no cargo build, empty artifact");
            match vendor_python_dependencies(
                registry,
                package_id,
                &meta.package_name,
                &meta.version,
                &manifest,
                &source_dir,
                work.path(),
                config,
            )
            .await
            {
                Ok(()) => Ok(CargoBuildSuccess {
                    artifact: Vec::new(),
                    exit_status: Some(0),
                }),
                Err(reason) => Err(BuildError::Failed {
                    reason,
                    exit_status: None,
                    exit_signal: None,
                }),
            }
        }
        _ => cargo_build(package_id, &source_dir, config).await,
    };

    // Single emit-finished point: classify outcome, compute wall-clock,
    // call the audit fn, then return the original Result up the stack.
    let wall_clock_ms = build_started_at
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let final_result: Result<BuildParts, BuildError> = match result {
        Ok(success) => {
            // CLOACI-T-0836/T-0831: bundle the constructor providers this package
            // consumes (resolve → build per the provider's declared runtime →
            // pack) into `package_providers`, so the reconciler can resolve its
            // constructor nodes hermetically at load. Discovery: RUST scans the
            // source for `constructor!`/`#[reactor]` `from` refs (Cargo.toml is
            // the dep source of truth) AND honors `[metadata.providers]`
            // additively (CLOACI-T-0907 — e.g. a native kafka provider backing a
            // stream accumulator, kept out of the consumer's Cargo graph);
            // PYTHON reads the AUTHORITATIVE `[metadata.providers]` section (no
            // Cargo.toml — a scratch Cargo project is synthesized around the
            // specs). Either way, a declared-but-unbundleable provider FAILS the
            // build (fail closed — the package could never load).
            // Inner Result so a bundling failure still reaches the "failed"
            // audit emit below — a bare `?` here would short-circuit past BOTH
            // the success emit and the Err(..) match arms, leaving the T-0576
            // audit stream with a started event and no finished event.
            let bundle_result: Result<(), BuildError> = async {
                // `[metadata.providers]` from the (untyped) manifest, deserialized
                // through the canonical ProviderDep schema. AUTHORITATIVE for
                // Python (no Cargo.toml); ADDITIVE for Rust (CLOACI-T-0907): a
                // Rust consumer uses it to bundle providers it doesn't want in
                // its own Cargo graph — e.g. a NATIVE rdkafka provider backing a
                // `[[metadata.accumulators]] provider = ..` stream declaration.
                let providers: std::collections::HashMap<
                    String,
                    cloacina::cloacina_workflow_plugin::ProviderDep,
                > = match manifest
                    .get("metadata")
                    .and_then(|m| m.get("providers"))
                    .cloned()
                {
                    Some(v) => v.try_into().map_err(|e| {
                        BuildError::internal(format!(
                            "invalid [metadata.providers] in {} v{}: {e}",
                            meta.package_name, meta.version
                        ))
                    })?,
                    None => Default::default(),
                };
                let specs: Vec<(String, String)> = providers
                    .iter()
                    .map(|(name, dep)| (name.clone(), dep.to_toml_value()))
                    .collect();

                let mut packed = if language == "rust" {
                    let refs =
                        cloacina::packaging::provider_bundle::discover_provider_refs(&source_dir);
                    if refs.is_empty() {
                        Vec::new()
                    } else {
                        info!(
                            %package_id,
                            providers = ?refs.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
                            "bundling constructor providers referenced by the package"
                        );
                        let src = source_dir.clone();
                        // CLOACINA-T-1054: the provider crate is the package's
                        // choice, and bundling builds it and runs its manifest
                        // binary. Those children get the same cleared
                        // environment and limits as the build.
                        let policy = provider_child_policy(config);
                        tokio::task::spawn_blocking(move || {
                            cloacina::packaging::provider_bundle::pack_providers_with(
                                &src, &refs, true, &policy,
                            )
                        })
                        .await
                        .map_err(|e| BuildError::internal(format!("provider bundle join: {e}")))?
                        .map_err(|e| {
                            BuildError::internal(format!(
                                "failed to bundle constructor provider(s) for {} v{}: {e}",
                                meta.package_name, meta.version
                            ))
                        })?
                    }
                } else {
                    Vec::new()
                };

                if !specs.is_empty() {
                    info!(
                        %package_id,
                        providers = ?specs.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
                        "bundling [metadata.providers] declared by the package"
                    );
                    let policy = provider_child_policy(config);
                    let from_specs = tokio::task::spawn_blocking(move || {
                        cloacina::packaging::provider_bundle::pack_providers_from_specs_with(
                            &specs, true, &policy,
                        )
                    })
                    .await
                    .map_err(|e| BuildError::internal(format!("provider bundle join: {e}")))?
                    .map_err(|e| {
                        BuildError::internal(format!(
                            "failed to bundle [metadata.providers] for {} v{}: {e}",
                            meta.package_name, meta.version
                        ))
                    })?;
                    // A provider both Cargo-referenced AND spec-declared bundles once.
                    for p in from_specs {
                        if !packed.iter().any(|q| q.provider_name == p.provider_name) {
                            packed.push(p);
                        }
                    }
                }
                if !packed.is_empty() {
                    use cloacina::packaging::constructor_provider::ProviderRuntime;
                    // CLOACI-T-0908: a PRIMARY compile (no build_target) stores
                    // NULL-triple rows for every provider; a PER-TARGET compile
                    // stores triple-keyed rows for NATIVE providers only — wasm
                    // bundles are arch-neutral, the primary row serves all archs.
                    let target_triple = config.build_target.as_deref();
                    let rows: Vec<(String, String, String, Vec<u8>)> = packed
                        .into_iter()
                        .filter(|p| target_triple.is_none() || p.runtime == ProviderRuntime::Native)
                        .map(|p| {
                            let runtime = match p.runtime {
                                ProviderRuntime::Native => "native".to_string(),
                                ProviderRuntime::Wasm => "wasm".to_string(),
                            };
                            (p.provider_name, p.version, runtime, p.archive)
                        })
                        .collect();
                    if !rows.is_empty() {
                        registry
                            .store_package_providers(
                                &meta.package_name,
                                &meta.version,
                                target_triple,
                                rows,
                            )
                            .await
                            .map_err(|e| {
                                BuildError::internal(format!(
                                    "failed to store bundled providers for {} v{}: {e}",
                                    meta.package_name, meta.version
                                ))
                            })?;
                    }
                }
                Ok(())
            }
            .await;
            // CLOACINA-T-1074: read the package's metadata in a child process
            // and keep the document. After the bundling: a Python import
            // resolves its bundled providers. A describe that fails, or
            // passes its time limit, fails the build with the reason; the
            // inner Result keeps the "failed" audit emit below.
            let described: Result<Option<PackageDescription>, BuildError> = match bundle_result {
                Err(err) => Err(err),
                // A per-target compiler fills a library; the primary
                // compiler describes the package.
                Ok(()) if config.build_target.is_some() => Ok(None),
                Ok(()) => describe_stored(
                    registry,
                    &meta.package_name,
                    &meta.version,
                    &success.artifact,
                    &source_bytes,
                    &source_dir,
                    work.path(),
                    config,
                )
                .await
                .map(Some)
                .map_err(|reason| {
                    BuildError::internal(format!(
                        "describe of {} v{} failed: {reason}",
                        meta.package_name, meta.version
                    ))
                }),
            };
            let mut document = match described {
                Ok(document) => document,
                Err(err) => {
                    let reason = match &err {
                        BuildError::Failed { reason, .. } => reason.clone(),
                        other => format!("{other:?}"),
                    };
                    audit::log_compiler_build_finished(
                        build_claim_id,
                        &meta.package_name,
                        &meta.version,
                        &cargo_toml_hash,
                        cargo_lock_hash.as_deref(),
                        config.compiler_instance_id,
                        "failed",
                        success.exit_status,
                        None,
                        wall_clock_ms,
                        Some(&reason),
                        // Build-process sandbox excised (2026-07-11); the field stays
                        // for audit-schema stability and states the truth.
                        "none",
                    );
                    return Err(err);
                }
            };

            audit::log_compiler_build_finished(
                build_claim_id,
                &meta.package_name,
                &meta.version,
                &cargo_toml_hash,
                cargo_lock_hash.as_deref(),
                config.compiler_instance_id,
                "success",
                success.exit_status,
                None,
                wall_clock_ms,
                None,
                // Build-process sandbox excised (2026-07-11); the field stays
                // for audit-schema stability and states the truth.
                "none",
            );
            info!(
                %package_id,
                artifact_bytes = success.artifact.len(),
                documented_tasks = task_docs.len(),
                declared_params = declared_params.len(),
                "build succeeded"
            );
            // The docs, and for Python the declared params, secrets and
            // surfaces, come from the source parse, not from the code.
            if let Some(doc) = document.as_mut() {
                doc.overlay_source_parse(&task_docs, &declared_params, &declared_surfaces);
            }
            Ok((
                success.artifact,
                task_docs,
                declared_params,
                declared_surfaces,
                document,
            ))
        }
        Err(BuildError::Failed {
            reason,
            exit_status,
            exit_signal,
        }) => {
            audit::log_compiler_build_finished(
                build_claim_id,
                &meta.package_name,
                &meta.version,
                &cargo_toml_hash,
                cargo_lock_hash.as_deref(),
                config.compiler_instance_id,
                "failed",
                exit_status,
                exit_signal.as_deref(),
                wall_clock_ms,
                Some(&reason),
                // Build-process sandbox excised (2026-07-11); the field stays
                // for audit-schema stability and states the truth.
                "none",
            );
            Err(BuildError::Failed {
                reason,
                exit_status,
                exit_signal,
            })
        }
        Err(BuildError::TimedOut { elapsed }) => {
            audit::log_compiler_build_finished(
                build_claim_id,
                &meta.package_name,
                &meta.version,
                &cargo_toml_hash,
                cargo_lock_hash.as_deref(),
                config.compiler_instance_id,
                "timeout_killed",
                None,
                Some("SIGKILL"),
                wall_clock_ms,
                Some(&format!(
                    "cargo build exceeded build_timeout after {}s",
                    elapsed.as_secs()
                )),
                // Build-process sandbox excised (2026-07-11); the field stays
                // for audit-schema stability and states the truth.
                "none",
            );
            Err(BuildError::TimedOut { elapsed })
        }
    };
    final_result
}

/// The Python step of a build (CLOACINA-T-1073): vendor the dependencies the
/// package's lock file names and store one set per platform, or, for a
/// package that ships `vendor/` or has no dependencies, make sure no set of an
/// earlier build of the same version is left. An `Err` is the build error.
///
/// A per-target compiler (`--build-target`) does nothing here: it fills
/// libraries for one architecture, and the primary compiler vendors for every
/// platform of its settings.
#[allow(clippy::too_many_arguments)]
async fn vendor_python_dependencies(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package_id: uuid::Uuid,
    package_name: &str,
    version: &str,
    manifest: &toml::Value,
    source_dir: &Path,
    work: &Path,
    config: &CompilerConfig,
) -> Result<(), String> {
    use crate::python::VendorOutcome;

    if config.build_target.is_some() {
        return Ok(());
    }
    let requires_python = manifest
        .get("metadata")
        .and_then(|m| m.get("requires_python"))
        .and_then(|v| v.as_str());
    let outcome = crate::python::vendor_package(
        package_name,
        version,
        requires_python,
        source_dir,
        work,
        &config.python,
        &child_policy(config),
    )
    .await?;

    let stored = match outcome {
        VendorOutcome::Prebuilt | VendorOutcome::NoDependencies => {
            registry.clear_python_vendor(package_name, version).await
        }
        VendorOutcome::Vendored(sets) => {
            // The package runs where it has a set, and nowhere else.
            let platforms: Vec<String> = sets.iter().map(|s| s.platform.clone()).collect();
            let rows = sets.into_iter().map(|s| (s.platform, s.archive)).collect();
            registry
                .store_python_vendor(package_id, package_name, version, rows, Some(platforms))
                .await
        }
    };
    stored.map_err(|e| {
        format!("failed to store the vendored dependencies of {package_name} v{version}: {e}")
    })
}

fn load_manifest(source_dir: &Path) -> Result<toml::Value, String> {
    let manifest_path = source_dir.join("package.toml");
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {e}", manifest_path.display()))?;
    // CLOACI-T-0735: resolve through the ONE shared resolver so the compiler
    // sees the same defaulted/inferred manifest every other site does — a
    // minimal manifest (no `language`) must branch correctly here too, or we
    // recreate the T-0666 drift class.
    cloacina_workflow_plugin::manifest::resolve_manifest_str(&raw, source_dir)
        .map_err(|e| format!("failed to parse {}: {e}", manifest_path.display()))
}

fn manifest_language(manifest: &toml::Value) -> String {
    // `language` is canonically a `[metadata]` field — that's what the upload
    // schema accepts and what every example/fixture sets. (CLOACI-T-0666: this
    // previously read `[package].language`, which is never populated, so every
    // packaged-Python build wrongly fell through to cargo and failed.) Keep a
    // `[package]` fallback for resilience against older hand-authored manifests.
    manifest
        .get("metadata")
        .and_then(|m| m.get("language"))
        .or_else(|| manifest.get("package").and_then(|p| p.get("language")))
        .and_then(|v| v.as_str())
        .unwrap_or("rust")
        .to_ascii_lowercase()
}

/// Classify a non-zero cargo exit into an operator-actionable error message
/// for the common offline / vendor-missing failure modes. Returns `None` if
/// the stderr doesn't match a known offline-failure shape — the caller
/// falls back to the raw stderr tail.
///
/// Patterns covered (cargo's exact wording can drift across releases; we
/// match conservatively on stable substrings):
///
/// 1. **Missing crates.** `error: no matching package named \`<name>\` found`
///    appears once per missing crate. We aggregate names into a single
///    "dependencies not available offline" message naming each.
/// 2. **Missing Cargo.lock under `--frozen`.** Builds with no lockfile
///    error with `the lock file ... needs to be generated but --frozen
///    was specified`. The operator needs the package author to commit
///    `Cargo.lock`.
/// 3. **Git source offline.** Builds whose deps are git-sourced and not
///    in the vendor cache error with `failed to load source for
///    dependency` + an "offline" mention. Operator vendors the dep.
///
/// CLOACI-T-0574 / SEC-06.
fn classify_offline_failure(stderr: &str) -> Option<String> {
    // 1. Missing-crate aggregation.
    let needle = "no matching package named `";
    let mut missing: Vec<String> = Vec::new();
    let mut cursor = stderr;
    while let Some(start) = cursor.find(needle) {
        let after = &cursor[start + needle.len()..];
        if let Some(end) = after.find('`') {
            let name = after[..end].to_string();
            if !missing.contains(&name) {
                missing.push(name);
            }
            cursor = &after[end..];
        } else {
            break;
        }
    }
    if !missing.is_empty() {
        return Some(format!(
            "dependencies not available offline: {}. \
             Operator workflow: `cargo vendor` against a source tree that \
             includes these crates, then point --vendor-dir at the \
             resulting cargo home.",
            missing.join(", ")
        ));
    }

    // 2. Missing Cargo.lock under --frozen.
    if stderr.contains("lock file")
        && stderr.contains("needs to be generated but --frozen was specified")
    {
        return Some(
            "Cargo.lock is missing from the package source; --frozen requires \
             it. Commit `Cargo.lock` to the package and re-upload."
                .to_string(),
        );
    }

    // 3. Git source unavailable offline.
    if stderr.contains("failed to load source for dependency") && stderr.contains("offline") {
        return Some(
            "Git dependency unavailable offline. Vendor the dependency \
             (`cargo vendor`) and point --vendor-dir at the result."
                .to_string(),
        );
    }

    None
}

/// CLOACI-T-0737: inject the byte-identical build wiring when a minimal
/// package omits it — `[lib] crate-type = ["cdylib","rlib"]` (the compiler
/// NEEDS a cdylib) and the `packaged` feature the macro expansions cfg on.
/// Explicit values are never touched; errors are non-fatal (cargo then
/// reports the real problem with full context).
fn ensure_build_wiring(source_dir: &Path) {
    let manifest_path = source_dir.join("Cargo.toml");
    let Ok(raw) = std::fs::read_to_string(&manifest_path) else {
        return;
    };
    let Ok(mut doc) = raw.parse::<toml::Table>().map(toml::Value::Table) else {
        return;
    };
    let Some(root) = doc.as_table_mut() else {
        return;
    };
    let mut changed = false;

    let lib = root
        .entry("lib")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if let Some(lib) = lib.as_table_mut() {
        if !lib.contains_key("crate-type") {
            lib.insert(
                "crate-type".into(),
                toml::Value::Array(vec![
                    toml::Value::String("cdylib".into()),
                    toml::Value::String("rlib".into()),
                ]),
            );
            changed = true;
        }
    }

    let features = root
        .entry("features")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if let Some(features) = features.as_table_mut() {
        if !features.contains_key("packaged") {
            features.insert("packaged".into(), toml::Value::Array(vec![]));
            changed = true;
        }
    }

    if changed {
        if let Ok(out) = toml::to_string(&doc) {
            let _ = std::fs::write(&manifest_path, out);
            tracing::info!(
                dir = %source_dir.display(),
                "injected minimal-package build wiring (crate-type/packaged feature) — CLOACI-T-0737"
            );
        }
    }
}

/// DEV ESCAPE HATCH (CLOACI-T-0887): when `--dev-workspace <root>` is set, inject
/// a `[patch.crates-io]` section mapping every local cloacina workspace crate
/// (`<root>/crates/*`) to its path. This lets a package that ships
/// PRODUCTION-shaped crates.io version deps (`cloacina-workflow = "0.x"`)
/// resolve against the UNPUBLISHED local crates while cloacina isn't on
/// crates.io yet. NOT for production — real packages resolve from crates.io;
/// this is only for dev/e2e stacks (the demo compiler passes `--dev-workspace`).
fn inject_dev_patch(source_dir: &Path, workspace: &Path) {
    let crates_dir = workspace.join("crates");
    let Ok(entries) = std::fs::read_dir(&crates_dir) else {
        return;
    };
    let mut patches = toml::value::Table::new();
    for entry in entries.flatten() {
        let dir = entry.path();
        let Ok(raw) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
            continue;
        };
        let Ok(doc) = raw.parse::<toml::Table>().map(toml::Value::Table) else {
            continue;
        };
        if let Some(name) = doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        {
            let mut spec = toml::value::Table::new();
            spec.insert(
                "path".into(),
                toml::Value::String(dir.to_string_lossy().to_string()),
            );
            patches.insert(name.to_string(), toml::Value::Table(spec));
        }
    }
    if patches.is_empty() {
        return;
    }
    let manifest_path = source_dir.join("Cargo.toml");
    let Ok(raw) = std::fs::read_to_string(&manifest_path) else {
        return;
    };
    let Ok(mut doc) = raw.parse::<toml::Table>().map(toml::Value::Table) else {
        return;
    };
    let Some(root) = doc.as_table_mut() else {
        return;
    };
    let patch = root
        .entry("patch")
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if let Some(patch) = patch.as_table_mut() {
        patch.insert("crates-io".into(), toml::Value::Table(patches));
    }
    if let Ok(out) = toml::to_string(&doc) {
        let _ = std::fs::write(&manifest_path, out);
        tracing::info!(
            dir = %source_dir.display(),
            workspace = %workspace.display(),
            "DEV: injected [patch.crates-io] -> local workspace crates (CLOACI-T-0887 --dev-workspace)"
        );
    }
}

/// Phase 1 of the two-phase build (CLOACI-T-0887): `cargo fetch`. Fetch only
/// resolves the dependency graph and downloads crates — no build scripts or
/// proc-macros execute. It needs the network and a writable `CARGO_HOME`, so
/// it is a `ChildStep::Resolve` child: the cleared environment of every build
/// child plus the proxy, certificate and cargo network variables, by name
/// (CLOACINA-T-1054). Phase 2 (`cargo build --offline`) is where package code
/// (build.rs / proc-macros) runs. There is no process sandbox around phase 2
/// (the I-0105 bwrap/landlock sandbox was excised 2026-07-11; tenant-level
/// isolation is the security boundary) — but `--offline` keeps cargo itself
/// from resolving anything the fetch didn't pin. Fetch also writes the
/// `Cargo.lock` phase 2 builds against, so the two phases see one graph.
async fn run_cargo_fetch(
    package_id: uuid::Uuid,
    source_dir: &Path,
    config: &CompilerConfig,
) -> Result<(), BuildError> {
    const MAX_ERR: usize = 16 * 1024;
    tracing::info!(
        package_id = %package_id,
        "two-phase build: cargo fetch (no package code runs in this phase)"
    );
    let mut cmd = cargo_child(&child_policy(config), ChildStep::Resolve);
    cmd.arg("fetch")
        .current_dir(source_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = cmd
        .spawn()
        .map_err(|e| BuildError::internal(format!("failed to spawn cargo fetch: {e}")))?;
    let pid = child.id();
    let out = match tokio::time::timeout(config.build_timeout, child.wait_with_output()).await {
        Ok(out) => {
            out.map_err(|e| BuildError::internal(format!("cargo fetch wait failed: {e}")))?
        }
        Err(_) => {
            // Dropping the wait kills cargo (kill_on_drop); the group kill
            // takes what cargo started (a git or credential helper) with it.
            if let Some(pid) = pid {
                kill_child_group(pid);
            }
            return Err(BuildError::internal(format!(
                "cargo fetch timed out after {:?}",
                config.build_timeout
            )));
        }
    };
    if !out.status.success() {
        let mut err = String::from_utf8_lossy(&out.stderr).to_string();
        if err.len() > MAX_ERR {
            err.truncate(MAX_ERR);
        }
        return Err(BuildError::Failed {
            reason: format!("cargo fetch failed:\n{err}"),
            exit_status: out.status.code(),
            exit_signal: None,
        });
    }
    Ok(())
}

async fn cargo_build(
    package_id: uuid::Uuid,
    source_dir: &Path,
    config: &CompilerConfig,
) -> Result<CargoBuildSuccess, BuildError> {
    const MAX_ERR: usize = 64 * 1024;

    ensure_build_wiring(source_dir);

    // DEV hatch (CLOACI-T-0887): resolve version-dep packages against the local
    // unpublished workspace crates. No-op unless `--dev-workspace` is set.
    if let Some(ws) = &config.dev_workspace {
        inject_dev_patch(source_dir, ws);
    }
    // Two-phase build (CLOACI-T-0887): `cargo fetch` resolves + downloads the
    // full graph and pins the Cargo.lock, then the compile runs with
    // `--offline` appended — deterministic (one resolution feeds both phases)
    // and fetch failures surface separately from compile failures. Operators
    // running the curated vendor posture (`--frozen`/`--offline` in
    // --cargo-flag) keep single-phase behavior: their registry is
    // authoritative and a fetch would defeat the curation.
    let operator_offline = config
        .cargo_flags
        .iter()
        .any(|f| f == "--frozen" || f == "--offline");
    if !operator_offline {
        run_cargo_fetch(package_id, source_dir, config).await?;
    }

    // The build-process sandbox (I-0105: bwrap/landlock around this spawn)
    // was EXCISED by maintainer decision (2026-07-11): tenant-level isolation
    // is the security boundary; process sandboxing is a possible future layer
    // if needed. The build runs cargo directly.
    tracing::info!(package_id = %package_id, "spawning build");

    // CLOACI-T-0918: untrusted build.rs / proc-macro code runs in this child,
    // so it must NOT inherit the compiler's ambient environment (which holds
    // DATABASE_URL among other secrets). `cargo_child` clears it and passes
    // only the allowlist, with the rlimits (CLOACI-T-0575) and, through the
    // policy, the `--vendor-dir` CARGO_HOME (CLOACI-T-0574); the deliberate
    // build-wiring vars are injected explicitly below.
    let mut cmd = cargo_child(&child_policy(config), ChildStep::Build);
    cmd.args(&config.cargo_flags);
    if !operator_offline {
        // Phase 2 of the two-phase build: the fetch above resolved + pinned
        // the graph, so the compile resolves nothing new.
        cmd.arg("--offline");
    }
    cmd.current_dir(source_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Share a cargo target cache across builds so transitive deps
    // (cloacina-workflow, cloacina-macros, tokio, …) compile once and are
    // reused by every subsequent package. Without this, every build is a
    // cold ~120-crate compile.
    if let Some(target_dir) = &config.cargo_target_dir {
        std::fs::create_dir_all(target_dir).map_err(|e| {
            BuildError::internal(format!(
                "failed to create cargo_target_dir {}: {e}",
                target_dir.display()
            ))
        })?;
        cmd.env("CARGO_TARGET_DIR", target_dir);
    }
    // Default the produced cdylibs to line-tables-only debuginfo. These are
    // deployment artifacts the agent/server dlopen — never debugged
    // interactively — so full `debuginfo=2` only buys an LLVM memory blowup
    // (OOMs the build of workflows that pull in the heavy `cloacina` lib),
    // slower builds, and a larger artifact to ship by digest. `line-tables-only`
    // keeps file:line in panic backtraces at a fraction of the cost. Only set
    // when the operator hasn't already pinned it via the environment (the demo
    // compose stack, for instance, can force `0`). CLOACI-I-0124.
    if std::env::var_os("CARGO_PROFILE_DEV_DEBUG").is_none() {
        cmd.env("CARGO_PROFILE_DEV_DEBUG", "line-tables-only");
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| BuildError::internal(format!("failed to spawn cargo: {e}")))?;

    let started = std::time::Instant::now();
    let timeout = config.build_timeout;

    // Drain stdout + stderr concurrently with the wait. We want stderr available
    // both for the failure-tail message *and* for parsing missing-dep errors
    // (T-0574) out of cargo's structured output.
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let drain_stdout = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut s) = stdout {
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut s, &mut buf).await;
        }
        buf
    });
    let drain_stderr = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut s) = stderr {
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut s, &mut buf).await;
        }
        buf
    });

    let exit_status: std::process::ExitStatus =
        match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => {
                return Err(BuildError::internal(format!("cargo wait failed: {e}")));
            }
            Err(_elapsed_err) => {
                // Timed out. Send SIGKILL to the cargo child and reap so we
                // don't leak a zombie. kill_on_drop would handle this on drop,
                // but doing it explicitly lets us reap *before* returning so the
                // post-condition is clean.
                let elapsed = started.elapsed();
                warn!(
                    %package_id,
                    elapsed_s = elapsed.as_secs(),
                    timeout_s = timeout.as_secs(),
                    "cargo build exceeded build_timeout; killing subprocess. \
                     Build row will be left for stale-build sweeper to reset."
                );
                // The group kill takes rustc and a running build script with
                // cargo (CLOACINA-T-1054); `child.kill()` then reaps cargo.
                if let Some(pid) = child.id() {
                    kill_child_group(pid);
                }
                if let Err(e) = child.kill().await {
                    warn!(%package_id, %e, "failed to SIGKILL timed-out cargo child");
                }
                // Reap. wait() after kill is bounded — the kernel has already
                // delivered SIGKILL by this point.
                let _ = child.wait().await;
                // Best-effort drain so the spawn handles don't hang their tasks.
                let _ = drain_stdout.await;
                let _ = drain_stderr.await;
                return Err(BuildError::TimedOut { elapsed });
            }
        };

    // Extract cargo's exit code / signal once for both the audit hand-off
    // and the failure-branch reporting. On Unix, signal-terminated children
    // have `code()` = None and `signal()` = Some(num); normal exits flip
    // that around.
    let cargo_exit_status_code: Option<i32> = exit_status.code();
    #[cfg(unix)]
    let cargo_exit_signal_name: Option<String> = {
        use std::os::unix::process::ExitStatusExt;
        exit_status.signal().map(signal_name)
    };
    #[cfg(not(unix))]
    let cargo_exit_signal_name: Option<String> = None;

    // Collect drained pipes (these complete after the child exits since the
    // kernel closes its write end on exit).
    let _stdout_bytes = drain_stdout.await.unwrap_or_default();
    let stderr_bytes = drain_stderr.await.unwrap_or_default();

    if !exit_status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        let tail = if stderr.len() > MAX_ERR {
            let start = stderr.len() - MAX_ERR;
            stderr[start..].to_string()
        } else {
            stderr.to_string()
        };
        warn!(status = ?exit_status, %package_id, "cargo build failed");
        // CLOACI-T-0574: try to classify the failure as an offline/vendor
        // issue first so the operator gets an actionable error instead of
        // a 64 KiB stderr tail.
        let reason = classify_offline_failure(&stderr)
            .unwrap_or_else(|| format!("cargo build failed:\n{tail}"));
        return Err(BuildError::Failed {
            reason,
            exit_status: cargo_exit_status_code,
            exit_signal: cargo_exit_signal_name,
        });
    }

    let target_subdir = profile_for_flags(&config.cargo_flags);
    let target_root = config
        .cargo_target_dir
        .clone()
        .unwrap_or_else(|| source_dir.join("target"));
    let target_dir = target_root.join(target_subdir);
    // With a shared cargo_target_dir, multiple packages' cdylibs coexist.
    // Match on the Cargo.toml [package].name to pick the right one.
    let pkg_name = read_cargo_package_name(source_dir).map_err(BuildError::internal)?;
    let lib_path = find_cdylib(&target_dir, &pkg_name).map_err(BuildError::internal)?;
    let artifact = std::fs::read(&lib_path).map_err(|e| {
        BuildError::internal(format!(
            "failed to read compiled library {}: {e}",
            lib_path.display()
        ))
    })?;

    Ok(CargoBuildSuccess {
        artifact,
        exit_status: cargo_exit_status_code,
    })
}

fn profile_for_flags(flags: &[String]) -> &'static str {
    if flags.iter().any(|f| f == "--release") {
        "release"
    } else {
        "debug"
    }
}

fn find_cdylib(target_dir: &Path, pkg_name: &str) -> Result<PathBuf, String> {
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    // Cargo normalizes `-` to `_` in the emitted libfoo.dylib.
    let normalized = pkg_name.replace('-', "_");
    let expected = format!("lib{}.{}", normalized, ext);

    let candidate = target_dir.join(&expected);
    if candidate.exists() {
        return Ok(candidate);
    }

    Err(format!(
        "expected {} in {} (built package: {})",
        expected,
        target_dir.display(),
        pkg_name
    ))
}

fn read_cargo_package_name(source_dir: &Path) -> Result<String, String> {
    let cargo_toml = source_dir.join("Cargo.toml");
    let raw = std::fs::read_to_string(&cargo_toml)
        .map_err(|e| format!("failed to read {}: {e}", cargo_toml.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .map_err(|e| format!("failed to parse {}: {e}", cargo_toml.display()))?;
    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("[package].name missing in {}", cargo_toml.display()))
}

// ---------------------------------------------------------------------------
// CLOACINA-T-1054: one policy and one command for every child of a build
// ---------------------------------------------------------------------------

/// The policy for every child process the compiler starts for a package:
/// `cargo fetch`, the build, and the provider bundling children (`cargo
/// metadata`, the provider crate's builds, its manifest binary).
///
/// With the I-0105 process sandbox excised (2026-07-11), a package's
/// `build.rs`, its proc-macros and a provider crate it names run with the
/// compiler's own OS privileges. So no child inherits the compiler's
/// environment (`DATABASE_URL`, credentials): the environment is cleared to
/// `cloacina::packaging::build_child::CHILD_ENV_ALLOWLIST` (CLOACI-T-0918),
/// the wall-clock limit is `--build-timeout-s` (CLOACI-T-0573) and the
/// rlimits are `--build-rlimit-*` (CLOACI-T-0575, Linux).
///
/// `--vendor-dir` sets `CARGO_HOME` for all of them (CLOACI-T-0574), so no
/// child resolves from a registry the operator did not curate.
fn child_policy(config: &CompilerConfig) -> ChildPolicy {
    let rlimits = ChildRlimits {
        cpu_s: config.build_rlimits.cpu_s,
        mem_bytes: config.build_rlimits.mem_bytes,
        files: config.build_rlimits.files,
        procs: config.build_rlimits.procs,
    };
    let mut policy = ChildPolicy::confined(config.build_timeout, rlimits);
    if let Some(vendor) = &config.vendor_dir {
        policy = policy.env("CARGO_HOME", vendor);
    }
    policy
}

/// [`child_policy`] for provider bundling. The provider children keep writing
/// where they wrote before the environment was cleared: the `CARGO_TARGET_DIR`
/// of the compiler's own environment when set (the compiler image sets it to
/// its warm cache), else the provider crate's own `target/`. The value is
/// passed by name; the bundler reads it back to find the artifact.
fn provider_child_policy(config: &CompilerConfig) -> ChildPolicy {
    let policy = child_policy(config);
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) if !dir.is_empty() => policy.env("CARGO_TARGET_DIR", dir),
        _ => policy,
    }
}

/// A `cargo` child of the async build path, made by the one function that
/// makes build children. `kill_on_drop`: if the awaited future is dropped
/// (the parent task is cancelled), tokio SIGKILLs cargo rather than leak it.
fn cargo_child(policy: &ChildPolicy, step: ChildStep) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::from(build_child_command("cargo", policy, step));
    cmd.kill_on_drop(true);
    cmd
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BuildRlimits;
    use std::net::SocketAddr;
    use std::time::Instant;

    /// CLOACINA-T-0957: a minimal package gets the cdylib crate-type and the
    /// `packaged` feature. Parsing the manifest as a document (toml 1.x reads
    /// `str::parse::<Value>` as a single value) must not make this a no-op.
    #[test]
    fn ensure_build_wiring_injects_crate_type_and_packaged_feature() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"minimal\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nserde_json = \"1.0\"\n",
        )
        .expect("write Cargo.toml");

        ensure_build_wiring(tmp.path());

        let doc: toml::Table = std::fs::read_to_string(tmp.path().join("Cargo.toml"))
            .expect("read Cargo.toml")
            .parse()
            .expect("valid TOML");
        let crate_type = doc["lib"]["crate-type"].as_array().expect("crate-type");
        assert!(crate_type.iter().any(|v| v.as_str() == Some("cdylib")));
        assert!(doc["features"].get("packaged").is_some());
        assert_eq!(doc["dependencies"]["serde_json"].as_str(), Some("1.0"));
    }

    /// Build a minimal cargo package whose `build.rs` sleeps for `sleep_secs`
    /// seconds. The library itself is an empty cdylib; the slow component is
    /// the build script. Returns the package directory.
    fn synthetic_sleeper_package(work: &Path, sleep_secs: u64) -> PathBuf {
        let pkg = work.join("t0573-sleeper");
        std::fs::create_dir_all(pkg.join("src")).expect("mkdir src");
        std::fs::write(
            pkg.join("Cargo.toml"),
            "[package]\n\
             name = \"t0573-sleeper\"\n\
             version = \"0.0.0\"\n\
             edition = \"2024\"\n\
             \n\
             [lib]\n\
             crate-type = [\"cdylib\"]\n",
        )
        .expect("write Cargo.toml");
        std::fs::write(pkg.join("src/lib.rs"), "").expect("write src/lib.rs");
        std::fs::write(
            pkg.join("build.rs"),
            format!(
                "fn main() {{\n    \
                     std::thread::sleep(std::time::Duration::from_secs({sleep_secs}));\n\
                 }}\n"
            ),
        )
        .expect("write build.rs");
        pkg
    }

    fn test_config(home: &Path, build_timeout: Duration) -> CompilerConfig {
        CompilerConfig {
            home: home.to_path_buf(),
            bind: "127.0.0.1:0".parse::<SocketAddr>().expect("parse bind"),
            metrics_bind: None,
            database_url: "unused-in-this-test".to_string(),
            verbose: false,
            poll_interval: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(5),
            stale_threshold: Duration::from_secs(30),
            sweep_interval: Duration::from_secs(15),
            cargo_flags: vec!["build".to_string()],
            tmp_root: None,
            cargo_target_dir: Some(home.join("target")),
            build_timeout,
            vendor_dir: None,
            dev_workspace: None,
            // Generous rlimits for the regular tests so a normal cargo
            // build never trips them; rlimit-specific tests override the
            // relevant field explicitly.
            build_rlimits: BuildRlimits {
                cpu_s: 600,
                mem_bytes: 8 * 1024 * 1024 * 1024,
                files: 4096,
                procs: 4096,
            },
            compiler_instance_id: UniversalUuid::new_v4(),
            log_retention_days: 7,
            tenant_schema: None,
            build_target: None,
            build_target_package: None,
            python: Default::default(),
            describe_timeout: Duration::from_secs(60),
            describe_program: None,
        }
    }

    /// CLOACI-T-0573: a build whose `build.rs` exceeds `--build-timeout-s`
    /// returns `BuildError::TimedOut` within a bounded wall-clock window.
    ///
    /// We set `build_timeout = 5s` and `build.rs` sleeps for 60s. Whether
    /// the timeout fires during cargo's startup, during `build.rs` compile,
    /// or while `build.rs` is sleeping, the contract is the same: cargo is
    /// SIGKILL'd, drains complete, and `TimedOut` surfaces — bounded.
    ///
    /// Serial: another test of this module puts a stand-in `cargo` on `PATH`.
    #[tokio::test]
    #[serial_test::serial]
    async fn cargo_build_returns_timed_out_when_build_rs_sleeps_past_timeout() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let package = synthetic_sleeper_package(tmp.path(), 60);
        let config = test_config(tmp.path(), Duration::from_secs(5));

        let started = Instant::now();
        let result = cargo_build(uuid::Uuid::new_v4(), &package, &config).await;
        let elapsed = started.elapsed();

        // Wall-clock must be bounded by build_timeout + a tolerance for the
        // SIGKILL + reap + drain steps. If the kill path is wrong (e.g. we
        // leak the child or fail to reap), this assertion catches it.
        assert!(
            elapsed < Duration::from_secs(20),
            "cargo_build hung past the configured 5s timeout (elapsed = {elapsed:?})"
        );

        match result {
            Err(BuildError::TimedOut { elapsed: reported }) => {
                // Reported elapsed should be in the same ballpark as the
                // configured timeout — Instant::elapsed inside cargo_build
                // measures from spawn to the timeout firing.
                assert!(
                    reported >= Duration::from_secs(4) && reported < Duration::from_secs(20),
                    "reported elapsed {reported:?} not in [4s, 20s)"
                );
            }
            other => panic!("expected BuildError::TimedOut, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0918 / CLOACINA-T-1054: every child's environment and limits
    // -----------------------------------------------------------------------

    /// Print the environment of a child made the way the build path makes
    /// its cargo children, with `env` in place of `cargo`.
    async fn child_env(policy: &ChildPolicy, step: ChildStep) -> String {
        let mut cmd = tokio::process::Command::from(build_child_command("env", policy, step));
        let out = cmd.output().await.expect("spawn `env`");
        assert!(out.status.success(), "`env` exited nonzero");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn has_var(printed: &str, key: &str) -> bool {
        let prefix = format!("{key}=");
        printed.lines().any(|l| l.starts_with(&prefix))
    }

    /// No child of a build sees the compiler's own variables: not the fetch,
    /// not the build, not the provider bundling children. `PATH` passes.
    #[tokio::test]
    #[serial_test::serial]
    async fn no_build_child_sees_the_compiler_environment() {
        let secrets = [
            ("DATABASE_URL", "postgres://user:secret@host/db"),
            ("CLOACINA_DATABASE_URL", "postgres://user:secret@host/db"),
            ("CLOACINA_T_1054_MARKER", "compiler-only"),
        ];
        for (k, v) in secrets {
            // SAFETY: serial test; no other thread of this test reads the env.
            unsafe { std::env::set_var(k, v) };
        }

        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = test_config(tmp.path(), Duration::from_secs(30));
        config.database_url = "postgres://user:config-secret@host/db".to_string();
        let children = [
            ("cargo fetch", child_policy(&config), ChildStep::Resolve),
            ("cargo build", child_policy(&config), ChildStep::Build),
            (
                "cargo metadata (providers)",
                provider_child_policy(&config),
                ChildStep::Resolve,
            ),
            (
                "provider build and manifest binary",
                provider_child_policy(&config),
                ChildStep::Build,
            ),
        ];
        for (what, policy, step) in &children {
            let printed = child_env(policy, *step).await;
            for (k, _) in secrets {
                assert!(!has_var(&printed, k), "{k} reached {what}:\n{printed}");
            }
            assert!(
                !printed.contains("secret"),
                "a secret value reached {what}:\n{printed}"
            );
            assert!(
                has_var(&printed, "PATH"),
                "PATH must pass through to {what}:\n{printed}"
            );
            assert!(policy.is_confined(), "{what} must be confined");
            assert_eq!(
                policy.timeout(),
                Some(config.build_timeout),
                "{what} must carry the build time limit"
            );
        }

        for (k, _) in secrets {
            // SAFETY: as above.
            unsafe { std::env::remove_var(k) };
        }
    }

    /// Allowlisted vars present in the parent env pass through unchanged.
    #[tokio::test]
    #[serial_test::serial]
    async fn compile_env_passes_allowlisted_vars_through() {
        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("CARGO_PROFILE_DEV_DEBUG", "0") };

        let tmp = tempfile::tempdir().expect("tempdir");
        let config = test_config(tmp.path(), Duration::from_secs(30));
        let printed = child_env(&child_policy(&config), ChildStep::Build).await;
        assert!(
            printed.lines().any(|l| l == "CARGO_PROFILE_DEV_DEBUG=0"),
            "allowlisted CARGO_PROFILE_DEV_DEBUG must pass through:\n{printed}"
        );

        // FIXME: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var("CARGO_PROFILE_DEV_DEBUG") };
    }

    /// `--vendor-dir` reaches every child as `CARGO_HOME`, and the provider
    /// children get the compiler's own `CARGO_TARGET_DIR` by name.
    #[tokio::test]
    #[serial_test::serial]
    async fn vendor_dir_and_target_dir_reach_children_by_name() {
        // SAFETY: serial test; no other thread of this test reads the env.
        unsafe { std::env::set_var("CARGO_TARGET_DIR", "/compiler/target") };
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = test_config(tmp.path(), Duration::from_secs(30));
        config.vendor_dir = Some(PathBuf::from("/curated/cargo-home"));

        let build = child_env(&child_policy(&config), ChildStep::Build).await;
        let provider = child_env(&provider_child_policy(&config), ChildStep::Build).await;
        // SAFETY: as above.
        unsafe { std::env::remove_var("CARGO_TARGET_DIR") };

        for printed in [&build, &provider] {
            assert!(
                printed
                    .lines()
                    .any(|l| l == "CARGO_HOME=/curated/cargo-home"),
                "{printed}"
            );
        }
        assert!(!has_var(&build, "CARGO_TARGET_DIR"), "{build}");
        assert!(
            provider
                .lines()
                .any(|l| l == "CARGO_TARGET_DIR=/compiler/target"),
            "{provider}"
        );
    }

    /// CLOACINA-T-1054: the compiler starts no process except through
    /// `build_child_command`, in its own source and in the packaging code it
    /// calls; and it calls no bundling entry that lets a child inherit the
    /// compiler's environment. A new call site fails this test.
    #[test]
    fn compiler_starts_no_process_outside_the_one_function() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let compiler_src = manifest.join("src");
        let packaging = manifest.join("../cloacina/src/packaging");

        let spawn = ["Command", "::", "new"].concat();
        let mut hits = source_lines_with(&compiler_src, &[spawn.as_str()], &[]);
        hits.extend(source_lines_with(
            &packaging,
            &[spawn.as_str()],
            &["build_child.rs"],
        ));
        assert!(
            hits.is_empty(),
            "process started outside packaging/build_child.rs:\n{}",
            hits.join("\n")
        );

        // The entries below give their children the caller's whole environment.
        let open_entries: Vec<String> = [
            ["ChildPolicy::", "inherit"].concat(),
            ["pack_providers", "("].concat(),
            ["pack_providers_from_specs", "("].concat(),
            ["bundle_providers", "("].concat(),
            ["resolve_provider_crate", "("].concat(),
            ["package_constructor_provider", "("].concat(),
        ]
        .into_iter()
        .collect();
        let needles: Vec<&str> = open_entries.iter().map(String::as_str).collect();
        let hits = source_lines_with(&compiler_src, &needles, &[]);
        assert!(
            hits.is_empty(),
            "the compiler calls a bundling entry with an inherited environment:\n{}",
            hits.join("\n")
        );
    }

    /// CLOACINA-T-1074: the compiler process opens no built library. Only the
    /// describe child does, and its code is in `describe.rs`. The registry
    /// entries that open a library in the calling process are not called from
    /// the compiler either.
    #[test]
    fn compiler_opens_no_library_outside_the_describe_child() {
        let compiler_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let opens: Vec<String> = [
            ["load_", "library"].concat(),
            ["extract_", "metadata"].concat(),
            ["extract_", "graph_metadata"].concat(),
            ["extract_", "reactor_metadata"].concat(),
            ["extract_", "trigger_metadata"].concat(),
            ["extract_", "constructor_metadata"].concat(),
            ["extract_", "triggerless_graph_metadata"].concat(),
            ["describe_", "library"].concat(),
            ["PackageLoader", "::new"].concat(),
            ["TaskRegistrar", "::new"].concat(),
            ["mark_build_success", "("].concat(),
            ["mark_build_success_with_docs", "("].concat(),
            ["load_workflow_", "package"].concat(),
            ["load_cg_", "package"].concat(),
        ]
        .into_iter()
        .collect();
        let needles: Vec<&str> = opens.iter().map(String::as_str).collect();
        let hits = source_lines_with(&compiler_src, &needles, &["describe.rs"]);
        assert!(
            hits.is_empty(),
            "the compiler opens a library, or imports a package, outside describe.rs:\n{}",
            hits.join("\n")
        );

        // In describe.rs they are in the child's half only: after the line
        // that starts it.
        let text = std::fs::read_to_string(compiler_src.join("describe.rs")).expect("describe.rs");
        let child_half = text
            .find("// The child's half")
            .expect("describe.rs marks the child's half");
        for needle in &needles {
            let in_parent_half = text[..child_half]
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .any(|l| l.contains(needle));
            assert!(
                !in_parent_half,
                "`{needle}` is in the compiler's half of describe.rs"
            );
        }
    }

    /// Lines of the `.rs` files under `dir` (recursive) that hold one of
    /// `needles`. Comment lines do not count. Files named in `skip` are not
    /// read.
    fn source_lines_with(dir: &Path, needles: &[&str], skip: &[&str]) -> Vec<String> {
        let mut hits = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        let mut files = 0;
        while let Some(d) = stack.pop() {
            let entries =
                std::fs::read_dir(&d).unwrap_or_else(|e| panic!("read dir {}: {e}", d.display()));
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.ends_with(".rs") || skip.contains(&name) {
                    continue;
                }
                files += 1;
                let text = std::fs::read_to_string(&path).expect("read source");
                for (i, line) in text.lines().enumerate() {
                    if line.trim_start().starts_with("//") {
                        continue;
                    }
                    if needles.iter().any(|n| line.contains(n)) {
                        hits.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
                    }
                }
            }
        }
        assert!(files > 0, "no source read under {}", dir.display());
        hits
    }

    /// A `cargo fetch` that outlives `--build-timeout-s` is killed and the
    /// build fails. The "cargo" here is a script that sleeps: the fetch child
    /// finds it first on `PATH`, which the allowlist passes.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial]
    async fn cargo_fetch_past_the_time_limit_is_killed() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().expect("tempdir");
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&bin).expect("mkdir bin");
        let fake = bin.join("cargo");
        std::fs::write(&fake, "#!/bin/sh\nsleep 30\n").expect("write fake cargo");
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let old_path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![bin.clone()];
        paths.extend(std::env::split_paths(&old_path));
        // SAFETY: serial test; no other thread of this test reads the env.
        unsafe { std::env::set_var("PATH", std::env::join_paths(paths).expect("join PATH")) };

        let config = test_config(tmp.path(), Duration::from_secs(1));
        let started = Instant::now();
        let result = run_cargo_fetch(uuid::Uuid::new_v4(), tmp.path(), &config).await;
        let elapsed = started.elapsed();

        // SAFETY: as above.
        unsafe { std::env::set_var("PATH", old_path) };

        let err = result.expect_err("a sleeping fetch must fail");
        assert!(
            format!("{err:?}").contains("timed out"),
            "expected a time-out, got {err:?}"
        );
        assert!(
            elapsed < Duration::from_secs(15),
            "the kill took {elapsed:?}"
        );
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0574: offline-failure classifier
    // -----------------------------------------------------------------------

    #[test]
    fn classify_offline_failure_extracts_single_missing_crate() {
        let stderr = "\
error: no matching package named `unobtanium` found\n\
location searched: registry `crates-io`\n\
required by package `t0574-victim v0.0.0 (/tmp/t0574)`\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(
            msg.contains("unobtanium"),
            "classifier output should name the missing crate, got: {msg}"
        );
        assert!(
            msg.starts_with("dependencies not available offline:"),
            "should use the canonical missing-deps preamble, got: {msg}"
        );
    }

    #[test]
    fn classify_offline_failure_aggregates_multiple_missing_crates() {
        let stderr = "\
error: no matching package named `alpha` found\n\
required by package `victim v0.0.0`\n\
error: no matching package named `beta` found\n\
required by package `victim v0.0.0`\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(msg.contains("alpha"), "should name alpha, got: {msg}");
        assert!(msg.contains("beta"), "should name beta, got: {msg}");
    }

    #[test]
    fn classify_offline_failure_dedupes_repeated_missing_crate_mentions() {
        let stderr = "\
error: no matching package named `alpha` found\n\
error: no matching package named `alpha` found\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        // The crate should appear exactly once in the joined output (`, alpha`
        // would indicate a duplicate after the first).
        let occurrences = msg.matches("alpha").count();
        assert_eq!(occurrences, 1, "alpha should appear once, got: {msg}");
    }

    #[test]
    fn classify_offline_failure_recognizes_missing_lockfile() {
        let stderr = "\
error: the lock file /tmp/t0574/Cargo.lock needs to be generated but --frozen was specified\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(
            msg.contains("Cargo.lock"),
            "should mention Cargo.lock, got: {msg}"
        );
        assert!(
            msg.contains("--frozen"),
            "should mention --frozen, got: {msg}"
        );
    }

    #[test]
    fn classify_offline_failure_recognizes_git_dep_offline() {
        let stderr = "\
error: failed to load source for dependency `mygit`\n\
Caused by:\n  attempting to make an HTTP request, but --offline was specified\n";
        let msg = classify_offline_failure(stderr).expect("should classify");
        assert!(
            msg.to_lowercase().contains("git"),
            "should mention git, got: {msg}"
        );
        assert!(
            msg.contains("--vendor-dir"),
            "should point at --vendor-dir, got: {msg}"
        );
    }

    #[test]
    fn classify_offline_failure_returns_none_for_unrelated_stderr() {
        // A regular compile error should not be misclassified as an
        // offline-failure — the caller falls back to the raw stderr tail.
        let stderr = "error[E0425]: cannot find value `foo` in this scope\n";
        assert!(classify_offline_failure(stderr).is_none());
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0575: setrlimit kills a build that overshoots its limits
    // -----------------------------------------------------------------------

    /// A `build.rs` that tries to allocate 8 GiB. Under RLIMIT_AS=128 MiB,
    /// the allocation aborts the build.rs process; cargo observes the
    /// build-script failure and exits non-zero.
    #[cfg(target_os = "linux")]
    fn synthetic_memory_hog_package(work: &Path) -> PathBuf {
        let pkg = work.join("t0575-memhog");
        std::fs::create_dir_all(pkg.join("src")).expect("mkdir src");
        std::fs::write(
            pkg.join("Cargo.toml"),
            "[package]\n\
             name = \"t0575-memhog\"\n\
             version = \"0.0.0\"\n\
             edition = \"2024\"\n\
             \n\
             [lib]\n\
             crate-type = [\"cdylib\"]\n",
        )
        .expect("write Cargo.toml");
        std::fs::write(pkg.join("src/lib.rs"), "").expect("write src/lib.rs");
        std::fs::write(
            pkg.join("build.rs"),
            "fn main() {\n    \
                 // Try to reserve 8 GiB. With RLIMIT_AS well below this,\n    \
                 // the allocation aborts the process before main returns.\n    \
                 let big: Vec<u8> = vec![0u8; 8 * 1024 * 1024 * 1024];\n    \
                 // Touch a byte so the optimizer can't elide the alloc.\n    \
                 std::hint::black_box(&big[0]);\n\
             }\n",
        )
        .expect("write build.rs");
        pkg
    }

    /// CLOACI-T-0575 (Linux only): a build whose `build.rs` allocates past
    /// `RLIMIT_AS` is killed by the kernel; `cargo_build` surfaces it as
    /// `BuildError::Failed` (not `TimedOut` — the wall-clock budget was
    /// never approached).
    ///
    /// We squeeze RLIMIT_AS to 128 MiB. The build script attempts an 8 GiB
    /// allocation; on Linux the kernel refuses, glibc aborts, cargo exits
    /// non-zero.
    ///
    /// Serial: another test of this module puts a stand-in `cargo` on `PATH`.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    #[serial_test::serial]
    async fn cargo_build_fails_when_build_rs_overshoots_rlimit_as() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let package = synthetic_memory_hog_package(tmp.path());
        let mut config = test_config(tmp.path(), Duration::from_secs(60));
        // 128 MiB — well under the 8 GiB the build.rs wants. Keep CPU and
        // other limits generous so we only test the AS dimension.
        config.build_rlimits.mem_bytes = 128 * 1024 * 1024;

        let started = std::time::Instant::now();
        let result = cargo_build(uuid::Uuid::new_v4(), &package, &config).await;
        let elapsed = started.elapsed();

        // The build should fail fast — well under the 60s wall-clock
        // budget. If we exceed 30s the test is suspicious (kernel didn't
        // enforce the limit, or pre_exec isn't actually wired).
        assert!(
            elapsed < Duration::from_secs(30),
            "build did not fail fast under RLIMIT_AS (elapsed = {elapsed:?})"
        );

        match result {
            Err(BuildError::Failed { .. }) => { /* expected */ }
            Err(BuildError::TimedOut { .. }) => {
                panic!("RLIMIT_AS overshoot reported as TimedOut; kernel didn't enforce")
            }
            Ok(_) => panic!("RLIMIT_AS=128MiB should have aborted an 8 GiB allocation"),
        }
    }
}
