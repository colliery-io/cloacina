---
id: python-sdk-cloacina-client-on-pypi
level: task
title: "Python SDK — cloacina-client on PyPI, sync+async shim, WS wrapper, live contract suite"
short_code: "CLOACI-T-0647"
created_at: 2026-06-10T01:30:35.665327+00:00
updated_at: 2026-06-10T01:30:35.665327+00:00
parent: CLOACI-I-0113
blocked_by: ["CLOACI-T-0643", "CLOACI-T-0644"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# Python SDK — cloacina-client on PyPI, sync+async shim, WS wrapper, live contract suite

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Ship the Python service SDK: `cloacina-client` on PyPI (import name `cloacina_client`, no collision with embedded `cloaca`), generated from `openapi.json` via pinned `openapi-python-client`, with a hand-written ergonomics shim (sync + async clients, pagination iterators, WS wrapper via `websockets`). Python 3.10+. Includes its own live-server contract suite (REQ-007).

## Acceptance Criteria **[REQUIRED]**

- [ ] Generated client under `clients/python/` from pinned `openapi-python-client`; deterministic regen with drift CI (NFR-001)
- [ ] `cloacina_client.Client(server, api_key, tenant=...)` in sync + async variants, pagination iterators (REQ-005)
- [ ] WS wrapper via `websockets` with reconnect (REQ-004)
- [ ] Packaging via uv/hatchling; `pip install cloacina-client` works against a local server; import name `cloacina_client` — no `cloaca` collision
- [ ] Live-server contract suite: every documented endpoint + ≥1 WS subscription lifecycle; Python 3.10+ tested (REQ-007)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Pinned `openapi-python-client` (httpx-based) generating into `clients/python/generated/`; hand-written shim module providing `Client`/`AsyncClient`, pagination iterators, and a `websockets`-based WS wrapper. Build with uv/hatchling. Contract suite in pytest against the composed server.

### Dependencies
Blocked by CLOACI-T-0643 (spec) and CLOACI-T-0644 (WS protocol). Runs in parallel with T-0645/T-0646.

### Risk Considerations
Generator handling of custom auth headers and pagination params needs verification against the real spec. Naming discipline: this is the *service client* — keep it cleanly separated from embedded `cloaca` in docs and package metadata (prior naming-confusion pain). PyPI publish wiring lands in T-0648; note the existing release workflow already publishes cloaca — keep package paths distinct.

## Status Updates **[REQUIRED]**

*To be added during implementation*
