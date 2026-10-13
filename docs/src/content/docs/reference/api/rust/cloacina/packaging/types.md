---
title: "cloacina::packaging::types"
description: "API reference of the Rust module cloacina::packaging::types."
editUrl: false
sidebar:
  order: 152
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

## Structs

### <span id="struct.CompileOptions"></span><code style="font-size: 0.85em">struct CompileOptions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Options for compiling a workflow

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `target` | `Option<String>` | Target triple for cross-compilation |
| `profile` | `String` | Build profile (debug/release) |
| `cargo_flags` | `Vec<String>` | Additional cargo flags |
| `jobs` | `Option<u32>` | Number of parallel jobs |

### <span id="struct.CargoToml"></span><code style="font-size: 0.85em">struct CargoToml</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Deserialize`

Parsed Cargo.toml structure

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package` | `Option<CargoPackage>` |  |
| `lib` | `Option<CargoLib>` |  |
| `dependencies` | `Option<toml::Value>` |  |

### <span id="struct.CargoPackage"></span><code style="font-size: 0.85em">struct CargoPackage</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Deserialize`

Package section from Cargo.toml

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `version` | `String` |  |
| `description` | `Option<String>` |  |
| `authors` | `Option<Vec<String>>` |  |
| `keywords` | `Option<Vec<String>>` |  |
| `rust_version` | `Option<String>` |  |

### <span id="struct.CargoLib"></span><code style="font-size: 0.85em">struct CargoLib</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Deserialize`

Library section from Cargo.toml

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `crate_type` | `Option<Vec<String>>` |  |

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.MANIFEST_FILENAME"></span>`MANIFEST_FILENAME` | `&str` | `"manifest.json"` | Constants |
| <span id="const.CLOACINA_VERSION"></span>`CLOACINA_VERSION` | `&str` | `env!("CARGO_PKG_VERSION")` |  |
