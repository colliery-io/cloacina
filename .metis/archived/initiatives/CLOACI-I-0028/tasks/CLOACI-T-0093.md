---
id: c4-level-3-component-diagrams
level: task
title: "C4 Level 3 — Component Diagrams: Security Subsystem"
short_code: "CLOACI-T-0093"
created_at: 2026-03-13T14:29:58.503892+00:00
updated_at: 2026-03-13T16:32:30.894999+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# C4 Level 3 — Component Diagrams: Security Subsystem

**Phase:** 2 — C4 Architecture Documentation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Create the C4 Level 3 (Component) diagram and documentation for the Security Subsystem — Ed25519 signing, key management, package verification (online/offline), key encryption (AES-256-GCM), and audit logging.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Mermaid C4 Component diagram for the Security Subsystem
- [ ] Components: DbPackageSigner, DbKeyManager, PackageVerifier, KeyEncryption, AuditLogger
- [ ] Signing flow documented: key generation → package signing → signature storage
- [ ] Verification flow documented: online (DB) and offline (PEM file) paths
- [ ] Key lifecycle documented: generate → encrypt → store → rotate → export → trust
- [ ] AES-256-GCM encryption of private keys with master key documented
- [ ] All components verified against source in `crates/cloacina/src/security/`

## Implementation Notes

### Components to Document
- **DbPackageSigner** (`crates/cloacina/src/security/signing.rs`) — Ed25519 signing with DB-backed keys
- **DbKeyManager** (`crates/cloacina/src/security/key_manager.rs`) — key generation, rotation, export, trust management
- **PackageVerifier** (`crates/cloacina/src/security/verification.rs`) — online (DB) and offline (PEM) verification
- **KeyEncryption** (`crates/cloacina/src/security/encryption.rs`) — AES-256-GCM private key encryption
- **AuditLogger** (`crates/cloacina/src/security/audit.rs`) — security event logging

### Key Flows to Diagram
1. Package signing: Developer creates package → Operator signs with `cloacinactl key sign` → Signature stored in DB
2. Online verification: Package loaded → Signature checked against DB-stored public key
3. Offline verification: Package loaded → Signature checked against exported PEM public key
4. Key trust: Remote key imported → Trust level set → Used for verification

## Status Updates

### Completed 2026-03-13

**Created:** `docs/content/explanation/architecture/c4-security.md`

**Components:** Signing, KeyEncryption, DbKeyManager, DbPackageSigner, Verification, AuditLogger
- Signing + verification flows as sequence diagrams
- Key lifecycle (generate → encrypt → store → export → trust → revoke)
- Online vs offline verification documented
- Detached signature format documented
- Trust ACL inheritance documented
- All verified against `crates/cloacina/src/crypto/` and `src/security/`

**Build:** 98 pages, clean
