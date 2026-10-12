---
title: "cloacina::security::key_manager"
description: "Key manager trait and associated types."
sidebar:
  order: 210
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Key manager trait and associated types.

The [`KeyManager`] trait defines the interface for managing Ed25519 signing keys,
trusted public keys, and trust relationships between organizations.

## Structs

### <span id="struct.SigningKeyInfo"></span>`struct SigningKeyInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Information about a signing key (excludes private key material).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key` | `Vec < u8 >` | 32-byte Ed25519 public key |
| `created_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` |  |

#### Methods

##### <span id="SigningKeyInfo.is_active"></span>`is_active`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_active (& self) -> bool
```

Check if this key is currently active (not revoked).

<details>
<summary>Source</summary>

```rust
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
```

</details>





### <span id="struct.TrustedKeyInfo"></span>`struct TrustedKeyInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Information about a trusted public key for verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key` | `Vec < u8 >` | 32-byte Ed25519 public key |
| `key_name` | `Option < String >` | Optional human-readable name |
| `trusted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` |  |

#### Methods

##### <span id="TrustedKeyInfo.is_active"></span>`is_active`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn is_active (& self) -> bool
```

Check if this key is currently trusted (not revoked).

<details>
<summary>Source</summary>

```rust
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
```

</details>





### <span id="struct.PublicKeyExport"></span>`struct PublicKeyExport`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

Public key export in multiple formats.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key_pem` | `String` | PEM-encoded public key (Ed25519 SubjectPublicKeyInfo format) |
| `public_key_raw` | `Vec < u8 >` | Raw 32-byte Ed25519 public key |



## Enums

### <span id="enum.KeyError"></span>`enum KeyError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Errors that can occur during key management operations.

#### Variants

- **`NotFound`**
- **`Revoked`**
- **`DuplicateName`**
- **`InvalidFormat`**
- **`InvalidPem`**
- **`Encryption`**
- **`Decryption`**
- **`TrustAlreadyExists`**
- **`TrustNotFound`**
- **`Database`**



