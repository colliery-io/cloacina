# cloacina::models::signing_key <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Domain models for Ed25519 signing keys.

Signing keys are used to cryptographically sign workflow packages.
Private keys are stored encrypted at rest using AES-256-GCM.

## Structs

### `cloacina::models::signing_key::SigningKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a signing key.

The private key is stored encrypted - use the crypto module to decrypt.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `encrypted_private_key` | `Vec < u8 >` | AES-256-GCM encrypted Ed25519 private key (nonce || ciphertext || tag) |
| `public_key` | `Vec < u8 >` | Ed25519 public key (32 bytes) |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `created_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` | None if active, Some if revoked |

#### Methods

##### `is_active` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_active (& self) -> bool
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



##### `is_revoked` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_revoked (& self) -> bool
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





### `cloacina::models::signing_key::NewSigningKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new signing key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `org_id` | `UniversalUuid` |  |
| `key_name` | `String` |  |
| `encrypted_private_key` | `Vec < u8 >` |  |
| `public_key` | `Vec < u8 >` |  |
| `key_fingerprint` | `String` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (org_id : UniversalUuid , key_name : String , encrypted_private_key : Vec < u8 > , public_key : Vec < u8 > , key_fingerprint : String ,) -> Self
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





### `cloacina::models::signing_key::SigningKeyInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
| `revoked_at` | `Option < UniversalTimestamp >` |  |
