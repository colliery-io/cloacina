---
id: ws-protocol-spec-ws-protocol-md
level: task
title: "WS protocol spec — ws-protocol.md, message envelope JSON schemas, protocol_version handshake"
short_code: "CLOACI-T-0644"
created_at: 2026-06-10T01:30:17.866473+00:00
updated_at: 2026-06-10T02:48:40.643911+00:00
parent: CLOACI-I-0113
blocked_by: [CLOACI-T-0642]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# WS protocol spec — ws-protocol.md, message envelope JSON schemas, protocol_version handshake

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Document the WS computation-graph protocol as a versioned spec (REQ-002): `docs/reference/ws-protocol.md` covering message types, subscription lifecycle, reconnection/ack/resync semantics, plus a JSON Schema for every message envelope variant. Embed `protocol_version` in the connect handshake so the protocol can evolve without silently breaking SDK WS wrappers.

## Acceptance Criteria **[REQUIRED]**

- [x] WS protocol doc covers every message type, subscription lifecycle, reconnect/ack/resync semantics (REQ-002) — extended the *existing* `docs/content/platform/reference/websocket-protocol.md` (accumulator + reactor were already documented; added the substrate delivery endpoint) rather than creating a parallel `ws-protocol.md`
- [x] JSON Schema (draft 2020-12) checked in for every message envelope variant at `docs/static/schemas/ws/` — delivery server/client, reactor command/response; linked from the doc and served by the docs site
- [x] `protocol_version` in the connect handshake; server now validates `hello.protocol_version` and closes 4426 on mismatch (was previously echoed in `welcome` but never validated)
- [x] Spec audited against `routes/ws.rs` + `delivery_ws.rs` — ReactorCommand (5) / ReactorResponse (5) / ServerMessage (2) / ClientMessage (2) all match code, no phantom variants; accumulator frames documented as boundary-typed (no fixed schema)
- [x] Execution-events subscription (T-0629) documented: ticket mint → `exec_events:<id>` recipient → push/ack flow, matching `cloacinactl execution follow`

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Audit `routes/ws.rs` and the envelope types (now in `cloacina-api-types` per CLOACI-T-0642); hand-write `docs/reference/ws-protocol.md` plus one JSON Schema per message variant. Adding `protocol_version` to the connect handshake is a small server change — version starts at 1, server rejects unknown majors.

### Dependencies
CLOACI-T-0642 for the shared envelope types. Runs in parallel with CLOACI-T-0643.

### Risk Considerations
A hand-written doc can drift from code later — the T-0648 coverage rule (every documented WS variant round-tripped in contract suites) is what keeps it honest. Cover the versioned envelope/ack/resync semantics from the I-0115 substrate work (T-0627) and the execution-events surface (T-0629) — don't document the agent-fleet WS protocol as client-facing; it's internal.

## Status Updates **[REQUIRED]**

**2026-06-09** — Implemented on `i0113-server-sdks`:
- Audited all three WS surfaces: `routes/ws.rs` (accumulator binary ingest, reactor JSON command/response), `routes/delivery_ws.rs` (substrate envelope), CLI `execution follow` (recipient `exec_events:<exec_id>`, `kind: execution_event`, payload = base64 JSON).
- Extended `docs/content/platform/reference/websocket-protocol.md` with the substrate delivery section: versioned envelope, welcome/push/hello/ack with examples, at-least-once + dedup-on-id, server-side resync (delivered→pending reset on reconnect + sweeper), bounded-channel backpressure (rows stay pending — contrast with accumulator drop semantics), single-subscriber takeover, execution-events flow, new close codes.
- Four JSON Schemas at `docs/static/schemas/ws/`, linked from a new Message Schemas section. Accumulator payloads documented as boundary-typed (debug=JSON/release=bincode — no fixed schema possible).
- **Server change:** `handle_client_frame` now returns a `FrameOutcome` and validates `hello.protocol_version` — mismatch closes 4426 `unsupported protocol_version`; unparsable frames keep closing 4400. CLI unaffected (it never sends `hello`). This is the loud-failure path for future protocol bumps (REQ from initiative: WS protocol versioning strategy).
- Schemas validated as JSON; server compile-checked green.
