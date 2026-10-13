---
title: "cloacina::dal::unified::oidc_sessions"
description: "Server-side OIDC **login-session** store (CLOACI-T-0793, T-0923). **Postgres only** — like the rest of the auth strand (`api_keys`, `local_accounts`, `oidc_login_flows`), because the API server runs on Postgres."
editUrl: false
sidebar:
  order: 79
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Server-side OIDC **login-session** store (CLOACI-T-0793, T-0923). **Postgres only** — like the rest of the auth strand (`api_keys`, `local_accounts`, `oidc_login_flows`), because the API server runs on Postgres.

One row per live login session. `key_id` is the session's *currently valid*
minted key — [`OidcSessionDAL::rotate`](../../../../../rust/cloacina/dal/unified/oidc_sessions/#OidcSessionDAL.rotate) moves it onto the freshly minted key
on every refresh — and `expires_at` is the session's **absolute** deadline:
the wall past which no amount of refreshing keeps the user in and a full
OIDC re-auth is required. `/auth/refresh` + `/auth/logout` consume this
store.

T-0793 originally built the row to hold an IdP refresh token, encrypted at
rest with AES-256-GCM under a 32-byte server key. T-0923 chose a session
record over IdP-token custody (rationale in `routes/session.rs`), so
`refresh_enc` is NULL in practice; the column and the
[`encrypt_token`](../../../../../rust/cloacina/dal/unified/oidc_sessions/#fn.encrypt_token)/[`decrypt_token`](../../../../../rust/cloacina/dal/unified/oidc_sessions/#fn.decrypt_token) helpers are kept so a deployment that
later wants true token custody needs no migration. If it is ever populated
the plaintext must never be logged or returned to the browser.

## Structs

### <span id="struct.RefreshSession"></span><code style="font-size: 0.85em">struct RefreshSession</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

A decrypted refresh session: the provider that issued it + the plaintext refresh token (empty when no token is stored — the T-0923 session-record design). Never logged.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `provider` | `String` |  |
| `refresh_token` | `Vec<u8>` |  |
| `expires_at` | `Option<DateTime<Utc>>` | The session's absolute deadline. `None` = unbounded (never produced by [`OidcSessionDAL::create_session`](../../../../../rust/cloacina/dal/unified/oidc_sessions/#OidcSessionDAL.create_session)). |

### <span id="struct.OidcSessionDAL"></span><code style="font-size: 0.85em">struct OidcSessionDAL&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

DAL for the encrypted refresh-token store. Postgres only.

#### Methods

##### <span id="OidcSessionDAL.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }
```

</details>

##### <span id="OidcSessionDAL.create"></span><code>create</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create(
    &self,
    key_id: Uuid,
    provider: &str,
    refresh_token: &[u8],
    enc_key: &[u8],
    expires_at: Option<DateTime<Utc>>,
) -> Result<(), ValidationError>
```

Store an encrypted refresh token for a minted key. Encrypts under `enc_key` (32 bytes); the plaintext never hits the DB or a log.

<details>
<summary>Source</summary>

```rust
    pub async fn create(
        &self,
        key_id: Uuid,
        provider: &str,
        refresh_token: &[u8],
        enc_key: &[u8],
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), ValidationError> {
        let refresh_enc = Some(encrypt_token(refresh_token, enc_key)?);
        let row = NewOidcSession {
            id: Uuid::new_v4(),
            key_id,
            provider: provider.to_string(),
            refresh_enc,
            expires_at: expires_at.map(|t| t.naive_utc()),
        };
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(crate::database::schema::postgres::oidc_sessions::table)
                .values(&row)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
```

</details>

##### <span id="OidcSessionDAL.get"></span><code>get</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get(
    &self,
    key_id: Uuid,
    enc_key: &[u8],
) -> Result<Option<RefreshSession>, ValidationError>
```

Fetch + decrypt the refresh session for a minted key, if any.

<details>
<summary>Source</summary>

```rust
    pub async fn get(
        &self,
        key_id: Uuid,
        enc_key: &[u8],
    ) -> Result<Option<RefreshSession>, ValidationError> {
        use crate::database::schema::postgres::oidc_sessions;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let row: Option<OidcSessionRow> = conn
            .interact(move |conn| {
                oidc_sessions::table
                    .filter(oidc_sessions::key_id.eq(key_id))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        match row {
            None => Ok(None),
            Some(r) => {
                let refresh_token = match &r.refresh_enc {
                    Some(enc) => decrypt_token(enc, enc_key)?,
                    None => Vec::new(),
                };
                Ok(Some(RefreshSession {
                    provider: r.provider,
                    refresh_token,
                    expires_at: r.expires_at.map(|t| t.and_utc()),
                }))
            }
        }
    }
```

</details>

##### <span id="OidcSessionDAL.create_session"></span><code>create_​session</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn create_session(
    &self,
    key_id: Uuid,
    provider: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), ValidationError>
```

Open a login session for a freshly minted key (CLOACI-T-0923).

`expires_at` is the session's **absolute** deadline — refresh extends
the *key*, never this. No IdP token is stored (`refresh_enc` stays
NULL); see the module docs for why. Lapsed rows are pruned
opportunistically first, so the table stays bounded without a dedicated
sweeper (mirrors the `ws_tickets` hygiene pattern).

<details>
<summary>Source</summary>

```rust
    pub async fn create_session(
        &self,
        key_id: Uuid,
        provider: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        use crate::database::schema::postgres::oidc_sessions;
        let row = NewOidcSession {
            id: Uuid::new_v4(),
            key_id,
            provider: provider.to_string(),
            refresh_enc: None,
            expires_at: Some(expires_at.naive_utc()),
        };
        let now = Utc::now().naive_utc();
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::delete(oidc_sessions::table.filter(oidc_sessions::expires_at.lt(now)))
                .execute(conn)?;
            diesel::insert_into(oidc_sessions::table)
                .values(&row)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }
```

</details>

##### <span id="OidcSessionDAL.get_session"></span><code>get_​session</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_session(
    &self,
    key_id: Uuid,
) -> Result<Option<(String, Option<DateTime<Utc>>)>, ValidationError>
```

Look up the session a minted key belongs to, without touching `refresh_enc` (so no encryption key is needed on the refresh path). Returns `(provider, absolute_expiry)`.

<details>
<summary>Source</summary>

```rust
    pub async fn get_session(
        &self,
        key_id: Uuid,
    ) -> Result<Option<(String, Option<DateTime<Utc>>)>, ValidationError> {
        use crate::database::schema::postgres::oidc_sessions;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let row: Option<(String, Option<chrono::NaiveDateTime>)> = conn
            .interact(move |conn| {
                oidc_sessions::table
                    .filter(oidc_sessions::key_id.eq(key_id))
                    .select((oidc_sessions::provider, oidc_sessions::expires_at))
                    .first(conn)
                    .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row.map(|(p, e)| (p, e.map(|t| t.and_utc()))))
    }
```

</details>

##### <span id="OidcSessionDAL.rotate"></span><code>rotate</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn rotate(
    &self,
    old_key_id: Uuid,
    new_key_id: Uuid,
) -> Result<bool, ValidationError>
```

Point an existing session at a newly minted key (refresh rotation).

The session's `expires_at` is deliberately **not** extended: the
absolute deadline is the whole point of the record. Returns true when a
session was rotated.

<details>
<summary>Source</summary>

```rust
    pub async fn rotate(
        &self,
        old_key_id: Uuid,
        new_key_id: Uuid,
    ) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::oidc_sessions;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n: usize = conn
            .interact(move |conn| {
                diesel::update(oidc_sessions::table.filter(oidc_sessions::key_id.eq(old_key_id)))
                    .set(oidc_sessions::key_id.eq(new_key_id))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }
```

</details>

##### <span id="OidcSessionDAL.delete"></span><code>delete</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn delete(&self, key_id: Uuid) -> Result<bool, ValidationError>
```

Forget a minted key's refresh session (logout). Returns true if removed.

<details>
<summary>Source</summary>

```rust
    pub async fn delete(&self, key_id: Uuid) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::oidc_sessions;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n: usize = conn
            .interact(move |conn| {
                diesel::delete(oidc_sessions::table.filter(oidc_sessions::key_id.eq(key_id)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }
```

</details>

##### <span id="OidcSessionDAL.sweep_expired"></span><code>sweep_​expired</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn sweep_expired(&self) -> Result<usize, ValidationError>
```

Delete all sessions whose `expires_at` has passed. Returns the count. Run periodically by the server sweeper (mirrors the ws-ticket/outbox hygiene pattern). NULL `expires_at` = never swept here.

<details>
<summary>Source</summary>

```rust
    pub async fn sweep_expired(&self) -> Result<usize, ValidationError> {
        use crate::database::schema::postgres::oidc_sessions;
        let now = Utc::now().naive_utc();
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n: usize = conn
            .interact(move |conn| {
                diesel::delete(oidc_sessions::table.filter(oidc_sessions::expires_at.lt(now)))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n)
    }
```

</details>

## Functions

### <span id="fn.encrypt_token"></span><code style="font-size: 0.85em">fn encrypt_​token</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn encrypt_token(
    plaintext: &[u8],
    enc_key: &[u8],
) -> Result<Vec<u8>, ValidationError>
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

### <span id="fn.decrypt_token"></span><code style="font-size: 0.85em">fn decrypt_​token</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn decrypt_token(
    ciphertext: &[u8],
    enc_key: &[u8],
) -> Result<Vec<u8>, ValidationError>
```

Decrypt a refresh token previously sealed by [`encrypt_token`](../../../../../rust/cloacina/dal/unified/oidc_sessions/#fn.encrypt_token).

<details>
<summary>Source</summary>

```rust
pub fn decrypt_token(ciphertext: &[u8], enc_key: &[u8]) -> Result<Vec<u8>, ValidationError> {
    crate::crypto::decrypt_private_key(ciphertext, enc_key)
        .map_err(|e| ValidationError::ConnectionPool(format!("refresh-token decryption: {e}")))
}
```

</details>
