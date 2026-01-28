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

//! Domain models for trusted public keys.
//!
//! Trusted keys are public keys that an organization trusts for verifying
//! package signatures. They can be imported from external sources or
//! derived from the organization's own signing keys.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Domain model for a trusted public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedKey {
    pub id: UniversalUuid,
    pub org_id: UniversalUuid,
    /// SHA256 hex fingerprint of the public key
    pub key_fingerprint: String,
    /// Ed25519 public key (32 bytes)
    pub public_key: Vec<u8>,
    /// Optional human-readable name for the key
    pub key_name: Option<String>,
    pub trusted_at: UniversalTimestamp,
    /// None if active, Some if revoked
    pub revoked_at: Option<UniversalTimestamp>,
}

impl TrustedKey {
    /// Check if this key is currently trusted (not revoked)
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }

    /// Check if this key has been revoked
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}

/// Model for creating a new trusted key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTrustedKey {
    pub org_id: UniversalUuid,
    pub key_fingerprint: String,
    pub public_key: Vec<u8>,
    pub key_name: Option<String>,
}

impl NewTrustedKey {
    pub fn new(
        org_id: UniversalUuid,
        key_fingerprint: String,
        public_key: Vec<u8>,
        key_name: Option<String>,
    ) -> Self {
        Self {
            org_id,
            key_fingerprint,
            public_key,
            key_name,
        }
    }

    /// Create a trusted key from a signing key's public key.
    pub fn from_signing_key(
        org_id: UniversalUuid,
        key_fingerprint: String,
        public_key: Vec<u8>,
        key_name: String,
    ) -> Self {
        Self {
            org_id,
            key_fingerprint,
            public_key,
            key_name: Some(key_name),
        }
    }
}
