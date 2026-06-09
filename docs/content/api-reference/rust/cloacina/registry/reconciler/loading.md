# cloacina::registry::reconciler::loading <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Package loading, unloading, and task/workflow registration.

## Structs

### `cloacina::registry::reconciler::loading::PackageLoadView`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


T-0554 — Unified package metadata view fed into the precedence pipeline. Wire-format types from `cloacina-workflow-plugin`. Both the Rust FFI extraction path and (future) Python scoped-Runtime adapter produce values of this shape.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `triggers` | `Vec < cloacina_workflow_plugin :: TriggerPackageMetadata >` |  |
| `reactors` | `Vec < cloacina_workflow_plugin :: ReactorPackageMetadata >` |  |
| `graph` | `Option < cloacina_workflow_plugin :: GraphPackageMetadata >` |  |



## Functions

### `cloacina::registry::reconciler::loading::parse_humantime_duration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn parse_humantime_duration (s : & str) -> Option < std :: time :: Duration >
```

Best-effort humantime parser for trigger metadata's poll_interval strings (e.g. "5s", "500ms", "1m"). Falls back to `None` for unparsable values; callers default to a safe constant. Used by `step_load_custom_triggers` when registering FFI trigger adapters from packaged cdylibs (the cdylib serializes the duration as a string in `TriggerPackageMetadata`).

<details>
<summary>Source</summary>

```rust
fn parse_humantime_duration(s: &str) -> Option<std::time::Duration> {
    let trimmed = s.trim();
    if let Some(num) = trimmed.strip_suffix("ms") {
        num.trim()
            .parse::<u64>()
            .ok()
            .map(std::time::Duration::from_millis)
    } else if let Some(num) = trimmed.strip_suffix('s') {
        num.trim()
            .parse::<u64>()
            .ok()
            .map(std::time::Duration::from_secs)
    } else if let Some(num) = trimmed.strip_suffix('m') {
        num.trim()
            .parse::<u64>()
            .ok()
            .map(|m| std::time::Duration::from_secs(m * 60))
    } else if let Some(num) = trimmed.strip_suffix('h') {
        num.trim()
            .parse::<u64>()
            .ok()
            .map(|h| std::time::Duration::from_secs(h * 3600))
    } else {
        trimmed
            .parse::<u64>()
            .ok()
            .map(std::time::Duration::from_secs)
    }
}
```

</details>



### `cloacina::registry::reconciler::loading::load_plugin_handle_from_bytes`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn load_plugin_handle_from_bytes (library_data : & [u8]) -> Result < fidius_host :: PluginHandle , String >
```

Write the cdylib bytes to a temp path and dlopen via fidius. The returned `PluginHandle` keeps the dlopen'd library alive; drop it to release. Used by `step_load_custom_triggers` to register FFI `Trigger` adapters for packaged cdylibs whose inventory submissions don't reach the host's `inventory::iter` (cross-cdylib linker boundary).

<details>
<summary>Source</summary>

```rust
fn load_plugin_handle_from_bytes(library_data: &[u8]) -> Result<fidius_host::PluginHandle, String> {
    use std::io::Write;
    let library_extension = crate::registry::loader::package_loader::get_library_extension();
    let temp_dir = tempfile::TempDir::new().map_err(|e| format!("temp dir: {}", e))?;
    let temp_path = temp_dir
        .path()
        .join(format!("trigger_plugin.{}", library_extension));
    {
        let mut f =
            std::fs::File::create(&temp_path).map_err(|e| format!("create temp library: {}", e))?;
        f.write_all(library_data)
            .map_err(|e| format!("write temp library: {}", e))?;
    }
    let loaded = fidius_host::loader::load_library(&temp_path)
        .map_err(|e| format!("dlopen failed: {:?}", e))?;
    let plugin = loaded
        .plugins
        .into_iter()
        .next()
        .ok_or_else(|| "library exposes no fidius plugins".to_string())?;
    let handle = fidius_host::PluginHandle::from_loaded(plugin);
    // Leak the temp_dir so the file path stays valid for the lifetime
    // of the dlopen handle. The OS reclaims on process exit; for the
    // long-running daemon/server use case this matches the intended
    // load lifecycle (one tempdir per package, dropped on full process
    // restart).
    std::mem::forget(temp_dir);
    Ok(handle)
}
```

</details>
