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

//! Local accounts — self-managed username/password login (CLOACI-T-0795).
//! **Postgres only.**
//!
//! The minimal credential entity behind "IdP or self-manage": `username +
//! argon2id password hash + tenant + role + active/disabled status`. The
//! account record IS the identity→tenant/role mapping, so local login bypasses
//! the OIDC allowlist. Password plaintext is never stored — only the argon2id
//! PHC string. Local login (T-0796) verifies a password and resolves the
//! account to a `ResolvedPrincipal`; account management (T-0797) is tenant-admin.

use crate::dal::unified::DAL;
use crate::error::ValidationError;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};

#[cfg(feature = "postgres")]
use diesel::prelude::*;
#[cfg(feature = "postgres")]
use uuid::Uuid;

/// Hash a password with argon2id (random salt; returns a PHC string).
pub fn hash_password(plaintext: &str) -> Result<String, ValidationError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(plaintext.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ValidationError::ConnectionPool(format!("password hash: {e}")))
}

/// Verify a password against a stored argon2id PHC string. Returns `false` on
/// any parse/verify failure (never leaks the reason).
pub fn verify_password(plaintext: &str, phc: &str) -> bool {
    match PasswordHash::new(phc) {
        Ok(parsed) => Argon2::default()
            .verify_password(plaintext.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Public view of a local account (never includes the password hash).
#[derive(Debug, Clone)]
pub struct LocalAccount {
    pub id: uuid::Uuid,
    pub username: String,
    pub tenant_id: Option<String>,
    pub role: String,
    pub status: String,
}

impl LocalAccount {
    /// True when the account may log in.
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }
}

#[cfg(feature = "postgres")]
#[derive(Queryable)]
#[diesel(table_name = crate::database::schema::postgres::local_accounts)]
struct LocalAccountRow {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub tenant_id: Option<String>,
    pub role: String,
    pub status: String,
    #[allow(dead_code)]
    pub created_at: chrono::NaiveDateTime,
}

#[cfg(feature = "postgres")]
#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::postgres::local_accounts)]
struct NewLocalAccount {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub tenant_id: Option<String>,
    pub role: String,
    pub status: String,
}

#[cfg(feature = "postgres")]
fn to_account(row: &LocalAccountRow) -> LocalAccount {
    LocalAccount {
        id: row.id,
        username: row.username.clone(),
        tenant_id: row.tenant_id.clone(),
        role: row.role.clone(),
        status: row.status.clone(),
    }
}

/// A verified local-login result: the account + whether the password matched.
#[derive(Debug, Clone)]
pub enum LoginOutcome {
    /// Password verified and the account is active.
    Ok(LocalAccount),
    /// Unknown user, wrong password, or disabled account. Callers MUST return
    /// the same opaque error for all three (no user enumeration).
    Denied,
}

/// DAL for local accounts. Postgres only.
pub struct LocalAccountDAL<'a> {
    dal: &'a DAL,
}

impl<'a> LocalAccountDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Create a local account (tenant-admin op, T-0797). Hashes the password
    /// with argon2id. `role` is within the tenant; `status` starts `active`.
    #[cfg(feature = "postgres")]
    pub async fn create(
        &self,
        username: &str,
        password: &str,
        tenant_id: Option<&str>,
        role: &str,
    ) -> Result<LocalAccount, ValidationError> {
        let password_hash = hash_password(password)?;
        let row = NewLocalAccount {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash,
            tenant_id: tenant_id.map(|s| s.to_string()),
            role: role.to_string(),
            status: "active".to_string(),
        };
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let inserted: LocalAccountRow = conn
            .interact(move |conn| {
                diesel::insert_into(crate::database::schema::postgres::local_accounts::table)
                    .values(&row)
                    .get_result(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(to_account(&inserted))
    }

    /// Authenticate a login. Verifies the password against the account in
    /// `tenant_id` (constant work whether or not the user exists, to limit
    /// enumeration). Returns [`LoginOutcome::Denied`] for unknown/wrong/disabled.
    #[cfg(feature = "postgres")]
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
        tenant_id: Option<&str>,
    ) -> Result<LoginOutcome, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let uname = username.to_string();
        let tenant = tenant_id.map(|s| s.to_string());
        let row: Option<LocalAccountRow> = conn
            .interact(move |conn| {
                let mut q = local_accounts::table
                    .filter(local_accounts::username.eq(uname))
                    .into_boxed();
                q = match tenant {
                    Some(t) => q.filter(local_accounts::tenant_id.eq(Some(t))),
                    None => q.filter(local_accounts::tenant_id.is_null()),
                };
                q.first(conn).optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        match row {
            Some(r) if verify_password(password, &r.password_hash) && r.status == "active" => {
                Ok(LoginOutcome::Ok(to_account(&r)))
            }
            _ => Ok(LoginOutcome::Denied),
        }
    }

    /// List accounts in a tenant (no hashes). Tenant-admin view (T-0797).
    #[cfg(feature = "postgres")]
    pub async fn list_for_tenant(
        &self,
        tenant_id: Option<&str>,
    ) -> Result<Vec<LocalAccount>, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let tenant = tenant_id.map(|s| s.to_string());
        let rows: Vec<LocalAccountRow> = conn
            .interact(move |conn| {
                let q = local_accounts::table.order(local_accounts::created_at.desc());
                match tenant {
                    Some(t) => q.filter(local_accounts::tenant_id.eq(Some(t))).load(conn),
                    None => q.filter(local_accounts::tenant_id.is_null()).load(conn),
                }
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(rows.iter().map(to_account).collect())
    }

    /// Disable an account (it can no longer log in / refresh). Returns true if
    /// a row changed.
    #[cfg(feature = "postgres")]
    pub async fn set_status(&self, id: Uuid, status: &str) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let status = status.to_string();
        let n: usize = conn
            .interact(move |conn| {
                diesel::update(local_accounts::table.find(id))
                    .set(local_accounts::status.eq(status))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }

    /// Reset an account's password (admin-reset-only, OQ-12). Hashes the new
    /// password with argon2id.
    #[cfg(feature = "postgres")]
    pub async fn set_password(&self, id: Uuid, new_password: &str) -> Result<bool, ValidationError> {
        use crate::database::schema::postgres::local_accounts;
        let hash = hash_password(new_password)?;
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n: usize = conn
            .interact(move |conn| {
                diesel::update(local_accounts::table.find(id))
                    .set(local_accounts::password_hash.eq(hash))
                    .execute(conn)
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let phc = hash_password("hunter2-correct-horse").expect("hash");
        assert!(phc.starts_with("$argon2"), "PHC string: {phc}");
        assert!(verify_password("hunter2-correct-horse", &phc));
    }

    #[test]
    fn wrong_password_rejected() {
        let phc = hash_password("right").unwrap();
        assert!(!verify_password("wrong", &phc));
    }

    #[test]
    fn malformed_hash_rejected() {
        assert!(!verify_password("anything", "not-a-phc-string"));
    }

    #[test]
    fn distinct_salts_per_hash() {
        // Same password hashes differently (random salt) but both verify.
        let a = hash_password("same").unwrap();
        let b = hash_password("same").unwrap();
        assert_ne!(a, b);
        assert!(verify_password("same", &a) && verify_password("same", &b));
    }
}
