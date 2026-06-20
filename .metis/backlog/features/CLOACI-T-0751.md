---
id: operator-facing-manual-event
level: task
title: "Operator-facing manual event injection — UI/CLI surface over the existing reactor FireWith / accumulator-push protocol"
short_code: "CLOACI-T-0751"
created_at: 2026-06-20T02:33:01.448043+00:00
updated_at: 2026-06-20T02:33:01.448043+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Operator-facing manual event injection — UI/CLI surface over the existing reactor FireWith / accumulator-push protocol

## Origin

Surfaced during a live demo (2026-06-18/19): "verify if we can inject single
events into running computation graphs for manual operational checks."

## Verification result — POSSIBLE TODAY at the protocol level, but no operator surface

A codebase check confirms the capability **already exists** over WebSocket, but
there is no operator-friendly way to use it (no UI button, no CLI command, no
REST endpoint). An operator doing a manual check would have to hand-craft a
WebSocket JSON message containing **raw serialized boundary bytes** — not viable
as an operational check.

**What exists today:**

- **Reactor `FireWith` (inject a cache snapshot + fire):**
  `ManualCommand::FireWith(InputCache)` —
  `crates/cloacina/src/computation_graph/reactor.rs:177` and handling at
  `:639`. Exposed over WebSocket `GET /v1/ws/reactor/{name}` with
  `{"command":"fire_with","cache":{...}}` —
  `crates/cloacina-server/src/routes/ws.rs:436`. `ForceFire` (fire with current
  cache) also exists.
- **Accumulator push (feed one event in the front door):** push a serialized
  boundary to `GET /v1/ws/accumulator/{name}` —
  `crates/cloacina-server/src/routes/ws.rs:154`; the accumulator forwards to the
  reactor and the graph fires when reaction criteria are met.
- Spec-level definition: `.metis/specifications/CLOACI-S-0005/specification.md:64`.
- Protocol docs: `docs/content/reference/websocket-protocol.md:149`.

**The gaps (why it's not usable for a quick operational check):**

- **WebSocket-only** — no REST `POST .../reactor/{name}/fire` equivalent.
- **Raw bytes** — `cache` values are `Vec<u8>` of serialized boundary bytes; the
  operator must already know the wire encoding. No typed/JSON-friendly input.
- **Full-replace only** — `FireWith` does `replace_all()` on the cache; no
  partial/merge update — `reactor.rs:645`.
- **No CLI** — no `cloacinactl reactor fire` / `accumulator push` command.
- **No UI** — nothing in the web UI to fire a reactor or push a test event.
- **No manual trigger-fire** — triggers are scheduler-polled only; there is no
  `POST /triggers/{name}/fire` (triggers drive workflows, not graphs, but
  operators will expect a "run now" too).

## Objective

Give operators a safe, discoverable way to inject a single event into a running
computation graph for a manual operational check — without hand-crafting raw
WebSocket payloads — by wrapping the existing `FireWith` / accumulator-push
mechanics in a UI control and/or a CLI command, with typed input.

## Backlog Item Details

### Type
- [x] Feature — operational tooling (UI + CLI + possibly REST), over existing core

### Priority
- [x] P2 — Medium (capability exists; this is the operator-usable surface + ergonomics)

### Business Justification
- **User Value**: Operators can smoke-test a running graph ("does this reactor
  fire and produce the right output?") on demand, during an investigation or
  after a deploy.
- **Business Value**: Faster operational validation; demoable; reduces reliance
  on waiting for real event sources.
- **Effort Estimate**: M (core exists; work is surfaces + typed input + auth).

## Scope sketch (refine on pickup)

- **UI**: a "fire with test event" / "force fire" control on the reactor (CG
  health / reactor detail) view; a "push test event" control on the accumulator.
  Accept typed input and serialize server-side rather than asking for raw bytes.
- **CLI**: `cloacinactl reactor fire-with` / `force-fire`, `accumulator push`.
- **Typed input**: accept JSON / a typed payload and serialize to the boundary
  encoding server-side, so operators don't deal in `Vec<u8>`.
- **(Optional) REST**: a non-WebSocket fire endpoint for scriptability.
- **(Optional) partial cache update**: a merge mode alongside `replace_all`.
- **Auth**: reuse the existing reactor op authorization
  (`ReactorAuthPolicy` / `check_reactor_op_auth` with `ReactorOp::FireWith`,
  `crates/cloacina/src/computation_graph/registry.rs:395`).
- **Safety**: clearly mark manual fires as operator-injected (audit/log), since
  they bypass the real event source.

## Acceptance Criteria

- [ ] An operator can inject a single event into a running computation graph from
      the UI without hand-crafting raw byte payloads.
- [ ] Input is typed/JSON and serialized server-side to the boundary encoding.
- [ ] Manual fires reuse existing reactor authorization and are
      audit-logged/marked as operator-injected.
- [ ] (If included) `cloacinactl` command(s) for reactor fire / accumulator push.
- [ ] Behavior of `replace_all` vs. partial update is documented (and a merge
      mode added if in scope).

## Related work

- **CLOACI-I-0117** — web UI (the surface).
- **CLOACI-S-0005** — Reactor spec (defines `FireWith`/`ForceFire`).
- **CLOACI-T-0749** — Universal pause: pause/resume + manual-fire together give
  the full "hold it, then poke it" operational story.
- **CLOACI-T-0742** (completed) — reactors as first-class UI entities; this adds
  an *action* to that surface.

## Status Updates

*To be added during implementation*
