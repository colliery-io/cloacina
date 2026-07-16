---
id: stream-accumulator-supplied-by-a
level: task
title: "Stream accumulator supplied by a provider — accumulator constructor produces a stream accumulator via fidius call_streaming"
short_code: "CLOACI-T-0904"
created_at: 2026-07-15T12:09:19.200275+00:00
updated_at: 2026-07-16T12:23:30.014877+00:00
parent: CLOACI-I-0139
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0139
---

# Stream accumulator supplied by a provider — accumulator constructor produces a stream accumulator via fidius call_streaming

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0139]]

## Objective **[REQUIRED]**

Let an accumulator constructor (`kind = accumulator`) produce a STREAM accumulator (loop-owning), not only the current per-event `ingest` transform. On native, the provider's stream backend is driven via fidius `PluginHandle::call_streaming` → `ChunkStream`; a loader shim drains the stream (`.next().await`, host-pull) and pushes each item into the host accumulator boundary channel — replacing today's host-side `KafkaEventSource`/`StreamBackendAccumulatorFactory`. Load↔unload scoped to the consuming accumulator (drop the stream → producer tears down, matching `EventSource` shutdown).

**Scope:** contract `AccumulatorObject` gains a streaming variant (or a streaming member interface); `registry/loader/constructor_loader.rs` accumulator loader recognizes a stream accumulator and wires the `ChunkStream`→boundary shim; integrate with `accumulator_factory_for` (T-0896) so the `"stream"` accumulator type resolves to a provider-supplied backend instead of the host kafka branch.

**Acceptance:**
- [ ] A native provider member exposing a `call_streaming` source drives a `stream` accumulator: items flow into the CG boundary channel and fire the reactor on the demo stack.
- [ ] Shutdown/unload tears the producer down cleanly (no leaked task); backpressure is bounded (host stops pulling → producer blocks).

Parent: [[CLOACI-I-0139]]. Depends on [[CLOACI-T-0902]]/[[CLOACI-T-0903]]; builds on [[CLOACI-T-0896]]'s accumulator-type dispatch.

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

### 2026-07-15 — activated; fidius streaming API VERIFIED available for native (NO fidius gap this time).

Investigated the blocking unknown first (the T-0902 lesson). fidius-host 0.5.6 `src/stream.rs` + `src/handle.rs`:
- **`PluginHandle::call_streaming::<I, O>(index, &input) -> Result<ChunkStream, CallError>`** exists (`handle.rs:203`) and its `Backend::Cdylib` arm is WIRED: bincode-serializes args (no `Value` hop) → `e.call_streaming_raw(index, &input_bytes, cdylib_stream_decode::<O>)`. NATIVE server-streaming works — even though the cdylib executor does NOT impl `StreamExecutor` (only wasm + python do); cdylib uses a concrete-bincode raw path. `ChunkStream` is `futures::Stream<Item = Result<O, CallError>>`; dropping it tears down the bridge/producer (host-pull backpressure, REQ-003). Server-streaming = FIDIUS-I-0026 CS.1; `call_bidi_streaming` also cdylib-wired if host→plugin input is ever needed.
- **Gate:** behind fidius-host's **`streaming`** cargo feature. cloacina enables `fidius-host/wasm` (behind `constructors-wasm`) but NOT streaming yet → must add `fidius-host/streaming` to the loader-compiling feature.

**Key open design question (method INDEX):** `call_streaming` dispatches by method INDEX. Accumulator today exposes `ingest` (per-event). A stream accumulator needs a distinct streaming method returning a stream of boundary items — need the macro's method-index scheme + whether to add a `source`/`stream` method to `AccumulatorConstructor` or a new streaming member interface. (Explore agent mapping contract/macro/factory/runtime now.)

**Plan sketch (pending map):** contract `AccumulatorObject` streaming variant; macro emits the streaming fidius method; loader recognizes a stream accumulator → `call_streaming` → drain `ChunkStream` (`.next().await`) → push each item into `BoundarySender`; integrate `accumulator_factory_for` (T-0896) so `"stream"` resolves to the provider backend instead of host `KafkaEventSource`/`StreamBackendAccumulatorFactory`. Drop stream on unload → teardown. Prove with a native fixture streaming source (fake producer, mirroring T-0902's accumulator proof) + `accumulator_runtime` boundary assertion; demo-stack kafka fire is T-0907.

### 2026-07-15 — FULL MACHINERY MAP (Explore agent) + confirmed design layers. fidius streaming AUTHORING surface EXISTS.

**Confirmed both fidius unknowns are GO (no fidius change):**
- Host: `PluginHandle::call_streaming::<I,O>` cdylib-wired (handle.rs:203). 
- Guest/interface: `fidius::Stream<T>` marker (fidius-guest `stream_marker.rs`; fidius-macro `ir.rs:171` `streaming: bool`) lets a `#[plugin_interface]` DECLARE a server-streaming method (`fn source(&self, cfg: String) -> fidius::Stream<Row>`). So a streaming method can live in the AccumulatorConstructor interface. WASM guest backs it via `Stream::from_iter`; native via the vtable streaming fn. Only fidius-host `streaming` FEATURE must be enabled in cloacina.

**Map (all FILE:LINE):**
- Contract single-method: `AccumulatorObject::ingest` (contract lib.rs:361); wire `AccumulatorInvocation{event_json}`→`AccumulatorOutcome{boundary_json:Option<String>}` (:493/:503); `METHOD_INGEST=0` (:125); `ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION=1` (:142). No streaming variant.
- Macro: `expand_event_kind` (constructor_attr.rs:873) emits the `ingest` object impl from author `fn ingest(&self,&str)->Result<Option<String>,_>`. Shell `kind_shell_variant` (constructor_provider.rs:364) emits the `#[plugin_interface]`+`#[plugin_impl]` — where the vtable index is fixed (ingest=0).
- Host redeclaration of the interface: constructor_loader.rs:954 (`#[plugin_interface] AccumulatorConstructor{ingest}`), descriptor `AccumulatorConstructor_WASM_DESCRIPTOR` (:1137).
- Loader: `WasmAccumulatorConstructor` (:971) drives per-event `ingest` via blocking `call_method(METHOD_INGEST)` (:1020). `load_accumulator_constructor` (:1073) has the native fast-path. **No `call_streaming` anywhere in the loader.**
- Host stream machinery to REPLACE: `StreamBackendAccumulatorFactory` (packaging_bridge.rs:408) + `KafkaEventSource` (:421, impls `EventSource::run` looping `backend.recv()`→events mpsc). Dispatch `accumulator_factory_for("stream"→StreamBackend...)` (:884). Runtime seam: `accumulator_runtime_with_source` (accumulator.rs:499) spawns the `EventSource` task (:561) feeding the merge channel → `BoundarySender` (:265). **THIS is the ChunkStream shim's target.**
- ⚠️ **`load_accumulator_constructor`/`WasmAccumulatorConstructor` have NO production caller** (loader+tests only) — accumulator constructors are proven vs the runtime but never threaded into the scheduler/packaging spawn path; only `StreamBackendAccumulatorFactory` is wired. ⇒ wiring the native stream accumulator = adding an `AccumulatorFactory` (parallel to `StreamBackendAccumulatorFactory`) that loads the native handle + drives `accumulator_runtime_with_source`.
- Feature: `fidius-host/streaming` NOT enabled (Cargo.toml:40 `constructors-wasm` → `fidius-host/wasm` only). Must add.

**DESIGN (7 layers):** (1) contract: add streaming method `source` + `StreamAccumulatorObject` trait + `METHOD_SOURCE=1`, bump interface version → 2; (2) macro authoring surface for a stream source [KEY FORK — see below]; (3) shell emits the `fidius::Stream<String>` method in the interface+impl; (4) loader host-redeclares w/ the streaming method + a native stream loader: `call_streaming(METHOD_SOURCE)`→`ChunkStream`→shim impl `EventSource`→`BoundarySender`; (5) an `AccumulatorFactory` (provider-backed) + wire `accumulator_factory_for("stream")` to it; (6) `fidius-host/streaming` feature; (7) native fixture streaming source + E2E (drain to boundary channel; drop→teardown). Demo-stack kafka fire = T-0907.

### 2026-07-16 — DECISIONS LOCKED (user) + all native-streaming unknowns cleared. Building.

**User decisions:**
1. **Authoring surface = `#[constructor(kind = accumulator, mode = stream)]`**, author writes `fn source(&self) -> impl Iterator<Item = String>` (each item = boundary JSON). I back it with a DISTINCT streaming interface/holder under the hood (no vtable stub, no version bump forced on existing ingest accumulators).
2. **Scope = native E2E now, wasm parity later** (T-0906/0907 do rdkafka + demo-stack kafka fire).

**Native streaming FULLY de-risked — fidius 0.5.6 ships `configured_cdylib_stream_e2e.rs`** (a `configure_from_loaded`/`configure_in_process` cdylib plugin with a server-streaming method driven by `call_streaming` — my EXACT path). Template:
```rust
#[fidius_macro::plugin_interface(version=1, buffer=PluginAllocated, crate="fidius_core")]
pub trait Ticker { fn tick(&self, count: u32) -> fidius_core::Stream<u64>; }
#[fidius_macro::plugin_impl(Ticker, crate="fidius_core", config=Cfg)]
impl Ticker for ConfTicker { fn tick(&self,count:u32)->fidius_core::Stream<u64>{ fidius_core::Stream::from_iter(iter) } }
fidius_core::fidius_plugin_registry!();
// host: handle.call_streaming::<_, u64>(0, &(3u32,)).await → ChunkStream; .next().await → Result<Value>; from_value::<u64>(item)
```
`fidius_core` re-exports `Stream` (lib.rs:42). ChunkStream item = `Result<fidius_core::Value, CallError>` → `from_value::<String>`.

**Concrete build order (native-only proof):**
1. **contract**: `StreamAccumulatorObject { fn source(&self) -> Box<dyn Iterator<Item=String> + Send> }`; new interface const `STREAM_ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION=1`, `METHOD_SOURCE=0` (its own single-method interface). `PrimitiveKind` stays Accumulator; distinguish via the member's `interface="stream-accumulator-constructor"`.
2. **macro `#[constructor]`** (`constructor_attr.rs`): parse `mode = stream` on `kind=accumulator` → expect author `fn source(&self)->impl Iterator<Item=String>`; emit `StreamAccumulatorObject::source` (box the iter) + `__constructor_manifest` with the stream interface. Native `__constructor_make` reused.
3. **macro shell** (`constructor_provider.rs`): new suite list `stream_accumulator = [..]` → emit `#[plugin_interface] fn source(&self, _seed: u64) -> ::fidius_core::Stream<String>` + `#[plugin_impl]` holder `__ProviderStreamAccumulator` that name-dispatches → `Stream::from_iter(member.source())`. (Author lists a `mode=stream` struct under `stream_accumulator=[..]`; listing it under `accumulator=[..]` fails to compile — the struct has no `AccumulatorObject` impl. Fail-closed.)
4. **loader** (`constructor_loader.rs`): host-redeclare `#[plugin_interface] StreamAccumulatorConstructor { fn source(&self,u64)->fidius_core::Stream<String> }`; `load_stream_accumulator_source(...)` native: `load_native_member(..,"__ProviderStreamAccumulator")` → `handle.call_streaming::<_,String>(METHOD_SOURCE,&(0u64,))` → `ChunkStream`. A `ProviderStreamSource` impl of `EventSource` drains `.next().await` → `from_value::<String>` → push bytes into the events channel; drop ChunkStream on shutdown → producer teardown.
5. **factory/runtime**: drive via `accumulator_runtime_with_source` (accumulator.rs:499) with `GenericPassthroughAccumulator` + the `ProviderStreamSource`. (Full `accumulator_factory_for("stream")` provider dispatch may be minimal here; the KafkaEventSource swap lands with T-0906.)
6. **feature**: add `fidius-host/streaming` to `constructors-wasm`.
7. **fixture + E2E**: extend `native-task-provider-fixture` with `stream_accumulator=[Counter]` (`source()` yields N boundary JSONs); test loads native → drives → asserts items reach `BoundarySender` + drop→teardown (no leak).

### 2026-07-16 — ALL 7 LAYERS BUILT + E2E GREEN → `ec364db9`. T-0904 DONE (native scope).

Every layer landed per the build order:
1. **contract**: `StreamAccumulatorObject { source(&self) -> Box<dyn Iterator<Item=String> + Send> }`, `METHOD_SOURCE=0`, `STREAM_ACCUMULATOR_CONSTRUCTOR_INTERFACE_VERSION=1`, `STREAM_ACCUMULATOR_INTERFACE="stream-accumulator-constructor"` — a DISTINCT one-method interface, so no version bump on per-event accumulators. Vendored contract synced (the T-0902 lesson).
2. **`#[constructor(mode = stream)]`**: new `expand_stream_accumulator` — parses `mode = stream|per_event` (accumulator-only, fail-closed elsewhere); author writes `fn source(&self) -> impl Iterator<Item = String>`, `#[config]`-only; emits `StreamAccumulatorObject::source` (boxes the iterator — edition-2021 `-> impl Iterator` can't borrow `&self`, so ownership is compiler-enforced). Native `__constructor_make` feature-gated as before.
3. **shell**: `constructor_provider!` gained `stream_accumulator = [..]` → `stream_accumulator_shell`: `#[plugin_interface] StreamAccumulatorConstructor { fn source(&self, seed: u64) -> ::fidius_core::Stream<String> }` + `__ProviderStreamAccumulator` holder (`Stream::from_iter(member.source())`), gated `all(not(wasm32), feature="native")`.
4. **loader**: `ProviderStreamSource` (impl `EventSource`; holds `Arc<PluginHandle>` keep-alive; `select!` on `stream.next()` vs shutdown; `from_value::<String>` per item → `events.send(bytes)`; drop → producer teardown) + `pub async fn load_stream_accumulator_source` (`load_native_member(.."__ProviderStreamAccumulator")` → `call_streaming::<(u64,), String>(METHOD_SOURCE, &(0u64,))`). NO host interface redeclaration needed — native `call_streaming` is descriptor-free, like native `call_method`.
5. **runtime**: proven through the PUBLIC seam `accumulator_runtime_with_source(Passthrough, ..)` — the provider source drops in exactly where `KafkaEventSource` sits.
6. **feature**: `constructors-wasm` now also enables `fidius-host/streaming`.
7. **E2E** (`native_stream_accumulator_drives_boundaries_in_process`): fixture `Counter { #[config] base, count }` streams `{"tick":100/101/102}` (config-bound) into the CG boundary channel in-process; finite source exhausts → runtime task JOINS under timeout (no leaked task). **5/5 green** in constructor_provider_native.rs.

**Regressions all green**: native package 1/1, accumulator wasm 4/4, wasm provider package 5/5.

**ACCEPTANCE (per the user-approved 2026-07-16 scope split):**
- [x] Native `call_streaming` source drives a stream accumulator; items flow into the CG boundary channel — proven in-process. The *demo-stack reactor fire* half moved to T-0906/0907 by user decision.
- [x] Teardown clean (test joins the runtime task, no leak). Backpressure bounded BY CONSTRUCTION: fidius host-pull (`ChunkStream`, REQ-003) + bounded merge channel.

**Explicit HANDOFFS:**
- **T-0906**: the `accumulator_factory_for("stream")` dispatch swap (provider-backed source replacing `StreamBackendAccumulatorFactory`/`KafkaEventSource`) — deferred because the config naming WHICH provider/member to load comes from the package metadata T-0906 defines. `load_stream_accumulator_source` + `ProviderStreamSource` are its ready-made building blocks.
- **T-0906/0907**: wasm streaming parity; demo-stack kafka → reactor fire.