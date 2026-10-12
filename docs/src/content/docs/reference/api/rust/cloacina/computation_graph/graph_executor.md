---
title: "cloacina::computation_graph::graph_executor"
description: "The graph-execution seam (CLOACI-T-0722)."
sidebar:
  order: 59
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


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

### <span id="struct.GraphFireEvent"></span>`struct GraphFireEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



### <span id="struct.InProcessGraphExecutor"></span>`struct InProcessGraphExecutor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Default`

Default executor: run the compiled graph closure in this process — byte-for-byte the pre-seam behavior.



## Functions

### <span id="fn.in_process_graph_executor"></span>`fn in_process_graph_executor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



