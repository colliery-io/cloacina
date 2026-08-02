# cloacina::computation_graph::graph_executor <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


The graph-execution seam (CLOACI-T-0722).

Mirrors the task-side `TaskExecutor` trait: when a reactor fires, it hands
the firing to a [`GraphExecutor`] instead of calling the compiled graph
closure directly. The default [`InProcessGraphExecutor`] preserves today's
behavior exactly; `cloacina-server`'s fleet executor ships the firing (the
`InputCache` snapshot + the CG package digest) to an execution agent and
awaits the result — accumulators and reactor state stay host-side, only
the compute leaves.
Every [`GraphFireEvent`] carries the compiled in-process closure, so a
fleet executor can ALWAYS fall back to local execution (no agent capacity,
unresolvable package, dispatch timeout) rather than wedging the reactor's
hot path.

## Structs

### `cloacina::computation_graph::graph_executor::GraphFireEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


One reactor firing, ready to execute somewhere.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `graph_name` | `String` | The graph (== reactor) name — the fleet executor resolves the owning
package (and artifact digest) from it at fire time. |
| `tenant_id` | `Option < String >` | Tenant scope for agent selection; `None` for untagged graphs. |
| `snapshot` | `InputCache` | The input snapshot the graph consumes. |
| `in_process` | `CompiledGraphFn` | The compiled in-process closure — the default execution AND the
universal fallback for remote executors. |



### `cloacina::computation_graph::graph_executor::InProcessGraphExecutor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Default`

Default executor: run the compiled graph closure in this process — byte-for-byte the pre-seam behavior.



## Functions

### `cloacina::computation_graph::graph_executor::in_process_graph_executor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn in_process_graph_executor () -> Arc < dyn GraphExecutor >
```

The shared default used when nothing is injected.

<details>
<summary>Source</summary>

```rust
pub fn in_process_graph_executor() -> Arc<dyn GraphExecutor> {
    Arc::new(InProcessGraphExecutor)
}
```

</details>
