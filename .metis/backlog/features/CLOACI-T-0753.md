---
id: operator-facing-accumulator
level: task
title: "Operator-facing accumulator injection (REST) — open the design phase"
short_code: "CLOACI-T-0753"
created_at: 2026-06-20T14:58:36.073841+00:00
updated_at: 2026-06-20T14:58:36.073841+00:00
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

# Operator-facing accumulator injection (REST) — open the design phase

## Origin

Surfaced 2026-06-20 while scoping the explicit-injectable-interface work
([[CLOACI-I-0128]] / [[CLOACI-S-0013]]). Checking which injectable surfaces
currently expose injection:

- **Reactor** — injection is exposed via **REST** (`POST
  /v1/health/reactors/{name}/fire`, added by CLOACI-T-0751) **and** WS
  (`/v1/ws/reactor/{name}`).
- **Accumulator** — injection is **WS-only** (`/v1/ws/accumulator/{name}`,
  `accumulator_ws`). There is **no operator-facing REST injection** for
  accumulators; the only REST accumulator route is read-only
  (`list_accumulators`).

So accumulators lack the first-class, operator-usable injection surface that
reactors got. This ticket **opens the design phase** for it. (Maintainer note on
filing: "I know how it should work already" — capture the intended design here
when picking up.)

## Objective

Provide an operator-facing REST surface to inject a single event into a named
accumulator (parallel to the reactor fire endpoint), so operators can drive an
accumulator for a manual operational check without crafting a raw WebSocket
frame — and so the typed-interface work (I-0128) has a REST surface to type and
validate against.

## Backlog Item Details

### Type
- [x] Feature — operational tooling (server REST; design phase)

### Priority
- [x] P2 — Medium (parity with reactor injection; enables typed accumulator
      injection under I-0128)

### Business Justification
- **User Value**: operators can push a test event to an accumulator from the UI/CLI
  the same way they can fire a reactor.
- **Business Value**: completes the operator-injection story across CG surfaces;
  gives I-0128 an accumulator REST surface to apply typed validation to.

## Scope sketch (to confirm against the maintainer's intended design)

- A REST endpoint to inject an event into a named accumulator (mirror the
  reactor `fire` shape: `POST /v1/health/accumulators/{name}/inject` or similar),
  reusing the existing accumulator ingest path that `accumulator_ws` drives.
- Typed/JSON input serialized server-side to the boundary encoding (don't make
  operators deal in `Vec<u8>`) — converges with [[CLOACI-S-0013]].
- Audit + auth consistent with the reactor fire endpoint (CLOACI-T-0751).
- `cloacinactl accumulator inject` CLI verb (parallel to `reactor fire`).

## Acceptance Criteria

- [ ] Documented design for operator-facing accumulator injection (endpoint
      shape, input typing, auth, audit), agreed with the maintainer.
- [ ] (Implementation, once design lands) REST endpoint + optional CLI verb that
      injects a single event into a named accumulator; typed input; authorized +
      audited; tests via angreal lanes.

## Related work
- **CLOACI-I-0128 / CLOACI-S-0013** — typed injectable interfaces; this provides
  the accumulator REST surface that work types + validates.
- **CLOACI-T-0751** — the reactor analogue (REST fire + CLI), to mirror.
- **CLOACI-S-0002 / S-0004** — ComputationBoundary & Accumulator definitions.

## Status Updates

*To be added during implementation*
