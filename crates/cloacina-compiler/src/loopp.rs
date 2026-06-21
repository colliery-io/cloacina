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
                .mark_build_success_with_docs(package_id, artifact, task_docs, declared_params)
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
