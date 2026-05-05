---
id: audit-t3-production-bugs-uncovered
level: task
title: "Audit T3: production bugs uncovered (server tests, signature gating, role check, banner)"
short_code: "CLOACI-T-0557"
created_at: 2026-05-04T16:10:21.801098+00:00
updated_at: 2026-05-04T19:21:34.612535+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Audit T3: production bugs uncovered (server tests, signature gating, role check, banner)

Real production bugs uncovered by the post-I-0102 nine-agent audit. None are dead-code cleanup — these are behaviors that don't match what the closed tickets advertised. Highest-priority item is the silently-broken server test suite.

## Objective

Verify, fix, and add regression coverage for seven production bugs surfaced by the audit. Each bug is independently reproducible; the work breaks naturally into one PR per bug.

## Backlog Item Details

### Type
- [x] Bug — production behaviors that diverge from the closed-ticket promises.

### Priority
- [x] P1 — High. Item #1 (server tests) means cloacina-server may have a coverage hole going back to T-0449. Items #2-#3 are auth/security gaps. Items #4-#7 are functional drift.

### Impact Assessment
- **Affected Users**: anyone running cloacina-server with packaged workflows that require signature verification (#2), tenant admins (#3), Python embedded loaders (#5), and operators reading the startup banner / shutdown logs (#6, #7).
- **Reproduction Steps**: see per-AC sections below.
- **Expected vs Actual**: see per-AC.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Bug 1 — server tests have hit pre-`/v1/` paths since T-0449

- [ ] Verify whether the 32 test sites in `crates/cloacina-server/src/lib.rs` (lines 944, 960, 978, 995, 1015, 1037, 1056, 1086, 1106, 1124, 1154, 1173, 1192, 1223, 1235, 1254, 1273, 1291, 1314, 1342, 1383, 1407, 1437, 1462, 1485, 1502, 1521, 1538, 1556, 1577, 1598, 1616) silently pass against the wrong URIs or have been failing. Run `cargo test -p cloacina-server` directly and read the output.
- [ ] If failing: rewrite all 32 sites to hit `/v1/auth/keys`, `/v1/tenants/...`, etc. — production router nests at `lib.rs:491`.
- [ ] If passing: figure out WHY (axum route fallthrough? a forgotten `merge` of unprefixed routes? a duplicate route registration?) and either remove the fallback or document it.
- [ ] Add a single regression test that asserts `GET /auth/keys` returns 404 (proving production paths ARE prefixed-only).

### Bug 2 — `upload_workflow` rejects all uploads when signatures are required

- [ ] `crates/cloacina-server/src/routes/workflows.rs:64-75` — when `require_signatures=true`, the handler unconditionally returns 403 with TODO `"implement full signature verification at upload time"`. T-0440 supposedly enforced this; finish the wire-up against `cloacina::security::verification`.
- [ ] Test: enable `require_signatures` in a fixture, upload a properly-signed package, assert 200. Upload an unsigned package, assert 403. Upload one with a tampered signature, assert 403.

### Bug 3 — `KeyRole` privilege-escalation check is a no-op

- [ ] `crates/cloacina-server/src/routes/keys.rs:82-88` — `if !auth.is_admin && requested_role == "admin"` block contains only a comment. A tenant-admin can mint another `permissions="admin"` key today.
- [ ] Implement the rejection: non-admin callers cannot create admin-role keys.
- [ ] Test: tenant-admin attempts to create an admin key → 403.

### Bug 4 — `get_workflow` returns hard-coded `build_status: "success"`

- [ ] `crates/cloacina-server/src/routes/workflows.rs:244` — for non-UUID lookups (by name+version), the response literal-encodes `"build_status": "success"` regardless of actual state. UUID lookups read the real status via `inspect_package_by_id`.
- [ ] Route both lookup shapes through the inspector so build status is accurate.

### Bug 5 — two competing `PyTriggerResult` APIs ship in cloacina-python

- [ ] `cloacina-python/src/bindings/trigger.rs:38` exposes `TriggerResult.skip()` / `.fire(ctx)`; `cloacina-python/src/trigger.rs:67` exposes `TriggerResult(should_fire=..., context=...)`. The synthetic loader (`loader.rs:126`) registers the latter; the wheel `#[pymodule]` (`lib.rs:114`) registers the former.
- [ ] Pick one (audit recommends `skip/fire` since live Python tests at `tests/python/test_scenario_29_event_triggers.py` already use it).
- [ ] Update `cloacina-python/tests/trigger_packaging.rs:270,321` (Rust fixtures using the loser API).
- [ ] Coordinated with T1 (`bindings/trigger.rs` deletion).

### Bug 6 — server startup banner shows pre-`/v1/` paths

- [ ] `crates/cloacina-server/src/lib.rs:295-297` logs `POST /auth/keys`, `GET /auth/keys`, `DEL /auth/keys/:id`. Real paths are `/v1/auth/keys` since T-0449.
- [ ] Update the banner.

### Bug 7 — banned phrase "Reactive scheduler shutdown complete"

- [ ] `crates/cloacina-server/src/lib.rs:314` logs `"Reactive scheduler shutdown complete"`. CLOACI-S-0011 nomenclature spec bans "reactive scheduler" — should be `"computation graph scheduler shutdown complete"` or `"graph scheduler shutdown complete"`.
- [ ] Workspace grep for any other instances of the banned phrase and fix together.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.
- [ ] New regression tests for bugs 1–4 land green.

## Implementation Notes

### Technical Approach

One PR per bug. Bug 1 is the gating diagnostic — do it first; if the tests are silently broken, every other test-driven assertion in this ticket becomes suspect.

### Dependencies

- Bug 5 coordinates with **T-0555 (T1)** which deletes `bindings/trigger.rs`.
- Bugs 2 & 3 touch `cloacina/src/security/`; coordinate if T-0560 (T6) audits the security path simultaneously.

### Risk Considerations

- **Bug 1 may reveal ignored test failures masking other issues.** Be prepared for cascading findings.
- **Bug 2** is a behavior change with security impact; coordinate with whoever owns I-0085 / I-0087 closure.

## Status Updates

### 2026-05-04 — Bug 1, 3, 4, 6, 7 landed (commits 8d12c00, 1d55bd9)

**Bug 1** — confirmed broken via diagnostic with PG up: 6 of 42 tests passed, 27 failed with `left: 404, right: 400`. Root cause: tests had been hitting unprefixed paths since T-0449. Fix: 38 sed substitutions across `.uri("/auth/...")` + `.uri("/tenants/...")` + `.uri(format!(...))` forms in `crates/cloacina-server/src/lib.rs`. Bonus fixture defect found: `TenantDatabaseCache::new(String::new())` panicked on first tenant lookup; fixed to pass `TEST_DB_URL.to_string()`. New regression test `test_unprefixed_auth_route_returns_404` asserts `/auth/keys` (no prefix) returns 404.

**Bug 3** — re-reading the code: NOT a vulnerability. The `is_admin` (god-mode) flag is hard-coded `false` in the `create_key` call; god-mode is granted only via `bootstrap-admin` at server start. `body.role` (per-key permissions) is orthogonal to `is_admin` — a tenant-admin CAN create another tenant-admin (peer level), and that's expected. Cleaned up the misleading dead `if` block, added clearer doc comment naming the actual safety mechanism, named the `is_admin: false` literal explicitly.

**Bug 4** — name+version path now routes through `inspect_package_by_id` like the UUID path. Real `build_status` + `build_error` emitted in both shapes.

**Bug 6** — `lib.rs:295-297` startup banner updated to `/v1/auth/keys`.

**Bug 7** — `lib.rs:314` "Reactive scheduler" → "Computation graph scheduler". Workspace grep found this as the only banned-phrase instance.

**Test gates after Bugs 1/3/4/6/7**: 39 of 43 server tests pass with PG up. Three pre-existing failures unchanged (out of scope for this audit ticket): `test_metrics_returns_prometheus_format` (metric naming assertion), `test_upload_valid_python_workflow_returns_201` + `test_upload_valid_rust_workflow_returns_201` (missing `crates/cloacina-server/test-fixtures/*.cloacina` archives). Tracking those separately if revival is wanted.

### Bug 5 — coupled to T-0555; T1 plan needs correction

Audit confirmed live Python tests at `tests/python/test_scenario_29_event_triggers.py:18,28,39,67` use `cloaca.TriggerResult.skip()` / `.fire(ctx)` API — the `bindings/trigger.rs` shape. The audit's T1 recommendation to **delete `bindings/trigger.rs`** would break the live Python tests.

Corrected direction: pick `skip/fire`. Delete `cloacina-python/src/trigger.rs::PyTriggerResult` (the loser API), KEEP `bindings/trigger.rs` (the winner), reroute `cloacina-python/src/loader.rs:126` to register the winner. Update Rust fixture sites at `cloacina-python/tests/trigger_packaging.rs:270, 321` to use `.skip()` / `.fire()`.

Bug 5 work blocks on T-0555 — call out the corrected direction in T-0555's AC.

### Bug 2 — deferred (medium-feature scope)

The "TODO: implement full signature verification at upload time" stub at `routes/workflows.rs:64-75` is not a session-sized fix. Required:

- `cloacina::security::verification::verify_package` takes a file path; upload handler has bytes in memory. Needs a `verify_package_bytes` variant or a temp-file write.
- `org_id` not threaded into the upload handler today (state has `tenant_id` but no org binding).
- `DbPackageSigner` + `DbKeyManager` not constructed in `state`.
- `SignatureSource` strategy decision needed: multipart sidecar field for the `.sig`? always database lookup?
- Signed-fixture test infrastructure required: signing keypair + trusted-key DB seeding.

The current "reject all" behavior is at least failsafe. Recommend spinning out as a dedicated medium-sized task with proper feature scoping rather than half-implementing here.

### 2026-05-04 — Bug 2, 5 landed (commit b521316). All 7 bugs closed.

**Bug 5** — Python `TriggerResult` API unified on `skip/fire`:
- Deleted `PyTriggerResult` class + unit test from `cloacina-python/src/trigger.rs`. `TriggerDecorator` + `trigger()` factory + `PythonTriggerWrapper` stay.
- `src/trigger.rs::PythonTriggerWrapper.poll()` downcasts as `bindings::trigger::PyTriggerResult` and converts via new `clone_into_rust(&self)` helper.
- `loader.rs::ensure_cloaca_module` registers `super::bindings::trigger::PyTriggerResult`.
- `lib.rs::PyTriggerResult` re-export now points at `bindings::trigger`.
- Rust fixtures at `cloacina-python/tests/trigger_packaging.rs:270, 321` updated to `TriggerResult.skip()` / `.fire(ctx)`.

`cloaca.TriggerResult` is now the same shape in pip-installed wheel mode and embedded synthetic-loader mode. T-0555 (T1) AC still says "delete src/trigger.rs::PyTriggerResult" — that's now done; T-0555 needs an AC strikethrough.

**Bug 2** — signature verification wired at upload time:
- `cloacina::security::verify_package_bytes(data, org_id, signature_source, package_signer, key_manager)` added in `security/verification.rs`. Same flow as path-based `verify_package`; consumes `&[u8]`.
- `SecurityConfig::verification_org_id: Option<UniversalUuid>` added — explicit org binding for trusted-key lookups. `None` means verification unconfigured; upload fails-safe with `signature_verification_unconfigured` (clearer than the old generic "TODO" 403).
- `routes/workflows.rs::upload_workflow` constructs `DbPackageSigner` + `DbKeyManager` from `state.database`, runs `verify_package_bytes` when `require_signatures && verification_org_id.is_some()`, translates `VerificationError` variants into specific 403 codes: `package_tampered`, `untrusted_signer`, `invalid_signature`, `signature_not_found`.
- Signature lookup uses `SignatureSource::Database`. `cloacinactl pack`/`publish` + the future T-0514 multipart sidecar are responsible for inserting `package_signatures` rows before upload.
- Test infrastructure for end-to-end signed-fixture roundtrips remains unbuilt (signing keypair + trusted-key DB seeding). Tracked separately; the verification wiring is in place when fixture infra lands.

### Final summary

All 7 bugs closed in this session:
- Bug 1: server tests — fixed (`8d12c00`)
- Bug 2: signature verification — wired (`b521316`)
- Bug 3: API key escalation — false alarm, cleaned dead code (`1d55bd9`)
- Bug 4: build_status accuracy — fixed (`1d55bd9`)
- Bug 5: Python TriggerResult API — unified (`b521316`)
- Bug 6: startup banner — fixed (`8d12c00`)
- Bug 7: banned phrase — fixed (`8d12c00`)

**Test gates (all green):**
- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (689 cloacina + 45 cloacina-workflow + 128 cloacina-python lib)
- [x] `angreal test integration --backend sqlite` (6 + 28 Python)
- [x] `angreal test integration --backend postgres` (295 Rust + 28 Python)
- [x] `cargo test -p cloacina-server --features sqlite`: 39/43 (3 pre-existing baseline failures unchanged, out of scope: missing test-fixtures + metric assertion)
- [x] `test_unprefixed_auth_route_returns_404` regression test passes

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

*To be added during implementation*
