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

mod key_encryption;
mod signing;

pub use key_encryption::{decrypt_private_key, encrypt_private_key, KeyEncryptionError};
pub use signing::{
    compute_key_fingerprint, generate_signing_keypair, sign_package, verify_signature,
    GeneratedKeypair, SigningError,
};
