---
id: multi-arch-artifacts-per-target
level: task
title: "Multi-arch artifacts — per-target cdylibs with triple-matched dispatch"
short_code: "CLOACI-T-0780"
created_at: 2026-06-23T02:04:15.730326+00:00
updated_at: 2026-06-23T17:32:23.061630+00:00
parent: CLOACI-I-0131
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0131
---

# Multi-arch artifacts — per-target cdylibs with triple-matched dispatch

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0131]]

## Objective **[REQUIRED]**

A package can hold only ONE cdylib today (workflow_packages, unique per
package+version, single compiled_data), and dispatch stamps build_target_triple
from the SERVER host. Heterogeneous fleets can't work: a non-matching agent
fail-closed refuses with no alternate artifact. Add per-target artifacts +
triple-matched dispatch so a package can carry an x86_64 AND aarch64 cdylib and
each agent is handed the one matching its target_triple. Extends the per-tenant
compiler pattern (CLOACI-T-0779) to per-target.

## Plan (additive — host path untouched, zero risk to current execution)

- **Storage:** new `package_artifacts(content_hash, package_name, version,
  tenant_id, target_triple, compiled_data, created_at; unique(name,version,tenant,
  triple))` — EXTRA per-triple cdylibs. workflow_packages stays the primary
  (host) build. sqlite + postgres migration.
- **Dispatch:** select artifact for (package, tenant, AGENT triple) =
  package_artifacts[triple] ?? workflow_packages primary; stamp build_target_triple
  from the chosen artifact's actual triple (not server host). Host agents keep
  hitting the primary (unchanged).
- **Compiler:** `--build-target <triple>` flag (alongside --tenant-schema); when
  set + != host, store into package_artifacts tagged with the triple. Actual
  cross-cargo-build needs cross toolchains in the image — DEFERRED (no demo value
  single-arch); wire the flag + storage now.
- **Agent fetch:** unchanged (content-addressed by digest).
- **VERIFY (single-arch):** insert a synthetic 2nd-triple artifact; assert
  get_dispatch(package, "aarch64-…") returns it while host triple returns the
  primary — proves triple-matched selection. Real cross-exec needs a 2nd-arch
  runner (out of scope).

## Status Updates **[REQUIRED]**

- 2026-06-23: Scoped off the per-tenant compiler (T-0779). User: wire up multi-arch
  now. Additive package_artifacts + triple-matched dispatch; cross-toolchain
  deferred. Building.
- 2026-06-23: MATCHING HALF DONE + verified. Shipped: package_artifacts table
  (sqlite 027 / postgres 031); DAL get_artifact_digest_for_target /
  get_artifact_data_by_content_hash / upsert_artifact, with
  get_compiled_data_by_content_hash falling back to package_artifacts; fleet
  dispatch prefers the artifact for the SELECTED AGENT's triple and stamps
  build_target_triple from it (was always the server host), else the primary.
  Commits b5l0qjtj9 (core) + the trace-downgrade. VERIFIED synthetically: a
  package_artifacts row for demo-slow-rust@aarch64-linux made the dispatch resolve
  Ok(Some("synthetic-multiarch-001")), the agent fetched THAT digest via the
  fallback (7×), and the run Completed — while cron packages with no per-target row
  returned Ok(None) → primary. The agent fail-closed triple guard is unchanged.
  REMAINING (producer half, deferred — no single-arch demo value): a compiler
  `--build-target` that fills package_artifacts needs (a) a per-(package,target)
  build queue [the pending queue is per-package today, so host+target compilers
  would race one row], and (b) real target-arch runners to native-build (or cross
  toolchains in the image). That's a separate follow-on; the matching/runtime half
  this task targeted is complete.

## Producer-half plan (phase 2 — user chose to build it; emulate targets via docker linux/amd64)

Model = SCAN-AND-FILL (avoids the per-package pending-queue race): a per-target
compiler doesn't claim pending rows; it finds SUCCESS packages lacking ITS arch's
artifact and builds them from the retained source (get_source_for_build), then
upserts package_artifacts[triple]. Native build in an emulated arch container ⇒
no cross-toolchains. Key scheme facts (verified in code):
- execute_build returns the cdylib as Vec<u8>; workflow_packages.content_hash is
  the SOURCE hash (same across arches) → per-target rows MUST use a distinct hash
  = sha256(cdylib bytes), so each arch has its own digest (no collision).
- get_source_for_build is gated build_status='success' — perfect for scan-and-fill.
- agent + compiler both derive the triple from host_target_triple() in their own
  container, so they align (x86 container ⇒ "x86_64-linux" on both sides).

Steps: (1) DAL find_packages_missing_target_artifact(triple, tenant[, name filter])
→ success rows with no package_artifacts[triple]; (2) compiler --build-target
<triple> runs the scan-and-fill loop (reuse execute_build; sha256(cdylib) →
upsert_artifact) instead of claim-pending; optional package-name filter so the
emulated build stays cheap (just demo-slow-rust); (3) demo compiler-x86
(platform linux/amd64, --build-target x86_64-linux, filter demo-slow-rust) +
agent-x86 (platform linux/amd64); (4) VERIFY: x86 agent runs demo_slow on the x86
cdylib (dispatch hands it package_artifacts[x86_64-linux], not the aarch64 primary).

## PRODUCER HALF — DONE + VERIFIED (true cross-arch execution)

- Built: DAL find_packages_missing_target_artifact; compiler --build-target/
  --build-target-package + run_per_target scan-and-fill (reuse execute_build,
  sha256(cdylib) -> upsert_artifact); demo compiler-x86 + agent-x86 (platform
  linux/amd64, profile 'multiarch'). Trim: build-arg CARGO_BINS builds compiler-x86
  with compiler+cloacinactl only (skips the cloacina-server crate). Commits on main.
- ENV LEARNINGS: qemu segfaults compiling Rust under emulation (rustc stack) — FIX
  = Rosetta (UseVirtualizationFrameworkRosetta=true), compiles cleanly. `docker
  compose build` delegated to the Desktop dashboard and silently produced no image
  in non-TTY; direct `docker build` streams+loads (use that). Enabling Rosetta needs
  a Docker restart, which disrupts the stack — bring postgres/server back after.
- VERIFIED end-to-end: compiler-x86 (amd64/Rosetta) built demo-slow-rust ->
  package_artifacts[x86_64-linux]=da17103b; agent-x86 registered x86_64-linux,
  dispatch handed it the x86 digest, it loaded the x86 cdylib (only an x86 process
  can) and a demo_slow run Completed on it (ingest completed while agent-x86 was the
  only public agent). aarch64 fleet unaffected.
- DEMO NOTE: agent-x86 is public-realm but only demo-slow-rust is built for x86, so
  it fail-closed REFUSES other public packages (demo-cron/demo-py-cron) — correct
  triple guard, but noisy. Cleanest watch = scale down the aarch64 `agent` replicas.
  Building all public packages for x86 (or scoping the agent) removes it — optional.

## COMPILE-EVERYTHING + INTERPRETED-PACKAGES (heterogeneous fleet complete)

- compiler-x86 default = compile WHOLE catalog (dropped --build-target-package). Bug
  found+fixed: execute_build returns EMPTY for interpreted (Python) packages (no arch
  cdylib) → run_per_target stored sha256("")=e3b0c44 empty rows; now skips empty
  artifacts. Rust packages backfill per-arch; Python skipped.
- INTERPRETED-PACKAGES-ARE-ARCH-INDEPENDENT (fleet_executor): resolve (digest,
  language) BEFORE agent selection; for language=python skip the arch filter (any
  agent eligible) and stamp the SELECTED agent's own triple so the fail-closed guard
  is a no-op. Compiled (Rust) path unchanged. DEMONSTRATED: with aarch64 public
  agents stopped, demo_py_workflow (4 Python tasks) ran to Completion on agent-x86
  (x86 running Python via its interpreter). "Any agent runs any package" now literal:
  Rust → any agent whose arch has a cdylib; Python → any agent.
- INFRA: emulated amd64 IMAGE builds (heavy) crash Docker Desktop alongside the live
  stack — build with the stack down, or recreate (not rebuild) once the image exists.
  Native server rebuilds are stable. package_artifacts survives crashes (pg volume).

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

*To be added during implementation*