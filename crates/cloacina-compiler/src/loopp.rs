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

//! Main build loop. This task is T-0520's stub — it claims rows and immediately
//! marks them successful with an empty `compiled_data`. T-0521 replaces the body
//! with real cargo invocation; T-0522 layers heartbeats + a sweeper on top.

use anyhow::Result;
use cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage;
use cloacina::dal::DAL;
use cloacina::database::Database;
use cloacina::registry::workflow_registry::WorkflowRegistryImpl;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::CompilerConfig;

pub(crate) async fn run(config: CompilerConfig, shutdown: CancellationToken) -> Result<()> {
    let database = Database::new(&config.database_url, "", 5);
    database
        .run_migrations()
        .await
        .map_err(|e| anyhow::anyhow!("migration failed: {e}"))?;

    let storage = UnifiedRegistryStorage::new(database.clone());
    let registry = WorkflowRegistryImpl::new(storage, database.clone())
        .map_err(|e| anyhow::anyhow!("failed to construct workflow registry: {e}"))?;

    let _dal = DAL::new(database);

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
                        // STUB (T-0520): immediately mark success with empty
                        // cdylib bytes. T-0521 replaces with real cargo build.
                        info!(
                            package_id = %claim.id,
                            package_name = %claim.package_name,
                            version = %claim.version,
                            "claimed build (stub: marking success with empty artifact)"
                        );
                        if let Err(e) = registry.mark_build_success(claim.id, Vec::new()).await {
                            warn!(%e, "stub mark_build_success failed");
                        }
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
                    Ok(n) => info!(reset = n, "swept stale builds"),
                    Err(e) => warn!(%e, "sweep_stale_builds failed"),
                }
            }
        }
    }
}
