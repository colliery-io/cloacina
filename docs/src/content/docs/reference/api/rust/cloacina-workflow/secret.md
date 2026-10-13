---
title: "cloacina-workflow::secret"
description: "Secret resolution side channel (CLOACI-I-0133 / T-0858, design D-1)."
editUrl: false
sidebar:
  order: 36
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Secret resolution side channel (CLOACI-I-0133 / T-0858, design D-1).

A task/constructor reads a resolved secret through [`Context::secret`](../../../rust/cloacina-workflow/context/#Context.secret) — a
dedicated accessor on the execution scope that is **structurally distinct**
from the durable [`Context`](../../../rust/cloacina-workflow/context/#struct.Context) data. The resolved plaintext is
*returned* to the task; it is never inserted into the context's serialized
`data` map, so it can never land in `schedules.params`, the fires log, audit
rows, or execution history (NFR-001).

This module defines only the trait + error types that live in the authoring
crate. The concrete backend (which decrypts against the tenant-scoped
`SecretStore`) lives in the `cloacina` runtime crate as `SecretStoreResolver`
and is threaded onto the `Context` by the executor at fire time.

## Structs

### <span id="struct.MapSecretResolver"></span><code style="font-size: 0.85em">struct MapSecretResolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

In-memory resolver over already-resolved secret values, keyed by concrete secret name (CLOACI-T-0895).

The packaged-task bridge uses this on the PLUGIN side: the host resolves
every `{"$secret"}`-referenced secret through its real backend before the
plugin call and ships the values across the boundary in the
`TaskExecutionRequest`; the plugin shell rebuilds the execution scope with
this resolver so `context.secret(...)` works identically inside the
package. Values live only in this object for the duration of one task
invocation — never serialized into the durable context (NFR-001).

#### Methods

##### <span id="MapSecretResolver.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(secrets: BTreeMap<String, BTreeMap<String, String>>) -> Self
```

Wrap a `{secret_name: {field: value}}` map.

<details>
<summary>Source</summary>

```rust
    pub fn new(secrets: BTreeMap<String, BTreeMap<String, String>>) -> Self {
        Self { secrets }
    }
```

</details>

## Enums

### <span id="enum.SecretResolverError"></span><code style="font-size: 0.85em">enum SecretResolverError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Error returned by a [`SecretResolver`](../../../rust/cloacina-workflow/secret/#trait.SecretResolver) backend implementation.

#### Variants

- **`NotFound`** - No secret of that name is visible to this tenant/scope.
- **`NotGranted`** - The name is not in this scope's granted secret allow-list
  (CLOACI-I-0133 / T-0860, design D-3). Returned **before** any decryption —
  the holder was never authorized to resolve this secret, regardless of
  whether it exists. Distinct from `NotFound` so a denial
  is not confusable with a missing secret in audit/logs.
- **`Backend`** - The backend failed to resolve (decrypt failure, DB error, misconfigured
  KEK, …). The message is a redacted, non-plaintext description.

### <span id="enum.SecretAccessError"></span><code style="font-size: 0.85em">enum SecretAccessError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Error surfaced to a task body by the [`Context`](../../../rust/cloacina-workflow/context/#struct.Context) secret accessor.

#### Variants

- **`NotConfigured`** - No resolver was configured on this execution scope. On the embedded /
  in-process path the host/runner wires one in; when it is absent,
  `context.secret(...)` fails clearly instead of silently returning empty.
- **`NotFound`** - The named secret does not exist (or is not visible to this tenant).
- **`NotGranted`** - The execution scope's grant does not include this secret name
  (CLOACI-I-0133 / T-0860, D-3). The resolver denied it **before** any
  decrypt; add the name to the constructor's `secrets` grant to allow it.
- **`FieldNotFound`** - The secret exists but has no field of that name.
- **`Backend`** - The backend failed to resolve the secret.

## Traits

### <span id="trait.SecretResolver"></span><code style="font-size: 0.85em">trait SecretResolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

A backend that resolves a named secret into its plaintext `{field: value}` map at fire time.

Implementations decrypt at the last possible moment and return the fields to
the caller; they MUST NOT persist or log the plaintext. The runtime attaches
a resolver to the [`Context`](../../../rust/cloacina-workflow/context/#struct.Context) via a non-serialized handle
(see [`Context::set_secret_resolver`](../../../rust/cloacina-workflow/context/#Context.set_secret_resolver)),
which is what keeps resolution structurally separate from the durable
context.

#### Methods

##### <span id="SecretResolver.resolve"></span><code>resolve</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn resolve(
    &self,
    name: &str,
) -> Result<BTreeMap<String, String>, SecretResolverError>
```

Resolve `name` to its decrypted `{field: value}` map.

<details>
<summary>Source</summary>

```rust
    async fn resolve(&self, name: &str) -> Result<BTreeMap<String, String>, SecretResolverError>;
```

</details>

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.SECRET_REFS_KEY"></span>`SECRET_REFS_KEY` | `&str` | `"__cloacina_secret_refs__"` | Reserved `Context` data key holding the instance's `{"$secret": name}` binding map (CLOACI-I-0133 / T-0859, design D-4).<br>At fire time `merge_instance_params` recognizes a `{"$secret": "name"}` param value, keeps the **resolved value** out of the context entirely, and records only the non-sensitive `local_binding_name -> secret_name` alias here. The map carries NAMES ONLY (never values), so it is safe to serialize into the durable context; it survives the fire → persist → execute boundary and lets [`Context::secret`](../../../rust/cloacina-workflow/context/#Context.secret) resolve a task's declared local binding name to the concrete secret the instance chose. |
