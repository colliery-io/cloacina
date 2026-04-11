# cloacina::crypto::key_encryption <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


AES-256-GCM encryption for private key storage at rest.

Private keys are stored encrypted in the database using AES-256-GCM.
The encrypted format is: `nonce (12 bytes) || ciphertext || tag (16 bytes)`.
The encryption key should be derived from a secure source such as:
- A master key stored in a hardware security module (HSM)
- A key derived from a passphrase using a KDF like Argon2
- A key management service (KMS)

## Enums

### `cloacina::crypto::key_encryption::KeyEncryptionError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during key encryption/decryption.

#### Variants

- **`EncryptionFailed`**
- **`DecryptionFailed`**
- **`InvalidKeyLength`**
- **`InvalidEncryptedData`**



## Functions

### `cloacina::crypto::key_encryption::encrypt_private_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn encrypt_private_key (private_key : & [u8] , encryption_key : & [u8] ,) -> Result < Vec < u8 > , KeyEncryptionError >
```

Encrypts an Ed25519 private key using AES-256-GCM.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `private_key` | `-` | The 32-byte Ed25519 private key seed to encrypt |
| `encryption_key` | `-` | The 32-byte AES-256 encryption key |


**Returns:**

The encrypted data in format: `nonce (12 bytes) || ciphertext || tag (16 bytes)`

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns `KeyEncryptionError` if encryption fails or key lengths are invalid. |


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
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
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



### `cloacina::crypto::key_encryption::decrypt_private_key`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn decrypt_private_key (encrypted_data : & [u8] , encryption_key : & [u8] ,) -> Result < Vec < u8 > , KeyEncryptionError >
```

Decrypts an Ed25519 private key that was encrypted with AES-256-GCM.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `encrypted_data` | `-` | The encrypted data in format: `nonce (12 bytes) \|\| ciphertext \|\| tag` |
| `encryption_key` | `-` | The 32-byte AES-256 encryption key |


**Returns:**

The decrypted 32-byte Ed25519 private key seed.

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns `KeyEncryptionError` if decryption fails or data is invalid. |


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
