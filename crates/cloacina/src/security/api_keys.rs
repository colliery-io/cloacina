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

//! PAK (Prefixed API Key) generation and verification.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::RngCore;

/// Generate a new API key.
///
/// Returns (full_key, prefix, hash).
/// The full_key is the secret shown to the user once.
/// The prefix is used for cache lookup.
/// The hash is stored in the database.
pub fn generate_api_key(env: &str, tenant_name: &str) -> Result<(String, String, String), String> {
    // Generate 16 random bytes
    let mut random_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut random_bytes);
    let random_hex = hex::encode(random_bytes);

    // Format: cloacina_<env>_<tenant>_<random>
    let full_key = format!("cloacina_{}_{}_{}", env, tenant_name, random_hex);

    // Prefix: env_tenant
    let prefix = extract_prefix(&full_key);

    // Hash with argon2
    let hash = hash_key(&full_key)?;

    Ok((full_key, prefix, hash))
}

/// Extract the prefix from a full PAK key for cache lookup.
pub fn extract_prefix(full_key: &str) -> String {
    // Take the env_tenant portion: cloacina_<env>_<tenant>_<random>
    let parts: Vec<&str> = full_key.splitn(4, '_').collect();
    if parts.len() >= 3 {
        format!("{}_{}", parts[1], parts[2]) // env_tenant
    } else {
        full_key[..full_key.len().min(16)].to_string()
    }
}

/// Hash a key using argon2.
///
/// Returns an error if the argon2 hashing operation fails (should not happen
/// under normal conditions).
pub fn hash_key(key: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(key.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("Failed to hash key: {}", e))
}

/// Verify a key against a stored hash.
pub fn verify_key(key: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(key.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key_format() {
        let (key, prefix, hash) = generate_api_key("live", "acme").unwrap();
        assert!(key.starts_with("cloacina_live_acme_"));
        assert!(!prefix.is_empty());
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_verify_key_correct() {
        let (key, _, hash) = generate_api_key("test", "demo").unwrap();
        assert!(verify_key(&key, &hash));
    }

    #[test]
    fn test_verify_key_wrong() {
        let (_, _, hash) = generate_api_key("test", "demo").unwrap();
        assert!(!verify_key("wrong_key", &hash));
    }

    #[test]
    fn test_extract_prefix() {
        let prefix = extract_prefix("cloacina_live_acme_k7f3a9b2c1d4e5f6");
        assert_eq!(prefix, "live_acme");
    }

    #[test]
    fn test_extract_prefix_global() {
        let prefix = extract_prefix("cloacina_live__k7f3a9b2c1d4e5f6");
        assert_eq!(prefix, "live_");
    }

    #[test]
    fn test_unique_keys() {
        let (key1, _, _) = generate_api_key("live", "acme").unwrap();
        let (key2, _, _) = generate_api_key("live", "acme").unwrap();
        assert_ne!(key1, key2);
    }
}
