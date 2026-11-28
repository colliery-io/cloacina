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

//! A graph firing the fleet cannot take fails with its reason and does not
//! run in the server process (CLOACINA-T-1076). Needs Postgres, like the
//! other server tests.

use super::*;

use cloacina::cloacina_computation_graph::{GraphResult, InputCache};
use cloacina::computation_graph::graph_executor::{GraphExecutor, GraphFireEvent};
use std::sync::atomic::{AtomicBool, Ordering};

fn fleet_graph_executor(state: &AppState) -> crate::fleet_graph_executor::FleetGraphExecutor {
    let dal = cloacina::dal::DAL::new(state.database.clone());
    let storage = cloacina::dal::unified::workflow_registry_storage::UnifiedRegistryStorage::new(
        state.database.clone(),
    );
    let registry = cloacina::registry::workflow_registry::WorkflowRegistryImpl::new(
        storage,
        state.database.clone(),
    )
    .expect("registry");
    crate::fleet_graph_executor::FleetGraphExecutor::new(
        dal.clone(),
        dal,
        state.agent_registry.clone(),
        state.fleet_coordinator.clone(),
        state.delivery_wake.clone(),
        registry,
    )
}

/// A firing whose closure marks that it ran in this process.
fn marking_firing(graph_name: &str, ran: Arc<AtomicBool>) -> GraphFireEvent {
    GraphFireEvent {
        graph_name: graph_name.to_string(),
        tenant_id: Some("public".to_string()),
        snapshot: InputCache::new(),
        in_process: Arc::new(move |_cache| {
            let ran = ran.clone();
            Box::pin(async move {
                ran.store(true, Ordering::SeqCst);
                GraphResult::completed(Vec::new())
            })
        }),
    }
}

/// No package owns the graph: the firing fails with that reason, and the
/// closure that would run it in the server is not called.
#[tokio::test]
#[serial]
async fn a_firing_with_no_owning_package_fails_and_does_not_run_in_the_server() {
    let state = test_state().await;
    let executor = fleet_graph_executor(&state);
    let ran = Arc::new(AtomicBool::new(false));
    let graph = format!("t1076_unowned_{}", uuid::Uuid::new_v4().simple());

    let result = executor.execute(marking_firing(&graph, ran.clone())).await;

    assert!(
        !ran.load(Ordering::SeqCst),
        "the server ran the graph firing in its own process"
    );
    match result {
        GraphResult::Error(e) => {
            let text = e.to_string();
            assert!(text.contains("no owning package"), "reason: {text}");
            assert!(text.contains(&graph), "reason: {text}");
            assert!(
                text.contains("not run in the server process"),
                "reason: {text}"
            );
        }
        GraphResult::Completed { .. } => panic!("the firing completed; it must fail"),
    }
}
