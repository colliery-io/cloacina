---
title: "cloacina::security::key_manager"
description: "Key manager trait and associated types."
editUrl: false
sidebar:
  order: 192
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Key manager trait and associated types.

The [`KeyManager`](../../../../rust/cloacina/security/key_manager/#trait.KeyManager) trait defines the interface for managing Ed25519 signing keys,
trusted public keys, and trust relationships between organizations.

## Structs

### <span id="struct.SigningKeyInfo"></span><code style="font-size: 0.85em">struct SigningKeyInfo</code>

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
| `public_key` | `Vec<u8>` | 32-byte Ed25519 public key |
| `created_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option<UniversalTimestamp>` |  |

#### Methods

##### <span id="SigningKeyInfo.is_active"></span><code>is_​active</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_active(&self) -> bool
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

### <span id="struct.TrustedKeyInfo"></span><code style="font-size: 0.85em">struct TrustedKeyInfo</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Information about a trusted public key for verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key` | `Vec<u8>` | 32-byte Ed25519 public key |
| `key_name` | `Option<String>` | Optional human-readable name |
| `trusted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option<UniversalTimestamp>` |  |

#### Methods

##### <span id="TrustedKeyInfo.is_active"></span><code>is_​active</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_active(&self) -> bool
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

### <span id="struct.PublicKeyExport"></span><code style="font-size: 0.85em">struct PublicKeyExport</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Public key export in multiple formats.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key_pem` | `String` | PEM-encoded public key (Ed25519 SubjectPublicKeyInfo format) |
| `public_key_raw` | `Vec<u8>` | Raw 32-byte Ed25519 public key |

## Enums

### <span id="enum.KeyError"></span><code style="font-size: 0.85em">enum KeyError</code>

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

## Traits

### <span id="trait.KeyManager"></span><code style="font-size: 0.85em">trait KeyManager</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Trait for managing signing keys, trusted keys, and trust relationships.

Implementations must be thread-safe (`Send + Sync`) and should NOT cache
results to ensure key revocations take effect immediately.

#### Methods

##### <span id="KeyManager.create_signing_key"></span><code>create_​signing_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create_signing_key(
    &self,
    org_id: UniversalUuid,
    name: &str,
    master_key: &[u8],
) -> Result<SigningKeyInfo, KeyError>
```

Generate a new Ed25519 signing keypair and store it encrypted in the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `org_id` | `UniversalUuid` | Organization that owns this key |
| `name` | `&str` | Human-readable name for the key (must be unique per org) |
| `master_key` | `&[u8]` | 32-byte AES-256 key for encrypting the private key |

**Returns:**

Information about the created signing key.

<details>
<summary>Source</summary>

```rust
    async fn create_signing_key(
```

</details>

##### <span id="KeyManager.get_signing_key_info"></span><code>get_​signing_​key_​info</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_signing_key_info(
    &self,
    key_id: UniversalUuid,
) -> Result<SigningKeyInfo, KeyError>
```

Get information about a signing key (without the private key).

<details>
<summary>Source</summary>

```rust
    async fn get_signing_key_info(&self, key_id: UniversalUuid)
```

</details>

##### <span id="KeyManager.get_signing_key"></span><code>get_​signing_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_signing_key(
    &self,
    key_id: UniversalUuid,
    master_key: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), KeyError>
```

Get the decrypted signing key for signing operations.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key_id` | `UniversalUuid` | The signing key ID |
| `master_key` | `&[u8]` | 32-byte AES-256 key for decrypting the private key |

**Returns:**

Tuple of (public_key, private_key) as raw bytes.

<details>
<summary>Source</summary>

```rust
    async fn get_signing_key(
```

</details>

##### <span id="KeyManager.export_public_key"></span><code>export_​public_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn export_public_key(
    &self,
    key_id: UniversalUuid,
) -> Result<PublicKeyExport, KeyError>
```

Export a public key in multiple formats for distribution.

<details>
<summary>Source</summary>

```rust
    async fn export_public_key(&self, key_id: UniversalUuid) -> Result<PublicKeyExport, KeyError>;
```

</details>

##### <span id="KeyManager.trust_public_key"></span><code>trust_​public_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn trust_public_key(
    &self,
    org_id: UniversalUuid,
    public_key: &[u8],
    name: Option<&str>,
) -> Result<TrustedKeyInfo, KeyError>
```

Import an external public key to trust for verification.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `org_id` | `UniversalUuid` | Organization that will trust this key |
| `public_key` | `&[u8]` | 32-byte Ed25519 public key |
| `name` | `Option<&str>` | Optional human-readable name |

<details>
<summary>Source</summary>

```rust
    async fn trust_public_key(
```

</details>

##### <span id="KeyManager.trust_public_key_pem"></span><code>trust_​public_​key_​pem</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn trust_public_key_pem(
    &self,
    org_id: UniversalUuid,
    pem: &str,
    name: Option<&str>,
) -> Result<TrustedKeyInfo, KeyError>
```

Import a public key from PEM format and add it to trusted keys.

<details>
<summary>Source</summary>

```rust
    async fn trust_public_key_pem(
```

</details>

##### <span id="KeyManager.revoke_signing_key"></span><code>revoke_​signing_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn revoke_signing_key(
    &self) -> Result<(,
    key_id: UniversalUuid) -> Result<(,
) -> Result<(), KeyError>
```

Revoke a signing key (prevents future signing operations).

<details>
<summary>Source</summary>

```rust
    async fn revoke_signing_key(&self, key_id: UniversalUuid) -> Result<(), KeyError>;
```

</details>

##### <span id="KeyManager.revoke_trusted_key"></span><code>revoke_​trusted_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn revoke_trusted_key(
    &self) -> Result<(,
    key_id: UniversalUuid) -> Result<(,
) -> Result<(), KeyError>
```

Revoke a trusted key (prevents future verification with this key).

<details>
<summary>Source</summary>

```rust
    async fn revoke_trusted_key(&self, key_id: UniversalUuid) -> Result<(), KeyError>;
```

</details>

##### <span id="KeyManager.grant_trust"></span><code>grant_​trust</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn grant_trust(
    &self,
    parent_org: UniversalUuid,
    child_org: UniversalUuid,
) -> Result<(), KeyError>
```

Grant trust from a parent organization to a child organization.

When trust is granted, the parent org implicitly trusts all keys
that are trusted by the child org.

<details>
<summary>Source</summary>

```rust
    async fn grant_trust(
```

</details>

##### <span id="KeyManager.revoke_trust"></span><code>revoke_​trust</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn revoke_trust(
    &self,
    parent_org: UniversalUuid,
    child_org: UniversalUuid,
) -> Result<(), KeyError>
```

Revoke trust grant between organizations.

<details>
<summary>Source</summary>

```rust
    async fn revoke_trust(
```

</details>

##### <span id="KeyManager.list_signing_keys"></span><code>list_​signing_​keys</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_signing_keys(
    &self,
    org_id: UniversalUuid,
) -> Result<Vec<SigningKeyInfo>, KeyError>
```

List all signing keys for an organization.

<details>
<summary>Source</summary>

```rust
    async fn list_signing_keys(
```

</details>

##### <span id="KeyManager.list_trusted_keys"></span><code>list_​trusted_​keys</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_trusted_keys(
    &self,
    org_id: UniversalUuid,
) -> Result<Vec<TrustedKeyInfo>, KeyError>
```

List all trusted keys for an organization, including inherited keys via ACLs.

<details>
<summary>Source</summary>

```rust
    async fn list_trusted_keys(
```

</details>

##### <span id="KeyManager.find_trusted_key"></span><code>find_​trusted_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_trusted_key(
    &self,
    org_id: UniversalUuid,
    fingerprint: &str,
) -> Result<Option<TrustedKeyInfo>, KeyError>
```

Find a trusted key by fingerprint for verification.

Searches both directly trusted keys and inherited keys via ACLs.

<details>
<summary>Source</summary>

```rust
    async fn find_trusted_key(
```

</details>
