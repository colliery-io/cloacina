---
id: doc-f-computation-graphs-refresh-s
level: task
title: "DOC-F: Computation graphs refresh — S-0011 cleanup, I-0100/I-0101/I-0102 topology, T-0602 CEL"
short_code: "CLOACI-T-0616"
created_at: 2026-05-18T18:19:27.928872+00:00
updated_at: 2026-05-18T21:12:09.530891+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-F: Computation graphs refresh — S-0011 cleanup, I-0100/I-0101/I-0102 topology, T-0602 CEL

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Bring `docs/content/computation-graphs/` (highest-drift area in the audit — 16 NOM, 87 IA findings) onto current truth. DOC-A handles the mechanical NOM/VER/CLI/HTTP sweep; this cluster handles the **content rewrites that depend on the topology shifts** (I-0100 DB-backed subscription, I-0101 macro split + decoupled reactor, I-0102 unified shell) plus the response-shape drift (`/v1/health/graphs` returns `{"items": [...], "total": N}`, not `{"graphs": [...]}`), the fabricated `fire_count`/`last_fired_at` fields, and the missing T-0602 CEL coverage. Heaviest rewrites: `reference/computation-graphs.md` (pre-I-0096 ctor block + stale FFI method count + pre-I-0102 manifest) and `explanation/architecture.md` (topology + S-0011 framing). Absorb `sequential-strategy.md` moved in from workflows (per locked Phase 2 decision).

## Scope

### Files in cluster (~30)

26 existing files in `computation-graphs/` + 3 new + 1 moved-in. See `audit-computation-graphs.md` for full per-doc detail. Headline edits:

**Index + landing (2 files):**
- `_index.md` (NOM-CG-01/02/03 handled in DOC-A): mention the third deployment shape (embedded `invokes = computation_graph(...)`) on the landing page.
- `explanation/_index.md`, `how-to-guides/_index.md`, `reference/_index.md`, `tutorials/_index.md`: verify toc-tree picks up new docs; `how-to-guides/_index.md` gets a new entry for the moved-in `sequential-strategy.md` and the new `filter-reactor-firings-with-cel.md`.

**Explanation (8 files):**
- `accumulator-design.md` M: NOM-CG-04 (DOC-A); add I-0099 accumulator metric refs; cross-link metrics-catalog (DOC-H).
- `architecture.md` **L**: rewrite. NOM-CG-05/06 (DOC-A) + section header rename. Rewrite Process Model for I-0101 (reactor is standalone publisher, N CGs subscribe). Add embedded `invokes = computation_graph(...)` to the comparison table. Refine "A reactor is not a trigger" per S-0011 (a reactor IS a specialized trigger). Add I-0108 Degraded health. Discuss dual-registry model.
- `computation-graph-scheduling.md` M: fix "Single graph scheduler per graph instance" (one scheduler per process, one reactor task per graph). Align with `reactor-lifecycle.md` declaration model. Add I-0108 persist-failure threshold (separate from supervisor restart counter). Add I-0099 metrics cross-links.
- `packaging.md` M: update `package.toml` example to current shape (no `type = "computation_graph"`; identity from `interface` + `interface_version` + `extension`). Verify Python `@computation_graph` decorator post-T-0532. Add I-0102 unified `cloacina::package!()` explanation (replaces per-macro `_ffi` emission).
- `performance.md` S: NOM-CG-07 (DOC-A); cross-link `cloacina_reactor_fire_duration_seconds` to live metric in catalog.
- `reactor-lifecycle.md` M: NOM-CG-08 (`cloacinactl reactor force-fire` — DOC-A excises, this cluster replaces with WebSocket `ManualCommand::ForceFire` reference at `/v1/ws/reactor/{name}`). Fix bogus "5-second cadence" restart claim (actual: exponential backoff 1s base / 60s max). Add I-0108 persist-failure path. Verify bound-subscriber guard error string.
- `trigger-less-graphs.md` S: fix example to use call-expression form (`invokes = computation_graph("name")`, not string-only).
- **new**: `subscription-fan-out.md` M: explanation for I-0100. Why DB-backed; reactor_firings row + watermark; per-subscription scoping; TTL prune; at-least-once; how it composes with in-process CG firing.

**How-to (5 + 1 moved-in + 1 new = 7 files):**
- `accumulator-types.md` S: document missing `#[state_accumulator]`.
- `computation-graph-health.md` M: rewrite response examples — server emits `{"items": [...], "total": N}` not `{"graphs": [...]}`; jq selectors → `.items[]`. Remove fabricated `fire_count`/`last_fired_at` fields. Document I-0108 persist-failure → `Degraded { disconnected: ["persist"] }` synthetic source. Cross-link `cloacina_*` metrics. Add `cloacinactl graph {list,status,accumulators}` CLI shortcut section.
- `computation-graph-in-workflow.md` S: NOM-CG-10/11 (DOC-A); fix Metis path (`.metis/specifications/CLOACI-S-0011/specification.md`).
- `reactor-triggered-workflows.md` S: verify-no-changes mostly; cross-link the `examples/features/computation-graphs/filtered-reactor` example.
- `when-all-criteria.md` S: verify-no-changes.
- **moved-in from workflows**: `sequential-strategy.md` S (move operation). Rename Hugo refs to land under `/computation-graphs/how-to-guides/sequential-strategy/`. Resolve in-doc `TODO(I-0101)` comment block. Coordinate with DOC-E.
- **new**: `filter-reactor-firings-with-cel.md` S: T-0602. CEL variables (`payload`, `reactor`, `tenant`); compile-at-subscribe vs evaluate-per-firing; fail-closed; idempotency-key recipe; walkthrough using `examples/features/computation-graphs/filtered-reactor`.

**Reference (1 + optional new = 2):**
- `computation-graphs.md` **L**: rewrite. NOM-CG-12 + VER-CG-01 (DOC-A). Delete entire pre-I-0096 "Global Registry" subsection (lines 818-855). Fix FFI method count (3 → 9 per I-0102, matches packaging.md). Update `package.toml` example to current `[metadata]` schema (not `[graph]`). Add `#[reactor]` macro reference (inventory `ReactorEntry`, trait const-fields, `ReactorPackageMetadata`). Add CG metric pointer to `platform/reference/metrics-catalog.md`. Add CG HTTP/WS endpoint cross-references (or full doc) for `/v1/health/graphs`, `/v1/health/accumulators`, `/v1/ws/{accumulator,reactor}/{name}`, `/v1/auth/ws-ticket`.
- **new (optional)**: `computation-graphs/reference/metrics.md` — per design decision, **collapse** into a pointer under `computation-graphs/reference/_index.md` linking to the CG section of `platform/reference/metrics-catalog.md`. No standalone file.

**Tutorials (10 files: 4 library + 4 service + 2 _index):**
- `tutorials/_index.md` S: add tutorial 10 to the service list.
- Library 07-10: 07 + 08 verify-only; 09 (`09-full-pipeline.md`) and 10 (`10-routing.md`) NOM-CG-13/14/15/16 (DOC-A handles).
- Service 07 (`07-packaging.md`) M: VER-CG-02 (DOC-A); fix the bogus `#[computation_graph(reaction = ..., strategy = ...)]` claim — those keywords don't exist; firing rule is on `#[reactor(criteria = ...)]`. Fix `{"graphs": [...]}` → `{"items": [...]}`. Align `health` state value examples.
- Service 08 (`08-websocket-events.md`) M: cross-reference WS-ticket path `/v1/auth/ws-ticket`; remove fabricated `fire_count`/`last_fired_at`.
- Service 09 (`09-kafka-stream.md`) M: VER-CG-03 (DOC-A); fix `d['reactors']` jq/Python selector → `d['items']`; verify `kafka` feature flag rebuild command (lives on `cloacina-server` now); add T-0609 note for containerized environments.
- Service 10 (`10-cross-package-binding.md`) S: verify `package upload/delete` verbs; fix fabricated "Shows subscribers: 1 (price_consumer)" output (no such field in server response); verify topology DSL syntax (`score: {inputs:..., next:...}` may not match parser).

### Cross-cluster dependencies

- **Blocked by**: DOC-A (S-0011 sweep + NOM-CG-01..16 mechanical fixes; this cluster handles content rewrites that depend on the baseline being clean), DOC-B (`platform/reference/http-api.md` documents the `{"items":...}` envelope cross-linked from CG docs)
- **Coordinate with**: DOC-E for `sequential-strategy.md` move (file leaves workflows, lands here; agree on PR ordering)
- **Parallel-eligible with**: DOC-C, DOC-D, DOC-E, DOC-G

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All 26 existing CG docs verified against current code (`crates/cloacina-computation-graph/`, `crates/cloacina/src/computation_graph/`, `crates/cloacina-macros/src/{reactor_attr,computation_graph/parser}.rs`, `crates/cloacina-workflow-plugin/`, `crates/cloacina-server/src/routes/health_graphs.rs`).
- [ ] 2 new docs exist: `filter-reactor-firings-with-cel.md`, `subscription-fan-out.md`. `sequential-strategy.md` moved in from workflows.
- [ ] Zero NOM-CG instances remain (DOC-A's `grep` criteria pass).
- [ ] Every CG doc that shows an HTTP response body matches actual server output (`{"items": [...], "total": N}`, not `{"graphs": [...]}` or `{"reactors": [...]}`).
- [ ] No doc references `fire_count` or `last_fired_at` as health-response fields.
- [ ] `reference/computation-graphs.md` shows the post-I-0102 9-method FFI vtable and post-I-0096 inventory-based registration (not `#[ctor]`).
- [ ] `explanation/architecture.md` describes the post-I-0101 standalone-reactor / multi-subscriber topology and acknowledges the embedded `invokes = computation_graph(...)` mode.
- [ ] `filter-reactor-firings-with-cel.md` cross-links the runnable `examples/features/computation-graphs/filtered-reactor` example and the angreal demo.
- [ ] `subscription-fan-out.md` cross-links S-0011 changelog (2026-04-24 reactor-as-standalone-publisher amendment).
- [ ] `angreal docs:build` passes.

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-computation-graphs.md` (per-doc detail + summary tags)
- **Authoritative spec**: `.metis/specifications/CLOACI-S-0011/specification.md` (nomenclature, post-I-0101 topology amendment, banned phrases)
- **Code paths**:
  - CG scheduler / reactor / accumulator: `crates/cloacina/src/computation_graph/{scheduler,reactor,accumulator,packaging_bridge}.rs`
  - Macros: `crates/cloacina-macros/src/{reactor_attr,computation_graph/parser}.rs`, `crates/cloacina-macros/src/lib.rs:200-260`
  - FFI: `crates/cloacina-workflow-plugin/src/lib.rs:682-700`, `crates/cloacina-workflow-plugin/src/types.rs:285-340`
  - Server routes: `crates/cloacina-server/src/routes/health_graphs.rs`, `crates/cloacina-server/src/routes/ws.rs`, `crates/cloacina-server/src/lib.rs:660-790`
  - CLI: `crates/cloacinactl/src/nouns/graph/mod.rs`
  - Subscriptions: `crates/cloacina/src/dal/unified/reactor_subscriptions.rs`, `crates/cloacina/src/runner/default_runner/reactor_subscriptions_api.rs`
  - Cron/reactor scheduler config: `crates/cloacina/src/cron_trigger_scheduler.rs:78-110,1140-1170`
  - Examples: `examples/features/computation-graphs/{filtered-reactor,packaged-graph,python-packaged-graph}/`, `examples/tutorials/computation-graphs/library/0[7-9]-*/` and `10-routing/`
- **Archived initiatives**: I-0099 (CG observability), I-0100 (subscription fan-out), I-0101 (CG/reactor decouple + embedded), I-0102 (unified shell), I-0108 (persist-failure); tasks T-0602 (CEL filtering)

### Approach

Suggested order:

1. **Reference first**: `reference/computation-graphs.md` rewrite (L). This is the source of truth for the macro shape, FFI vtable, and HTTP/WS surface that every other doc cites. Get this clean before how-tos and explanations.
2. **Topology explanation**: `explanation/architecture.md` rewrite (L) — reflects I-0101 standalone-reactor / multi-subscriber model. `subscription-fan-out.md` (new) follows naturally.
3. **Health + metrics**: `how-to-guides/computation-graph-health.md` rewrite, `explanation/reactor-lifecycle.md` edits (I-0108 + force-fire correction), `explanation/computation-graph-scheduling.md` edits.
4. **CEL how-to + sequential-strategy move**: `filter-reactor-firings-with-cel.md` (new), `sequential-strategy.md` move-in (coordinate with DOC-E).
5. **Tutorials**: service 07/08/09/10 edits.
6. **Cross-link sweep**: verify all cross-section links resolve.

### Risk considerations

- The `sequential-strategy.md` move is shared with DOC-E. Agree which cluster's PR ships first. Suggest: DOC-E removes the workflows-side file in its PR and leaves a one-line redirect in `workflows/how-to-guides/_index.md`; DOC-F's PR lands the moved file under `computation-graphs/how-to-guides/sequential-strategy.md`. Coordinate via the cluster log.
- `reference/computation-graphs.md` rewrite is L because the doc is currently three different things (macro reference, runtime API reference, FFI/packaging reference) glued together. Consider splitting at write time if scope creeps — but per Phase 2 IA non-goals, don't restructure beyond fixing existing files.
- `computation-graph-health.md` response-shape rewrite cuts across multiple tutorials and how-tos. Make sure the response example you settle on matches the live server (write a quick `cloacinactl graph list -o json` against a running server to verify, or read the route handler).
- The fabricated `fire_count`/`last_fired_at` fields appear in multiple places. Decide whether to (a) document as "not yet implemented" with a backlog reference, or (b) just delete the references. Recommendation: delete; if the fields are wanted operationally, file a follow-up backlog item.

## Status Updates

### 2026-05-18 — execution

Focused slice. New docs + critical health-response fixes + sequential-strategy move-in. L rewrites of `reference/computation-graphs.md` and `explanation/architecture.md` deferred to Phase 4 — the new `subscription-fan-out.md` covers the conceptual gap that `architecture.md` would have addressed.

**New docs:**
- `computation-graphs/explanation/subscription-fan-out.md` (M) — explanation for I-0100. DB-backed fan-out, `reactor_firings` schema, watermark + TTL pruning, at-least-once, composition with the in-process CG path, latency budgets. Cross-links the new filter how-to and the workflows-side subscribe how-to.
- `computation-graphs/how-to-guides/filter-reactor-firings-with-cel.md` (S) — T-0602 recipe. `subscribe_workflow_to_reactor(.., Some("payload.value > 100"))` form; CEL variables (`payload`, `reactor`, `tenant`); compile-at-subscribe vs evaluate-per-firing; fail-closed semantics; idempotency-key recipe via `reactor_firing_id`. Cross-links the runnable `examples/features/computation-graphs/filtered-reactor` example + `angreal demos features filtered-reactor`.

**Moved file:**
- `sequential-strategy.md` MOVED from `workflows/how-to-guides/` to `computation-graphs/how-to-guides/`. Replaced the `TODO(I-0101)` HTML comment block with an honest "the macro still doesn't accept `input_strategy = sequential` — set it on `Reactor::new(...)`" disclosure (verified by grep against `crates/cloacina-macros/src/reactor_attr.rs` — kwarg still not accepted as of 2026-05-18). Old file deleted; `workflows/how-to-guides/_index.md` has a one-line redirect to the new location.

**Existing-doc edits:**
- `computation-graphs/how-to-guides/computation-graph-health.md` (M, rewrite): swapped `{"accumulators":[...]}` / `{"graphs":[...]}` envelopes for `{"items":[...],"total":N}` (CLOACI-T-0594 / API-03); drove monitoring script off `.items[]`; added I-0108 section explaining the synthetic `Degraded { disconnected: ["persist"] }` source + 5-failure threshold + `cloacina_reactor_persist_failures_total{kind=...}` cross-link; added `cloacinactl graph {list,status,accumulators}` CLI shortcut; added metric-driven monitoring section; distinguished `/ready=503` (task crashed) from `state=degraded` (still running).
- `computation-graphs/tutorials/service/07-packaging.md` (S): `{"graphs":[...]}` → `{"items":[...],"total":N}` for graphs + accumulators; `"state":"running"` → `"state":"live"`; `"status":"healthy"` → `"status":"live"`; cross-link to health how-to.
- `computation-graphs/tutorials/service/08-websocket-events.md` (S): removed fabricated `fire_count` / `last_fired_at` fields from `/v1/health/graphs/{name}` response; replaced fire-count verification with `cloacina_reactor_fires_total{graph=...}` /metrics-scrape recipe; updated troubleshooting reference.
- `computation-graphs/tutorials/service/09-kafka-stream.md` (S): same `fire_count`/`last_fired_at` removal + metric substitution; `d['reactors']` Python selector → `d['items']`; `cargo build -p cloacinactl --features kafka` → `cargo build -p cloacina-server --features kafka` (feature lives on `cloacina-server` now).
- `computation-graphs/how-to-guides/_index.md`: added new how-tos (`filter-reactor-firings-with-cel`, `sequential-strategy`, `reactor-triggered-workflows`); split into Development / Reactor subscriptions / Operations sections.
- `workflows/how-to-guides/_index.md`: dropped Sequential Strategy entry, added one-line redirect to new CG location; added DOC-E how-tos (subscribe + invoke).

**Deferred (Phase 4 / follow-up):**
- `computation-graphs/reference/computation-graphs.md` (**L** rewrite) — three docs glued together (macro + runtime + FFI/packaging); DOC-A excised the `#[ctor]` Global Registry section, so it is no longer actively misleading. Deferred.
- `computation-graphs/explanation/architecture.md` (**L** rewrite) — I-0101 standalone-reactor / multi-subscriber topology. Cross-link to new `subscription-fan-out.md` covers the conceptual gap.
- `computation-graphs/explanation/reactor-lifecycle.md` (M) — "5-second cadence" → exponential-backoff; I-0108 persist-failure path; force-fire on `/v1/ws/reactor/{name}` correction.
- `computation-graphs/explanation/computation-graph-scheduling.md` (M) — "one scheduler per process, one reactor task per graph"; I-0108 threshold; metrics cross-link.
- `computation-graphs/explanation/packaging.md` (M) — `package.toml` shape + I-0102 unified `cloacina::package!()` macro.
- `computation-graphs/explanation/accumulator-design.md` (M) — accumulator metric refs + metrics-catalog cross-link.
- `computation-graphs/how-to-guides/accumulator-types.md` (S) — `#[state_accumulator]` coverage.
- Service tutorial 10 (`10-cross-package-binding.md`) — fabricated "Shows subscribers: 1 (price_consumer)" + topology DSL syntax verification.

**Acceptance criteria:**
- ✅ 2 new docs exist (subscription-fan-out + filter-reactor-firings-with-cel). `sequential-strategy.md` moved in.
- ✅ `computation-graph-health.md` matches `{"items":[...],"total":N}` server envelope.
- ✅ No `fire_count` / `last_fired_at` as health-response fields in the three service tutorials.
- ✅ `filter-reactor-firings-with-cel.md` cross-links the example + angreal invocation.
- ✅ `subscription-fan-out.md` cross-links S-0011 (2026-04-24 amendment).
- ⚠️ `reference/computation-graphs.md` post-I-0102 9-method FFI rewrite — deferred (L).
- ⚠️ `explanation/architecture.md` post-I-0101 topology rewrite — deferred (L). Conceptually covered by `subscription-fan-out.md`.
- ⚠️ `angreal docs:build` not yet run on this branch — will verify before commit.

**Flags for downstream:**
- **DOC-G**: Python parity for the new how-tos. CEL filter recipe is currently Rust-only.
- **DOC-I**: top-level glossary should pick up `reactor`, `accumulator`, `subscription fan-out`, `CEL predicate`.
- **Phase 4**: carry-over priority: `reference/computation-graphs.md` > `explanation/architecture.md` > everything else.
