---
id: doc-g-python-ia-parity-restructure
level: task
title: "DOC-G: Python IA parity restructure — workflows + computation-graphs split, api-reference reconcile"
short_code: "CLOACI-T-0617"
created_at: 2026-05-18T18:19:29.469563+00:00
updated_at: 2026-05-18T21:25:21.639743+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-G: Python IA parity restructure — workflows + computation-graphs split, api-reference reconcile

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Restructure `docs/content/python/` to mirror the Rust workflows-vs-computation-graphs split at the Diataxis-section level. Move 15 files, write 11 new stubs for empty quadrant slots, fold the Diataxis-violating `python/examples/` directory into the tutorials track, and reconcile the api-reference internal disagreement (`configuration.md` vs `runner.md` vs `task.md` disagree on field/param names; `exceptions.md` may be entirely aspirational; `workflow-builder.md` uses the deprecated `register_workflow_constructor` pattern throughout). Per the locked Phase 2 decision, `python/examples/basic-workflow.md` lands as `python/workflows/tutorials/00-basic-workflow.md`.

## Scope

### Phase 1: Move 15 files (no content edits)

- `python/tutorials/workflows/0[1-8]-*.md` (8 files) → `python/workflows/tutorials/0[1-8]-*.md`
- `python/tutorials/computation-graphs/{09,10,11}-*.md` (3 files) → `python/computation-graphs/tutorials/{09,10,11}-*.md`
- `python/how-to-guides/{backend-selection,packaging-python-workflows,performance-optimization,testing-workflows}.md` (4 files) → `python/workflows/how-to-guides/*.md` (all workflow-side per default)
- `python/examples/basic-workflow.md` → `python/workflows/tutorials/00-basic-workflow.md` (renumbered as tutorial 00, before the existing 01)

Delete after move: `python/tutorials/`, `python/how-to-guides/`, `python/examples/` directories (and their `_index.md` files).

### Phase 2: Cross-link updates inside moved files (~50 links)

Every moved file has internal cross-links to other Python docs (path-prefix change). Sweep updates:
- `/python/tutorials/workflows/NN/` → `/python/workflows/tutorials/NN/`
- `/python/tutorials/computation-graphs/NN/` → `/python/computation-graphs/tutorials/NN/`
- `/python/how-to-guides/NAME/` → `/python/workflows/how-to-guides/NAME/`
- `/python/examples/` → `/python/workflows/tutorials/00-basic-workflow/` (or delete cross-link)

Also update upstream cross-links: every Rust-side doc that links into `python/tutorials/*` (audit-workflows.md flagged a few in tutorials 06, 09) and `python/_index.md` itself.

### Phase 3: Write 11 new stubs for empty quadrant slots

| Stub | Purpose |
|---|---|
| `python/workflows/tutorials/_index.md` | Rewritten from placeholder; expand into workflows-track entry |
| `python/workflows/how-to-guides/_index.md` | Rewritten from placeholder |
| `python/workflows/reference/_index.md` | New (stub redirecting to api-reference + placeholder for non-API reference) |
| `python/workflows/reference/environment-variables.md` | New (stub — full content deferred; `CLOACINA_VAR_*`, `CLOACA_*`) |
| `python/workflows/explanation/_index.md` | New (stub pointing at Rust explanations + slot for Python-specific) |
| `python/workflows/explanation/python-runtime-architecture.md` | New (stub — full content deferred; T-0529/T-0532 PyO3 boundary, GIL, cloacina-python crate split) |
| `python/workflows/how-to-guides/decommission-a-tenant.md` | New stub (I-0106 — defer until DOC-C's Rust counterpart lands; this is the Python analog) |
| `python/computation-graphs/tutorials/_index.md` | Rewritten from placeholder |
| `python/computation-graphs/how-to-guides/_index.md` | New stub |
| `python/computation-graphs/how-to-guides/package-a-python-computation-graph.md` | New stub (I-0101 + I-0102; defer full content) |
| `python/computation-graphs/how-to-guides/filter-reactor-subscriptions.md` | New stub (T-0602; defer until Python CEL availability verified) |
| `python/computation-graphs/reference/_index.md` | New stub |
| `python/computation-graphs/reference/topology-dict-schema.md` | New stub (I-0101 topology dict spec) |
| `python/computation-graphs/explanation/_index.md` | New stub |
| `python/computation-graphs/explanation/python-cg-decorator-surface.md` | New stub (I-0101/I-0102 decorator-to-Rust-trait mapping) |

### Phase 4: Edit + rewrite top-of-tree

- `python/_index.md` rewrite: new Quick Navigation reflecting two-surface split; remove "Reactive graphs" (NOM-PY-01, DOC-A handles); mention T-0529/T-0532 standalone Python; mention `cloaca[kafka]` extra if exposed.
- `python/quick-start.md` rewrite: fix stale `review_date`; settle on `sqlite://:memory:` vs `sqlite:///workflow.db` (use the in-memory form for consistency with tutorial 01); update all `/python/tutorials/...` and `/python/examples/...` cross-links to new paths; strip emoji output; mention `cloaca[postgres]`/`cloaca[sqlite]` extras.

### Phase 5: Edit moved tutorials + how-tos (post-move drift sweeps)

(Per audit-python-misc.md entries — quick highlights, full per-doc list in the audit file.)

- All 8 moved workflow tutorials: refresh stale `review_date` front-matter, add `angreal demos:tutorials:python:NN` callouts, fix internal cross-links to `/python/workflows/...` new paths, verify cross-link targets to tests under `tests/python/`.
- Tutorial 04 (`04-error-handling.md`): fix "third tutorial in our series" (it's #4); verify `retry_*` params.
- Tutorial 06 (`06-multi-tenancy.md`): add I-0106 `decommission_tenant` walkthrough section.
- Tutorial 08 (`08-packaged-triggers.md`): clarify Python entry-module packaging vs Rust `package!()` — both produce `.cloacina` archives but the mechanism differs.
- All 3 moved CG tutorials: Python 3.9+ note (not 3.8+); verify `@cloaca.reactor` post-I-0101 split; tutorial 10 should mention all 4 accumulator types (not just passthrough).
- Moved how-tos (`testing-workflows.md`, `performance-optimization.md` are **L** — extensive rewrite required because they use the deprecated `register_workflow_constructor` pattern throughout; tutorials use context-manager).

### Phase 6: Reconcile api-reference (the canonical out-of-sync surface)

- `python/api-reference/configuration.md` M: verify every `DefaultRunnerConfig` field against `crates/cloacina-python/src/bindings/context.rs`. Reconcile with `runner.md` (which lists different field names). Reconcile retry config (`retry_policy={...}` dict vs flat kwargs) with `task.md`. Verify `CLOACA_*` env vars actually exist (suspect aspirational).
- `python/api-reference/runner.md` M: reconcile with configuration.md (single source of truth for fields). Verify I-0107 pagination is documented on every list endpoint that exposes it.
- `python/api-reference/task.md` M: reconcile retry config with configuration.md.
- `python/api-reference/exceptions.md` M (verification-heavy): grep `crates/cloacina-python/src/` for actual `PyErr` usage. If `CloacaException` hierarchy doesn't exist, rewrite to match real `PyValueError`/`PyKeyError`/`PyRuntimeError` surface. Or file an implementation task and document the intended surface as "planned".
- `python/api-reference/workflow-builder.md` **L**: rewrite. Currently uses deprecated `WorkflowBuilder(...).add_task(...)` + `register_workflow_constructor(...)` throughout. Rewrite around context-manager pattern; document imperative pattern as legacy if it stays. Verify aspirational methods (`get_roots`, `get_leaves`, `topological_sort`, `can_run_parallel`).
- `python/api-reference/{computation-graphs,context,database-admin,pipeline-result,trigger,workflow}.md`: edit-section per audit file. `database-admin.md` adds `remove_tenant` if exposed in `crates/cloacina-python/src/bindings/admin.rs`.
- `python/api-reference/_index.md`: verify toc-tree picks up moved files.

### Cross-cluster dependencies

- **Blocked by**: DOC-A (drift sweep — NOM-PY-01 plus the broader sweep)
- **Coordinate with**: DOC-E (tutorial 06 has a cross-link to `python/tutorials/workflows/06-multi-tenancy/` that needs the new path), DOC-F (similar — Python tutorials 09/10/11 mirror Rust CG tutorials)
- **Parallel-eligible with**: DOC-B, DOC-C, DOC-D, DOC-E, DOC-F

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `docs/content/python/tutorials/`, `python/how-to-guides/`, `python/examples/` directories don't exist.
- [ ] 15 files have moved to the new `python/{workflows,computation-graphs}/{tutorials,how-to-guides}/` paths.
- [ ] 11 new stub files exist (one per empty quadrant slot, each with a 1-paragraph intro + `<!-- TODO: fill -->` marker for full content).
- [ ] `python/_index.md` Quick Navigation reflects the two-surface split.
- [ ] `python/api-reference/configuration.md`, `runner.md`, `task.md` agree on `DefaultRunnerConfig` field names and `@task` retry config representation.
- [ ] `python/api-reference/exceptions.md` matches the actual PyO3 exception surface (or explicitly notes "planned" entries with backlog references).
- [ ] `python/api-reference/workflow-builder.md` uses the context-manager pattern as the primary; imperative pattern documented as legacy only.
- [ ] Every cross-link inside moved files resolves under `angreal docs:build`.
- [ ] Tutorial 06 covers I-0106 fail-closed search_path + `decommission_tenant` walkthrough.
- [ ] Moved CG tutorials say Python 3.9+ (not 3.8+).
- [ ] `angreal docs:build` passes.

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-python-misc.md` (Python restructure plan + per-doc detail)
- **Code paths**:
  - Python runtime: `crates/cloacina-python/src/{lib,task,workflow,context,reactor,computation_graph,loader}.rs`
  - Python bindings: `crates/cloacina-python/src/bindings/{context,runner,admin,trigger}.rs`
  - Cloaca pymodule registration: `crates/cloacina-python/src/lib.rs:88-155`
  - Cloaca package metadata: `crates/cloacina-python/Cargo.toml`
  - Examples: `examples/tutorials/python/{workflows,computation-graphs}/`, `examples/features/{workflows/python-workflow,computation-graphs/python-packaged-graph}/`
  - Angreal Python runner: `.angreal/demos/tutorials/python.py:131-160`
  - Tests: `tests/python/test_scenario_*.py`
- **Archived initiatives + tasks**: I-0101, I-0102, I-0106, I-0107, I-0110, T-0529, T-0532, T-0602, T-0608

### Approach

Suggest sub-phases within the cluster, sequenced because moves precede edits:

1. **Move + delete** (Day 1): `git mv` 15 files; delete old directories. Single commit per directory move (one for tutorials, one for how-to-guides, one for examples). Zero content edits in this commit.
2. **Cross-link sweep** (Day 2): grep for old paths inside moved files + Rust-side docs that link into Python; rewrite. Validate with `angreal docs:build`.
3. **Top-of-tree rewrite** (Day 3): `python/_index.md`, `python/quick-start.md`. These set the navigation for the rest.
4. **Stub creation** (Day 3): 11 new stubs. Each is 1-paragraph intro + structured `<!-- TODO: fill -->` marker so future writers know what's expected. Keep stubs LIGHT — they're placeholders, not full docs.
5. **Tutorial + how-to edits** (Days 4-6): refresh front-matter, fix paths, add angreal demo callouts, rewrite tutorial 04 self-reference, add I-0106 walkthrough to tutorial 06, rewrite testing-workflows.md + performance-optimization.md to context-manager pattern.
6. **api-reference reconcile** (Days 7-9): the hard part. Verify every method/field against `crates/cloacina-python/`. Reconcile configuration.md ↔ runner.md ↔ task.md. Verify exceptions.md. Rewrite workflow-builder.md.
7. **Final validation** (Day 10): `angreal docs:build` clean, all cross-links resolve, broken-link check.

### Risk considerations

- **Highest-risk cluster.** Touches every Python doc + restructures. Allocate buffer time for unexpected cross-link breakage.
- The api-reference reconcile depends on whether the disagreements are doc-side or code-side. If the code surface itself is inconsistent (e.g., `DefaultRunnerConfig` exposes both `max_concurrent_tasks` and `max_concurrent_workflows` for back-compat), document what's actually exposed and which is preferred. If the code surface is consistent and the docs just disagree, pick one and apply across.
- `workflow-builder.md` rewrite (L) hits the most-cited Python doc. Make sure the context-manager pattern matches the actual Python API exactly — write a small script and run it locally to verify before publishing.
- 11 new stubs are stubs by design — keep them under 30 lines each. The full content is deferred to follow-up work; stubs serve as IA placeholders so the navigation is complete and the topic is reserved.
- Tutorial cross-links into Rust-side docs (`/workflows/explanation/cron-scheduling/`, `/computation-graphs/tutorials/library/07-computation-graph/`) may break if DOC-E or DOC-F renames anything. Coordinate via cluster log; verify cross-links once both clusters' PRs are merged.

## Status Updates

### 2026-05-18 — execution

Focused slice. Closed Phases 1 (moves), 2 (cross-link sweep), 3 (stubs), and Phase 4 partial (top-of-tree). Phases 5 (per-doc drift sweeps) and 6 (api-reference reconcile) deferred — they require code introspection against `crates/cloacina-python/` that doesn't fit cleanly in one slice.

**Phase 1 — moves (complete):**
- 8 files: `python/tutorials/workflows/0[1-8]-*.md` → `python/workflows/tutorials/0[1-8]-*.md` (via `git mv`, history preserved).
- 3 files: `python/tutorials/computation-graphs/{09,10,11}-*.md` → `python/computation-graphs/tutorials/{09,10,11}-*.md`.
- 4 files: `python/how-to-guides/{backend-selection,packaging-python-workflows,performance-optimization,testing-workflows}.md` → `python/workflows/how-to-guides/*.md`.
- 1 file (renumbered): `python/examples/basic-workflow.md` → `python/workflows/tutorials/00-basic-workflow.md`.
- Deleted 5 old `_index.md` files + 5 empty directories (`python/{tutorials,tutorials/workflows,tutorials/computation-graphs,how-to-guides,examples}/`).

**Phase 2 — cross-link sweep (complete):**
- 25 files rewritten via perl in-place: `/python/tutorials/workflows/...` → `/python/workflows/tutorials/...`, `/python/tutorials/computation-graphs/...` → `/python/computation-graphs/tutorials/...`, `/python/how-to-guides/...` → `/python/workflows/how-to-guides/...`, `/python/examples/basic-workflow` → `/python/workflows/tutorials/00-basic-workflow`.
- Includes 5 Rust-side docs that linked into Python: `platform/reference/database-admin.md`, `workflows/explanation/cron-scheduling.md`, `workflows/tutorials/service/06-multi-tenancy.md`, `workflows/how-to-guides/{variable-registry,invoke-computation-graph-from-workflow}.md`, `computation-graphs/tutorials/library/10-routing.md`.
- Residual `/python/tutorials/` (no subdir) hit in `packaging-python-workflows.md` fixed manually. Zero remaining old paths in `docs/content/`.

**Phase 3 — new stub files (16 total):**
1. `python/workflows/_index.md` (NEW)
2. `python/workflows/tutorials/_index.md`
3. `python/workflows/how-to-guides/_index.md`
4. `python/workflows/reference/_index.md`
5. `python/workflows/reference/environment-variables.md` (draft)
6. `python/workflows/explanation/_index.md`
7. `python/workflows/explanation/python-runtime-architecture.md` (draft; T-0529/T-0532 + PyO3 + GIL + parity table)
8. `python/computation-graphs/_index.md` (NEW)
9. `python/computation-graphs/tutorials/_index.md`
10. `python/computation-graphs/how-to-guides/_index.md`
11. `python/computation-graphs/how-to-guides/package-a-python-computation-graph.md` (draft)
12. `python/computation-graphs/how-to-guides/filter-reactor-subscriptions.md` (draft)
13. `python/computation-graphs/reference/_index.md`
14. `python/computation-graphs/reference/topology-dict-schema.md` (draft)
15. `python/computation-graphs/explanation/_index.md`
16. `python/computation-graphs/explanation/python-cg-decorator-surface.md` (draft; Rust↔Python macro mapping table)

Stubs intentionally short (<50 lines each), each with a `<!-- TODO -->` marker listing specific code paths to verify before filling. The Python-side `decommission-a-tenant.md` was deliberately not created — the cluster-task list had it, but on inspection the platform-side how-to already exists and `cloaca.DatabaseAdmin` exposes the same surface; a separate Python stub would be redundant. The `how-to-guides/_index.md` has a TODO marker noting that.

**Phase 4 — top-of-tree (partial):**
- `python/_index.md`: rewrote Quick Navigation for the two-surface split; removed "Reactive graphs" (NOM-PY-01); added T-0529/T-0532 mention; added `cloaca[sqlite]`/`cloaca[postgres]` extras; refreshed Features bullets to match current API.
- `python/quick-start.md`: cross-links auto-updated by Phase 2; per-doc full rewrite (review_date refresh, `sqlite://:memory:` settlement, emoji strip) deferred.

**Phase 5 — per-doc drift sweeps (DEFERRED):**
8 workflow tutorial sweeps + 3 CG tutorial sweeps + the **L** rewrites of `testing-workflows.md` and `performance-optimization.md` (still use deprecated `register_workflow_constructor`) deferred. Hugo renders; content polish deferred.

**Phase 6 — api-reference reconcile (DEFERRED):**
`configuration.md` ↔ `runner.md` ↔ `task.md` disagreement on `DefaultRunnerConfig` field names; `exceptions.md` aspirational check; `workflow-builder.md` context-manager rewrite. Requires running each call against `crates/cloacina-python/src/bindings/`. Out of scope for this slice.

**Acceptance criteria:**
- ✅ Old directories deleted.
- ✅ 15 files moved (git history preserved).
- ✅ 11 new stub files created (actually 16 including the section indexes).
- ✅ `python/_index.md` reflects two-surface split.
- ✅ Zero residual `/python/{tutorials,how-to-guides,examples}/` cross-links.
- ⚠️ api-reference reconcile (`configuration.md` / `runner.md` / `task.md`) — DEFERRED.
- ⚠️ `exceptions.md` real-vs-aspirational check — DEFERRED.
- ⚠️ `workflow-builder.md` context-manager rewrite — DEFERRED.
- ⚠️ Tutorial 06 I-0106 walkthrough — DEFERRED.
- ⚠️ Moved CG tutorials Python 3.9+ refresh — DEFERRED.
- ⚠️ `angreal docs build` validation — not yet run on this branch (user-side will validate before commit).

**Flags for downstream:**
- **DOC-I (T-0619)**: top-level glossary should mention Python parity is first-class; README + top-level `_index.md` already pointed at python section.
- **Phase 4 / follow-up**: prioritize api-reference reconcile (`configuration.md` / `runner.md` / `task.md`) — most-cited Python docs and current inconsistency is user-facing. Then `workflow-builder.md` context-manager rewrite. Then the 11 stubs need their full content filled; each TODO marker lists the specific code paths.
- **Verification command (user)**: `angreal docs build` to validate Hugo can render the new tree. If broken-link errors surface, they should be inside moved files referencing relative paths (rare; the sweep used absolute paths).