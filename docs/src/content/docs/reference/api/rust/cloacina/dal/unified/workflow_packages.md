---
title: "cloacina::dal::unified::workflow_packages"
description: "Unified Workflow Packages DAL with runtime backend selection"
sidebar:
  order: 110
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Unified Workflow Packages DAL with runtime backend selection

This module provides CRUD operations for WorkflowPackage entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### <span id="struct.WorkflowPackagesDAL"></span>`struct WorkflowPackagesDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Data access layer for workflow package operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### <span id="fn.select_provider_rows_for_target"></span>`fn select_provider_rows_for_target`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn select_provider_rows_for_target (rows : Vec < crate :: dal :: unified :: models :: PackageProvider > , target_triple : & str ,) -> Vec < crate :: dal :: unified :: models :: PackageProvider >
```

CLOACI-T-0908 (consumer): pick the provider rows THIS host should stage from the full row set of a package — for each `provider_name`, prefer the row whose `target_triple` matches `target_triple` exactly (a per-arch NATIVE build), else fall back to the primary (`target_triple` NULL) row. Rows for OTHER triples are never returned. Pure function so the reconciler, the agent route, and tests share one selection semantic.

<details>
<summary>Source</summary>

```rust
pub fn select_provider_rows_for_target(
    rows: Vec<crate::dal::unified::models::PackageProvider>,
    target_triple: &str,
) -> Vec<crate::dal::unified::models::PackageProvider> {
    let mut names: Vec<String> = Vec::new();
    for r in &rows {
        if !names.contains(&r.provider_name) {
            names.push(r.provider_name.clone());
        }
    }
    names
        .into_iter()
        .filter_map(|name| {
            rows.iter()
                .find(|r| {
                    r.provider_name == name && r.target_triple.as_deref() == Some(target_triple)
                })
                .or_else(|| {
                    rows.iter()
                        .find(|r| r.provider_name == name && r.target_triple.is_none())
                })
                .cloned()
        })
        .collect()
}
```

</details>



