# cloacina::models::package_signature <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Domain models for package signatures.

Package signatures provide cryptographic proof that a workflow package
was signed by a specific key. Signatures are Ed25519 signatures over
the SHA256 hash of the package binary.

## Structs

### `cloacina::models::package_signature::PackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Domain model for a package signature.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` |  |
| `package_hash` | `String` | SHA256 hex hash of the package binary |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the signing key |
| `signature` | `Vec < u8 >` | Ed25519 signature (64 bytes) |
| `signed_at` | `UniversalTimestamp` |  |



### `cloacina::models::package_signature::NewPackageSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Model for creating a new package signature.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `String` |  |
| `key_fingerprint` | `String` |  |
| `signature` | `Vec < u8 >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (package_hash : String , key_fingerprint : String , signature : Vec < u8 >) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(package_hash: String, key_fingerprint: String, signature: Vec<u8>) -> Self {
        Self {
            package_hash,
            key_fingerprint,
            signature,
        }
    }
```

</details>





### `cloacina::models::package_signature::SignatureVerification`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Result of signature verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `is_valid` | `bool` | Whether the signature is valid |
| `signer_fingerprint` | `String` | The fingerprint of the key that signed the package |
| `signed_at` | `UniversalTimestamp` | When the package was signed |
| `signer_name` | `Option < String >` | Name of the signing key (if known) |
