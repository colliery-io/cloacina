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

//! # Sync (COLLIERY-I-0632, CLOACINA-A-0013)
//!
//! Move records from a **source** to a **destination** through sandboxed
//! connectors, one **segment** per workflow execution, with a **cursor** that
//! advances only after the durable write.
//!
//! This crate holds the built-in [`workflow`] with its three tasks (read a
//! segment, map and enforce, write), the [`mapping`] engine, and the
//! [`params`] a sync definition binds. It is linked into `cloacina-server` and
//! `cloacina-agent` so the workflow is seeded from inventory on both; an
//! embedded runner gets it by depending on this crate and calling
//! [`ensure_linked`] once.
//!
//! The [`dispatcher`] loops over the segments of a pass (CLOACINA-T-0977). A
//! sync definition is a workflow instance of the Sync workflow; [`install`]
//! puts the [`hook`] on a runner so a fire of that instance, on its cron or by
//! hand, runs a pass instead of one execution (CLOACINA-T-0978).

// The `#[workflow]` macro emits `cfg(feature = "packaged")`, a feature this
// crate does not declare.
#![allow(unexpected_cfgs)]

pub mod corpus;
pub mod credential;
pub mod definition;
pub mod dispatcher;
pub mod fault;
pub mod hook;
pub mod importer;
pub mod mapping;
pub mod params;
pub mod workflow;

// The MSSQL and Snowflake connectors' statements are pure, but the connectors
// are wasm guests no lane builds natively: their unit tests run here
// (CLOACINA-T-0984, CLOACINA-T-0986).
#[cfg(test)]
#[path = "../../../connectors/mssql/src/sql.rs"]
mod mssql_sql;
#[cfg(test)]
#[path = "../../../connectors/snowflake/src/sql.rs"]
mod snowflake_sql;

/// The name of the built-in Sync workflow, as a sync definition's instance
/// names it and as the dispatcher executes it.
pub const WORKFLOW_NAME: &str = "sync";

/// Keep this crate linked. The workflow registers itself through `inventory`
/// entries in this crate's object file; a binary that depends on the crate
/// but never names a symbol from it may have the linker drop the object and
/// the entries with it. Call this once at startup from any binary that must
/// run syncs. It does nothing else.
pub fn ensure_linked() {
    std::hint::black_box(WORKFLOW_NAME);
}

/// Install Sync on a runner: the instance fire hook that turns a fire of the
/// Sync workflow into a pass. `ownership` and `tenant` are for a server with
/// replicas; an embedded runner passes `None`.
pub fn install(
    runner: &cloacina::runner::DefaultRunner,
    ownership: Option<
        std::sync::Arc<dyn cloacina::computation_graph::reactor_ownership::ReactorOwnership>,
    >,
    tenant: Option<String>,
) -> std::sync::Arc<hook::SyncFireHook> {
    ensure_linked();
    let hook = std::sync::Arc::new(hook::SyncFireHook::new(ownership, tenant));
    hook.set_runner(runner.clone());
    runner.set_instance_fire_hook(hook.clone());
    hook
}

/// Run one pass of the sync definition `instance` by hand and wait for it, on
/// an embedded runner or anywhere a runner is at hand. The instance's params
/// come from its schedule row.
pub async fn run_pass(
    runner: &cloacina::runner::DefaultRunner,
    instance: &str,
    stop: tokio::sync::watch::Receiver<bool>,
) -> Result<dispatcher::PassOutcome, dispatcher::DispatchError> {
    use dispatcher::DispatchError;
    let dal = std::sync::Arc::new(cloacina::dal::DAL::new(runner.database().clone()));
    let schedule = dal
        .schedule()
        .find_by_instance_name(WORKFLOW_NAME, instance)
        .await
        .map_err(|e| DispatchError::Database(e.to_string()))?
        .ok_or_else(|| DispatchError::Params(format!("no sync definition named `{instance}`")))?;
    let mut context = cloacina::Context::new();
    if let Some(params) = schedule.params.as_deref() {
        cloacina::workflow_instance::merge_instance_params(&mut context, params)
            .map_err(|e| DispatchError::Params(e.to_string()))?;
    }
    let def = dispatcher::SyncDefinition::new(context.data().clone().into_iter().collect());
    let executor: std::sync::Arc<dyn cloacina::executor::WorkflowExecutor> =
        std::sync::Arc::new(runner.clone());
    dispatcher::Dispatcher::new(executor, dal)
        .run_pass(&def, stop)
        .await
}
