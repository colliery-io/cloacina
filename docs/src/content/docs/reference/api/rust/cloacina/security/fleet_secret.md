---
title: "cloacina::security::fleet_secret"
description: "Fleet secret resolution — server-side wrap + agent-side unwrap (CLOACI-T-0861, I-0133 D-2/D-5/D-6, NFR-003)."
editUrl: false
sidebar:
  order: 191
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Fleet secret resolution — server-side wrap + agent-side unwrap (CLOACI-T-0861, I-0133 D-2/D-5/D-6, NFR-003).

This module bridges the [`crate::crypto::envelope`](../../../../rust/cloacina/crypto/envelope/) HPKE primitive and the
[`crate::fleet::protocol`](../../../../rust/cloacina/fleet/protocol/) wire types into the two halves of the fleet path:

- **Server:** [`resolve_and_wrap_secrets`](../../../../rust/cloacina/security/fleet_secret/#fn.resolve_and_wrap_secrets) grant-checks + decrypts each named
  secret through a [`SecretResolver`](../../../../rust/cloacina-workflow/secret/#trait.SecretResolver) (e.g. the gated
  [`crate::security::SecretStoreResolver`](../../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver)) and HPKE-wraps the plaintext to
  the agent's advertised ephemeral public key, producing
  [`WrappedSecret`](../../../../rust/cloacina/fleet/protocol/#struct.WrappedSecret) blobs. Only
  ciphertext leaves the server.
- **Agent:** [`InMemorySecretResolver::from_wrapped`](../../../../rust/cloacina/security/fleet_secret/#InMemorySecretResolver.from_wrapped) unwraps those blobs with
  the agent's ephemeral private key into an in-memory map, then serves them to
  task bodies via [`cloacina_workflow::Context::secret`](../../../../rust/cloacina-workflow/context/#Context.secret). The
  agent never holds the at-rest KEK and never persists the plaintext.

**Binding (replay/isolation)**

Each wrap's AEAD associated data is `"{execution_id}/{name}"` ([`secret_aad`](../../../../rust/cloacina/security/fleet_secret/#fn.secret_aad)).
Both sides derive it identically, so a ciphertext for one execution+secret
cannot be opened as a different one — on top of the per-keypair binding the
HPKE KEM already provides.

**Per-execution forward secrecy via a one-time key POOL (D-5, refined)**

The live fleet protocol is **push**, not claim — the server pushes a
`WorkPacket` and there is no per-dispatch round-trip on which the agent could
attach a fresh key. D-5 therefore uses a **one-time key pool**: the agent
mints a pool of ephemeral keypairs ([`AgentKeyPool`](../../../../rust/cloacina/security/fleet_secret/#struct.AgentKeyPool)), advertises the public
halves (each with a `key_id`) at registration + replenish; the server persists
the unused public keys ([`ServerKeyPool`](../../../../rust/cloacina/security/fleet_secret/#struct.ServerKeyPool)) and **consumes exactly one** per
secret-bearing dispatch, stamping its `key_id` on the packet. The agent looks
up the matching private key, unwraps ONCE, and discards it. Each execution
thus gets a fresh single-use key = true per-execution forward secrecy, with no
dispatch latency. Pool exhaustion fails the dispatch cleanly (never plaintext,
never key reuse); the server signals the agent to top up before that happens.

## Structs

### <span id="struct.InMemorySecretResolver"></span><code style="font-size: 0.85em">struct InMemorySecretResolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`, `Default`

A [`SecretResolver`](../../../../rust/cloacina-workflow/secret/#trait.SecretResolver) that serves already-resolved values from an in-memory map — the fleet **agent's** resolver (CLOACI-T-0861).

Built by unwrapping the dispatch's [`WrappedSecret`](../../../../rust/cloacina/fleet/protocol/#struct.WrappedSecret) blobs. Holds the
decrypted `{field: value}` maps in memory for the run only; there is no DB,
no KEK, and nothing is persisted. Its `Debug` renders names only.

#### Methods

##### <span id="InMemorySecretResolver.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(secrets: HashMap<String, BTreeMap<String, String>>) -> Self
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

##### <span id="InMemorySecretResolver.empty"></span><code>empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn empty() -> Self
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

##### <span id="InMemorySecretResolver.from_wrapped"></span><code>from_​wrapped</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_wrapped(
    private_key: &EphemeralPrivateKey,
    wrapped: &[WrappedSecret],
    execution_id: &str,
) -> Result<Self, FleetSecretError>
```

Agent side: unwrap a set of [`WrappedSecret`](../../../../rust/cloacina/fleet/protocol/#struct.WrappedSecret) blobs with the ephemeral private key into an in-memory resolver.

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

##### <span id="InMemorySecretResolver.len"></span><code>len</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn len(&self) -> usize
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

##### <span id="InMemorySecretResolver.is_empty"></span><code>is_​empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_empty(&self) -> bool
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

##### <span id="InMemorySecretResolver.into_arc"></span><code>into_​arc</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn into_arc(self) -> Arc<dyn SecretResolver>
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

### <span id="struct.AgentKeyPool"></span><code style="font-size: 0.85em">struct AgentKeyPool</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Default`

The **agent's** one-time ephemeral key pool (CLOACI-T-0861 / D-5).

Mints ephemeral X25519 keypairs, retains the private halves keyed by an opaque
`key_id`, and hands the public halves out as [`EphemeralKeyEntry`](../../../../rust/cloacina/fleet/protocol/#struct.EphemeralKeyEntry)s to
advertise to the server. A private key is used **at most once**: on a dispatch
stamped with a `key_id`, the agent [`take`](../../../../rust/cloacina/security/fleet_secret/#AgentKeyPool.take)s (removes) that
key, unwraps, and the key is gone forever. The private key material never
leaves the process and is never serialized.

#### Methods

##### <span id="AgentKeyPool.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
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

##### <span id="AgentKeyPool.mint"></span><code>mint</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn mint(&mut self, n: usize) -> Vec<EphemeralKeyEntry>
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

##### <span id="AgentKeyPool.len"></span><code>len</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn len(&self) -> usize
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

##### <span id="AgentKeyPool.is_empty"></span><code>is_​empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_empty(&self) -> bool
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

##### <span id="AgentKeyPool.take"></span><code>take</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn take(&mut self, key_id: &str) -> Option<EphemeralPrivateKey>
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

### <span id="struct.ServerKeyPool"></span><code style="font-size: 0.85em">struct ServerKeyPool</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Default`, `Clone`

The **server's** view of one agent's UNUSED one-time public keys (CLOACI-T-0861 / D-5).

Consume-once: [`consume`](../../../../rust/cloacina/security/fleet_secret/#ServerKeyPool.consume) hands out each key exactly
once and removes it, so a key is NEVER reused across dispatches. Exhaustion
(`consume` → `None`) MUST fail the dispatch cleanly — never send plaintext,
never reuse a key. FIFO so the oldest advertised key is spent first.

#### Methods

##### <span id="ServerKeyPool.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
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

##### <span id="ServerKeyPool.from_entries"></span><code>from_​entries</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_entries(entries: Vec<EphemeralKeyEntry>) -> Self
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

##### <span id="ServerKeyPool.replenish"></span><code>replenish</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn replenish(
    &mut self,
    entries: impl IntoIterator<Item = EphemeralKeyEntry>,
) -> usize
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

##### <span id="ServerKeyPool.consume"></span><code>consume</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn consume(&mut self) -> Option<EphemeralKeyEntry>
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

##### <span id="ServerKeyPool.len"></span><code>len</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn len(&self) -> usize
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

##### <span id="ServerKeyPool.is_empty"></span><code>is_​empty</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_empty(&self) -> bool
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

##### <span id="ServerKeyPool.replenish_deficit"></span><code>replenish_​deficit</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn replenish_deficit(&self, target: usize) -> usize
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

### <span id="enum.FleetSecretError"></span><code style="font-size: 0.85em">enum FleetSecretError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors from the fleet wrap/unwrap path.

#### Variants

- **`Resolve`** - A secret failed to resolve on the server before wrapping.
- **`Envelope`** - HPKE wrap/unwrap failed.
- **`Encoding`** - A base64 field on a [`WrappedSecret`](../../../../rust/cloacina/fleet/protocol/#struct.WrappedSecret) was malformed.
- **`Payload`** - The unwrapped bytes were not a valid JSON `{field: value}` map.

## Functions

### <span id="fn.secret_aad"></span><code style="font-size: 0.85em">fn secret_​aad</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn secret_aad(execution_id: &str, name: &str) -> Vec<u8>
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

### <span id="fn.resolve_and_wrap_secrets"></span><code style="font-size: 0.85em">fn resolve_​and_​wrap_​secrets</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn resolve_and_wrap_secrets(
    resolver: &dyn SecretResolver,
    names: &[String],
    execution_id: &str,
    recipient_public_key: &[u8],
) -> Result<Vec<WrappedSecret>, FleetSecretError>
```

Server side: resolve each named secret (grant-checked by `resolver`) and HPKE-wrap it to `recipient_public_key`, bound to `execution_id`.

`resolver` should be the tenant + grant scoped resolver from T-0858/T-0860
(the wrap step adds no authorization of its own — it trusts the resolver's
gate). Returns one [`WrappedSecret`](../../../../rust/cloacina/fleet/protocol/#struct.WrappedSecret) per name; the plaintext exists only
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

### <span id="fn.wrap_field_map"></span><code style="font-size: 0.85em">fn wrap_​field_​map</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn wrap_field_map(
    name: &str,
    fields: &BTreeMap<String, String>,
    execution_id: &str,
    recipient_public_key: &[u8],
) -> Result<WrappedSecret, FleetSecretError>
```

Wrap an already-resolved `{field: value}` map into a [`WrappedSecret`](../../../../rust/cloacina/fleet/protocol/#struct.WrappedSecret).

Split out from [`resolve_and_wrap_secrets`](../../../../rust/cloacina/security/fleet_secret/#fn.resolve_and_wrap_secrets) so callers holding pre-resolved
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

### <span id="fn.secret_ref_names"></span><code style="font-size: 0.85em">fn secret_​ref_​names</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn secret_ref_names(context: &serde_json::Value) -> Vec<String>
```

Extract the concrete secret NAMES a fire context references via the T-0859 `{"$secret": name}` alias map (stored under [`cloacina_workflow::secret::SECRET_REFS_KEY`](../../../../rust/cloacina-workflow/secret/#const.SECRET_REFS_KEY)).

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

### <span id="fn.decode_pool_public_key"></span><code style="font-size: 0.85em">fn decode_​pool_​public_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn decode_pool_public_key(
    entry: &EphemeralKeyEntry,
) -> Result<Vec<u8>, FleetSecretError>
```

Decode an [`EphemeralKeyEntry`](../../../../rust/cloacina/fleet/protocol/#struct.EphemeralKeyEntry)'s base64 public key into raw X25519 bytes.

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
