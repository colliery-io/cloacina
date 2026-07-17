---
id: per-arch-native-provider-bundles
level: task
title: "Per-arch native provider bundles — target_triple on package_providers + triple-keyed staging"
short_code: "CLOACI-T-0908"
created_at: 2026-07-17T02:37:42.746499+00:00
updated_at: 2026-07-17T02:42:14.124456+00:00
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