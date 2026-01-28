---
id: security-audit-logging-for-package
level: task
title: "Security audit logging for package and key operations"
short_code: "CLOACI-T-0064"
created_at: 2026-01-28T14:15:49.369913+00:00
updated_at: 2026-01-28T16:43:18.028229+00:00
parent: CLOACI-I-0008
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0008
---

# Security audit logging for package and key operations

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0008]]

## Objective

Implement comprehensive audit logging for all security-sensitive operations: package loads (success/failure), key operations (create, revoke, trust), and signature verification. Logs should be structured for SIEM integration.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] Structured logging for all package load attempts
- [x] Structured logging for all key management operations
- [x] Structured logging for signature verification (success/failure)
- [x] Log fields compatible with common SIEM systems (event_type, org_id, key_fingerprint, etc.)
- [x] Configurable log level/verbosity (uses tracing crate levels: INFO, WARN, ERROR)
- [x] No sensitive data (private keys) in logs - only fingerprints and metadata
- [x] Unit tests verifying log output (6 tests)

## Log Events

### Package Operations

```rust
/// Log when a package is loaded
#[derive(Debug, Serialize)]
pub struct PackageLoadEvent {
    pub event_type: &'static str,  // "package.load.success" or "package.load.failure"
    pub timestamp: DateTime<Utc>,
    pub org_id: Uuid,
    pub package_path: String,
    pub package_hash: String,
    pub signer_fingerprint: Option<String>,  // None if unsigned
    pub signature_verified: bool,
    pub error: Option<String>,
}

// Example log output (JSON):
// {
//   "event_type": "package.load.success",
//   "timestamp": "2026-01-28T12:00:00Z",
//   "org_id": "550e8400-e29b-41d4-a716-446655440000",
//   "package_path": "/packages/workflow.so",
//   "package_hash": "abc123...",
//   "signer_fingerprint": "def456...",
//   "signature_verified": true,
//   "error": null
// }
```

### Key Operations

```rust
#[derive(Debug, Serialize)]
pub struct KeyOperationEvent {
    pub event_type: &'static str,  // "key.created", "key.revoked", "key.trusted", etc.
    pub timestamp: DateTime<Utc>,
    pub org_id: Uuid,
    pub key_id: Option<Uuid>,
    pub key_fingerprint: Option<String>,
    pub key_name: Option<String>,
    pub actor: Option<String>,  // Who performed the operation (if available)
    pub error: Option<String>,
}

// Event types:
// - key.signing.created
// - key.signing.revoked
// - key.trusted.added
// - key.trusted.revoked
// - key.trust_acl.granted
// - key.trust_acl.revoked
// - key.exported
```

### Verification Events

```rust
#[derive(Debug, Serialize)]
pub struct VerificationEvent {
    pub event_type: &'static str,  // "verification.success", "verification.failure"
    pub timestamp: DateTime<Utc>,
    pub org_id: Uuid,
    pub package_hash: String,
    pub signer_fingerprint: String,
    pub failure_reason: Option<String>,  // "tampered", "untrusted_signer", "invalid_signature"
}
```

## Implementation

### Logging Macros

Use `tracing` with structured fields:

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(key_manager))]
pub async fn verify_and_load_package(...) -> Result<Library, PackageError> {
    // ... verification logic ...

    if config.require_signatures {
        match verify_signature(...) {
            Ok(()) => {
                info!(
                    event_type = "package.load.success",
                    org_id = %org_id,
                    package_path = %package_path.display(),
                    package_hash = %signature.package_hash,
                    signer_fingerprint = %signature.key_fingerprint,
                    signature_verified = true,
                    "Package loaded successfully"
                );
            }
            Err(e) => {
                warn!(
                    event_type = "package.load.failure",
                    org_id = %org_id,
                    package_path = %package_path.display(),
                    error = %e,
                    "Package verification failed"
                );
                return Err(e.into());
            }
        }
    } else {
        info!(
            event_type = "package.load.success",
            org_id = %org_id,
            package_path = %package_path.display(),
            signature_verified = false,
            "Package loaded (signatures not required)"
        );
    }
}
```

### Key Operation Logging

```rust
impl DbKeyManager {
    pub async fn create_signing_key(&self, org_id: Uuid, name: &str) -> Result<SigningKeyInfo, KeyError> {
        let result = self.do_create_signing_key(org_id, name).await;

        match &result {
            Ok(key_info) => {
                info!(
                    event_type = "key.signing.created",
                    org_id = %org_id,
                    key_id = %key_info.id,
                    key_fingerprint = %key_info.fingerprint,
                    key_name = %name,
                    "Signing key created"
                );
            }
            Err(e) => {
                error!(
                    event_type = "key.signing.create_failed",
                    org_id = %org_id,
                    key_name = %name,
                    error = %e,
                    "Failed to create signing key"
                );
            }
        }

        result
    }

    pub async fn revoke_signing_key(&self, key_id: Uuid) -> Result<(), KeyError> {
        let key_info = self.get_signing_key_info(key_id).await?;
        let result = self.do_revoke_signing_key(key_id).await;

        if result.is_ok() {
            warn!(
                event_type = "key.signing.revoked",
                org_id = %key_info.org_id,
                key_id = %key_id,
                key_fingerprint = %key_info.fingerprint,
                key_name = %key_info.key_name,
                "Signing key revoked"
            );
        }

        result
    }
}
```

## Log Levels

| Event | Level | Rationale |
|-------|-------|-----------|
| Package load success | INFO | Normal operation, audit trail |
| Package load failure | WARN | Security event, needs attention |
| Key created | INFO | Normal operation |
| Key revoked | WARN | Security event |
| Trust granted/revoked | WARN | Security policy change |
| Verification failure | WARN | Potential attack or misconfiguration |

## SIEM Integration

Logs are JSON-structured for easy parsing. Common fields:
- `event_type`: Dot-notation event identifier
- `timestamp`: ISO8601 timestamp
- `org_id`: Organization UUID for multi-tenant filtering
- `key_fingerprint`: For key correlation

Example Splunk query:
```
index=cloacina event_type="package.load.failure" | stats count by org_id, error
```

## File Locations

- Event types: `crates/cloacina/src/security/audit.rs`
- Integration: Add logging calls to existing functions in `key_manager.rs`, `verification.rs`, `signing.rs`

## Requires

- CLOACI-T-0061 (key management API)
- CLOACI-T-0063 (verification integration)

## Status Updates

### Implementation Complete

**Files Created:**
- `crates/cloacina/src/security/audit.rs` - Audit logging module with:
  - Event type constants (e.g., `key.signing.created`, `verification.failure`)
  - Logging functions for all security operations
  - Unit tests verifying log output

**Files Modified:**
- `crates/cloacina/src/security/mod.rs` - Export audit module
- `crates/cloacina/src/security/db_key_manager.rs` - Added audit logging for:
  - `create_signing_key` - logs key creation
  - `export_public_key` - logs key export
  - `trust_public_key` - logs trusted key addition
  - `revoke_signing_key` - logs key revocation
  - `revoke_trusted_key` - logs trusted key revocation
  - `grant_trust` - logs trust ACL grants
  - `revoke_trust` - logs trust ACL revocations
- `crates/cloacina/src/security/package_signer.rs` - Added audit logging for:
  - `sign_package_with_db_key` - logs signing success/failure
- `crates/cloacina/src/security/verification.rs` - Added audit logging for:
  - Verification success with structured fields
  - Verification failures with failure_reason (tampered, untrusted_signer, invalid_signature)

**Tests:** All 17 security tests pass including 6 new audit tests
