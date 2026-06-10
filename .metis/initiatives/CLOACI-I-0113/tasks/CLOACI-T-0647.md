---
id: python-sdk-cloacina-client-on-pypi
level: task
title: "Python SDK — cloacina-client on PyPI, sync+async shim, WS wrapper, live contract suite"
short_code: "CLOACI-T-0647"
created_at: 2026-06-10T01:30:35.665327+00:00
updated_at: 2026-06-10T10:25:51.300152+00:00
parent: CLOACI-I-0113
blocked_by: [CLOACI-T-0643, CLOACI-T-0644]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# Python SDK — cloacina-client on PyPI, sync+async shim, WS wrapper, live contract suite

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Ship the Python service SDK: `cloacina-client` on PyPI (import name `cloacina_client`, no collision with embedded `cloaca`), generated from `openapi.json` via pinned `openapi-python-client`, with a hand-written ergonomics shim (sync + async clients, pagination iterators, WS wrapper via `websockets`). Python 3.10+. Includes its own live-server contract suite (REQ-007).

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Generated client vendored at `clients/python/src/cloacina_client/_generated/` from **pinned `openapi-python-client@0.29.0`** (regen command in pyproject + README; CI drift wiring in T-0648's matrix — note: 0.21.x is broken with current typer/click, hence the newer pin)
- [x] `cloacina_client.Client(server, api_key=..., tenant=...)` + `AsyncClient` with the full endpoint surface, `iterate_executions` pagination (sync generator + async iterator), `CloacinaApiError` with status+code (REQ-005)
- [x] `_ws.py` via `websockets`: fresh ticket per connect, hello v1, dedup-on-id, ack-after-yield, exponential-backoff reconnect, 4426 → `ProtocolVersionError` (REQ-004)
- [x] Packaging via uv/hatchling: `uv build` produces sdist+wheel; wheel installed into an isolated env and verified against a live server; import name `cloacina_client`, README explicitly distinguishes the service client from embedded `cloaca`
- [x] Live-server contract suite: 18 pytest tests, every documented endpoint + WS lifecycle (idle-connect + hello-v99 → close 4426), 18/18 twice on a fresh server; local venv is Python 3.12, `requires-python = ">=3.10"` with 3.10-compatible code (full version matrix rides T-0648 CI)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Pinned `openapi-python-client` (httpx-based) generating into `clients/python/generated/`; hand-written shim module providing `Client`/`AsyncClient`, pagination iterators, and a `websockets`-based WS wrapper. Build with uv/hatchling. Contract suite in pytest against the composed server.

### Dependencies
Blocked by CLOACI-T-0643 (spec) and CLOACI-T-0644 (WS protocol). Runs in parallel with T-0645/T-0646.

### Risk Considerations
Generator handling of custom auth headers and pagination params needs verification against the real spec. Naming discipline: this is the *service client* — keep it cleanly separated from embedded `cloaca` in docs and package metadata (prior naming-confusion pain). PyPI publish wiring lands in T-0648; note the existing release workflow already publishes cloaca — keep package paths distinct.

## Status Updates **[REQUIRED]**

**2026-06-10** — Implemented on `i0113-server-sdks`:
- Generator: `openapi-python-client@0.29.0` via uvx (0.21.x fails with a typer/click incompatibility — pinned the working version instead). Output vendored *inside* the package at `src/cloacina_client/_generated` (`--meta none`, `package_name_override: _generated`) so one wheel ships everything.
- Shim: `_client.py` — `_Base` holds the generated `AuthenticatedClient`; `Client` (sync) covers the full surface; `AsyncClient` covers the async-relevant surface + WS entry points; `_unwrap` raises `CloacinaApiError` from `*_detailed` responses; `/health` (schema-less in spec) parsed from raw content. `_ws.py` mirrors the TS/Rust delivery consumers.
- Suite findings: generated `ExecuteRequest.context` is `Any | Unset` (no wrapper model — adjusted shim); everything else round-tripped cleanly on the first live run except `/health` parsing. 18/18 twice against a fresh server.
- Packaging: `uv build` → sdist + wheel; wheel verified in an isolated `uv run --no-project` env against the live server. `[dependency-groups] dev` for pytest/pytest-asyncio.
- PyPI publish wiring deliberately deferred to T-0648 (release task), as scoped.
