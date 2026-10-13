---
title: "cloacina::tenant_scope"
description: "Shared tenant-keying primitives for process-global name registries."
editUrl: false
sidebar:
  order: 199
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Shared tenant-keying primitives for process-global name registries.

Tenant is THE isolation boundary (see CLOACI-S-0011 and the authz design),
so any map that lives for the life of the process and is keyed by an entity
name authored by a tenant must carry the tenant *in the key*, not as payload
hanging off the value.

CLOACI-T-0921 established this shape for the `EndpointRegistry`
(`EndpointKey` / `EndpointOwner` / `EndpointScope`); CLOACI-T-0924 lifted it
here so the [`Runtime`](../../../rust/cloacina/runtime/#struct.Runtime) registries and the
[`ComputationGraphScheduler`](../../../rust/cloacina/computation_graph/scheduler/#struct.ComputationGraphScheduler)
reuse *one* convention instead of inventing a third.

Three ideas:

* [`TenantKey`](../../../rust/cloacina/tenant_scope/#struct.TenantKey) — the **map key**: `(tenant, name)`. `tenant_id: None` is the
  untenanted/embedded single-tenant case and keeps pre-multi-tenancy
  behaviour byte-for-byte.
* [`TenantOwner`](../../../rust/cloacina/tenant_scope/#struct.TenantOwner) — the rest of the **provenance** stamped on each entry
  (which package claimed the name), used to make a same-tenant collision
  between two packages a loud error instead of a silent overwrite.
* [`TenantScope`](../../../rust/cloacina/tenant_scope/#struct.TenantScope) — the **caller's** scope, used to resolve a bare `name`
  into a concrete key via [`resolve_tenant_key`](../../../rust/cloacina/tenant_scope/#fn.resolve_tenant_key).

**Why not [`TaskNamespace`](../../../rust/cloacina-workflow/namespace/#struct.TaskNamespace)?**

`RuntimeInner.tasks` is keyed by `TaskNamespace`
(`tenant::package::workflow::task`) and is deliberately *not* migrated to
this shape: a task's full namespace string is persisted on `task_executions`
rows, so every reader genuinely has all four components. The registries this
module serves are addressed by **bare name** at their read sites (a workflow
name off a `workflow_executions` row, a trigger name off a cron schedule, a
reactor name off a WebSocket path), so package belongs in the *owner
metadata* used to detect collisions, not in the key.

## Structs

### <span id="struct.TenantKey"></span><code style="font-size: 0.85em">struct TenantKey</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`

Composite `(tenant, name)` lookup key for a process-global registry.

`tenant_id: None` is the untenanted/embedded entry. The tenant is stored
verbatim — there is deliberately no folding of the reserved `"public"`
tenant onto `None`, so this key matches CLOACI-T-0921's `EndpointKey` and a
single deployment never has two spellings of the same scope.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option<String>` |  |
| `name` | `String` |  |

#### Methods

##### <span id="TenantKey.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(tenant_id: Option<&str>, name: &str) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(tenant_id: Option<&str>, name: &str) -> Self {
        Self {
            tenant_id: tenant_id.map(|t| t.to_string()),
            name: name.to_string(),
        }
    }
```

</details>

##### <span id="TenantKey.tenant_label"></span><code>tenant_​label</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tenant_label(&self) -> &str
```

Operator-facing tenant label for error messages.

<details>
<summary>Source</summary>

```rust
    pub fn tenant_label(&self) -> &str {
        self.tenant_id.as_deref().unwrap_or("<untenanted>")
    }
```

</details>

### <span id="struct.TenantOwner"></span><code style="font-size: 0.85em">struct TenantOwner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`

Provenance stamped on a registry entry beyond its [`TenantKey`](../../../rust/cloacina/tenant_scope/#struct.TenantKey).

The key is `(tenant, name)`; this records *who* claimed it. It exists so a
same-tenant claim on a live name by a **different package** is rejected
loudly at load time instead of silently replacing the incumbent, and so a
package's unload only ever removes entries it installed.

`package: None` means "provenance unknown" — hand-wired registrations,
macro/`inventory` seeding, and the Python decorator paths. An unknown owner
never *causes* a conflict (there is nothing to compare), which keeps every
pre-existing caller behaving exactly as it did.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package` | `Option<String>` |  |

#### Methods

##### <span id="TenantOwner.unknown"></span><code>unknown</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unknown() -> Self
```

An owner with no package provenance (hand-wired / inventory / embedded).

<details>
<summary>Source</summary>

```rust
    pub fn unknown() -> Self {
        Self { package: None }
    }
```

</details>

##### <span id="TenantOwner.package"></span><code>package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn package(package: impl Into<String>) -> Self
```

An owner identified by the package that claimed the name.

<details>
<summary>Source</summary>

```rust
    pub fn package(package: impl Into<String>) -> Self {
        Self {
            package: Some(package.into()),
        }
    }
```

</details>

##### <span id="TenantOwner.may_replace"></span><code>may_​replace</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn may_replace(&self, other: &TenantOwner) -> bool
```

`true` when `other` may replace an entry owned by `self`.

Same package replaces (a package reload / restart depends on it); an
unknown owner on either side is permissive, because there is no
provenance to contradict. Two *named and different* packages conflict.

<details>
<summary>Source</summary>

```rust
    pub fn may_replace(&self, other: &TenantOwner) -> bool {
        match (&self.package, &other.package) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        }
    }
```

</details>

##### <span id="TenantOwner.label"></span><code>label</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn label(&self) -> String
```

Operator-facing provenance string for conflict errors.

<details>
<summary>Source</summary>

```rust
    pub fn label(&self) -> String {
        match &self.package {
            Some(p) => format!("package '{}'", p),
            None => "an unattributed registration".to_string(),
        }
    }
```

</details>

### <span id="struct.TenantScope"></span><code style="font-size: 0.85em">struct TenantScope&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`

Tenant scope of a *caller* performing a lookup.

Routes build this from the authenticated key; in-process callers build it
from the tenant their runner/reconciler is bound to.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option<&'a str>` |  |
| `is_admin` | `bool` |  |

#### Methods

##### <span id="TenantScope.of"></span><code>of</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn of(tenant_id: Option<&'a str>) -> Self
```

A non-admin caller in `tenant_id` (`None` = untenanted deployment).

<details>
<summary>Source</summary>

```rust
    pub fn of(tenant_id: Option<&'a str>) -> Self {
        Self {
            tenant_id,
            is_admin: false,
        }
    }
```

</details>

##### <span id="TenantScope.tenant"></span><code>tenant</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tenant(tenant_id: &'a str) -> Self
```

A non-admin caller in a specific tenant.

<details>
<summary>Source</summary>

```rust
    pub fn tenant(tenant_id: &'a str) -> Self {
        Self::of(Some(tenant_id))
    }
```

</details>

##### <span id="TenantScope.untenanted"></span><code>untenanted</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn untenanted() -> Self
```

The untenanted (embedded / single-tenant) caller.

<details>
<summary>Source</summary>

```rust
    pub fn untenanted() -> Self {
        Self::of(None)
    }
```

</details>

##### <span id="TenantScope.admin"></span><code>admin</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn admin() -> Self
```

An admin caller: may reach any tenant's entry, but only when the name is unambiguous across tenants.

<details>
<summary>Source</summary>

```rust
    pub fn admin() -> Self {
        Self {
            tenant_id: None,
            is_admin: true,
        }
    }
```

</details>

##### <span id="TenantScope.own_key"></span><code>own_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn own_key(&self, name: &str) -> TenantKey
```

The key this scope *writes* under.

<details>
<summary>Source</summary>

```rust
    pub fn own_key(&self, name: &str) -> TenantKey {
        TenantKey::new(self.tenant_id, name)
    }
```

</details>

## Enums

### <span id="enum.TenantResolveMiss"></span><code style="font-size: 0.85em">enum TenantResolveMiss</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Why a scoped lookup failed to land on exactly one key.

#### Variants

- **`NotFound`**
- **`Ambiguous`** - The name exists in more than one tenant and the caller is an admin —
  report both rather than guessing.

## Functions

### <span id="fn.resolve_tenant_key"></span><code style="font-size: 0.85em">fn resolve_​tenant_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn resolve_tenant_key<V>(
    map: &HashMap<TenantKey, V>,
    scope: TenantScope<'_>,
    name: &str,
) -> Result<TenantKey, TenantResolveMiss>
```

Resolve a bare `name` to a concrete [`TenantKey`](../../../rust/cloacina/tenant_scope/#struct.TenantKey) within `scope`.

Order:
1. the caller's own tenant — the normal multi-tenant path;
2. the untenanted entry (`tenant_id: None`) — embedded/pre-tenancy entries,
   which were always globally addressable;
3. admins only: a unique cross-tenant match. Two tenants owning the same
   name is [`TenantResolveMiss::Ambiguous`](../../../rust/cloacina/tenant_scope/#enum.TenantResolveMiss), never a guess.

A non-admin caller can therefore never resolve another tenant's entry.

<details>
<summary>Source</summary>

```rust
pub fn resolve_tenant_key<V>(
    map: &HashMap<TenantKey, V>,
    scope: TenantScope<'_>,
    name: &str,
) -> Result<TenantKey, TenantResolveMiss> {
    if let Some(tenant) = scope.tenant_id {
        let key = TenantKey::new(Some(tenant), name);
        if map.contains_key(&key) {
            return Ok(key);
        }
    }

    let untenanted = TenantKey::new(None, name);
    if map.contains_key(&untenanted) {
        return Ok(untenanted);
    }

    if scope.is_admin {
        let mut matches: Vec<&TenantKey> = map.keys().filter(|k| k.name == name).collect();
        matches.sort();
        match matches.len() {
            0 => return Err(TenantResolveMiss::NotFound),
            1 => return Ok(matches[0].clone()),
            _ => {
                return Err(TenantResolveMiss::Ambiguous(
                    matches
                        .iter()
                        .map(|k| k.tenant_label().to_string())
                        .collect(),
                ));
            }
        }
    }

    Err(TenantResolveMiss::NotFound)
}
```

</details>

### <span id="fn.visible_keys"></span><code style="font-size: 0.85em">fn visible_​keys</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn visible_keys<'m, V>(
    map: &'m HashMap<TenantKey, V>,
    scope: TenantScope<'_>,
) -> impl Iterator<Item = &'m TenantKey> + use<'m, V>
```

Every key in `map` visible to `scope`: the caller's own tenant plus the untenanted entries, or everything for an admin.

<details>
<summary>Source</summary>

```rust
pub fn visible_keys<'m, V>(
    map: &'m HashMap<TenantKey, V>,
    scope: TenantScope<'_>,
) -> impl Iterator<Item = &'m TenantKey> + use<'m, V> {
    let tenant = scope.tenant_id.map(|t| t.to_string());
    let is_admin = scope.is_admin;
    map.keys().filter(move |k| {
        is_admin || k.tenant_id.is_none() || k.tenant_id.as_deref() == tenant.as_deref()
    })
}
```

</details>
