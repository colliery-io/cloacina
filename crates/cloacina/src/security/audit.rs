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

//! Security audit logging for SIEM integration.
//!
//! This module provides structured audit logging for all security-sensitive operations:
//! - Package loads (success/failure)
//! - Key operations (create, revoke, trust)
//! - Signature verification
//!
//! All events use structured fields compatible with common SIEM systems.
//! Events are logged using the `tracing` crate at appropriate levels.

use crate::database::universal_types::UniversalUuid;

/// Event types for package operations.
pub mod events {
    /// Package load success event type.
    pub const PACKAGE_LOAD_SUCCESS: &str = "package.load.success";
    /// Package load failure event type.
    pub const PACKAGE_LOAD_FAILURE: &str = "package.load.failure";
    /// Package signed event type.
    pub const PACKAGE_SIGNED: &str = "package.signed";
    /// Package sign failure event type.
    pub const PACKAGE_SIGN_FAILURE: &str = "package.sign.failure";

    /// Signing key created event type.
    pub const KEY_SIGNING_CREATED: &str = "key.signing.created";
    /// Signing key create failure event type.
    pub const KEY_SIGNING_CREATE_FAILED: &str = "key.signing.create_failed";
    /// Signing key revoked event type.
    pub const KEY_SIGNING_REVOKED: &str = "key.signing.revoked";
    /// Signing key exported event type.
    pub const KEY_EXPORTED: &str = "key.exported";

    /// Trusted key added event type.
    pub const KEY_TRUSTED_ADDED: &str = "key.trusted.added";
    /// Trusted key revoked event type.
    pub const KEY_TRUSTED_REVOKED: &str = "key.trusted.revoked";

    /// Trust ACL granted event type.
    pub const KEY_TRUST_ACL_GRANTED: &str = "key.trust_acl.granted";
    /// Trust ACL revoked event type.
    pub const KEY_TRUST_ACL_REVOKED: &str = "key.trust_acl.revoked";

    /// Verification success event type.
    pub const VERIFICATION_SUCCESS: &str = "verification.success";
    /// Verification failure event type.
    pub const VERIFICATION_FAILURE: &str = "verification.failure";
}

/// Log a signing key creation event.
pub fn log_signing_key_created(
    org_id: UniversalUuid,
    key_id: UniversalUuid,
    key_fingerprint: &str,
    key_name: &str,
) {
    tracing::info!(
        event_type = events::KEY_SIGNING_CREATED,
        org_id = %org_id,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        key_name = %key_name,
        "Signing key created"
    );
}

/// Log a signing key creation failure.
pub fn log_signing_key_create_failed(org_id: UniversalUuid, key_name: &str, error: &str) {
    tracing::error!(
        event_type = events::KEY_SIGNING_CREATE_FAILED,
        org_id = %org_id,
        key_name = %key_name,
        error = %error,
        "Failed to create signing key"
    );
}

/// Log a signing key revocation event.
pub fn log_signing_key_revoked(
    org_id: UniversalUuid,
    key_id: UniversalUuid,
    key_fingerprint: &str,
    key_name: Option<&str>,
) {
    tracing::warn!(
        event_type = events::KEY_SIGNING_REVOKED,
        org_id = %org_id,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        key_name = key_name.unwrap_or("<unknown>"),
        "Signing key revoked"
    );
}

/// Log a public key export event.
pub fn log_key_exported(key_id: UniversalUuid, key_fingerprint: &str) {
    tracing::info!(
        event_type = events::KEY_EXPORTED,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        "Public key exported"
    );
}

/// Log a trusted key addition event.
pub fn log_trusted_key_added(
    org_id: UniversalUuid,
    key_id: UniversalUuid,
    key_fingerprint: &str,
    key_name: Option<&str>,
) {
    tracing::warn!(
        event_type = events::KEY_TRUSTED_ADDED,
        org_id = %org_id,
        key_id = %key_id,
        key_fingerprint = %key_fingerprint,
        key_name = key_name.unwrap_or("<unnamed>"),
        "Trusted key added"
    );
}

/// Log a trusted key revocation event.
pub fn log_trusted_key_revoked(key_id: UniversalUuid) {
    tracing::warn!(
        event_type = events::KEY_TRUSTED_REVOKED,
        key_id = %key_id,
        "Trusted key revoked"
    );
}

/// Log a trust ACL grant event.
pub fn log_trust_acl_granted(parent_org: UniversalUuid, child_org: UniversalUuid) {
    tracing::warn!(
        event_type = events::KEY_TRUST_ACL_GRANTED,
        parent_org_id = %parent_org,
        child_org_id = %child_org,
        "Trust ACL granted"
    );
}

/// Log a trust ACL revocation event.
pub fn log_trust_acl_revoked(parent_org: UniversalUuid, child_org: UniversalUuid) {
    tracing::warn!(
        event_type = events::KEY_TRUST_ACL_REVOKED,
        parent_org_id = %parent_org,
        child_org_id = %child_org,
        "Trust ACL revoked"
    );
}

/// Log a package signing event.
pub fn log_package_signed(package_path: &str, package_hash: &str, key_fingerprint: &str) {
    tracing::info!(
        event_type = events::PACKAGE_SIGNED,
        package_path = %package_path,
        package_hash = %package_hash,
        key_fingerprint = %key_fingerprint,
        "Package signed"
    );
}

/// Log a package signing failure.
pub fn log_package_sign_failed(package_path: &str, error: &str) {
    tracing::error!(
        event_type = events::PACKAGE_SIGN_FAILURE,
        package_path = %package_path,
        error = %error,
        "Package signing failed"
    );
}

/// Log a package load success event.
pub fn log_package_load_success(
    org_id: UniversalUuid,
    package_path: &str,
    package_hash: &str,
    signer_fingerprint: Option<&str>,
    signature_verified: bool,
) {
    tracing::info!(
        event_type = events::PACKAGE_LOAD_SUCCESS,
        org_id = %org_id,
        package_path = %package_path,
        package_hash = %package_hash,
        signer_fingerprint = signer_fingerprint.unwrap_or("<none>"),
        signature_verified = signature_verified,
        "Package loaded successfully"
    );
}

/// Log a package load failure event.
pub fn log_package_load_failure(
    org_id: UniversalUuid,
    package_path: &str,
    error: &str,
    failure_reason: &str,
) {
    tracing::warn!(
        event_type = events::PACKAGE_LOAD_FAILURE,
        org_id = %org_id,
        package_path = %package_path,
        error = %error,
        failure_reason = %failure_reason,
        "Package load failed"
    );
}

/// Log a verification success event.
pub fn log_verification_success(
    org_id: UniversalUuid,
    package_hash: &str,
    signer_fingerprint: &str,
    signer_name: Option<&str>,
) {
    tracing::info!(
        event_type = events::VERIFICATION_SUCCESS,
        org_id = %org_id,
        package_hash = %package_hash,
        signer_fingerprint = %signer_fingerprint,
        signer_name = signer_name.unwrap_or("<unknown>"),
        "Package signature verified successfully"
    );
}

/// Log a verification failure event.
pub fn log_verification_failure(
    org_id: UniversalUuid,
    package_hash: &str,
    failure_reason: &str,
    signer_fingerprint: Option<&str>,
) {
    tracing::warn!(
        event_type = events::VERIFICATION_FAILURE,
        org_id = %org_id,
        package_hash = %package_hash,
        failure_reason = %failure_reason,
        signer_fingerprint = signer_fingerprint.unwrap_or("<unknown>"),
        "Package signature verification failed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone)]
    struct StringWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for StringWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for StringWriter {
        type Writer = StringWriter;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    // Helper to capture log output
    fn with_captured_logs<F>(f: F) -> String
    where
        F: FnOnce(),
    {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = StringWriter(buffer.clone());

        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, f);

        let result = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        result
    }

    #[test]
    fn test_log_signing_key_created() {
        let output = with_captured_logs(|| {
            log_signing_key_created(
                UniversalUuid::new_v4(),
                UniversalUuid::new_v4(),
                "abc123",
                "test-key",
            );
        });

        assert!(output.contains(events::KEY_SIGNING_CREATED));
        assert!(output.contains("test-key"));
        assert!(output.contains("abc123"));
    }

    #[test]
    fn test_log_verification_failure() {
        let output = with_captured_logs(|| {
            log_verification_failure(
                UniversalUuid::new_v4(),
                "package_hash_123",
                "untrusted_signer",
                Some("fingerprint_abc"),
            );
        });

        assert!(output.contains(events::VERIFICATION_FAILURE));
        assert!(output.contains("untrusted_signer"));
        assert!(output.contains("fingerprint_abc"));
    }

    #[test]
    fn test_log_package_load_success() {
        let output = with_captured_logs(|| {
            log_package_load_success(
                UniversalUuid::new_v4(),
                "/path/to/package.so",
                "hash_123",
                Some("fingerprint_456"),
                true,
            );
        });

        assert!(output.contains(events::PACKAGE_LOAD_SUCCESS));
        assert!(output.contains("/path/to/package.so"));
        assert!(output.contains("signature_verified"));
    }

    #[test]
    fn test_log_trust_acl_granted() {
        let output = with_captured_logs(|| {
            log_trust_acl_granted(UniversalUuid::new_v4(), UniversalUuid::new_v4());
        });

        assert!(output.contains(events::KEY_TRUST_ACL_GRANTED));
        assert!(output.contains("parent_org_id"));
        assert!(output.contains("child_org_id"));
    }

    #[test]
    fn test_event_type_constants() {
        // Verify event type naming convention follows dot notation
        assert!(events::PACKAGE_LOAD_SUCCESS.starts_with("package."));
        assert!(events::KEY_SIGNING_CREATED.starts_with("key."));
        assert!(events::VERIFICATION_SUCCESS.starts_with("verification."));
    }
}
