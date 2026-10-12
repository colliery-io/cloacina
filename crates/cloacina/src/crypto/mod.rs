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

//! Cryptographic utilities for package signing.
//!
//! This module provides:
//! - Ed25519 key generation and signing
//! - AES-256-GCM encryption for private key storage at rest
//! - Key fingerprint computation

pub mod envelope;
mod key_encryption;
mod signing;

pub use envelope::{
    EnvelopeError, EphemeralKeypair, EphemeralPrivateKey, generate_ephemeral_keypair,
    unwrap as envelope_unwrap, wrap as envelope_wrap,
};
pub use key_encryption::{
    KeyEncryptionError, decrypt_bytes, decrypt_private_key, encrypt_bytes, encrypt_private_key,
};
pub use signing::{
    GeneratedKeypair, SigningError, compute_key_fingerprint, generate_signing_keypair,
    sign_package, verify_signature,
};

/// Values written by builds before the crypto crate upgrade (CLOACINA-T-0963),
/// pinned so a dependency bump can't make stored data unreadable: secrets
/// and private keys (AES-256-GCM), password hashes (argon2id PHC), package
/// signatures (Ed25519 over SHA-256) and API key hashes (SHA-256).
#[cfg(test)]
mod stored_data_compat {
    const AES_KEY: [u8; 32] = [7u8; 32];
    const AES_CT: &str = "a0d039ef6fed8e6dde59526a8a967fca51c7c1d20af00e6fe9a2fabc7fe6327de633515f2fef05f81b51f371ce6ccc0815f342fd65";
    const ARGON2_PHC: &str = "$argon2id$v=19$m=19456,t=2,p=1$qQ9KbJArHa/1rkOGvTuFtQ$m936iDe0xF7jXmqPVnelEOGqSWIww8vcSjQgiImS61c";
    const PKG_HASH: &str = "c86c0785e5968d24080f79a3ea56498c6ce85911e1c8644f0f9fab490e254beb";
    const SIG: &str = "51aa8f78da361442de6da2721e38ffa550cf9576ffd3fdbb441395b123493efc013e13ad768e5ff5b1318ad2e2f2524dec47bea705d3d9ed41aa31530aeb2207";
    const PK: &str = "fd1724385aa0c75b64fb78cd602fa1d991fdebf76b13c58ed702eac835e9f618";
    const FINGERPRINT: &str = "dbc298251c51321b7266e78d1c151c2b62aff8cb95b293096d3463018544face";
    const APIKEY_HASH: &str = "5ac1490041bec4c73fb4ce713722a72aaebbf21314d01a985335c9e0acbbbe8b";

    #[test]
    fn secret_encrypted_by_earlier_build_decrypts() {
        let ct = hex::decode(AES_CT).unwrap();
        let pt = super::decrypt_bytes(&ct, &AES_KEY).expect("decrypts");
        assert_eq!(pt, b"cloacina stored secret v1");
    }

    // Local accounts exist only with the postgres feature.
    #[cfg(feature = "postgres")]
    #[test]
    fn password_hash_from_earlier_build_verifies() {
        use crate::dal::unified::local_accounts::verify_password;
        assert!(verify_password("correct horse battery staple", ARGON2_PHC));
        assert!(!verify_password("wrong password", ARGON2_PHC));
    }

    #[test]
    fn package_signature_from_earlier_build_verifies() {
        let hash = super::signing::compute_package_hash(b"cloacina package bytes v1");
        assert_eq!(hash, PKG_HASH);
        let sig = hex::decode(SIG).unwrap();
        let pk = hex::decode(PK).unwrap();
        super::verify_signature(hash.as_bytes(), &sig, &pk).expect("signature verifies");
        assert_eq!(super::compute_key_fingerprint(&pk), FINGERPRINT);
    }

    #[test]
    fn signing_is_deterministic_across_versions() {
        // Ed25519 is deterministic: the same key and message give the same
        // signature, so a new build must reproduce the stored one exactly.
        let sig = super::sign_package(PKG_HASH.as_bytes(), &[9u8; 32]).unwrap();
        assert_eq!(hex::encode(sig), SIG);
    }

    #[test]
    fn api_key_hash_is_unchanged() {
        assert_eq!(
            crate::security::api_keys::hash_api_key("clk_fixture_key_0001"),
            APIKEY_HASH
        );
    }
}
