/*
 *  Copyright 2026 Colliery Software
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

//! API key generation and hashing utilities.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Generates a new API key, returning `(plaintext, hash)`.
///
/// The plaintext has the form `clk_` followed by 32 random bytes encoded as
/// base64url (no padding). The hash is the lowercase hex SHA-256 digest of the
/// full plaintext string.
pub fn generate_api_key() -> (String, String) {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);

    let plaintext = format!("clk_{}", URL_SAFE_NO_PAD.encode(bytes));
    let hash = hash_api_key(&plaintext);
    (plaintext, hash)
}

/// Returns the lowercase hex SHA-256 hash of an API key string.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_api_key_format() {
        let (plaintext, hash) = generate_api_key();
        assert!(plaintext.starts_with("clk_"));
        // 32 bytes base64url = 43 chars, plus "clk_" prefix = 47
        assert_eq!(plaintext.len(), 47);
        // SHA-256 hex = 64 chars
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let key = "clk_test1234567890";
        assert_eq!(hash_api_key(key), hash_api_key(key));
    }

    #[test]
    fn test_generate_api_key_uniqueness() {
        let (a, _) = generate_api_key();
        let (b, _) = generate_api_key();
        assert_ne!(a, b);
    }
}
