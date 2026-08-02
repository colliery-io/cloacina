# cloacina::dal::unified::workflow_packages <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Workflow Packages DAL with runtime backend selection

This module provides CRUD operations for WorkflowPackage entities that work with
both PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

## Structs

### `cloacina::dal::unified::workflow_packages::WorkflowPackagesDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Data access layer for workflow package operations with runtime backend selection.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### `cloacina::dal::unified::workflow_packages::select_provider_rows_for_target`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
