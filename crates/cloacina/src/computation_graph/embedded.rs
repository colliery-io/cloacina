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

//! Embedded computation-graph runtime builder (CLOACI-T-0738).
//!
//! Replaces the ~60-line hand-wired `main()` block (four channels, an
//! `AccumulatorContext` full of `None`s, a `CompiledGraphFn` closure, an
//! unused `manual_rx`, two `tokio::spawn`s) that embedded CG examples used to
//! copy-paste. The production scheduler already does all of that wiring in
//! `load_graph`; this is the embedded-friendly face of the same machinery:
//!
//! ```ignore
//! let graph = EmbeddedGraph::spawn(my_graph_declaration()).await?;
//! graph.push("prices", &serde_json::json!({"symbol": "X", "px": 42.0})).await?;
//! // ... later
//! graph.shutdown().await;
//! ```
//!
//! Manual wiring still works — this is additive.

use serde::Serialize;

use super::registry::EndpointRegistry;
use super::scheduler::{ComputationGraphDeclaration, ComputationGraphScheduler};

/// A running embedded computation graph: accumulators spawned, reactor live,
/// events pushed via [`push`](Self::push). Dropping the value does NOT stop
/// the graph — call [`shutdown`](Self::shutdown).
pub struct EmbeddedGraph {
    scheduler: ComputationGraphScheduler,
    registry: EndpointRegistry,
    graph_name: String,
}

impl EmbeddedGraph {
    /// Wire and spawn `decl` (accumulators + reactor + compiled graph fn) —
    /// the whole block embedded examples used to hand-write.
    pub async fn spawn(decl: ComputationGraphDeclaration) -> Result<Self, String> {
        let registry = EndpointRegistry::new();
        let scheduler = ComputationGraphScheduler::new(registry.clone());
        let graph_name = decl.name.clone();
        scheduler.load_graph(decl).await?;
        Ok(Self {
            scheduler,
            registry,
            graph_name,
        })
    }

    /// Push a JSON-serializable event into an accumulator by name (the same
    /// raw-JSON socket contract the server's WS/REST injection uses).
    pub async fn push(&self, accumulator: &str, event: &impl Serialize) -> Result<(), String> {
        let bytes = serde_json::to_vec(event).map_err(|e| e.to_string())?;
        self.push_raw(accumulator, bytes).await
    }

    /// Push pre-encoded raw event bytes into an accumulator by name.
    pub async fn push_raw(&self, accumulator: &str, bytes: Vec<u8>) -> Result<(), String> {
        self.registry
            .send_to_accumulator(accumulator, bytes)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// The graph's name (== reactor name for self-reactor declarations).
    pub fn graph_name(&self) -> &str {
        &self.graph_name
    }

    /// Escape hatch: the underlying scheduler, for anything the lean surface
    /// doesn't cover (manual force-fire, health, additional graphs).
    pub fn scheduler(&self) -> &ComputationGraphScheduler {
        &self.scheduler
    }

    /// Escape hatch: the endpoint registry (reactor handles, health).
    pub fn registry(&self) -> &EndpointRegistry {
        &self.registry
    }

    /// Stop the reactor and accumulators.
    pub async fn shutdown(self) {
        self.scheduler.shutdown_all().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computation_graph::packaging_bridge::PassthroughAccumulatorFactory;
    use crate::computation_graph::reactor::{InputStrategy, ReactionCriteria};
    use crate::computation_graph::scheduler::{AccumulatorDeclaration, ReactorDeclaration};
    use cloacina_computation_graph::{CompiledGraphFn, GraphResult, InputCache};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// CLOACI-T-0738 regression guard: the minimal embedded author — one
    /// declaration + EmbeddedGraph::spawn + push — with NO hand-wired
    /// channels/contexts/spawns. Asserts the reactor actually fires on the
    /// pushed event (accumulators advance end-to-end).
    #[tokio::test]
    async fn minimal_embedded_author_fires() {
        let fires = Arc::new(AtomicU32::new(0));
        let fires_in_graph = fires.clone();
        let graph_fn: CompiledGraphFn = Arc::new(move |_cache: InputCache| {
            let fires = fires_in_graph.clone();
            Box::pin(async move {
                fires.fetch_add(1, Ordering::SeqCst);
                GraphResult::completed(vec![])
            })
        });

        let decl = ComputationGraphDeclaration {
            name: "embedded_min".to_string(),
            accumulators: vec![AccumulatorDeclaration {
                name: "events".to_string(),
                factory: Arc::new(PassthroughAccumulatorFactory),
            }],
            reactor: ReactorDeclaration {
                criteria: ReactionCriteria::WhenAny,
                strategy: InputStrategy::Latest,
                graph_fn,
                constructor: None,
            },
            tenant_id: None,
            reactor_name: None,
            topology: None,
        };

        let graph = EmbeddedGraph::spawn(decl).await.expect("spawn");
        graph
            .push("events", &serde_json::json!({"value": 42.0}))
            .await
            .expect("push");

        // The fire is async — poll briefly.
        for _ in 0..50 {
            if fires.load(Ordering::SeqCst) > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(
            fires.load(Ordering::SeqCst) > 0,
            "pushed event must fire the reactor through the embedded builder"
        );
        graph.shutdown().await;
    }
}
