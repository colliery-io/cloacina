---
id: security-hardening-auth-package
level: initiative
title: "Security Hardening — Auth, Package Validation, Python Sandboxing, API Protection"
short_code: "CLOACI-I-0039"
created_at: 2026-03-21T18:39:27.917565+00:00
updated_at: 2026-03-22T00:59:04.915613+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: security-hardening-auth-package
---

# Security Hardening — Auth, Package Validation, Python Sandboxing, API Protection Initiative

## Context

Comprehensive security audit of the post-v0.3.0 codebase (82k lines added) identified 2 critical, 6 high, and 5 medium severity security issues across the REST API server, authentication system, Python workflow execution, and package validation pipeline. These must be addressed before any production deployment.

## Goals & Non-Goals

**Goals:**
- Fix all critical and high severity security findings
- Establish security baselines for API protection, auth, and package validation
- Harden Python execution boundary against malicious packages

**Non-Goals:**
- Full penetration testing (separate engagement)
- WAF/network-level security (infrastructure concern)
- Cryptographic algorithm changes (Argon2/Ed25519 are appropriate)

## Findings (from audit)

### Critical

| # | Finding | Location |
|---|---------|----------|
| C-1 | **Auth bypass when no DB configured** — protected endpoints fully open, no auth middleware applied | `serve.rs:164-172` |
| C-2 | **Archive path traversal** — `tar.unpack()` without symlink/path sanitization on user-uploaded packages | `python_loader.rs:123`, `reconciler/extraction.rs`, `workflow_registry/package.rs` |

### High

| # | Finding | Location |
|---|---------|----------|
| H-1 | **No request body size limit** — OOM DoS via multipart upload | `serve.rs` (no `DefaultBodyLimit`) |
| H-2 | **No rate limiting** — brute-force API key + Argon2 CPU exhaustion | All endpoints |
| H-3 | **No CORS configuration** — undefined cross-origin behavior | `serve.rs` |
| H-4 | **Python sys.path injection** — malicious package shadows stdlib modules | `python/loader.rs:137-160` |
| H-5 | **No Python execution resource limits** — infinite loop, memory, network, subprocess | `python/loader.rs`, `python/executor.rs` |
| H-6 | **SQL injection in tenant DDL** — `format!()` for CREATE SCHEMA/USER/GRANT | `database/admin.rs:152-206` |

### Medium

| # | Finding | Location |
|---|---------|----------|
| M-1 | **Revoked keys valid for 60s** — auth cache TTL not invalidated | `auth/cache.rs` |
| M-2 | **Tenant isolation not enforced** — tenant-scoped keys access all data | `routes/executions.rs`, `routes/workflows.rs` |
| M-3 | **FFI smoke test can't catch abort()** — malicious library crashes validator | `validator/ffi_smoke.rs:128-144` |
| M-4 | **Decompression bomb** — no compressed-to-decompressed ratio limit | `python_loader.rs`, `reconciler/extraction.rs` |
| M-5 | **Server binds 0.0.0.0 by default** — combined with C-1, wide open on network | `config.rs:61` |

## Implementation Plan

### Phase 1: Critical fixes (must-have)
- Auth rejection middleware when no DB configured
- Archive path traversal sanitization (reject symlinks, `..` components, absolute paths)

### Phase 2: API hardening
- Request body size limit (`DefaultBodyLimit::max()`)
- Rate limiting middleware (tower-based)
- CORS configuration
- Default bind to 127.0.0.1

### Phase 3: Python sandbox
- sys.path validation (reject stdlib-shadowing files)
- Execution timeouts (`tokio::time::timeout` on GIL thread)
- Deny list for dangerous imports (subprocess, os.system)

### Phase 4: Tenant isolation + auth polish
- Enforce ABAC patterns in route handlers
- Invalidate auth cache on key revocation
- Audit tenant DDL validation for completeness
