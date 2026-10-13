---
title: "cloacina::registry::workflow_registry::filesystem"
description: "Filesystem-backed workflow registry for daemon mode."
editUrl: false
sidebar:
  order: 174
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Filesystem-backed workflow registry for daemon mode.

Implements `WorkflowRegistry` by scanning directories for `.cloacina` package
files. Packages live on disk — the filesystem IS the package store. SQLite
handles operational state (schedules, executions) separately.

## Structs

### <span id="struct.FilesystemWorkflowRegistry"></span><code style="font-size: 0.85em">struct FilesystemWorkflowRegistry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

A `WorkflowRegistry` implementation backed by directories of `.cloacina` files.

The daemon uses this instead of the database-backed registry. Packages are
discovered by scanning watch directories for `.cloacina` files. Package data
is read from disk on demand — no blobs stored in the database.

Supports multiple watch directories so users can organize packages across
different locations (e.g., `~/.cloacina/packages/`, `/opt/workflows/`,
`~/my-project/packages/`).

#### Methods

##### <span id="FilesystemWorkflowRegistry.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(watch_dirs: Vec<PathBuf>) -> Self
```

Create a new filesystem registry watching the given directories.

Directories that don't exist are logged as warnings but not rejected —
they may be created later (e.g., on first package drop).

<details>
<summary>Source</summary>

```rust
    pub fn new(watch_dirs: Vec<PathBuf>) -> Self {
        for dir in &watch_dirs {
            if !dir.exists() {
                warn!(
                    "Watch directory does not exist (will be watched if created later): {}",
                    dir.display()
                );
            }
        }
        Self { watch_dirs }
    }
```

</details>
