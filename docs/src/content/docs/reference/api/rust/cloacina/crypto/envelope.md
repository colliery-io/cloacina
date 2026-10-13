---
title: "cloacina::crypto::envelope"
description: "Per-execution HPKE envelope wrap for fleet secret resolution (CLOACI-T-0861)."
editUrl: false
sidebar:
  order: 59
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Per-execution HPKE envelope wrap for fleet secret resolution (CLOACI-T-0861).

Implements the wrap-to-ephemeral-public-key step of the fleet secret
resolution design (I-0133 D-2/D-5/D-6, NFR-003): a remote agent generates a
fresh ephemeral X25519 keypair per task claim and advertises the public key;
the server resolves the at-rest secret and HPKE-wraps (RFC 9180) the value to
that public key; the agent unwraps with its ephemeral private key into the
in-memory `Secrets` accessor and never persists the plaintext.

**Suite**

A single fixed RFC 9180 ciphersuite is used (crypto agility is deferred — the
wire format could carry a suite id later):

- **KEM:** X25519-HKDF-SHA256 (`0x0020`)
- **KDF:** HKDF-SHA256 (`0x0001`)
- **AEAD:** ChaCha20Poly1305 (`0x0003`)

**Binding**

Each wrap is bound to one recipient by construction (the ephemeral public
key), and additionally to an application context via the HPKE `info` string
and per-message AEAD associated data (`aad`). Callers SHOULD pass an `aad`
that names the execution + secret so a captured ciphertext cannot be replayed
against a different execution even to the same key.

## Structs

### <span id="struct.EphemeralPrivateKey"></span><code style="font-size: 0.85em">struct EphemeralPrivateKey</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

The recipient (agent) half of a per-execution keypair.

Holds the ephemeral X25519 private key. Never serialized to the wire, never
persisted. Only the paired public key (`EphemeralKeypair::public_key_bytes`)
leaves the agent.

### <span id="struct.EphemeralKeypair"></span><code style="font-size: 0.85em">struct EphemeralKeypair</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`

A freshly generated ephemeral keypair for one task claim/execution.

The agent keeps `private` and sends `public_key_bytes` to the server. The
server wraps secrets to `public_key_bytes`; the agent unwraps with `private`.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `private` | `EphemeralPrivateKey` | The private half — kept by the agent, never leaves the process. |
| `public_key_bytes` | `Vec<u8>` | The serialized X25519 public key — advertised to the server so it can wrap secrets to this execution. |

## Enums

### <span id="enum.EnvelopeError"></span><code style="font-size: 0.85em">enum EnvelopeError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors from envelope wrap/unwrap.

#### Variants

- **`InvalidPublicKey`** - The recipient public key bytes were not a valid X25519 public key.
- **`InvalidEncappedKey`** - The encapsulated key bytes (`enc`) were malformed.
- **`Wrap`** - HPKE seal (wrap) failed.
- **`Unwrap`** - HPKE open (unwrap) failed — wrong key, tampered ciphertext, or bad aad.

## Functions

### <span id="fn.generate_ephemeral_keypair"></span><code style="font-size: 0.85em">fn generate_​ephemeral_​keypair</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn generate_ephemeral_keypair() -> EphemeralKeypair
```

Generate a fresh ephemeral X25519 keypair (D-5: per task claim).

Keygen is microseconds; call it once per claim so a leaked key exposes at
most one execution's secrets.

<details>
<summary>Source</summary>

```rust
pub fn generate_ephemeral_keypair() -> EphemeralKeypair {
    let (private, public) = Kem::gen_keypair();
    EphemeralKeypair {
        private: EphemeralPrivateKey(private),
        public_key_bytes: public.to_bytes().to_vec(),
    }
}
```

</details>

### <span id="fn.wrap"></span><code style="font-size: 0.85em">fn wrap</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn wrap(
    recipient_public_key: &[u8],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), EnvelopeError>
```

HPKE-wrap `plaintext` to `recipient_public_key` (single-shot seal).

Returns `(enc, ciphertext)` where `enc` is the HPKE encapsulated key and
`ciphertext` is the AEAD-sealed payload. Both are needed to unwrap. `aad` is
authenticated but not encrypted — bind it to the execution + secret name.

Only `(enc, ciphertext)` should cross the wire; the plaintext never does.

<details>
<summary>Source</summary>

```rust
pub fn wrap(
    recipient_public_key: &[u8],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), EnvelopeError> {
    let pk_recip = <Kem as KemTrait>::PublicKey::from_bytes(recipient_public_key)
        .map_err(|_| EnvelopeError::InvalidPublicKey)?;

    let (encapped, ciphertext) = hpke::single_shot_seal::<Aead, Kdf, Kem>(
        &OpModeS::Base,
        &pk_recip,
        HPKE_INFO,
        plaintext,
        aad,
    )
    .map_err(|e| EnvelopeError::Wrap(e.to_string()))?;

    Ok((encapped.to_bytes().to_vec(), ciphertext))
}
```

</details>

### <span id="fn.unwrap"></span><code style="font-size: 0.85em">fn unwrap</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn unwrap(
    recipient_private_key: &EphemeralPrivateKey,
    enc: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, EnvelopeError>
```

HPKE-unwrap a `(enc, ciphertext)` pair with the recipient private key.

`aad` MUST match the value passed to [`wrap`](../../../../rust/cloacina/crypto/envelope/#fn.wrap) exactly, or the AEAD open
fails. Fails closed on a wrong key, tampered ciphertext, or mismatched aad.

<details>
<summary>Source</summary>

```rust
pub fn unwrap(
    recipient_private_key: &EphemeralPrivateKey,
    enc: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, EnvelopeError> {
    let encapped = <Kem as KemTrait>::EncappedKey::from_bytes(enc)
        .map_err(|_| EnvelopeError::InvalidEncappedKey)?;

    hpke::single_shot_open::<Aead, Kdf, Kem>(
        &OpModeR::Base,
        &recipient_private_key.0,
        &encapped,
        HPKE_INFO,
        ciphertext,
        aad,
    )
    .map_err(|e| EnvelopeError::Unwrap(e.to_string()))
}
```

</details>
