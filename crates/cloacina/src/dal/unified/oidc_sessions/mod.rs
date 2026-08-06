/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Server-side OIDC **login-session** store (CLOACI-T-0793, T-0923).
//! **Postgres only** — like the rest of the auth strand (`api_keys`,
//! `local_accounts`, `oidc_login_flows`), because the API server runs on
//! Postgres.
//!
//! One row per live login session. `key_id` is the session's *currently valid*
//! minted key — [`OidcSessionDAL::rotate`] moves it onto the freshly minted key
//! on every refresh — and `expires_at` is the session's **absolute** deadline:
//! the wall past which no amount of refreshing keeps the user in and a full
//! OIDC re-auth is required. `/auth/refresh` + `/auth/logout` consume this
//! store.
//!
//! T-0793 originally built the row to hold an IdP refresh token, encrypted at
//! rest with AES-256-GCM under a 32-byte server key. T-0923 chose a session
//! record over IdP-token custody (rationale in `routes/session.rs`), so
//! `refresh_enc` is NULL in practice; the column and the
//! [`encrypt_token`]/[`decrypt_token`] helpers are kept so a deployment that
//! later wants true token custody needs no migration. If it is ever populated
//! the plaintext must never be logged or returned to the browser.

use crate::dal::unified::DAL;
use crate::error::ValidationError;

#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use diesel::prelude::*;
#[cfg(feature = "postgres")]
use uuid::Uuid;

/// Encrypt a refresh token with the 32-byte server key (AES-256-GCM, random
/// nonce; output is `nonce || ciphertext || tag`).
pub fn encrypt_token(plaintext: &[u8], enc_key: &[u8]) -> Result<Vec<u8>, ValidationError> {
    crate::crypto::encrypt_private_key(plaintext, enc_key)
        .map_err(|e| ValidationError::ConnectionPool(format!("refresh-token encryption: {e}")))
}

/// Decrypt a refresh token previously sealed by [`encrypt_token`].
pub fn decrypt_token(ciphertext: &[u8], enc_key: &[u8]) -> Result<Vec<u8>, ValidationError> {
    crate::crypto::decrypt_private_key(ciphertext, enc_key)
        .map_err(|e| ValidationError::ConnectionPool(format!("refresh-token decryption: {e}")))
}

/// A decrypted refresh session: the provider that issued it + the plaintext
/// refresh token (empty when no token is stored — the T-0923 session-record
/// design). Never logged.
#[derive(Debug, Clone)]
pub struct RefreshSession {
    pub provider: String,
    pub refresh_token: Vec<u8>,
    /// The session's absolute deadline. `None` = unbounded (never produced by
    /// [`OidcSessionDAL::create_session`]).
    pub expires_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
#[derive(Queryable)]
#[diesel(table_name = crate::database::schema::postgres::oidc_sessions)]
struct OidcSessionRow {
    #[allow(dead_code)]
    pub id: Uuid,
    #[allow(dead_code)]
    pub key_id: Uuid,
    pub provider: String,
    pub refresh_enc: Option<Vec<u8>>,
    #[allow(dead_code)]
    pub created_at: chrono::NaiveDateTime,
    pub expires_at: Option<chrono::NaiveDateTime>,
}

#[cfg(feature = "postgres")]
#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::postgres::oidc_sessions)]
struct NewOidcSession {
    pub id: Uuid,
    pub key_id: Uuid,
    pub provider: String,
    pub refresh_enc: Option<Vec<u8>>,
    pub expires_at: Option<chrono::NaiveDateTime>,
}

/// DAL for the encrypted refresh-token store. Postgres only.
pub struct OidcSessionDAL<'a> {
    dal: &'a DAL,
}

impl<'a> OidcSessionDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Store an encrypted refresh token for a minted key. Encrypts under
    /// `enc_key` (32 bytes); the plaintext never hits the DB or a log.
    #[cfg(feature = "postgres")]
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

    /// Fetch + decrypt the refresh session for a minted key, if any.
    #[cfg(feature = "postgres")]
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

    /// Open a login session for a freshly minted key (CLOACI-T-0923).
    ///
    /// `expires_at` is the session's **absolute** deadline — refresh extends
    /// the *key*, never this. No IdP token is stored (`refresh_enc` stays
    /// NULL); see the module docs for why. Lapsed rows are pruned
    /// opportunistically first, so the table stays bounded without a dedicated
    /// sweeper (mirrors the `ws_tickets` hygiene pattern).
    #[cfg(feature = "postgres")]
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

    /// Look up the session a minted key belongs to, without touching
    /// `refresh_enc` (so no encryption key is needed on the refresh path).
    /// Returns `(provider, absolute_expiry)`.
    #[cfg(feature = "postgres")]
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

    /// Point an existing session at a newly minted key (refresh rotation).
    ///
    /// The session's `expires_at` is deliberately **not** extended: the
    /// absolute deadline is the whole point of the record. Returns true when a
    /// session was rotated.
    #[cfg(feature = "postgres")]
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

    /// Forget a minted key's refresh session (logout). Returns true if removed.
    #[cfg(feature = "postgres")]
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

    /// Delete all sessions whose `expires_at` has passed. Returns the count.
    /// Run periodically by the server sweeper (mirrors the ws-ticket/outbox
    /// hygiene pattern). NULL `expires_at` = never swept here.
    #[cfg(feature = "postgres")]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_encrypt_decrypt_roundtrip() {
        let key = [0x11u8; 32];
        let token = b"refresh-token-abc.def.ghi";
        let enc = encrypt_token(token, &key).expect("encrypt");
        assert_ne!(&enc[..], &token[..], "must not store plaintext");
        let dec = decrypt_token(&enc, &key).expect("decrypt");
        assert_eq!(&dec, token);
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let token = b"secret";
        let enc = encrypt_token(token, &[0x11u8; 32]).unwrap();
        assert!(decrypt_token(&enc, &[0x22u8; 32]).is_err());
    }

    #[test]
    fn bad_key_length_errors() {
        assert!(encrypt_token(b"x", &[0u8; 16]).is_err());
    }
}
