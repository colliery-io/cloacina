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

//! Bridge from FFI-loaded package metadata to ReactiveScheduler types.
//!
//! Converts `GraphPackageMetadata` + library data into `ComputationGraphDeclaration`
//! with `AccumulatorFactory` implementations and a `CompiledGraphFn` that calls
//! `execute_graph()` via fidius FFI.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use cloacina_workflow_plugin::{
    AccumulatorDeclarationEntry, GraphExecutionRequest, GraphPackageMetadata,
};

use super::accumulator::{
    accumulator_runtime, AccumulatorContext, AccumulatorRuntimeConfig, BoundarySender,
};
use super::reactor::{CompiledGraphFn, InputStrategy, ReactionCriteria};
use super::scheduler::{
    AccumulatorDeclaration, AccumulatorFactory, AccumulatorSpawnConfig,
    ComputationGraphDeclaration, ReactorDeclaration,
};
use super::types::{GraphError, GraphResult, InputCache, SourceName};

/// Convert FFI graph metadata + library data into a `ComputationGraphDeclaration`
/// that the `ReactiveScheduler` can load.
pub fn build_declaration_from_ffi(
    graph_meta: &GraphPackageMetadata,
    library_data: Vec<u8>,
) -> ComputationGraphDeclaration {
    let criteria = match graph_meta.reaction_mode.as_str() {
        "when_all" => ReactionCriteria::WhenAll,
        _ => ReactionCriteria::WhenAny,
    };

    let strategy = match graph_meta.input_strategy.as_str() {
        "sequential" => InputStrategy::Sequential,
        _ => InputStrategy::Latest,
    };

    // Create the CompiledGraphFn that calls execute_graph() via FFI
    let lib_data = Arc::new(library_data);
    let graph_fn: CompiledGraphFn = {
        let lib_data = lib_data.clone();
        Arc::new(move |cache: InputCache| {
            let lib_data = lib_data.clone();
            Box::pin(async move { execute_graph_via_ffi(&lib_data, &cache).await })
        })
    };

    // Create accumulator factories from FFI metadata
    let accumulators = graph_meta
        .accumulators
        .iter()
        .map(|acc_entry| AccumulatorDeclaration {
            name: acc_entry.name.clone(),
            factory: Arc::new(PassthroughAccumulatorFactory),
        })
        .collect();

    ComputationGraphDeclaration {
        name: graph_meta.graph_name.clone(),
        accumulators,
        reactor: ReactorDeclaration {
            criteria,
            strategy,
            graph_fn,
        },
        tenant_id: None, // Set by the reconciler based on package ownership
    }
}

/// Execute a computation graph via FFI by loading the library and calling
/// `execute_graph()` (method index 3).
async fn execute_graph_via_ffi(library_data: &[u8], cache: &InputCache) -> GraphResult {
    let lib_data = library_data.to_vec();
    let cache_snapshot = cache.snapshot();

    // Serialize cache entries to JSON for the FFI boundary
    let mut ffi_cache: HashMap<String, String> = HashMap::new();
    for source_name in cache_snapshot.sources() {
        if let Some(raw_bytes) = cache_snapshot.get_raw(source_name.as_str()) {
            // The cache stores bytes in debug-JSON or release-bincode format.
            // For the FFI boundary, we need JSON strings.
            // In debug mode, raw_bytes IS JSON. In release, it's bincode.
            #[cfg(debug_assertions)]
            {
                let json_str = String::from_utf8_lossy(raw_bytes).to_string();
                ffi_cache.insert(source_name.as_str().to_string(), json_str);
            }
            #[cfg(not(debug_assertions))]
            {
                // Deserialize from bincode to serde_json::Value, then to JSON string
                match bincode::deserialize::<serde_json::Value>(raw_bytes) {
                    Ok(val) => {
                        let json_str = serde_json::to_string(&val).unwrap_or_default();
                        ffi_cache.insert(source_name.as_str().to_string(), json_str);
                    }
                    Err(e) => {
                        return GraphResult::error(GraphError::Serialization(format!(
                            "Failed to deserialize cache entry '{}' for FFI: {}",
                            source_name.as_str(),
                            e
                        )));
                    }
                }
            }
        }
    }

    let request = GraphExecutionRequest { cache: ffi_cache };

    // Call the FFI method in a blocking task (library loading is synchronous)
    let result =
        tokio::task::spawn_blocking(move || call_execute_graph_ffi(&lib_data, request)).await;

    match result {
        Ok(Ok(ffi_result)) => {
            if ffi_result.success {
                // Convert terminal outputs from JSON strings to boxed Any values
                let outputs: Vec<Box<dyn std::any::Any + Send>> =
                    if let Some(json_outputs) = ffi_result.terminal_outputs_json {
                        json_outputs
                            .into_iter()
                            .filter_map(|json_str| {
                                serde_json::from_str::<serde_json::Value>(&json_str)
                                    .ok()
                                    .map(|v| Box::new(v) as Box<dyn std::any::Any + Send>)
                            })
                            .collect()
                    } else {
                        vec![]
                    };
                GraphResult::completed(outputs)
            } else {
                let error_msg = ffi_result
                    .error
                    .unwrap_or_else(|| "unknown FFI execution error".to_string());
                GraphResult::error(GraphError::NodeExecution(error_msg))
            }
        }
        Ok(Err(e)) => GraphResult::error(GraphError::NodeExecution(format!(
            "FFI execute_graph call failed: {}",
            e
        ))),
        Err(join_err) => GraphResult::error(GraphError::NodeExecution(format!(
            "FFI execute_graph panicked: {}",
            join_err
        ))),
    }
}

/// Load the library and call execute_graph (method index 3) synchronously.
fn call_execute_graph_ffi(
    library_data: &[u8],
    request: GraphExecutionRequest,
) -> Result<cloacina_workflow_plugin::GraphExecutionResult, String> {
    let temp_dir =
        tempfile::TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let library_extension = if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    };

    let temp_path = temp_dir
        .path()
        .join(format!("graph_exec.{}", library_extension));
    std::fs::write(&temp_path, library_data)
        .map_err(|e| format!("Failed to write library: {}", e))?;

    let loaded = fidius_host::loader::load_library(&temp_path)
        .map_err(|e| format!("Failed to load library: {}", e))?;

    let plugin = loaded
        .plugins
        .into_iter()
        .next()
        .ok_or_else(|| "No plugins in library".to_string())?;

    let handle = fidius_host::PluginHandle::from_loaded(plugin);

    // Method index 3 = execute_graph
    handle
        .call_method(3, &(request,))
        .map_err(|e| format!("execute_graph FFI call failed: {}", e))
}

/// A generic passthrough accumulator factory for FFI-loaded packages.
///
/// All packaged accumulators are passthrough at the host level — they receive
/// serialized events via WebSocket/socket and forward them to the reactor.
/// The actual processing logic lives inside the FFI plugin's `execute_graph()`.
struct PassthroughAccumulatorFactory;

struct GenericPassthroughAccumulator;

#[async_trait::async_trait]
impl super::Accumulator for GenericPassthroughAccumulator {
    type Event = serde_json::Value;
    type Output = serde_json::Value;

    fn process(&mut self, event: serde_json::Value) -> Option<serde_json::Value> {
        Some(event)
    }
}

impl AccumulatorFactory for PassthroughAccumulatorFactory {
    fn spawn(
        &self,
        name: String,
        boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
        shutdown_rx: watch::Receiver<bool>,
        config: AccumulatorSpawnConfig,
    ) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>) {
        let (socket_tx, socket_rx) = mpsc::channel(64);

        let checkpoint = config.dal.map(|dal| {
            super::accumulator::CheckpointHandle::new(dal, config.graph_name.clone(), name.clone())
        });

        let sender = BoundarySender::new(boundary_tx, SourceName::new(&name));
        let ctx = AccumulatorContext {
            output: sender,
            name: name.clone(),
            shutdown: shutdown_rx,
            checkpoint,
            health: config.health_tx,
        };

        let handle = tokio::spawn(accumulator_runtime(
            GenericPassthroughAccumulator,
            ctx,
            socket_rx,
            AccumulatorRuntimeConfig::default(),
        ));

        (socket_tx, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_declaration_from_ffi_metadata() {
        let meta = GraphPackageMetadata {
            graph_name: "test_graph".to_string(),
            package_name: "test-pkg".to_string(),
            reaction_mode: "when_any".to_string(),
            input_strategy: "latest".to_string(),
            accumulators: vec![
                AccumulatorDeclarationEntry {
                    name: "alpha".to_string(),
                    accumulator_type: "passthrough".to_string(),
                    config: HashMap::new(),
                },
                AccumulatorDeclarationEntry {
                    name: "beta".to_string(),
                    accumulator_type: "stream".to_string(),
                    config: [("topic".to_string(), "test.topic".to_string())]
                        .into_iter()
                        .collect(),
                },
            ],
        };

        let decl = build_declaration_from_ffi(&meta, vec![0u8; 100]);

        assert_eq!(decl.name, "test_graph");
        assert_eq!(decl.accumulators.len(), 2);
        assert_eq!(decl.accumulators[0].name, "alpha");
        assert_eq!(decl.accumulators[1].name, "beta");
    }

    #[test]
    fn test_reaction_mode_parsing() {
        let meta_any = GraphPackageMetadata {
            graph_name: "g".to_string(),
            package_name: "p".to_string(),
            reaction_mode: "when_any".to_string(),
            input_strategy: "latest".to_string(),
            accumulators: vec![],
        };
        let decl_any = build_declaration_from_ffi(&meta_any, vec![]);
        assert!(matches!(
            decl_any.reactor.criteria,
            ReactionCriteria::WhenAny
        ));

        let meta_all = GraphPackageMetadata {
            graph_name: "g".to_string(),
            package_name: "p".to_string(),
            reaction_mode: "when_all".to_string(),
            input_strategy: "sequential".to_string(),
            accumulators: vec![],
        };
        let decl_all = build_declaration_from_ffi(&meta_all, vec![]);
        assert!(matches!(
            decl_all.reactor.criteria,
            ReactionCriteria::WhenAll
        ));
        assert!(matches!(
            decl_all.reactor.strategy,
            InputStrategy::Sequential
        ));
    }
}
