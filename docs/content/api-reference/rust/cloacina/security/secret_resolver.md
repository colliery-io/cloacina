# cloacina::security::secret_resolver <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Concrete secret resolver over the tenant-scoped [`SecretStore`] (CLOACI-I-0133 / T-0858, design D-1).

[`SecretStoreResolver`] is the embedded / in-process backend for the
[`Context::secret`](cloacina_workflow::Context::secret) accessor: it bundles
the three things resolution needs — a [`SecretStore`] handle, the tenant
`org_id`, and the server **KEK** — and decrypts a named secret into its
`{field: value}` map at fire time via [`SecretStore::resolve_secret`]. The
resolved plaintext is returned to the task and never persisted or logged
(NFR-001).

## Structs

### `cloacina::security::secret_resolver::SecretStoreResolver`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Resolves secrets by decrypting against a tenant-scoped [`SecretStore`].

Holds the server KEK in memory for the life of the resolver; it is never
serialized, logged, or exposed through [`Debug`] (see the manual impl below).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `store` | `SecretStore` |  |
| `org_id` | `UniversalUuid` |  |
| `kek` | `Vec < u8 >` |  |
| `allow` | `SecretAllow` | The enforced allow-list gate (CLOACI-T-0860, D-3). `All` for the trusted
embedded host; `List` (fail-closed) for the gated packaged path. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (store : SecretStore , org_id : UniversalUuid , kek : Vec < u8 >) -> Self
```

Construct a **trusted, ungated** resolver from an explicit KEK (32 bytes).

This is the embedded-host wiring seam (T-0858): the host owns the KEK and
is trusted, so the resolver may resolve any secret in `org_id`
([`SecretAllow::All`]). Untrusted packaged/WASM code must NOT be handed a
resolver built this way — use [`new_gated`](Self::new_gated) /
[`from_grants`](Self::from_grants) so its granted allow-list is enforced.
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



##### `new_gated` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new_gated (store : SecretStore , org_id : UniversalUuid , kek : Vec < u8 > , allow : SecretAllow ,) -> Self
```

Construct a **gated, fail-closed** resolver whose `resolve` is restricted to `allow` (CLOACI-T-0860, D-3).

This is the constructor the untrusted packaged-workflow / WASM path takes:
pass a [`SecretAllow::List`] (e.g. built by `SecretAllow::from_grants` from
the constructor's `ResolvedGrants`) so a name the tenant did not grant is
denied before any decrypt. An empty [`SecretAllow::List`] denies everything.

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



##### `from_grants` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_grants (store : SecretStore , org_id : UniversalUuid , kek : Vec < u8 > , grants : & crate :: registry :: loader :: grants :: ResolvedGrants ,) -> Self
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



##### `into_arc` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn into_arc (self) -> Arc < dyn SecretResolver >
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



##### `parse_kek` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn parse_kek (raw : & str) -> Result < Vec < u8 > , SecretResolverConfigError >
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



##### `kek_from_env` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn kek_from_env () -> Result < Vec < u8 > , SecretResolverConfigError >
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



##### `from_env` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_env (store : SecretStore , org_id : UniversalUuid ,) -> Result < Option < Self > , SecretResolverConfigError >
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

### `cloacina::security::secret_resolver::SecretAllow` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Which secret names a [`SecretStoreResolver`] is permitted to resolve (CLOACI-I-0133 / T-0860, design D-3) — the enforced trust boundary.

The resolver already scopes every lookup to its `org_id` (tenant is the outer
boundary), so this is the *inner* gate: within the tenant, which named secrets
may this holder resolve.
- [`SecretAllow::All`] — the **trusted embedded-host** path. The host owns the
KEK and wires the resolver in directly (T-0858), so it may resolve any secret
in its own tenant. Reachable ONLY via the explicitly-named ungated
constructor [`SecretStoreResolver::new`] / [`SecretStoreResolver::from_env`].
- [`SecretAllow::List`] — the **fail-closed gated** path for untrusted
packaged-workflow / WASM-constructor code. The set is the constructor's
granted `ResolvedGrants.secrets`; a name not in it is denied before any
decrypt. An empty set denies everything. Reachable via the gated
constructor [`SecretStoreResolver::new_gated`] (and, under the
`constructors-wasm` feature, the `from_grants` conveniences).
The two paths are separate constructors on purpose: "forgot to set the list"
cannot silently ungate the packaged path, because ungating (`All`) requires
calling the explicitly-named trusted constructor.

#### Variants

- **`All`** - Trusted: any secret in the resolver's tenant may be resolved.
- **`List`** - Fail-closed: only names in this set may be resolved.



### `cloacina::security::secret_resolver::SecretResolverConfigError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors constructing a [`SecretStoreResolver`] from configuration/environment.

#### Variants

- **`MissingEnv`**
- **`InvalidKek`**
