---
id: ws-protocol-spec-ws-protocol-md
level: task
title: "WS protocol spec — ws-protocol.md, message envelope JSON schemas, protocol_version handshake"
short_code: "CLOACI-T-0644"
created_at: 2026-06-10T01:30:17.866473+00:00
updated_at: 2026-06-10T01:30:17.866473+00:00
parent: CLOACI-I-0113
blocked_by: ["CLOACI-T-0642"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# WS protocol spec — ws-protocol.md, message envelope JSON schemas, protocol_version handshake

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Document the WS computation-graph protocol as a versioned spec (REQ-002): `docs/reference/ws-protocol.md` covering message types, subscription lifecycle, reconnection/ack/resync semantics, plus a JSON Schema for every message envelope variant. Embed `protocol_version` in the connect handshake so the protocol can evolve without silently breaking SDK WS wrappers.

## Acceptance Criteria **[REQUIRED]**

- [ ] `docs/reference/ws-protocol.md` documents every message type, subscription lifecycle, reconnect/ack/resync semantics (REQ-002)
- [ ] JSON Schema checked in for every message envelope variant, alongside the doc
- [ ] `protocol_version` field in the connect handshake; server validates it
- [ ] Spec audited against `routes/ws.rs` — every variant in code is documented, no phantom variants
- [ ] Execution-events subscription surface (from CLOACI-T-0629) covered

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Audit `routes/ws.rs` and the envelope types (now in `cloacina-api-types` per CLOACI-T-0642); hand-write `docs/reference/ws-protocol.md` plus one JSON Schema per message variant. Adding `protocol_version` to the connect handshake is a small server change — version starts at 1, server rejects unknown majors.

### Dependencies
CLOACI-T-0642 for the shared envelope types. Runs in parallel with CLOACI-T-0643.

### Risk Considerations
A hand-written doc can drift from code later — the T-0648 coverage rule (every documented WS variant round-tripped in contract suites) is what keeps it honest. Cover the versioned envelope/ack/resync semantics from the I-0115 substrate work (T-0627) and the execution-events surface (T-0629) — don't document the agent-fleet WS protocol as client-facing; it's internal.

## Status Updates **[REQUIRED]**

*To be added during implementation*
