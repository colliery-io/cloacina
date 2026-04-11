# cloacina::security::key_manager <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Key manager trait and associated types.

The [`KeyManager`] trait defines the interface for managing Ed25519 signing keys,
trusted public keys, and trust relationships between organizations.

## Structs

### `cloacina::security::key_manager::SigningKeyInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

##### `is_active` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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





### `cloacina::security::key_manager::TrustedKeyInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

##### `is_active` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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





### `cloacina::security::key_manager::PublicKeyExport`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Public key export in multiple formats.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key_pem` | `String` | PEM-encoded public key (Ed25519 SubjectPublicKeyInfo format) |
| `public_key_raw` | `Vec < u8 >` | Raw 32-byte Ed25519 public key |



## Enums

### `cloacina::security::key_manager::KeyError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
