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

//! OIDC in-flight login state (CLOACI-T-0801). **Postgres only.**
//!
//! Persists the authorization-code flow's `state -> (nonce, pkce_verifier)` so
//! the callback can land on any replica (NFR-003, no sticky sessions). Single
//! use: [`take`](OidcLoginFlowDAL::take) deletes-and-returns in one statement,
//! and only for an unexpired row, so a replayed/expired/unknown `state` yields
//! `None` (the callback then fails closed).

use crate::dal::unified::DAL;
use crate::error::ValidationError;

#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use diesel::prelude::*;

#[cfg(feature = "postgres")]
#[derive(Insertable)]
#[diesel(table_name = crate::database::schema::postgres::oidc_login_flows)]
struct NewLoginFlow {
    state: String,
    nonce: String,
    pkce_verifier: String,
    expires_at: chrono::NaiveDateTime,
}

/// DAL for the OIDC login-flow state. Postgres only.
pub struct OidcLoginFlowDAL<'a> {
    dal: &'a DAL,
}

impl<'a> OidcLoginFlowDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Stash an in-flight login keyed by `state`, valid until `expires_at`.
    #[cfg(feature = "postgres")]
    pub async fn put(
        &self,
        state: String,
        nonce: String,
        pkce_verifier: String,
        expires_at: DateTime<Utc>,
    ) -> Result<(), ValidationError> {
        let row = NewLoginFlow {
            state,
            nonce,
            pkce_verifier,
            expires_at: expires_at.naive_utc(),
        };
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        conn.interact(move |conn| {
            diesel::insert_into(crate::database::schema::postgres::oidc_login_flows::table)
                .values(&row)
                .execute(conn)
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(())
    }

    /// Consume the flow for `state` (single use): delete-and-return its
    /// `(nonce, pkce_verifier)` iff the row exists and has not expired. A
    /// missing/expired/replayed `state` returns `None`.
    #[cfg(feature = "postgres")]
    pub async fn take(&self, state: &str) -> Result<Option<(String, String)>, ValidationError> {
        use crate::database::schema::postgres::oidc_login_flows as t;
        let state = state.to_string();
        let now = Utc::now().naive_utc();
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let row: Option<(String, String)> = conn
            .interact(move |conn| {
                diesel::delete(
                    t::table.filter(t::state.eq(state).and(t::expires_at.gt(now))),
                )
                .returning((t::nonce, t::pkce_verifier))
                .get_result(conn)
                .optional()
            })
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(row)
    }

    /// Delete all flows whose `expires_at` has passed. Returns the count.
    /// Run periodically by the server sweeper.
    #[cfg(feature = "postgres")]
    pub async fn sweep_expired(&self) -> Result<usize, ValidationError> {
        use crate::database::schema::postgres::oidc_login_flows as t;
        let now = Utc::now().naive_utc();
        let conn = self
            .dal
            .database
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;
        let n: usize = conn
            .interact(move |conn| diesel::delete(t::table.filter(t::expires_at.lt(now))).execute(conn))
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;
        Ok(n)
    }
}
