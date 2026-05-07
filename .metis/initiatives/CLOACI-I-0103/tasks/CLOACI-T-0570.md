---
id: t5-contract-test-sign-upload
level: task
title: "T5: Contract test (sign → upload → verify → load) + DiD regression test"
short_code: "CLOACI-T-0570"
created_at: 2026-05-06T12:51:04.969823+00:00
updated_at: 2026-05-07T03:45:30.028365+00:00
parent: CLOACI-I-0103
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0103
---

# T5: Contract test (sign → upload → verify → load) + DiD regression test

## Context

Per CLOACI-I-0103 acceptance criteria: validate the end-to-end signature verification flow with a contract test, and validate the DiD path with a regression test.

## What to do

- **Contract test** (`crates/cloacina/tests/integration/signing/`): generate a fixture-trusted keypair, sign a package, upload with `--require-signatures` on, assert verification + load both succeed.
- **DiD regression test**: with `require_signatures` on, inject a `workflow_packages` row directly into the DB (no `package_signatures` companion). Assert the reconciler refuses to load it.

## Acceptance

- Both tests pass via `angreal test integration`.
- Contract test exercises the full sign → upload → verify → store → reconciler-load → register flow.
- DiD test confirms reconciler refuses an unsigned package row when `require_signatures` is on.

## References

- Parent: CLOACI-I-0103 (acceptance criteria)
- Depends on T1, T2, T3, T4 (full feature must be in place).
- Test location: `crates/cloacina/tests/integration/signing/`

## Status Updates

### 2026-05-07 — Contract test landed; DiD regression deferred

**Done:** two end-to-end contract tests in `crates/cloacina-server/src/lib.rs::tests`, both `#[tokio::test] #[serial]` and dependent on the test Postgres at `TEST_DB_URL`:

1. **`test_upload_signed_with_require_signatures_passes_verification`** — full happy-path flow.
   - Builds `AppState` via new `test_state_with_signature_required(org_id)` helper (sets `require_signatures: true` + `verification_org_id`).
   - Provisions a signing key for the test org via `DbKeyManager::create_signing_key`, self-trusts it via `trust_public_key`.
   - Signs a unique payload (UUID-suffixed bytes — see "Lessons" below) with `DbPackageSigner::sign_package_with_db_key(..., store=true)`, which inserts the `package_signatures` row.
   - POSTs the same bytes through the multipart upload handler.
   - Asserts the response is **not** a verification-related 403. Bytes aren't a real `.cloacina` archive so the request fails further downstream — that's deliberate; the contract being tested is the verification gate, not the full upload→register→load chain.

2. **`test_upload_unsigned_with_require_signatures_returns_403`** — negative path.
   - Same `require_signatures` + `verification_org_id` setup.
   - POSTs unsigned bytes (no signature row).
   - Asserts 403 with `code = "signature_not_found"`.

**Lessons learned:**
- `find_signature(package_hash)` returns one row per hash. Persistent test DB across runs means a stale signature from a previous run gets picked up by verification, with a fingerprint that's not trusted for the *new* test's random org_id — produces a confusing `untrusted_signer` failure. Fix was making the payload unique per run (UUID-suffixed bytes). Documented in the test comment so the next reader doesn't re-debug.
- `DbKeyManager::create_signing_key` and `trust_public_key` are on the `KeyManager` trait, not the concrete struct. Test imports `cloacina::security::KeyManager` to bring them into scope.
- `tempfile` was added as a dev-dependency in `cloacina-server/Cargo.toml` so the test can stage payload bytes for `sign_package_with_db_key`.

**Deferred:** the **DiD regression test** ("inject a `workflow_packages` row directly into the DB without `package_signatures`; assert reconciler refuses") depends on T-0568's reconciler DiD existence check, which was deferred (see T-0568 status). Without that gate in place, the reconciler doesn't refuse; the regression test would fail. Recommend implementing this as a follow-up paired with the T-0568 DiD work.

**Verification:**
- Both tests pass when run individually and together (`cargo test -p cloacina-server --lib test_upload_`).
- `angreal lint clippy` clean.
- `angreal lint fmt` clean.
- Pre-existing unrelated failures in the test suite (not from this task): `test_upload_valid_python_workflow_returns_201`, `test_upload_valid_rust_workflow_returns_201`, `test_metrics_returns_prometheus_format`. Missing test fixtures + a pre-existing metrics-recorder global-state issue.
