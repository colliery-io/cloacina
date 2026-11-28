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
            .create_key(&hash, "test-key", None, false, "admin")
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
            .create_key(&hash, "list-test-key", None, false, "admin")
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
            .create_key(&hash, "revoke-test", None, false, "admin")
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

    /// CLOACINA-T-1065: the last live platform admin key is not revoked; a
    /// tenant key does not count as one, and the bootstrap check follows.
    #[tokio::test]
    #[serial]
    async fn test_the_last_platform_admin_key_is_kept() {
        use cloacina::dal::unified::api_keys::RevokeOutcome;

        let fixture = get_or_init_fixture().await;
        let mut fixture = fixture.lock().unwrap();
        fixture.reset_database().await;
        fixture.initialize().await;
        let dal = fixture.get_dal();
        let keys = dal.api_keys();

        // Only a tenant admin key: no live platform admin key.
        let (_, tenant_hash) = generate_api_key();
        let tenant_admin = keys
            .create_key(&tenant_hash, "tenant-admin", Some("acme"), false, "admin")
            .await
            .expect("create tenant key");
        assert!(!keys.has_live_platform_admin_key().await.unwrap());

        let (_, first_hash) = generate_api_key();
        let first = keys
            .create_key(&first_hash, "first-admin", None, true, "admin")
            .await
            .expect("create first admin");
        assert!(keys.has_live_platform_admin_key().await.unwrap());

        // The only platform admin key stays.
        assert_eq!(
            keys.revoke_key_keeping_platform_admin(first.id)
                .await
                .unwrap(),
            RevokeOutcome::LastPlatformAdmin
        );
        assert!(keys.validate_hash(&first_hash).await.unwrap().is_some());

        // With a second one, the first can go; then the second is the last.
        let (_, second_hash) = generate_api_key();
        let second = keys
            .create_key(&second_hash, "second-admin", None, true, "admin")
            .await
            .expect("create second admin");
        assert_eq!(
            keys.revoke_key_keeping_platform_admin(first.id)
                .await
                .unwrap(),
            RevokeOutcome::Revoked
        );
        assert_eq!(
            keys.revoke_key_keeping_platform_admin(first.id)
                .await
                .unwrap(),
            RevokeOutcome::NotFound
        );
        assert_eq!(
            keys.revoke_key_keeping_platform_admin(second.id)
                .await
                .unwrap(),
            RevokeOutcome::LastPlatformAdmin
        );

        // A tenant key is revoked as before.
        assert_eq!(
            keys.revoke_key_keeping_platform_admin(tenant_admin.id)
                .await
                .unwrap(),
            RevokeOutcome::Revoked
        );

        // Revoked in the database (the recovery step): the bootstrap would run.
        assert!(keys.revoke_key(second.id).await.unwrap());
        assert!(!keys.has_live_platform_admin_key().await.unwrap());
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
            .create_key(&hash, "has-any-test", None, false, "admin")
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
