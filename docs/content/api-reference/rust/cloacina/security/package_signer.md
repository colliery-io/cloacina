# cloacina::security::package_signer <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Package signing and signature storage.

This module provides:
- [`PackageSigner`] trait for signing packages
- [`DbPackageSigner`] database-backed implementation
- [`DetachedSignature`] format for standalone signature files

## Structs

### `cloacina::security::package_signer::PackageSignatureInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

A package signature with all metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `String` | SHA256 hex hash of the package binary |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the signing key |
| `signature` | `Vec < u8 >` | 64-byte Ed25519 signature |
| `signed_at` | `UniversalTimestamp` | When the package was signed |



### `cloacina::security::package_signer::DetachedSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Detached signature file format.

This is a JSON-serializable format for standalone `.sig` files
that can be distributed alongside packages.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `version` | `u32` | Format version (currently 1) |
| `algorithm` | `String` | Signature algorithm (currently "ed25519") |
| `package_hash` | `String` | SHA256 hex hash of the package binary |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the signing key |
| `signature` | `String` | Base64-encoded 64-byte signature |
| `signed_at` | `String` | ISO8601 timestamp of when the signature was created |

#### Methods

##### `from_signature_info` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_signature_info (info : & PackageSignatureInfo) -> Self
```

Create a detached signature from signature info.

<details>
<summary>Source</summary>

```rust
    pub fn from_signature_info(info: &PackageSignatureInfo) -> Self {
        Self {
            version: Self::VERSION,
            algorithm: Self::ALGORITHM.to_string(),
            package_hash: info.package_hash.clone(),
            key_fingerprint: info.key_fingerprint.clone(),
            signature: BASE64.encode(&info.signature),
            signed_at: info.signed_at.to_rfc3339(),
        }
    }
```

</details>



##### `from_json` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_json (json : & str) -> Result < Self , PackageSignError >
```

Parse a detached signature from JSON.

<details>
<summary>Source</summary>

```rust
    pub fn from_json(json: &str) -> Result<Self, PackageSignError> {
        serde_json::from_str(json)
            .map_err(|e| PackageSignError::InvalidSignatureFile(e.to_string()))
    }
```

</details>



##### `to_json` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn to_json (& self) -> Result < String , PackageSignError >
```

Serialize to JSON.

<details>
<summary>Source</summary>

```rust
    pub fn to_json(&self) -> Result<String, PackageSignError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| PackageSignError::InvalidSignatureFile(e.to_string()))
    }
```

</details>



##### `signature_bytes` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn signature_bytes (& self) -> Result < Vec < u8 > , PackageSignError >
```

Get the raw signature bytes.

<details>
<summary>Source</summary>

```rust
    pub fn signature_bytes(&self) -> Result<Vec<u8>, PackageSignError> {
        BASE64
            .decode(&self.signature)
            .map_err(|e| PackageSignError::InvalidSignatureFile(e.to_string()))
    }
```

</details>



##### `write_to_file` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn write_to_file (& self , path : & Path) -> Result < () , PackageSignError >
```

Write the detached signature to a file.

<details>
<summary>Source</summary>

```rust
    pub fn write_to_file(&self, path: &Path) -> Result<(), PackageSignError> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }
```

</details>



##### `read_from_file` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn read_from_file (path : & Path) -> Result < Self , PackageSignError >
```

Read a detached signature from a file.

<details>
<summary>Source</summary>

```rust
    pub fn read_from_file(path: &Path) -> Result<Self, PackageSignError> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
```

</details>





### `cloacina::security::package_signer::DbPackageSigner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Database-backed package signer implementation.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL) -> Self
```

Create a new database-backed package signer.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }
```

</details>



##### `compute_file_hash` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn compute_file_hash (path : & Path) -> Result < String , PackageSignError >
```

Compute the SHA256 hash of a file.

<details>
<summary>Source</summary>

```rust
    fn compute_file_hash(path: &Path) -> Result<String, PackageSignError> {
        let data = std::fs::read(path)?;
        Self::compute_data_hash(&data)
    }
```

</details>



##### `compute_data_hash` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn compute_data_hash (data : & [u8]) -> Result < String , PackageSignError >
```

Compute the SHA256 hash of data.

<details>
<summary>Source</summary>

```rust
    fn compute_data_hash(data: &[u8]) -> Result<String, PackageSignError> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(hex::encode(hasher.finalize()))
    }
```

</details>



##### `to_signature_info` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn to_signature_info (sig : UnifiedPackageSignature) -> PackageSignatureInfo
```

Convert database model to SignatureInfo.

<details>
<summary>Source</summary>

```rust
    fn to_signature_info(sig: UnifiedPackageSignature) -> PackageSignatureInfo {
        PackageSignatureInfo {
            package_hash: sig.package_hash,
            key_fingerprint: sig.key_fingerprint,
            signature: sig.signature.into_inner(),
            signed_at: sig.signed_at,
        }
    }
```

</details>





## Enums

### `cloacina::security::package_signer::PackageSignError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during package signing operations.

#### Variants

- **`FileReadError`**
- **`SigningFailed`**
- **`KeyNotFound`**
- **`KeyRevoked`**
- **`Database`**
- **`SignatureNotFound`**
- **`VerificationFailed`**
- **`InvalidSignatureFile`**
