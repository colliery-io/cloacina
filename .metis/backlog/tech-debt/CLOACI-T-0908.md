---
id: per-arch-native-provider-bundles
level: task
title: "Per-arch native provider bundles — target_triple on package_providers + triple-keyed staging"
short_code: "CLOACI-T-0908"
created_at: 2026-07-17T02:37:42.746499+00:00
updated_at: 2026-07-17T03:03:46.749358+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Per-arch native provider bundles — target_triple on package_providers + triple-keyed staging

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Close the known multi-arch gap I-0139 left behind (recorded in [[CLOACI-T-0907]]'s status notes): **NATIVE provider bundles are arch-specific, but `package_providers` stores exactly ONE build per (package, provider) with no `target_triple` column** — the arch of whatever machine ran the compiler. On a mixed-arch fleet, an agent on a different arch unpacks the bundle and fails at `load_library` (a loud dlopen error, but not self-healing; there is no other-arch build to select).

WASM provider bundles are arch-neutral and unaffected. Single-arch deployments (the demo stack, uniform docker fleets) are unaffected — that's why this is backlog, not urgent: it becomes real the day a mixed-arch fleet consumes a native provider (e.g. `cloacina-provider-kafka`).

**The fix is the T-0780/T-0905 pattern applied to `package_providers`** (both halves already exist for workflow cdylibs in `package_artifacts`):
1. Add `target_triple` to `package_providers` (migration: ADD COLUMN, backfill existing rows with the compiler host's triple — never DROP+CREATE on sqlite).
2. The per-target compiler scan (`cloacina-compiler/src/loopp.rs::run_per_target`) also rebuilds each success-package's NATIVE providers for its `build_target` (providers with the `[package.metadata.cloacina] runtime = "native"` marker; wasm providers skip — one arch-neutral row suffices) and upserts triple-keyed rows.
3. `stage_bundled_providers` (reconciler/loading.rs) and the agent bundle fetch select rows matching `host_target_triple()`, falling back to the primary row with a provenance-carrying error on dlopen failure (the exact T-0905 style: name which artifact was tried + the missing triple).

Related: [[CLOACI-T-0905]] (the workflow-artifact twin of this), [[CLOACI-T-0906]] (the flagship native provider), [[CLOACI-T-0780]].

**Acceptance:**
- [ ] `package_providers` rows are triple-keyed; the per-target compiler fills missing-arch NATIVE provider builds from retained consumer source.
- [ ] Reconciler/agent staging selects its own triple's bundle, falls back to primary when absent; a wrong-arch/missing-arch load failure names the triple and the artifact tried.
- [ ] A simulated-triple test (the `test_get_compiled_data_for_target_is_version_and_triple_scoped` pattern) proves selection is version- AND triple-scoped for provider rows.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [x] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [x] P2 - Medium (nice to have)
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

## Acceptance Criteria

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

### 2026-07-16 — activated; surface mapped. UPGRADE: this is a live BUG, not just a missing feature.

**Discovery:** `run_per_target` calls the FULL `execute_build` (loopp.rs:262), and since T-0907 `execute_build` ALSO re-bundles providers + `store_package_providers` (build.rs:314-414) — whose `upsert_provider` delete-filter has NO triple. ⇒ the moment a second-arch per-target compiler processes a native-provider package, it **clobbers the primary provider row with its own arch's cdylib**. The triple column fixes correctness, not just coverage.

**Surface (all confirmed):** migration next = 042 (pg + sqlite, ADD COLUMN style per 040); all consumers funnel through DAL `get_providers_for_package` (workflow_packages.rs:447) — reconciler impl (workflow_registry/mod.rs:708) + agent route `GET /v1/agent/providers/{digest}` (routes/agent.rs:418); agent client = cloacina-agent/main.rs:1298; `PackageProvider` is `Selectable` (by-name select — adding columns is safe).

**Plan:**
1. Migration 042 both backends: `ADD COLUMN target_triple TEXT` (NULL = primary build, exactly what existing rows are — no backfill) + `ADD COLUMN runtime TEXT NOT NULL DEFAULT 'wasm'` (lets the missing-scan target native rows without unpacking archives).
2. schema.rs (all package_providers blocks) + models gain both fields.
3. DAL: `upsert_provider` gains `runtime: &str, target_triple: Option<&str>`; delete-filter includes triple so primary + per-arch rows COEXIST (kills the clobber). New `find_packages_missing_target_provider(target, name_filter)`.
4. `PackedProvider` gains `runtime` (pack_providers already computes it).
5. build.rs: store passes runtime + `config.build_target` as triple (primary loop = None → NULL rows; per-target = triple rows), and per-target stores ONLY native providers (wasm bundles are arch-neutral).
6. `store_package_providers`: rows become `(name, version, runtime_str, archive)` + `target_triple: Option<&str>` (runtime as str — the contract crate is feature-gated).
7. Selection: pub `select_provider_rows_for_target(rows, triple)` (prefer exact-triple row per provider, else NULL primary). Reconciler selects for `host_target_triple()`; agent route gains `?target_triple=` (absent → primary rows = old-agent behavior); cloacina-agent sends its triple.
8. run_per_target: missing set = artifact-missing ∪ provider-missing.
9. `load_native_member`'s `load_library` error names the host triple.
10. Tests: DAL coexist/selection/version-scoping + selection-helper unit test.

### 2026-07-16 — IMPLEMENTED + PROVEN → `2b1ab795`. T-0908 DONE.

All 10 plan items landed. Two things the implementation surfaced:
- **The unique key had to change too**: `idx_package_providers_key` (039/028) was UNIQUE on (package, version, tenant, provider) — the new DAL test failed with a duplicate-key violation on the second arch row, proving the old key made coexistence impossible. Migration 042 rebuilds it with `COALESCE(target_triple,'')` appended (the same NULL pattern the key already used for tenant).
- **Old-agent compat**: the providers route treats an absent `?target_triple=` as "primary rows only" — byte-for-byte the pre-T-0908 response for agents that haven't been rebuilt.

**ACCEPTANCE — all MET:**
- [x] Rows are triple-keyed; `run_per_target` unions `find_packages_missing_target_provider` with the artifact scan and `execute_build` stores triple-keyed NATIVE rows (`config.build_target`), wasm skipped (arch-neutral).
- [x] Reconciler selects `host_target_triple()` with primary fallback; agent route selects per `?target_triple=`; the dlopen error names the host triple + the per-target hint.
- [x] `test_provider_rows_are_triple_scoped_and_selected_per_target` (postgres): primary + 2 per-arch + wasm rows COEXIST; per-arch re-upsert replaces only its triple; arm reader gets arm bytes; unknown arch falls back to PRIMARY, never another arch. Pre-existing round-trip test green. All five crates compile; **cg-feature-tour lane green end-to-end on the triple-aware code**.

**Bonus fix**: the clobber bug (second-arch per-target compile overwrote the primary provider row) is dead — the delete-filter is triple-scoped.

**Untested remainder (honest):** an actual two-arch fleet run (requires a second-arch per-target compiler container + a remote agent; the machinery is now in place and unit/integration-proven per component). The nightly per-target lane will exercise the producer half when it next runs.

### 2026-07-16 — TWO-ARCH VALIDATION RUN in progress (the T-0780 `multiarch` demo lane).

The demo compose already models the topology: `compiler-x86` (linux/amd64 under Rosetta, `--build-target x86_64-linux`, scan-and-fill) + `agent-x86`, against the aarch64-linux primary stack. Server carries `CLOACINA_VAR_KAFKA_BROKER=kafka:9092`; the workspace image bakes THIS branch at `/workspace` (the kafka provider's `__WORKSPACE__` path dep resolves in-container, pack-demo-fixtures convention).

**Run plan:**
1. Override scopes `compiler-x86` to `--build-target-package cg-feature-tour` (no whole-catalog backfill under emulation). Fresh volumes.
2. Build multiarch images (long pole: amd64 workspace under Rosetta), `up -d`.
3. Upload cg-feature-tour (`__WORKSPACE__` → `/workspace`) via host cloacinactl → server :8080 (demo bootstrap key).
4. Primary (aarch64-linux) compiler builds → NULL-triple native provider row; server stages it; kafka stream starts.
5. `compiler-x86` fills the x86_64-linux artifact + provider rows (rdkafka built under emulation).
6. **Assert (psql, demo postgres)**: TWO `package_providers` rows for cloacina-provider-kafka — (NULL, native) and ('x86_64-linux', native) — with DIFFERENT content hashes; plus the x86_64-linux `package_artifacts` row.

**Scope note:** the agent-side per-arch provider FETCH is exercised only by `constructor!`-node workflows (stream accumulators run on the SERVER); no demo package carries a native constructor! task node, so that leg stays covered by the route/DAL tests. This run proves the PRODUCER half — distinct per-arch native provider builds coexisting — on real (emulated) two-arch hardware.

### 2026-07-17 — TWO-ARCH RUN GREEN. The proof, verbatim from the demo postgres:

```
      provider_name      | runtime |    triple    |     hash     |  bytes
-------------------------+---------+--------------+--------------+---------
 cloacina-provider-kafka | native  | (primary)    | 767dd73d24f8 | 1115574   ← aarch64-linux (primary compiler container)
 cloacina-provider-kafka | native  | x86_64-linux | 91c37974933e | 1196618   ← amd64 build under Rosetta (compiler-x86)
```
Plus the `x86_64-linux` `package_artifacts` row for cg-feature-tour (4.0MB). **Two coexisting rows, different hashes/sizes — the primary was NOT clobbered** (the exact bug the pre-T-0908 code had). The x86 compiler log shows the full per-target flow: "bundling [metadata.providers] … cloacina-provider-kafka" → `Compiling cloacina-provider-kafka` (rdkafka under amd64 emulation) → "per-target: stored artifact … target=x86_64-linux". And the CONTAINERIZED arm64 server independently proved the consumer leg in a new environment: "Unpacked 1 bundled provider(s) for cg-feature-tour" → "provider-backed stream accumulator started (native, trusted)".

**Two infra bugs the run surfaced + fixed:**
1. `8a08d458` — ALL docker rust base images pinned 1.93 (production Dockerfile: 1.85), below wasmtime 46's 1.94 MSRV (the fidius 0.5.6 bump, T-0902). No container on the branch could build; every containerized CI lane would have hit it.
2. Dockerfile.demo's `/workspace/target` buildkit cache was SHARED across arch builds — concurrent multiarch builds collided ('File exists' mid-cargo). Fixed with an arch-keyed cache id (`cloacina-demo-target-${TARGETARCH}`); registry/git caches stay shared (cargo file-locks them).

**Run mechanics for posterity:** demo compose + `--profile multiarch` + an override scoping `compiler-x86` to `--build-target-package cg-feature-tour`; sequential image builds (arm64 set → compiler-x86 → agent-x86) until the cache fix landed; upload = host cloacinactl → :8080 with the pack staged `__WORKSPACE__`→`/workspace`.