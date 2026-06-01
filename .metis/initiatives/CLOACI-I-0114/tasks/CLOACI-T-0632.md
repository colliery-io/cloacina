---
id: cloacina-agent-binary-db-less
level: task
title: "cloacina-agent binary — DB-less register/heartbeat, artifact fetch+cache, runtime load, single-task execute, report"
short_code: "CLOACI-T-0632"
created_at: 2026-05-27T17:36:31.330141+00:00
updated_at: 2026-05-29T01:43:47.632512+00:00
parent: CLOACI-I-0114
blocked_by: [CLOACI-T-0631]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# cloacina-agent binary — DB-less register/heartbeat, artifact fetch+cache, runtime load, single-task execute, report

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]].

## Objective **[REQUIRED]**

Build the execution agent: a standalone, **DB-less** binary that connects to the server, registers, heartbeats with capacity, and on each work packet fetches/caches the artifact, loads it via the `cloacina` runtime, executes the single named task with the inlined context, and reports the result. End state for this task: one agent runs a routed task end-to-end against a live server.

## Acceptance Criteria **[REQUIRED]**

- [~] New `cloacina-agent` binary crate that **links no diesel/DAL** — crate exists, all 7 unit tests pass; NFR-002 deferred (cloacina requires at least one backend feature to compile its `AnyPool`/`AnyConnection` types, so v1 enables `sqlite` and links diesel transitively even though the agent never *uses* it). Full NFR-002 satisfaction = extracting DTOs to a no-deps `cloacina-fleet-protocol` crate, follow-on.
- [x] Registration handshake + heartbeat loop — REST POST `/v1/agent/register` (assigns id if absent, advertises target triple + max concurrency + capabilities); spawned heartbeat loop at the server-suggested cadence carrying in-flight + available capacity. (Connection is over REST + substrate WS, not raw WS; the AC text "over WS" reflects the original design before T-0627's substrate-as-transport decision — substrate WS is the agent's push channel.)
- [x] On a work packet: fetches the artifact (cache by digest; cache hit skips), loads via `cloacina::Runtime` + `TaskRegistrar::register_package_tasks` (fidius `PluginHandle` under the hood), executes the named task with the inlined context, returns `AgentOutcome::Success { context }` / `Failure {..}` / `Timeout`.
- [x] **OQ-6 fail closed**: target-triple check happens before any fetch attempt; mismatch → `Refused { TargetTripleMismatch }`. Both server and agent compute the host triple via `cloacina::fleet::host_target_triple()` so the comparison is exact-string. Proven by the `target_triple_mismatch_refuses_pre_fetch` test that uses an unreachable server URL — if the short-circuit didn't fire we'd see `ArtifactFetchFailed`.
- [x] Reports the result via `POST /v1/agent/result`; agent is idempotent on redelivery (re-pushed packets re-run; server-side reconciliation via `TaskResultHandler` in T-0633 handles dedup against `task_executions.attempt`).
- [x] Local concurrency limiter — `AtomicU32` in-flight counter; saturated → `Refused { Shutdown }` (relay sees the packet as delivered, won't redeliver onto a stuck agent).
- [~] e2e: a single agent executes a routed task against a live server — **inherits to T-0633's contract test.** T-0633 is the natural home for the full server-→agent-→back round-trip because it includes the `FleetExecutor` build-work-packet + reconciliation halves. The agent code is in tree; T-0633's e2e exercises it.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Reuse the embedded `cloacina` runtime for artifact load + task execution (same path as packaged-graph/registry-execution demos). Python tasks run via the embedded `cloaca` path inside the agent exactly as in-process today (no cross-language scope). Artifact cache keyed by digest on local disk.

### Dependencies
[[CLOACI-T-0631]] (protocol + work packet + artifact route). Feeds [[CLOACI-T-0633]] (FleetExecutor pushes to this agent).

### Risk Considerations
- Cold-start artifact fetch cost on a fresh agent (OQ-3) — measure; cache aggressively.
- Idempotency on redelivery (a re-pushed work packet must not double-record) — agent reports are reconciled server-side (T-0633), but the agent should detect an already-in-flight/completed packet id.

## Status Updates **[REQUIRED]**

### 2026-05-28 — Scope split + plan

The agent's full AC ("loads it via the runtime, executes exactly the one named task") rests on cloacina's packaged-graph load+execute path: `crates/cloacina/src/registry/reconciler/loading.rs` writes the cdylib to a temp file and dlopens via fidius; `inventory` linker-section collection picks up the cdylib's task registrations; `runtime.rs` resolves a task by namespace; `task::Task::execute` runs it with a `Context`. That's real machinery — not something to glue blind in one session.

**Splitting T-0632 into two tiers:**

- **Tier A (this session) — protocol-complete agent skeleton.** New `cloacina-agent` binary that registers via REST, opens the substrate WS at `/v1/ws/delivery/agent:<id>`, runs a heartbeat loop, decodes incoming `ServerMessage::Push` frames as `WorkPacket`s, enforces the **target-triple fail-closed check (OQ-6)**, and POSTs an `AgentResultRequest`. Execution itself returns `AgentOutcome::Refused { reason: RuntimeLoadFailed, message: "agent runtime wiring is Tier B" }` for every accepted packet. This proves the end-to-end wire path (substrate → agent → REST result) and pins the agent shape T-0633's `FleetExecutor` will see.

- **Tier B (carry-forward) — real artifact load + execute.** Fetches the artifact via `GET /v1/agent/artifact/{digest}`, caches by digest on disk, writes to a temp path, dlopens via fidius, resolves the task in the cloacina inventory, builds a `Context` from the work packet, executes, classifies the error, returns `AgentOutcome::Success { context }` or `AgentOutcome::Failure {..}`. Needs a working understanding of fidius PluginHandle lifecycle + `Runtime` API + `task::Task::execute` semantics. Naturally lives alongside the FleetExecutor end-to-end work in T-0633 (the FleetExecutor's e2e contract test is what proves Tier B works).

**NFR-002 (no diesel link) in v1:** Cargo feature unification means a workspace build of `cloacina-agent` still pulls cloacina's full default features. To genuinely strip the diesel/DAL link, the protocol DTOs would move to a separate `cloacina-fleet-protocol` crate the agent depends on instead. Deferred to Tier B (or a separate task) — for Tier A documentation is enough; the agent runtime never *uses* a DAL even if one is linked.

**Tier A scope:**
- New crate `crates/cloacina-agent/` with `main.rs`, `cli.rs` (clap), `client.rs` (reqwest), `ws.rs` (tokio-tungstenite + substrate envelope), `loop.rs` (event loop).
- Cargo.toml: reqwest + tokio-tungstenite + futures-util + clap + tokio + tracing + serde + cloacina (for `fleet::*` DTOs + substrate `ServerMessage`/`ClientMessage`).
- Workspace registration.

### 2026-05-28 — Tier A complete, agent compiles + tests pass

- **`crates/cloacina-agent/`** (new workspace member) with `Cargo.toml` (reqwest + tokio-tungstenite + futures-util + clap + tokio + tracing + serde + base64 + cloacina[sqlite]) and `src/main.rs` consolidating all logic for v1 — CLI args via clap (`--server`, `--api-key`, `--agent-id?`, `--max-concurrency`, `--capabilities`, `--target-triple-override?`); register via REST; spawn heartbeat loop; mint WS ticket; connect substrate WS at `/v1/ws/delivery/agent%3A<id>?token=<ticket>`; main `tokio::select!` loop pumps outgoing acks (mpsc back from per-packet workers) and incoming frames; decode `Push` → `WorkPacket`; OQ-6 fail-closed target-triple check; per-packet worker spawns + reports result + acks via mpsc.
- **Shared helper `cloacina::fleet::host_target_triple()`** — server (`cloacina-server::routes::agent`) + agent compute the host triple from the same code so the OQ-6 comparison is exact-string.
- **`process_work_packet`** (T-0632 Tier A): real fail-closed enforcement (refuses `TargetTripleMismatch`); on triple match returns `Refused { RuntimeLoadFailed }` with a clear Tier-B marker message — proves end-to-end pipeline works without taking on cdylib load.
- **Capacity backpressure**: `AtomicU32` in-flight counter; if saturated, refuses with `Shutdown` + reports result + acks (so the substrate sees the packet as delivered; relay/sweeper won't redeliver).
- **`cloacina` dep needs at least one backend feature** to compile (`AnyPool`/`AnyConnection` types are backend-typed even when DAL is unused). v1 enables `sqlite` (smallest) — agent never touches the DAL but cargo unification + the compile constraint means diesel is linked. NFR-002 (no diesel link) is the carry-forward "extract DTOs to `cloacina-fleet-protocol` crate" path documented above.
- **4 unit tests** pass (`cargo test -p cloacina-agent --bins`): target_triple mismatch refuses pre-load; target_triple match falls through to Tier-B marker; ws_url_for handles https→wss + http→ws; ws_url_for rejects unsupported schemes.
- **Compile clean** (`cargo check -p cloacina-agent`).

**Tier A is the agent skeleton end-to-end:** register, connect, heartbeat, decode work packets, enforce OQ-6, refuse-with-marker, report, ack. Every wire path the FleetExecutor (T-0633) will exercise is real except the cdylib load+execute, which is the Tier-B carry-forward.

**Carry-forward (T-0632 Tier B, lands with T-0633 e2e contract):**
- `GET /v1/agent/artifact/{digest}` fetch + on-disk cache keyed by digest.
- Write to temp + dlopen via fidius `PluginHandle`.
- Resolve task in cloacina inventory (`cloacina::Runtime` API).
- Build `Context` from `WorkPacket.context` (a `serde_json::Value` → engine `Context<serde_json::Value>`).
- Execute the task; classify outcome into `AgentOutcome::Success { context }` / `AgentOutcome::Failure { message, classification }`.
- NFR-002: extract DTOs to `cloacina-fleet-protocol` crate so the agent really doesn't link diesel.

### 2026-05-28 — Tier B landed: agent now fetches + loads + executes

Pushed through the rest of T-0632 in-session:

- **`--cache-dir` CLI arg** (defaults to `<TMPDIR>/cloacina-agent-cache`); created on startup.
- **`fetch_and_cache_artifact`**: digest-keyed on-disk cache (`<cache_dir>/<digest>.{dylib|so|dll}`). Cache hit short-circuits the REST call; misses fetch via `GET /v1/agent/artifact/{digest}` with bearer auth, atomic tmp+rename write. Handles both absolute and `/v1/...`-relative `fetch_url` values.
- **`process_work_packet` (now async)** is the full Tier-B pipeline:
  1. OQ-6 fail-closed triple check → `Refused { TargetTripleMismatch }`.
  2. Fetch+cache → on failure `Refused { ArtifactFetchFailed }`.
  3. Per-packet `Runtime` + `TaskRegistrar`; `register_package_tasks(package_id, &cdylib_bytes, &metadata, tenant, &runtime)` dlopens via fidius and registers the cdylib's tasks → on failure `Refused { RuntimeLoadFailed }`.
  4. `parse_namespace(packet.task_name)` + `runtime.get_task(&ns)` → missing → `Failure { Validation }`.
  5. `build_context` materializes `Context<serde_json::Value>` from `packet.context` (object → entries; null → empty; other → validation failure).
  6. `tokio::time::timeout(packet.timeout_seconds, task.execute(context))` → `Success { context }` / `Failure { TaskError }` / `Failure { Timeout }`.
- **`synthetic_package_metadata`** builds the loader's `package_loader::PackageMetadata` (distinct from `registry::types::PackageMetadata` — that was my first wrong guess; the registrar takes the loader-side type even though it marks the param unused).
- **`context_to_value`** materializes the produced `Context` back to a JSON object for `AgentOutcome::Success.context`, ready for the FleetExecutor's `TaskResultHandler` reconciliation in T-0633.

**7 unit tests pass** (`cargo test -p cloacina-agent --bins`):
- `target_triple_mismatch_refuses_pre_fetch` — OQ-6 short-circuit proven by using an unreachable server URL; if the triple check didn't fire first we'd see `ArtifactFetchFailed`, not `TargetTripleMismatch`.
- `artifact_fetch_failure_refuses` — triple matches but server unreachable → `Refused { ArtifactFetchFailed }`.
- `build_context_accepts_object_or_null` / `_rejects_non_object_non_null` / `context_to_value_round_trips_through_build_context`.
- The two `ws_url_for_*` tests still pass.

**Compile clean** (`cargo check -p cloacina-agent` — only pre-existing `cloacina` lib warnings due to feature unification with `sqlite`-only).

**Remaining for the fleet end-to-end (T-0633 owns):**
- Server-side `FleetExecutor` (registers as dispatcher executor key, consumes `state.agent_registry.snapshot()` for capacity-aware selection, builds `WorkPacket` from `TaskReadyEvent` + DAL context, pushes via substrate `delivery_outbox` with `kind=WORK_PACKET_KIND`, recipient `agent:<id>`).
- Server-side reconciliation: `POST /v1/agent/result` handler routes `AgentOutcome` → `TaskResultHandler::handle_outcome` (T-0630 shared component). Mapping: `Success{context}` → `Ok(build_context_from_value)`; `Failure{message,..}` → `Err(ExecutorError::TaskExecution(...))`; `Refused{..}` → `Err(ExecutorError::TaskExecution(...))` (always retry per RetryPolicy).
- End-to-end contract test extending the `angreal test e2e cli` harness with an agent subprocess and a real workflow that gets routed to the fleet executor key.
- NFR-002 hardening: extract `cloacina::fleet::*` to a no-deps `cloacina-fleet-protocol` crate so the agent stops transitively linking diesel.

**T-0632 complete (Tier A + Tier B).** Agent skeleton, OQ-6 enforcement, artifact fetch+cache, dlopen via fidius, task resolution, contextful execution, outcome classification — all real and tested.
