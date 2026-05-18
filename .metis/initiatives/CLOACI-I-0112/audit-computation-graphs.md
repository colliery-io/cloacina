# Audit — computation-graphs/ docs (CLOACI-I-0112 Phase 2)

> Produced by parallel audit agent (general-purpose). Per-doc design.md entries; preserved verbatim for Phase 3 reference. Synthesis lives in [design.md](./design.md).

### docs/content/computation-graphs/_index.md (status: existing)
- **Category:** Index
- **Audience:** Reader landing on the CG area for the first time
- **Status delta:** edit-section (NOM-CG-01)
- **Drift / gaps found:**
  - **NOM-CG-01** `_index.md:9` — "Cloacina's reactive execution engine" (banned phrase). Rewrite as "computation graph runtime / scheduler" per S-0011 R1.
  - **NOM-CG-02** `_index.md:19` — "Applications needing reactive computation" (banned). Replace with "event-driven traversals" or similar.
  - **NOM-CG-03** `_index.md:29` — "Multi-tenant reactive workloads" (banned). Replace with "multi-tenant event-driven workloads."
  - IA-CG-01 No mention of trigger-less / embedded `invokes = computation_graph(...)` mode on the landing page — the doc only describes "Library (Embedded)" and "Service" run modes and skips the third deployment shape entirely.
- **Coverage (May 2026 batch):** I-0101 (split form mentioned in subsidiary docs but absent from landing copy); none others.
- **Sources:** `.metis/specifications/CLOACI-S-0011/specification.md`; `crates/cloacina-macros/src/computation_graph/parser.rs`.
- **Effort:** S

### docs/content/computation-graphs/explanation/_index.md (status: existing)
- **Category:** Index
- **Status delta:** verify-no-changes (toc-tree shortcode picks up new docs automatically)
- **Effort:** S

### docs/content/computation-graphs/explanation/accumulator-design.md (status: existing)
- **Category:** Explanation
- **Audience:** Engineers building custom accumulators or operators trying to understand backpressure / health semantics
- **Status delta:** edit-section
- **Drift / gaps found:**
  - **NOM-CG-04** `accumulator-design.md:211` — "the full reactive model" in a "Further Reading" caption. Reword to "the full event-driven model."
  - IA-CG-02 `accumulator-design.md:210-213` cross-links assume `[Performance Characteristics]`, `[Architecture]`, `[Packaging & FFI]` and `[Accumulator Design]` are siblings — they are, but no link points at `computation-graph-scheduling.md` (the sibling doc that overlaps heavily on health-state semantics; duplication should be tightened).
  - IA-CG-03 The doc claims `Accumulator::process(&mut self, ...) -> Option<Output>` is the trait surface — matches `crates/cloacina/src/computation_graph/accumulator.rs`. No drift.
  - IA-CG-04 Missing: no mention of the persist-failure metrics (`cloacina_accumulator_persist_failures_total`, `cloacina_accumulator_checkpoint_writes_total`, `cloacina_accumulator_emit_duration_seconds`, `cloacina_accumulator_buffer_depth`, `cloacina_accumulator_events_total`) that I-0099 emits — should at least gesture at them or link out to a metrics catalog.
- **Coverage (May 2026 batch):** None (I-0099 accumulator metrics, I-0108 persist-failure semantics absent).
- **Effort:** M

### docs/content/computation-graphs/explanation/architecture.md (status: existing)
- **Category:** Explanation
- **Audience:** Platform engineers reading to understand why CGs are separate from workflows
- **Status delta:** rewrite (substantial — I-0100 / I-0101 / I-0102 topology shift)
- **Drift / gaps found:**
  - **NOM-CG-05** `architecture.md:12` — "unsuitable for reactive workloads" (banned: "reactive workloads"). Replace with "event-driven workloads" or "real-time event correlation workloads."
  - **NOM-CG-06** `architecture.md:111` — "the correct behavior for reactive workloads" (banned phrase). Same fix.
  - IA-CG-05 `architecture.md:24-25` — section header "The Reactive Model" itself uses a banned framing. Rename to "The Event-Driven Execution Model" or "The Computation Graph Model."
  - IA-CG-06 `architecture.md:55-65` — "Process Model" still describes a 1:1 reactor-per-graph relationship: "Reactors — one per computation graph." Post-I-0101, a reactor is a standalone publisher and N CGs can subscribe to one reactor. Needs a rewrite.
  - IA-CG-07 `architecture.md:117-128` — "How Graphs Differ from Workflows" comparison table has "Trigger | Boundary arrival + reaction criteria | Cron schedule or trigger rules" — true but doesn't mention the embedded `invokes = computation_graph(...)` path where the trigger is "the workflow task that invokes the graph."
  - IA-CG-08 `architecture.md:128` — "A 'reactor' is not a 'trigger'" — this needs nuance after S-0011: a reactor IS a specialized trigger. Per S-0011 §Reactor: "A reactor is a specialized trigger that consumes accumulator boundary events." Rewrite to: "A reactor is a kind of trigger (event source), specialized for accumulator boundary consumption; it is not the same primitive as the 'trigger' label used elsewhere for cron/HTTP push triggers."
  - IA-CG-09 `architecture.md:132-139` — "Where the Graph Scheduler Lives" says Postgres-only. Needs verification — `crates/cloacina/src/cron_trigger_scheduler.rs:78-103` defines `reactor_*` config on the unified `SchedulerConfig`, which exists for both DBs. Confirm CG scheduler's storage requirements.
  - IA-CG-10 No discussion of `Runtime` registry vs scheduler registry (the dual-registry model described in `reactor-lifecycle.md`).
- **Coverage (May 2026 batch):** Partial — needs I-0101 (multi-subscriber topology), I-0100 (subscription log alongside in-process firing), I-0108 (Degraded health after persist failures).
- **Effort:** L

### docs/content/computation-graphs/explanation/computation-graph-scheduling.md (status: existing)
- **Category:** Explanation
- **Audience:** Operators / SREs trying to understand scheduler lifecycle, health, supervision, recovery
- **Status delta:** edit-section
- **Drift / gaps found:**
  - IA-CG-11 `computation-graph-scheduling.md:29` — "Single graph scheduler per graph instance" is misleading. There is one `ComputationGraphScheduler` per server process (not per graph); each graph has one *reactor* task. Reword.
  - IA-CG-12 `computation-graph-scheduling.md:60-67` — "Declarations" section says scheduler receives `ComputationGraphDeclaration` from the reconciler or code registration, accumulators, reactor declaration, tenant id. Post-I-0101 the reactor is loaded separately via `load_reactor()` before `load_graph()` — described correctly in `reactor-lifecycle.md`. This doc should be aligned.
  - IA-CG-13 `computation-graph-scheduling.md:155-162` — "Backoff and Limits" table says "Max recovery attempts | 5 | After 5 consecutive failures, the graph is marked as failed." Verified at `crates/cloacina/src/computation_graph/scheduler.rs:259` (`MAX_RECOVERY_ATTEMPTS = 5`) and `:925,1082` — correct. Add explicit cross-link to the I-0108 persist-failure threshold (`PERSIST_FAILURE_DEGRADE_THRESHOLD = 5`, separate counter from supervisor restarts) so readers don't conflate them.
  - IA-CG-14 No mention of the I-0108 `Degraded` transition triggered by 5 consecutive `persist_reactor_state` failures (`crates/cloacina/src/computation_graph/reactor.rs:840,920-961`). Should be in the "Reactor Lifecycle / Health States" table.
  - IA-CG-15 `computation-graph-scheduling.md:135-141` Sequential/Latest cross-link points at `/workflows/how-to-guides/sequential-strategy` — verify that target exists post-rename.
  - IA-CG-16 No mention of metrics (`cloacina_reactor_fires_total`, `cloacina_reactor_fire_duration_seconds`, `cloacina_supervisor_restarts_total`, `cloacina_component_health`).
- **Coverage (May 2026 batch):** Partial — I-0099 metrics, I-0108 Degraded transition both missing.
- **Effort:** M

### docs/content/computation-graphs/explanation/packaging.md (status: existing)
- **Category:** Explanation
- **Audience:** Packagers; engineers debugging FFI loading
- **Status delta:** edit-section
- **Drift / gaps found:**
  - IA-CG-17 `packaging.md:34-48` — table claims "nine methods on the `CloacinaPlugin` trait." Verified at `crates/cloacina-workflow-plugin/src/lib.rs:682-698` (METHOD_GET_TASK_METADATA=0 through METHOD_INVOKE_TRIGGERLESS_GRAPH=8 — nine methods 0..=8). Correct.
  - IA-CG-18 `packaging.md:170` — example `package.toml` includes `type = "computation_graph"` in `[package]`. Post-I-0102 (`crates/cloacina/src/registry/reconciler/loading.rs:229`), `package_type` is hard-rejected. The example's `type =` field is the older `[package]` key, also removed. Update example to current shape (no `type` in `[package]`; identity comes from `interface` + `interface_version` + `extension`).
  - IA-CG-19 `packaging.md:142,195` — references `ComputationGraphScheduler` and `@computation_graph` decorator. Confirm the decorator name post-cloacina-python split (T-0532).
  - IA-CG-20 The doc claims `LoadedGraphPlugin` uses `std::sync::Mutex`; verify at `crates/cloacina/src/computation_graph/packaging_bridge.rs`. The doc snippet is fine as illustration but should be flagged as illustrative-not-literal.
  - IA-CG-21 Missing: no mention of the unified `cloacina::package!()` shell as the *single* FFI entry point per cdylib (I-0102), and how it replaces per-macro `_ffi` emission.
- **Coverage (May 2026 batch):** Partial — I-0102 (unified shell, removal of `package_type`) needs explicit treatment.
- **Effort:** M

### docs/content/computation-graphs/explanation/performance.md (status: existing)
- **Category:** Explanation
- **Audience:** Engineers tuning throughput / latency
- **Status delta:** edit-section
- **Drift / gaps found:**
  - **NOM-CG-07** `performance.md:123` — "This is the correct behavior for reactive workloads." (banned). Replace with "for event-driven workloads."
  - IA-CG-22 Otherwise structurally sound; numbers are pinned to an explicit benchmark binary and Apple M3 reference machine. No code-truth gap detected.
  - IA-CG-23 Missing: I-0099 throughput-related metrics (`cloacina_reactor_fire_duration_seconds` histogram is what reads these numbers in production; a sentence linking the benchmark output to the live metric would help).
- **Coverage (May 2026 batch):** Touches I-0099 indirectly.
- **Effort:** S

### docs/content/computation-graphs/explanation/reactor-lifecycle.md (status: existing)
- **Category:** Explanation
- **Audience:** Operators handling load / unload / reload events; package authors writing reactors
- **Status delta:** edit-section (NOM-CG-08 plus stale CLI verb)
- **Drift / gaps found:**
  - **NOM-CG-08** `reactor-lifecycle.md:46,73` — `cloacinactl reactor force-fire <name>` appears twice. Per S-0011 R5, the noun is `graph`, not `reactor`. **Doubly drifted**: the `cloacinactl graph` CLI in `crates/cloacinactl/src/nouns/graph/mod.rs` does NOT implement a `force-fire` verb at all (only `list`, `status`, `accumulators`). Either remove the references or document where force-fire lives today (WebSocket `/v1/ws/reactor/{name}` with `{"type":"ForceFire"}` per `08-websocket-events.md:388-396`). Recommend rewrite to point at the WS endpoint.
  - IA-CG-24 `reactor-lifecycle.md:41-55` correctly captures the dual-registry model (`ComputationGraphScheduler` + `Runtime`) — keep this content.
  - IA-CG-25 `reactor-lifecycle.md:88-92` bound-subscriber guard error string verified at `crates/cloacina/src/computation_graph/scheduler.rs` — actual string may differ slightly; verify wording or mark as illustrative.
  - IA-CG-26 `reactor-lifecycle.md:125-131` restart cadence claims "5-second cadence" and `MAX_RECOVERY_ATTEMPTS = 5`. The actual `scheduler.rs:259` confirms `MAX_RECOVERY_ATTEMPTS = 5` but the supervisor uses exponential backoff (1s base, 60s max, success-reset at 60s) per `computation-graph-scheduling.md:155-162` — the "5-second cadence" claim is wrong. Backoff is exponential, not fixed at 5s.
  - IA-CG-27 No mention of I-0108 persist-failure path. The "Restart vs unload" section discusses crash restarts but not the Degraded transition triggered by persist failures (a different code path).
- **Coverage (May 2026 batch):** Covers I-0101 (declaration model); partial on I-0099/I-0108.
- **Effort:** M

### docs/content/computation-graphs/explanation/trigger-less-graphs.md (status: existing)
- **Category:** Explanation
- **Audience:** Engineers using `invokes = computation_graph(...)` in workflow tasks
- **Status delta:** verify-no-changes (light copy-edit only)
- **Drift / gaps found:**
  - IA-CG-28 `trigger-less-graphs.md:71-78` example uses `#[task(id = "score_inputs", invokes = "decision_graph")]` (string-only form). Per `crates/cloacina-macros/src/tasks.rs:56,158`, the actual macro accepts `invokes = computation_graph("name")` (call expression), not a bare string. Update the example. The corresponding how-to (`computation-graph-in-workflow.md`) already uses the correct form.
  - IA-CG-29 The doc captures the FFI method indices 7+8 (`get_triggerless_graph_metadata`, `invoke_triggerless_graph`) — matches `crates/cloacina-workflow-plugin/src/lib.rs:696-698`. Good.
- **Coverage (May 2026 batch):** Covers I-0101 (the macro split) and indirectly I-0102 (unified shell).
- **Effort:** S

### docs/content/computation-graphs/how-to-guides/_index.md (status: existing)
- **Category:** Index
- **Status delta:** edit-section
- **Drift / gaps found:**
  - IA-CG-30 Missing entry for `reactor-triggered-workflows.md` — listed in code under `computation-graphs/how-to-guides/`, exists at file path, but not linked from the index page. Add to "Operations" section.
  - IA-CG-31 The index doesn't reference the proposed new CEL-filtering how-to (see "New docs" below).
- **Effort:** S

### docs/content/computation-graphs/how-to-guides/accumulator-types.md (status: existing)
- **Category:** How-to
- **Audience:** Engineers picking an accumulator type for a new graph
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - IA-CG-32 No drift on macro syntax — `#[passthrough_accumulator]`, `#[stream_accumulator(type = "kafka", topic = ...)]`, `#[polling_accumulator(interval = "5s")]`, `#[batch_accumulator(flush_interval = "1s", max_buffer_size = 500)]` all verified.
  - IA-CG-33 `accumulator-types.md:71-95` runtime wiring example is correct.
  - IA-CG-34 Missing: no mention of `#[state_accumulator(capacity = N)]` — exists in code (`crates/cloacina-macros/src/lib.rs:239`) but absent from this how-to. Document or explicitly defer.
- **Effort:** S

### docs/content/computation-graphs/how-to-guides/computation-graph-health.md (status: existing)
- **Category:** How-to
- **Audience:** SREs monitoring running CG instances
- **Status delta:** edit-section (NOM-CG-09)
- **Drift / gaps found:**
  - **NOM-CG-09** `computation-graph-health.md:220-221` — monitoring script uses `jq -r '.reactors[]'` against `/v1/health/graphs`. Two layers of drift: (a) the field is no longer `reactors` (S-0011 R5 renamed it), and (b) the actual server response body now is `{"items": [...], "total": N}` per `crates/cloacina-server/src/routes/health_graphs.rs:127-132`, NOT `{"graphs": [...]}` as the response example claims (`computation-graph-health.md:84-107`). The response example AND the jq selector both need updating to `.items[]`.
  - IA-CG-35 `computation-graph-health.md:84-107` — example response body says `{"graphs": [...]}`. Server emits `{"items": [...], "total": N}` per `health_graphs.rs:125-131`. **All response examples in this doc need refresh** (lines 41-58 for accumulators, 84-106 for graphs).
  - IA-CG-36 The doc says reactor "fire_count" and "last_fired_at" fields exist on the per-graph health — these are NOT in the current `health_graphs.rs` response (only `name`, `health`, `accumulators`, `paused`). The fields appear in tutorial `08-websocket-events.md:228-239` too. Either the server is missing these fields or they live somewhere else; needs code-truth check (likely a doc fabrication).
  - IA-CG-37 `computation-graph-health.md:154-157` `disconnected` field — verify shape in `crates/cloacina/src/computation_graph/reactor.rs` `ReactorHealth::Degraded { disconnected: Vec<String> }` (matches line 957-959 of reactor.rs). Good.
  - IA-CG-38 Missing: I-0108 — the doc should call out that 5 consecutive persist failures move a healthy reactor into `Degraded { disconnected: ["persist"] }` (a synthetic source name), per `reactor.rs:957`. Operators seeing `disconnected: ["persist"]` need to know this isn't an accumulator outage but a DB-write streak.
  - IA-CG-39 Missing: no link to `cloacina_*` metrics anywhere — a how-to about health should at least mention `cloacina_reactor_persist_failures_total` and `cloacina_component_health` exist for Prometheus scraping.
  - IA-CG-40 Missing: no mention of `cloacinactl graph <list|status|accumulators>` — operators get the same info via CLI per `crates/cloacinactl/src/nouns/graph/mod.rs`. Should add a "CLI shortcut" section.
- **Coverage (May 2026 batch):** I-0099, I-0108 — both partially needed but missing.
- **Effort:** M

### docs/content/computation-graphs/how-to-guides/computation-graph-in-workflow.md (status: existing)
- **Category:** How-to
- **Audience:** Workflow authors who want to embed a deterministic graph as a workflow step
- **Status delta:** edit-section (NOM-CG-10/11)
- **Drift / gaps found:**
  - **NOM-CG-10** `computation-graph-in-workflow.md:20` — "the graph is the reactive quantum of work" (banned framing). Replace with "the traversal is the quantum of execution" or "the graph traversal is the unit of work" per S-0011.
  - **NOM-CG-11** `computation-graph-in-workflow.md:219` — "Tutorial 09 — Full Reactive Pipeline" — link label keeps the banned phrase. If the target tutorial title changes, this label too; flag for sweep.
  - IA-CG-41 `computation-graph-in-workflow.md:222` — references `https://github.com/colliery-io/cloacina/blob/main/.metis/specs/CLOACI-S-0011.md`. Confirm path — actual location is `.metis/specifications/CLOACI-S-0011/specification.md`. Fix.
  - IA-CG-42 Otherwise correct against `crates/cloacina-macros/src/tasks.rs:56-160` — `invokes = computation_graph("name")` and optional `post_invocation = callback` both supported.
- **Coverage (May 2026 batch):** Covers I-0101 (embedded form).
- **Effort:** S

### docs/content/computation-graphs/how-to-guides/reactor-triggered-workflows.md (status: existing)
- **Category:** How-to
- **Audience:** Operators wiring durable cross-system reactions to reactor firings
- **Status delta:** verify-no-changes (it's accurate; minor metric-table polish)
- **Drift / gaps found:**
  - IA-CG-43 `reactor-triggered-workflows.md:75-100` — Python and Rust API signatures verified against `crates/cloacina/src/runner/default_runner/reactor_subscriptions_api.rs:75-93` and `crates/cloacina-python/src/bindings/runner.rs:1250-1273`. Python uses keyword `when=`, Rust uses positional 4th arg `predicate: Option<&str>`. Matches doc.
  - IA-CG-44 Config table at lines 198-205 — `reactor_poll_interval=1s`, `reactor_poll_batch_limit=100`, `reactor_firings_prune_interval=1h`, `reactor_firings_retention=7days` all match `crates/cloacina/src/cron_trigger_scheduler.rs:100-103`.
  - IA-CG-45 Metrics table at lines 210-216 names `cloacina_reactor_firings_total` (labels: `graph`, `reactor`) and `cloacina_reactor_firings_pruned_total` — verified in `crates/cloacina/src/computation_graph/reactor.rs:827-833` and `crates/cloacina-server/src/lib.rs:1849-1856`. Correct.
  - IA-CG-46 Missing: no mention of the predicate compile errors at subscribe time being typed (`InvalidPredicate` / `ValueError`) — actually the doc says this at line 144. Good.
  - IA-CG-47 No reference to the runnable `examples/features/computation-graphs/filtered-reactor` example; should cross-link.
- **Coverage (May 2026 batch):** Covers I-0100 (subscription fan-out) and T-0602 (CEL filtering). Excellent coverage.
- **Effort:** S

### docs/content/computation-graphs/how-to-guides/when-all-criteria.md (status: existing)
- **Category:** How-to
- **Audience:** Engineers tightening reactor firing semantics to multi-source joins
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - IA-CG-48 Uses the correct post-I-0101 form: `#[reactor(criteria = when_all(orderbook, pricing))]` + `trigger = reactor("market_pipeline_reactor")`. Verified.
  - IA-CG-49 The explicit `Reactor::with_expected_sources(...)` builder call at lines 68-83 — verified against `crates/cloacina/src/computation_graph/reactor.rs`. The doc usefully calls out that without seeding, `all_set()` is wrong on first arrival (matches the implementation rationale).
  - IA-CG-50 No drift detected.
- **Coverage (May 2026 batch):** Covers I-0101 (split form) properly.
- **Effort:** S

### docs/content/computation-graphs/reference/_index.md (status: existing)
- **Category:** Index
- **Status delta:** verify-no-changes
- **Effort:** S

### docs/content/computation-graphs/reference/computation-graphs.md (status: existing)
- **Category:** Reference
- **Audience:** Engineers looking up macro args, trait methods, types, registry calls, FFI exports
- **Status delta:** rewrite (high drift — pre-I-0096 ctor path + pre-I-0102 FFI claims + version pins)
- **Drift / gaps found:**
  - **NOM-CG-12** `computation-graphs.md:9` — "Computation graphs are Cloacina's reactive data processing primitive." Banned framing ("reactive data processing"). Rewrite per S-0011 R1: "computation graph is a DAG where the traversal is the quantum of scheduling and execution."
  - **VER-CG-01** `computation-graphs.md:895-896` — `cloacina-computation-graph = { version = "0.1" }` and `cloacina-workflow-plugin = { version = "0.1", optional = true }`. Workspace is `0.6.1` per `crates/cloacina/Cargo.toml:1-3` + `Cargo.toml:1`. Update to `"0.6.1"`.
  - IA-CG-51 `computation-graphs.md:200-210` correctly captures inventory-based registration ("`inventory` mechanism replaces the pre-I-0096 `#[ctor]`-based path"). Good — that's the I-0096 surface.
  - IA-CG-52 `computation-graphs.md:818` — "Global Registry" section says "Graphs register themselves at program startup via `#[ctor]`. `#[ctor]` runs the annotated function automatically..." This contradicts line 200-210 (which says the `ctor` path was replaced by inventory in I-0096). The "Global Registry" subsection is **stale pre-I-0096 content** that needs deletion or rewrite around the inventory API.
  - IA-CG-53 `computation-graphs.md:822-855` `register_computation_graph_constructor` / `global_computation_graph_registry` / `deregister_computation_graph` symbols — verify these still exist post-I-0096 in `crates/cloacina/src/computation_graph/global_registry.rs`. If they're now internal-only or renamed, this section should reflect that.
  - IA-CG-54 `computation-graphs.md:920-924` — "The generated FFI plugin exposes three methods: `get_task_metadata`, `get_graph_metadata`, `execute_graph`." **Stale by I-0102** — the unified `cloacina::package!()` shell exposes nine methods (0–8) per `crates/cloacina-workflow-plugin/src/lib.rs:682-698`. The packaging explanation doc (`packaging.md:34-48`) has the correct 9-method table; this reference table needs to match.
  - IA-CG-55 `computation-graphs.md:928-948` — `package.toml` example includes `type = "computation_graph"` at `[package]` level. Removed in I-0102 (`crates/cloacina/src/registry/reconciler/loading.rs:215-262`). Update example to current shape with `interface`, `interface_version`, `extension`.
  - IA-CG-56 `computation-graphs.md:931-948` — `[graph]` and `[[graph.accumulators]]` tables. Verify against `crates/cloacina-workflow-plugin/src/types.rs:285-340` (`CloacinaMetadata` schema). Actual schema uses `[metadata]` + `[[metadata.accumulators]]`, NOT `[graph]` + `[[graph.accumulators]]`. The example is wrong.
  - IA-CG-57 `computation-graphs.md:594-614` `#[stream_accumulator]` arg table omits the `state` arg (line 614 lists it). Cross-reference with how-to/accumulator-types.md (which correctly documents `state`). OK.
  - IA-CG-58 `computation-graphs.md:618-635` `StateAccumulator` and `#[state_accumulator]` — verify exists at `crates/cloacina-macros/src/lib.rs:239`. Good.
  - IA-CG-59 No mention of the `#[reactor]` macro's `inventory::submit!` of `ReactorEntry`, the `Reactor` trait const-fields shape (`name`, `ACCUMULATORS`, `REACTION_MODE`), or the post-I-0101 `ReactorPackageMetadata` returned by FFI method 4. These belong in the reference.
  - IA-CG-60 No table of metrics emitted; the reference should at minimum list the `cloacina_*` metric names that arise from CG code paths and link to a metrics-catalog reference.
  - IA-CG-61 No HTTP API documentation (`/v1/health/graphs`, `/v1/health/graphs/{name}`, `/v1/health/accumulators`, `/v1/ws/accumulator/{name}`, `/v1/ws/reactor/{name}`, `/v1/auth/ws-ticket`) — these belong in a reference doc, either here or under platform/reference/http-api.md.
- **Coverage (May 2026 batch):** Partial. Needs full refresh for I-0099 (metrics), I-0101 (macro split — partially present), I-0102 (FFI method count, unified shell, manifest schema).
- **Effort:** L

### docs/content/computation-graphs/tutorials/_index.md (status: existing)
- **Category:** Index
- **Status delta:** edit-section
- **Drift / gaps found:**
  - IA-CG-62 `tutorials/_index.md:24-26` lists three service tutorials (`07-packaging`, `08-websocket-events`, `09-kafka-stream`). The directory also contains `10-cross-package-binding.md`. Add to the list.
- **Effort:** S

### docs/content/computation-graphs/tutorials/library/_index.md (status: existing)
- **Status delta:** verify-no-changes
- **Effort:** S

### docs/content/computation-graphs/tutorials/library/07-computation-graph.md (status: existing)
- **Category:** Tutorial
- **Audience:** Rust developer building their first CG locally
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - IA-CG-63 Macro shape uses the post-I-0101 form correctly: `#[reactor(name = "...", accumulators = [orderbook], criteria = when_any(orderbook))]` + `#[computation_graph(trigger = reactor("..."), graph = {...})]`. Verified. Good.
  - IA-CG-64 `07-computation-graph.md:30-32` — runnable command `angreal demos tutorials rust 07`. Verified. Good.
- **Coverage (May 2026 batch):** Covers I-0101 (split form).
- **Effort:** S

### docs/content/computation-graphs/tutorials/library/08-accumulators.md (status: existing)
- **Category:** Tutorial
- **Audience:** Rust developer adding live event ingestion to their first CG
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - IA-CG-65 `Accumulator` trait usage matches `crates/cloacina/src/computation_graph/accumulator.rs`. Channel sizes, `BoundarySender`, `accumulator_runtime`, `AccumulatorContext`, `AccumulatorRuntimeConfig::default()`, `shutdown_signal()` all verified.
  - IA-CG-66 Reactor construction with `ReactionCriteria::WhenAny` + `InputStrategy::Latest` matches code.
- **Effort:** S

### docs/content/computation-graphs/tutorials/library/09-full-pipeline.md (status: existing)
- **Category:** Tutorial
- **Audience:** Rust developer wiring multi-source CGs
- **Status delta:** edit-section (NOM-CG-13)
- **Drift / gaps found:**
  - **NOM-CG-13** `09-full-pipeline.md:7` — "you'll build a full reactive pipeline" (banned framing in tutorial body). Reword to "you'll build a full event-driven multi-source pipeline" per S-0011.
  - **NOM-CG-14** `09-full-pipeline.md:320` — "You've built a full multi-source reactive pipeline" (banned). Same fix.
  - **NOM-CG-15** `09-full-pipeline.md:title-line=5` — title "09 - Full Reactive Pipeline" carries banned framing. Retitle to "09 - Full Multi-Source Pipeline" or "09 - Full Event-Driven Pipeline."
  - IA-CG-67 Otherwise correct.
- **Effort:** S

### docs/content/computation-graphs/tutorials/library/10-routing.md (status: existing)
- **Category:** Tutorial
- **Audience:** Rust developer using enum dispatch in CGs
- **Status delta:** edit-section (NOM-CG-16)
- **Drift / gaps found:**
  - **NOM-CG-16** `10-routing.md:313` — "fully reactive, multi-source, routed pipeline" (banned). Rewrite to "fully event-driven, multi-source, routed pipeline" per S-0011.
  - IA-CG-68 Otherwise correct.
- **Effort:** S

### docs/content/computation-graphs/tutorials/service/_index.md (status: existing)
- **Status delta:** verify-no-changes
- **Effort:** S

### docs/content/computation-graphs/tutorials/service/07-packaging.md (status: existing)
- **Category:** Tutorial
- **Audience:** Engineer building and uploading a packaged CG to a running server
- **Status delta:** edit-section (VER + I-0102 followups)
- **Drift / gaps found:**
  - **VER-CG-02** `07-packaging.md:92-103` — `cloacina-computation-graph = "0.3"`, `cloacina-macros = "0.3"`, `cloacina-workflow = { version = "0.3", features = ["packaged"] }`, `cloacina-workflow-plugin = "0.3"`, `cloacina-build = "0.3"`. Workspace is `0.6.1`. Update all version pins.
  - IA-CG-69 `07-packaging.md:72` — "Reaction mode and input strategy are read from the `#[computation_graph(reaction = ..., strategy = ...)]` attributes on the macro itself." **Wrong**: the parser at `crates/cloacina-macros/src/computation_graph/parser.rs:140-198` only accepts `trigger` and `graph` — there is no `reaction`/`strategy` keyword anymore. For Rust packages, the firing rule lives on `#[reactor(criteria = when_any(...))]` (per `reactor_attr.rs`). For Python, `package.toml` `[metadata] reaction_mode` and `input_strategy` are still read (`crates/cloacina-workflow-plugin/src/types.rs:313-317`). Rewrite the note.
  - IA-CG-70 `07-packaging.md:289-318` — example response body `{"graphs": [...]}` does not match `crates/cloacina-server/src/routes/health_graphs.rs:128` which emits `{"items": [...], "total": N}`. Same drift as `computation-graph-health.md` (IA-CG-35).
  - IA-CG-71 `07-packaging.md:124-132` — uses `cloacina_workflow_plugin::package!()`. Correct per actual examples (`examples/features/computation-graphs/packaged-graph/src/lib.rs:25`).
  - IA-CG-72 Note: `health` state values shown as `"running"` (e.g. line 311-312, 314, 332) — verify against the actual `ReactorHealth` JSON tag. The reactor health states per `crates/cloacina/src/computation_graph/reactor.rs` are `Starting | Warming | Live | Degraded` (lowercased in JSON). `running` is the "no health-tx" fallback in `health_graphs.rs:114-115`. Slightly misleading; align with documented states.
- **Coverage (May 2026 batch):** Touches I-0102 (notes the removal of `package_type`); I-0101 absent (no mention of reactor decoupling in this tutorial's macro section).
- **Effort:** M

### docs/content/computation-graphs/tutorials/service/08-websocket-events.md (status: existing)
- **Category:** Tutorial
- **Audience:** Engineer pushing events into accumulators over WebSocket
- **Status delta:** edit-section
- **Drift / gaps found:**
  - IA-CG-73 `08-websocket-events.md:35` — `GET /v1/ws/accumulator/{name}` matches `crates/cloacina-server/src/lib.rs:761-762,777` (nested under `/v1`). Good.
  - IA-CG-74 `08-websocket-events.md:42-49` — auth via query parameter `?token=...` OR `Authorization: Bearer ...` header. Verify the server accepts both. (Production uses single-use WS tickets via `/v1/auth/ws-ticket` per `keys.rs:252-260` — this tutorial should at least cross-reference that path; cross-package tutorial 10 uses it.)
  - IA-CG-75 `08-websocket-events.md:228-239` example reactor-health response includes `fire_count` and `last_fired_at` fields. These are NOT in the current `health_graphs.rs:159-165` response shape (only `name`, `health`, `accumulators`, `paused`). Same fabrication as IA-CG-36.
  - IA-CG-76 `08-websocket-events.md:384-396` — Reactor WebSocket command frame table is correct against `crates/cloacina/src/computation_graph/reactor.rs::ManualCommand`. Good.
  - IA-CG-77 `08-websocket-events.md:7` — "Tutorial 07" is the prerequisite; correct.
- **Effort:** M

### docs/content/computation-graphs/tutorials/service/09-kafka-stream.md (status: existing)
- **Category:** Tutorial
- **Audience:** Engineer wiring a Kafka-backed stream accumulator into a packaged CG
- **Status delta:** edit-section (VER)
- **Drift / gaps found:**
  - **VER-CG-03** `09-kafka-stream.md:164-174` — `cloacina-computation-graph = "0.3"`, `cloacina-macros = "0.3"`, `cloacina-workflow-plugin = "0.3"`, `cloacina-build = "0.3"`. Workspace is `0.6.1`. Update.
  - IA-CG-78 `09-kafka-stream.md:275` — `if any(r['name']=='kafka_price_signal' for r in d['reactors'])` reads `d['reactors']` from `/v1/health/graphs` response. Two issues: (1) the field, if it were present, would be `graphs`, not `reactors` (S-0011 R5), but (2) the actual response shape is `{"items": [...], "total": N}` per `health_graphs.rs:127-131`. Update jq/Python selectors to `d['items']`.
  - IA-CG-79 `09-kafka-stream.md:585-590` — "Server was built without the `kafka` feature flag. Rebuild with: `cargo build -p cloacinactl --features kafka`." The `kafka` feature flag now lives on `cloacina-server` (per T-0609 making `rdkafka` a build dep of the server image). Verify the right rebuild command.
  - IA-CG-80 The doc covers passthrough, stateful, batch patterns. The batch pattern (Pattern 3, lines 422-540) uses `once_cell` and a process-global `Mutex<Vec<T>>` — this works but bypasses the proper `#[batch_accumulator]` macro path documented elsewhere. Add a "for production batch use, prefer the `#[batch_accumulator]` macro" hint.
  - IA-CG-81 No mention of T-0609 (Kafka build deps in server Dockerfile) for operators trying to run the example in a containerized environment.
- **Coverage (May 2026 batch):** Touches T-0609 indirectly; doesn't call it out.
- **Effort:** M

### docs/content/computation-graphs/tutorials/service/10-cross-package-binding.md (status: existing)
- **Category:** Tutorial
- **Audience:** Engineer building publisher/subscriber package pairs sharing a reactor
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - IA-CG-82 Already on correct version pins (`0.6.1`) per lines 88-95. Good — this tutorial was written post-T-0528.
  - IA-CG-83 Uses `cloacina::package!();` (line 146, 241) — matches the macro's own doc comment example in `crates/cloacina-workflow-plugin/src/lib.rs:83`. Note other examples use `cloacina_workflow_plugin::package!()`; both compile because `cloacina` re-exports `cloacina_workflow_plugin` (`crates/cloacina/src/lib.rs:445`). Acceptable.
  - IA-CG-84 CLI commands `cloacinactl graph list`, `cloacinactl graph status PriceReactor`, `cloacinactl package upload/delete` — `graph list/status` verified against `crates/cloacinactl/src/nouns/graph/mod.rs:39-72` (good). `package upload/delete` belong under `package` noun — verify they're real verbs.
  - IA-CG-85 `cloacinactl graph status PriceReactor` is shown returning "Shows subscribers: 1 (price_consumer)" — the current `graph status` CLI calls `GET /v1/health/graphs/{name}` per `mod.rs:62-65`, and the server response body is `{name, health, accumulators, paused}` (no `subscribers` field). This output line is fabricated or aspirational; remove or replace with a JSON example matching the actual response.
  - IA-CG-86 `/v1/auth/ws-ticket` at line 299 — verified at `crates/cloacina-server/src/lib.rs:673` (mounted under `/v1`). Good.
  - IA-CG-87 The `#[computation_graph(graph = { score: { inputs: [...], next: "..." }, publish: {} })]` syntax (lines 207-212) uses a JSON-like inputs/next form. Per `crates/cloacina-macros/src/computation_graph/parser.rs:90-100` the topology DSL is `node(inputs) -> next, node -> next` — the `score: { inputs: [...], next: "publish" }` shape is NOT the current parser surface. Fix the example or verify a separate parser path.
- **Coverage (May 2026 batch):** Excellent on I-0101 (cross-package binding by name); touches reactor-lifecycle.
- **Effort:** S

## New docs

### docs/content/computation-graphs/how-to-guides/filter-reactor-firings-with-cel.md (status: new)
- **Category:** How-to
- **Audience:** Operators tuning a noisy reactor with CEL predicates
- **Status delta:** new-write
- **Coverage (May 2026 batch):** T-0602 (CEL predicate filtering).
- **Sources:** `examples/features/computation-graphs/filtered-reactor/src/main.rs`; `crates/cloacina/src/runner/default_runner/reactor_subscriptions_api.rs:42-133`; `.metis/archived/tasks/CLOACI-T-0602/`.
- **Key topics to cover/preserve:** CEL variables (`payload`, `reactor`, `tenant`); compile-at-subscribe vs evaluate-per-firing; fail-closed semantics; idempotency-key recipe; walkthrough using the `filtered-reactor` example.
- **Effort:** S
- **Note:** If preferred, this content can live inline in `reactor-triggered-workflows.md` under a "Worked example: filtered-reactor" section instead of a separate file. Recommendation: separate how-to so it can be linked from the angreal demo (`angreal demos features filtered-reactor`).

### docs/content/computation-graphs/explanation/subscription-fan-out.md (status: new)
- **Category:** Explanation
- **Audience:** Architects understanding the I-0100 DB-backed reactor publisher → workflow subscriber model
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0100 (DB-backed subscription fan-out + durable event log), I-0099 (firing metrics), CLOACI-S-0011 (post-2026-04-24 topology amendment: reactor is a standalone publisher).
- **Sources:** `crates/cloacina/src/dal/unified/reactor_subscriptions.rs`; `crates/cloacina/src/runner/default_runner/reactor_subscriptions_api.rs`; `crates/cloacina/src/cron_trigger_scheduler.rs:78-110,1140-1170`; `.metis/specifications/CLOACI-S-0011/specification.md:Changelog`; `.metis/archived/initiatives/CLOACI-I-0100/`.
- **Key topics to cover/preserve:** Why DB-backed (durability over restart); reactor_firings row + watermark mechanics; per-subscription per-tenant scoping; TTL prune trade-off; at-least-once semantics; how this composes with in-process CG firing (both fan-outs happen for the same firing).
- **Effort:** M

### docs/content/computation-graphs/reference/metrics.md (status: new — or merged into platform/reference/metrics-catalog.md)
- **Category:** Reference
- **Audience:** SREs writing Prometheus scrapers
- **Status delta:** new-write (subset; full catalog lives at platform/reference/metrics-catalog.md per parent initiative IA changes)
- **Coverage (May 2026 batch):** I-0099 (full CG metric set), I-0108 (persist-failure counters), T-0602 (subscription metrics).
- **Sources:** `crates/cloacina-server/src/lib.rs:1940-1962` (canonical I-0099 metric list); `crates/cloacina/src/computation_graph/{reactor,accumulator,scheduler}.rs`; `crates/cloacina-server/src/routes/ws.rs:98-145`.
- **Key topics to cover/preserve:** Per-metric: name, type (counter/histogram/gauge), label set (bounded), source code path, what it means operationally. Full list per code: `cloacina_scheduler_claim_attempts_total`, `cloacina_scheduler_heartbeat_writes_total`, `cloacina_scheduler_stale_claims_swept_total`, `cloacina_supervisor_restarts_total`, `cloacina_component_health`, `cloacina_accumulator_events_total`, `cloacina_accumulator_emit_duration_seconds`, `cloacina_accumulator_buffer_depth`, `cloacina_accumulator_checkpoint_writes_total`, `cloacina_accumulator_persist_failures_total`, `cloacina_reactor_fires_total`, `cloacina_reactor_fire_duration_seconds`, `cloacina_reactor_cache_age_seconds`, `cloacina_reactor_deduped_events_total`, `cloacina_reactor_persist_failures_total`, `cloacina_reactor_firings_total`, `cloacina_reactor_firings_pruned_total`, `cloacina_ws_connections_active`, `cloacina_ws_messages_total`, `cloacina_ws_auth_failures_total`, `cloacina_context_merge_failures_total`.
- **Effort:** M
- **Note:** If platform/reference/metrics-catalog.md is the single canonical catalog (per the I-0112 IA changes), this doc may be deleted in favour of a *pointer* under `computation-graphs/reference/_index.md` linking to the CG section of the platform catalog. Recommend the pointer route to avoid duplication.

---

## Summary

- **Files reviewed:** 26 (all `.md` files under `docs/content/computation-graphs/`).
- **Drift findings (citation-tagged):**
  - **NOM-CG**: 16 (banned-phrase / stale-noun / stale-CLI-verb / stale-route-field instances).
  - **VER-CG**: 3 (stale `cloacina-* = "0.1"|"0.3"` pins).
  - **IA-CG**: 87 (response-body drift, fabricated/aspirational fields, stale FFI method count, removed `package_type`/`type` manifest keys, pre-I-0096 `#[ctor]` registry block, missing I-0099 metric coverage, missing I-0108 Degraded path, missing topology updates for I-0100/I-0101, missing fixed `force-fire` verb at the CLI, etc.).
- **New-doc proposals:** 3 (1 how-to: CEL filtering recipe; 1 explanation: subscription fan-out; 1 reference: CG metrics pointer — may collapse into the parent platform metrics-catalog).
- **High-priority files for rewrite:** `reference/computation-graphs.md` (L), `explanation/architecture.md` (L). All other edits are M or S.
- **Highest concentration of S-0011 drift:** `_index.md` (3 banned phrases in 30 lines), `architecture.md` (3 instances + topology stale), `09-full-pipeline.md` (title + body, 3 instances), `reference/computation-graphs.md` (line 9 banned framing + line 818-855 pre-I-0096 registry block + line 920-948 stale FFI/manifest).
- **CLI/HTTP integrity issues worth flagging beyond S-0011:**
  - `cloacinactl reactor force-fire` doesn't exist under either noun — the only force-fire is via WebSocket `ManualCommand::ForceFire`
  - The `/v1/health/graphs` response body is `{"items": [...], "total": N}`, not `{"graphs": [...]}` — every doc that displays a response body needs the envelope fix
  - `health` per-graph fields `fire_count` and `last_fired_at` are documented but not present in `crates/cloacina-server/src/routes/health_graphs.rs`
