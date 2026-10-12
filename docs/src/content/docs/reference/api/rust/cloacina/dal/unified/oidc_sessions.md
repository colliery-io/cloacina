---
title: "cloacina::dal::unified::oidc_sessions"
description: "Server-side OIDC **login-session** store (CLOACI-T-0793, T-0923). **Postgres only** — like the rest of the auth strand (`api_keys`, `local_accounts`, `oidc_login_flows`), because the API server runs on Postgres."
sidebar:
  order: 95
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Server-side OIDC **login-session** store (CLOACI-T-0793, T-0923). **Postgres only** — like the rest of the auth strand (`api_keys`, `local_accounts`, `oidc_login_flows`), because the API server runs on Postgres.

One row per live login session. `key_id` is the session's *currently valid*
minted key — [`OidcSessionDAL::rotate`] moves it onto the freshly minted key
on every refresh — and `expires_at` is the session's **absolute** deadline:
the wall past which no amount of refreshing keeps the user in and a full
OIDC re-auth is required. `/auth/refresh` + `/auth/logout` consume this
store.
T-0793 originally built the row to hold an IdP refresh token, encrypted at
rest with AES-256-GCM under a 32-byte server key. T-0923 chose a session
record over IdP-token custody (rationale in `routes/session.rs`), so
`refresh_enc` is NULL in practice; the column and the
[`encrypt_token`]/[`decrypt_token`] helpers are kept so a deployment that
later wants true token custody needs no migration. If it is ever populated
the plaintext must never be logged or returned to the browser.

## Structs

### <span id="struct.RefreshSession"></span>`struct RefreshSession`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


**Derives:** `Debug`, `Clone`

A decrypted refresh session: the provider that issued it + the plaintext refresh token (empty when no token is stored — the T-0923 session-record design). Never logged.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `provider` | `String` |  |
| `refresh_token` | `Vec < u8 >` |  |
| `expires_at` | `Option < DateTime < Utc > >` | The session's absolute deadline. `None` = unbounded (never produced by
[`OidcSessionDAL::create_session`]). |



### <span id="struct.OidcSessionRow"></span>`struct OidcSessionRow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Queryable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `key_id` | `Uuid` |  |
| `provider` | `String` |  |
| `refresh_enc` | `Option < Vec < u8 > >` |  |
| `created_at` | `chrono :: NaiveDateTime` |  |
| `expires_at` | `Option < chrono :: NaiveDateTime >` |  |



### <span id="struct.NewOidcSession"></span>`struct NewOidcSession`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


**Derives:** `Insertable`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `Uuid` |  |
| `key_id` | `Uuid` |  |
| `provider` | `String` |  |
| `refresh_enc` | `Option < Vec < u8 > >` |  |
| `expires_at` | `Option < chrono :: NaiveDateTime >` |  |



### <span id="struct.OidcSessionDAL"></span>`struct OidcSessionDAL<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


DAL for the encrypted refresh-token store. Postgres only.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |



## Functions

### <span id="fn.encrypt_token"></span>`fn encrypt_token`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



### <span id="fn.decrypt_token"></span>`fn decrypt_token`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



