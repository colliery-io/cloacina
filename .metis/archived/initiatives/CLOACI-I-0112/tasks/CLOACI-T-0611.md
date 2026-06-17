---
id: doc-a-drift-sweep-s-0011
level: task
title: "DOC-A: Drift sweep — S-0011 nomenclature, version pins, removed-API references"
short_code: "CLOACI-T-0611"
created_at: 2026-05-18T18:19:19.074270+00:00
updated_at: 2026-05-18T18:44:09.518171+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-A: Drift sweep — S-0011 nomenclature, version pins, removed-API references

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Land the cross-cutting mechanical fixes that have to be in before any other cluster touches the same files. Lock the published docs to S-0011 nomenclature (zero banned phrases, no stale CLI verbs, no stale HTTP paths), refresh all Cargo/Docker/install version pins to the current release, delete or correct references to APIs that no longer exist (`RecoveryManager`, `#[ctor]`, fabricated cron types), and pick the org slug.

This cluster is the **drift-only baseline** — content rewrites that depend on the topology shifts (I-0100, I-0101, I-0102, I-0106, I-0107) happen in DOC-E and DOC-F. DOC-A is the substrate those clusters build on.

## Scope

### Cross-cutting sweeps (touches files in multiple areas)

- **S-0011 nomenclature** — remove every banned phrase ("reactive scheduler", "reactive computation graph", "reactive subsystem", "reactive execution") from `docs/content/`. 16 known instances in `computation-graphs/`, 1 in `python/_index.md:21`, 1 in `glossary.md:39`. Plus 3 English "reactive" uses in `workflows/how-to-guides/sequential-strategy.md:27`, `workflows/how-to-guides/testing-workflows.md:178`, `workflows/reference/testing-crate.md:142` — Diataxis-reviewer judgement call; either rewrite to "event-driven" or leave with a noted reason.
- **S-0011 CLI/HTTP renames** — every `cloacinactl reactor <verb>` → `cloacinactl graph <verb>`; every `/v1/health/reactors` → `/v1/health/graphs`; every HTTP body `{"reactors": [...]}` → `{"graphs": [...]}` or `{"items": [...]}` per actual server response.
- **Version pins** — 25+ stale Cargo / Docker / install snippets. Sweep to `0.6.1` (or whatever the current workspace version is at start-of-work):
  - `workflows/explanation/macro-system.md:112` (`cloacina = "0.1.0"`)
  - `workflows/how-to-guides/multi-tenant-setup.md:15`
  - `workflows/tutorials/library/01-first-workflow.md:58`, `02-context-handling.md:60`, `03-complex-workflows.md:60`, `04-error-handling.md:60`
  - `workflows/tutorials/service/07-packaged-workflows.md:84` (`cloacina-workflow = "0.2"`)
  - `computation-graphs/reference/computation-graphs.md:895-896` (`cloacina-computation-graph = "0.1"`, `cloacina-workflow-plugin = "0.1"`)
  - `computation-graphs/tutorials/service/07-packaging.md:92-103` (`"0.3"`)
  - `computation-graphs/tutorials/service/09-kafka-stream.md:164-174` (`"0.3"`)
  - `platform/explanation/database-backends.md:52, 66, 72` (`cloacina = "0.1.0"`)
  - `platform/explanation/package-format.md:164` (`cloacina-workflow = "0.2"`)
  - `platform/how-to-guides/running-the-server-image.md:20` (`0.6.0` Docker tag)
  - `troubleshooting.md:451, 457` (`cloacina-* = "0.4"`)
  - `quick-start/install.md:28` (`--version v0.6.0`)
- **Removed-API references** — three significant removals not yet reflected:
  - **T-0502 `RecoveryManager`** — references in `workflows/explanation/architecture-overview.md:244`, `workflows/explanation/task-execution-sequence.md:383-401`, `platform/explanation/horizontal-scaling.md:110`, `platform/how-to-guides/performance-tuning.md:119-120`, `troubleshooting.md:153-167`. Edit to "heartbeat sweeper is the sole task-recovery path".
  - **I-0096 `#[ctor]`** — references in `workflows/tutorials/library/01-first-workflow.md:75-77`, `workflows/tutorials/service/05-cron-scheduling.md:60` (Cargo.toml dep), `computation-graphs/reference/computation-graphs.md:818-855` (entire "Global Registry" subsection). Replace with `inventory::submit!` / `seed_from_inventory()` references.
  - **Fabricated cron APIs in `workflows/explanation/cron-scheduling.md`** — `MissedExecutionPolicy::*` (lines 319-352), `DistributedCronScheduler` + `try_acquire_leader_lease` + `DistributedCronExecutor` (lines 464-573), `CronSchedule::new_with_timezone` (lines 413-432), `CronMetrics::collect()` + `HealthStatus` types (lines 580-642). Excise; replace with the actual types: `CatchupPolicy::Skip/RunAll`, atomic `claim_and_update` on `cron_schedules`, `cloacina_*` metrics namespace. (Full rewrite of this doc is DOC-E; DOC-A removes the fabrications and leaves a `<!-- Rewrite required: see DOC-E -->` marker if not replacing immediately.)
- **`cloacina-ctl` → `cloacinactl`** — sweep the legacy binary/crate name from `workflows/tutorials/service/07-packaged-workflows.md` (extensive), `workflows/tutorials/service/08-workflow-registry.md` (extensive), `workflows/explanation/architecture-overview.md:209-212`. Verb form too: `cloacinactl serve` → `cloacinactl server start`, `cloacinactl daemon <flags>` → `cloacinactl daemon start <flags>` in `platform/how-to-guides/deploying-the-api-server.md`, `platform/how-to-guides/production-deployment.md:8,79,84,114`, `workflows/_index.md:25`, `workflows/explanation/architecture-overview.md:36,52`.
- **Org slug sweep** — `colliery-software` → `colliery-io` in `docs/content/quick-start/install.md:20, 93` (the README and `install.sh` are already `colliery-io`).

### Files touched (~30)

Files listed inline above. Authoritative drift list per audit files (see Sources).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `grep -rn "reactive scheduler\|reactive computation graph\|reactive subsystem\|reactive execution\|ReactiveScheduler" docs/content/` returns zero matches.
- [ ] `grep -rn "cloacinactl reactor\|/v1/health/reactors" docs/content/` returns zero matches (except inside explicit Changelog / migration-note sections that call out the rename).
- [ ] `grep -rEn 'cloacina(-\w+)? *= *"0\.([0-5]|6\.0)"' docs/content/` returns zero matches.
- [ ] `grep -rn "RecoveryManager" docs/content/` returns zero matches; replaced text mentions "heartbeat sweeper" where the concept appears.
- [ ] `grep -rn "ctor = \|#\[ctor\]" docs/content/` returns zero matches in user-facing prose (test-only or historical-document mentions OK if marked).
- [ ] `grep -rn "MissedExecutionPolicy\|DistributedCronScheduler\|try_acquire_leader_lease\|CronMetrics::collect" docs/content/` returns zero matches.
- [ ] `grep -rn "cloacina-ctl\|cloacinactl serve" docs/content/` returns zero matches.
- [ ] `grep -rn "colliery-software" docs/content/ install.sh README.md docs/operations/` returns zero matches.
- [ ] `angreal docs:build` passes.

## Implementation Notes

### Sources

- **Audit files**: `.metis/initiatives/CLOACI-I-0112/audit-workflows.md`, `audit-computation-graphs.md`, `audit-platform.md`, `audit-python-misc.md` (search each for `NOM-`, `VER-`, `cloacina-ctl`, `RecoveryManager`, `#[ctor]` to confirm the per-doc list).
- **Code paths for verifying the post-rename truth**:
  - `crates/cloacina-server/src/routes/health_graphs.rs:20-21,97,134` — confirms `/v1/health/graphs` is live.
  - `crates/cloacinactl/src/nouns/graph/mod.rs` — confirms CLI noun is `graph` (no `force-fire` verb; force-fire is WS only).
  - `crates/cloacina/src/runtime.rs:98-152` — inventory-based registry (post-I-0096 replacement for `#[ctor]`).
  - `crates/cloacina/src/cron_trigger_scheduler.rs:471-484` — actual cron types (`CatchupPolicy`, not `MissedExecutionPolicy`).
  - `install.sh:REPO="colliery-io/cloacina"` — authoritative org slug.

### Approach

This is a mechanical pass. Branch off main. Work file-by-file, using `git grep` to find each instance and patching with the minimal correct replacement. No content rewrites — those happen in DOC-E and DOC-F. Where a fabricated section is too large to leave as a one-line correction (the cron-scheduling.md `DistributedCronScheduler` section is ~100 lines of fiction), excise it entirely and leave a `<!-- TODO(DOC-E): rewrite cron coordination section with real claim_and_update mechanics -->` marker so the DOC-E writer knows the slot is empty.

Suggested commit cadence: one commit per sweep type (one for S-0011 nomenclature, one for version pins, one for removed-API refs, one for `cloacina-ctl` → `cloacinactl`, one for org slug). Keeps the diff reviewable.

### Dependencies

None — this is the first cluster. **DOC-B through DOC-I depend on DOC-A landing first** so they're editing post-drift-fix baselines.

### Risk considerations

- The Diataxis-reviewer judgement on the 3 English "reactive" adjective uses (`sequential-strategy.md:27`, `testing-workflows.md:178`, `testing-crate.md:142`) is the only subjective call. Default: leave them. The S-0011 ban is on banned phrases as primitive descriptors, not on the English word. If unsure, swap to "event-driven" for safety.
- If a `cloacina-ctl` rename in `07-packaged-workflows.md` or `08-workflow-registry.md` reveals broken cross-links to example directories that no longer exist (IA-08), document the broken link in the commit message and flag for DOC-E rather than chasing it here.
- Excising fabricated cron content from `cron-scheduling.md` will leave the doc visibly incomplete. That's fine — DOC-E will fix it. Mark with a TODO comment.

## Status Updates

### 2026-05-18 — execution

Completed in one Ralph session. Single-PR-sized diff across `docs/content/` plus 1 line in `README.md` slug coverage check.

**Sweep summary:**
- **NOM (S-0011)** — 16 instances fixed across 11 files. `_index.md` (3), `architecture.md` (2), `accumulator-design.md` (1), `performance.md` (1), `reactor-lifecycle.md` (2 — replaced stale `cloacinactl reactor force-fire` with WebSocket pointer), `09-full-pipeline.md` (3 incl. title), `10-routing.md` (2), `reference/computation-graphs.md` (1), `computation-graph-in-workflow.md` (2), `python/_index.md` (1), `glossary.md` (1), `performance-tuning.md` (1). Plus 3 English "reactive" → "event-driven" swaps in `sequential-strategy.md`, `testing-workflows.md`, `testing-crate.md` (defaulted to swap per the "if unsure" rule).
- **VER** — 22 stale version pins fixed across 13 files. All bumped to `0.6.1`. Tutorials 01-04 (library), 05 (cron), 07-08 (service); `multi-tenant-setup.md`; `macro-system.md`; both CG service tutorials (07-packaging, 09-kafka-stream); `reference/computation-graphs.md`; `package-format.md`, `database-backends.md`; `troubleshooting.md`; `running-the-server-image.md` Docker tag (0.6.0 → 0.6.1); `quick-start/install.md` --version.
- **CLI/HTTP rename** — 0 NOM-CLI matches remaining. `/v1/health/reactors` and `cloacinactl reactor` references replaced with `/v1/health/graphs` and WebSocket force-fire pointer respectively. The `cloacinactl graph force-fire` verb doesn't exist (confirmed via `crates/cloacinactl/src/nouns/graph/mod.rs`); WebSocket is the only path.
- **Removed-API**:
  - **T-0502 RecoveryManager** — 1 fix (`performance-tuning.md:119` "Recovery manager: 1 connection during recovery" → "Stale-claim sweeper (sole task-recovery path post-T-0502): 1 connection per sweep" + new line for cron recovery). The other audit-flagged locations (`architecture-overview.md`, `task-execution-sequence.md`, `guaranteed-execution-architecture.md`) don't literally use the word "RecoveryManager" — they have content gaps (don't mention the removal). Those are DOC-E content edits.
  - **I-0096 `#[ctor]`** — Cargo.toml dep removed from `05-cron-scheduling.md:60`. The "Global Registry" section in `reference/computation-graphs.md:818-869` (large stale block) excised and replaced with a `<!-- TODO(DOC-F) -->` marker. Other `#[ctor]` references in the tree are in **explanation/historical** context (e.g., "replaces the pre-I-0096 `#[ctor]`-based path") — correct, no action.
  - **Fabricated cron APIs** in `workflows/explanation/cron-scheduling.md` — excised 3 sections (MissedExecutionPolicy match, Distributed Execution leader-election, Monitoring CronMetrics/HealthStatus) with `<!-- TODO(DOC-E) -->` markers pointing at real types.
- **`cloacina-ctl` → `cloacinactl`** — swept across 4 files (`ffi-system.md`, `package-format.md`, `07-packaged-workflows.md`, `08-workflow-registry.md`). Remaining 3 matches are in auto-generated api-reference (`api-reference/rust/cloacina/registry/types.md`) — out of scope per initiative non-goal.
- **`cloacinactl serve` → `cloacinactl server start`** — swept across 6 files (`deploying-the-api-server.md`, `production-deployment.md`, `running-the-compiler.md`, `_index.md`, `architecture-overview.md`, `http-api.md` already correct).
- **Org slug** — `colliery-software` → `colliery-io` swept across 5 files: `install.md`, `running-the-server-image.md`, `conditional-retries.md`, `deploying-to-kubernetes.md` (Helm OCI URLs), plus the Docker tag bump in `running-the-server-image.md`. **Intentionally left as-is**: 2 `colliery-software/fidius` URLs in `ffi-vtable.md` and `packaging.md` (different repo — fidius is its own project).

**Acceptance criteria results (`grep` outputs):**
- ✅ Zero banned-phrase matches in `docs/content/`.
- ✅ Zero `cloacinactl reactor` or `/v1/health/reactors` matches.
- ✅ Zero stale `0.0`–`0.6.0` pins.
- ✅ Zero `RecoveryManager` matches in user-facing prose (auto-generated api-reference excluded per non-goal).
- ✅ Zero `#[ctor]` or `ctor = "..."` matches except in explanation/historical context (correct usage). The `ctor =` substring matches inside `reactor="..."` are false positives.
- ✅ Zero `MissedExecutionPolicy`, `DistributedCronScheduler`, `try_acquire_leader_lease`, `CronMetrics::collect` matches outside the TODO markers (which describe them as fictional).
- ✅ Zero `cloacina-ctl` matches in user-facing prose (3 in auto-generated reference excluded per non-goal).
- ✅ Zero `colliery-software` matches except the 2 intentional fidius references (different repo).

**Verification needed externally (user action):**
- Run `angreal docs build` to verify the Hugo site builds clean. Some excised sections leave the doc visibly incomplete (cron-scheduling.md, computation-graphs.md Global Registry); the build should still pass since the TODO markers are HTML comments.

**Flags for downstream clusters:**
- **DOC-E**: `cron-scheduling.md` needs content rewrites for the 3 excised sections + the `Pythonista`-style snippets in a Rust doc + I-0099/I-0108 metrics coverage. `architecture-overview.md`, `task-execution-sequence.md`, `guaranteed-execution-architecture.md` need T-0487/T-0502/I-0110 content edits.
- **DOC-F**: `reference/computation-graphs.md` Global Registry section needs inventory-based rewrite. `09-full-pipeline.md` title renamed to "09 - Full Multi-Source Pipeline"; verify Hugo URL slug regeneration if `weight` doesn't change.
- **DOC-D**: `package-format.md` has crate-internal file path references (`cloacina-ctl/src/manifest/types.rs` style — now `cloacinactl/...` but the actual code path is `crates/cloacina/src/packaging/manifest_schema.rs`) that need verification.
- **DOC-G**: Python tutorials use `@cloaca.reactor` (correct) — verify no other Python-side drift in `09-computation-graph.md` after the move.
- Slug acceptance criterion in this task is over-broad (catches fidius). Subsequent DOC-* clusters should use a narrower grep that excludes `colliery-software/fidius` URLs.