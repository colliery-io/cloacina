---
id: agent-protocol-work-packet-dtos-ws
level: task
title: "Agent protocol + work packet — DTOs, WS message set, artifact-fetch REST route, OpenAPI docs"
short_code: "CLOACI-T-0631"
created_at: 2026-05-27T17:36:29.559541+00:00
updated_at: 2026-05-28T18:48:03.842566+00:00
parent: CLOACI-I-0114
blocked_by: [CLOACI-T-0627]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Agent protocol + work packet — DTOs, WS message set, artifact-fetch REST route, OpenAPI docs

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]]. Builds on the substrate envelope from [[CLOACI-T-0627]] / [[CLOACI-I-0115]].

## Objective **[REQUIRED]**

Define the agent↔server contract — as a *consumer of the substrate*, not a bespoke protocol. The crux is the **work packet**: a self-contained DTO carrying everything a DB-less agent needs to run one task (identity, inlined dependency context, artifact reference, attempt, timeout, tenant scope). Plus the WS message set (register, heartbeat/capacity, work, result), the artifact-fetch REST route, and OpenAPI/WS documentation.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Protocol DTO module — `cloacina/src/fleet/{mod,protocol}.rs`. Pure types (NFR-004): no diesel, no engine internals. Reuses the substrate envelope by riding `ServerMessage::Push` with `kind = WORK_PACKET_KIND`.
- [x] Work packet inlines all required fields including the merged dependency `Context` (eagerly resolved server-side since agents are DB-less), `ArtifactRef`, attempt, timeout, tenant.
- [x] Artifact reference carries `build_target_triple` (OQ-6 fail-closed); `AgentRegisterRequest` carries the agent's own `target_triple`. T-0632's refusal path will be `AgentOutcome::Refused { reason: TargetTripleMismatch, .. }`.
- [x] Wire message set defined — `WorkPacket` (server→agent over substrate), `AgentRegisterRequest/Response`, `AgentHeartbeatRequest/Response`, `AgentResultRequest/Response` (agent→server over REST). `register/heartbeat/work/result` covered.
- [x] Artifact-fetch REST route — `GET /v1/agent/artifact/{digest}` backed by new `WorkflowPackagesDAL::get_compiled_data_by_content_hash`. Auth'd; immutable cache headers; `x-build-target-triple` header carries the server's host triple.
- [~] OpenAPI/WS documentation — `utoipa` integration is in [[CLOACI-I-0113]]'s scope (not yet in deps). DTOs are documented in code; physical OpenAPI emission lands when I-0113 wires `utoipa` and we coordinate OQ-E (shared SDK protocol crate).
- [~] Contract-test scaffold against a live server — register + result + artifact-fetch endpoints are real; a live-server round-trip lands naturally with T-0633 (FleetExecutor end-to-end) using the same `angreal test e2e cli` harness pattern that proved out the substrate.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
DTOs in a shareable protocol module (candidate: shared with the I-0113 SDK types — settle OQ-E first). Work delivery rides the substrate outbox/relay; this task defines the `work` payload and the `result` inbound, not the delivery machinery. Artifact route reuses registry + packaged-graph fetch. Context inlining size threshold is OQ-1 (initiative) — define the packet to allow a future context-by-reference variant.

### Dependencies
[[CLOACI-T-0627]] (envelope). Coordinate with [[CLOACI-I-0113]] (OpenAPI/SDK). Feeds [[CLOACI-T-0632]] (agent) and [[CLOACI-T-0633]] (FleetExecutor).

### Risk Considerations
- Work-packet bloat from large inlined contexts (OQ-1) — design for context-by-reference fallback now even if not implemented.
- Leaking server-internal types into the DTOs (NFR-004) — keep the module diesel-free.

## Status Updates **[REQUIRED]**

### 2026-05-28 — Plan + findings

**Wire-direction split:**
- **Server → agent**: rides the substrate (S-0012/T-0626/T-0627). `ServerMessage::Push` with `kind = "agent_work"`; payload is a JSON-serialized `WorkPacket`. Agent connects to `/v1/ws/delivery/agent:<agent_id>` and gets pushes via the existing relay/sink. No bespoke server→agent transport invented.
- **Agent → server**: REST POSTs. The substrate envelope's `ClientMessage` is intentionally narrow (`Hello`, `Ack`) — extending it with register/heartbeat/result would couple substrate generality to fleet specifics. Cleaner: dedicated REST endpoints (`POST /v1/agent/register`, `POST /v1/agent/heartbeat`, `POST /v1/agent/result`) + `GET /v1/agent/artifact/{digest}`. The substrate `Ack` is still used (recipient confirms receipt of the work packet).

**Findings from code scout:**
- `workflow_packages` already has `content_hash` (digest) + `compiled_data` (cdylib bytes). Artifact-fetch resolves digest → row → bytes. No new schema needed for the artifact-fetch path itself.
- **`workflow_packages` does NOT yet track `build_target_triple` per artifact.** OQ-6 Option A (fail-closed) needs this. For v1 the server stamps its **own host triple** (`std::env::consts::ARCH/OS`-derived) as the artifact's build target — accurate when the compiler ran on the same host as the server (today's `cloacina-compiler` deployment model). Per-artifact column is a future refinement; doesn't gate T-0631.
- Auth idiom is `Extension(auth): Extension<AuthenticatedKey>` + `auth.can_access_tenant(...)`. Agent endpoints will use this; agent must present a valid API key.

**DTO home: `cloacina` lib (`crates/cloacina/src/fleet/`).** Pure types, no diesel — accessible to both `cloacina-server` and the future `cloacina-agent` crate (T-0632). OQ-E (physical share with [[CLOACI-I-0113]] SDK crate) can move them later without behavior change.

**Staging:**
- A: DTO module (`fleet/protocol.rs`) — register/heartbeat/work packet/result/refusal/artifact-ref types + protocol-version constants.
- B: In-memory `AgentRegistry` in `cloacina-server` + AppState wiring.
- C: Routes (`routes/agent.rs`): register/heartbeat (real state mutation), result (stub — T-0633 reconciliation), artifact-fetch (real via `WorkflowPackagesDAL` by content_hash, with server-host triple stamped).
- D: Unit tests for DTO round-trips + artifact-fetch contract; compile-check.

### 2026-05-28 — Complete: DTOs + routes + registry wired; `angreal test unit` ✅

**Landed:**
- **`cloacina/src/fleet/{mod,protocol}.rs`** — `AgentRegisterRequest/Response`, `AgentHeartbeatRequest/Response`, `WorkPacket`, `ArtifactRef`, `AgentResultRequest/Response`, `AgentOutcome { Success | Failure | Refused }`, `FailureClassification`, `RefusalReason`; `AGENT_PROTOCOL_VERSION`, `WORK_PACKET_KIND`, `AGENT_RECIPIENT_PREFIX`, `DEFAULT_HEARTBEAT_INTERVAL_SECONDS`. Pure types — no diesel, no engine internals (NFR-004 satisfied). Substrate consumption is documented up top: server→agent rides `ServerMessage::Push` with `kind == WORK_PACKET_KIND` to recipient `agent:<id>`; agent→server is REST POSTs.
- **`cloacina/src/dal/unified/workflow_packages.rs`** — new `get_compiled_data_by_content_hash(digest)` DAL method (both backends) returning `Option<Vec<u8>>` for the cdylib body. Filters on `build_status = 'success'`; the existing `idx_wfp_content_hash_success` partial index keeps it cheap on the hot path.
- **`cloacina-server/src/agent_registry.rs`** — `AgentRegistry` with `register`/`record_heartbeat`/`deregister`/`snapshot`/`len` over a `Mutex<HashMap>`. Per-replica roster (multi-replica liveness sweep lands in T-0634). 5 unit tests pin the invariants.
- **`cloacina-server/src/routes/agent.rs`** — handlers for `POST /v1/agent/register` (assigns id if absent, records target triple + capacity), `POST /v1/agent/heartbeat` (updates roster, 404s unknown agent), `POST /v1/agent/result` (validates protocol_version + logs; reconciliation through `TaskResultHandler` lands in T-0633), `GET /v1/agent/artifact/{digest}` (content-addressed cdylib fetch with immutable cache headers + `x-build-target-triple` from the server's own host triple). `server_host_target_triple()` documented as v1 simplification (no full Rust target triple yet).
- **AppState + router**: `agent_registry: Arc<AgentRegistry>` on AppState (both prod and test constructors); `agent_routes` merged into `/v1` alongside auth/graph/ws.

**OQ-6 resolution wired:** every `ArtifactRef` carries `build_target_triple`; every `AgentRegisterRequest` carries the agent's own. The agent (T-0632) compares and refuses with `RefusalReason::TargetTripleMismatch` rather than attempting `dlopen`. v1 is `<arch>-<os>` (doesn't distinguish glibc vs musl); per-artifact `build_target` column on `workflow_packages` is future work.

**Verification:** `angreal test unit` ✅ — 702 tests, 0 failures. 4 new DTO round-trip tests pass under `fleet::protocol::tests::*`. `angreal check crate crates/cloacina-server` ✅ — 0 errors (cloacina-server tests for `agent_registry`/`agent` are not exercised by `angreal test unit`; same project-level gap we've documented for cloacina-server tests).

**Deferred (T-0633 owns):**
- Building a `WorkPacket` from a `TaskReadyEvent` + DAL context resolution.
- Pushing the work packet via the substrate (`ServerMessage::Push` with `kind = WORK_PACKET_KIND` to `agent:<id>` recipient).
- Reconciling `POST /v1/agent/result` via `TaskResultHandler::handle_outcome` (mapping `AgentOutcome` → `Result<Context, ExecutorError>`).
- Capacity-aware selection consuming `agent_registry.snapshot()`.

**T-0631 complete.** The wire contract everything downstream depends on (T-0632 agent binary, T-0633 FleetExecutor) is now in tree, compile-verified, and unit-tested.