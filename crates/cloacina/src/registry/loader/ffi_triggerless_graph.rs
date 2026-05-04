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

//! Host-side adapter that dispatches a trigger-less computation graph
//! through fidius FFI into a packaged cdylib.
//!
//! Same pattern as `ffi_trigger.rs` but for graphs: independently-
//! compiled cdylibs don't reach the host's `inventory::iter`, so the
//! reconciler can't directly install the cdylib's
//! `TriggerlessGraphFn` into the runtime. Instead, it builds a
//! host-side `TriggerlessGraphRegistration` whose `graph_fn` hands the
//! invocation back across the FFI boundary via method index 8
//! (`invoke_triggerless_graph`).

use cloacina_computation_graph::{GraphError, GraphResult};
use cloacina_workflow::Context;
use cloacina_workflow_plugin::{TriggerlessGraphInvokeRequest, TriggerlessGraphInvokeResult};
use std::sync::Arc;

/// Method index of `invoke_triggerless_graph` on `CloacinaPlugin`.
const INVOKE_TRIGGERLESS_GRAPH_METHOD_INDEX: usize = 8;

/// Build a `TriggerlessGraphFn` that dispatches the named graph
/// through the cdylib at every invocation. The returned closure
/// captures an `Arc<PluginHandle>` (so the dlopen stays alive) plus
/// the graph name + terminal-output names. Each invocation:
///
/// 1. Serializes the workflow context to JSON.
/// 2. Bounces through `tokio::task::spawn_blocking` (fidius is sync).
/// 3. Calls `handle.call_method(8, ...)` with a
///    `TriggerlessGraphInvokeRequest`.
/// 4. Reconstructs `GraphResult` from the wire result. Terminal
///    outputs are returned as boxed `serde_json::Value` so the
///    workflow-task post-invocation logic in `cloacina-macros` can
///    `downcast_ref::<serde_json::Value>` them, identical to the
///    in-process path.
pub fn build_ffi_triggerless_graph_fn(
    handle: Arc<fidius_host::PluginHandle>,
    graph_name: String,
    terminal_count: usize,
) -> cloacina_workflow_plugin::TriggerlessGraphFn {
    Arc::new(move |ctx: Context<serde_json::Value>| {
        let handle = handle.clone();
        let graph_name = graph_name.clone();
        let terminal_count = terminal_count;
        Box::pin(async move {
            let context_json = match ctx.to_json() {
                Ok(s) => s,
                Err(e) => {
                    return GraphResult::error(GraphError::Serialization(format!(
                        "context serialize failed: {}",
                        e
                    )));
                }
            };
            let request = TriggerlessGraphInvokeRequest {
                graph_name: graph_name.clone(),
                context_json,
            };
            let call_result: Result<TriggerlessGraphInvokeResult, fidius_host::CallError> =
                tokio::task::spawn_blocking(move || {
                    handle.call_method(INVOKE_TRIGGERLESS_GRAPH_METHOD_INDEX, &request)
                })
                .await
                .unwrap_or_else(|e| {
                    Err(fidius_host::CallError::Serialization(format!(
                        "spawn_blocking join failed: {}",
                        e
                    )))
                });

            let r = match call_result {
                Ok(r) => r,
                Err(e) => {
                    return GraphResult::error(GraphError::Execution(format!(
                        "FFI invoke_triggerless_graph for '{}' failed: {:?}",
                        graph_name, e
                    )));
                }
            };

            if let Some(err) = r.error {
                return GraphResult::error(GraphError::Execution(err));
            }

            if !r.success {
                return GraphResult::error(GraphError::Execution(format!(
                    "Graph '{}' returned !success without an error message",
                    graph_name
                )));
            }

            // Decode terminal outputs into boxed serde_json::Value so
            // the workflow-task post-invocation downcast in
            // cloacina-macros sees the same shape as the in-process
            // path. Pad to terminal_count with Null when the cdylib
            // returned fewer entries (defensive — the cdylib already
            // pads, but it's cheap insurance).
            let parsed: Vec<serde_json::Value> = match r.terminal_outputs_json {
                Some(json) => match serde_json::from_str::<Vec<serde_json::Value>>(&json) {
                    Ok(v) => v,
                    Err(e) => {
                        return GraphResult::error(GraphError::Serialization(format!(
                            "Failed to parse terminal outputs: {}",
                            e
                        )));
                    }
                },
                None => Vec::new(),
            };
            let mut outputs: Vec<Box<dyn std::any::Any + Send>> =
                Vec::with_capacity(terminal_count.max(parsed.len()));
            for v in parsed.into_iter() {
                outputs.push(Box::new(v) as Box<dyn std::any::Any + Send>);
            }
            while outputs.len() < terminal_count {
                outputs.push(Box::new(serde_json::Value::Null) as Box<dyn std::any::Any + Send>);
            }
            GraphResult::completed(outputs)
        })
    })
}
