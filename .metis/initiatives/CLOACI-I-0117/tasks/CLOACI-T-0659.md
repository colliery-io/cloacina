---
id: ui-containerize-deploy-multi-stage
level: task
title: "UI containerize + deploy — multi-stage image, runtime server-URL config, compose + optional Helm"
short_code: "CLOACI-T-0659"
created_at: 2026-06-11T02:19:02.091675+00:00
updated_at: 2026-06-11T02:19:02.091675+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0651"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI containerize + deploy — multi-stage image, runtime server-URL config, compose + optional Helm

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Package the SPA as a deployable container (initiative deploy decision): a multi-stage image (Vite build → nginx static serving), **runtime** server-URL injection so one image works against any server, plus a base compose service and an optional Helm deployment. Resolves initiative OQ-5 (runtime config mechanism).

## Acceptance Criteria **[REQUIRED]**

- [ ] Multi-stage `Dockerfile`: Vite build → static assets served by nginx. Built in CI, version-locked to the server release (NFR-004).
- [ ] **Runtime server-URL config (OQ-5)**: target server URL injected at container start (env → generated `config.js` or nginx-templated) — the bundle is server-agnostic; record the chosen mechanism.
- [ ] Base compose: a `ui` service alongside `server`, with the server configured `CLOACINA_CORS_ALLOWED_ORIGINS` for the UI origin (exercises the I-0113 CORS opt-in). `compose up` → reach the UI and connect to the server.
- [ ] Optional Helm: a `ui` deployment + service (+ ingress), gated like cloacina's other optional components.
- [ ] Image published by CI in lockstep with the server image.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
nginx serving `/`, SPA fallback to `index.html`. Runtime config via an entrypoint that writes `window.__CLOACINA_CONFIG__` (or templates nginx) from env before nginx starts — avoids rebuilding per environment. Mirror the existing cloacina Docker/Helm conventions.

### Dependencies
Blocked by CLOACI-T-0651 (needs a building app). Feeds T-0660's demo-compose profile and T-0661's e2e (which serve this image).

### Risk Considerations
CORS correctness across container origins — the compose wiring must set the server's allowed origin to the UI's. Same-origin-behind-one-ingress is a possible alternative if cross-origin proves fiddly, but the decision is separate-container/cross-origin per the initiative.

## Status Updates **[REQUIRED]**

*To be added during implementation*
