---
title: "cloacina::security::verification"
description: "Package verification for secure loading."
editUrl: false
sidebar:
  order: 196
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Package verification for secure loading.

This module provides:
- [`SecurityConfig`](../../../../rust/cloacina/security/verification/#struct.SecurityConfig) for configuring signature requirements
- [`VerificationError`](../../../../rust/cloacina/security/verification/#enum.VerificationError) for specific failure types
- [`SignatureSource`](../../../../rust/cloacina/security/verification/#enum.SignatureSource) for specifying where to find signatures
- `verify_and_load_package` for verified package loading

## Structs

### <span id="struct.SecurityConfig"></span><code style="font-size: 0.85em">struct SecurityConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`, `Default`

Security configuration for package verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `require_signatures` | `bool` | Whether package signatures are required (default: false).<br>When `true`, packages without valid signatures from trusted keys will fail to load with a hard error.<br>When `false`, packages load without verification (for local development). |
| `key_encryption_key` | `Option<[u8; 32]>` | Master encryption key for decrypting signing keys (optional).<br>Only needed if using database-stored signing keys for signing operations. For verification-only scenarios (loading packages), this is not required. |
| `verification_org_id` | `Option<UniversalUuid>` | Organization ID against which uploaded-package signatures are verified. The verification flow looks up trusted keys scoped to this org (`KeyManager::find_trusted_key(org_id, fingerprint)`). `None` means "no org binding configured" — verification cannot run, and `require_signatures = true` becomes fail-safe (reject all uploads). Set this to the server's default-org UUID when signature verification is enabled. (T-0557 Bug 2) |

#### Methods

##### <span id="SecurityConfig.require_signatures"></span><code>require_​signatures</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn require_signatures() -> Self
```

Create a security config that requires signatures.

<details>
<summary>Source</summary>

```rust
    pub fn require_signatures() -> Self {
        Self {
            require_signatures: true,
            key_encryption_key: None,
            verification_org_id: None,
        }
    }
```

</details>

##### <span id="SecurityConfig.development"></span><code>development</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn development() -> Self
```

Create a security config with no signature requirements (for development).

<details>
<summary>Source</summary>

```rust
    pub fn development() -> Self {
        Self::default()
    }
```

</details>

##### <span id="SecurityConfig.with_encryption_key"></span><code>with_​encryption_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_encryption_key(mut self, key: [u8; 32]) -> Self
```

Set the key encryption key for signing operations.

<details>
<summary>Source</summary>

```rust
    pub fn with_encryption_key(mut self, key: [u8; 32]) -> Self {
        self.key_encryption_key = Some(key);
        self
    }
```

</details>

### <span id="struct.VerificationResult"></span><code style="font-size: 0.85em">struct VerificationResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Result of successful verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `String` | Hash of the verified package |
| `signer_fingerprint` | `String` | Fingerprint of the key that signed it |
| `signer_name` | `Option<String>` | Name of the trusted key (if available) |

## Enums

### <span id="enum.VerificationError"></span><code style="font-size: 0.85em">enum VerificationError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that occur during package verification.

These are hard failures - there are no "warnings" for security.

#### Variants

- **`TamperedPackage`**
- **`UntrustedSigner`**
- **`InvalidSignature`**
- **`SignatureNotFound`**
- **`MalformedSignature`**
- **`FileReadError`**
- **`HashError`**
- **`DatabaseError`**
- **`KeyManagerError`**

### <span id="enum.SignatureSource"></span><code style="font-size: 0.85em">enum SignatureSource</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Where to find the signature for a package.

#### Variants

- **`Database`** - Load signature from database by package hash.
- **`DetachedFile`** - Load signature from a detached `.sig` file.
- **`Auto`** - Try detached file first (package_path + ".sig"), then database.

## Functions

### <span id="fn.verify_package"></span><code style="font-size: 0.85em">fn verify_​package</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn verify_package<P: AsRef<Path>>(
    package_path: P,
    org_id: UniversalUuid,
    signature_source: SignatureSource,
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<VerificationResult, VerificationError>
```

Verify a package signature.

This function performs full cryptographic verification:
1. Computes the package hash
2. Loads the signature (from database or file)
3. Finds the trusted key that signed it
4. Verifies the Ed25519 signature

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_path` | `P` | Path to the package file to verify |
| `org_id` | `UniversalUuid` | Organization ID to check trusted keys for |
| `signature_source` | `SignatureSource` | Where to find the signature |
| `package_signer` | `&DbPackageSigner` | Package signer for database operations |
| `key_manager` | `&DbKeyManager` | Key manager for trusted key lookup |

**Returns:**

`Ok(VerificationResult)` if verification succeeds, `Err(VerificationError)` otherwise.

<details>
<summary>Source</summary>

```rust
pub async fn verify_package<P: AsRef<Path>>(
    package_path: P,
    org_id: UniversalUuid,
    signature_source: SignatureSource,
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<VerificationResult, VerificationError> {
    let package_path = package_path.as_ref();

    // 1. Compute package hash
    let package_data =
        std::fs::read(package_path).map_err(|e| VerificationError::FileReadError {
            error: e.to_string(),
        })?;

    let package_hash = compute_package_hash(&package_data)?;

    // 2. Load signature based on source
    let signature = match signature_source {
        SignatureSource::Database => load_signature_from_db(&package_hash, package_signer).await?,

        SignatureSource::DetachedFile { path } => load_signature_from_file(&path)?,

        SignatureSource::Auto => {
            // Try detached file first
            let sig_path = package_path.with_extension(format!(
                "{}.sig",
                package_path
                    .extension()
                    .map(|e| e.to_str().unwrap_or(""))
                    .unwrap_or("")
            ));

            if sig_path.exists() {
                load_signature_from_file(&sig_path)?
            } else {
                load_signature_from_db(&package_hash, package_signer).await?
            }
        }
    };

    // 3. Verify hash matches (tamper detection)
    if signature.package_hash != package_hash {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "tampered",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::TamperedPackage {
            expected: signature.package_hash,
            actual: package_hash,
        });
    }

    // 4. Find trusted key by fingerprint
    let trusted_key = key_manager
        .find_trusted_key(org_id, &signature.key_fingerprint)
        .await
        .map_err(|e| VerificationError::KeyManagerError {
            error: e.to_string(),
        })?
        .ok_or_else(|| {
            audit::log_verification_failure(
                org_id,
                &package_hash,
                "untrusted_signer",
                Some(&signature.key_fingerprint),
            );
            VerificationError::UntrustedSigner {
                fingerprint: signature.key_fingerprint.clone(),
            }
        })?;

    // 5. Verify Ed25519 signature
    let hash_bytes = hex::decode(&package_hash).map_err(|e| VerificationError::HashError {
        error: e.to_string(),
    })?;

    let sig_bytes = signature.signature_bytes().map_err(|_| {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        VerificationError::InvalidSignature
    })?;

    if verify_signature(&hash_bytes, &sig_bytes, &trusted_key.public_key).is_err() {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::InvalidSignature);
    }

    // 6. Success - audit log
    audit::log_verification_success(
        org_id,
        &package_hash,
        &signature.key_fingerprint,
        trusted_key.key_name.as_deref(),
    );

    Ok(VerificationResult {
        package_hash,
        signer_fingerprint: signature.key_fingerprint,
        signer_name: trusted_key.key_name,
    })
}
```

</details>

### <span id="fn.verify_package_offline"></span><code style="font-size: 0.85em">fn verify_​package_​offline</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn verify_package_offline<P: AsRef<Path>, S: AsRef<Path>>(
    package_path: P,
    signature_path: S,
    public_key: &[u8],
) -> Result<VerificationResult, VerificationError>
```

Verify a package using only a detached signature and public key (offline mode).

This is useful when the database is not available or for offline verification.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_path` | `P` | Path to the package file to verify |
| `signature_path` | `S` | Path to the detached signature file |
| `public_key` | `&[u8]` | The 32-byte Ed25519 public key to verify against |

**Returns:**

`Ok(())` if verification succeeds, `Err(VerificationError)` otherwise.

<details>
<summary>Source</summary>

```rust
pub fn verify_package_offline<P: AsRef<Path>, S: AsRef<Path>>(
    package_path: P,
    signature_path: S,
    public_key: &[u8],
) -> Result<VerificationResult, VerificationError> {
    let package_path = package_path.as_ref();
    let signature_path = signature_path.as_ref();

    // 1. Load package and compute hash
    let package_data =
        std::fs::read(package_path).map_err(|e| VerificationError::FileReadError {
            error: e.to_string(),
        })?;

    let package_hash = compute_package_hash(&package_data)?;

    // 2. Load signature from file
    let signature = load_signature_from_file(signature_path)?;

    // 3. Verify hash matches
    if signature.package_hash != package_hash {
        return Err(VerificationError::TamperedPackage {
            expected: signature.package_hash,
            actual: package_hash,
        });
    }

    // 4. Verify key fingerprint matches
    let expected_fingerprint = crate::crypto::compute_key_fingerprint(public_key);
    if signature.key_fingerprint != expected_fingerprint {
        return Err(VerificationError::UntrustedSigner {
            fingerprint: signature.key_fingerprint,
        });
    }

    // 5. Verify signature
    let hash_bytes = hex::decode(&package_hash).map_err(|e| VerificationError::HashError {
        error: e.to_string(),
    })?;

    let sig_bytes = signature
        .signature_bytes()
        .map_err(|_| VerificationError::InvalidSignature)?;

    verify_signature(&hash_bytes, &sig_bytes, public_key)
        .map_err(|_| VerificationError::InvalidSignature)?;

    tracing::info!(
        event_type = "verification.success.offline",
        package = %package_path.display(),
        signer_fingerprint = %signature.key_fingerprint,
        "Package signature verified (offline mode)"
    );

    Ok(VerificationResult {
        package_hash,
        signer_fingerprint: signature.key_fingerprint,
        signer_name: None,
    })
}
```

</details>

### <span id="fn.verify_package_bytes"></span><code style="font-size: 0.85em">fn verify_​package_​bytes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
async fn verify_package_bytes(
    package_data: &[u8],
    org_id: UniversalUuid,
    signature_source: SignatureSource,
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<VerificationResult, VerificationError>
```

Verify a package signature against in-memory bytes. Same flow as `verify_package` but takes a `&[u8]` instead of a path; useful for upload handlers that already have the package buffered (T-0557 Bug 2).

<details>
<summary>Source</summary>

```rust
pub async fn verify_package_bytes(
    package_data: &[u8],
    org_id: UniversalUuid,
    signature_source: SignatureSource,
    package_signer: &DbPackageSigner,
    key_manager: &DbKeyManager,
) -> Result<VerificationResult, VerificationError> {
    let package_hash = compute_package_hash(package_data)?;

    let signature = match signature_source {
        SignatureSource::Database => load_signature_from_db(&package_hash, package_signer).await?,
        SignatureSource::DetachedFile { path } => load_signature_from_file(&path)?,
        SignatureSource::Auto => {
            // No on-disk path to derive a sidecar from — fall back to
            // database lookup. Callers wanting detached-file behavior
            // must pass `DetachedFile` explicitly.
            load_signature_from_db(&package_hash, package_signer).await?
        }
    };

    if signature.package_hash != package_hash {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "tampered",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::TamperedPackage {
            expected: signature.package_hash,
            actual: package_hash,
        });
    }

    let trusted_key = key_manager
        .find_trusted_key(org_id, &signature.key_fingerprint)
        .await
        .map_err(|e| VerificationError::KeyManagerError {
            error: e.to_string(),
        })?
        .ok_or_else(|| {
            audit::log_verification_failure(
                org_id,
                &package_hash,
                "untrusted_signer",
                Some(&signature.key_fingerprint),
            );
            VerificationError::UntrustedSigner {
                fingerprint: signature.key_fingerprint.clone(),
            }
        })?;

    let hash_bytes = hex::decode(&package_hash).map_err(|e| VerificationError::HashError {
        error: e.to_string(),
    })?;

    let sig_bytes = signature.signature_bytes().map_err(|_| {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        VerificationError::InvalidSignature
    })?;

    if verify_signature(&hash_bytes, &sig_bytes, &trusted_key.public_key).is_err() {
        audit::log_verification_failure(
            org_id,
            &package_hash,
            "invalid_signature",
            Some(&signature.key_fingerprint),
        );
        return Err(VerificationError::InvalidSignature);
    }

    audit::log_verification_success(
        org_id,
        &package_hash,
        &signature.key_fingerprint,
        trusted_key.key_name.as_deref(),
    );

    Ok(VerificationResult {
        package_hash,
        signer_fingerprint: signature.key_fingerprint,
        signer_name: trusted_key.key_name,
    })
}
```

</details>
