---
id: per-arch-native-artifact-selection
level: task
title: "Per-arch native artifact selection at load — wire content_hash_for_target into the reconciler"
short_code: "CLOACI-T-0905"
created_at: 2026-07-15T12:09:20.216080+00:00
updated_at: 2026-07-16T12:33:53.201288+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# Per-arch native artifact selection at load — wire content_hash_for_target into the reconciler

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Close a PRE-EXISTING gap surfaced by I-0139: the per-arch artifact SELECTION at load. The compiler already builds native cdylibs per-arch into `package_artifacts.target_triple` (T-0780, `compiler/src/loopp.rs::run_per_target`), and the DAL exposes `content_hash_for_target(package, target_triple)` — but the reconciler load path (`registry/reconciler/loading.rs:271`) still reads the primary `compiled_data` and errors if absent, so it never picks the running agent's arch artifact. Wire `content_hash_for_target` into the reconciler/agent load so each agent loads its OWN arch's cdylib.

**Why in this initiative:** native providers are per-arch cdylibs and hit the exact same gap; fixing it here unblocks native providers AND benefits packaged Rust workflows (multi-arch fleets).

**Acceptance:**
- [ ] The reconciler/agent selects the artifact for its `target_triple` via `content_hash_for_target`, falling back to the primary build when no per-arch artifact exists.
- [ ] A multi-arch fixture (or simulated triple) loads the correct per-arch cdylib; missing-arch is a clear error, not a silent wrong-arch load.

Parent: [[CLOACI-I-0139]]. Independent of the native-provider tasks (can land first); benefits [[CLOACI-T-0780]] workflows too.

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

### 2026-07-16 — investigated, built, tested → `5e79fe10`. T-0905 DONE.

**Investigation corrections to the task premise:**
- `content_hash_for_target` doesn't exist by that name — the actual DAL fn is `get_artifact_digest_for_target(package, triple)` (workflow_packages.rs:241). And it is **version-agnostic** (filters name+triple only, `.first()` no ordering) — fine for fleet digest serving, WRONG for a reconciler load of a specific version.
- The fleet DISPATCH path was already per-arch aware (`fleet_executor.rs:516-534`). The genuine gap was ONLY the local reconciler (`loading.rs:271` always took primary `compiled_data`).
- `host_target_triple()` (fleet/protocol.rs:77, `{arch}-{os}` v1 format) is the shared triple vocabulary (compiler stamping + OQ-6 fail-closed agent comparison) — the reconciler now uses the same.
- `workflow_packages` has NO triple column for the primary build ⇒ can't fail-closed compare the primary; the fallback is kept and load failures carry PROVENANCE instead.

**Implementation:**
1. **DAL** `get_artifact_data_for_target(package, version, triple)` — VERSION-scoped single query on `package_artifacts` (tenant-neutral, like the T-0780 fns).
2. **`WorkflowRegistry` trait** gains `get_compiled_data_for_target(package, version, triple)` with default `Ok(None)` (filesystem/mocks keep the primary fallback); `WorkflowRegistryImpl` overrides via the DAL fn.
3. **Reconciler** (`loading.rs`): prefers the per-target artifact for `host_target_triple()`, falls back to primary; an `artifact_provenance` string is woven into BOTH failure modes — the no-data error names the missing triple, and a `build_view_rust` (dlopen) failure names which artifact was tried + a wrong-arch hint. `rust_cdylib_bytes` keeps its name/type so the downstream CG-step reuse is untouched.

**ACCEPTANCE:**
- [x] Reconciler selects its `target_triple`'s artifact, falling back to primary when absent.
- [x] Simulated-triple test `test_get_compiled_data_for_target_is_version_and_triple_scoped` (sqlite in-memory): simulated arch gets ITS bytes; missing triple → None (fallback); **another version's artifact never satisfies this version** (the scoping the old DAL fn lacked). Missing-arch load failure is a clear provenance-carrying error, not a silent wrong-arch load. 1/1 green; `--tests` compile clean under `constructors-wasm,sqlite`.

**Follow-up note (out of scope):** the fleet's `get_artifact_digest_for_target` remains version-agnostic — latent wrong-version risk on the DISPATCH path when multiple versions carry per-target artifacts; worth a ticket if multi-version fleets materialize.