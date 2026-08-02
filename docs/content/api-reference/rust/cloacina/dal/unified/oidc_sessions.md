# cloacina::dal::unified::oidc_sessions <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Encrypted server-side refresh-token store (CLOACI-T-0793). **Postgres only.**

One row per minted login key. The refresh token is encrypted at rest with
AES-256-GCM (reusing [`crate::crypto::key_encryption`]) under a 32-byte
server key; the plaintext is never logged or returned to the browser. A
sweeper deletes lapsed rows. `/auth/refresh` + `/auth/logout` (T-0794)
consume this store.

## Structs

### `cloacina::dal::unified::oidc_sessions::RefreshSession`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

A decrypted refresh session: the provider that issued it + the plaintext refresh token. Never logged.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `provider` | `String` |  |
| `refresh_token` | `Vec < u8 >` |  |
| `expires_at` | `Option < DateTime < Utc > >` |  |



### `cloacina::dal::unified::oidc_sessions::OidcSessionRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Queryable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `key_id` | `Uuid` |  |
| `provider` | `String` |  |
| `refresh_enc` | `Vec < u8 >` |  |
| `created_at` | `chrono :: NaiveDateTime` |  |
| `expires_at` | `Option < chrono :: NaiveDateTime >` |  |



### `cloacina::dal::unified::oidc_sessions::NewOidcSession`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `key_id` | `Uuid` |  |
| `provider` | `String` |  |
| `refresh_enc` | `Vec < u8 >` |  |
| `expires_at` | `Option < chrono :: NaiveDateTime >` |  |



### `cloacina::dal::unified::oidc_sessions::OidcSessionDAL`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


DAL for the encrypted refresh-token store. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### `cloacina::dal::unified::oidc_sessions::encrypt_token`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn encrypt_token (plaintext : & [u8] , enc_key : & [u8]) -> Result < Vec < u8 > , ValidationError >
```

Encrypt a refresh token with the 32-byte server key (AES-256-GCM, random nonce; output is `nonce || ciphertext || tag`).

<details>
<summary>Source</summary>

```rust
pub fn encrypt_token(plaintext: &[u8], enc_key: &[u8]) -> Result<Vec<u8>, ValidationError> {
    crate::crypto::encrypt_private_key(plaintext, enc_key)
        .map_err(|e| ValidationError::ConnectionPool(format!("refresh-token encryption: {e}")))
}
```

</details>



### `cloacina::dal::unified::oidc_sessions::decrypt_token`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn decrypt_token (ciphertext : & [u8] , enc_key : & [u8]) -> Result < Vec < u8 > , ValidationError >
```

Decrypt a refresh token previously sealed by [`encrypt_token`].

<details>
<summary>Source</summary>

```rust
pub fn decrypt_token(ciphertext: &[u8], enc_key: &[u8]) -> Result<Vec<u8>, ValidationError> {
    crate::crypto::decrypt_private_key(ciphertext, enc_key)
        .map_err(|e| ValidationError::ConnectionPool(format!("refresh-token decryption: {e}")))
}
```

</details>
