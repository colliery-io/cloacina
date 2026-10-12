---
title: "cloacina::executor::host_impl"
description: "The engine's implementation of the `CloacinaHost` callback interface (CLOACI-T-0897) — what a packaged task's `defer_until` actually drives."
sidebar:
  order: 137
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


The engine's implementation of the `CloacinaHost` callback interface (CLOACI-T-0897) — what a packaged task's `defer_until` actually drives.

## Structs

### <span id="struct.EngineHost"></span>`struct EngineHost`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Services `defer_until` callbacks from packaged tasks.

Stateless: everything it needs is looked up per call from the deferral
registry, keyed by the task-execution id the plugin sends. That keeps one
bound instance correct for every tenant and every concurrently running task.

#### Methods

##### <span id="EngineHost.parse_id"></span>`parse_id`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn parse_id (raw : & str) -> Result < UniversalUuid , PluginError >
```

<details>
<summary>Source</summary>

```rust
    fn parse_id(raw: &str) -> Result<UniversalUuid, PluginError> {
        uuid::Uuid::from_str(raw)
            .map(UniversalUuid::from)
            .map_err(|e| PluginError::new("BAD_TASK_ID", format!("malformed task id {raw:?}: {e}")))
    }
```

</details>



##### <span id="EngineHost.entry"></span>`entry`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn entry (raw : & str) -> Result < deferral_registry :: DeferralEntry , PluginError >
```

<details>
<summary>Source</summary>

```rust
    fn entry(raw: &str) -> Result<deferral_registry::DeferralEntry, PluginError> {
        let id = Self::parse_id(raw)?;
        deferral_registry::lookup(&id).ok_or_else(|| {
            // Not a panic and not a silent success: a callback for a task that
            // has already finished (or was never registered) is a real error
            // the task should see.
            PluginError::new(
                "TASK_NOT_RUNNING",
                format!("no running task registered for {raw} — it may have already completed"),
            )
        })
    }
```

</details>





