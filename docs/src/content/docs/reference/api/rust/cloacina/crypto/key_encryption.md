---
title: "cloacina::crypto::key_encryption"
description: "AES-256-GCM encryption for private key storage at rest."
editUrl: false
sidebar:
  order: 60
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

AES-256-GCM encryption for private key storage at rest.

Private keys are stored encrypted in the database using AES-256-GCM.
The encrypted format is: `nonce (12 bytes) || ciphertext || tag (16 bytes)`.

The encryption key should be derived from a secure source such as:
- A master key stored in a hardware security module (HSM)
- A key derived from a passphrase using a KDF like Argon2
- A key management service (KMS)

## Enums

### <span id="enum.KeyEncryptionError"></span><code style="font-size: 0.85em">enum KeyEncryptionError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during key encryption/decryption.

#### Variants

- **`EncryptionFailed`**
- **`DecryptionFailed`**
- **`InvalidKeyLength`**
- **`InvalidEncryptedData`**

## Functions

### <span id="fn.encrypt_private_key"></span><code style="font-size: 0.85em">fn encrypt_​private_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn encrypt_private_key(
    private_key: &[u8],
    encryption_key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError>
```

Encrypts an Ed25519 private key using AES-256-GCM.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `private_key` | `&[u8]` | The 32-byte Ed25519 private key seed to encrypt |
| `encryption_key` | `&[u8]` | The 32-byte AES-256 encryption key |

**Returns:**

The encrypted data in format: `nonce (12 bytes) || ciphertext || tag (16 bytes)`

**Errors:**

Returns `KeyEncryptionError` if encryption fails or key lengths are invalid.

<details>
<summary>Source</summary>

```rust
pub fn encrypt_private_key(
    private_key: &[u8],
    encryption_key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError> {
    if encryption_key.len() != 32 {
        return Err(KeyEncryptionError::InvalidKeyLength(encryption_key.len()));
    }

    let cipher = Aes256Gcm::new_from_slice(encryption_key)
        .map_err(|e| KeyEncryptionError::EncryptionFailed(e.to_string()))?;

    // Generate a random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the private key
    let ciphertext = cipher
        .encrypt(nonce, private_key)
        .map_err(|e| KeyEncryptionError::EncryptionFailed(e.to_string()))?;

    // Concatenate nonce || ciphertext (which includes the tag)
    let mut encrypted = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    encrypted.extend_from_slice(&nonce_bytes);
    encrypted.extend_from_slice(&ciphertext);

    Ok(encrypted)
}
```

</details>

### <span id="fn.decrypt_private_key"></span><code style="font-size: 0.85em">fn decrypt_​private_​key</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn decrypt_private_key(
    encrypted_data: &[u8],
    encryption_key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError>
```

Decrypts an Ed25519 private key that was encrypted with AES-256-GCM.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `encrypted_data` | `&[u8]` | The encrypted data in format: `nonce (12 bytes) \|\| ciphertext \|\| tag` |
| `encryption_key` | `&[u8]` | The 32-byte AES-256 encryption key |

**Returns:**

The decrypted 32-byte Ed25519 private key seed.

**Errors:**

Returns `KeyEncryptionError` if decryption fails or data is invalid.

<details>
<summary>Source</summary>

```rust
pub fn decrypt_private_key(
    encrypted_data: &[u8],
    encryption_key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError> {
    if encryption_key.len() != 32 {
        return Err(KeyEncryptionError::InvalidKeyLength(encryption_key.len()));
    }

    // Minimum size: nonce (12) + tag (16) + at least 1 byte of ciphertext
    if encrypted_data.len() < NONCE_SIZE + 17 {
        return Err(KeyEncryptionError::InvalidEncryptedData);
    }

    let cipher = Aes256Gcm::new_from_slice(encryption_key)
        .map_err(|e| KeyEncryptionError::DecryptionFailed(e.to_string()))?;

    // Extract nonce and ciphertext
    let nonce = Nonce::from_slice(&encrypted_data[..NONCE_SIZE]);
    let ciphertext = &encrypted_data[NONCE_SIZE..];

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| KeyEncryptionError::DecryptionFailed(e.to_string()))?;

    Ok(plaintext)
}
```

</details>

### <span id="fn.encrypt_bytes"></span><code style="font-size: 0.85em">fn encrypt_​bytes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn encrypt_bytes(
    plaintext: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError>
```

Encrypts arbitrary bytes using AES-256-GCM.

This is a thin, intent-revealing alias for [`encrypt_private_key`](../../../../rust/cloacina/crypto/key_encryption/#fn.encrypt_private_key): the
underlying AEAD is generic over the plaintext, so the same primitive that
wraps signing keys also wraps a tenant data key (DEK) or a serialized
`{field: value}` secret map (CLOACI-I-0133 / T-0857). Output format is
`nonce (12 bytes) || ciphertext || tag (16 bytes)`.

<details>
<summary>Source</summary>

```rust
pub fn encrypt_bytes(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, KeyEncryptionError> {
    encrypt_private_key(plaintext, key)
}
```

</details>

### <span id="fn.decrypt_bytes"></span><code style="font-size: 0.85em">fn decrypt_​bytes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn decrypt_bytes(
    encrypted_data: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, KeyEncryptionError>
```

Decrypts bytes produced by [`encrypt_bytes`](../../../../rust/cloacina/crypto/key_encryption/#fn.encrypt_bytes) (or [`encrypt_private_key`](../../../../rust/cloacina/crypto/key_encryption/#fn.encrypt_private_key)).

<details>
<summary>Source</summary>

```rust
pub fn decrypt_bytes(encrypted_data: &[u8], key: &[u8]) -> Result<Vec<u8>, KeyEncryptionError> {
    decrypt_private_key(encrypted_data, key)
}
```

</details>
