---
title: "cloacina::packaging::platform"
description: "Platform detection and target validation for workflow packages."
editUrl: false
sidebar:
  order: 150
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Platform detection and target validation for workflow packages.

## Functions

### <span id="fn.detect_current_platform"></span><code style="font-size: 0.85em">fn detect_​current_​platform</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn detect_current_platform() -> &'static str
```

Detect the current platform as a target string.

<details>
<summary>Source</summary>

```rust
pub fn detect_current_platform() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "macos-x86_64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-arm64"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    {
        "unknown"
    }
}
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.SUPPORTED_TARGETS"></span>`SUPPORTED_TARGETS` | `&[&str]` | `&["linux-x86_64", "linux-arm64", "macos-x86_64", "macos-a...` | Supported target platforms for workflow packages. |
