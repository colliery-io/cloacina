---
title: "cloacina::security::package_signer"
description: "Package signing and signature storage."
sidebar:
  order: 211
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Package signing and signature storage.

This module provides:
- [`PackageSigner`] trait for signing packages
- [`DbPackageSigner`] database-backed implementation
- [`DetachedSignature`] format for standalone signature files

## Structs

### <span id="struct.PackageSignatureInfo"></span>`struct PackageSignatureInfo`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

A package signature with all metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `String` | SHA256 hex hash of the package binary |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the signing key |
| `signature` | `Vec < u8 >` | 64-byte Ed25519 signature |
| `signed_at` | `UniversalTimestamp` | When the package was signed |



### <span id="struct.DetachedSignature"></span>`struct DetachedSignature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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

##### <span id="DetachedSignature.from_signature_info"></span>`from_signature_info`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="DetachedSignature.from_json"></span>`from_json`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="DetachedSignature.to_json"></span>`to_json`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="DetachedSignature.signature_bytes"></span>`signature_bytes`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="DetachedSignature.write_to_file"></span>`write_to_file`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="DetachedSignature.read_from_file"></span>`read_from_file`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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





### <span id="struct.DbPackageSigner"></span>`struct DbPackageSigner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Clone`

Database-backed package signer implementation.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |

#### Methods

##### <span id="DbPackageSigner.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="DbPackageSigner.compute_file_hash"></span>`compute_file_hash`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


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



##### <span id="DbPackageSigner.compute_data_hash"></span>`compute_data_hash`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


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



##### <span id="DbPackageSigner.to_signature_info"></span>`to_signature_info`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


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

### <span id="enum.PackageSignError"></span>`enum PackageSignError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



