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

//! Bearer token auth middleware with LRU cache.
//!
//! Extracts `Authorization: Bearer <key>` headers, hashes the token,
//! checks an LRU cache (30s TTL) before falling back to the DAL.
//! Applied via `route_layer` so unauthenticated routes still 404 correctly.

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::warn;

use cloacina::dal::unified::api_keys::ApiKeyInfo;

use crate::commands::serve::AppState;
use crate::server::error::ApiError;

/// Authenticated key info inserted into request extensions.
#[derive(Clone, Debug)]
pub struct AuthenticatedKey {
    pub key_id: uuid::Uuid,
    pub name: String,
    pub permissions: String,
    pub tenant_id: Option<String>,
    pub is_admin: bool,
}

/// A cached entry with TTL tracking.
struct CachedEntry {
    info: ApiKeyInfo,
    inserted_at: Instant,
}

/// LRU cache for validated API key hashes with TTL expiry.
pub struct KeyCache {
    cache: Mutex<LruCache<String, CachedEntry>>,
    ttl: Duration,
}

impl KeyCache {
    /// Create a new key cache.
    /// Default: 256 entries, 30 second TTL.
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("cache capacity must be > 0"),
            )),
            ttl,
        }
    }

    /// Create with default settings (256 entries, 30s TTL).
    pub fn default_cache() -> Self {
        Self::new(256, Duration::from_secs(30))
    }

    /// Look up a key hash. Returns None if not cached or expired.
    pub async fn get(&self, hash: &str) -> Option<ApiKeyInfo> {
        let mut cache = self.cache.lock().await;
        if let Some(entry) = cache.get(hash) {
            if entry.inserted_at.elapsed() < self.ttl {
                return Some(entry.info.clone());
            }
            // Expired — lazy eviction
            cache.pop(hash);
        }
        None
    }

    /// Insert a validated key into the cache.
    pub async fn insert(&self, hash: String, info: ApiKeyInfo) {
        let mut cache = self.cache.lock().await;
        cache.put(
            hash,
            CachedEntry {
                info,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Evict a specific key (used after revocation).
    #[allow(dead_code)]
    pub async fn evict(&self, hash: &str) {
        let mut cache = self.cache.lock().await;
        cache.pop(hash);
    }

    /// Clear all entries.
    pub async fn clear(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }
}

/// Validate a bearer token and return the authenticated key info.
///
/// Shared logic used by both the HTTP middleware and WebSocket handlers.
/// Checks the LRU cache first, then falls back to the DAL.
pub async fn validate_token(
    state: &AppState,
    token: &str,
) -> Result<AuthenticatedKey, (StatusCode, Json<serde_json::Value>)> {
    let hash = cloacina::security::api_keys::hash_api_key(token);

    // Check cache first (avoids DB hit)
    if let Some(info) = state.key_cache.get(&hash).await {
        return Ok(AuthenticatedKey {
            key_id: info.id,
            name: info.name,
            permissions: info.permissions,
            tenant_id: info.tenant_id,
            is_admin: info.is_admin,
        });
    }

    // Cache miss — check DB
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.api_keys().validate_hash(&hash).await {
        Ok(Some(info)) => {
            let auth = AuthenticatedKey {
                key_id: info.id,
                name: info.name.clone(),
                permissions: info.permissions.clone(),
                tenant_id: info.tenant_id.clone(),
                is_admin: info.is_admin,
            };
            state.key_cache.insert(hash, info).await;
            Ok(auth)
        }
        Ok(None) => {
            warn!("API key validation failed — unknown or revoked key");
            Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid or revoked API key"})),
            ))
        }
        Err(e) => {
            warn!("API key validation error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal error during authentication"})),
            ))
        }
    }
}

/// Auth middleware — validates Bearer token against cache then DAL.
///
/// On success, inserts `AuthenticatedKey` into request extensions.
/// On failure, returns 401 JSON error.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer_token(&request) {
        Some(t) => t.to_string(),
        None => {
            return ApiError::unauthorized("missing or malformed Authorization header")
                .into_response();
        }
    };

    match validate_token(&state, &token).await {
        Ok(auth) => {
            request.extensions_mut().insert(auth);
            next.run(request).await
        }
        Err(resp) => resp.into_response(),
    }
}

/// Extract the Bearer token from the Authorization header.
fn extract_bearer_token(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

// ---------------------------------------------------------------------------
// Authorization helpers — used by handlers to enforce tenant and admin checks
// ---------------------------------------------------------------------------

impl AuthenticatedKey {
    /// Check if this key can access the given tenant's resources.
    ///
    /// - Admin keys (is_admin=true) can access any tenant — god mode.
    /// - Tenant-scoped keys can only access their own tenant.
    /// - Global keys (tenant_id=None) can access global/public resources only.
    pub fn can_access_tenant(&self, tenant_id: &str) -> bool {
        if self.is_admin {
            return true;
        }
        match &self.tenant_id {
            Some(key_tenant) => key_tenant == tenant_id,
            None => tenant_id == "public",
        }
    }

    /// Returns a 403 response for tenant access denied.
    pub fn forbidden_response() -> ApiError {
        ApiError::forbidden("tenant_access_denied", "access denied for this tenant")
    }

    /// Returns a 403 response for admin-only operations.
    pub fn admin_required_response() -> ApiError {
        ApiError::forbidden("admin_required", "admin access required")
    }

    /// Check if this key has at least write permission.
    /// Roles: admin > write > read.
    /// God mode (is_admin) implicitly has all permissions.
    pub fn can_write(&self) -> bool {
        self.is_admin || self.permissions == "admin" || self.permissions == "write"
    }

    /// Check if this key has admin role within its tenant.
    /// Note: is_admin (god mode) is separate from permissions="admin" (tenant admin).
    pub fn can_admin(&self) -> bool {
        self.is_admin || self.permissions == "admin"
    }

    /// Returns a 403 response for insufficient role.
    pub fn insufficient_role_response() -> ApiError {
        ApiError::forbidden("insufficient_permissions", "insufficient permissions")
    }
}

// ---------------------------------------------------------------------------
// WebSocket ticket store — short-lived, single-use tickets for WS auth
// ---------------------------------------------------------------------------

use std::collections::HashMap;

/// A single-use, time-limited ticket for WebSocket authentication.
/// Avoids exposing long-lived API keys in query parameters.
struct WsTicket {
    auth: AuthenticatedKey,
    expires_at: Instant,
}

/// Thread-safe store for WebSocket auth tickets.
///
/// Bounded to prevent memory exhaustion from repeated `POST /auth/ws-ticket`
/// calls without consumption. Expired tickets are lazily evicted on `issue()`.
pub struct WsTicketStore {
    tickets: Mutex<HashMap<String, WsTicket>>,
    ttl: Duration,
    max_capacity: usize,
}

impl WsTicketStore {
    /// Create a new ticket store with the given TTL (e.g., 60 seconds).
    pub fn new(ttl: Duration) -> Self {
        Self {
            tickets: Mutex::new(HashMap::new()),
            ttl,
            max_capacity: 1024,
        }
    }

    /// Issue a new ticket for the given authenticated key.
    /// Returns the ticket string (UUID).
    ///
    /// Evicts expired tickets before inserting. If the store is still at
    /// capacity after eviction, the oldest ticket is removed.
    pub async fn issue(&self, auth: AuthenticatedKey) -> String {
        let ticket = uuid::Uuid::new_v4().to_string();
        let entry = WsTicket {
            auth,
            expires_at: Instant::now() + self.ttl,
        };
        let mut store = self.tickets.lock().await;

        // Evict expired tickets
        let now = Instant::now();
        store.retain(|_, v| v.expires_at > now);

        // If still at capacity, drop the oldest entry
        if store.len() >= self.max_capacity {
            if let Some(oldest_key) = store
                .iter()
                .min_by_key(|(_, v)| v.expires_at)
                .map(|(k, _)| k.clone())
            {
                store.remove(&oldest_key);
            }
        }

        store.insert(ticket.clone(), entry);
        ticket
    }

    /// Consume a ticket — returns the authenticated key if valid and not expired.
    /// The ticket is removed on use (single-use).
    pub async fn consume(&self, ticket: &str) -> Option<AuthenticatedKey> {
        let mut store = self.tickets.lock().await;
        if let Some(entry) = store.remove(ticket) {
            if entry.expires_at > Instant::now() {
                return Some(entry.auth);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auth(name: &str) -> AuthenticatedKey {
        AuthenticatedKey {
            key_id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            permissions: "read".to_string(),
            tenant_id: None,
            is_admin: false,
        }
    }

    #[tokio::test]
    async fn test_ticket_issue_and_consume() {
        let store = WsTicketStore::new(Duration::from_secs(60));
        let auth = make_auth("test-key");
        let ticket = store.issue(auth.clone()).await;

        let consumed = store.consume(&ticket).await;
        assert!(consumed.is_some(), "valid ticket should be consumed");
        assert_eq!(consumed.unwrap().name, "test-key");
    }

    #[tokio::test]
    async fn test_ticket_single_use() {
        let store = WsTicketStore::new(Duration::from_secs(60));
        let ticket = store.issue(make_auth("key")).await;

        let first = store.consume(&ticket).await;
        assert!(first.is_some(), "first consume should succeed");

        let second = store.consume(&ticket).await;
        assert!(second.is_none(), "second consume should fail (single-use)");
    }

    #[tokio::test]
    async fn test_ticket_invalid_rejected() {
        let store = WsTicketStore::new(Duration::from_secs(60));
        store.issue(make_auth("key")).await;

        let result = store.consume("not-a-real-ticket").await;
        assert!(result.is_none(), "invalid ticket should be rejected");
    }

    #[tokio::test]
    async fn test_ticket_expired_rejected() {
        let store = WsTicketStore::new(Duration::from_millis(1));
        let ticket = store.issue(make_auth("key")).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        let result = store.consume(&ticket).await;
        assert!(result.is_none(), "expired ticket should be rejected");
    }

    #[tokio::test]
    async fn test_ticket_store_bounded() {
        let store = WsTicketStore::new(Duration::from_secs(60));
        // Override max_capacity for testing
        let store = WsTicketStore {
            max_capacity: 3,
            ..store
        };

        // Issue 4 tickets — store should evict oldest when at capacity
        let _t1 = store.issue(make_auth("key1")).await;
        let _t2 = store.issue(make_auth("key2")).await;
        let _t3 = store.issue(make_auth("key3")).await;
        let t4 = store.issue(make_auth("key4")).await;

        // Store should have at most 3 entries
        let count = store.tickets.lock().await.len();
        assert!(
            count <= 3,
            "store should be bounded to max_capacity, got {}",
            count
        );

        // Most recent ticket should still be valid
        let result = store.consume(&t4).await;
        assert!(
            result.is_some(),
            "most recent ticket should survive eviction"
        );
    }

    #[tokio::test]
    async fn test_ticket_store_evicts_expired_on_issue() {
        let store = WsTicketStore::new(Duration::from_millis(1));

        // Issue tickets that will expire
        store.issue(make_auth("expired1")).await;
        store.issue(make_auth("expired2")).await;

        tokio::time::sleep(Duration::from_millis(10)).await;

        // Issue a new ticket — should evict expired ones
        let store = WsTicketStore {
            tickets: store.tickets,
            ttl: Duration::from_secs(60), // new tickets get long TTL
            max_capacity: store.max_capacity,
        };
        let fresh = store.issue(make_auth("fresh")).await;

        // Only the fresh ticket should remain
        let count = store.tickets.lock().await.len();
        assert_eq!(count, 1, "expired tickets should be evicted on issue");

        let result = store.consume(&fresh).await;
        assert!(result.is_some(), "fresh ticket should be valid");
    }
}
