# cloacina::registry::loader::ffi_triggerless_graph <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Host-side adapter that dispatches a trigger-less computation graph through fidius FFI into a packaged cdylib.

Same pattern as `ffi_trigger.rs` but for graphs: independently-
compiled cdylibs don't reach the host's `inventory::iter`, so the
reconciler can't directly install the cdylib's
`TriggerlessGraphFn` into the runtime. Instead, it builds a
host-side `TriggerlessGraphRegistration` whose `graph_fn` hands the
invocation back across the FFI boundary via method index 8
(`invoke_triggerless_graph`).

## Functions

### `cloacina::registry::loader::ffi_triggerless_graph::build_ffi_triggerless_graph_fn`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn build_ffi_triggerless_graph_fn (handle : Arc < fidius_host :: PluginHandle > , graph_name : String , terminal_count : usize ,) -> cloacina_workflow_plugin :: TriggerlessGraphFn
```

Build a `TriggerlessGraphFn` that dispatches the named graph through the cdylib at every invocation. The returned closure captures an `Arc<PluginHandle>` (so the dlopen stays alive) plus the graph name + terminal-output names. Each invocation:

1. Serializes the workflow context to JSON.
2. Bounces through `tokio::task::spawn_blocking` (fidius is sync).
3. Calls `handle.call_method(8, ...)` with a
`TriggerlessGraphInvokeRequest`.
4. Reconstructs `GraphResult` from the wire result. Terminal
outputs are returned as boxed `serde_json::Value` so the
workflow-task post-invocation logic in `cloacina-macros` can
`downcast_ref::<serde_json::Value>` them, identical to the
in-process path.

<details>
<summary>Source</summary>

```rust
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
                    handle.call_method(METHOD_INVOKE_TRIGGERLESS_GRAPH, &request)
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
```

</details>
