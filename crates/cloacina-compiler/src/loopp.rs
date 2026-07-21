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
        } => {
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
                .mark_build_success_with_docs(
                    package_id,
                    artifact,
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
                                let digest = format!("{:x}", hasher.finalize());
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
