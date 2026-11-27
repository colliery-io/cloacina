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

//! CLOACI-T-0571 — the `find_signature` predicate of the registry: whether
//! a `package_signatures` row exists for a package hash.
//!
//! Tests here exercise the lookup against a real Postgres fixture: insert a
//! signature, ask the registry, assert the presence/absence flips.
//!
//! The reconciler's check at load is more than this predicate
//! (CLOACINA-T-1062): it verifies the stored signature against the trusted
//! keys, through `WorkflowRegistry::verify_package_signature`. The tests of
//! that check are in `cloacina::security::verification`, and the tests of a
//! refused and an accepted load in `cloacina-server`
//! (`tests/package_signing.rs`).

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_fixture;
    use cloacina::dal::FilesystemRegistryStorage;
    use cloacina::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
    use cloacina::registry::WorkflowRegistryImpl;
    use cloacina::registry::traits::WorkflowRegistry;
    use serial_test::serial;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    /// Build a registry impl backed by the test fixture's database.
    /// We don't actually exercise the storage backend here — the test
    /// only touches the signature lookup, which lives on the database
    /// side of the registry.
    fn build_registry(
        database: cloacina::Database,
    ) -> WorkflowRegistryImpl<FilesystemRegistryStorage> {
        let tmp = TempDir::new().expect("tempdir");
        let storage = FilesystemRegistryStorage::new(tmp.path()).expect("storage");
        // Leak the tempdir so the storage path remains valid for the
        // duration of the test process. The fixture handles cleanup of
        // the surrounding state.
        std::mem::forget(tmp);
        WorkflowRegistryImpl::new(storage, database).expect("registry")
    }

    /// `find_signature` returns false when no row matches, true once a
    /// `package_signatures` row exists for the given hash.
    #[tokio::test]
    #[serial]
    async fn test_find_signature_present_and_absent() {
        let fixture = get_or_init_fixture().await;
        let fixture = fixture.lock().unwrap();
        let database = fixture.get_database();
        let registry = build_registry(database.clone());

        // Unique hash per run keeps fixtures from poisoning each other.
        let payload = format!("payload-{}", uuid::Uuid::new_v4());
        let hash = sha256_hex(payload.as_bytes());

        // Absent → false.
        let absent = registry
            .find_signature(&hash)
            .await
            .expect("find_signature(absent)");
        assert!(!absent, "expected no signature row for fresh hash {}", hash);

        // Insert a signature row directly via the signer DAL.
        let signer = cloacina::security::DbPackageSigner::new(cloacina::dal::DAL::new(database));
        let info = cloacina::security::PackageSignatureInfo {
            package_hash: hash.clone(),
            key_fingerprint: format!("fp-{}", uuid::Uuid::new_v4().simple()),
            signature: vec![0u8; 64],
            signed_at: UniversalTimestamp::now(),
        };
        let _id: UniversalUuid = cloacina::security::PackageSigner::store_signature(&signer, &info)
            .await
            .expect("store_signature");

        // Present → true.
        let present = registry
            .find_signature(&hash)
            .await
            .expect("find_signature(present)");
        assert!(present, "expected signature row to be visible after insert");

        // Sanity: distinct hash still returns false.
        let other = sha256_hex(format!("other-{}", uuid::Uuid::new_v4()).as_bytes());
        let other_absent = registry
            .find_signature(&other)
            .await
            .expect("find_signature(other)");
        assert!(
            !other_absent,
            "unrelated hash must not match the inserted row"
        );

        // Suppress unused-import warnings on the binary type.
        let _bin = UniversalBinary::new(vec![0u8; 1]);
    }
}
