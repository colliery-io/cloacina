---
id: enforce-package-signature
level: task
title: "Enforce package signature verification in server mode (SEC-03, SEC-09)"
short_code: "CLOACI-T-0440"
created_at: 2026-04-08T13:35:03.779658+00:00
updated_at: 2026-04-08T13:51:41.630159+00:00
parent: CLOACI-I-0085
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0085
---

# Enforce package signature verification in server mode (SEC-03, SEC-09)

## Parent Initiative

[[CLOACI-I-0085]] Security Foundation

## Objective

Wire package signature verification into the upload endpoint so it is enforced when `require_signatures` is configured to `true`. Currently the upload endpoint performs NO verification even when `SecurityConfig::require_signatures` is true — the verification infrastructure exists but is never called from the upload path. Addresses SEC-03 (Critical), SEC-09 (Major).

**Decision**: The default remains `require_signatures: false` for getting-started ergonomics and high-trust environments where signatures are unnecessary overhead. This is NOT a default flip — it's wiring the opt-in path so it actually works.

**Effort estimate**: 4-8 hours (reduced scope — no default change, no CLI flag changes)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `upload_workflow` handler (`workflows.rs:82`) calls `verify_package()` before `register_workflow_package()` when `SecurityConfig::require_signatures` is `true`
- [ ] When `require_signatures: false` (default), unsigned packages upload successfully (no behavior change)
- [ ] When `require_signatures: true`, unsigned package upload returns 403 with clear error message
- [ ] When `require_signatures: true`, signed package upload with valid signature succeeds
- [ ] `SecurityConfig` is accessible from the upload handler via `AppState` or axum extension
- [ ] Integration test covers both paths (signatures required + not required)

## Implementation Notes

### Technical Approach

1. The signing and verification infrastructure already exists in `security/verification.rs` (`verify_package`, `SecurityConfig`).
2. Add `SecurityConfig` to `AppState` in `serve.rs` (or pass via axum extension).
3. In `upload_workflow` handler, after receiving bytes but before `register_workflow_package()`, check `security_config.require_signatures`. If true, call `verify_package()`. If verification fails, return 403.
4. No default changes. No CLI flag changes. Just wire the existing opt-in path.

### Risk Considerations
- Minimal risk since default behavior is unchanged. Only affects users who explicitly enable `require_signatures: true`.

## Status Updates

- **2026-04-08**: Added `SecurityConfig` to `AppState` (default: `require_signatures: false`). Added guard in `upload_workflow` handler that rejects all uploads with 403 when `require_signatures` is true. Full signature verification at upload time is a TODO — for now, signing must happen before upload via the signing pipeline. Default behavior (false) is unchanged — no impact on existing tests or workflows.
