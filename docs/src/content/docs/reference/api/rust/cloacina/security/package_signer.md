---
title: "cloacina::security::package_signer"
description: "Package signing and signature storage."
editUrl: false
sidebar:
  order: 193
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Package signing and signature storage.

This module provides:
- [`PackageSigner`](../../../../rust/cloacina/security/package_signer/#trait.PackageSigner) trait for signing packages
- [`DbPackageSigner`](../../../../rust/cloacina/security/package_signer/#struct.DbPackageSigner) database-backed implementation
- [`DetachedSignature`](../../../../rust/cloacina/security/package_signer/#struct.DetachedSignature) format for standalone signature files

## Structs

### <span id="struct.PackageSignatureInfo"></span><code style="font-size: 0.85em">struct PackageSignatureInfo</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

A package signature with all metadata.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `String` | SHA256 hex hash of the package binary |
| `key_fingerprint` | `String` | SHA256 hex fingerprint of the signing key |
| `signature` | `Vec<u8>` | 64-byte Ed25519 signature |
| `signed_at` | `UniversalTimestamp` | When the package was signed |

### <span id="struct.DetachedSignature"></span><code style="font-size: 0.85em">struct DetachedSignature</code>

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

##### <span id="DetachedSignature.from_signature_info"></span><code>from_​signature_​info</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_signature_info(info: &PackageSignatureInfo) -> Self
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

##### <span id="DetachedSignature.from_json"></span><code>from_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_json(json: &str) -> Result<Self, PackageSignError>
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

##### <span id="DetachedSignature.to_json"></span><code>to_​json</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn to_json(&self) -> Result<String, PackageSignError>
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

##### <span id="DetachedSignature.signature_bytes"></span><code>signature_​bytes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn signature_bytes(&self) -> Result<Vec<u8>, PackageSignError>
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

##### <span id="DetachedSignature.write_to_file"></span><code>write_​to_​file</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn write_to_file(&self, path: &Path) -> Result<(), PackageSignError>
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

##### <span id="DetachedSignature.read_from_file"></span><code>read_​from_​file</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn read_from_file(path: &Path) -> Result<Self, PackageSignError>
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

### <span id="struct.DbPackageSigner"></span><code style="font-size: 0.85em">struct DbPackageSigner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Clone`

Database-backed package signer implementation.

#### Methods

##### <span id="DbPackageSigner.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: DAL) -> Self
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

## Enums

### <span id="enum.PackageSignError"></span><code style="font-size: 0.85em">enum PackageSignError</code>

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

## Traits

### <span id="trait.PackageSigner"></span><code style="font-size: 0.85em">trait PackageSigner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Trait for signing packages and managing signatures.

#### Methods

##### <span id="PackageSigner.sign_package_with_db_key"></span><code>sign_​package_​with_​db_​key</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn sign_package_with_db_key(
    &self,
    package_path: &Path,
    key_id: UniversalUuid,
    master_key: &[u8],
    store_signature: bool,
) -> Result<PackageSignatureInfo, PackageSignError>
```

Sign a package file using a key from the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_path` | `&Path` | Path to the package file to sign |
| `key_id` | `UniversalUuid` | ID of the signing key in the database |
| `master_key` | `&[u8]` | AES-256 key for decrypting the signing key |
| `store_signature` | `bool` | Whether to store the signature in the database |

**Returns:**

The signature information.

<details>
<summary>Source</summary>

```rust
    async fn sign_package_with_db_key(
```

</details>

##### <span id="PackageSigner.sign_package_with_raw_key"></span><code>sign_​package_​with_​raw_​key</code>

```rust
fn sign_package_with_raw_key(
    &self,
    package_path: &Path,
    private_key: &[u8],
    public_key: &[u8],
) -> Result<PackageSignatureInfo, PackageSignError>
```

Sign a package file using a raw key (for offline signing).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_path` | `&Path` | Path to the package file to sign |
| `private_key` | `&[u8]` | 32-byte Ed25519 private key |
| `public_key` | `&[u8]` | 32-byte Ed25519 public key (for fingerprint) |

**Returns:**

The signature information.

<details>
<summary>Source</summary>

```rust
    fn sign_package_with_raw_key(
```

</details>

##### <span id="PackageSigner.sign_package_data"></span><code>sign_​package_​data</code>

```rust
fn sign_package_data(
    &self,
    package_data: &[u8],
    private_key: &[u8],
    public_key: &[u8],
) -> Result<PackageSignatureInfo, PackageSignError>
```

Sign package data directly (already in memory).

<details>
<summary>Source</summary>

```rust
    fn sign_package_data(
```

</details>

##### <span id="PackageSigner.store_signature"></span><code>store_​signature</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn store_signature(
    &self,
    signature: &PackageSignatureInfo,
) -> Result<UniversalUuid, PackageSignError>
```

Store a signature in the database.

<details>
<summary>Source</summary>

```rust
    async fn store_signature(
```

</details>

##### <span id="PackageSigner.find_signature"></span><code>find_​signature</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_signature(
    &self,
    package_hash: &str,
) -> Result<Option<PackageSignatureInfo>, PackageSignError>
```

Find a signature by package hash.

<details>
<summary>Source</summary>

```rust
    async fn find_signature(
```

</details>

##### <span id="PackageSigner.find_signatures"></span><code>find_​signatures</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn find_signatures(
    &self,
    package_hash: &str,
) -> Result<Vec<PackageSignatureInfo>, PackageSignError>
```

Find all signatures for a package hash.

<details>
<summary>Source</summary>

```rust
    async fn find_signatures(
```

</details>

##### <span id="PackageSigner.verify_package"></span><code>verify_​package</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn verify_package(
    &self,
    package_path: &Path,
    org_id: UniversalUuid,
) -> Result<PackageSignatureInfo, PackageSignError>
```

Verify a package against a stored signature.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_path` | `&Path` | Path to the package file to verify |
| `org_id` | `UniversalUuid` | Organization to check trusted keys for |

**Returns:**

The signature info if verification succeeds.

<details>
<summary>Source</summary>

```rust
    async fn verify_package(
```

</details>

##### <span id="PackageSigner.verify_package_with_detached_signature"></span><code>verify_​package_​with_​detached_​signature</code>

```rust
fn verify_package_with_detached_signature(
    &self,
    package_path: &Path,
    signature: &DetachedSignature,
    public_key: &[u8],
) -> Result<(), PackageSignError>
```

Verify a package against a detached signature file.

<details>
<summary>Source</summary>

```rust
    fn verify_package_with_detached_signature(
```

</details>
