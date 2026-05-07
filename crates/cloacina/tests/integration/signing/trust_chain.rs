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

//! Trust chain unit tests.
//!
//! The DB-backed trust-chain ACL tests (direct trust, parent→child grants,
//! cross-org isolation, revocation) were removed in CLOACI-T-0569 — they were
//! `todo!()` placeholders covering multi-org SaaS scenarios that are deferred
//! per CLOACI-A-0005 (trust model: server is platform/enterprise, multi-org
//! is out of scope until CLOACI-I-0106 matures). Re-introduce them when
//! per-tenant trust becomes a shipping feature.

use cloacina::crypto::generate_signing_keypair;

#[test]
fn test_key_fingerprint_computation() {
    let keypair = generate_signing_keypair();

    // Verify fingerprint is 64 hex characters (SHA256)
    assert_eq!(keypair.fingerprint.len(), 64);
    assert!(keypair.fingerprint.chars().all(|c| c.is_ascii_hexdigit()));

    // Verify fingerprint is deterministic
    let fingerprint_again = cloacina::crypto::compute_key_fingerprint(&keypair.public_key);
    assert_eq!(keypair.fingerprint, fingerprint_again);
}

#[test]
fn test_different_keys_have_different_fingerprints() {
    let keypair1 = generate_signing_keypair();
    let keypair2 = generate_signing_keypair();

    assert_ne!(keypair1.fingerprint, keypair2.fingerprint);
    assert_ne!(keypair1.public_key, keypair2.public_key);
}
