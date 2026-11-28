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

//! Main build loop. Orchestrates claim → build-with-heartbeat → mark success
//! or failed, plus a sweeper tick that resets rows whose heartbeats have gone
//! stale.

use std::sync::Arc;

use anyhow::Result;
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::CompilerConfig;

/// Run a single build with a heartbeat task running alongside it. The
/// heartbeat keeps `build_claimed_at` fresh so the sweeper doesn't reset the
/// row; it is cancelled as soon as the build terminates (success or failure),
/// before the final mark_build_* UPDATE fires — preventing a stray heartbeat
/// from overwriting the terminal state.
async fn run_build_with_heartbeat(
    registry: Arc<WorkflowRegistryImpl<UnifiedRegistryStorage>>,
    package_id: uuid::Uuid,
    config: &CompilerConfig,
) {
    let heartbeat_cancel = CancellationToken::new();
    let hb_token = heartbeat_cancel.clone();
    let hb_interval = config.heartbeat_interval;
    let hb_registry = Arc::clone(&registry);
    let heartbeat = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(hb_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // Skip the immediate tick — claim_next_build already set a fresh
        // build_claimed_at moments ago.
        ticker.tick().await;
        loop {
            tokio::select! {
                _ = hb_token.cancelled() => return,
                _ = ticker.tick() => {
                    if let Err(e) = hb_registry.heartbeat_build(package_id).await {
                        warn!(%e, %package_id, "heartbeat update failed");
                        metrics::counter!("cloacina_compiler_heartbeat_failures_total")
                            .increment(1);
                    }
                }
            }
        }
    });

    let build_started = std::time::Instant::now();
    let outcome = crate::build::execute_build(&registry, package_id, config).await;
    metrics::histogram!("cloacina_compiler_build_duration_seconds")
        .record(build_started.elapsed().as_secs_f64());

    heartbeat_cancel.cancel();
    let _ = heartbeat.await;

    match outcome {
        crate::build::BuildOutcome::Success {
            artifact,
            task_docs,
            declared_params,
            declared_surfaces,
            document,
        } => {
            // CLOACINA-T-1074: the compiler does not open the library. The
            // describe child read its metadata; the document is stored with
            // the build, and the row's `metadata` is filled from it.
            let Some(document) = document else {
                metrics::counter!(
                    "cloacina_compiler_builds_total",
                    "status" => "failed",
                )
                .increment(1);
                warn!(%package_id, "build gave no metadata document");
                if let Err(e) = registry
                    .mark_build_failed(package_id, "the build gave no metadata document")
                    .await
                {
                    warn!(%e, %package_id, "mark_build_failed failed");
                }
                return;
            };
            metrics::counter!(
                "cloacina_compiler_builds_total",
                "status" => "ok",
            )
            .increment(1);
            // CLOACI-T-0752: carry compiler-parsed per-task docs into the
            // persisted metadata alongside the compiled artifact.
            // CLOACI-T-0760: also carry source-parsed declared params (the
            // Python parity path — Rust gets them from the FFI instead).
            if let Err(e) = registry
                .mark_build_success_described(
                    package_id,
                    artifact,
                    &document,
                    task_docs,
                    declared_params,
                    declared_surfaces,
                )
                .await
            {
                warn!(%e, %package_id, "mark_build_success failed");
            }
        }
        crate::build::BuildOutcome::Failed(err) => {
            metrics::counter!(
                "cloacina_compiler_builds_total",
                "status" => "failed",
            )
            .increment(1);
            warn!(%package_id, "build failed");
            // CLOACINA-T-1085: the one rebuild the in-process path asked for
            // still has another interface. The row goes back to its library,
            // in service, marked as needing a new publish. A new upload of
            // another interface fails as any build does.
            if cloacina::registry::plugin_interface::is_stale_artifact_error(&err) {
                use cloacina::registry::plugin_interface::{
                    built_for_of_error, needs_new_build_reason,
                };
                let built_for = built_for_of_error(&err);
                let reason = needs_new_build_reason(
                    built_for,
                    "it resolves the crates of the old release again (a pin to the old minor, \
                     or its Cargo.lock)",
                );
                match registry
                    .end_interface_rebuild(package_id, built_for, &reason)
                    .await
                {
                    Ok(true) => {
                        warn!(%package_id, %reason, "a package needs a new publish");
                        return;
                    }
                    Ok(false) => {}
                    Err(e) => warn!(%e, %package_id, "end_interface_rebuild failed"),
                }
            }
            if let Err(e) = registry.mark_build_failed(package_id, &err).await {
                warn!(%e, %package_id, "mark_build_failed failed");
            }
        }
        crate::build::BuildOutcome::TimedOut { elapsed } => {
            metrics::counter!(
                "cloacina_compiler_builds_total",
                "status" => "timed_out",
            )
            .increment(1);
            // CLOACI-T-0573: heartbeat was cancelled above; row's
            // build_claimed_at is now stale. The sweeper will reclaim it on
            // its next tick (stale_threshold after the last heartbeat).
            // Do NOT call mark_build_failed — the row should be reset to
            // `pending`, not terminally failed.
            warn!(
                %package_id,
                elapsed_s = elapsed.as_secs(),
                "build timed out; leaving row for stale-build sweeper to reset"
            );
        }
    }
}

/// The most packages one pass of [`describe_pass`] describes. The pass runs
/// between builds, on the sweep tick: a small batch keeps a new upload from
/// waiting behind a long list.
const DESCRIBE_PASS_BATCH: i64 = 5;

/// Describe packages that are in service and have no metadata document of
/// this compiler's format (CLOACINA-T-1074): packages built before the
/// document existed, and packages whose build was taken over from an
/// identical upload. Returns how many packages it worked on.
///
/// No build runs and no row leaves service. The stored library (for Python,
/// the stored archive) is described in a child process. A package that can be
/// described gets its document. One that cannot gets `describe_error` with
/// the reason, and is not tried again by this compiler version.
pub async fn describe_pass(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    config: &CompilerConfig,
) -> usize {
    let packages = match registry
        .list_undescribed_packages(DESCRIBE_PASS_BATCH)
        .await
    {
        Ok(packages) => packages,
        Err(e) => {
            warn!(%e, "list_undescribed_packages failed");
            return 0;
        }
    };
    let count = packages.len();
    for package in packages {
        let described = crate::build::describe_existing(registry, package.id, config).await;
        let stored = match &described {
            Ok(document) => {
                info!(
                    package_id = %package.id,
                    package_name = %package.package_name,
                    version = %package.version,
                    tasks = document.tasks.len(),
                    "described a package built before"
                );
                metrics::counter!("cloacina_compiler_describes_total", "status" => "ok")
                    .increment(1);
                registry
                    .store_package_description(package.id, document)
                    .await
            }
            Err(reason) => {
                warn!(
                    package_id = %package.id,
                    package_name = %package.package_name,
                    version = %package.version,
                    %reason,
                    "a package built before cannot be described"
                );
                metrics::counter!("cloacina_compiler_describes_total", "status" => "failed")
                    .increment(1);
                registry.store_describe_error(package.id, reason).await
            }
        };
        if let Err(e) = stored {
            warn!(%e, package_id = %package.id, "storing the describe result failed");
        }
    }
    count
}

/// The most packages one pass of [`interface_pass`] checks. A check can
/// rebuild a package, so the batch is small.
const INTERFACE_PASS_BATCH: i64 = 5;

/// The longest text of a failed rebuild that goes into the reason a team
/// reads.
const MAX_WHY: usize = 2000;

/// Check packages in service for this compiler's plugin interface
/// (CLOACINA-T-1085): rows with no interface recorded, or another one, that
/// were not checked for this interface yet. Returns how many it worked on.
///
/// A Python package has no interface: it is recorded as checked. A Rust
/// package's stored library is opened in the describe child. It opens: the
/// row gets this interface. It was built for another interface or fidius
/// ABI: the package gets one rebuild from its kept source, in place, and
/// stays in service meanwhile. The rebuild has this interface: the row gets
/// the new library and runs with no publish. It does not (the package pins
/// the old minor, or its Cargo.lock does), or the rebuild fails: the row is
/// marked as needing a new publish, with the reason and the step. Each row
/// is checked once per interface version, across restarts and replicas.
pub async fn interface_pass(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    config: &CompilerConfig,
) -> usize {
    let packages = match registry
        .list_packages_to_check_interface(INTERFACE_PASS_BATCH)
        .await
    {
        Ok(packages) => packages,
        Err(e) => {
            warn!(%e, "list_packages_to_check_interface failed");
            return 0;
        }
    };
    let count = packages.len();
    for package in packages {
        if let Err(e) = check_interface(registry, &package, config).await {
            warn!(%e, package_id = %package.id, "storing the interface check failed");
        }
    }
    count
}

async fn check_interface(
    registry: &WorkflowRegistryImpl<UnifiedRegistryStorage>,
    package: &cloacina::registry::workflow_registry::InterfaceCheck,
    config: &CompilerConfig,
) -> Result<(), cloacina::registry::RegistryError> {
    use cloacina::registry::plugin_interface::{
        built_for_of_error, is_stale_artifact_error, needs_new_build_reason,
    };

    let library = registry
        .get_compiled_data_by_id(package.id)
        .await?
        .unwrap_or_default();
    if library.is_empty() {
        // A Python package: no plugin interface.
        registry.record_interface_checked(package.id).await?;
        return Ok(());
    }
    let opened = crate::build::open_stored_library(&library, config).await;
    let stale = match opened {
        Ok(()) => {
            registry.record_interface_current(package.id).await?;
            return Ok(());
        }
        Err(reason) if is_stale_artifact_error(&reason) => reason,
        Err(reason) => {
            // Not the interface: the describe pass reports what it can.
            warn!(
                package_id = %package.id,
                package_name = %package.package_name,
                version = %package.version,
                %reason,
                "the stored library of a package does not open; not an interface mismatch"
            );
            registry.record_interface_checked(package.id).await?;
            return Ok(());
        }
    };
    let old = built_for_of_error(&stale).or(package.plugin_interface_version);
    info!(
        package_id = %package.id,
        package_name = %package.package_name,
        version = %package.version,
        built_for = ?old,
        "a package was built for another plugin interface; one rebuild from its kept source"
    );
    let mark = |built_for: Option<u32>, why: String| {
        let reason = needs_new_build_reason(built_for, &why);
        async move {
            warn!(
                package_id = %package.id,
                package_name = %package.package_name,
                version = %package.version,
                %reason,
                "a package needs a new publish"
            );
            metrics::counter!("cloacina_compiler_interface_checks_total", "status" => "needs_new_build")
                .increment(1);
            registry
                .mark_needs_new_build(package.id, built_for, &reason)
                .await
                .map(|_| ())
        }
    };
    match crate::build::execute_build(registry, package.id, config).await {
        crate::build::BuildOutcome::Success {
            artifact,
            task_docs,
            declared_params,
            declared_surfaces,
            document: Some(document),
        } if !artifact.is_empty() => {
            let written = registry
                .replace_build_in_place(
                    package.id,
                    artifact,
                    &document,
                    task_docs,
                    declared_params,
                    declared_surfaces,
                )
                .await?;
            if written {
                info!(
                    package_id = %package.id,
                    package_name = %package.package_name,
                    version = %package.version,
                    "the rebuild from kept source has this plugin interface; the package runs \
                     with no new publish"
                );
                metrics::counter!("cloacina_compiler_interface_checks_total", "status" => "rebuilt")
                    .increment(1);
            }
            Ok(())
        }
        crate::build::BuildOutcome::Success { .. } => {
            mark(
                old,
                "the rebuild gave no library or no metadata document".into(),
            )
            .await
        }
        crate::build::BuildOutcome::Failed(err) => {
            let built_for = built_for_of_error(&err).or(old);
            let why = if is_stale_artifact_error(&err) {
                "it resolves the crates of the old release again (a pin to the old minor, or \
                 its Cargo.lock)"
                    .to_string()
            } else {
                format!("the rebuild failed: {}", tail(&err))
            };
            mark(built_for, why).await
        }
        crate::build::BuildOutcome::TimedOut { elapsed } => {
            mark(
                old,
                format!("the rebuild timed out after {}s", elapsed.as_secs()),
            )
            .await
        }
    }
}

/// The end of a long text, on a character boundary.
fn tail(text: &str) -> String {
    let text = text.trim();
    if text.len() <= MAX_WHY {
        return text.to_string();
    }
    let mut start = text.len() - MAX_WHY;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("…{}", &text[start..])
}

pub(crate) async fn run(
    registry: Arc<WorkflowRegistryImpl<UnifiedRegistryStorage>>,
    config: CompilerConfig,
    shutdown: CancellationToken,
) -> Result<()> {
    let mut poll_ticker = tokio::time::interval(config.poll_interval);
    poll_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut sweep_ticker = tokio::time::interval(config.sweep_interval);
    sweep_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                info!("shutdown requested — exiting build loop");
                return Ok(());
            }
            _ = poll_ticker.tick() => {
                match registry.claim_next_build().await {
                    Ok(Some(claim)) => {
                        info!(
                            package_id = %claim.id,
                            package_name = %claim.package_name,
                            version = %claim.version,
                            "claimed build"
                        );
                        run_build_with_heartbeat(Arc::clone(&registry), claim.id, &config).await;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!(%e, "claim_next_build failed; backing off one tick");
                    }
                }
            }
            _ = sweep_ticker.tick() => {
                match registry.sweep_stale_builds(config.stale_threshold).await {
                    Ok(0) => {}
                    Ok(n) => {
                        info!(reset = n, "swept stale builds");
                        metrics::counter!("cloacina_compiler_sweep_resets_total")
                            .increment(n as u64);
                    }
                    Err(e) => warn!(%e, "sweep_stale_builds failed"),
                }
                // CLOACINA-T-1074: packages in service with no metadata
                // document get one, from their stored build.
                describe_pass(&registry, &config).await;
                // CLOACINA-T-1085: packages in service built for another
                // plugin interface get one rebuild, or the mark.
                interface_pass(&registry, &config).await;
                // SQL-derived queue-depth gauge (REC-06 / I-0108 pattern) —
                // re-seeded every sweep tick so a panic between
                // enqueue/dequeue can't drift the gauge. Best-effort:
                // stats failure logs and the prior tick's value is
                // retained rather than zeroing.
                match registry.build_queue_stats().await {
                    Ok(stats) => {
                        metrics::gauge!(
                            "cloacina_compiler_queue_depth",
                            "state" => "queued",
                        )
                        .set(stats.pending as f64);
                        metrics::gauge!(
                            "cloacina_compiler_queue_depth",
                            "state" => "building",
                        )
                        .set(stats.building as f64);
                    }
                    Err(e) => {
                        warn!(%e, "build_queue_stats failed; queue_depth gauge not refreshed");
                    }
                }
            }
        }
    }
}

/// CLOACI-T-0780: per-target compiler loop. Instead of claiming pending rows, it
/// scan-and-fills `package_artifacts` for `config.build_target`: success packages
/// in this schema lacking this arch's artifact are rebuilt from the retained
/// source (NATIVE build — run the container on that arch, e.g. docker
/// `platform: linux/amd64`) and stored under `sha256(cdylib)`. Additive and
/// idempotent — never touches the primary (`workflow_packages`) host build, so a
/// host compiler and this can run side by side without racing.
pub(crate) async fn run_per_target(
    registry: Arc<WorkflowRegistryImpl<UnifiedRegistryStorage>>,
    database: cloacina::database::Database,
    config: CompilerConfig,
    shutdown: CancellationToken,
) -> Result<()> {
    use sha2::{Digest, Sha256};

    let target = config
        .build_target
        .clone()
        .expect("run_per_target requires build_target");
    let name_filter = config.build_target_package.clone();
    // tenant_id is NULL in every schema (the schema IS the isolation); the DB
    // handle is already schema-scoped, so scope DAL queries by IS NULL (None).
    let dal = cloacina::dal::DAL::new(database);

    let mut poll_ticker = tokio::time::interval(config.poll_interval);
    poll_ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    info!(%target, ?name_filter, "per-target compiler: scan-and-fill package_artifacts");

    // CLOACI-T-0780: packages that FAILED to build for this target — skip them on
    // later scans so a package that can't build for this arch (e.g. too heavy to
    // compile under emulation, rustc-LLVM OOM) doesn't get retried every tick
    // forever, burning CPU/disk. It simply has no artifact for this arch and runs
    // where it does. Cleared only on process restart (a transient cause may resolve).
    let mut failed: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                info!("shutdown requested — exiting per-target loop");
                return Ok(());
            }
            _ = poll_ticker.tick() => {
                let mut missing = match dal
                    .workflow_packages()
                    .find_packages_missing_target_artifact(&target, None, name_filter.as_deref())
                    .await
                {
                    Ok(m) => m,
                    Err(e) => {
                        warn!(%e, "find_packages_missing_target_artifact failed; backing off one tick");
                        continue;
                    }
                };
                // CLOACI-T-0908: also fill missing per-arch NATIVE provider
                // bundles. `execute_build` re-bundles providers as part of the
                // rebuild (storing triple-keyed rows via config.build_target),
                // so a package that HAS its artifact but LACKS this arch's
                // native provider rows still needs one pass through the loop.
                match dal
                    .workflow_packages()
                    .find_packages_missing_target_provider(&target, None, name_filter.as_deref())
                    .await
                {
                    Ok(provider_missing) => {
                        for entry in provider_missing {
                            if !missing.iter().any(|(id, _, _)| *id == entry.0) {
                                info!(name = %entry.1, version = %entry.2, %target,
                                      "per-target: native provider bundle missing for this arch");
                                missing.push(entry);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(%e, "find_packages_missing_target_provider failed; skipping provider fill this tick");
                    }
                }
                for (package_id, name, version) in missing {
                    if failed.contains(&(name.clone(), version.clone())) {
                        continue; // already failed this run — don't retry-loop on it
                    }
                    info!(%name, %version, %target, "per-target: building artifact");
                    match crate::build::execute_build(&registry, package_id.0, &config).await {
                        crate::build::BuildOutcome::Success { artifact, .. } => {
                            // CLOACI-T-0780: an empty artifact means there is no
                            // arch-specific cdylib — an interpreted (e.g. Python)
                            // package that runs from its source on ANY agent. There's
                            // nothing to build per-arch, so skip it; storing an empty
                            // row would hand an agent an empty `.so` to load.
                            if artifact.is_empty() {
                                info!(
                                    %name, %version, %target,
                                    "per-target: no cdylib (arch-independent package) — skipping"
                                );
                            } else {
                                let mut hasher = Sha256::new();
                                hasher.update(&artifact);
                                let digest = hex::encode(hasher.finalize());
                                match dal
                                    .workflow_packages()
                                    .upsert_artifact(&name, &version, None, &target, &digest, artifact)
                                    .await
                                {
                                    Ok(()) => info!(
                                        %name, %version, %target, %digest,
                                        "per-target: stored artifact"
                                    ),
                                    Err(e) => warn!(%e, %name, "per-target: upsert_artifact failed"),
                                }
                            }
                        }
                        crate::build::BuildOutcome::Failed(err) => {
                            warn!(%name, %err, "per-target: build failed — won't retry this run");
                            failed.insert((name, version));
                        }
                        crate::build::BuildOutcome::TimedOut { elapsed } => {
                            warn!(
                                %name,
                                elapsed_s = elapsed.as_secs(),
                                "per-target: build timed out — won't retry this run"
                            );
                            failed.insert((name, version));
                        }
                    }
                }
            }
        }
    }
}
