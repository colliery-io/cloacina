# cloacina::security::verification <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Package verification for secure loading.

This module provides:
- [`SecurityConfig`] for configuring signature requirements
- [`VerificationError`] for specific failure types
- [`SignatureSource`] for specifying where to find signatures
- [`verify_and_load_package`] for verified package loading

## Structs

### `cloacina::security::verification::SecurityConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Default`

Security configuration for package verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `require_signatures` | `bool` | Whether package signatures are required (default: false).

When `true`, packages without valid signatures from trusted keys
will fail to load with a hard error.

When `false`, packages load without verification (for local development). |
| `key_encryption_key` | `Option < [u8 ; 32] >` | Master encryption key for decrypting signing keys (optional).

Only needed if using database-stored signing keys for signing operations.
For verification-only scenarios (loading packages), this is not required. |

#### Methods

##### `require_signatures` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn require_signatures () -> Self
```

Create a security config that requires signatures.

<details>
<summary>Source</summary>

```rust
    pub fn require_signatures() -> Self {
        Self {
            require_signatures: true,
            key_encryption_key: None,
        }
    }
```

</details>



##### `development` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn development () -> Self
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



##### `with_encryption_key` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_encryption_key (mut self , key : [u8 ; 32]) -> Self
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





### `cloacina::security::verification::VerificationResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Result of successful verification.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `String` | Hash of the verified package |
| `signer_fingerprint` | `String` | Fingerprint of the key that signed it |
| `signer_name` | `Option < String >` | Name of the trusted key (if available) |



## Enums

### `cloacina::security::verification::VerificationError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::security::verification::SignatureSource` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Where to find the signature for a package.

#### Variants

- **`Database`** - Load signature from database by package hash.
- **`DetachedFile`** - Load signature from a detached `.sig` file.
- **`Auto`** - Try detached file first (package_path + ".sig"), then database.



## Functions

### `cloacina::security::verification::verify_package`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
async fn verify_package < P : AsRef < Path > > (package_path : P , org_id : UniversalUuid , signature_source : SignatureSource , package_signer : & DbPackageSigner , key_manager : & DbKeyManager ,) -> Result < VerificationResult , VerificationError >
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
| `package_path` | `-` | Path to the package file to verify |
| `org_id` | `-` | Organization ID to check trusted keys for |
| `signature_source` | `-` | Where to find the signature |
| `package_signer` | `-` | Package signer for database operations |
| `key_manager` | `-` | Key manager for trusted key lookup |


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



### `cloacina::security::verification::verify_package_offline`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn verify_package_offline < P : AsRef < Path > , S : AsRef < Path > > (package_path : P , signature_path : S , public_key : & [u8] ,) -> Result < VerificationResult , VerificationError >
```

Verify a package using only a detached signature and public key (offline mode).

This is useful when the database is not available or for offline verification.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_path` | `-` | Path to the package file to verify |
| `signature_path` | `-` | Path to the detached signature file |
| `public_key` | `-` | The 32-byte Ed25519 public key to verify against |


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



### `cloacina::security::verification::compute_package_hash`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn compute_package_hash (data : & [u8]) -> Result < String , VerificationError >
```

Compute SHA256 hash of package data.

<details>
<summary>Source</summary>

```rust
fn compute_package_hash(data: &[u8]) -> Result<String, VerificationError> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(data);
    Ok(hex::encode(hasher.finalize()))
}
```

</details>



### `cloacina::security::verification::load_signature_from_db`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
async fn load_signature_from_db (package_hash : & str , package_signer : & DbPackageSigner ,) -> Result < DetachedSignature , VerificationError >
```

Load signature from database.

<details>
<summary>Source</summary>

```rust
async fn load_signature_from_db(
    package_hash: &str,
    package_signer: &DbPackageSigner,
) -> Result<DetachedSignature, VerificationError> {
    let signature = package_signer
        .find_signature(package_hash)
        .await
        .map_err(|e| VerificationError::DatabaseError {
            error: e.to_string(),
        })?
        .ok_or_else(|| VerificationError::SignatureNotFound {
            hash: package_hash.to_string(),
        })?;

    Ok(DetachedSignature::from_signature_info(&signature))
}
```

</details>



### `cloacina::security::verification::load_signature_from_file`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn load_signature_from_file (path : & Path) -> Result < DetachedSignature , VerificationError >
```

Load signature from file.

<details>
<summary>Source</summary>

```rust
fn load_signature_from_file(path: &Path) -> Result<DetachedSignature, VerificationError> {
    DetachedSignature::read_from_file(path).map_err(|e| VerificationError::MalformedSignature {
        reason: e.to_string(),
    })
}
```

</details>
