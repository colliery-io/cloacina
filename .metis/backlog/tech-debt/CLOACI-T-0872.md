---
id: independent-provider-release-path
level: task
title: "Independent provider release path — publish/tag first-party providers on their own cadence (not core's v* tag)"
short_code: "CLOACI-T-0872"
created_at: 2026-07-08T11:43:21.080493+00:00
updated_at: 2026-07-31T05:39:49.412122+00:00
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

# Independent provider release path — publish/tag first-party providers on their own cadence (not core's v* tag)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Surfaced during I-0134 (2026-07-08). Providers are STRUCTURALLY independent (standalone crates, own semver, [[CLOACI-A-0010]]) but there is NO operational release path for them: `.github/workflows/unified_release.yml` triggers on CORE `v*` tags only, and the first-party providers aren't published anywhere. So "independent release schedule" is not yet real — only the structure exists.

Design + build a release path for first-party providers that lets them publish/tag on their OWN cadence, decoupled from core's `v*` tag (e.g. per-provider tags like `provider-fs-vX.Y.Z` → publish that crate to crates.io). Keep it separate from the core release pipeline (I-0134 non-goal: don't entangle core's release with providers). Blocked-ish on [[CLOACI-T-0871]] (which providers are real + where they live). Do NOT re-litigate A-0010.

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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] `provider_release.yml` wave workflow exists (tag `providers-v<YYYY.MM[.n]>` or dispatch) implementing classify → guard → certify → publish → record → wave notes, with per-provider-atomic failure semantics.
- [ ] Certification compiles + E2E-runs each provider's consumer fixture against **crates.io** core (no local patching) for BOTH candidates and unchanged providers.
- [ ] Compat table (in-repo, machine-readable: provider × version × certified core × wave) is regenerated and committed by the wave.
- [ ] PR guard fails any change to `providers/<name>/src` that doesn't bump the version + add a changelog entry.
- [ ] `angreal providers wave` front door exists (prep checks + tag cut); post-core-release prep PR flow (pin bumps + compat patch releases) documented.
- [ ] Inaugural wave publishes `cloacina-provider-kafka` (publish=false flipped) and the compat table records it certified against core 0.10.

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

### Design — the provider WAVE model (blessed by user 2026-07-28; supersedes the per-provider-tag sketch in the Objective)

**What a provider release IS:** a verified claim with four parts — (1) an immutable source crate at version X on crates.io; (2) a **machine-earned compatibility claim** ("tested against released core Y" — the release gate compiles + E2E-runs the provider's consumer fixture against crates.io core, never the working tree); (3) the provider's own semver where the API surface is the **config schema** (renaming a config key is breaking even if no Rust signature moved — changelogs written in config-schema terms); (4) a row in the tested-set **compat table**. NOT part of a release: binaries or signing — the compiler builds native cdylibs from source at workflow-compile time (validated 2026-07-28), so providers are source releases.

**Cadence model (Airflow-style waves):** provider versions stay independent (A-0010) but release ceremony is BATCHED — one wave trigger covers every provider. (Airflow's actual model for ~90 providers: independent versions + periodic release waves + published constraint files recording the tested combination. We adopt all three at small scale.)

**Standing state between waves:**
- Providers live in `providers/` (post-T-0871), each: standalone crate, own `[workspace]`, own version, per-provider CHANGELOG.md, a consumer fixture, core deps PINNED to an exact released minor (`= "0.10"`); ship-form version deps with harness local-patching for dev (same mechanism as ship-form examples).
- PR guard: any PR touching `providers/<name>/src` with manifest version == last PUBLISHED version FAILS (must bump + changelog). Main is self-describing by wave time.

**The wave (`provider_release.yml`, triggered by `providers-v<YYYY.MM[.n]>` tag or dispatch; `angreal providers wave` as the front door):**
1. **Classify** each provider: manifest version > crates.io published → RELEASE CANDIDATE; equal → RE-CERTIFY ONLY.
2. **Guards (candidates only):** changelog entry for the new version exists; core deps are published versions (no path deps); version increased.
3. **Certify (BOTH classes, identical step — the heart):** clean env, NO local patching — compile the consumer fixture resolving core from crates.io, run it E2E (kafka: broker service container). This step manufactures the compat claim; unchanged providers RE-EARN theirs rather than assume it.
4. **Publish (candidates only):** `cargo publish` from `providers/<name>` (existing crates.io token).
5. **Record (everyone):** regenerate + commit the compat table (in-repo, machine-readable: provider × version × certified core × wave).
6. **Wave release notes:** one GH release on the wave tag — "published: X (changelog excerpt); re-certified: Y, Z".

**Failure semantics:** waves are per-provider atomic — a failing candidate drops out with a report, the rest proceed. An UNCHANGED provider failing re-certification is the contract-drift alarm: it keeps its old compat row (visibly not certified for current core) and becomes the next wave's top work item. No silent rot.

**Core-release coupling (emergent from the pin policy):** pinned deps make every provider a mechanical candidate after a core release — one prep PR bumps all pins + patch versions + "compat: core 0.N" changelog lines, then a wave publishes compat releases for all. Providers therefore ride core's cadence plus ad-hoc feature waves between. Loosen pins to ranges only if/when the roster grows enough for this to hurt.

**Key property:** bumped and unbumped providers run the IDENTICAL pipeline differing at exactly one step (publish) — because the claim, not the upload, is what a release means.

### Dependencies
- [[CLOACI-T-0871]] first: audit roster (kafka + fs real; sensor/quorum/extract stay illustrative examples) and move real providers to `providers/`.
- Core 0.10.0 published on crates.io (v0.10.0 train, 2026-07-28) — providers must version-dep against released core.
- Flip `publish = false` off the real providers as part of their first wave prep.

### Risk Considerations
- Consumer-fixture E2E against crates.io core IS the certification — a shallow fixture means a shallow compat claim. Fixtures must instantiate constructors and execute, not just compile.
- The nightly contract-drift lane (build providers against MAIN's contract crates) complements waves: early warning that the NEXT core release breaks providers, before release day.
- First wave is the plumbing shakedown: expect crates.io first-publish quirks (mirror the core train's soft-fail handling).

## Status Updates **[REQUIRED]**

### 2026-07-31 — BUILD-OUT COMPLETE (branch feat/t0872-provider-waves); BOTH providers certified locally

All six acceptance criteria implemented:
- `providers/<name>/certify/` harnesses (fs = grant demo adapted; kafka = native test adapted, broker REQUIRED — no vacuous pass): bin crates whose cloacina deps resolve from CRATES.IO, provider from `..`; `exclude = ["certify"]` on provider manifests so publish never ships them. **Both PASS locally**: fs 3/3 grant cases; kafka 3/3 real messages through the signed native package via published packaging API + loader. Kafka harness gotcha: boundary frames are the fidius bincode wire — `deserialize` before JSON (matches the in-repo test).
- `scripts/provider_wave.py` (standalone): classify (crates.io query; UA header required), guard (changelog / no path deps / publish flag / contract pin), compat (regenerate COMPAT.toml; uncertified providers keep their previously-earned row), pr-check (the PR guard).
- `providers/COMPAT.toml` — machine-generated tested set, seeded from the local certification runs (wave 2026.07-preflight).
- `.github/workflows/provider_release.yml` — the wave: per-provider jobs (classify → guard → certify → publish, already-published tolerated) + record job (compat PR via bot branch + wave GH release on providers-v* tags). Kafka job runs a named apache/kafka:3.9.0 container (docker-exec producer parity with the dev stack).
- `.github/workflows/provider_guard.yml` — PR guard on providers/** (certify/ exempt).
- `.angreal/task_providers.py` — `providers check` + `providers wave` (pre-flight; tag push stays human).
- PROVIDERS.md updated with the machinery.

Remaining for close: PR merge → wave prep PR (flip publish=false) → inaugural `providers-v*` tag → first wave publishes both.

### 2026-07-31 — Certification step PROVEN by dry-run: pure crates.io, all cases green

Manual dry-run of the wave's certify step, fully outside the repo (scratchpad `crates-io-cert/`): ship-form `cloacina-provider-fs` + a copy of `fs-grant-demo` with `cloacina`/`cloacina-workflow`/`cloacina-build` flipped to **crates.io 0.10** version deps. Result: **all three grant cases pass E2E** — packaged to a WASM component by the PUBLISHED packaging API, loaded by the published loader, both suite members via `constructor!`: granted read ✓ / default-closed denial ✓ / granted write ✓ — zero repo code in the graph except the unpublished provider. The run also validated the re-certification thesis on contact: it immediately caught real drift (`fs-grant-demo`, in no CI lane per T-0892, had rotted against I-0139's `ProviderPackageOptions.runtime` — fixed, PR #220). Wave implementation note: this scratch-copy + version-dep-flip recipe IS the certify-harness shape; kafka's variant needs the native path + a broker container.

### 2026-07-28 — Validated model + urgency raised during 0.10.0 release prep

The consumption chain was validated in code end-to-end (user prompt), confirming this task's premise exactly:
- A consumer depends on the provider **as a plain Cargo dependency** — `provider-consumer-fixture/Cargo.toml`: "The provider, referenced exactly as a consumer would (A-0010: `from` = this name)".
- The **workflow package is what gets compiled**; the compiler resolves its deps from crates.io in production (`cloacina-compiler/src/build.rs:696-698`: "real packages resolve from crates.io" — path-dep injection is a dev/test-only shim).
- For NATIVE providers (I-0139), the compiler's provider bundler (`pack_providers`/`bundle_providers`, T-0907) builds the host cdylib while compiling the consuming workflow — so **crates.io publication of the provider crate is the complete consumer story**; `cloacinactl constructor package --native` is the standalone/manual packaging path.

**Consequence:** until this task lands, NO server-compiled workflow can use `cloacina-provider-kafka` (or any first-party provider) — every provider still carries `publish = false` "until the providers publish home (T-0871/0872) exists" per its own Cargo.toml. With kafka (I-0139, completed) now the flagship REAL provider, T-0871's audit question has a partial answer (kafka + likely fs are real; sensor/quorum/extract likely illustrative) and this task's priority rises accordingly.

(A duplicate task CLOACI-T-0909 was created 2026-07-28 in error and archived; nothing from it supersedes this task's design direction: per-provider tags `provider-<name>-vX.Y.Z` → publish that crate to crates.io, separate workflow from core's release train.)
