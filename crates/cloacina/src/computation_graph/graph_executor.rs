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

//! The graph-execution seam (CLOACI-T-0722).
//!
//! Mirrors the task-side `TaskExecutor` trait: when a reactor fires, it hands
//! the firing to a [`GraphExecutor`] instead of calling the compiled graph
//! closure directly. The default [`InProcessGraphExecutor`] preserves today's
//! behavior exactly; `cloacina-server`'s fleet executor ships the firing (the
//! `InputCache` snapshot + the CG package digest) to an execution agent and
//! awaits the result — accumulators and reactor state stay host-side, only
//! the compute leaves.
//!
//! Every [`GraphFireEvent`] carries the compiled in-process closure, so a
//! fleet executor can ALWAYS fall back to local execution (no agent capacity,
//! unresolvable package, dispatch timeout) rather than wedging the reactor's
//! hot path.

use std::sync::Arc;

use cloacina_computation_graph::{CompiledGraphFn, GraphResult, InputCache};

/// One reactor firing, ready to execute somewhere.
pub struct GraphFireEvent {
    /// The graph (== reactor) name — the fleet executor resolves the owning
    /// package (and artifact digest) from it at fire time.
    pub graph_name: String,
    /// Tenant scope for agent selection; `None` for untagged graphs.
    pub tenant_id: Option<String>,
    /// The input snapshot the graph consumes.
    pub snapshot: InputCache,
    /// The compiled in-process closure — the default execution AND the
    /// universal fallback for remote executors.
    pub in_process: CompiledGraphFn,
}

/// Where a reactor firing runs. Implementations must be cheap to call per
/// fire; the reactor awaits the returned result on its hot path.
#[async_trait::async_trait]
pub trait GraphExecutor: Send + Sync {
    async fn execute(&self, fire: GraphFireEvent) -> GraphResult;
}

/// Default executor: run the compiled graph closure in this process —
/// byte-for-byte the pre-seam behavior.
#[derive(Default)]
pub struct InProcessGraphExecutor;

#[async_trait::async_trait]
impl GraphExecutor for InProcessGraphExecutor {
    async fn execute(&self, fire: GraphFireEvent) -> GraphResult {
        (fire.in_process)(fire.snapshot).await
    }
}

/// The shared default used when nothing is injected.
pub fn in_process_graph_executor() -> Arc<dyn GraphExecutor> {
    Arc::new(InProcessGraphExecutor)
}
