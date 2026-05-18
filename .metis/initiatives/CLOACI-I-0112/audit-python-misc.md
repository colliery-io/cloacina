# Audit — python/, quick-start/, top-level, contributing/, operations fold-in, README (CLOACI-I-0112 Phase 2)

> Produced by parallel audit agent (general-purpose). Per-doc design.md entries; preserved verbatim for Phase 3 reference. Synthesis lives in [design.md](./design.md). This audit also produces the Python IA parity restructure plan.

## Top-level

### docs/content/_index.md (status: edit-section)
- **Category:** Index
- **Audience:** First-time visitor landing on the docs site root
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `_index.md:27` points to `/workflows/tutorials/` — fine, but no acknowledgement that there's a service-mode entrypoint or a CG entrypoint, just "Cloacina" and "Cloaca."
  - No top-level mention of `cloacinactl`, the install script (I-0111), or the Docker/Helm distribution path — the home page acts like all Cloacina is embedded library mode.
  - No mention of computation graphs as a peer surface to workflows.
  - No "see also: glossary / troubleshooting / quick-start" rail.
- **Coverage (May 2026 batch):** I-0111 (install), I-0101 (CG split — should at least call CG out)
- **Effort:** S

### docs/content/glossary.md (status: edit-section)
- **Category:** Reference
- **Audience:** Reader of any doc who needs a definition without leaving their flow
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `glossary.md:39` — "Computation Graph" entry says "reactive, streaming computation model." S-0011 bans "reactive" as a modifier of these primitives. Change to "event-driven, streaming computation model" or remove the modifier.
  - `glossary.md:135` — "Reactor" entry is correct post-S-0011 (no banned phrasing) and references `ComputationGraphScheduler` indirectly via `unload_reactor` at line 148 (good).
  - Missing entries: `cloacinactl`, `cloacina-compiler`, `cloacina-server`, `cloacina-daemon`, `cloacina-python`, `ApiError`, `CEL predicate`, `complete_task_transaction`, `install script`, `Helm chart`, `Filtered subscription`, `Verification org`.
  - Missing entry: `var` / `var_or` (`cloaca.var()` / `cloaca.var_or()`).
  - "Pipeline" entry (`glossary.md:121-123`) doesn't link to the metrics catalog where `cloacina_pipelines_total` lives; once metrics-catalog lands, add a cross-link.
- **Coverage (May 2026 batch):** I-0101, I-0102, I-0103, I-0106, I-0107, I-0110, I-0111, T-0529, T-0602, T-0610
- **Effort:** M

### docs/content/troubleshooting.md (status: rewrite)
- **Category:** Reference
- **Audience:** User who has a specific error or symptom and needs the cause + fix
- **Status delta:** rewrite (S-0011 sweep + version refresh + native-crash merge + post-I-0096 inventory model + post-T-0502 heartbeat-only recovery)
- **Drift / gaps found:**
  - `troubleshooting.md:439-452` — Section "10. Unresolved module or unlinked crate cloacina_computation_graph" pins `cloacina-computation-graph = { version = "0.4" }` and `cloacina-macros = { version = "0.4" }`. **VER-T-01.**
  - `troubleshooting.md:457-459` — `cloacina = { version = "0.4", features = ["macros"] }` — stale. **VER-T-02.**
  - `troubleshooting.md:436` — claims the macro references `cloacina-computation-graph` types and tells users to add it; with I-0102 `package!()` unification this is no longer the user-facing surface (macros are owned by the `package!()` shell). Rewrite section to point at `cloacina::package!()` and `cloacina = { version = "0.6.1", features = ["macros", "packaged"] }`.
  - `troubleshooting.md:534` — "Cloacina test harness creates temporary databases per test" — needs T-0608 update (SQLite `:memory:` substitution in test paths).
  - `troubleshooting.md:813-880` — Section "19. ImportError / SIGSEGV on import" is the merge target for `docs/SIGSEGV_TROUBLESHOOTING.md`. The current section already references the historical workaround; the merge needs to bring in the alternative-approaches list and debugging tips from the source file as a `### Native crash troubleshooting (historical)` subsection.
  - `troubleshooting.md:153-167` — Stale-claim section still describes the sweeper but doesn't note T-0502 (RecoveryManager removed; heartbeat sweeper is now the *sole* recovery path).
  - `troubleshooting.md:212-217` — `registry_enable_startup_reconciliation` — verify against current `DefaultRunnerConfig` builder.
  - `troubleshooting.md:565-580` — Tenant provisioning section already uses `DatabaseAdmin` and `create_tenant` — needs an I-0106 update for the fail-closed `SET search_path` semantics and the `remove_tenant` teardown path.
  - `troubleshooting.md:891-927` — Section "20. Backend not available" — `cloaca.features()` is documented but not present in `cloaca` pymodule (verified `crates/cloacina-python/src/lib.rs:88-155`). **API drift.**
  - `troubleshooting.md:1041` — `max_concurrent_tasks(2)` — verify the field on the post-T-0483 `DefaultRunnerConfig::builder()` (likely still correct; ServiceManager extraction was internal).
  - `troubleshooting.md:60` — `angreal db migrate` — verify this task name still exists post-`cloacinactl` redesign.
- **Coverage (May 2026 batch):** I-0102, I-0106, T-0487, T-0502, T-0529 / T-0532, T-0608
- **Effort:** L

## Quick-start

### docs/content/quick-start/_index.md (status: edit-section)
- **Category:** Index
- **Audience:** New user who wants to find the right tutorial track
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `quick-start/_index.md:51` — Points at "Tutorial 01 — Deploy a Server" at `/platform/tutorials/01-deploy-a-server`. Per initiative IA-02, the `platform/tutorials/` track is thin; for now this link is correct but should add a "more tutorials coming" note.
  - `quick-start/_index.md:31` — "Tutorial 01 — Your First Workflow" at `/workflows/tutorials/library/01-first-workflow` — verify the path after IA cleanup. **IA-Q-01.**
  - `quick-start/_index.md:70` — "Tutorial 10 — Cross-Package Reactor Binding" — verify Rust tutorial path.
  - No mention of `cloacinactl` install one-liner from `quick-start/install.md`.
- **Coverage (May 2026 batch):** I-0111 (should call out install)
- **Effort:** S

### docs/content/quick-start/install.md (status: edit-section)
- **Category:** How-to
- **Audience:** Operator / developer installing `cloacinactl` for the first time
- **Status delta:** edit-section
- **Drift / gaps found:**
  - **VER-Q-01** — `quick-start/install.md:28` — "Pinning a version" example uses `--version v0.6.0`; current crates are at `0.6.1`.
  - `install.md:93` — `cargo install --git https://github.com/colliery-software/cloacina cloacinactl` — verify the org slug; the rest of the file uses `colliery-software` but `README.md:8` references `colliery-io` in the image URL. Pick one and audit.
  - `install.md:97` — "Pass `--tag vX.Y.Z` to pin a release" — `cargo install --git` does not take `--tag`. The correct flag is `--rev` (commit) or `--tag` only via `--branch`/`--tag` *if* recent cargo. Verify; likely needs correction to `--rev <commit>` or `--branch <tag>`.
  - `install.md:55-59` — Supported platforms list looks current (x86_64 + aarch64 linux/darwin) — confirm against the I-0111 install-script README in repo.
  - `install.md:104` — "pip install cloaca" — should mention extras (`[sqlite]`, `[postgres]`) for parity with `python/_index.md:14-16`.
  - **Gap:** No mention of the Docker image (`ghcr.io/colliery-software/cloacina-server:<tag>`) or the Helm chart. These are I-0111 surfaces and belong on this page or a sibling "install via container" page.
- **Coverage (May 2026 batch):** I-0111, T-0610, partially T-0538 (`cloacinactl daemon` subcommand mention)
- **Effort:** S

## Python — restructure plan

The Python tree currently has tutorials split (under `python/tutorials/workflows/` and `python/tutorials/computation-graphs/`) but how-to / reference / explanation are flat, and `python/examples/` is a Diataxis violation. Per the initiative IA spec, the target is `python/{workflows,computation-graphs}/{tutorials,how-to-guides,reference,explanation}/` mirroring the Rust side, plus `python/api-reference/` (unchanged structurally — it's auto-generated) and `python/_index.md` + `python/quick-start.md` at the top.

The move pattern:
- Tutorials: `python/tutorials/workflows/NN-*.md` → `python/workflows/tutorials/NN-*.md`; `python/tutorials/computation-graphs/NN-*.md` → `python/computation-graphs/tutorials/NN-*.md`. Delete the old `python/tutorials/` parent (and its `_index.md`).
- How-to guides: `python/how-to-guides/*.md` → `python/{workflows,computation-graphs}/how-to-guides/*.md` based on subject. All four current how-tos (`backend-selection`, `packaging-python-workflows`, `performance-optimization`, `testing-workflows`) are workflow-side per the constraint default. Delete the old flat `python/how-to-guides/` directory.
- Reference: No flat `python/reference/` exists today — but `python/api-reference/` does and stays. Empty `python/{workflows,computation-graphs}/reference/` slots get stub `_index.md` markers (gaps).
- Explanation: No `python/explanation/` exists today. Empty `python/{workflows,computation-graphs}/explanation/` slots get stub markers.
- `python/examples/` (Diataxis violation): fold into the new tree. `basic-workflow.md` → relocate as `python/workflows/tutorials/00-basic-workflow.md`. Delete `python/examples/_index.md` and the `python/examples/` directory.
- `python/_index.md` and `python/quick-start.md` stay at their current location; both need substantive edits.

After the move, `python/tutorials/`, `python/how-to-guides/`, and `python/examples/` directories no longer exist.

### python/_index.md (status: rewrite)
- **Category:** Index
- **Audience:** Python developer landing on the Cloaca tree
- **Status delta:** rewrite (post-restructure)
- **Drift / gaps found:**
  - `python/_index.md:27-34` — Quick Navigation table doesn't reflect the two-surface split.
  - `python/_index.md:21` — "Reactive graphs" — banned phrase per S-0011. **NOM-PY-01.** Change to "event-driven graphs" or "computation graphs."
  - No mention of `cloacina-python` crate split (T-0529 / T-0532) — standalone Python usage works without a server.
  - `python/_index.md:13-17` — Install commands look current. Should add a `pip install cloaca[kafka]` extra mention if the feature is exposed in wheels.
- **Coverage (May 2026 batch):** T-0529, T-0532, I-0101, I-0102 indirectly
- **Effort:** S

### python/quick-start.md (status: rewrite)
- **Category:** Tutorial
- **Audience:** New Python user running their first workflow in 5 minutes
- **Status delta:** rewrite (drift + restructure cross-links + S-0011 cleanup)
- **Drift / gaps found:**
  - `python/quick-start.md:6` — Front-matter `review_date: "2025-01-07"` is over a year old. Stale.
  - `python/quick-start.md:99` — `sqlite:///workflow.db` — fine for tutorial, but the example then uses `sqlite://:memory:` in tutorial 01. Inconsistency.
  - `python/quick-start.md:287-289` — `{{< button >}}` link points at `/python/examples/` — that directory is being deleted.
  - `python/quick-start.md:293-298` — "Recommended Learning Path" hard-codes paths like `/python/tutorials/workflows/01-first-python-workflow/`; after restructure these become `/python/workflows/tutorials/01-first-python-workflow/`. **Cross-link update required across the restructure.**
  - `python/quick-start.md:24-30` — Build-from-source note correctly mentions PostgreSQL `libpq-dev` and Rust toolchain. Should also mention `cloaca[postgres]` vs `cloaca[sqlite]` extras consistent with `python/_index.md:14-16`.
  - `python/quick-start.md:110` — Uses emoji checkmarks/X marks in example output. The repository convention is no emojis in docs.
- **Coverage (May 2026 batch):** T-0529 / T-0532 (standalone Python — quick-start *is* the standalone path); T-0608 (`:memory:`)
- **Effort:** M

## python/workflows/tutorials

### python/workflows/tutorials/_index.md (status: new — replaces the placeholder)
- **Category:** Index
- **Audience:** Python developer entering the workflows learning path
- **Status delta:** new-write (the current `python/tutorials/workflows/_index.md` is a one-line `{{< toc-tree >}}` — when moved, expand it to be the workflows-tutorial entrypoint with a numbered list and "what's next" pointer to CG tutorials)
- **Effort:** S

### python/workflows/tutorials/01-first-python-workflow.md (status: move-from python/tutorials/workflows/01-first-python-workflow.md)
- **Category:** Tutorial
- **Audience:** Python developer writing their first Cloaca workflow
- **Status delta:** move + edit-section (S-0011 cleanup + version sweep + reproducibility)
- **Drift / gaps found:**
  - `01-first-python-workflow.md:6` — `review_date: "2025-01-07"` stale.
  - `01-first-python-workflow.md:113` — "Modify this example to add validation or parallel processing" — fine; cross-check that the `examples/tutorials/python/workflows/01_first_workflow.py` source exists and matches (verified).
  - **Reproducibility:** No `angreal demos:tutorials:python:01` callout. The angreal runner at `.angreal/demos/tutorials/python.py:131-160` registers a command per numbered tutorial — the tutorial doc should explicitly say "Reproduce with `angreal demos:tutorials:python:01`."
  - `01-first-python-workflow.md:441` — References `test_scenario_02_single_task_workflow_execution.py` — exists in `tests/python/`.
  - `01-first-python-workflow.md:438-439` — Cross-link to `/python/examples/` — must be updated since that dir is being deleted.
- **Coverage (May 2026 batch):** T-0608 (`:memory:` — example uses `sqlite://:memory:` at line 121, current)
- **Effort:** S

### python/workflows/tutorials/02-context-handling.md (status: move-from python/tutorials/workflows/02-context-handling.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `02-context-handling.md:6` — Stale `review_date`.
  - `02-context-handling.md:643-644` — Cross-link to `/python/examples/basic-workflow/` and `/python/how-to-guides/testing-workflows/` — both target paths change after restructure.
  - No `angreal demos:tutorials:python:02` callout.
- **Effort:** S

### python/workflows/tutorials/03-complex-workflows.md (status: move-from python/tutorials/workflows/03-complex-workflows.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `03-complex-workflows.md:6` — Stale `review_date`.
  - `03-complex-workflows.md:1006-1007` — Cross-links to `/python/how-to-guides/performance-optimization/` — path change.
  - No `angreal demos:tutorials:python:03`.
- **Effort:** S

### python/workflows/tutorials/04-error-handling.md (status: move-from python/tutorials/workflows/04-error-handling.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `04-error-handling.md:6` — Stale `review_date`.
  - `04-error-handling.md:11` — Calls itself "third tutorial in our Python Cloacina series" — incorrect (it's #4). Edit.
  - `04-error-handling.md:845-846` — Cross-link to `/python/how-to-guides/testing-workflows/` — path change.
  - No `angreal demos:tutorials:python:04`.
  - `04-error-handling.md:115-119` — `retry_attempts`/`retry_delay_ms`/`retry_backoff` task params — verify against `crates/cloacina-python/src/task.rs` for current parameter names.
- **Coverage (May 2026 batch):** I-0110 indirectly (typed JSON parse/merge errors); T-0487 (cooperative cancellation — could be a callout)
- **Effort:** S

### python/workflows/tutorials/05-cron-scheduling.md (status: move-from python/tutorials/workflows/05-cron-scheduling.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `05-cron-scheduling.md:6` — `review_date: "2025-06-08"` — most recently reviewed but still 11 months old.
  - `05-cron-scheduling.md:88` — `cloaca.DefaultRunner(":memory:")` — should be `sqlite://:memory:` for SQLAlchemy-style consistency. Verify accepted forms.
  - `05-cron-scheduling.md:807-809` — Cross-link to Rust tutorial `/workflows/tutorials/service/05-cron-scheduling/` — out of our scope but flag for cross-track audit.
  - `05-cron-scheduling.md:108-117` — Cron expression `*/30 * * * * *` (6-field) — verify the runner accepts 6-field cron.
  - No `angreal demos:tutorials:python:05`.
  - `05-cron-scheduling.md:497-498` — `datetime.timedelta` is used but not imported in the snippet.
- **Coverage (May 2026 batch):** T-0608 (`:memory:`)
- **Effort:** S

### python/workflows/tutorials/06-multi-tenancy.md (status: move-from python/tutorials/workflows/06-multi-tenancy.md)
- **Status delta:** move + edit-section (I-0106 update)
- **Drift / gaps found:**
  - `06-multi-tenancy.md:6` — Stale `review_date`.
  - **I-0106 coverage gap:** The tutorial doesn't reflect fail-closed `SET search_path` semantics or the `remove_tenant` teardown orchestration. Currently shows only `create_tenant` (lines 595-626). Add a `decommission_tenant` walkthrough section to cover the I-0106 surface.
  - `06-multi-tenancy.md:1207-1208` — Cross-link to `/workflows/how-to-guides/multi-tenant-recovery/` — out of scope but ensure the target exists.
  - No `angreal demos:tutorials:python:06`.
  - `06-multi-tenancy.md:1217-1218` — Reference to `examples/multi_tenant/` and `examples/per_tenant_credentials/` — verify these exist at `examples/features/workflows/multi-tenant/` and `examples/features/workflows/per-tenant-credentials/`.
- **Coverage (May 2026 batch):** I-0106 (multi-tenant abstraction)
- **Effort:** M

### python/workflows/tutorials/07-event-triggers.md (status: move-from python/tutorials/workflows/07-event-triggers.md)
- **Status delta:** move + edit-section (cross-link refresh)
- **Drift / gaps found:**
  - `07-event-triggers.md:6` — `review_date: "2025-12-13"` — relatively current.
  - `07-event-triggers.md:528-529` — Cross-link to `/workflows/explanation/trigger-rules/` (Rust side) and `python/tutorials/workflows/05-cron-scheduling` — second link updates after restructure.
  - No `angreal demos:tutorials:python:07`.
- **Coverage (May 2026 batch):** I-0107 (could mention `list_triggers` pagination + SSE `--follow`); I-0100 (DB-backed reactor — note that triggers and reactors are distinct primitives per S-0011)
- **Effort:** S

### python/workflows/tutorials/08-packaged-triggers.md (status: move-from python/tutorials/workflows/08-packaged-triggers.md)
- **Status delta:** move + edit-section (I-0102 unified package shell coverage)
- **Drift / gaps found:**
  - `08-packaged-triggers.md` — No `review_date`/`reviewer` front-matter at all. Add.
  - `08-packaged-triggers.md:9` — Says "the reconciler (the daemon component that discovers, validates, and registers packages)" — accurate for both daemon and server.
  - **I-0102 gap:** Doesn't mention that the unified `cloacina::package!();` shell on the Rust side and the Python `entry_module` declaration both produce `.cloacina` archives — the Python-side packaging is *not* `package!()` based (it's an entry-module loader). Clarify in the docs that I-0102 is Rust-side; Python uses entry-module + manifest.
  - `08-packaged-triggers.md:91` — Cross-link to `/platform/reference/package-manifest` — verify the manifest schema is current (`format_version: "2"` per how-to packaging guide line 142).
  - No `angreal demos:tutorials:python:08`.
  - **Reproducibility gap:** No reference to `examples/features/workflows/packaged-triggers/` (Rust) or a Python equivalent. The Python-side example for tutorial 08 should live somewhere — verify in `examples/tutorials/python/workflows/08_packaged_triggers.py` (confirmed exists).
- **Coverage (May 2026 batch):** I-0102 (with the clarification above), I-0103 (could mention `--require-signatures` impact on packaged triggers)
- **Effort:** M

## python/workflows/how-to-guides

### python/workflows/how-to-guides/_index.md (status: new)
- **Category:** Index
- **Audience:** Python developer hunting for a recipe
- **Status delta:** new-write (current `python/how-to-guides/_index.md` is a 12-line placeholder with `{{< toc-tree >}}`; expand after restructure)
- **Effort:** S

### python/workflows/how-to-guides/testing-workflows.md (status: move-from python/how-to-guides/testing-workflows.md)
- **Category:** How-to
- **Audience:** Python developer writing pytest fixtures for Cloaca
- **Status delta:** move + rewrite (deprecated API usage)
- **Drift / gaps found:**
  - **API drift:** `testing-workflows.md:117-122`, `159-164`, `273-277` etc. use the deprecated `builder.add_task("task_id")` + `cloaca.register_workflow_constructor(name, constructor)` flow. The current pattern (per `python/quick-start.md` and tutorials 01-08) is the `with cloaca.WorkflowBuilder(...) as builder:` context manager that auto-registers tasks. This how-to is **out of sync with the rest of the Python docs**. Rewrite all examples to the context-manager pattern.
  - `testing-workflows.md:281` — "Should handle failure gracefully" — fine, but the pattern uses `register_workflow_constructor` which is *exported* but is being deprecated in favor of context-manager. Verify with the cloacina-python team's stance; keep documented as the back-compat path if it stays.
  - `testing-workflows.md:332` — `tracemalloc` usage example — fine but bears no Cloaca specifics; could move to an explanation doc on "How Cloaca uses memory."
- **Coverage (May 2026 batch):** T-0608 (`:memory:` substitution — partially covered; the in-memory pytest fixture is the canonical Python use case)
- **Effort:** L (full rewrite of code examples)

### python/workflows/how-to-guides/backend-selection.md (status: move-from python/how-to-guides/backend-selection.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `backend-selection.md:329-332` — Cross-links to `/python/quick-start/`, `/python/tutorials/workflows/06-multi-tenancy/`, `/python/how-to-guides/performance-optimization/`, `/python/api-reference/configuration/`. The middle two paths change after restructure.
  - No S-0011 banned phrases; no version pins drift; no API drift.
  - `backend-selection.md:65-69` — "Limitations: No multi-tenancy support" for SQLite — accurate.
  - **I-0106 gap:** No mention of fail-closed `search_path` semantics for the PostgreSQL multi-tenant configuration block.
- **Coverage (May 2026 batch):** I-0106 (light touch — fail-closed search_path); T-0608 (`:memory:` for testing fixtures — covered)
- **Effort:** S

### python/workflows/how-to-guides/performance-optimization.md (status: move-from python/how-to-guides/performance-optimization.md)
- **Status delta:** move + rewrite (deprecated API)
- **Drift / gaps found:**
  - **API drift:** `performance-optimization.md:114-129`, `492-497` use the deprecated `builder = cloaca.WorkflowBuilder(...)` + `builder.add_task(...)` + `cloaca.register_workflow_constructor(...)` flow. Same fix as `testing-workflows.md` — rewrite to context-manager pattern.
  - **I-0099 gap:** This doc is the natural home for "scrape `cloacina_*` metrics from your runner" content; currently has zero mention of the metrics catalog (or any metric name). After metrics-catalog ships, cross-link from here.
  - `performance-optimization.md:566-571` — Cross-links to `/python/how-to-guides/backend-selection/`, `/python/how-to-guides/testing-workflows/`, `/python/tutorials/workflows/06-multi-tenancy/`, `/python/api-reference/` — paths change.
  - `performance-optimization.md:309-323` — Connection-pool parameters (`pool_min_size`, `pool_max_size`, `pool_timeout`, `pool_recycle`) — verify these survive into the Python URL parser.
- **Coverage (May 2026 batch):** I-0099 (metrics catalog cross-link is the gap), I-0108 (gauge re-seed), T-0610 (Helm chart with embedded postgres)
- **Effort:** L (API rewrite + metrics-catalog integration)

### python/workflows/how-to-guides/packaging-python-workflows.md (status: move-from python/how-to-guides/packaging-python-workflows.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `packaging-python-workflows.md:158` — manifest example uses `"format_version": "2"` and `"created_at": "2026-01-15T10:30:00Z"` — verify against current manifest schema.
  - `packaging-python-workflows.md:244-249` — Cross-links to `/platform/reference/package-manifest`, `/platform/explanation/package-format`, `/platform/how-to-guides/running-the-daemon`, `/platform/how-to-guides/deploying-the-api-server` — verify all four exist on the platform side.
  - `packaging-python-workflows.md:147` — `"targets": ["linux-x86_64", "linux-arm64", "macos-x86_64", "macos-arm64"]` — match against `install.md:55-59` platform list.
  - `packaging-python-workflows.md:108-110` — Stdlib shadowing list is a security-critical surface; verify against the Python loader code in `crates/cloacina-python/src/loader.rs`.
- **Coverage (May 2026 batch):** I-0102 (entry_module-based packaging on Python side); I-0103 (could mention `--require-signatures` impact when uploading)
- **Effort:** S

### python/workflows/how-to-guides/decommission-a-tenant.md (status: new, stub)
- **Category:** How-to
- **Audience:** Python developer running a multi-tenant SaaS who needs to clean up a tenant
- **Status delta:** stub-marker
- **Drift / gaps found:** Gap — I-0106 added `remove_tenant` orchestration with drain timeout; no Python how-to covers it.
- **Coverage (May 2026 batch):** I-0106
- **Effort:** M (when filled)

## python/workflows/reference

### python/workflows/reference/_index.md (status: stub)
- **Category:** Index
- **Status delta:** stub-marker (no current Python reference directory exists outside api-reference; this quadrant slot is empty)
- **Effort:** S

### python/workflows/reference/environment-variables.md (status: new, stub)
- **Category:** Reference
- **Audience:** Python developer / operator looking up `CLOACINA_VAR_*` and `CLOACA_*` env vars
- **Status delta:** stub-marker
- **Coverage (May 2026 batch):** T-0529 / T-0532 (standalone Python crate has its own env-var surface)
- **Effort:** M (when filled)

## python/workflows/explanation

### python/workflows/explanation/_index.md (status: stub)
- **Status delta:** stub-marker
- **Effort:** S

### python/workflows/explanation/python-runtime-architecture.md (status: new, stub)
- **Category:** Explanation
- **Audience:** Python developer who wants to understand the PyO3 boundary, GIL handling, and cloacina-python crate split
- **Status delta:** stub-marker
- **Coverage (May 2026 batch):** T-0529, T-0532
- **Effort:** M (when filled)

## python/computation-graphs/tutorials

### python/computation-graphs/tutorials/_index.md (status: new)
- **Category:** Index
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0101 (CG split — landing page should explain reactor-first model in one paragraph), I-0102 (Python packaging path)
- **Effort:** S

### python/computation-graphs/tutorials/09-computation-graph.md (status: move-from python/tutorials/computation-graphs/09-computation-graph.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `09-computation-graph.md:18` — "Python 3.8+" — wheels are abi3-py39 per `troubleshooting.md:828`. Update to "Python 3.9+".
  - `09-computation-graph.md:7` — Cross-link to `/computation-graphs/tutorials/library/07-computation-graph/` — verify Rust path post-IA cleanup.
  - `09-computation-graph.md:39-48` — Uses `@cloaca.reactor` first-class class (correct post-I-0101 split). The doc explicitly notes "CLOACI-I-0101 split — the bundled `react={...}` kwarg was removed." Good — already on-spec.
  - `09-computation-graph.md:185` — Cross-link to tutorial 10.
  - No `angreal demos:tutorials:python:09`.
- **Coverage (May 2026 batch):** I-0101 (correctly reflects the split); I-0102 (no `package!()` macro since this is library mode)
- **Effort:** S

### python/computation-graphs/tutorials/10-accumulators.md (status: move-from python/tutorials/computation-graphs/10-accumulators.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `10-accumulators.md:63-69` — `@cloaca.reactor` correctly used; explicitly references I-0101 split. Good.
  - `10-accumulators.md:206` — Cross-link to tutorial 11.
  - No `angreal demos:tutorials:python:10`.
  - Should add a callout for the four accumulator types (passthrough, stream, polling, batch) per `python/api-reference/computation-graphs.md:269-422` — tutorial only shows passthrough.
- **Coverage (May 2026 batch):** I-0099 (accumulator metrics — `cloacina_accumulator_*` family); I-0101 (CG split)
- **Effort:** S

### python/computation-graphs/tutorials/11-routing.md (status: move-from python/tutorials/computation-graphs/11-routing.md)
- **Status delta:** move + edit-section
- **Drift / gaps found:**
  - `11-routing.md:7` — Cross-link to `/computation-graphs/tutorials/library/10-routing/` — Rust path, verify.
  - `11-routing.md:44-50` — `@cloaca.reactor` correctly used.
  - `11-routing.md:234` — Cross-link to Rust tutorial 10 (out of scope).
  - No `angreal demos:tutorials:python:11`.
- **Coverage (May 2026 batch):** I-0101 (CG split)
- **Effort:** S

## python/computation-graphs/how-to-guides

### python/computation-graphs/how-to-guides/_index.md (status: new, stub)
- **Status delta:** stub-marker
- **Effort:** S

### python/computation-graphs/how-to-guides/package-a-python-computation-graph.md (status: new, stub)
- **Category:** How-to
- **Audience:** Python developer packaging a CG (reactor + accumulators + graph) for daemon/server deployment
- **Status delta:** stub-marker
- **Coverage (May 2026 batch):** I-0101, I-0102
- **Effort:** M (when filled)

### python/computation-graphs/how-to-guides/filter-reactor-subscriptions.md (status: new, stub)
- **Category:** How-to
- **Audience:** Python developer who wants to filter which boundary events fire a graph
- **Status delta:** stub-marker
- **Coverage (May 2026 batch):** T-0602
- **Effort:** M (when filled; depends on Python CEL filter availability)

## python/computation-graphs/reference

### python/computation-graphs/reference/_index.md (status: stub)
- **Status delta:** stub-marker
- **Effort:** S

### python/computation-graphs/reference/topology-dict-schema.md (status: new, stub)
- **Status delta:** stub-marker
- **Effort:** S (when filled)

## python/computation-graphs/explanation

### python/computation-graphs/explanation/_index.md (status: stub)
- **Status delta:** stub-marker
- **Effort:** S

### python/computation-graphs/explanation/python-cg-decorator-surface.md (status: new, stub)
- **Status delta:** stub-marker
- **Coverage (May 2026 batch):** I-0101 (CG split — decoupled reactor / CG); I-0102 (Python packaging surface)
- **Effort:** M (when filled)

## python/api-reference

(All entries here are verify-only — auto-generated from pdoc per initiative non-goals — but prose around them needs the same drift sweep.)

### python/api-reference/_index.md (status: verify-no-changes)
- **Effort:** S (zero edit; verify the toc-tree picks up renamed files)

### python/api-reference/computation-graphs.md (status: edit-section)
- **Status delta:** edit-section (cross-links + small drift)
- **Drift / gaps found:**
  - `computation-graphs.md:17` — "As of CLOACI-I-0101 the firing criterion is its own first-class primitive" — correct post-I-0101.
  - `computation-graphs.md:451` — Cross-link to `/computation-graphs/tutorials/service/07-packaging/` — verify Rust path post-IA cleanup.
  - `computation-graphs.md:657-659` — Cross-links to `/python/tutorials/computation-graphs/`, `/python/api-reference/workflow-builder/`, `/python/api-reference/runner/` — first path changes after restructure.
- **Effort:** S

### python/api-reference/configuration.md (status: edit-section)
- **Status delta:** edit-section (API drift verification + env-var consolidation)
- **Drift / gaps found:**
  - **Drift with runner.md:** Config field names disagree (`max_concurrent_workflows` vs `max_concurrent_tasks`; `connection_pool_size` vs `db_pool_size`). Reconcile.
  - **Drift with task.md:** `retry_policy` dict-style vs flat retry kwargs. Reconcile.
  - `configuration.md:224-235` — Env-var list (`CLOACA_DATABASE_URL`, `CLOACA_LOG_LEVEL`, `CLOACA_MAX_WORKERS`, etc.). Verify each is actually read by the runtime — these look aspirational.
  - `configuration.md:158-160` — `max_missed_runs=3`, `catch_up=False` for CronSchedule — verify.
- **Effort:** M (drift reconciliation between two reference files)

### python/api-reference/context.md (status: verify-no-changes)
- **Effort:** S (sweep only)

### python/api-reference/database-admin.md (status: edit-section)
- **Status delta:** edit-section (I-0106 update)
- **Drift / gaps found:**
  - **I-0106 gap:** Only `create_tenant` is documented. The I-0106 work added `remove_tenant` with drain timeout — needs documentation if exposed in `crates/cloacina-python/src/bindings/admin.rs`.
  - `database-admin.md:219-223` — Cross-links: last path changes after restructure.
- **Effort:** M

### python/api-reference/exceptions.md (status: edit-section)
- **Status delta:** edit-section (verify exception surface)
- **Drift / gaps found:**
  - **API drift risk:** `exceptions.md:13-27` lists a clean hierarchy: `CloacaException`, `WorkflowError`, `WorkflowExecutionError`, `WorkflowTimeoutError`, `TaskError`, `TaskValidationError`, `TaskExecutionError`, `TaskTimeoutError`, `ContextError`, `ConfigurationError`, `DatabaseError`, `ConnectionError`, `MigrationError`. Verify against `crates/cloacina-python/src/` — only `PyValueError` and `PyKeyError` are mentioned in bindings; the explicit `CloacaException` hierarchy may not exist in code. If it doesn't, this reference is aspirational and needs to be either implemented or rewritten to match the actual exception surface (probably `ValueError`/`KeyError`/`RuntimeError`).
- **Coverage (May 2026 batch):** I-0110 (typed JSON parse/merge errors — should surface as either ContextError or DatabaseError)
- **Effort:** M (verification-heavy)

### python/api-reference/pipeline-result.md (status: edit-section)
- **Status delta:** edit-section (API drift)
- **Drift / gaps found:**
  - **API drift:** `pipeline-result.md:155` uses `cloaca.register_workflow_constructor` — deprecated pattern.
  - `pipeline-result.md:14-22` — Properties: `status`, `workflow_name`, `execution_id`, `final_context`, `start_time`, `end_time`, `duration`. Verify against `crates/cloacina-python/src/bindings/runner.rs` `PyWorkflowResult` struct.
  - **Drift with runner.md:** `runner.md:107-113` shows `result.status`, `result.final_context`, `result.error_message`. `pipeline-result.md` doesn't mention `error_message` as a property. Reconcile.
- **Effort:** S

### python/api-reference/runner.md (status: edit-section)
- **Status delta:** edit-section (config drift + I-0107 list pagination)
- **Drift / gaps found:**
  - **Drift with configuration.md:** Config field names disagree. Pick one source of truth.
  - **I-0107 gap:** `list_trigger_schedules` accepts `limit`/`offset` (lines 305-308) and `list_cron_schedules` likewise (lines 167-170) — good, consistent with I-0107 pagination work. Verify no other endpoints expose pagination but aren't documented.
- **Coverage (May 2026 batch):** I-0107 (pagination on list endpoints — covered); T-0529 / T-0532 (standalone Python runtime usage — runner doc is the canonical entrypoint)
- **Effort:** M

### python/api-reference/task.md (status: edit-section)
- **Status delta:** edit-section (drift between this and configuration.md retry config)
- **Drift / gaps found:**
  - **Drift with configuration.md:** This file documents flat retry kwargs at lines 31-38. `configuration.md:107-115` documents a nested `retry_policy={...}` dict. Pick one and reconcile.
  - `task.md:349` — Cross-link to `/workflows/tutorials/service/10-task-deferral` — Rust tutorial path, verify post-IA.
  - `task.md:246-310` — TaskHandle / `defer_until` — verify against `crates/cloacina-python/src/task.rs` `PyTaskHandle`.
- **Coverage (May 2026 batch):** T-0487 (cooperative cancellation); T-0502 (heartbeat-only recovery)
- **Effort:** M

### python/api-reference/trigger.md (status: edit-section)
- **Status delta:** edit-section (cross-link)
- **Drift / gaps found:**
  - `trigger.md:264` — Cross-link to `/workflows/tutorials/service/09-event-triggers/` (Rust) — verify path post-IA.
- **Effort:** S

### python/api-reference/workflow.md (status: edit-section)
- **Status delta:** edit-section (API drift)
- **Drift / gaps found:**
  - **API drift:** `workflow.md:67-70` uses `cloaca.register_workflow_constructor("my_workflow", lambda: workflow)` — deprecated pattern.
  - `workflow.md:42` — `builder.build()` — verify whether the modern Python API exposes a `.build()` method or just relies on context-manager `__exit__`.
- **Coverage (May 2026 batch):** I-0110 (final_context deterministic tiebreaker)
- **Effort:** S

### python/api-reference/workflow-builder.md (status: rewrite)
- **Status delta:** rewrite (API drift — extensive)
- **Drift / gaps found:**
  - **API drift:** Lines 88-103, 121, 245-248, 294-296, 549-559 — the entire file uses the deprecated `WorkflowBuilder(...).add_task(...).build()` pattern and `register_workflow_constructor(...)`. Tutorials and the test suite use the context-manager pattern. This reference is **the canonical out-of-sync document** in the Python tree. Either re-orient as "back-compat reference for the imperative builder API (deprecated)" or rewrite around the context-manager pattern with the imperative pattern documented as legacy.
  - `workflow-builder.md:374-378` — `workflow.get_roots()`, `workflow.get_leaves()`, `workflow.get_execution_levels()` — verify these methods exist on the Python `PyWorkflow` class.
  - `workflow-builder.md:406` — `workflow.topological_sort()`, `workflow.can_run_parallel(a, b)` — likely aspirational.
- **Coverage (May 2026 batch):** T-0529 / T-0532
- **Effort:** L

## Contributing

### docs/content/contributing/_index.md (status: edit-section)
- **Category:** Explanation / How-to mix (Diataxis-borderline — leans how-to)
- **Audience:** External contributor proposing a change
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `contributing/_index.md:56-58` — "Adding a Prometheus metric? Also update `docs/operations/metrics.md`" — this path is being deleted/folded into `docs/content/platform/reference/metrics-catalog.md`. Update the instruction.
  - `contributing/_index.md:6` — `review_date: "2024-03-19"` is ~2 years old — front-matter is stale; whole doc may need refresh.
  - `contributing/_index.md:17-25` — `pip install angreal` — verify against current angreal install path; mention `flox` / `cargo-make` if those are also supported.
  - No mention of Metis as the planning system. Contributors who don't know about Metis won't find it.
  - No mention of S-0011 nomenclature compliance — contributors writing docs need to know "reactive" is banned in primitive contexts.
- **Coverage (May 2026 batch):** I-0099 (metrics catalog rename)
- **Effort:** S

### docs/content/contributing/documentation.md (status: edit-section)
- **Category:** How-to
- **Audience:** Contributor adding or updating Cloacina docs
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `documentation.md:14-18` — "Our documentation follows the [Diátaxis Framework](https://diataxis.fr/)" and lists directories as `docs/content/tutorials/`, `docs/content/how-to/`, `docs/content/reference/`, `docs/content/explanation/`. **This is wrong** — the actual structure splits by feature area first (`workflows/`, `computation-graphs/`, `platform/`, `python/`) then by Diataxis quadrant. Either rewrite to reflect the actual IA or update the IA.
  - `documentation.md:6` — `review_date: "2024-03-19"` — same stale front-matter.
  - `documentation.md:31-61` — `api-link` shortcode usage — verify shortcode still exists at `docs/layouts/shortcodes/api-link.html`.
  - `documentation.md:79-80` — `hugo server -D` instruction — verify against current Hugo config / `angreal docs serve` task; the project uses angreal not raw hugo per the user-memory feedback.
  - No mention of the cross-link policy (tutorials → reference, how-to → explanation, etc.) that initiative locks in.
- **Effort:** M

## Operations fold-in

### docs/content/platform/how-to-guides/compiler-deployment-runbook.md (status: new, fold-in-from docs/operations/compiler-deployment.md)
- **Category:** How-to
- **Audience:** Platform operator deploying the compiler + server pair
- **Status delta:** new-write (port + extend source content)
- **Drift / gaps found in source (`docs/operations/compiler-deployment.md`):**
  - `compiler-deployment.md:75` — "Image tags pin to `:latest` until the T-0501 release pipeline starts publishing versioned images" — T-0610 already published `ghcr.io/colliery-software/cloacina-server` Docker images and embedded the Postgres subchart. Update to remove the "until T-0501" caveat and document the concrete image path.
  - `compiler-deployment.md:88` — "Not a Helm chart — that ships with T-0501." — Now ships per I-0111. Update to point at the Helm chart.
  - `compiler-deployment.md:111, 127` — Image refs use `ghcr.io/colliery-io/cloacina-server` and `ghcr.io/colliery-io/cloacina-compiler` — verify the actual repo path is `colliery-io` or `colliery-software` (README is inconsistent).
  - `compiler-deployment.md:155-179` — "Playbook: a build is stuck" — verify that `cloacinactl package inspect <id>`, `cloacinactl compiler status`, `cloacinactl compiler health` are all present in the post-I-0098/T-0538 CLI.
  - `compiler-deployment.md:181-187` — References to ADR-0004, CLOACI-I-0097, CLOACI-S-0010 — verify these still match the canonical ADR/spec IDs (initiative says `CLOACI-A-0004` is the compiler service ADR).
- **Coverage (May 2026 batch):** I-0097, I-0098 (cloacinactl redesign), I-0104 (compiler hardening Phase 1 — should add hardening-flag callouts), I-0109 (`/metrics` endpoint, `--log-retention-days`), I-0111 (Docker image, Helm chart), T-0610 (embedded postgres subchart)
- **Effort:** L (port + extend with May 2026 batch)

### docs/content/platform/reference/metrics-catalog.md (status: new, fold-in-from docs/operations/metrics.md)
- **Category:** Reference
- **Audience:** Operator wiring Prometheus / alerting against Cloacina
- **Status delta:** new-write (port + extend source content)
- **Drift / gaps found in source (`docs/operations/metrics.md`):**
  - `metrics.md:202` — "`/v1/health/reactors` and `/v1/health/accumulators`" — `/v1/health/reactors` is on the S-0011 banned list (initiative); rename to `/v1/health/graphs`. **NOM-OPS-01.**
  - `metrics.md:8-12` — "ADR CLOACI-A-0005" reference — verify path; the path uses `../../.metis/adrs/` which is a repo-relative link that won't render under Hugo.
  - `metrics.md:196-201` — "Computation-graph observability for the full reactive stack" — uses "reactive stack" — **banned phrase per S-0011.** **NOM-OPS-02.**
  - `metrics.md:209` — Reference to `CLOACI-T-0498` — confirm task ID is correct.
  - `metrics.md:217-219` — "see T-0536 (server) + T-0591 (compiler)" — verify task IDs.
  - Otherwise the metric table itself is dense and current — covers all the I-0099/I-0108/I-0109 metrics with labels, descriptions, example PromQL.
- **Coverage (May 2026 batch):** I-0099 (CG observability — central doc), I-0108 (SQL-derived `cloacina_active_tasks` gauge), I-0109 (compiler /metrics)
- **Effort:** L (port + S-0011 fixes + cross-link wiring)

### docs/content/troubleshooting.md `### Native crash troubleshooting` section (status: edit-section, fold-in-from docs/SIGSEGV_TROUBLESHOOTING.md)
- **Category:** Reference (subsection)
- **Audience:** Developer or CI debugger hitting SIGSEGV on Python tests / import
- **Status delta:** edit-section (merge `docs/SIGSEGV_TROUBLESHOOTING.md` into the existing section 19 of troubleshooting.md)
- **Drift / gaps found in source (`docs/SIGSEGV_TROUBLESHOOTING.md`):**
  - Source is already framed as historical (lines 1-9 say "Historical document. The `#[ctor]`-based OpenSSL early-init workaround described below was removed during the I-0096 ctor → inventory flip"). Good — keep that framing.
  - `SIGSEGV_TROUBLESHOOTING.md:34-66` — Alternative approaches list is useful; merge in.
  - `SIGSEGV_TROUBLESHOOTING.md:67-71` — Debugging tips section overlaps with current `troubleshooting.md:877-880`. De-duplicate.
  - The current `troubleshooting.md:862-872` already mentions the historical pattern and links out to the source file via GitHub URL. After merge, replace the external link with the subsection anchor.
- **Coverage (May 2026 batch):** I-0096 (ctor → inventory flip, already documented as the reason the workaround was removed); T-0529 / T-0532 (cloacina-python crate split — the SIGSEGV failure mode now lives in the cloacina-python crate)
- **Effort:** M (port + dedupe with existing section 19)

## README.md

### /Users/dstorey/Desktop/cloacina/README.md (status: edit-section)
- **Category:** N/A (repo README — not part of Hugo site but cross-references the published docs)
- **Audience:** GitHub visitor / new contributor
- **Status delta:** edit-section (drift)
- **Drift / gaps found:**
  - `README.md:147` — "Compiler + Server Deployment Runbook" link points at `docs/operations/compiler-deployment.md`. After the fold-in, this file no longer exists. Update to point at `docs/content/platform/how-to-guides/compiler-deployment-runbook.md` or its published URL.
  - `README.md:8` — Logo URL: `https://github.com/colliery-io/cloacina/raw/main/docs/static/images/image.png` uses `colliery-io`. `install.md:20` uses `https://github.com/colliery-software/cloacina/releases`. **Org-slug drift.**
  - `README.md:34, 47, 50` — Cargo install snippets pin to `cloacina = "0.6.1"` — current. Good.
  - `README.md:127-141` — Repository structure block omits new crates from the May 2026 batch: `cloacina-build`, `cloacina-compiler`, `cloacina-python`, `cloacina-server`, `cloacina-workflow`, `cloacina-workflow-plugin`.
  - `README.md:133` — `bindings/cloaca-backend/` is listed but doesn't exist at that path (verified: no `bindings/` directory at the repo root). The actual location is `crates/cloacina-python/`. **Structural drift.**
  - `README.md:73-77` — Quick-start Rust snippet uses `workflow!` macro and `DefaultRunner::new` — verify current API.
  - `README.md:145` — Docs URL `https://colliery-io.github.io/cloacina/` — verify the GitHub Pages site is at this URL post-rename.
  - **Gap:** No mention of `cloacinactl` (the install script + CLI from I-0111) anywhere in the README.
  - **Gap:** No mention of Python — `README.md:13` notes Cloaca exists but the "Installation" section is Rust-only.
- **Coverage (May 2026 batch):** I-0111 (install script + Docker + Helm — README should call out), T-0529 / T-0532 (cloacina-python crate — repository structure needs update), I-0102 (Unified `package!()` macro — Rust quick-start snippet could showcase)
- **Effort:** M

---

## Summary

- **Total files reviewed:** 33 in scope
  - Python (28): `python/_index.md`, `python/quick-start.md`, 8 workflow tutorials, 3 CG tutorials, 4 how-to-guides, 2 examples files (folded), 12 api-reference files, 3 tutorials/_index.md files
  - Quick-start (2): `_index.md`, `install.md`
  - Top-level (3): `_index.md`, `glossary.md`, `troubleshooting.md`
  - Contributing (2): `_index.md`, `documentation.md`
  - Operations source files audited for fold-in (3): `compiler-deployment.md`, `metrics.md`, `SIGSEGV_TROUBLESHOOTING.md`
  - Repo README (1)
- **Drift findings:**
  - **NOM-X-NN (S-0011 banned phrases) found in scope:** 4
    - NOM-PY-01 (`python/_index.md:21` "reactive graphs")
    - NOM-OPS-01 (`docs/operations/metrics.md:202` "/v1/health/reactors" — in source destined for fold-in)
    - NOM-OPS-02 (`docs/operations/metrics.md:196` "reactive stack")
    - Plus one in the glossary at `glossary.md:39` ("reactive, streaming computation model")
  - **VER-X-NN (stale version pins) in scope:** 3
    - VER-Q-01 (`quick-start/install.md:28` `--version v0.6.0`)
    - VER-T-01 (`troubleshooting.md:451` `cloacina-computation-graph = { version = "0.4" }`)
    - VER-T-02 (`troubleshooting.md:457` `cloacina = { version = "0.4", features = ["macros"] }`)
  - **IA-X-NN (information-architecture issues) in scope:** 4
    - IA-Q-01 (`quick-start/_index.md:31` Rust tutorial path drift — flag only)
    - IA-PY-01 (Python tree restructure — central to this initiative; produces ~21 MOVE entries)
    - IA-PY-02 (`python/examples/` is a Diataxis-violation directory — needs deletion after folding basic-workflow into tutorials)
    - IA-PY-03 (`docs/operations/` orphan directory — folds into `platform/` + `troubleshooting`, then deletes)
  - **API drift (code surface ↔ docs):** 5+
    - `python/api-reference/configuration.md` vs `python/api-reference/runner.md` (config field names disagree)
    - `python/api-reference/configuration.md` vs `python/api-reference/task.md` (retry config representation disagrees)
    - `python/api-reference/exceptions.md` claims an exception hierarchy that may not exist in code
    - `python/api-reference/workflow-builder.md` extensively uses deprecated `register_workflow_constructor` + imperative builder pattern; tutorials use context-manager
    - `python/how-to-guides/testing-workflows.md` + `performance-optimization.md` ditto
    - `troubleshooting.md:891` claims `cloaca.features()` exists; not in the verified pymodule
- **New docs proposed:** 11 stubs (plus the 3 operations fold-ins)
  - `python/workflows/how-to-guides/decommission-a-tenant.md` (new, stub)
  - `python/workflows/reference/_index.md` (new, stub)
  - `python/workflows/reference/environment-variables.md` (new, stub)
  - `python/workflows/explanation/_index.md` (new, stub)
  - `python/workflows/explanation/python-runtime-architecture.md` (new, stub)
  - `python/computation-graphs/how-to-guides/_index.md` (new, stub)
  - `python/computation-graphs/how-to-guides/package-a-python-computation-graph.md` (new, stub)
  - `python/computation-graphs/how-to-guides/filter-reactor-subscriptions.md` (new, stub — pending Python CEL availability)
  - `python/computation-graphs/reference/_index.md` (new, stub)
  - `python/computation-graphs/reference/topology-dict-schema.md` (new, stub)
  - `python/computation-graphs/explanation/_index.md` (new, stub)
  - `python/computation-graphs/explanation/python-cg-decorator-surface.md` (new, stub)
  - `python/workflows/tutorials/_index.md` (new — replaces the placeholder)
  - `python/computation-graphs/tutorials/_index.md` (new — replaces the placeholder)
  - Plus the 3 fold-ins: `platform/how-to-guides/compiler-deployment-runbook.md`, `platform/reference/metrics-catalog.md`, the SIGSEGV merge subsection inside `troubleshooting.md`
- **Move count:** 15 file moves
  - 11 tutorials (8 workflow + 3 CG) under `python/`
  - 4 how-to-guides under `python/`
  - Plus 1 fold-in move (`python/examples/basic-workflow.md` → either `python/workflows/tutorials/00-basic-workflow.md` or a how-to)
- **Deletions:** 4 directories after restructure: `python/tutorials/`, `python/how-to-guides/`, `python/examples/`, `docs/operations/`; plus 1 root file `docs/SIGSEGV_TROUBLESHOOTING.md`
- **Cross-cutting work spanning multiple docs:** S-0011 banned-phrase sweep (4 occurrences in our scope); version pin sweep (3 occurrences); deprecated-API rewrite (4 docs need context-manager pattern); org-slug audit (`colliery-io` vs `colliery-software`); cross-link updates after every Python file moves (~50 cross-links across the moved files).
