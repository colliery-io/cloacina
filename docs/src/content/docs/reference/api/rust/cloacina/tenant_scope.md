---
title: "cloacina::tenant_scope"
description: "Shared tenant-keying primitives for process-global name registries."
sidebar:
  order: 217
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
here so the [`Runtime`](crate::Runtime) registries and the
[`ComputationGraphScheduler`](crate::computation_graph::ComputationGraphScheduler)
reuse *one* convention instead of inventing a third.
Three ideas:
* [`TenantKey`] — the **map key**: `(tenant, name)`. `tenant_id: None` is the
untenanted/embedded single-tenant case and keeps pre-multi-tenancy
behaviour byte-for-byte.
* [`TenantOwner`] — the rest of the **provenance** stamped on each entry
(which package claimed the name), used to make a same-tenant collision
between two packages a loud error instead of a silent overwrite.
* [`TenantScope`] — the **caller's** scope, used to resolve a bare `name`
into a concrete key via [`resolve_tenant_key`].

## Structs

### <span id="struct.TenantKey"></span>`struct TenantKey`

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
| `tenant_id` | `Option < String >` |  |
| `name` | `String` |  |

#### Methods

##### <span id="TenantKey.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn new (tenant_id : Option < & str > , name : & str) -> Self
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



##### <span id="TenantKey.tenant_label"></span>`tenant_label`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn tenant_label (& self) -> & str
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





### <span id="struct.TenantOwner"></span>`struct TenantOwner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`

Provenance stamped on a registry entry beyond its [`TenantKey`].

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
| `package` | `Option < String >` |  |

#### Methods

##### <span id="TenantOwner.unknown"></span>`unknown`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn unknown () -> Self
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



##### <span id="TenantOwner.package"></span>`package`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn package (package : impl Into < String >) -> Self
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



##### <span id="TenantOwner.may_replace"></span>`may_replace`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn may_replace (& self , other : & TenantOwner) -> bool
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



##### <span id="TenantOwner.label"></span>`label`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn label (& self) -> String
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





### <span id="struct.TenantScope"></span>`struct TenantScope<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`, `Copy`, `Default`, `PartialEq`, `Eq`

Tenant scope of a *caller* performing a lookup.

Routes build this from the authenticated key; in-process callers build it
from the tenant their runner/reconciler is bound to.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option < & 'a str >` |  |
| `is_admin` | `bool` |  |



## Enums

### <span id="enum.TenantResolveMiss"></span>`enum TenantResolveMiss`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Why a scoped lookup failed to land on exactly one key.

#### Variants

- **`NotFound`**
- **`Ambiguous`** - The name exists in more than one tenant and the caller is an admin —
report both rather than guessing.



## Functions

### <span id="fn.resolve_tenant_key"></span>`fn resolve_tenant_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn resolve_tenant_key < V > (map : & HashMap < TenantKey , V > , scope : TenantScope < '_ > , name : & str ,) -> Result < TenantKey , TenantResolveMiss >
```

Resolve a bare `name` to a concrete [`TenantKey`] within `scope`.

Order:
1. the caller's own tenant — the normal multi-tenant path;
2. the untenanted entry (`tenant_id: None`) — embedded/pre-tenancy entries,
which were always globally addressable;
3. admins only: a unique cross-tenant match. Two tenants owning the same
name is [`TenantResolveMiss::Ambiguous`], never a guess.
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
                ))
            }
        }
    }

    Err(TenantResolveMiss::NotFound)
}
```

</details>



### <span id="fn.visible_keys"></span>`fn visible_keys`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn visible_keys < 'm , V > (map : & 'm HashMap < TenantKey , V > , scope : TenantScope < '_ > ,) -> impl Iterator < Item = & 'm TenantKey >
```

Every key in `map` visible to `scope`: the caller's own tenant plus the untenanted entries, or everything for an admin.

<details>
<summary>Source</summary>

```rust
pub fn visible_keys<'m, V>(
    map: &'m HashMap<TenantKey, V>,
    scope: TenantScope<'_>,
) -> impl Iterator<Item = &'m TenantKey> {
    let tenant = scope.tenant_id.map(|t| t.to_string());
    let is_admin = scope.is_admin;
    map.keys().filter(move |k| {
        is_admin || k.tenant_id.is_none() || k.tenant_id.as_deref() == tenant.as_deref()
    })
}
```

</details>



