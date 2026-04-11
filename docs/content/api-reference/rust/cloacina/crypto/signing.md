# cloacina::crypto::signing <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Ed25519 signing utilities for package signatures.

Provides functions for:
- Generating Ed25519 signing keypairs
- Computing SHA256 key fingerprints
- Signing package hashes
- Verifying signatures

## Structs

### `cloacina::crypto::signing::GeneratedKeypair`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A generated Ed25519 keypair.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `private_key` | `Vec < u8 >` | The 32-byte private key seed (should be encrypted before storage) |
| `public_key` | `Vec < u8 >` | The 32-byte public key |
| `fingerprint` | `String` | SHA256 hex fingerprint of the public key |



## Enums

### `cloacina::crypto::signing::SigningError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during signing operations.

#### Variants

- **`InvalidPrivateKeyLength`**
- **`InvalidPublicKeyLength`**
- **`InvalidSignatureLength`**
- **`KeyCreationFailed`**
- **`SignatureFailed`**
- **`VerificationFailed`**



## Functions

### `cloacina::crypto::signing::generate_signing_keypair`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn generate_signing_keypair () -> GeneratedKeypair
```

Generates a new Ed25519 signing keypair.

**Returns:**

A `GeneratedKeypair` containing the private key, public key, and fingerprint.

<details>
<summary>Source</summary>

```rust
pub fn generate_signing_keypair() -> GeneratedKeypair {
    let mut csprng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let public_key_bytes = verifying_key.to_bytes();
    let fingerprint = compute_key_fingerprint(&public_key_bytes);

    GeneratedKeypair {
        private_key: signing_key.to_bytes().to_vec(),
        public_key: public_key_bytes.to_vec(),
        fingerprint,
    }
}
```

</details>



### `cloacina::crypto::signing::compute_key_fingerprint`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn compute_key_fingerprint (public_key : & [u8]) -> String
```

Computes the SHA256 hex fingerprint of a public key.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `public_key` | `-` | The 32-byte Ed25519 public key |


**Returns:**

A 64-character hex string representing the SHA256 hash of the public key.

<details>
<summary>Source</summary>

```rust
pub fn compute_key_fingerprint(public_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let hash = hasher.finalize();
    hex::encode(hash)
}
```

</details>



### `cloacina::crypto::signing::sign_package`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn sign_package (package_hash : & [u8] , private_key : & [u8]) -> Result < Vec < u8 > , SigningError >
```

Signs a package hash using an Ed25519 private key.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `-` | The SHA256 hash of the package (as raw bytes or hex string bytes) |
| `private_key` | `-` | The 32-byte Ed25519 private key seed |


**Returns:**

The 64-byte Ed25519 signature.

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns `SigningError` if the private key is invalid. |


<details>
<summary>Source</summary>

```rust
pub fn sign_package(package_hash: &[u8], private_key: &[u8]) -> Result<Vec<u8>, SigningError> {
    if private_key.len() != 32 {
        return Err(SigningError::InvalidPrivateKeyLength(private_key.len()));
    }

    let key_bytes: [u8; 32] = private_key
        .try_into()
        .map_err(|_| SigningError::InvalidPrivateKeyLength(private_key.len()))?;

    let signing_key = SigningKey::from_bytes(&key_bytes);
    let signature = signing_key.sign(package_hash);

    Ok(signature.to_bytes().to_vec())
}
```

</details>



### `cloacina::crypto::signing::verify_signature`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn verify_signature (package_hash : & [u8] , signature : & [u8] , public_key : & [u8] ,) -> Result < () , SigningError >
```

Verifies a package signature using an Ed25519 public key.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `package_hash` | `-` | The SHA256 hash of the package that was signed |
| `signature` | `-` | The 64-byte Ed25519 signature |
| `public_key` | `-` | The 32-byte Ed25519 public key |


**Returns:**

`Ok(())` if the signature is valid.

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns `SigningError` if the signature is invalid or verification fails. |


<details>
<summary>Source</summary>

```rust
pub fn verify_signature(
    package_hash: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<(), SigningError> {
    if public_key.len() != 32 {
        return Err(SigningError::InvalidPublicKeyLength(public_key.len()));
    }
    if signature.len() != 64 {
        return Err(SigningError::InvalidSignatureLength(signature.len()));
    }

    let key_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| SigningError::InvalidPublicKeyLength(public_key.len()))?;

    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| SigningError::InvalidSignatureLength(signature.len()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| SigningError::KeyCreationFailed(e.to_string()))?;

    let sig = Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(package_hash, &sig)
        .map_err(|_| SigningError::VerificationFailed)
}
```

</details>



### `cloacina::crypto::signing::compute_package_hash`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn compute_package_hash (data : & [u8]) -> String
```

Computes the SHA256 hash of package data.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `-` | The package binary data |


**Returns:**

A 64-character hex string representing the SHA256 hash.

<details>
<summary>Source</summary>

```rust
pub fn compute_package_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hex::encode(hash)
}
```

</details>
