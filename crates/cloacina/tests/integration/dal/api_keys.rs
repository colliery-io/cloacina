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

//! Integration tests for the API key DAL (Postgres only).

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_fixture;
    use cloacina::security::api_keys::{generate_api_key, hash_api_key};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_create_and_validate_key() {
        let fixture = get_or_init_fixture().await;
        let fixture = fixture.lock().unwrap();
        let dal = fixture.get_dal();

        let (plaintext, hash) = generate_api_key();
        let info = dal
            .api_keys()
            .create_key(&hash, "test-key")
            .await
            .expect("Failed to create key");

        assert_eq!(info.name, "test-key");
        assert_eq!(info.permissions, "admin");
        assert!(!info.revoked);

        // Validate with correct hash
        let validated = dal
            .api_keys()
            .validate_hash(&hash)
            .await
            .expect("Failed to validate");
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().id, info.id);

        // Validate with hash of plaintext (same thing)
        let rehash = hash_api_key(&plaintext);
        assert_eq!(rehash, hash);
    }

    #[tokio::test]
    #[serial]
    async fn test_validate_unknown_hash_returns_none() {
        let fixture = get_or_init_fixture().await;
        let fixture = fixture.lock().unwrap();
        let dal = fixture.get_dal();

        let result = dal
            .api_keys()
            .validate_hash("nonexistent_hash_value")
            .await
            .expect("Failed to validate");
        assert!(result.is_none());
    }

    #[tokio::test]
    #[serial]
    async fn test_list_keys() {
        let fixture = get_or_init_fixture().await;
        let fixture = fixture.lock().unwrap();
        let dal = fixture.get_dal();

        let (_, hash) = generate_api_key();
        dal.api_keys()
            .create_key(&hash, "list-test-key")
            .await
            .expect("Failed to create key");

        let keys = dal
            .api_keys()
            .list_keys()
            .await
            .expect("Failed to list keys");
        assert!(!keys.is_empty());

        let found = keys.iter().any(|k| k.name == "list-test-key");
        assert!(found, "Created key should appear in list");
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_key() {
        let fixture = get_or_init_fixture().await;
        let fixture = fixture.lock().unwrap();
        let dal = fixture.get_dal();

        let (_, hash) = generate_api_key();
        let info = dal
            .api_keys()
            .create_key(&hash, "revoke-test")
            .await
            .expect("Failed to create key");

        // Revoke it
        let revoked = dal
            .api_keys()
            .revoke_key(info.id)
            .await
            .expect("Failed to revoke");
        assert!(revoked);

        // Validate should return None for revoked key
        let validated = dal
            .api_keys()
            .validate_hash(&hash)
            .await
            .expect("Failed to validate");
        assert!(validated.is_none(), "Revoked key should not validate");

        // Revoking again returns false
        let revoked_again = dal
            .api_keys()
            .revoke_key(info.id)
            .await
            .expect("Failed to revoke again");
        assert!(!revoked_again);
    }

    #[tokio::test]
    #[serial]
    async fn test_has_any_keys() {
        let fixture = get_or_init_fixture().await;
        let fixture = fixture.lock().unwrap();
        let dal = fixture.get_dal();

        // After creating at least one key, has_any_keys should be true
        let (_, hash) = generate_api_key();
        dal.api_keys()
            .create_key(&hash, "has-any-test")
            .await
            .expect("Failed to create key");

        let has = dal
            .api_keys()
            .has_any_keys()
            .await
            .expect("Failed to check");
        assert!(has);
    }
}
