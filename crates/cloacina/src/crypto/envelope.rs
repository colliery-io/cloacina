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

//! Per-execution HPKE envelope wrap for fleet secret resolution (CLOACI-T-0861).
//!
//! Implements the wrap-to-ephemeral-public-key step of the fleet secret
//! resolution design (I-0133 D-2/D-5/D-6, NFR-003): a remote agent generates a
//! fresh ephemeral X25519 keypair per task claim and advertises the public key;
//! the server resolves the at-rest secret and HPKE-wraps (RFC 9180) the value to
//! that public key; the agent unwraps with its ephemeral private key into the
//! in-memory `Secrets` accessor and never persists the plaintext.
//!
//! ## Suite
//!
//! A single fixed RFC 9180 ciphersuite is used (crypto agility is deferred — the
//! wire format could carry a suite id later):
//!
//! - **KEM:** X25519-HKDF-SHA256 (`0x0020`)
//! - **KDF:** HKDF-SHA256 (`0x0001`)
//! - **AEAD:** ChaCha20Poly1305 (`0x0003`)
//!
//! ## Binding
//!
//! Each wrap is bound to one recipient by construction (the ephemeral public
//! key), and additionally to an application context via the HPKE `info` string
//! and per-message AEAD associated data (`aad`). Callers SHOULD pass an `aad`
//! that names the execution + secret so a captured ciphertext cannot be replayed
//! against a different execution even to the same key.

use hpke::{
    aead::ChaCha20Poly1305, kdf::HkdfSha256, kem::X25519HkdfSha256, Deserializable,
    Kem as KemTrait, OpModeR, OpModeS, Serializable,
};
use thiserror::Error;

/// KEM: X25519 with HKDF-SHA256.
type Kem = X25519HkdfSha256;
/// KDF: HKDF-SHA256.
type Kdf = HkdfSha256;
/// AEAD: ChaCha20Poly1305.
type Aead = ChaCha20Poly1305;

/// Domain-separation string mixed into every HPKE context for this application.
/// Distinguishes Cloacina fleet-secret wraps from any other HPKE use and pins
/// the wire-format version.
const HPKE_INFO: &[u8] = b"cloacina/fleet-secret-envelope/v1";

/// Errors from envelope wrap/unwrap.
#[derive(Debug, Error)]
pub enum EnvelopeError {
    /// The recipient public key bytes were not a valid X25519 public key.
    #[error("invalid recipient public key")]
    InvalidPublicKey,

    /// The encapsulated key bytes (`enc`) were malformed.
    #[error("invalid encapsulated key")]
    InvalidEncappedKey,

    /// HPKE seal (wrap) failed.
    #[error("envelope wrap failed: {0}")]
    Wrap(String),

    /// HPKE open (unwrap) failed — wrong key, tampered ciphertext, or bad aad.
    #[error("envelope unwrap failed: {0}")]
    Unwrap(String),
}

/// The recipient (agent) half of a per-execution keypair.
///
/// Holds the ephemeral X25519 private key. Never serialized to the wire, never
/// persisted. Only the paired public key ([`EphemeralKeypair::public_key_bytes`])
/// leaves the agent.
pub struct EphemeralPrivateKey(<Kem as KemTrait>::PrivateKey);

impl std::fmt::Debug for EphemeralPrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render private key material.
        f.write_str("EphemeralPrivateKey(<redacted>)")
    }
}

/// A freshly generated ephemeral keypair for one task claim/execution.
///
/// The agent keeps `private` and sends `public_key_bytes` to the server. The
/// server wraps secrets to `public_key_bytes`; the agent unwraps with `private`.
#[derive(Debug)]
pub struct EphemeralKeypair {
    /// The private half — kept by the agent, never leaves the process.
    pub private: EphemeralPrivateKey,
    /// The serialized X25519 public key — advertised to the server so it can
    /// wrap secrets to this execution.
    pub public_key_bytes: Vec<u8>,
}

/// Generate a fresh ephemeral X25519 keypair (D-5: per task claim).
///
/// Keygen is microseconds; call it once per claim so a leaked key exposes at
/// most one execution's secrets.
pub fn generate_ephemeral_keypair() -> EphemeralKeypair {
    let mut rng = rand::thread_rng();
    let (private, public) = Kem::gen_keypair(&mut rng);
    EphemeralKeypair {
        private: EphemeralPrivateKey(private),
        public_key_bytes: public.to_bytes().to_vec(),
    }
}

/// HPKE-wrap `plaintext` to `recipient_public_key` (single-shot seal).
///
/// Returns `(enc, ciphertext)` where `enc` is the HPKE encapsulated key and
/// `ciphertext` is the AEAD-sealed payload. Both are needed to unwrap. `aad` is
/// authenticated but not encrypted — bind it to the execution + secret name.
///
/// Only `(enc, ciphertext)` should cross the wire; the plaintext never does.
pub fn wrap(
    recipient_public_key: &[u8],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), EnvelopeError> {
    let pk_recip = <Kem as KemTrait>::PublicKey::from_bytes(recipient_public_key)
        .map_err(|_| EnvelopeError::InvalidPublicKey)?;

    let mut rng = rand::thread_rng();
    let (encapped, ciphertext) = hpke::single_shot_seal::<Aead, Kdf, Kem, _>(
        &OpModeS::Base,
        &pk_recip,
        HPKE_INFO,
        plaintext,
        aad,
        &mut rng,
    )
    .map_err(|e| EnvelopeError::Wrap(e.to_string()))?;

    Ok((encapped.to_bytes().to_vec(), ciphertext))
}

/// HPKE-unwrap a `(enc, ciphertext)` pair with the recipient private key.
///
/// `aad` MUST match the value passed to [`wrap`] exactly, or the AEAD open
/// fails. Fails closed on a wrong key, tampered ciphertext, or mismatched aad.
pub fn unwrap(
    recipient_private_key: &EphemeralPrivateKey,
    enc: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, EnvelopeError> {
    let encapped = <Kem as KemTrait>::EncappedKey::from_bytes(enc)
        .map_err(|_| EnvelopeError::InvalidEncappedKey)?;

    hpke::single_shot_open::<Aead, Kdf, Kem>(
        &OpModeR::Base,
        &recipient_private_key.0,
        &encapped,
        HPKE_INFO,
        ciphertext,
        aad,
    )
    .map_err(|e| EnvelopeError::Unwrap(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AAD: &[u8] = b"exec-1/db_prod";

    #[test]
    fn round_trip_recovers_plaintext() {
        let kp = generate_ephemeral_keypair();
        let plaintext = b"super-secret-password";

        let (enc, ct) = wrap(&kp.public_key_bytes, plaintext, AAD).expect("wrap");
        // The ciphertext must not equal the plaintext.
        assert_ne!(ct.as_slice(), plaintext.as_slice());

        let recovered = unwrap(&kp.private, &enc, &ct, AAD).expect("unwrap");
        assert_eq!(recovered.as_slice(), plaintext.as_slice());
    }

    #[test]
    fn blob_wrapped_to_a_does_not_unwrap_with_b() {
        let a = generate_ephemeral_keypair();
        let b = generate_ephemeral_keypair();
        let plaintext = b"bound-to-A-only";

        let (enc, ct) = wrap(&a.public_key_bytes, plaintext, AAD).expect("wrap to A");

        // Different keypair (a different agent / different execution) must fail.
        let err = unwrap(&b.private, &enc, &ct, AAD).expect_err("must not unwrap with B");
        assert!(matches!(err, EnvelopeError::Unwrap(_)));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let kp = generate_ephemeral_keypair();
        let (enc, mut ct) = wrap(&kp.public_key_bytes, b"integrity", AAD).expect("wrap");

        // Flip a bit in the AEAD ciphertext.
        ct[0] ^= 0x01;
        assert!(unwrap(&kp.private, &enc, &ct, AAD).is_err());
    }

    #[test]
    fn tampered_encapped_key_fails() {
        let kp = generate_ephemeral_keypair();
        let (mut enc, ct) = wrap(&kp.public_key_bytes, b"integrity", AAD).expect("wrap");

        enc[0] ^= 0x01;
        assert!(unwrap(&kp.private, &enc, &ct, AAD).is_err());
    }

    #[test]
    fn wrong_aad_fails() {
        let kp = generate_ephemeral_keypair();
        let (enc, ct) = wrap(&kp.public_key_bytes, b"aad-bound", AAD).expect("wrap");

        // A ciphertext bound to exec-1/db_prod must not open under a different
        // execution's aad — this is the replay/isolation binding.
        let err =
            unwrap(&kp.private, &enc, &ct, b"exec-2/db_prod").expect_err("aad mismatch must fail");
        assert!(matches!(err, EnvelopeError::Unwrap(_)));
    }

    #[test]
    fn public_key_is_x25519_sized() {
        let kp = generate_ephemeral_keypair();
        // X25519 public keys are 32 bytes.
        assert_eq!(kp.public_key_bytes.len(), 32);
    }

    #[test]
    fn invalid_public_key_rejected() {
        // Too short to be an X25519 public key.
        let err = wrap(&[0u8; 4], b"x", AAD).expect_err("bad pubkey");
        assert!(matches!(err, EnvelopeError::InvalidPublicKey));
    }
}
