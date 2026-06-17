---
id: documentation-review-and-refresh
level: initiative
title: "Documentation review and refresh — drift audit, gap coverage, and Diataxis compliance for the May 2026 release batch"
short_code: "CLOACI-I-0112"
created_at: 2026-05-18T15:47:20.059714+00:00
updated_at: 2026-06-17T11:38:12.723621+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: documentation-review-and-refresh
---

# Documentation review and refresh — drift audit, gap coverage, and Diataxis compliance for the May 2026 release batch Initiative

## Context

The Cloacina documentation site (`docs/`, served via Hugo, organized under the Diataxis framework) was last given a comprehensive pass on 2026-04-10 (commits `c747744b` "comprehensive documentation review", `15ceff24` "restructure site by feature area", `29c8229b` "Diátaxis compliance and clarity fixes", `5e6cfa69` "add 7 new Diátaxis docs"). Since then, ~16 closed initiatives and 40+ shipped tasks have changed the surface area materially. Spot checks already confirm drift in the published tree.

**Architecture / crate boundary shifts**

- Six new crates: `cloacina-build`, `cloacina-compiler`, `cloacina-python`, `cloacina-server`, `cloacina-workflow`, `cloacina-workflow-plugin`.
- `cloacina-compiler` extracted as a separate binary with DB-coordinated build queue (I-0097); compiler hardening Phase 1 (I-0104) added timeouts, `--frozen --offline` defaults, `setrlimit`, and an interim deployment posture.
- `cloacinactl` redesigned as a strict noun-verb CLI with profile model and `~/.cloacina/config.toml` (I-0098, T-0538). Nouns now: `compiler`, `daemon`, `execution`, `graph`, `key`, `package`, `server`, `tenant`, `trigger`, `workflow`.
- `cloacina-python` runtime isolated into its own crate (T-0529, T-0532).
- `ServiceManager` extracted from `DefaultRunner` (T-0483); `RecoveryManager` removed — heartbeat sweeper is now the sole recovery path (T-0502).
- Cooperative task cancellation on claim loss (T-0487).

**Nomenclature lockdown (S-0011 + T-0528, 2026-04-23)**

- Banned phrases: "reactive scheduler", "reactive computation graph", "reactive subsystem", "reactive execution".
- `ReactiveScheduler` → `ComputationGraphScheduler`; `cloacinactl reactor` → `cloacinactl graph`; `/v1/health/reactors` → `/v1/health/graphs`; `health_reactive.rs` → `health_graphs.rs`.
- Already-drifting docs caught in spot check:
  - `docs/content/computation-graphs/_index.md:9` — "Cloacina's reactive execution engine".
  - `docs/content/computation-graphs/explanation/reactor-lifecycle.md:73` — `cloacinactl reactor force-fire <name>`.

**May 2026 release batch — capability changes that need doc coverage**

| Initiative | Surface |
|---|---|
| I-0099 | CG observability metrics: reactor + accumulator + scheduler-loop + WS counters/histograms/gauges in `cloacina_*` namespace |
| I-0100 | DB-backed reactor → workflow subscription fan-out; durable event log; upstream-declaration pattern |
| I-0101 | `#[computation_graph]` macro split: `trigger = reactor("name")` standalone declaration, `invokes = computation_graph("name")` workflow-task variant |
| I-0102 | Unified `cloacina::package!();` shell, single FFI plugin per cdylib; `package_type` removed; macro-only declarations |
| I-0103 | `cloacina-server --require-signatures` + `--verification-org-id <UUID>`, fail-closed verification at canonical load path |
| I-0104 | Build timeouts, offline-by-default builds, `setrlimit` limits, interim deployment posture documented |
| I-0106 | Multi-tenant abstraction: fail-closed `SET search_path`, full `remove_tenant` teardown orchestration, drain timeout |
| I-0107 | Unified `ApiError` REST envelope, list pagination (`list_triggers`, `get_trigger`), SSE `--follow` for execution streaming |
| I-0108 | SQL-derived `cloacina_active_tasks` gauge re-seed; persist-failure counters; `Degraded` health for reactors after 5 consecutive persist failures |
| I-0109 | `cloacina-compiler /metrics` endpoint; `--log-retention-days` on compiler, server, and daemon |
| I-0110 | Atomic `complete_task_transaction`; typed JSON parse/merge errors with counters; deterministic `final_context` tiebreaker by completion timestamp |
| I-0111 | One-line install script, Docker image, Helm chart distribution |
| T-0602 | CEL predicate filtering for reactor-triggered workflows (`filtered-reactor` example) |
| T-0610 | Local Postgres subchart embedded into the Helm chart (replaces bitnamilegacy dependency) |
| T-0608, T-0609 | SQLite `:memory:` substitution in test paths; rdkafka build deps in server Dockerfile |

**Structural problems in the existing IA**

- `docs/operations/compiler-deployment.md` (186 lines), `docs/operations/metrics.md` (226 lines), and `docs/SIGSEGV_TROUBLESHOOTING.md` (70 lines) sit outside `docs/content/`. They are not part of the published Hugo site.
- `docs/content/platform/tutorials/` has only one tutorial (`01-deploy-a-server.md`). Service-side learning path is thin.
- Tutorial numbering is fragmented across four buckets (`workflows/library` 01–04, `workflows/service` 05–10, `computation-graphs/library` 07–10, `computation-graphs/service` 07–10) — the same numbers carry different meanings.
- Three docs still pin Cargo install snippets to `0.5.x` or `0.6.0` while crates are at `0.6.1`: `quick-start/install.md`, `platform/how-to-guides/configure-multi-tenant-deployment.md`, `platform/how-to-guides/running-the-server-image.md`.
- `docs/content/python/` does not mirror the Rust workflows-vs-computation-graphs split at the Diataxis-section level (tutorials are split, but how-to / reference / explanation are not). Inconsistent with the rest of the site.

## Goals & Non-Goals

**Goals:**
- **Accuracy.** Every claim in the published Hugo site (`docs/content/**`) cross-references against the current code. Banned-phrase drift (S-0011), stale CLI verbs, stale HTTP paths, stale version pins, removed types, and renamed modules are corrected.
- **Coverage of the May 2026 batch.** Each shipped initiative in the batch (I-0099, I-0100, I-0101, I-0102, I-0103, I-0104, I-0106, I-0107, I-0108, I-0109, I-0110, I-0111) plus the loose tasks T-0487, T-0502, T-0529, T-0532, T-0538, T-0602, T-0608, T-0609, T-0610 has at least one doc that explains the user-visible behavior. New surface gets reference entries.
- **Operations folded into platform.** `docs/operations/compiler-deployment.md` → `docs/content/platform/how-to-guides/` (deployment runbook). `docs/operations/metrics.md` → `docs/content/platform/reference/metrics-catalog.md`. `docs/SIGSEGV_TROUBLESHOOTING.md` → folded into `docs/content/troubleshooting.md`. Operations directory deleted.
- **Python IA parity.** `docs/content/python/` restructured into `python/workflows/{tutorials,how-to-guides,reference,explanation}` and `python/computation-graphs/{...}` to mirror the Rust side. Existing Python tutorials slot in; how-to/reference/explanation slots get migrated or marked TBD with explicit gap stubs.
- **Diataxis compliance.** Each document stays in its quadrant: tutorials are learning-oriented and reproducible end-to-end, how-to guides are task-oriented recipes with concrete goals, reference is austere and structured for lookup, explanation is understanding-oriented. Mixed docs get split.
- **Cross-linking.** Tutorials link to reference. How-to guides link to explanation for the "why." Explanation links to the relevant ADRs (`CLOACI-A-0001..0005`) and specs (`CLOACI-S-0001..0011`).
- **Concrete examples.** Real values, real flags, real Helm/Docker tags — no `{placeholder}` text. Where the example references an angreal task or `examples/` directory, the link points at the actual path.

**Non-Goals:**
- No new IA restructuring beyond the operations fold-in, the Python parity, and any S-0011-driven file renames. The high-level `workflows/`, `computation-graphs/`, `platform/`, `python/`, `api-reference/` split stands.
- No rewrite of auto-generated API reference content (`api-reference/rust/` and `api-reference/cloaca/`). Those come from `cargo doc` / `pdoc` and are out of scope; this initiative covers the prose around them.
- No new Hugo theme or layout work. The site uses `hugo-geekdoc`; the structure stays.
- No documentation for the active initiative `CLOACI-I-0105` (Compiler hardening Phase 2 — sandbox), which is still in discovery. Coverage of Phase-2 sandboxing is deferred to the initiative's own decompose phase.
- No archived-Metis-doc rewrites. Historical task and initiative records remain as written.

## Source Findings (May 2026 discovery)

### Confirmed drift in published docs

- **NOM-01** — `docs/content/computation-graphs/_index.md:9` uses the banned phrase "reactive execution engine".
- **NOM-02** — `docs/content/computation-graphs/explanation/reactor-lifecycle.md:73` references `cloacinactl reactor force-fire <name>`; post-T-0528 the noun is `graph`.
- **VER-01** — `docs/content/quick-start/install.md`, `docs/content/platform/how-to-guides/configure-multi-tenant-deployment.md`, `docs/content/platform/how-to-guides/running-the-server-image.md` pin `cloacina = "0.5.x"` or `"0.6.0"` Cargo snippets while crates are at `0.6.1`.
- **IA-01** — `docs/operations/{compiler-deployment.md,metrics.md}` and `docs/SIGSEGV_TROUBLESHOOTING.md` are orphaned from the Hugo content tree.
- **IA-02** — `platform/tutorials/` contains only `01-deploy-a-server.md`. No follow-up tutorials for compiler-pair deploy, multi-tenant setup, or signature verification.
- **IA-03** — Tutorial numbering collides across buckets: `workflows/library` 01–04, `workflows/service` 05–10, `computation-graphs/library` 07–10, `computation-graphs/service` 07–10. The same `07` means different things in different directories.

### Likely-gap inventory (to confirm during Phase 2)

For each shipped initiative, the user-visible surface and the doc-tree slot it likely belongs in:

- **I-0097 / I-0104 / I-0109 (compiler service):** how-to `platform/how-to-guides/running-the-compiler.md` exists — needs hardening flags, `/metrics`, log retention, build queue lifecycle. Migration target for `docs/operations/compiler-deployment.md`.
- **I-0098 / T-0538 (CLI redesign):** `platform/reference/cli.md` exists — verify completeness against `crates/cloacinactl/src/nouns/{compiler,daemon,execution,graph,key,package,server,tenant,trigger,workflow}`. New how-to candidates: "Use a named profile", "Switch between local daemon and remote server".
- **I-0099 / I-0108 (CG observability):** `platform/reference/metrics-catalog.md` (new, from `docs/operations/metrics.md`) — must list every `cloacina_*` metric with type, labels, and meaning. Cross-link from `computation-graphs/explanation/` and `platform/how-to-guides/`.
- **I-0100 (reactor → workflow subscriptions):** `computation-graphs/explanation/` needs a "subscription fan-out" doc. `computation-graphs/how-to-guides/reactor-triggered-workflows.md` exists — verify it covers the DB-backed durable path, not just the in-process fast path.
- **I-0101 (CG / reactor decouple):** `computation-graphs/explanation/architecture.md` needs the post-decoupling topology. New how-to for the embedded `invokes = computation_graph(...)` workflow-task pattern. Reference doc for the `#[computation_graph]` macro arguments must reflect the split.
- **I-0102 (unified package shell):** `platform/reference/package-manifest.md` and `platform/reference/package-shell-macro.md` exist — verify they reflect `cloacina::package!();`, removal of `package_type`, and macro-only triggers/reactors/graphs.
- **I-0103 (signature verification):** `platform/how-to-guides/security/` — needs a "Require signed packages" how-to and a reference entry for `--require-signatures` / `--verification-org-id`.
- **I-0106 (multi-tenant abstraction):** `platform/explanation/multi-tenancy.md` exists — verify it covers fail-closed `search_path`, `remove_tenant` orchestration order, drain timeout. New how-to candidate: "Decommission a tenant safely".
- **I-0107 (CLI/server contract):** `platform/reference/http-api.md` and `cli.md` — verify they document the unified `ApiError` envelope, list pagination on triggers, SSE `--follow` for executions.
- **I-0110 (correctness sweep):** `workflows/explanation/guaranteed-execution-architecture.md` exists — verify it reflects the atomic `complete_task_transaction`, JSON-merge typed errors, and deterministic tiebreaker.
- **I-0111 / T-0610 (distribution):** `quick-start/install.md` covers the install script; `platform/how-to-guides/deploying-to-kubernetes.md` covers the Helm chart — verify both reflect the embedded Postgres subchart and the `ghcr.io/colliery-software/cloacina-server` image.
- **T-0487 / T-0502 (cancellation + heartbeat-only recovery):** `workflows/explanation/guaranteed-execution-architecture.md` / `task-execution-sequence.md` — verify they reflect cooperative cancellation on claim loss and the removal of `RecoveryManager`.
- **T-0529 / T-0532 (cloacina-python split):** `python/_index.md` and `python/quick-start.md` — verify they reflect the crate split and standalone Python runtime usage.
- **T-0602 (CEL predicate filtering):** new how-to under `computation-graphs/how-to-guides/` for filtered reactor subscriptions, with the `examples/features/computation-graphs/filtered-reactor` example as the worked case.

## Detailed Design

### Diataxis quadrant rules (enforced in Phase 3)

- **Tutorials** must be reproducible end-to-end against the `examples/tutorials/` tree. Each tutorial states prerequisites, runnable command (preferred: an `angreal demos:tutorials:*` invocation), and expected output. No "fill in your own DB URL" hand-waving — concrete URLs (e.g. `sqlite://./tutorial.db`) only.
- **How-to guides** must each have a one-sentence problem statement, a list of prerequisites, ordered steps, and a verification step. No exposition; no "why" prose.
- **Reference** docs are lookup tables. Every flag, env var, endpoint, metric, error code, and CLI subcommand must be listed with type, default, and meaning. No tutorials disguised as reference.
- **Explanation** docs answer "why" and "how does this work conceptually." They link to ADRs and specs for the rationale, and to reference for the lookup details.

### Cross-link policy

- Every tutorial closes with a "What's next" block linking to the relevant how-to and reference.
- Every how-to opens with a "Background" link to the explanation doc that motivates it.
- Every explanation links to the relevant `CLOACI-A-*` ADR and/or `CLOACI-S-*` spec when one exists.
- Reference docs are link targets, not link sources.

### Information architecture changes (this initiative only)

1. **Fold `docs/operations/` into Hugo.**
   - `docs/operations/compiler-deployment.md` → `docs/content/platform/how-to-guides/compiler-deployment-runbook.md` (existing `running-the-compiler.md` becomes the short-form how-to; this is the long-form runbook).
   - `docs/operations/metrics.md` → `docs/content/platform/reference/metrics-catalog.md` (full catalog of every `cloacina_*` metric).
   - `docs/SIGSEGV_TROUBLESHOOTING.md` → merge into `docs/content/troubleshooting.md` as a `### Native crash troubleshooting` section.
   - Delete `docs/operations/` and `docs/SIGSEGV_TROUBLESHOOTING.md`.
2. **Python IA parity.** Reshape `docs/content/python/` to:
   - `python/_index.md` (overview, links to both surfaces)
   - `python/quick-start.md` (unchanged location)
   - `python/workflows/{tutorials,how-to-guides,reference,explanation}/`
   - `python/computation-graphs/{tutorials,how-to-guides,reference,explanation}/`
   - `python/api-reference/` (kept as-is — auto-generated)
   - Existing tutorials already split into `python/tutorials/workflows/` and `python/tutorials/computation-graphs/` — move to the new tree. Reference, how-to, and explanation slots that have no current content get explicit stub `_index.md` files marking them as gaps to fill.
3. **No tutorial-numbering rework.** The IA-03 collision is real but renumbering is out of scope for this initiative — flag as a future cleanup.

### Phase mapping

The user's 4-phase methodology maps to the initiative lifecycle:

| User phase | Initiative phase | Activity |
|---|---|---|
| 1. Deep Discovery | `discovery` (current) | Already done — see "Source Findings" + "Likely-gap inventory" |
| 2. Documentation Plan | `design` | Produce a per-document outline (title, target audience, key topics, dependencies) across all four Diataxis categories. Lock IA changes. Confirm doc-by-doc gap list. |
| 3. Write Documentation | `decompose` → `active` | One task per logical doc cluster (~5–10 docs per task to keep tasks 1–14 day shaped). Tasks executed via Ralph. Write fully — no TODOs or placeholders. |
| 4. Review | `active` (review tasks) → `completed` | Spawn four parallel review-agent tasks (Accuracy, Completeness, Clarity, Diataxis-Compliance). Findings folded back into open write tasks before transitioning the initiative to `completed`. |

### Phase 2 design-phase deliverable

A single document (`design.md` inside the initiative directory) listing every document to be written or rewritten, in this shape:

```markdown
### docs/content/platform/reference/metrics-catalog.md (new)
- **Category:** Reference
- **Audience:** Platform operators, on-call engineers
- **Sources:** docs/operations/metrics.md (port + extend), I-0099, I-0108, I-0109
- **Key topics:** Every cloacina_* metric: name, type, labels, meaning, where it's emitted
- **Depends on:** platform/explanation/observability.md (background), platform/how-to-guides/wire-prometheus.md (setup)
- **Cross-links:** computation-graphs/explanation/architecture.md, ADR-XXXX if applicable
```

That document is the load-bearing artifact between Phase 2 and Phase 3.

### Phase 4 review structure

Four parallel review tasks. Each reviews the **entire delta** produced by Phase 3 (not a slice), with a different lens:

- **Accuracy:** every claim cross-referenced against `crates/`, `examples/`, `crates/cloacinactl/src/nouns/`, server routes, Helm chart, install script. Flag discrepancies with file:line citations.
- **Completeness:** features, flags, config options, workflows, edge cases that exist in code but are missing from docs. Output: gap list.
- **Clarity:** read each doc from the target audience's perspective (named in Phase 2 outline). Flag jargon without definition, unclear steps, missing prerequisites.
- **Diataxis-Compliance:** verify each doc stays in its quadrant. Flag tutorials drifting into reference, how-to drifting into explanation, etc.

Each review agent returns a findings file under `.metis/initiatives/CLOACI-I-0112/review-<lens>.md`. Findings are triaged into either "fix in Phase 3 cluster task X" or "follow-up backlog item" before the initiative transitions to `completed`.

## Alternatives Considered

- **Drift-only quick pass.** Fix S-0011 nomenclature drift, version pins, and orphaned ops files. Rejected — leaves the May batch coverage unaddressed, and a future review would still have to do the larger pass.
- **Full IA restructure including tutorial renumbering.** Rejected for this initiative — collision of tutorial numbers across `library/` and `service/` buckets is real but solving it cleanly requires picking a numbering convention (continuous vs per-bucket) that affects every cross-link. Flagged as a follow-up cleanup.
- **Per-feature mini-initiatives.** One initiative per shipped feature with embedded doc work. Rejected — the May batch shipped as a coordinated release, and a single doc-pass produces a coherent site; the per-feature approach would scatter cross-link decisions and risk inconsistency between docs landing weeks apart.
- **Single-author serial Ralph loop.** Rejected — too slow at ~30 docs. The decompose-into-clusters approach lets multiple tasks run in parallel (different file surfaces, disjoint per the worktree-pattern feedback) while staying coherent under the Phase 2 outline.

## Implementation Plan

**Phase 1 — Discovery (complete on initiative creation).** This document.

**Phase 2 — Design (~3 days).** Produce `design.md` — the per-document plan covering every existing doc that needs rewrite-or-rename and every new doc to be authored. Lock the IA changes and the Python restructure. Human check-in to approve before decomposing.

**Phase 3 — Decompose + Write (~3 weeks).** Break the work into 5–10 tasks of roughly equal weight by file-surface clustering (not by Diataxis category — a cluster typically spans tutorial + how-to + reference + explanation for one feature area). Example clusters:

- **DOC-A** — Nomenclature & version pass: NOM-01, NOM-02, VER-01, S-0011 sweep across published docs.
- **DOC-B** — Platform reference refresh: CLI, HTTP API (incl. ApiError + pagination + SSE), configuration, environment variables, package manifest, package shell macro, metrics catalog (new).
- **DOC-C** — Platform how-to + tutorials: running the compiler, running the server, running the daemon, deploying to Kubernetes (Helm + embedded Postgres), require signed packages, decommission a tenant, configure multi-tenant deployment. Fold `compiler-deployment.md` runbook in here.
- **DOC-D** — Platform explanation: multi-tenancy, database backends, FFI system, package format, horizontal scaling, performance characteristics, observability (new).
- **DOC-E** — Workflows quadrant refresh: tutorials/library 01–04, tutorials/service 05–10, how-to-guides, reference, explanation. Cover I-0110, T-0487, T-0502.
- **DOC-F** — Computation graphs quadrant refresh: tutorials, how-to-guides, reference, explanation. Cover I-0099, I-0100, I-0101, I-0102, T-0602. Includes the architecture/topology rewrite under S-0011.
- **DOC-G** — Python IA parity: restructure into workflows + computation-graphs quadrants, port existing tutorials, stub gap docs.
- **DOC-H** — Operations fold-in: move `compiler-deployment.md` and `metrics.md` into Hugo, merge SIGSEGV into `troubleshooting.md`, delete `docs/operations/`.
- **DOC-I** — Quick-start + glossary: refresh install snippets to current version, refresh README cross-link block, sync glossary against S-0011 primitives.

Final cluster count and boundaries get pinned in Phase 2.

**Phase 4 — Review (~4 days).** Four parallel review-agent tasks. Triage findings into Phase-3 task tasks or backlog. Spec compliance verification (S-0011) is part of the Diataxis-Compliance reviewer's lens; nomenclature drift in any doc fails the review.

**Close.** Transition initiative to `completed` after all review findings are dispositioned and CI builds the Hugo site green.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- `grep -r "reactive scheduler\|reactive computation graph\|reactive subsystem\|reactive execution\|ReactiveScheduler" docs/content/` returns no matches.
- `grep -r "cloacinactl reactor\|/v1/health/reactors" docs/content/` returns no matches except inside `## Changelog` or migration-note sections that explicitly call out the rename.
- `grep -r "cloacina = \"0\." docs/content/` shows only the current version (`0.6.1` or whatever is current at completion).
- `docs/operations/` directory deleted; its content lives in `docs/content/platform/`. `docs/SIGSEGV_TROUBLESHOOTING.md` deleted; its content lives in `docs/content/troubleshooting.md`.
- For each May 2026 initiative (I-0099, I-0100, I-0101, I-0102, I-0103, I-0104, I-0106, I-0107, I-0108, I-0109, I-0110, I-0111) and each named task (T-0487, T-0502, T-0529, T-0532, T-0538, T-0602, T-0608, T-0609, T-0610), at least one published doc covers the user-visible surface. The Phase 2 `design.md` lists the specific doc per item; review verifies the doc exists and matches the code.
- `docs/content/python/` mirrors the workflows-vs-computation-graphs split at the Diataxis-section level. Each quadrant directory has at least an `_index.md`; gap stubs are explicit, not hidden behind missing files.
- `angreal docs:build` builds the Hugo site green. All internal Hugo `{{< ref >}}` links resolve.
- All four Phase 4 review-agent reports are filed and triaged. No "Accuracy" review finding remains unaddressed at close.

## References

- **Specs:** `CLOACI-S-0011` (Cloacina primitive nomenclature — authoritative for all renames in this pass).
- **Vision:** `CLOACI-V-0001` (parent — particularly Principles "Local Development First" and "Trust by Deployment Mode" inform tutorial structure).
- **ADRs:** `CLOACI-A-0001` (DB backend selection), `CLOACI-A-0002` (execution history), `CLOACI-A-0003` (`cloacinactl` CLI surface), `CLOACI-A-0004` (compiler service), `CLOACI-A-0005` (deployment-mode trust model). Each should be cross-linked from the explanation doc(s) that motivate the design.
- **Closed initiatives in scope:** `CLOACI-I-0096..I-0111` (most recent batch). Archived initiative docs at `.metis/archived/initiatives/CLOACI-I-0099..0111/`.
- **Diataxis framework:** https://diataxis.fr/ (the four quadrants — tutorial, how-to, reference, explanation — and the rules for what belongs in each).
- **Hugo site config:** `docs/hugo.toml` (menu structure, taxonomies, theme is `hugo-geekdoc`).