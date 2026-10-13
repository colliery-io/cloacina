---
title: "cloacina::models::signing_key"
description: "Domain models for Ed25519 signing keys."
editUrl: false
sidebar:
  order: 141
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Domain models for Ed25519 signing keys.

Signing keys are used to cryptographically sign workflow packages.
Private keys are stored encrypted at rest using AES-256-GCM.

## Structs

### <span id="struct.SigningKey"></span><code style="font-size: 0.85em">struct SigningKey</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a signing key.

The private key is stored encrypted - use the crypto module to decrypt.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `encrypted_private_key` | `Vec<u8>` | AES-256-GCM encrypted Ed25519 private key (nonce \|\| ciphertext \|\| tag) |
| `public_key` | `Vec<u8>` | Ed25519 public key (32 bytes) |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `created_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option<UniversalTimestamp>` | None if active, Some if revoked |

#### Methods

##### <span id="SigningKey.is_active"></span><code>is_​active</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_active(&self) -> bool
```

Check if this key is currently active (not revoked)

<details>
<summary>Source</summary>

```rust
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
```

</details>

##### <span id="SigningKey.is_revoked"></span><code>is_​revoked</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_revoked(&self) -> bool
```

Check if this key has been revoked

<details>
<summary>Source</summary>

```rust
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
```

</details>

### <span id="struct.NewSigningKey"></span><code style="font-size: 0.85em">struct NewSigningKey</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new signing key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `encrypted_private_key` | `Vec<u8>` |  |
| `public_key` | `Vec<u8>` |  |
| `key_fingerprint` | `String` |  |

#### Methods

##### <span id="NewSigningKey.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    org_id: UniversalUuid,
    key_name: String,
    encrypted_private_key: Vec<u8>,
    public_key: Vec<u8>,
    key_fingerprint: String,
) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        org_id: UniversalUuid,
        key_name: String,
        encrypted_private_key: Vec<u8>,
        public_key: Vec<u8>,
        key_fingerprint: String,
    ) -> Self {
        Self {
            org_id,
            key_name,
            encrypted_private_key,
            public_key,
            key_fingerprint,
        }
    }
```

</details>

### <span id="struct.SigningKeyInfo"></span><code style="font-size: 0.85em">struct SigningKeyInfo</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Information about a signing key (without the private key material).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `key_fingerprint` | `String` |  |
| `created_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option<UniversalTimestamp>` |  |
