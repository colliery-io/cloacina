---
title: "cloacina::security::secret_resolver"
description: "Concrete secret resolver over the tenant-scoped [`SecretStore`] (CLOACI-I-0133 / T-0858, design D-1)."
editUrl: false
sidebar:
  order: 194
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Concrete secret resolver over the tenant-scoped `SecretStore` (CLOACI-I-0133 / T-0858, design D-1).

[`SecretStoreResolver`](../../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver) is the embedded / in-process backend for the
[`cloacina_workflow::Context::secret`](../../../../rust/cloacina-workflow/context/#Context.secret) accessor: it bundles
the three things resolution needs — a `SecretStore` handle, the tenant
`org_id`, and the server **KEK** — and decrypts a named secret into its
`{field: value}` map at fire time via [`SecretStore::resolve_secret`](../../../../rust/cloacina/security/secret_store/#SecretStore.resolve_secret). The
resolved plaintext is returned to the task and never persisted or logged
(NFR-001).

**Server KEK sourcing (D-7)**

T-0857 left the server KEK unwired. This task sources it from the
`CLOACINA_SECRET_KEK` environment variable — the same env-driven pattern the
codebase already uses for secret material (e.g. `CLOACINA_VAR_API_KEY` in
[`crate::var`](../../../../rust/cloacina/var/)). The value is a base64 (standard) **or** hex encoding of
exactly 32 bytes (AES-256). Callers that manage the KEK themselves construct
the resolver directly via [`SecretStoreResolver::new`](../../../../rust/cloacina/security/secret_resolver/#SecretStoreResolver.new).

## Structs

### <span id="struct.SecretStoreResolver"></span><code style="font-size: 0.85em">struct SecretStoreResolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Resolves secrets by decrypting against a tenant-scoped `SecretStore`.

Holds the server KEK in memory for the life of the resolver; it is never
serialized, logged, or exposed through `Debug` (see the manual impl below).

#### Methods

##### <span id="SecretStoreResolver.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(store: SecretStore, org_id: UniversalUuid, kek: Vec<u8>) -> Self
```

Construct a **trusted, ungated** resolver from an explicit KEK (32 bytes).

This is the embedded-host wiring seam (T-0858): the host owns the KEK and
is trusted, so the resolver may resolve any secret in `org_id`
([`SecretAllow::All`](../../../../rust/cloacina/security/secret_resolver/#enum.SecretAllow)). Untrusted packaged/WASM code must NOT be handed a
resolver built this way — use `new_gated` /
`from_grants` so its granted allow-list is enforced.

`kek` must be 32 bytes; a wrong length surfaces later as a backend error at
resolve time (mirroring the store's own contract).

<details>
<summary>Source</summary>

```rust
    pub fn new(store: SecretStore, org_id: UniversalUuid, kek: Vec<u8>) -> Self {
        Self {
            store,
            org_id,
            kek,
            allow: SecretAllow::All,
        }
    }
```

</details>

##### <span id="SecretStoreResolver.new_gated"></span><code>new_​gated</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new_gated(
    store: SecretStore,
    org_id: UniversalUuid,
    kek: Vec<u8>,
    allow: SecretAllow,
) -> Self
```

Construct a **gated, fail-closed** resolver whose `resolve` is restricted to `allow` (CLOACI-T-0860, D-3).

This is the constructor the untrusted packaged-workflow / WASM path takes:
pass a [`SecretAllow::List`](../../../../rust/cloacina/security/secret_resolver/#enum.SecretAllow) (e.g. built by `SecretAllow::from_grants` from
the constructor's `ResolvedGrants`) so a name the tenant did not grant is
denied before any decrypt. An empty [`SecretAllow::List`](../../../../rust/cloacina/security/secret_resolver/#enum.SecretAllow) denies everything.

<details>
<summary>Source</summary>

```rust
    pub fn new_gated(
        store: SecretStore,
        org_id: UniversalUuid,
        kek: Vec<u8>,
        allow: SecretAllow,
    ) -> Self {
        Self {
            store,
            org_id,
            kek,
            allow,
        }
    }
```

</details>

##### <span id="SecretStoreResolver.from_grants"></span><code>from_​grants</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_grants(
    store: SecretStore,
    org_id: UniversalUuid,
    kek: Vec<u8>,
    grants: &crate::registry::loader::grants::ResolvedGrants,
) -> Self
```

Construct a gated resolver whose allow-list is the constructor's granted secrets (`ResolvedGrants.secrets`) — the one-call packaged-path seam.

Available only under `constructors-wasm` (the feature that defines the
grant model). This is where the packaged/WASM trust boundary is realized:
the resolver handed to untrusted constructor code is built from *its*
grants, fail-closed.

<details>
<summary>Source</summary>

```rust
    pub fn from_grants(
        store: SecretStore,
        org_id: UniversalUuid,
        kek: Vec<u8>,
        grants: &crate::registry::loader::grants::ResolvedGrants,
    ) -> Self {
        Self::new_gated(store, org_id, kek, SecretAllow::from_grants(grants))
    }
```

</details>

##### <span id="SecretStoreResolver.into_arc"></span><code>into_​arc</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn into_arc(self) -> Arc<dyn SecretResolver>
```

Construct + box as a trait object ready to attach to a `Context`.

<details>
<summary>Source</summary>

```rust
    pub fn into_arc(self) -> Arc<dyn SecretResolver> {
        Arc::new(self)
    }
```

</details>

##### <span id="SecretStoreResolver.parse_kek"></span><code>parse_​kek</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn parse_kek(raw: &str) -> Result<Vec<u8>, SecretResolverConfigError>
```

Parse a KEK from a base64 (standard) or hex string; requires 32 bytes.

<details>
<summary>Source</summary>

```rust
    pub fn parse_kek(raw: &str) -> Result<Vec<u8>, SecretResolverConfigError> {
        let raw = raw.trim();
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(raw) {
            if bytes.len() == 32 {
                return Ok(bytes);
            }
        }
        if let Ok(bytes) = hex::decode(raw) {
            if bytes.len() == 32 {
                return Ok(bytes);
            }
        }
        Err(SecretResolverConfigError::InvalidKek(KEK_ENV_VAR))
    }
```

</details>

##### <span id="SecretStoreResolver.kek_from_env"></span><code>kek_​from_​env</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn kek_from_env() -> Result<Vec<u8>, SecretResolverConfigError>
```

Read + parse the server KEK from `CLOACINA_SECRET_KEK`.

<details>
<summary>Source</summary>

```rust
    pub fn kek_from_env() -> Result<Vec<u8>, SecretResolverConfigError> {
        let raw = std::env::var(KEK_ENV_VAR)
            .map_err(|_| SecretResolverConfigError::MissingEnv(KEK_ENV_VAR))?;
        Self::parse_kek(&raw)
    }
```

</details>

##### <span id="SecretStoreResolver.from_env"></span><code>from_​env</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_env(
    store: SecretStore,
    org_id: UniversalUuid,
) -> Result<Option<Self>, SecretResolverConfigError>
```

Construct a resolver sourcing the KEK from `CLOACINA_SECRET_KEK`.

Returns `Ok(None)` when the env var is unset (secrets simply aren't
configured on this deployment); `Err` when it is set but malformed.

<details>
<summary>Source</summary>

```rust
    pub fn from_env(
        store: SecretStore,
        org_id: UniversalUuid,
    ) -> Result<Option<Self>, SecretResolverConfigError> {
        match std::env::var(KEK_ENV_VAR) {
            Err(_) => Ok(None),
            Ok(raw) => {
                let kek = Self::parse_kek(&raw)?;
                Ok(Some(Self::new(store, org_id, kek)))
            }
        }
    }
```

</details>

## Enums

### <span id="enum.SecretAllow"></span><code style="font-size: 0.85em">enum SecretAllow</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Which secret names a [`SecretStoreResolver`](../../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver) is permitted to resolve (CLOACI-I-0133 / T-0860, design D-3) — the enforced trust boundary.

The resolver already scopes every lookup to its `org_id` (tenant is the outer
boundary), so this is the *inner* gate: within the tenant, which named secrets
may this holder resolve.

- [`SecretAllow::All`](../../../../rust/cloacina/security/secret_resolver/#enum.SecretAllow) — the **trusted embedded-host** path. The host owns the
  KEK and wires the resolver in directly (T-0858), so it may resolve any secret
  in its own tenant. Reachable ONLY via the explicitly-named ungated
  constructor [`SecretStoreResolver::new`](../../../../rust/cloacina/security/secret_resolver/#SecretStoreResolver.new) / [`SecretStoreResolver::from_env`](../../../../rust/cloacina/security/secret_resolver/#SecretStoreResolver.from_env).
- [`SecretAllow::List`](../../../../rust/cloacina/security/secret_resolver/#enum.SecretAllow) — the **fail-closed gated** path for untrusted
  packaged-workflow / WASM-constructor code. The set is the constructor's
  granted `ResolvedGrants.secrets`; a name not in it is denied before any
  decrypt. An empty set denies everything. Reachable via the gated
  constructor [`SecretStoreResolver::new_gated`](../../../../rust/cloacina/security/secret_resolver/#SecretStoreResolver.new_gated) (and, under the
  `constructors-wasm` feature, the `from_grants` conveniences).

The two paths are separate constructors on purpose: "forgot to set the list"
cannot silently ungate the packaged path, because ungating (`All`) requires
calling the explicitly-named trusted constructor.

#### Variants

- **`All`** - Trusted: any secret in the resolver's tenant may be resolved.
- **`List`** - Fail-closed: only names in this set may be resolved.

#### Methods

##### <span id="SecretAllow.from_grants"></span><code>from_​grants</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_grants(
    grants: &crate::registry::loader::grants::ResolvedGrants,
) -> Self
```

Build the fail-closed allow-list from a constructor's resolved grants — the packaged/WASM path's gate (`ResolvedGrants.secrets` → allow-list).

Available only under `constructors-wasm`, the feature that defines the
grant model; the embedded host builds a [`SecretAllow`](../../../../rust/cloacina/security/secret_resolver/#enum.SecretAllow) directly.

<details>
<summary>Source</summary>

```rust
    pub fn from_grants(grants: &crate::registry::loader::grants::ResolvedGrants) -> Self {
        SecretAllow::List(grants.secrets.iter().cloned().collect())
    }
```

</details>

### <span id="enum.SecretResolverConfigError"></span><code style="font-size: 0.85em">enum SecretResolverConfigError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors constructing a [`SecretStoreResolver`](../../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver) from configuration/environment.

#### Variants

- **`MissingEnv`**
- **`InvalidKek`**

## Constants

| Name | Type | Value | Description |
|------|------|-------|-------------|
| <span id="const.KEK_ENV_VAR"></span>`KEK_ENV_VAR` | `&str` | `"CLOACINA_SECRET_KEK"` | Environment variable holding the server KEK (base64 or hex of 32 bytes). |
