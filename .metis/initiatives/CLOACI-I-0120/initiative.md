---
id: documentation-overhaul-orientation
level: initiative
title: "Documentation overhaul — orientation, correctness, IA, and usefulness across the docs tree"
short_code: "CLOACI-I-0120"
created_at: 2026-06-15T03:15:47.264703+00:00
updated_at: 2026-06-15T03:17:43.366643+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: documentation-overhaul-orientation
---

# Documentation overhaul — orientation, correctness, IA, and usefulness across the docs tree

## Context **[REQUIRED]**

The docs have grown organically with the product and drifted. A Diátaxis Phase-1
discovery (2026-06-15, four grounded inventory agents) surveyed the whole
`docs/content/` tree (~429 markdown files, Hugo + `hugo-geekdoc`, organized
**feature-area-first**: `workflows/`, `computation-graphs/`, `python/`,
`platform/`, each with the four quadrants; plus a 249-file auto-generated
`api-reference/`).

**The headline problem is correctness drift — and it runs deeper than stale
prose.** The docs (and even some specs/ADRs) describe *shipped* capability as
planned/in-progress:
- **Execution-agent fleet** (I-0114) — verified complete this cycle (e2e passes),
  but described as "planned/stub".
- **Web UI** (I-0117) — shipped (merged; demo stack runs it), described as "not
  started".
- **SDKs** rust/python/ts (I-0113) — completed + published, described as
  "scaffold/in-progress".
- **Interservice WS substrate** (I-0115) — implemented (exercised in the cli
  e2e), described as "in discovery".
- ~10 deferred/stub Python docs (DOC-G "Phase 5" TODOs) and 1 stale
  computation-graph reference (pre-I-0101/I-0102 macro syntax).

**Operating principle for this initiative:** ground every reference claim and
how-to step against **current code**, not specs or existing docs. Verify defaults
and signatures against source.

**Orientation/onboarding gaps (the prioritized concern):** no "when NOT to use
it"; no competitive positioning; value-prop scattered across README / `_index` /
quick-start; the best first-success path (demo stack → live UI) is buried;
near-absent reviewer/date metadata; Python docs siloed from Rust; confusing
shared tutorial numbering (CG tutorials start at 07).

**Strengths to preserve:** reference coverage maps cleanly to code (CLI noun/verb,
HTTP API, config, env vars all inventoried to file:line); a strong 270-term
glossary; the four-quadrant discipline mostly holds.

**Prior art:** CLOACI-I-0112 ("Documentation review and refresh", in `decompose`)
and its completed DOC-A…I tasks (T-0611–0619) were a May-2026 batch. This
initiative supersedes/absorbs I-0112's intent.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A clear orientation layer answering, up front: what Cloacina is, why use it,
  **when not to**, what its features are, and how to start (the fastest real
  first-success).
- Correctness: docs match current shipped reality (fleet/UI/SDKs/substrate are
  done); every reference claim is code-traceable.
- Usefulness/IA: audience + prerequisites on section landings; fixed cross-links
  and tutorial numbering; Python de-siloed from Rust; cross-cutting topics
  discoverable.
- Each written/overhauled doc passes the four adversarial reviewers (accuracy,
  completeness, clarity, diataxis-compliance) to zero blockers/majors.

**Non-Goals:**
- **Not** a structural IA migration. Decision (2026-06-15): improve *within* the
  current feature-area-first structure; preserve existing URLs.
- **Not** hand-writing the 249 auto-generated `api-reference/` docs — those are
  regenerated from rustdoc/plissken; only fix the 2 missing `_index.md` pages and
  trigger regeneration where stale.
- Not changing the docs toolchain (stay on Hugo + geekdoc).
- Not net-new product features (docs only; code bugs found get their own tickets).

## Detailed Design **[REQUIRED]**

Decisions (approved 2026-06-15):
- **IA:** improve within feature-area-first; add an orientation/concepts layer on
  top; add audience/prereqs to `_index.md` landings; preserve URLs.
- **Execution:** drive autonomously per slice — write → adversarial review loop →
  commit on a branch → PR; report at slice boundaries.
- **Grounding:** verify against current code, not specs/old docs.

Slice structure (each slice = one Metis task, one PR, one review loop):

- **T1 — P0 Orientation & onboarding.** Overhaul the top-of-funnel: site
  `_index`, README alignment, a crisp "What is Cloacina / why / **when not to
  use** / features overview" (explanation), a single clear getting-started that
  surfaces the fastest real path, and a concepts/primitives orientation page
  (per S-0011). Cross-link ADR-0005 trust model.
- **T2 — P1 Correctness & accuracy sweep.** Fix the status drift
  (fleet/UI/SDKs/substrate = shipped); finish-or-cut the ~10 deferred Python
  docs; fix the stale CG reference; reconcile the package-manifest/format docs
  already touched by I-0119; verify reference claims against code.
- **T3 — P2 IA & usefulness.** Audience/prereqs on section landings; fix tutorial
  numbering + the shared-namespace confusion; de-silo Python (clear "differs from
  Rust" framing, not a syntax-swap mirror); cross-cutting topic discoverability
  (multi-tenancy, observability, performance); reviewer/date metadata policy.
- **T4 — P3 Reference gap-fill + API index.** Fill reference gaps surfaced by the
  inventory (CLI flags, env vars, config keys, exit codes, error envelope, WS
  protocol); add the 2 missing `api-reference` `_index.md`; regenerate stale API
  pages.

Each slice runs the Phase-4 review gate: dispatch accuracy-reviewer,
completeness-reviewer, clarity-reviewer, diataxis-compliance-reviewer in
parallel; address all blockers/majors; re-run until accuracy/completeness/
compliance return zero blockers+majors and clarity zero blockers.

## Alternatives Considered **[REQUIRED]**

- **Restructure to quadrant-first IA** (Tutorials/How-to/Reference/Explanation at
  top level) — rejected (2026-06-15): large disruptive migration, breaks URLs;
  the current structure is documented as intentional in
  `contributing/documentation.md`. Improve within it instead.
- **Per-slice approval before writing** — rejected: user chose autonomous
  per-slice execution with review at slice boundaries (via PRs).
- **Rewrite all 429 docs including API reference** — rejected: the 249
  auto-generated API docs are regenerated, not hand-authored; rewriting them by
  hand would immediately drift.
- **Reuse I-0112** — rejected: I-0112 is a stale prior batch in `decompose` with
  its tasks already completed; a fresh initiative is cleaner. I-0112 to be
  superseded/closed.

## Implementation Plan **[REQUIRED]**

Build T1→T4 in order (T1 orientation is the user's priority and the highest
leverage). Each is an independently shippable PR with its own review loop. The
Phase-1 inventory (in the session record) is the coverage checklist T2/T4 are
measured against.

- T1 — P0 Orientation & onboarding
- T2 — P1 Correctness & accuracy sweep
- T3 — P2 IA & usefulness
- T4 — P3 Reference gap-fill + API index

Closeout: supersede/close CLOACI-I-0112.

## Exit Criteria

- Orientation layer answers what/why/when-not/features/getting-started, linked
  from the landing page and README.
- No doc describes a shipped capability (fleet, UI, SDKs, substrate) as
  planned/in-progress; the ~10 deferred Python docs are written or removed; the
  CG reference matches current macro syntax.
- Every reference claim in touched docs is code-traceable; the four reviewers
  return zero blockers/majors (clarity zero blockers) on each slice.
- Section landings carry audience + prerequisites; tutorial numbering is
  unambiguous; Python docs explain their relationship to Rust.
- CLOACI-I-0112 superseded/closed.