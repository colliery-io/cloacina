# cloacina::security::fleet_secret <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Fleet secret resolution — server-side wrap + agent-side unwrap (CLOACI-T-0861, I-0133 D-2/D-5/D-6, NFR-003).

This module bridges the [`crate::crypto::envelope`] HPKE primitive and the
[`crate::fleet::protocol`] wire types into the two halves of the fleet path:
- **Server:** [`resolve_and_wrap_secrets`] grant-checks + decrypts each named
secret through a [`SecretResolver`] (e.g. the gated
[`crate::security::SecretStoreResolver`]) and HPKE-wraps the plaintext to
the agent's advertised ephemeral public key, producing
[`WrappedSecret`](crate::fleet::protocol::WrappedSecret) blobs. Only
ciphertext leaves the server.
- **Agent:** [`InMemorySecretResolver::from_wrapped`] unwraps those blobs with
the agent's ephemeral private key into an in-memory map, then serves them to
task bodies via [`Context::secret`](cloacina_workflow::Context::secret). The
agent never holds the at-rest KEK and never persists the plaintext.

## Structs

### `cloacina::security::fleet_secret::InMemorySecretResolver`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`, `Default`

A [`SecretResolver`] that serves already-resolved values from an in-memory map — the fleet **agent's** resolver (CLOACI-T-0861).

Built by unwrapping the dispatch's [`WrappedSecret`] blobs. Holds the
decrypted `{field: value}` maps in memory for the run only; there is no DB,
no KEK, and nothing is persisted. Its [`Debug`] renders names only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `secrets` | `HashMap < String , BTreeMap < String , String > >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (secrets : HashMap < String , BTreeMap < String , String > >) -> Self
```

Build directly from a name → field-map table (mainly for tests / the embedded caller that already holds plaintext).

<details>
<summary>Source</summary>

```rust
    pub fn new(secrets: HashMap<String, BTreeMap<String, String>>) -> Self {
        Self { secrets }
    }
```

</details>



##### `empty` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn empty () -> Self
```

An empty resolver — serves nothing (every lookup is `NotFound`).

<details>
<summary>Source</summary>

```rust
    pub fn empty() -> Self {
        Self::default()
    }
```

</details>



##### `from_wrapped` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_wrapped (private_key : & EphemeralPrivateKey , wrapped : & [WrappedSecret] , execution_id : & str ,) -> Result < Self , FleetSecretError >
```

Agent side: unwrap a set of [`WrappedSecret`] blobs with the ephemeral private key into an in-memory resolver.

`execution_id` MUST be the same value the server used to wrap (the
dispatch's `task_execution_id` / `firing_id`) so the per-message AAD
matches; otherwise the AEAD open fails closed.

<details>
<summary>Source</summary>

```rust
    pub fn from_wrapped(
        private_key: &EphemeralPrivateKey,
        wrapped: &[WrappedSecret],
        execution_id: &str,
    ) -> Result<Self, FleetSecretError> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let mut secrets = HashMap::with_capacity(wrapped.len());
        for w in wrapped {
            let enc = b64
                .decode(&w.enc_b64)
                .map_err(|_| FleetSecretError::Encoding(w.name.clone()))?;
            let ciphertext = b64
                .decode(&w.ciphertext_b64)
                .map_err(|_| FleetSecretError::Encoding(w.name.clone()))?;
            let aad = secret_aad(execution_id, &w.name);
            let plaintext = envelope::unwrap(private_key, &enc, &ciphertext, &aad)?;
            let fields: BTreeMap<String, String> = serde_json::from_slice(&plaintext)
                .map_err(|_| FleetSecretError::Payload(w.name.clone()))?;
            secrets.insert(w.name.clone(), fields);
        }
        Ok(Self { secrets })
    }
```

</details>



##### `len` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn len (& self) -> usize
```

Number of secrets held.

<details>
<summary>Source</summary>

```rust
    pub fn len(&self) -> usize {
        self.secrets.len()
    }
```

</details>



##### `is_empty` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_empty (& self) -> bool
```

Whether the resolver holds no secrets.

<details>
<summary>Source</summary>

```rust
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
```

</details>



##### `into_arc` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn into_arc (self) -> Arc < dyn SecretResolver >
```

Box as a trait object ready to attach to a `Context`.

<details>
<summary>Source</summary>

```rust
    pub fn into_arc(self) -> Arc<dyn SecretResolver> {
        Arc::new(self)
    }
```

</details>





### `cloacina::security::fleet_secret::AgentKeyPool`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Default`

The **agent's** one-time ephemeral key pool (CLOACI-T-0861 / D-5).

Mints ephemeral X25519 keypairs, retains the private halves keyed by an opaque
`key_id`, and hands the public halves out as [`EphemeralKeyEntry`]s to
advertise to the server. A private key is used **at most once**: on a dispatch
stamped with a `key_id`, the agent [`take`](AgentKeyPool::take)s (removes) that
key, unwraps, and the key is gone forever. The private key material never
leaves the process and is never serialized.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `keys` | `HashMap < String , EphemeralPrivateKey >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

An empty pool.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self::default()
    }
```

</details>



##### `mint` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn mint (& mut self , n : usize) -> Vec < EphemeralKeyEntry >
```

Mint `n` fresh one-time keypairs: retain each private key under a fresh `key_id` and return the public entries to advertise to the server.

<details>
<summary>Source</summary>

```rust
    pub fn mint(&mut self, n: usize) -> Vec<EphemeralKeyEntry> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let kp = generate_ephemeral_keypair();
            let key_id = uuid::Uuid::new_v4().to_string();
            out.push(EphemeralKeyEntry {
                key_id: key_id.clone(),
                public_key_b64: b64.encode(&kp.public_key_bytes),
            });
            self.keys.insert(key_id, kp.private);
        }
        out
    }
```

</details>



##### `len` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn len (& self) -> usize
```

Number of unused private keys held.

<details>
<summary>Source</summary>

```rust
    pub fn len(&self) -> usize {
        self.keys.len()
    }
```

</details>



##### `is_empty` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_empty (& self) -> bool
```

Whether the pool holds no unused keys.

<details>
<summary>Source</summary>

```rust
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
```

</details>



##### `take` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn take (& mut self , key_id : & str) -> Option < EphemeralPrivateKey >
```

Take (remove) the private key for `key_id` for **one-time** use. `None` when the `key_id` is unknown or already consumed — the caller MUST then fail the execution cleanly rather than run with missing secrets.

<details>
<summary>Source</summary>

```rust
    pub fn take(&mut self, key_id: &str) -> Option<EphemeralPrivateKey> {
        self.keys.remove(key_id)
    }
```

</details>





### `cloacina::security::fleet_secret::ServerKeyPool`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Default`, `Clone`

The **server's** view of one agent's UNUSED one-time public keys (CLOACI-T-0861 / D-5).

Consume-once: [`consume`](ServerKeyPool::consume) hands out each key exactly
once and removes it, so a key is NEVER reused across dispatches. Exhaustion
(`consume` → `None`) MUST fail the dispatch cleanly — never send plaintext,
never reuse a key. FIFO so the oldest advertised key is spent first.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `unused` | `VecDeque < EphemeralKeyEntry >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

An empty pool.

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self::default()
    }
```

</details>



##### `from_entries` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_entries (entries : Vec < EphemeralKeyEntry >) -> Self
```

Seed the pool from an agent's advertised entries (e.g. at registration).

<details>
<summary>Source</summary>

```rust
    pub fn from_entries(entries: Vec<EphemeralKeyEntry>) -> Self {
        Self {
            unused: entries.into(),
        }
    }
```

</details>



##### `replenish` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn replenish (& mut self , entries : impl IntoIterator < Item = EphemeralKeyEntry >) -> usize
```

Append fresh entries (a replenish top-up). De-dupes by `key_id` so a retried top-up can't double-insert the same key.

<details>
<summary>Source</summary>

```rust
    pub fn replenish(&mut self, entries: impl IntoIterator<Item = EphemeralKeyEntry>) -> usize {
        let mut added = 0;
        for e in entries {
            if !self.unused.iter().any(|x| x.key_id == e.key_id) {
                self.unused.push_back(e);
                added += 1;
            }
        }
        added
    }
```

</details>



##### `consume` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn consume (& mut self) -> Option < EphemeralKeyEntry >
```

Consume one unused key (FIFO), removing it so it can never be handed out again (one-time). `None` when the pool is exhausted.

<details>
<summary>Source</summary>

```rust
    pub fn consume(&mut self) -> Option<EphemeralKeyEntry> {
        self.unused.pop_front()
    }
```

</details>



##### `len` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn len (& self) -> usize
```

Number of unused keys remaining.

<details>
<summary>Source</summary>

```rust
    pub fn len(&self) -> usize {
        self.unused.len()
    }
```

</details>



##### `is_empty` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_empty (& self) -> bool
```

Whether the pool is exhausted.

<details>
<summary>Source</summary>

```rust
    pub fn is_empty(&self) -> bool {
        self.unused.is_empty()
    }
```

</details>



##### `replenish_deficit` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn replenish_deficit (& self , target : usize) -> usize
```

How many more keys are needed to reach `target` (0 if at/above it) — the replenish signal the server returns to the agent.

<details>
<summary>Source</summary>

```rust
    pub fn replenish_deficit(&self, target: usize) -> usize {
        target.saturating_sub(self.unused.len())
    }
```

</details>





## Enums

### `cloacina::security::fleet_secret::FleetSecretError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors from the fleet wrap/unwrap path.

#### Variants

- **`Resolve`** - A secret failed to resolve on the server before wrapping.
- **`Envelope`** - HPKE wrap/unwrap failed.
- **`Encoding`** - A base64 field on a [`WrappedSecret`] was malformed.
- **`Payload`** - The unwrapped bytes were not a valid JSON `{field: value}` map.



## Functions

### `cloacina::security::fleet_secret::secret_aad`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn secret_aad (execution_id : & str , name : & str) -> Vec < u8 >
```

Derive the AEAD associated data binding a wrap to one execution + secret.

Both the server (wrap) and agent (unwrap) MUST derive this identically.

<details>
<summary>Source</summary>

```rust
pub fn secret_aad(execution_id: &str, name: &str) -> Vec<u8> {
    format!("{execution_id}/{name}").into_bytes()
}
```

</details>



### `cloacina::security::fleet_secret::resolve_and_wrap_secrets`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn resolve_and_wrap_secrets (resolver : & dyn SecretResolver , names : & [String] , execution_id : & str , recipient_public_key : & [u8] ,) -> Result < Vec < WrappedSecret > , FleetSecretError >
```

Server side: resolve each named secret (grant-checked by `resolver`) and HPKE-wrap it to `recipient_public_key`, bound to `execution_id`.

`resolver` should be the tenant + grant scoped resolver from T-0858/T-0860
(the wrap step adds no authorization of its own — it trusts the resolver's
gate). Returns one [`WrappedSecret`] per name; the plaintext exists only
transiently on the stack here and is never logged or persisted (NFR-001).

<details>
<summary>Source</summary>

```rust
pub async fn resolve_and_wrap_secrets(
    resolver: &dyn SecretResolver,
    names: &[String],
    execution_id: &str,
    recipient_public_key: &[u8],
) -> Result<Vec<WrappedSecret>, FleetSecretError> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let fields = resolver
            .resolve(name)
            .await
            .map_err(|source| FleetSecretError::Resolve {
                name: name.clone(),
                source,
            })?;
        out.push(wrap_field_map(
            name,
            &fields,
            execution_id,
            recipient_public_key,
        )?);
    }
    Ok(out)
}
```

</details>



### `cloacina::security::fleet_secret::wrap_field_map`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn wrap_field_map (name : & str , fields : & BTreeMap < String , String > , execution_id : & str , recipient_public_key : & [u8] ,) -> Result < WrappedSecret , FleetSecretError >
```

Wrap an already-resolved `{field: value}` map into a [`WrappedSecret`].

Split out from [`resolve_and_wrap_secrets`] so callers holding pre-resolved
values (and tests) can wrap directly.

<details>
<summary>Source</summary>

```rust
pub fn wrap_field_map(
    name: &str,
    fields: &BTreeMap<String, String>,
    execution_id: &str,
    recipient_public_key: &[u8],
) -> Result<WrappedSecret, FleetSecretError> {
    // Canonical JSON of the field map is the plaintext we seal.
    let plaintext =
        serde_json::to_vec(fields).map_err(|_| FleetSecretError::Payload(name.to_string()))?;
    let aad = secret_aad(execution_id, name);
    let (enc, ciphertext) = envelope::wrap(recipient_public_key, &plaintext, &aad)?;
    let b64 = base64::engine::general_purpose::STANDARD;
    Ok(WrappedSecret {
        name: name.to_string(),
        enc_b64: b64.encode(enc),
        ciphertext_b64: b64.encode(ciphertext),
    })
}
```

</details>



### `cloacina::security::fleet_secret::secret_ref_names`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn secret_ref_names (context : & serde_json :: Value) -> Vec < String >
```

Extract the concrete secret NAMES a fire context references via the T-0859 `{"$secret": name}` alias map (stored under [`SECRET_REFS_KEY`](cloacina_workflow::secret::SECRET_REFS_KEY)).

Returns the deduped, sorted set of secret names a fleet dispatch must resolve
+ wrap. NAMES only — this reads the alias map, which by construction carries
no secret values (NFR-001). Used by the server's `FleetExecutor` to learn
which secrets a task needs from the merged context it already builds.

<details>
<summary>Source</summary>

```rust
pub fn secret_ref_names(context: &serde_json::Value) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    if let Some(serde_json::Value::Object(map)) =
        context.get(cloacina_workflow::secret::SECRET_REFS_KEY)
    {
        for v in map.values() {
            if let serde_json::Value::String(name) = v {
                names.insert(name.clone());
            }
        }
    }
    names.into_iter().collect()
}
```

</details>



### `cloacina::security::fleet_secret::decode_pool_public_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn decode_pool_public_key (entry : & EphemeralKeyEntry) -> Result < Vec < u8 > , FleetSecretError >
```

Decode an [`EphemeralKeyEntry`]'s base64 public key into raw X25519 bytes.

<details>
<summary>Source</summary>

```rust
pub fn decode_pool_public_key(entry: &EphemeralKeyEntry) -> Result<Vec<u8>, FleetSecretError> {
    base64::engine::general_purpose::STANDARD
        .decode(&entry.public_key_b64)
        .map_err(|_| FleetSecretError::Encoding(entry.key_id.clone()))
}
```

</details>
