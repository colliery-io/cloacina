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

//! Domain models for Ed25519 signing keys.
//!
//! Signing keys are used to cryptographically sign workflow packages.
//! Private keys are stored encrypted at rest using AES-256-GCM.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Domain model for a signing key.
///
/// The private key is stored encrypted - use the crypto module to decrypt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningKey {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub key_name: String,
    /// AES-256-GCM encrypted Ed25519 private key (nonce || ciphertext || tag)
    pub encrypted_private_key: Vec<u8>,
    /// Ed25519 public key (32 bytes)
    pub public_key: Vec<u8>,
    /// SHA256 hex fingerprint of the public key
    pub key_fingerprint: String,
    pub created_at: UniversalTimestamp,
    /// None if active, Some if revoked
    pub revoked_at: Option<UniversalTimestamp>,
}

impl SigningKey {
    /// Check if this key is currently active (not revoked)
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }

    /// Check if this key has been revoked
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}

/// Model for creating a new signing key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSigningKey {
    pub org_id: UniversalUuid,
    pub key_name: String,
    pub encrypted_private_key: Vec<u8>,
    pub public_key: Vec<u8>,
    pub key_fingerprint: String,
}

impl NewSigningKey {
    pub fn new(
        org_id: UniversalUuid,
        key_name: String,
        encrypted_private_key: Vec<u8>,
        public_key: Vec<u8>,
        key_fingerprint: String,
    ) -> Self {
        Self {
            org_id,
            key_name,
            encrypted_private_key,
            public_key,
            key_fingerprint,
        }
    }
}

/// Information about a signing key (without the private key material).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningKeyInfo {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    pub key_name: String,
    pub key_fingerprint: String,
    pub created_at: UniversalTimestamp,
    pub revoked_at: Option<UniversalTimestamp>,
}

impl From<SigningKey> for SigningKeyInfo {
    fn from(key: SigningKey) -> Self {
        Self {
            id: key.id,
            org_id: key.org_id,
            key_name: key.key_name,
            key_fingerprint: key.key_fingerprint,
            created_at: key.created_at,
            revoked_at: key.revoked_at,
        }
    }
}
