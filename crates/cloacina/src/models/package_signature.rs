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

//! Domain models for package signatures.
//!
//! Package signatures provide cryptographic proof that a workflow package
//! was signed by a specific key. Signatures are Ed25519 signatures over
//! the SHA256 hash of the package binary.

use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use serde::{Deserialize, Serialize};

/// Domain model for a package signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSignature {
    pub id: UniversalUuid,
    /// SHA256 hex hash of the package binary
    pub package_hash: String,
    /// SHA256 hex fingerprint of the signing key
    pub key_fingerprint: String,
    /// Ed25519 signature (64 bytes)
    pub signature: Vec<u8>,
    pub signed_at: UniversalTimestamp,
}

/// Model for creating a new package signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPackageSignature {
    pub package_hash: String,
    pub key_fingerprint: String,
    pub signature: Vec<u8>,
}

impl NewPackageSignature {
    pub fn new(package_hash: String, key_fingerprint: String, signature: Vec<u8>) -> Self {
        Self {
            package_hash,
            key_fingerprint,
            signature,
        }
    }
}

/// Result of signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureVerification {
    /// Whether the signature is valid
    pub is_valid: bool,
    /// The fingerprint of the key that signed the package
    pub signer_fingerprint: String,
    /// When the package was signed
    pub signed_at: UniversalTimestamp,
    /// Name of the signing key (if known)
    pub signer_name: Option<String>,
}
