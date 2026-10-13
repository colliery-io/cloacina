---
title: "cloacina::models::trusted_key"
description: "Domain models for trusted public keys."
editUrl: false
sidebar:
  order: 144
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Domain models for trusted public keys.

Trusted keys are public keys that an organization trusts for verifying
package signatures. They can be imported from external sources or
derived from the organization's own signing keys.

## Structs

### <span id="struct.TrustedKey"></span><code style="font-size: 0.85em">struct TrustedKey</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a trusted public key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key` | `Vec<u8>` | Ed25519 public key (32 bytes) |
| `key_name` | `Option<String>` | Optional human-readable name for the key |
| `trusted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option<UniversalTimestamp>` | None if active, Some if revoked |

#### Methods

##### <span id="TrustedKey.is_active"></span><code>is_​active</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_active(&self) -> bool
```

Check if this key is currently trusted (not revoked)

<details>
<summary>Source</summary>

```rust
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
```

</details>

##### <span id="TrustedKey.is_revoked"></span><code>is_​revoked</code>

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

### <span id="struct.NewTrustedKey"></span><code style="font-size: 0.85em">struct NewTrustedKey</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new trusted key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `org_id` | `UniversalUuid` |  |
| `key_fingerprint` | `String` |  |
| `public_key` | `Vec<u8>` |  |
| `key_name` | `Option<String>` |  |

#### Methods

##### <span id="NewTrustedKey.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    org_id: UniversalUuid,
    key_fingerprint: String,
    public_key: Vec<u8>,
    key_name: Option<String>,
) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(
        org_id: UniversalUuid,
        key_fingerprint: String,
        public_key: Vec<u8>,
        key_name: Option<String>,
    ) -> Self {
        Self {
            org_id,
            key_fingerprint,
            public_key,
            key_name,
        }
    }
```

</details>

##### <span id="NewTrustedKey.from_signing_key"></span><code>from_​signing_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_signing_key(
    org_id: UniversalUuid,
    key_fingerprint: String,
    public_key: Vec<u8>,
    key_name: String,
) -> Self
```

Create a trusted key from a signing key's public key.

<details>
<summary>Source</summary>

```rust
    pub fn from_signing_key(
        org_id: UniversalUuid,
        key_fingerprint: String,
        public_key: Vec<u8>,
        key_name: String,
    ) -> Self {
        Self {
            org_id,
            key_fingerprint,
            public_key,
            key_name: Some(key_name),
        }
    }
```

</details>
