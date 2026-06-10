---
id: typescript-sdk-cloacina-client
level: task
title: "TypeScript SDK — @cloacina/client codegen + ergonomics shim, browser+node WS, live contract suite"
short_code: "CLOACI-T-0645"
created_at: 2026-06-10T01:30:23.418860+00:00
updated_at: 2026-06-10T01:30:23.418860+00:00
parent: CLOACI-I-0113
blocked_by: ["CLOACI-T-0643", "CLOACI-T-0644"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0113
---

# TypeScript SDK — @cloacina/client codegen + ergonomics shim, browser+node WS, live contract suite

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0113]]

## Objective **[REQUIRED]**

Ship `@cloacina/client` on npm: generated from `openapi.json` with a language-native generator, hand-written ergonomics shim, typed WS wrapper, dual ESM/CJS, usable from modern browsers and Node 20+. **This is the spine task** — the follow-on UI initiative consumes this package as its data layer, so it starts as soon as the spec lands and is not queued behind the Rust or Python work. Includes its own live-server contract suite (REQ-007).

## Acceptance Criteria **[REQUIRED]**

- [ ] Generator chosen and pinned (`openapi-typescript-codegen` vs `openapi-typescript` — decision recorded in the initiative); generated code under `clients/typescript/generated/`; deterministic regen with drift CI (NFR-001)
- [ ] Ergonomics shim: auth header injection, tenant scoping, pagination async iterator (REQ-005)
- [ ] Typed WS wrapper — native WebSocket in browser, `ws` in node — with reconnect + backpressure (REQ-004)
- [ ] Dual ESM/CJS build; node smoke + browser smoke against a CORS-enabled server
- [ ] Docs state API-key-in-browser is the v1 first-party-UI auth story (REQ-010)
- [ ] Live-server contract suite: every documented endpoint + ≥1 WS subscription lifecycle exercised against a real server (REQ-007)

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Generate with both candidate generators (`openapi-typescript-codegen`, `openapi-typescript` + fetch wrapper) against the real spec, pick one, pin it, record the decision in the initiative. Hand-written shim in `clients/typescript/src/` over the generated subtree. Dual ESM/CJS via `tsup`. Contract suite (vitest or similar) runs against the composed server; browser smoke via playwright or a headless harness hitting a CORS-enabled server.

### Dependencies
Blocked by CLOACI-T-0643 (spec) and CLOACI-T-0644 (WS protocol). Runs in parallel with T-0646/T-0647 — do not queue behind them; the UI initiative is waiting on this package.

### Risk Considerations
Generator output quality is unknown until the real spec exists — that's why the pick happens here, not earlier. Browser WebSocket API cannot set custom headers, so WS auth needs a query-param or subprotocol token — coordinate the server-side accommodation with whatever T-0644 specifies for the handshake. Per-SDK release-workflow wiring (npm publish) lands in T-0648, not here.

## Status Updates **[REQUIRED]**

*To be added during implementation*
