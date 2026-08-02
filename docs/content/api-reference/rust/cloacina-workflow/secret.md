# cloacina-workflow::secret <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Secret resolution side channel (CLOACI-I-0133 / T-0858, design D-1).

A task/constructor reads a resolved secret through [`Context::secret`] — a
dedicated accessor on the execution scope that is **structurally distinct**
from the durable [`Context`](crate::Context) data. The resolved plaintext is
*returned* to the task; it is never inserted into the context's serialized
`data` map, so it can never land in `schedules.params`, the fires log, audit
rows, or execution history (NFR-001).
This module defines only the trait + error types that live in the authoring
crate. The concrete backend (which decrypts against the tenant-scoped
`SecretStore`) lives in the `cloacina` runtime crate as `SecretStoreResolver`
and is threaded onto the `Context` by the executor at fire time.

## Structs

### `cloacina-workflow::secret::MapSecretResolver`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


In-memory resolver over already-resolved secret values, keyed by concrete secret name (CLOACI-T-0895).

The packaged-task bridge uses this on the PLUGIN side: the host resolves
every `{"$secret"}`-referenced secret through its real backend before the
plugin call and ships the values across the boundary in the
`TaskExecutionRequest`; the plugin shell rebuilds the execution scope with
this resolver so `context.secret(...)` works identically inside the
package. Values live only in this object for the duration of one task
invocation — never serialized into the durable context (NFR-001).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `secrets` | `BTreeMap < String , BTreeMap < String , String > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (secrets : BTreeMap < String , BTreeMap < String , String > >) -> Self
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

### `cloacina-workflow::secret::SecretResolverError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Error returned by a [`SecretResolver`] backend implementation.

#### Variants

- **`NotFound`** - No secret of that name is visible to this tenant/scope.
- **`NotGranted`** - The name is not in this scope's granted secret allow-list
(CLOACI-I-0133 / T-0860, design D-3). Returned **before** any decryption —
the holder was never authorized to resolve this secret, regardless of
whether it exists. Distinct from [`NotFound`](Self::NotFound) so a denial
is not confusable with a missing secret in audit/logs.
- **`Backend`** - The backend failed to resolve (decrypt failure, DB error, misconfigured
KEK, …). The message is a redacted, non-plaintext description.



### `cloacina-workflow::secret::SecretAccessError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Error surfaced to a task body by the [`Context`](crate::Context) secret accessor.

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
