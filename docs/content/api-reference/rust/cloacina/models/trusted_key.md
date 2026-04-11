# cloacina::models::trusted_key <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Domain models for trusted public keys.

Trusted keys are public keys that an organization trusts for verifying
package signatures. They can be imported from external sources or
derived from the organization's own signing keys.

## Structs

### `cloacina::models::trusted_key::TrustedKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a trusted public key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `org_id` | `UniversalUuid` |  |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the public key |
| `public_key` | `Vec < u8 >` | Ed25519 public key (32 bytes) |
| `key_name` | `Option < String >` | Optional human-readable name for the key |
| `trusted_at` | `UniversalTimestamp` |  |
| `revoked_at` | `Option < UniversalTimestamp >` | None if active, Some if revoked |

#### Methods

##### `is_active` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn is_active (& self) -> bool
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





### `cloacina::models::trusted_key::NewTrustedKey`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new trusted key.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `org_id` | `UniversalUuid` |  |
| `key_fingerprint` | `String` |  |
| `public_key` | `Vec < u8 >` |  |
| `key_name` | `Option < String >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (org_id : UniversalUuid , key_fingerprint : String , public_key : Vec < u8 > , key_name : Option < String > ,) -> Self
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



##### `from_signing_key` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_signing_key (org_id : UniversalUuid , key_fingerprint : String , public_key : Vec < u8 > , key_name : String ,) -> Self
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
