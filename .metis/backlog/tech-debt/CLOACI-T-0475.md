---
id: wire-signature-verification-in
level: task
title: "Wire signature verification in upload handler and harden server defaults"
short_code: "CLOACI-T-0475"
created_at: 2026-04-11T13:42:55.253374+00:00
updated_at: 2026-04-13T00:03:35.033187+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Wire signature verification in upload handler and harden server defaults

## Objective

Make signature verification actually work when enabled (currently the upload handler has a TODO and blanket-rejects), expose the config knob via CLI, harden the default bind address, and clean up dead rate-limiting code.

## Review Finding References

SEC-002, SEC-005 (from architecture review `review/10-recommendations.md` REC-003)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: `require_signatures: true` blanket-rejects all uploads (TODO in handler) instead of verifying. No CLI/config path to enable it. Server defaults to `0.0.0.0:8080` (all interfaces, no TLS). Vestigial `tower_governor` dep and dead `TOO_MANY_REQUESTS` error variant from removed rate limiting.
- **Benefits of Fixing**: Operators who need signature verification can actually use it. Server doesn't accidentally expose on all interfaces. Dead code removed.
- **Risk Assessment**: Low risk — default remains `require_signatures: false` (intentional for high-trust environments). Bind address change only affects new deployments.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `--require-signatures` CLI flag and `security.require_signatures` config key exposed on `cloacinactl serve`
- [ ] When `require_signatures: true`, `upload_workflow` calls `verify_package_signature()` and accepts valid packages (not blanket-reject)
- [ ] When `require_signatures: false` (default), uploads accepted without verification (unchanged behavior)
- [ ] Default bind address is `127.0.0.1:8080`
- [ ] Starting on non-loopback address without TLS requires `--allow-plaintext` flag
- [ ] `tower_governor` removed from `Cargo.toml`
- [ ] `TOO_MANY_REQUESTS` error variant removed from `error.rs`

## Implementation Notes

### Technical Approach

**Signature verification (SEC-002):**
- The verification pipeline in `security/verification.rs` already validates signatures against trusted keys
- In `upload_workflow` handler, replace the TODO block with a call to `verify_package_signature()` when `require_signatures` is true
- `SecurityConfig` already has the `require_signatures` field — just expose it via Clap arg and TOML config
- Default stays `false`

**Bind address hardening (SEC-005):**
- Change `default_value` in the Clap `#[arg]` attribute from `"0.0.0.0:8080"` to `"127.0.0.1:8080"`
- Add startup check: if bind address is non-loopback and no TLS configured and `--allow-plaintext` not set, refuse to start with clear error

**Dead code cleanup:**
- Remove `tower_governor` from `crates/cloacinactl/Cargo.toml`
- Remove `TOO_MANY_REQUESTS` variant from `error.rs`

### Key Files
- `crates/cloacinactl/src/server/workflows.rs` — `upload_workflow` handler with TODO
- `crates/cloacinactl/src/commands/serve.rs` — CLI args, SecurityConfig construction
- `crates/cloacina/src/security/verification.rs` — existing verification pipeline
- `crates/cloacinactl/src/server/error.rs` — dead `TOO_MANY_REQUESTS` variant
- `crates/cloacinactl/Cargo.toml` — vestigial `tower_governor` dep
- `crates/cloacinactl/src/main.rs` — bind address default

### Dependencies
None.

## Status Updates

*To be added during implementation*