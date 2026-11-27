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

//! Security module for package signing and key management.
//!
//! This module provides:
//! - [`KeyManager`] trait for managing signing keys and trust relationships
//! - [`DbKeyManager`] database-backed implementation
//! - Key generation, encryption, and PEM export/import
//! - Security audit logging for SIEM integration

pub mod api_keys;
pub mod audit;
mod db_key_manager;
pub mod fleet_secret;
mod key_manager;
pub mod ownership;
mod package_signer;
pub mod secret_grant;
mod secret_resolver;
mod secret_store;
mod tenant_org;
mod verification;

pub use db_key_manager::DbKeyManager;
pub use fleet_secret::{
    AgentKeyPool, FleetSecretError, InMemorySecretResolver, ServerKeyPool, decode_pool_public_key,
    resolve_and_wrap_secrets, secret_aad, secret_ref_names, wrap_field_map,
};
pub use key_manager::{KeyError, KeyManager, PublicKeyExport, SigningKeyInfo, TrustedKeyInfo};
pub use package_signer::{
    DbPackageSigner, DetachedSignature, PackageSignError, PackageSignatureInfo, PackageSigner,
};
pub use secret_grant::{
    DeclaredSecretResolver, SecretGrant, SecretGrantMode, SecretRefusal, SecretRefusalRecorder,
    declared_secret_grant, record_secret_refusal, register_secret_refusal_recorder,
    secret_bindings,
};
pub use secret_resolver::{
    KEK_ENV_VAR, SecretAllow, SecretResolverConfigError, SecretStoreResolver,
};
pub use secret_store::{SecretError, SecretMetadata, SecretStore};
pub use tenant_org::tenant_org_id;
pub use verification::{
    SecurityConfig, SignatureSource, VerificationError, VerificationResult, verify_package,
    verify_package_bytes, verify_package_bytes_for_orgs, verify_package_offline,
};
