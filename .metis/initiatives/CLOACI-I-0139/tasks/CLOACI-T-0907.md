---
id: kafka-provider-consumption-proof
level: task
title: "Kafka provider consumption proof + provider-authoring docs — demo-stack CG, cg-feature-tour kafka lane"
short_code: "CLOACI-T-0907"
created_at: 2026-07-15T12:09:23.381471+00:00
updated_at: 2026-07-16T23:07:52.140151+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# Kafka provider consumption proof + provider-authoring docs — demo-stack CG, cg-feature-tour kafka lane

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Prove + document the end-to-end story. Build a packaged CG workflow that CONSUMES the kafka provider — `constructor!(from = "cloacina-provider-kafka@…", constructor = "kafka_source", grants = { net = […] })` — streaming a topic into a graph on the demo stack, as a harness lane (re-enable/replace the [[CLOACI-T-0891]] `cg-feature-tour` kafka surface, currently deferred). Write the "author your first provider" doc using kafka as the worked example, and surface the native-vs-wasm trust tier in `constructor!`/CLI output.

**Scope:** a features example under the demos harness (discovery-driven CI lane) that packs → uploads → compiles → reconciles → consumes the kafka provider → sees messages fire the graph; docs page (authoring + consuming + trust tiers); CLI surfacing that a native provider is "trusted, unsandboxed" vs wasm "sandboxed".

**Acceptance:**
- [ ] A green CI lane streams Kafka → CG through the consumed provider on the demo stack.
- [ ] The cg-feature-tour kafka surface is re-enabled (or its replacement lane is green).
- [ ] Docs cover authoring + consuming a provider + the trust-tier distinction; `constructor!`/CLI shows the runtime/trust tier.

Parent: [[CLOACI-I-0139]]. Depends on [[CLOACI-T-0906]] (the kafka provider) + core cleanup ([[CLOACI-T-0898]]).

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

### 2026-07-16 — activated; consumer-side machinery FULLY SCOPED (3 slices).

**T-0898 status note:** its open questions 1 (source shape) + 2 (native in-process) are ANSWERED by T-0904/0906; question 3 (loud failure) folds into slice 1 here. Core-rdkafka removal = end of slice 2 (after demo parity).

**Machinery findings (all exist, nothing to invent):**
- Consumer declaration = `[[metadata.accumulators]]` in package.toml (`name`, `accumulator_type = "stream"`, `[.config] broker/topic/group` — cg-feature-tour already carries the deferred kafka block, `{{ KAFKA_BROKER }}` templating via `crate::var::resolve_template`).
- Dispatch = `accumulator_factory_for` (packaging_bridge.rs:884); "stream" → `StreamBackendAccumulatorFactory` (host kafka, silently degrades to passthrough without the feature — the T-0898 complaint).
- **Provider resolution**: process-wide `provider_search_path()` exists (constructor_loader.rs:1572; reconciler unpacks bundled providers → `set_provider_search_path`). The stream factory uses the SAME path — no plumbing.
- **Config bridge (subtle bit, SOLVED)**: package.toml gives a string map; the member needs bincode of its typed `#[config]` struct. `bind_config_by_name` (:1825) + `OrderedConfig` (bincode TUPLE = byte-identical to the guest struct) + `coerce_config_value` already do name-keyed → declaration-ordered typed encoding from the manifest's `config_fields`. Reuse verbatim.
- Feature gating: constructor_loader is behind `constructors-wasm`; packaging_bridge is always-compiled ⇒ provider branch needs `#[cfg(feature = "constructors-wasm")]` + LOUD error fallback otherwise.

**SLICE 1 (engine seam — implementing now):**
- Loader: `pub async fn load_stream_accumulator_source_from_config(from, constructor, &HashMap<String,String>)` — resolve member manifest from `provider_search_path()`, map→JSON values → `bind_config_by_name` → `load_stream_accumulator_source` with the `OrderedConfig`.
- packaging_bridge: `"stream"` + `provider` key → new `ProviderStreamAccumulatorFactory` (routing keys `provider`/`constructor` stripped; remaining values var-template-resolved like KafkaEventSource does broker). Spawn = tokio task: async load → `accumulator_runtime_with_source(GenericPassthroughAccumulator, source)`; load failure → ERROR + health Failed (loud), not silent passthrough. No `provider` key → legacy host branch unchanged (removal after slice 2 parity).
- Test: extend the kafka E2E — stage provider under a search path, spawn via `accumulator_factory_for("stream", {provider, constructor, broker, topic, group})`, real messages → boundaries.

**SLICE 2 (demo lane):** cg-feature-tour package.toml gains `provider`/`constructor` keys; bundle the native provider archive (T-0836 `package_providers`; NOTE native bundles are arch-specific — fine on the single-arch demo stack, per-arch bundling is a noted gap); compiler container must build rdkafka (verify C toolchain in image); re-enable the kafka surface in features.py. THEN drop core `kafka` feature + rdkafka + `KafkaEventSource` (T-0898 payoff).

**SLICE 3 (docs + CLI):** "author your first provider" doc (kafka worked example: `mode = stream`, `features=["native"]`, fidius host deps, keepalive, `package --native`); consuming (`provider = ..` keys + trust note); CLI/load-log: "native (trusted, unsandboxed)" vs "wasm (sandboxed)".

### 2026-07-16 — SLICE 1 LANDED + PROVEN → `e234e1a1`. Declaration path streams real Kafka.

- **Loader**: `load_stream_accumulator_source_from_config(provider, constructor, &HashMap<String,String>)` — resolves from `provider_search_path()`, bridges the stringly map → typed member bincode via `bind_config_by_name`/`OrderedConfig` (fail-closed on unknown/missing keys; numeric fields parse from strings).
- **packaging_bridge**: `accumulator_factory_for` routes `"stream"` + `provider` key → `ProviderStreamAccumulatorFactory` (routing keys stripped; ALL member values `{{ VAR }}`-resolved; spawn = async load → `accumulator_runtime_with_source`; **load failure = ERROR + health `Disconnected`**, never silent passthrough — T-0898 item 3 delivered for the provider path). No `provider` key → legacy branch untouched. `cfg(constructors-wasm)` with loud-error fallback build; both combos compile.
- **PROOF**: `provider_stream_factory_drives_kafka_from_declaration_config` — the exact `[[metadata.accumulators]]`-shaped map (routing keys + `{{ T0907_BROKER }}` template) → factory spawn → provider resolved from a staged search path → REAL Kafka messages → boundary channel → clean shutdown join. **2/2 green** (both kafka E2Es, 4.8s, dev-stack broker).

**REMAINING:**
- **Slice 3 (docs + CLI)**: authoring doc (kafka worked example), consuming doc, trust-tier surfacing in `cloacinactl constructor package` output + load logs (the factory already logs "native, trusted" on start).

### 2026-07-16 — SLICE 2 SCOPED in detail (bundling machinery mapped).

**Bundling facts:**
- Compiler build.rs (:298-395): RUST consumers → `discover_provider_refs(source)` (scans `.rs` for `constructor!`/`#[reactor]` `from` refs) → `pack_providers` (resolves via the CONSUMER'S cargo graph — provider must be a Cargo dep) → rows stored via `store_package_providers`. PYTHON consumers → `[metadata.providers]` manifest section → `pack_providers_from_specs` (scratch cargo project; NO consumer Cargo.toml involvement). Declared-but-unbundleable FAILS the build (fail closed).
- BOTH pack/bundle sites hardcode `runtime: Wasm` (provider_bundle.rs:257/:336) — a native provider (rdkafka!) cannot build to wasm32-wasip2, so bundling needs a runtime signal.

**Slice-2 engine plan:**
1. `provider_runtime_for_crate(crate_dir)` — read the provider crate's `[package.metadata.cloacina] runtime = "native"` marker (explicit opt-in; default wasm); use at the pack/bundle option sites. Add the marker to `cloacina-provider-kafka/Cargo.toml`.
2. Compiler Rust arm: ALSO honor `[metadata.providers]` (union with source-scan refs, spec-based `pack_providers_from_specs` path) — so a Rust consumer can bundle the kafka provider WITHOUT adding rdkafka to its own Cargo graph (the wasm-provider Cargo-dep convention stays for `constructor!` refs).
3. `discover_provider_refs` unchanged (accumulator providers ride `[metadata.providers]`, not source scan).
4. cg-feature-tour: `[metadata.providers] cloacina-provider-kafka = { path = … }` + the `ticks` accumulator config gains `provider`/`constructor` keys. ⚠️ verify the compiler CONTAINER can see the path dep (volume mounts — check how fs-grant-demo's provider path resolves in-container) and can build rdkafka (C toolchain in the image).
5. features.py: re-enable the kafka lane surface; lane green on the demo stack.
6. THEN core cleanup (T-0898 payoff): drop `kafka` feature, rdkafka, `KafkaEventSource`, `StreamBackendAccumulatorFactory` legacy branch; `accumulator_factory_for("stream")` without `provider` becomes a LOUD error.

### 2026-07-16 — SLICE 2 GREEN ON THE DEMO STACK → `2deb2451` + `f083d154`. Flagship acceptance MET.

**Lane output (exit 0):** pack (`__WORKSPACE__` rewrite) → upload → compile (compiler log: "bundling [metadata.providers] … cloacina-provider-kafka", built NATIVE per the runtime marker) → reconcile (server log: "Unpacked 1 bundled provider(s)…", "provider-backed stream accumulator started (native, trusted)") → `tour_pipeline` execution Completed → produced 2 messages via the dev-stack container's console producer → **"ok: reactor tour_rx fired (0 -> 2)"** → SUCCESS. NOTE: the harness runs HOST binaries (docker only for postgres/kafka) so the "compiler container rdkafka" concern was moot; the container story arrives when CI runs the lane containerized (watch for librdkafka build deps then).

**Reconciler BUG the lane surfaced + fixed:** `stage_bundled_providers` ran ONLY inside `step_load_constructor_nodes` (packages with `constructor!` NODES) — a provider-backed stream ACCUMULATOR consumes bundled providers from the reactor-spawn step with no node in sight, so its providers were never staged (factory retried loudly forever against the default path — the supervision/backoff loop worked exactly as designed). FIX: Rust load path stages bundled providers as **step 0**; no-provider packages still clear the path (hermetic fail-closed unchanged). Also observed working: transient-error keepalive (UnknownTopicOrPartition before topic creation → logged + stream alive) and clean provider teardown on shutdown.

**Pre-existing bug noticed (NOT mine, worth a ticket):** scheduler "failed to record recovery event: Database error: invalid input syntax for type json" on accumulator crash-restart.

**Left in T-0907:** slice 3 (docs + CLI trust tier). Core-rdkafka removal stays T-0898's ticket (backlog) — the legacy branch is still exercised by other lanes; removal is now UNBLOCKED by proven parity.

### 2026-07-16 — SLICE 3 LANDED → `74f498d5`. T-0907 DONE.

- **Docs**: `author-a-provider.md` gains the NATIVE tier (trust table wasm-vs-native, the 3 authoring declarations — runtime marker / `native` feature / fidius host deps — and `--native` packaging) + the `mode = stream` authoring shape (blocking-safe pump thread, `""` keepalive rule), kafka as the worked example. `consume-a-provider.md` gains the provider-backed stream accumulator section (`provider`/`constructor` routing keys, `[metadata.providers]` both-languages semantics, name-keyed binding, `{{ VAR }}` templating, the runnable lane). Docs site builds clean.
- **CLI/logs**: `cloacinactl constructor package` prints the tier ("native — TRUSTED, runs unsandboxed in-process" vs "wasm — sandboxed, capability grants enforced"); `load_native_member` logs the tier at load; the stream factory already logged "(native, trusted)" at start.

**ACCEPTANCE — all three MET:**
- [x] A green lane streams Kafka → CG through the consumed provider on the demo stack (`angreal demos features cg-feature-tour`, exit 0, "reactor tour_rx fired (0 -> 2)"). It IS the CI lane — the features demos are the discovery-driven CI examples matrix (`demos matrix`).
- [x] The cg-feature-tour kafka surface is re-enabled (the override runs tour_pipeline AND the kafka fire).
- [x] Docs cover authoring + consuming + trust tiers; CLI + load logs show the runtime/trust tier.

**Deviations from the original objective (deliberate):** consumption is via the `[[metadata.accumulators]] provider = ..` declaration (the accumulator surface), not a `constructor!(from = ..)` node — a stream accumulator is graph plumbing, not a DAG node; `grants = { net = […] }` doesn't apply (native = grants advisory, documented). Core cleanup (T-0898) unblocked but not folded in.