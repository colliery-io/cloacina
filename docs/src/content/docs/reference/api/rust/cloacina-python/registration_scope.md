---
title: "cloacina-python::registration_scope"
description: "Thread-local \"who is importing right now\" slot (CLOACI-T-0921)."
editUrl: false
sidebar:
  order: 21
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Thread-local "who is importing right now" slot (CLOACI-T-0921).

[`crate::runtime_scope::ScopedRuntime`](../../../rust/cloacina-python/runtime_scope/#struct.ScopedRuntime) tells Python decorators *which
`Runtime`* to register into. This is its tenancy counterpart: it tells them
*whose* registration this is, so the process-global Python registries
(`GRAPH_EXECUTORS`, `ACCUMULATOR_REGISTRY`) can be keyed by
`(tenant, package, name)` instead of a bare name.

Without it, two tenants each shipping a graph called `pipeline` share one
executor slot and the second import silently wins for both — the Python-side
form of the same doctrine breach CLOACI-T-0921 fixes in `EndpointRegistry`.

The loader installs a [`ScopedRegistration`](../../../rust/cloacina-python/registration_scope/#struct.ScopedRegistration) around each package import.
Unscoped registration (embedded use, `cloaca` imported directly in a REPL,
unit tests) yields the default all-`None` scope, which behaves exactly as
the pre-T-0921 bare-name registry did.

## Structs

### <span id="struct.RegistrationScope"></span><code style="font-size: 0.85em">struct RegistrationScope</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Default`, `PartialEq`, `Eq`, `Hash`

Identity of the package whose import is currently running on this thread.

`None` fields mean "unknown / not packaged" — the embedded and test paths.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `Option<String>` |  |
| `package` | `Option<String>` |  |

#### Methods

##### <span id="RegistrationScope.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(tenant_id: Option<&str>, package: Option<&str>) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(tenant_id: Option<&str>, package: Option<&str>) -> Self {
        Self {
            tenant_id: tenant_id.map(|s| s.to_string()),
            package: package.map(|s| s.to_string()),
        }
    }
```

</details>

##### <span id="RegistrationScope.unscoped"></span><code>unscoped</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unscoped() -> Self
```

The unscoped (embedded / direct-import / test) identity.

<details>
<summary>Source</summary>

```rust
    pub fn unscoped() -> Self {
        Self::default()
    }
```

</details>

##### <span id="RegistrationScope.is_unscoped"></span><code>is_​unscoped</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_unscoped(&self) -> bool
```

True when neither tenant nor package is known.

<details>
<summary>Source</summary>

```rust
    pub fn is_unscoped(&self) -> bool {
        self.tenant_id.is_none() && self.package.is_none()
    }
```

</details>

### <span id="struct.ScopedRegistration"></span><code style="font-size: 0.85em">struct ScopedRegistration</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

RAII guard installing a [`RegistrationScope`](../../../rust/cloacina-python/registration_scope/#struct.RegistrationScope) for the duration of an import.

Unlike [`crate::runtime_scope::ScopedRuntime`](../../../rust/cloacina-python/runtime_scope/#struct.ScopedRuntime) this nests: the previous
scope is saved and restored on drop, so a package import that transitively
triggers another scoped import cannot strand the outer scope.

#### Methods

##### <span id="ScopedRegistration.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(scope: RegistrationScope) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(scope: RegistrationScope) -> Self {
        let previous = CURRENT_SCOPE.with(|slot| slot.replace(scope));
        Self { previous }
    }
```

</details>

##### <span id="ScopedRegistration.for_package"></span><code>for_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn for_package(tenant_id: Option<&str>, package: Option<&str>) -> Self
```

Convenience for the loader paths, which hold `&str` tenant/package.

<details>
<summary>Source</summary>

```rust
    pub fn for_package(tenant_id: Option<&str>, package: Option<&str>) -> Self {
        Self::new(RegistrationScope::new(tenant_id, package))
    }
```

</details>

## Functions

### <span id="fn.current_registration_scope"></span><code style="font-size: 0.85em">fn current_​registration_​scope</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn current_registration_scope() -> RegistrationScope
```

The registration scope installed on this thread (all-`None` if unscoped).

<details>
<summary>Source</summary>

```rust
pub fn current_registration_scope() -> RegistrationScope {
    CURRENT_SCOPE.with(|slot| slot.borrow().clone())
}
```

</details>
