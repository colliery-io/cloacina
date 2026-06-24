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

// CLOACI-T-0792: the mint bridge has no caller until the producers land
// (OIDC callback T-0790/mapping T-0791, local login T-0796). Allow the interim
// dead_code so the crate stays warning-clean until then.
#![allow(dead_code)]

//! Provider-agnostic identity → minted-key bridge (CLOACI-T-0792).
//!
//! Both the OIDC mapping (T-0791) and local login (T-0796) resolve a human
//! login to a [`ResolvedPrincipal`]; [`mint_for_principal`] turns that into a
//! short-TTL cloacina API key via the existing `api_keys` DAL. The minted key
//! is an ordinary bearer key — it flows through `require_auth` + the Phase 0
//! authZ matcher unchanged, and `validate_hash` rejects it once its TTL passes.
//!
//! This module is deliberately **provider-agnostic**: it knows nothing about
//! OIDC or passwords, only the resolved `{tenant, role, provenance}` handoff.

use std::time::Duration;

use cloacina::dal::unified::api_keys::ApiKeyInfo;

use crate::routes::error::ApiError;
use crate::AppState;

/// Default lifetime of a minted key (OQ-3). Short by design — revocation/expiry
/// latency is bounded, and `/auth/refresh` (T-0794) silently re-mints before
/// this elapses so the human stays logged in without a long-lived credential.
pub const DEFAULT_MINTED_KEY_TTL: Duration = Duration::from_secs(15 * 60);

/// The provider-agnostic result of resolving a human login to a cloacina
/// principal. Produced by OIDC mapping (T-0791) or local login (T-0796); the
/// only consumer is [`mint_for_principal`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedPrincipal {
    /// Tenant the minted key is scoped to. `None` == global/public.
    pub tenant: Option<String>,
    /// Role for the minted key (`read` | `write` | `admin`).
    pub role: String,
    /// Provenance recorded on the key, e.g. `oidc:<issuer>:<sub>` or
    /// `local:<account_id>`. Also used as the key's display name.
    pub provenance: String,
}

/// Mint a short-TTL key for a resolved principal. Returns the plaintext key
/// (shown to the caller exactly once) plus its persisted [`ApiKeyInfo`].
///
/// The minted key is never god-mode and is scoped to `principal.tenant` +
/// `principal.role`, exactly as an equivalent manually-created key would be.
pub async fn mint_for_principal(
    state: &AppState,
    principal: &ResolvedPrincipal,
    ttl: Duration,
) -> Result<(String, ApiKeyInfo), ApiError> {
    let (plaintext, hash) = cloacina::security::api_keys::generate_api_key();

    let expires_at = chrono::Utc::now()
        + chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::minutes(15));

    let dal = cloacina::dal::DAL::new(state.database.clone());
    let info = dal
        .api_keys()
        .mint_key(
            &hash,
            &principal.provenance,
            principal.tenant.as_deref(),
            &principal.role,
            expires_at,
            &principal.provenance,
        )
        .await
        .map_err(|e| ApiError::internal(format!("failed to mint key: {e}")))?;

    Ok((plaintext, info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_minted_ttl_is_fifteen_minutes() {
        assert_eq!(DEFAULT_MINTED_KEY_TTL, Duration::from_secs(900));
    }

    #[test]
    fn resolved_principal_carries_scope_and_provenance() {
        let p = ResolvedPrincipal {
            tenant: Some("acme".into()),
            role: "write".into(),
            provenance: "oidc:https://idp.example:sub-123".into(),
        };
        assert_eq!(p.tenant.as_deref(), Some("acme"));
        assert_eq!(p.role, "write");
        assert!(p.provenance.starts_with("oidc:"));

        // A global (no-tenant) principal is valid too.
        let g = ResolvedPrincipal {
            tenant: None,
            role: "read".into(),
            provenance: "local:42".into(),
        };
        assert!(g.tenant.is_none());
        assert!(g.provenance.starts_with("local:"));
    }
}
