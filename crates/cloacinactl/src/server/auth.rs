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
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::warn;

use cloacina::dal::unified::api_keys::ApiKeyInfo;

use crate::commands::serve::AppState;

/// Authenticated key info inserted into request extensions.
#[derive(Clone, Debug)]
pub struct AuthenticatedKey {
    pub key_id: uuid::Uuid,
    pub name: String,
    pub permissions: String,
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
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "missing or malformed Authorization header"})),
            )
                .into_response();
        }
    };

    let hash = cloacina::security::api_keys::hash_api_key(&token);

    // Check cache first (avoids DB hit)
    if let Some(info) = state.key_cache.get(&hash).await {
        request.extensions_mut().insert(AuthenticatedKey {
            key_id: info.id,
            name: info.name,
            permissions: info.permissions,
        });
        return next.run(request).await;
    }

    // Cache miss — check DB
    let dal = cloacina::dal::DAL::new(state.database.clone());
    match dal.api_keys().validate_hash(&hash).await {
        Ok(Some(info)) => {
            let auth = AuthenticatedKey {
                key_id: info.id,
                name: info.name.clone(),
                permissions: info.permissions.clone(),
            };
            state.key_cache.insert(hash, info).await;
            request.extensions_mut().insert(auth);
            next.run(request).await
        }
        Ok(None) => {
            warn!("API key validation failed — unknown or revoked key");
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid or revoked API key"})),
            )
                .into_response()
        }
        Err(e) => {
            warn!("API key validation error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal error during authentication"})),
            )
                .into_response()
        }
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
