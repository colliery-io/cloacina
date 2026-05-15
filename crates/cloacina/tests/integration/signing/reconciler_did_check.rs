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

//! CLOACI-T-0571 — defense-in-depth signature-existence check.
//!
//! When `require_signatures` is on, the reconciler refuses to load any
//! `workflow_packages` row that has no companion `package_signatures`
//! row. The upload route is the strong gate (CLOACI-I-0103); this
//! catches direct DB inserts that bypass it.
//!
//! Tests here exercise the lookup contract end-to-end against a real
//! Postgres fixture: insert a signature, ask the registry, assert the
//! presence/absence flips. The reconciler integration end-to-end (write
//! a workflow_packages row, run the reconciler, assert it refuses) is
//! covered by the existing reconciler harness; this file pins the
//! `find_signature` predicate the reconciler depends on so a future DAL
//! refactor can't silently break the gate.

#[cfg(feature = "postgres")]
mod postgres_tests {
    use crate::fixtures::get_or_init_fixture;
    use cloacina::database::universal_types::{UniversalBinary, UniversalTimestamp, UniversalUuid};
    use cloacina::dal::FilesystemRegistryStorage;
    use cloacina::registry::traits::WorkflowRegistry;
    use cloacina::registry::WorkflowRegistryImpl;
    use serial_test::serial;
    use sha2::{Digest, Sha256};
    use tempfile::TempDir;

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Build a registry impl backed by the test fixture's database.
    /// We don't actually exercise the storage backend here — the test
    /// only touches the signature lookup, which lives on the database
    /// side of the registry.
    fn build_registry(database: cloacina::Database) -> WorkflowRegistryImpl<FilesystemRegistryStorage>
    {
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
        assert!(
            !absent,
            "expected no signature row for fresh hash {}",
            hash
        );

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
        assert!(!other_absent, "unrelated hash must not match the inserted row");

        // Suppress unused-import warnings on the binary type.
        let _bin = UniversalBinary::new(vec![0u8; 1]);
    }
}
