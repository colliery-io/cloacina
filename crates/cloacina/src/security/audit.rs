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

    /// Compiler build started event type. Emitted once per build claim by
    /// `cloacina-compiler` after the source archive is unpacked, just
    /// before the cargo subprocess fires. CLOACI-T-0576.
    pub const COMPILER_BUILD_STARTED: &str = "compiler.build.started";
    /// Compiler build finished event type. Emitted once per build claim
    /// on every outcome path (success, clean failure, timeout-kill,
    /// internal error). CLOACI-T-0576.
    pub const COMPILER_BUILD_FINISHED: &str = "compiler.build.finished";
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

/// Log a compiler build start event. Emitted by `cloacina-compiler` once
/// per build claim, after the source archive is unpacked and content
/// hashes are computed, just before the cargo subprocess fires.
/// CLOACI-T-0576.
pub fn log_compiler_build_started(
    build_claim_id: UniversalUuid,
    package_name: &str,
    package_version: &str,
    cargo_toml_hash: &str,
    cargo_lock_hash: Option<&str>,
    compiler_instance_id: UniversalUuid,
) {
    tracing::info!(
        event_type = events::COMPILER_BUILD_STARTED,
        build_claim_id = %build_claim_id,
        package_name = %package_name,
        package_version = %package_version,
        cargo_toml_hash = %cargo_toml_hash,
        cargo_lock_hash = cargo_lock_hash.unwrap_or("<none>"),
        compiler_instance_id = %compiler_instance_id,
        "Compiler build started"
    );
}

/// Log a compiler build finished event. Emitted exactly once per build
/// claim on every outcome path (`success`, `failed`, `timeout_killed`,
/// `internal_error`). CLOACI-T-0576.
///
/// `exit_status` is `Some(code)` only when cargo exited via `exit()`;
/// `exit_signal` is `Some(name)` only when cargo was signal-terminated.
/// On `timeout_killed`, the compiler SIGKILL'd cargo itself —
/// `exit_signal = Some("SIGKILL")`. `failure_reason` is `<none>` on
/// `success`.
#[allow(clippy::too_many_arguments)]
pub fn log_compiler_build_finished(
    build_claim_id: UniversalUuid,
    package_name: &str,
    package_version: &str,
    cargo_toml_hash: &str,
    cargo_lock_hash: Option<&str>,
    compiler_instance_id: UniversalUuid,
    outcome: &str,
    exit_status: Option<i32>,
    exit_signal: Option<&str>,
    wall_clock_ms: u64,
    failure_reason: Option<&str>,
) {
    tracing::info!(
        event_type = events::COMPILER_BUILD_FINISHED,
        build_claim_id = %build_claim_id,
        package_name = %package_name,
        package_version = %package_version,
        cargo_toml_hash = %cargo_toml_hash,
        cargo_lock_hash = cargo_lock_hash.unwrap_or("<none>"),
        compiler_instance_id = %compiler_instance_id,
        outcome = %outcome,
        exit_status = exit_status
            .map(|c| c.to_string())
            .unwrap_or_else(|| "<none>".to_string()),
        exit_signal = exit_signal.unwrap_or("<none>"),
        wall_clock_ms = wall_clock_ms,
        failure_reason = failure_reason.unwrap_or("<none>"),
        "Compiler build finished"
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

    #[test]
    fn test_log_signing_key_create_failed() {
        let output = with_captured_logs(|| {
            log_signing_key_create_failed(UniversalUuid::new_v4(), "bad-key", "encryption error");
        });
        assert!(output.contains(events::KEY_SIGNING_CREATE_FAILED));
        assert!(output.contains("bad-key"));
        assert!(output.contains("encryption error"));
    }

    #[test]
    fn test_log_signing_key_revoked() {
        let output = with_captured_logs(|| {
            log_signing_key_revoked(
                UniversalUuid::new_v4(),
                UniversalUuid::new_v4(),
                "fp_abc",
                Some("my-key"),
            );
        });
        assert!(output.contains(events::KEY_SIGNING_REVOKED));
        assert!(output.contains("fp_abc"));
        assert!(output.contains("my-key"));
    }

    #[test]
    fn test_log_signing_key_revoked_no_name() {
        let output = with_captured_logs(|| {
            log_signing_key_revoked(
                UniversalUuid::new_v4(),
                UniversalUuid::new_v4(),
                "fp_xyz",
                None,
            );
        });
        assert!(output.contains(events::KEY_SIGNING_REVOKED));
        assert!(output.contains("<unknown>"));
    }

    #[test]
    fn test_log_key_exported() {
        let output = with_captured_logs(|| {
            log_key_exported(UniversalUuid::new_v4(), "fp_export_123");
        });
        assert!(output.contains(events::KEY_EXPORTED));
        assert!(output.contains("fp_export_123"));
    }

    #[test]
    fn test_log_trusted_key_added() {
        let output = with_captured_logs(|| {
            log_trusted_key_added(
                UniversalUuid::new_v4(),
                UniversalUuid::new_v4(),
                "trusted_fp",
                Some("vendor-key"),
            );
        });
        assert!(output.contains(events::KEY_TRUSTED_ADDED));
        assert!(output.contains("trusted_fp"));
        assert!(output.contains("vendor-key"));
    }

    #[test]
    fn test_log_trusted_key_added_no_name() {
        let output = with_captured_logs(|| {
            log_trusted_key_added(
                UniversalUuid::new_v4(),
                UniversalUuid::new_v4(),
                "trusted_fp2",
                None,
            );
        });
        assert!(output.contains(events::KEY_TRUSTED_ADDED));
        assert!(output.contains("<unnamed>"));
    }

    #[test]
    fn test_log_trusted_key_revoked() {
        let output = with_captured_logs(|| {
            log_trusted_key_revoked(UniversalUuid::new_v4());
        });
        assert!(output.contains(events::KEY_TRUSTED_REVOKED));
    }

    #[test]
    fn test_log_trust_acl_revoked() {
        let output = with_captured_logs(|| {
            log_trust_acl_revoked(UniversalUuid::new_v4(), UniversalUuid::new_v4());
        });
        assert!(output.contains(events::KEY_TRUST_ACL_REVOKED));
        assert!(output.contains("parent_org_id"));
        assert!(output.contains("child_org_id"));
    }

    #[test]
    fn test_log_package_signed() {
        let output = with_captured_logs(|| {
            log_package_signed("/tmp/pkg.tar.gz", "hash_abc", "fp_signer");
        });
        assert!(output.contains(events::PACKAGE_SIGNED));
        assert!(output.contains("/tmp/pkg.tar.gz"));
        assert!(output.contains("hash_abc"));
        assert!(output.contains("fp_signer"));
    }

    #[test]
    fn test_log_package_sign_failed() {
        let output = with_captured_logs(|| {
            log_package_sign_failed("/tmp/bad.tar.gz", "key not found");
        });
        assert!(output.contains(events::PACKAGE_SIGN_FAILURE));
        assert!(output.contains("/tmp/bad.tar.gz"));
        assert!(output.contains("key not found"));
    }

    #[test]
    fn test_log_package_load_failure() {
        let output = with_captured_logs(|| {
            log_package_load_failure(
                UniversalUuid::new_v4(),
                "/path/to/bad.so",
                "hash mismatch",
                "tampered",
            );
        });
        assert!(output.contains(events::PACKAGE_LOAD_FAILURE));
        assert!(output.contains("/path/to/bad.so"));
        assert!(output.contains("hash mismatch"));
        assert!(output.contains("tampered"));
    }

    #[test]
    fn test_log_verification_success() {
        let output = with_captured_logs(|| {
            log_verification_success(
                UniversalUuid::new_v4(),
                "pkg_hash_ok",
                "signer_fp_ok",
                Some("trusted-vendor"),
            );
        });
        assert!(output.contains(events::VERIFICATION_SUCCESS));
        assert!(output.contains("pkg_hash_ok"));
        assert!(output.contains("signer_fp_ok"));
        assert!(output.contains("trusted-vendor"));
    }

    #[test]
    fn test_log_verification_success_no_name() {
        let output = with_captured_logs(|| {
            log_verification_success(UniversalUuid::new_v4(), "pkg_hash", "signer_fp", None);
        });
        assert!(output.contains(events::VERIFICATION_SUCCESS));
        assert!(output.contains("<unknown>"));
    }

    #[test]
    fn test_log_verification_failure_no_fingerprint() {
        let output = with_captured_logs(|| {
            log_verification_failure(UniversalUuid::new_v4(), "pkg_hash", "no matching key", None);
        });
        assert!(output.contains(events::VERIFICATION_FAILURE));
        assert!(output.contains("<unknown>"));
    }

    // -----------------------------------------------------------------------
    // CLOACI-T-0576: compiler build audit events
    // -----------------------------------------------------------------------

    #[test]
    fn test_log_compiler_build_started_full_payload() {
        let output = with_captured_logs(|| {
            log_compiler_build_started(
                UniversalUuid::new_v4(),
                "my-package",
                "1.2.3",
                "deadbeefcafef00d",
                Some("0123456789abcdef"),
                UniversalUuid::new_v4(),
            );
        });
        assert!(output.contains(events::COMPILER_BUILD_STARTED));
        assert!(output.contains("my-package"));
        assert!(output.contains("1.2.3"));
        assert!(output.contains("deadbeefcafef00d"));
        assert!(output.contains("0123456789abcdef"));
        assert!(output.contains("build_claim_id"));
        assert!(output.contains("compiler_instance_id"));
    }

    #[test]
    fn test_log_compiler_build_started_no_lockfile_renders_none() {
        let output = with_captured_logs(|| {
            log_compiler_build_started(
                UniversalUuid::new_v4(),
                "pkg-without-lock",
                "0.0.1",
                "abc",
                None,
                UniversalUuid::new_v4(),
            );
        });
        assert!(output.contains(events::COMPILER_BUILD_STARTED));
        // The lockfile field should render `<none>` placeholder.
        assert!(output.contains("cargo_lock_hash"));
        assert!(output.contains("<none>"));
    }

    #[test]
    fn test_log_compiler_build_finished_success() {
        let output = with_captured_logs(|| {
            log_compiler_build_finished(
                UniversalUuid::new_v4(),
                "happy-pkg",
                "1.0.0",
                "cafe",
                Some("babe"),
                UniversalUuid::new_v4(),
                "success",
                Some(0),
                None,
                4250,
                None,
            );
        });
        assert!(output.contains(events::COMPILER_BUILD_FINISHED));
        assert!(output.contains("outcome=\"success\"") || output.contains("outcome=success"));
        assert!(output.contains("wall_clock_ms"));
        assert!(output.contains("4250"));
        // failure_reason and exit_signal should render <none> on success.
        let none_occurrences = output.matches("<none>").count();
        assert!(
            none_occurrences >= 2,
            "expected at least two <none> placeholders (exit_signal + failure_reason), \
             got: {output}"
        );
    }

    #[test]
    fn test_log_compiler_build_finished_timeout_killed() {
        let output = with_captured_logs(|| {
            log_compiler_build_finished(
                UniversalUuid::new_v4(),
                "slow-pkg",
                "0.1.0",
                "feed",
                None,
                UniversalUuid::new_v4(),
                "timeout_killed",
                None,
                Some("SIGKILL"),
                600_000,
                Some("cargo build exceeded build_timeout"),
            );
        });
        assert!(output.contains(events::COMPILER_BUILD_FINISHED));
        assert!(
            output.contains("outcome=\"timeout_killed\"")
                || output.contains("outcome=timeout_killed")
        );
        assert!(output.contains("SIGKILL"));
        assert!(output.contains("600000"));
        assert!(output.contains("exceeded build_timeout"));
    }

    #[test]
    fn test_log_compiler_build_finished_clean_failure() {
        let output = with_captured_logs(|| {
            log_compiler_build_finished(
                UniversalUuid::new_v4(),
                "broken-pkg",
                "0.0.1",
                "f00d",
                Some("c001"),
                UniversalUuid::new_v4(),
                "failed",
                Some(101),
                None,
                2500,
                Some("dependencies not available offline: unobtanium"),
            );
        });
        assert!(output.contains(events::COMPILER_BUILD_FINISHED));
        assert!(output.contains("outcome=\"failed\"") || output.contains("outcome=failed"));
        assert!(output.contains("101"));
        assert!(output.contains("unobtanium"));
    }
}
