---
id: fleet-secret-resolution-per
level: task
title: "Fleet secret resolution — per-execution HPKE envelope wrap to agent ephemeral key"
short_code: "CLOACI-T-0861"
created_at: 2026-07-07T11:52:26.213065+00:00
updated_at: 2026-07-07T23:58:30.422084+00:00
parent: CLOACI-I-0133
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0133
---

# Fleet secret resolution — per-execution HPKE envelope wrap to agent ephemeral key

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0133]]

## Objective **[REQUIRED]**

The fleet execution path (D-2/D-5/D-6) — the riskiest task. A packaged task running on a remote agent resolves secrets via **per-execution HPKE envelope wrap**: the agent generates a fresh ephemeral X25519 keypair per task claim and sends the public key with the claim; the server resolves the at-rest secret and HPKE-wraps it to that public key; the agent unwraps with its ephemeral private key into the `Secrets` accessor, memory-only, never persisted. On-wire ciphertext is bound to one agent + one execution.

**Dependencies:** T-0858 (accessor + leak guarantee it reuses), T-0860 (grant check). Highest risk in the initiative.

**Design refs:** [[CLOACI-I-0133]] D-2/D-5/D-6, NFR-003. Fleet path: `crates/cloacina-server/src/fleet_graph_executor.rs` + the agent claim protocol; lease model [[CLOACI-A-0008]]. HPKE = RFC 9180 (Rust `hpke` crate).

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

- [ ] **Spike first (first AC):** a standalone HPKE wrap→unwrap round-trip (server wraps to an ephemeral pubkey, agent unwraps) proving the crate + scheme before wiring into the protocol.
- [ ] Agent generates an ephemeral X25519 keypair per task claim and includes the public key in the claim request.
- [ ] Server resolves the at-rest secret (grant-checked via T-0860), HPKE-wraps to the claim's pubkey; the wrapped blob is what crosses the wire.
- [ ] Agent unwraps into the `Secrets` accessor (T-0858); plaintext is memory-only for the run and never persisted to disk/DB by the agent.
- [ ] Replay/isolation: a wrapped blob for agent A / execution 1 cannot be unwrapped by a different keypair (test).
- [ ] NFR-001 leak test extended to the fleet path: no plaintext in agent logs, the dispatch record, or the wire capture (only ciphertext).
- [ ] Embedded/in-process path is untouched (no envelope there — confirmed by test).

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

### 2026-07-07 — PARTIAL / foundation landed (commit f97638bf), NOT complete
**Done + verified (branch compiles clean; 7+5+4 tests green):**
- `crypto/envelope.rs` — RFC 9180 HPKE (`hpke` 0.12, X25519-HKDF-SHA256/HKDF-SHA256/ChaCha20Poly1305), ephemeral keygen, wrap/unwrap+AAD. 7 tests (round-trip, A≠B, tamper×2, replay/AAD, invalid key). SPIKE AC ✅.
- `security/fleet_secret.rs` — `InMemorySecretResolver` (serves `Context::secret` from an unwrapped in-mem map; no DB/KEK/persistence) + `resolve_and_wrap_secrets` (reuses T-0860 grant gate, then HPKE-wraps; AAD=execution_id/name). 5 tests incl. wire-isolation, execution-id replay reject, un-granted denied at wrap, plaintext∉ciphertext.
- `fleet/protocol.rs` — `WrappedSecret` + `wrapped_secrets` on WorkPacket/GraphWorkPacket + `ephemeral_public_key` on register (serde(default), back-compat). 4 tests.
- `cloacina-agent` — per-session ephemeral keypair, advertised at register, threaded to worker, unwraps → `set_secret_resolver` on Context before execute. Compiles (agent test binary can't LOAD locally due to a pre-existing PyO3 Python3.framework rpath issue, unrelated).

**NOT done (remaining AC — end-to-end fleet delivery does not work yet):**
- Server dispatch leaves `wrapped_secrets` empty (seams at fleet_executor.rs + fleet_graph_executor.rs). Needs the executor to have: the task's declared (`InputSlot.encrypted`)+`$secret` names, the agent's advertised pubkey (register route must persist it; `select_fleet_agent` return it), and a gated `SecretStoreResolver` (store+KEK+ResolvedGrants).
- Register route ignores `ephemeral_public_key`; graph-FFI path unwired (no Context seam there).
- No live end-to-end fleet test; NFR-001 fleet-wire assertion is unit-level (fleet_secret) not a live dispatch.

**DESIGN DEVIATION from D-5 (needs maintainer decision):** live fleet is PUSH not CLAIM → the ephemeral key is currently per-CONNECTION (advertised at register) not per-EXECUTION. True per-execution forward secrecy needs a protocol change (pre-dispatch handshake or pre-advertised key pool). Crypto/resolver unaffected — only WHEN the key is minted. **Awaiting decision before finishing the wiring.**

### 2026-07-07 (part 2) — MECHANISM COMPLETE (commit 06c113f9)
Maintainer chose TRUE per-execution → implemented the **one-time key pool** (refined D-5). Agent pre-registers a pool of one-time X25519 keys (key-id'd, `EphemeralKeyEntry`/`ephemeral_key_pool`), replenishes via heartbeat signal + `POST /v1/agent/keys`; server `ServerKeyPool` on the `AgentRecord` consumes exactly one key per secret-bearing dispatch (never reused), HPKE-wraps granted secrets to it, stamps `secret_key_id` on the WorkPacket; agent `take`s the private key, unwraps ONCE, discards. `FleetExecutor` extracts `$secret` names, consumes a key, wraps via a gated `FleetSecretResolverFactory` seam — **fail-closed** on no-grant/exhausted-pool (never plaintext, never key reuse).
**Verified (re-run myself): all 3 crates compile clean (E0063 diagnostics were stale mid-edit again); 35 cloacina-lib + 17 server tests green** incl. `two_dispatches_use_two_different_one_time_keys`, `a_key_cannot_be_consumed_twice`, `pool_exhaustion_then_replenish`, `consume_secret_key_is_one_time_then_exhausts_and_replenishes`, `fleet_pool_wrap_unwrap_end_to_end_and_exhaustion` (plaintext absent from wire, only ciphertext+key_id). **True per-execution forward secrecy proven.**
**Seams handed to T-0862** (server secrets subsystem prerequisite): no concrete `FleetSecretResolverFactory` wired into the running server — needs tenant→org_id map + package `ResolvedGrants` + the CRUD subsystem T-0862 builds. Until then a `$secret` fleet task fails CLOSED at dispatch (safe). Graph-FFI secret path needs an agent-side graph accessor (separate follow-up). **The D-2/D-5/D-6 delivery MECHANISM is done + proven; activation-in-running-server folds into T-0862.**