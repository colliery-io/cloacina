---
id: typescript-sdk-cloacina-client
level: task
title: "TypeScript SDK — @cloacina/client codegen + ergonomics shim, browser+node WS, live contract suite"
short_code: "CLOACI-T-0645"
created_at: 2026-06-10T01:30:23.418860+00:00
updated_at: 2026-06-10T03:57:06.750621+00:00
parent: CLOACI-I-0113
blocked_by: [CLOACI-T-0643, CLOACI-T-0644]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# TypeScript SDK — @cloacina/client codegen + ergonomics shim, browser+node WS, live contract suite

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Ship `@cloacina/client` on npm: generated from `openapi.json` with a language-native generator, hand-written ergonomics shim, typed WS wrapper, dual ESM/CJS, usable from modern browsers and Node 20+. **This is the spine task** — the follow-on UI initiative consumes this package as its data layer, so it starts as soon as the spec lands and is not queued behind the Rust or Python work. Includes its own live-server contract suite (REQ-007).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Generator chosen and pinned: `openapi-typescript@7.13.0` + `openapi-fetch@0.13.5` (decision + rationale recorded in the initiative); generated code at `clients/typescript/generated/types.ts`; deterministic regen with drift CI (`npm run check:generated` in the ci.yml spec-check job) (NFR-001)
- [x] Ergonomics shim: bearer-injection middleware, default-tenant scoping + `withTenant`, `iterateExecutions` pagination async iterator, typed `CloacinaApiError` (REQ-005)
- [x] Typed WS wrapper — platform WebSocket (browser + Node ≥21 native; injectable impl for Node 20) — reconnect with exponential backoff, dedup-on-id, ack-after-yield, high-water-mark backpressure (REQ-004)
- [x] Dual ESM/CJS build (tsup + d.ts); node smoke via the vitest contract suite; CORS verified live against a CORS-enabled server (preflight allow-origin/methods/headers, disallowed-origin rejection, actual-request header) — full browser-engine (playwright) smoke rides the T-0648 harness
- [x] README states API-key-in-browser is the v1 first-party-UI auth story, with the ws-ticket mitigation (REQ-010)
- [x] Live-server contract suite: 27 tests, every documented endpoint + 2 WS lifecycle tests (welcome/hello/ack + 4426 version rejection), 27/27 across 4 consecutive runs on a fresh server (REQ-007)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Generate with both candidate generators (`openapi-typescript-codegen`, `openapi-typescript` + fetch wrapper) against the real spec, pick one, pin it, record the decision in the initiative. Hand-written shim in `clients/typescript/src/` over the generated subtree. Dual ESM/CJS via `tsup`. Contract suite (vitest or similar) runs against the composed server; browser smoke via playwright or a headless harness hitting a CORS-enabled server.

### Dependencies
Blocked by CLOACI-T-0643 (spec) and CLOACI-T-0644 (WS protocol). Runs in parallel with T-0646/T-0647 — do not queue behind them; the UI initiative is waiting on this package.

### Risk Considerations
Generator output quality is unknown until the real spec exists — that's why the pick happens here, not earlier. Browser WebSocket API cannot set custom headers, so WS auth needs a query-param or subprotocol token — coordinate the server-side accommodation with whatever T-0644 specifies for the handshake. Per-SDK release-workflow wiring (npm publish) lands in T-0648, not here.

## Status Updates **[REQUIRED]**

**2026-06-09/10** — Implementation on `i0113-server-sdks`:
- **Generator decision (recorded in initiative):** `openapi-typescript@7.13.0` + `openapi-fetch@0.13.5`. Bake-off against the real spec; `openapi-typescript-codegen` rejected as unmaintained.
- **Package:** `clients/typescript/` — `@cloacina/client` 0.7.0 (lockstep), `generated/types.ts` committed, hand shim in `src/`: `CloacinaClient` (bearer-injection middleware, default-tenant scoping + `withTenant`, helpers for all endpoints, `CloacinaApiError` with status+code, `iterateExecutions` pagination), `src/ws.ts` (`subscribeDelivery`/`followExecutionEvents`: per-connection ws-ticket mint, hello v1 handshake, dedup-on-id, ack-after-yield, exponential-backoff reconnect, 4426 terminal, high-water-mark backpressure via connection shed). Dual ESM/CJS via tsup + d.ts; typecheck clean; build green. Node ≥21 native WebSocket, injectable impl for Node 20.
- **Spec fix found by codegen:** `IntoParams` query structs defaulted to `parameter_in = Path` + required — `limit`/`offset` appeared as required *path* params in the spec. Fixed with `into_params(parameter_in = Query)` on both query structs; spec + types regenerated.
- **README:** REQ-010 browser-auth story stated explicitly; CORS opt-in documented; regen + contract-test instructions.
- **Drift gate:** `npm run check:generated` (regen + diff); CI wiring pending below.
- **Live contract run (local postgres compose + fresh DB, server on :18080):** 26/27 passing — 27 tests covering every endpoint + 2 WS lifecycle tests (welcome / hello-v1-accepted / ack-idempotent; hello-v99 → close 4426 verified live).
- **Contract findings:**
  1. `DELETE workflow` is idempotent (200 even when absent) — adopted as the documented contract (annotation description updated, test asserts 200/"deleted").
  2. **Bug filed CLOACI-T-0649:** server ignores the dbname in `--database-url` (`build_postgres_url` overwrites it with the hardcoded "cloacina"); discovered because the dedicated `sdk_contract` DB stayed completely empty.
  3. One remaining flake: `GET /v1/auth/keys` transiently returned `{items: [], total: 0}` right after tenant creation — suspected pooled-connection `search_path` leakage from create_tenant, BUT Docker crashed during the repro so the signal is contaminated. Re-verify on healthy Docker before concluding.
- **Interrupted:** Docker crashed mid-repro; user restarting. Resume: re-run contract suite (fresh `cloacina` DB; server `--bootstrap-key clk_sdk_contract_test_key_0001 --bind 127.0.0.1:18080 --home /tmp/cloacina-sdk-home2`), chase the list_keys flake, add TS drift check to the CI spec-check job, commit.

**2026-06-10 (resolution)** — Docker restored; flake root-caused and FIXED:
- **Root cause (multi-tenancy bug, pre-existing on main):** `DatabaseAdmin::create_tenant` ran `SET search_path TO <tenant>, public` (session-level) on a **shared admin-pool** connection and returned it to the pool dirty — every later checkout of that connection wrote/read the tenant schema instead of public. DB inspection showed api_keys rows scattered across tenant schemas (one run's global key landed in *another run's* tenant schema). Explains the roaming list_keys/revoke failures.
- **Fix in `admin.rs`:** `SET LOCAL` (dies with the transaction) + defensive `SET search_path TO public` before the connection returns to the pool. Also fixed an adjacent swallowed-error bug: the tenant-setup transaction result was bound to `_`, so failed schema/migration setup still returned Ok(credentials).
- **Verification:** 27/27 contract tests across 4 consecutive runs on a fresh DB (previously failed 1-2 per run with roaming failures). The TS contract suite is now the regression test for this bug.
- **CORS live verification (server `--cors-allowed-origins http://localhost:5173` on :18081):** preflight returns configured origin/methods/headers; disallowed origin gets no allow-origin; actual GET carries the header.
- **CI:** spec-check job now also runs `npm ci && check:generated && typecheck && build` for `clients/typescript`; `api` paths filter extended; actionlint clean. `package-lock.json` committed for `npm ci`.