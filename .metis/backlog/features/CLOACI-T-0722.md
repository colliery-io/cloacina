---
id: execute-computation-graphs-on-the
level: task
title: "Execute computation graphs on the agent fleet (whole-graph dispatch per reactor firing)"
short_code: "CLOACI-T-0722"
created_at: 2026-06-17T05:20:37.681473+00:00
updated_at: 2026-07-05T21:27:54.352089+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Execute computation graphs on the agent fleet (whole-graph dispatch per reactor firing)

## Objective

Let computation graphs execute on the execution-agent fleet instead of only
in-process in the server's reactor. **Whole-graph dispatch**: when a reactor
fires, ship that one firing (the `InputCache` snapshot + the CG package digest)
to an agent, which runs the compiled graph's `execute_graph` and reports back;
the reactor awaits the result. Accumulators + reactor state stay server-side —
only the compute leaves. (Per-*node* dispatch is explicitly out of scope — it's a
macro/scheduler rewrite; see "Rejected".)

## Type / Priority
- [x] Feature — get CG compute off the server onto the fleet (scaling + parity
  with task workflows, which already run on the fleet via CLOACI-T-0633/0716).
- [ ] P2 (scaling/architecture; CGs work in-process today).

## Background (verified)
- A CG runs in-process as a single compiled closure: `reactor.rs` does
  `let result = (graph)(snapshot).await;` at the **two** fire sites (Latest
  ~700, Sequential ~753). `graph: CompiledGraphFn` is an `Arc<dyn Fn(InputCache)
  -> Future<GraphResult>>` (cloacina-computation-graph/src/lib.rs:247).
- For a **packaged** CG that closure is already an FFI shim into the cdylib's
  `execute_graph` (`build_ffi_triggerless_graph_fn`,
  `registry/loader/ffi_triggerless_graph.rs:49`; plugin shell `execute_graph` at
  cloacina-workflow-plugin/src/lib.rs:334/742). So the cdylib is the dispatchable
  artifact — same shape as task packages (content_hash digest).
- `InputCache` is serde + bincode (`cloacina-computation-graph/src/lib.rs:79-85`),
  so the snapshot is shippable.
- Task→fleet pattern to mirror: `TaskExecutor` trait (dispatcher/traits.rs:144) →
  `FleetExecutor` (cloacina-server/src/fleet_executor.rs) builds a `WorkPacket`,
  enqueues to delivery_outbox, awaits via the fleet coordinator rendezvous; the
  agent fetches the artifact by digest, runs, reports.
- Verdict: whole-graph is dispatchable (artifact + serializable input + a result
  the reactor only logs). Per-node is not (no node identity/FFI; reactor-coupled
  state).

## Plan
1. **`GraphExecutor` seam (cloacina).** Define a trait (mirror `TaskExecutor`):
   `async fn execute_graph(&self, GraphFireEvent) -> Result<GraphResult, _>` where
   `GraphFireEvent { graph_name, package_digest, tenant_id, input_cache_bytes }`.
   The reactor holds an `Arc<dyn GraphExecutor>`; the two fire sites call it
   instead of `(graph)(snapshot)`. Default impl = `InProcessGraphExecutor` that
   wraps the existing `(graph)(snapshot)` (zero behaviour change when no fleet).
2. **Inject at construction.** `ComputationGraphScheduler` / `Reactor::new`
   (scheduler.rs:476) take the executor; server selects fleet vs in-process from
   `--default-executor` (same switch as tasks).
3. **`FleetGraphExecutor` (cloacina-server).** Resolve the CG package's active
   `content_hash` (reuse `get_active_dispatch_for_package`), build a
   `GraphWorkPacket { graph_name, artifact, input_cache_bytes, language,
   tenant_id }`, enqueue to delivery_outbox (new kind `graph_packet`), register a
   coordinator rendezvous, await the agent's `GraphResult`. Re-use the fleet
   coordinator + agent-selection from FleetExecutor.
4. **Agent graph path (cloacina-agent).** `process_graph_packet`: fetch cdylib by
   digest (Rust) or source archive (Python), load, deserialize `InputCache`,
   call the plugin's `execute_graph(graph_name, snapshot)` (Rust) or the
   `PythonRuntime::load_cg_package` path (Python — CLOACI-T-0716 added the PyO3
   runtime to the agent), report Completed/Error. Cache the loaded package by
   digest (as tasks do).
5. **Wire format.** GraphResult outputs are opaque `Any` the reactor discards
   today — the agent need only report success/failure (+ error string). Keep the
   packet lean.
6. Build/deploy/verify a packaged CG (e.g. `market_maker` / `demo_kafka_graph`)
   fires on an agent, and `demo_py_graph` (Python) fires on an agent.

## Open questions (resolve during build)
- **Reactor-subscriber composition.** Some CGs compose multiple `graph_fn`s
  in-process (`build_dispatcher_graph_fn`, scheduler.rs:202 — reactor subscribers
  fan a boundary to N graphs). Whole-graph fleet dispatch fits a single packaged
  graph cleanly; decide whether subscriber-fan-out dispatches each subscriber
  graph as its own fleet firing or stays in-process. Likely: dispatch each
  packaged graph; keep the in-process composition for reactor-only graphs.
- **Result re-injection.** Today outputs are discarded; if a future CG feeds
  outputs back to accumulators, the rendezvous must carry them. Out of scope now.
- **Latency / snapshot size.** Each firing ships the full `InputCache` snapshot.
  Fine for the demo; note if large.
- **Back-pressure / no-agent.** If no agent has capacity, the firing must either
  wait (reactor stalls) or fall back to in-process. Decide a policy (mirror the
  FleetExecutor dispatch timeout). **Recommended default: in-process fallback**
  after a dispatch timeout, so the reactor never deadlocks when the fleet is
  empty/unavailable; log a warning when falling back. (Pure-stall risks wedging
  the reactor hot path — avoid as the default.)

## Rejected: per-node dispatch
Each node on an agent would need the `#[computation_graph]` macro to stop
inlining and emit per-node FFI exports, a `NodeExecutor`/`NodeReadyEvent`/topo
scheduler state machine, and full per-node `InputCache` marshalling + output
re-injection. Research-level rewrite, low marginal payoff over whole-graph.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] With `--default-executor fleet`, a packaged Rust CG firing runs on an agent
      (verified via agent logs + graph fires advancing) and the reactor records
      the result; in-process behaviour is unchanged without the fleet.
- [ ] A Python-packaged CG (`demo_py_graph`) fires on an agent.
- [ ] No regression: reactor health/throughput + accumulator advance still work;
      the demo Graphs view still shows throughput.
- [ ] A sane no-capacity policy (documented) — reactor doesn't deadlock when no
      agent is available.

## Status Updates
- 2026-06-17: Filed with plan (plan-first, per request). Investigation confirmed
  whole-graph dispatch is feasible by mirroring the task→fleet path at the
  reactor's `(graph)(snapshot)` seam; per-node rejected. Awaiting go-ahead to
  build (large multi-crate change + agent rebuild).
- 2026-06-17: Parked as a **later** item (not on the current branch). Plan is
  self-contained; recorded recommended no-agent policy (in-process fallback after
  dispatch timeout). Stays in `#phase/backlog` until scheduled.

### 2026-07-05 — implementation (branch feat/t0722-cg-fleet-dispatch); steps 1–5 code-complete, all 3 crates check clean
Followed the plan with two verified simplifications: (a) NO reactor-side package plumbing — the fleet executor resolves graph→package at fire time via `find_package_for_surface("reactor", name)` (the T-0773 seam); (b) the SERVER pre-converts the snapshot to the FFI cache shape (extracted `input_cache_to_ffi_cache` from the bridge), keeping the agent lean and the packet JSON-friendly.
- **Seam (cloacina)**: `computation_graph::graph_executor` — `GraphExecutor` trait + `GraphFireEvent { graph_name, tenant_id, snapshot, in_process: CompiledGraphFn }` + `InProcessGraphExecutor` default. Reactor holds the executor (`with_graph_executor`); BOTH fire sites (Latest + Sequential) route through it. Scheduler stores a swappable default + `set_graph_executor`, applied at both spawn sites (incl. restart).
- **Wire**: `GRAPH_PACKET_KIND="agent_graph"` + `GraphWorkPacket { firing_id, graph_name, cache, artifact, timeout, tenant_id }`. The coordinator rendezvous reused verbatim — `/v1/agent/result` is a pure uuid→result forward; graph firings key it with a fresh `firing_id`.
- **Server** (`fleet_graph_executor.rs`): package resolve → digest/language → `select_fleet_agent` (per-target aware) → enqueue → await. **Fallback policy (the AC's no-capacity answer)**: PRE-dispatch failures (embedded graph, Python package, no agent, enqueue error, agent Refused) → in-process via the carried closure with a warning — the reactor NEVER deadlocks; POST-dispatch failures (agent Failure, result timeout) → GraphResult::error WITHOUT a local re-run (no double execution). Wired under `use_fleet` → `scheduler.set_graph_executor(..)`.
- **Agent**: `GRAPH_PACKET_KIND` arm + `process_graph_packet` (fetch cdylib by digest, per-digest `LoadedGraphPlugin` cache, FFI `execute_graph(cache)`, outputs → `context.outputs`); saturation → Refused (server falls back in-process).
- **SCOPE CALL — Python CGs stay in-process in v1** (deviation from AC #2, flagged): a py CG's compiled fn is a live PyObject built from module registrations, not a shippable artifact; dispatching it needs the agent to replicate the py CG load path per package — a real follow-on, not a packet field. The fleet executor detects `language == python` and cleanly falls back in-process (logged). Rust packaged CGs — the scaling case — dispatch fully.
Remaining: tests, live demo verification (Rust CG fires on an agent), PR.
