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

//! In-memory auth cache with TTL for API key lookups.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Cached API key with pre-loaded permissions and workflow patterns.
#[derive(Debug, Clone)]
pub struct CachedKey {
    pub key_hash: String,
    pub key_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_admin: bool,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub workflow_patterns: Vec<String>,
}

/// Cache entry: either found keys or negative cache.
#[derive(Debug, Clone)]
enum CacheEntry {
    Found {
        keys: Vec<CachedKey>,
        cached_at: Instant,
    },
    NotFound {
        cached_at: Instant,
    },
}

/// In-memory auth cache with configurable TTL.
#[derive(Clone)]
pub struct AuthCache {
    inner: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl: Duration,
}

impl AuthCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Look up cached keys by prefix. Returns None on miss, Some(empty) on negative cache.
    pub fn lookup(&self, prefix: &str) -> Option<Vec<CachedKey>> {
        let cache = self.inner.read();
        match cache.get(prefix) {
            Some(CacheEntry::Found { keys, cached_at }) => {
                if cached_at.elapsed() < self.ttl {
                    Some(keys.clone())
                } else {
                    None // stale
                }
            }
            Some(CacheEntry::NotFound { cached_at }) => {
                if cached_at.elapsed() < self.ttl {
                    Some(vec![]) // negative cache hit
                } else {
                    None // stale
                }
            }
            None => None, // miss
        }
    }

    /// Insert found keys into cache.
    pub fn insert(&self, prefix: String, keys: Vec<CachedKey>) {
        let mut cache = self.inner.write();
        cache.insert(
            prefix,
            CacheEntry::Found {
                keys,
                cached_at: Instant::now(),
            },
        );
    }

    /// Insert negative cache entry (prefix not found in DB).
    pub fn insert_not_found(&self, prefix: String) {
        let mut cache = self.inner.write();
        cache.insert(
            prefix,
            CacheEntry::NotFound {
                cached_at: Instant::now(),
            },
        );
    }

    /// Invalidate a specific prefix (e.g., after key creation or revocation).
    pub fn invalidate(&self, prefix: &str) {
        let mut cache = self.inner.write();
        cache.remove(prefix);
    }

    /// Clear the entire cache (e.g., after key revocation when prefix is unknown).
    pub fn clear(&self) {
        let mut cache = self.inner.write();
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cached_key(name: &str) -> CachedKey {
        CachedKey {
            key_hash: format!("hash_{}", name),
            key_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            can_read: true,
            can_write: false,
            can_execute: false,
            can_admin: false,
            expires_at: None,
            revoked_at: None,
            workflow_patterns: vec!["etl::*".to_string()],
        }
    }

    #[test]
    fn test_insert_and_lookup() {
        let cache = AuthCache::new(Duration::from_secs(60));
        let key = make_cached_key("test");
        cache.insert("live_acme".to_string(), vec![key.clone()]);

        let result = cache.lookup("live_acme");
        assert!(result.is_some());
        let keys = result.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_hash, "hash_test");
    }

    #[test]
    fn test_ttl_expiry() {
        let cache = AuthCache::new(Duration::from_millis(50));
        let key = make_cached_key("ttl");
        cache.insert("live_acme".to_string(), vec![key]);

        // Should be found immediately
        assert!(cache.lookup("live_acme").is_some());

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(80));

        // Should be stale now
        assert!(cache.lookup("live_acme").is_none());
    }

    #[test]
    fn test_negative_cache() {
        let cache = AuthCache::new(Duration::from_secs(60));
        cache.insert_not_found("unknown_prefix".to_string());

        let result = cache.lookup("unknown_prefix");
        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_invalidation() {
        let cache = AuthCache::new(Duration::from_secs(60));
        let key = make_cached_key("inv");
        cache.insert("live_acme".to_string(), vec![key]);

        assert!(cache.lookup("live_acme").is_some());

        cache.invalidate("live_acme");

        assert!(cache.lookup("live_acme").is_none());
    }

    #[test]
    fn test_miss_returns_none() {
        let cache = AuthCache::new(Duration::from_secs(60));
        assert!(cache.lookup("nonexistent").is_none());
    }
}
