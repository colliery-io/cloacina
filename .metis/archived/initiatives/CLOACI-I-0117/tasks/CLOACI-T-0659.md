---
id: ui-containerize-deploy-multi-stage
level: task
title: "UI containerize + deploy — multi-stage image, runtime server-URL config, compose + optional Helm"
short_code: "CLOACI-T-0659"
created_at: 2026-06-11T02:19:02.091675+00:00
updated_at: 2026-06-11T13:10:34.843784+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0651]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI containerize + deploy — multi-stage image, runtime server-URL config, compose + optional Helm

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Package the SPA as a deployable container (initiative deploy decision): a multi-stage image (Vite build → nginx static serving), **runtime** server-URL injection so one image works against any server, plus a base compose service and an optional Helm deployment. Resolves initiative OQ-5 (runtime config mechanism).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Multi-stage `ui/Dockerfile`: stage 1 (node) builds `@cloacina/client` then the Vite bundle; stage 2 (`nginx:alpine`) serves the static assets with SPA fallback (`ui/deploy/nginx.conf`). **Built + verified locally** (`docker build -f ui/Dockerfile .` → image runs, serves `/` 200, deep-link falls back to index). CI builds it in lockstep with the server (below).
- [x] **Runtime server-URL config (OQ-5) — mechanism recorded:** the entrypoint (`ui/deploy/docker-entrypoint.sh`, dropped in nginx's `/docker-entrypoint.d/`) renders `index.html` **from a committed template each start**, substituting `$CLOACINA_SERVER_URL` into `window.__CLOACINA_CONFIG__.defaultServerUrl`. Render-from-template = idempotent across restarts; empty/unset → the `/connect` form asks. One image, any server. Verified: `-e CLOACINA_SERVER_URL=…` shows up in the served HTML; `index.html` is `Cache-Control: no-store`.
- [x] Base compose `docker/docker-compose.ui.yml`: `postgres` + `server` + `ui`. The server gets `CLOACINA_CORS_ALLOWED_ORIGINS=http://localhost:8081` (the UI origin — the I-0113 / REQ-009 opt-in); the UI gets `CLOACINA_SERVER_URL=http://localhost:8080`. `compose up --build` → UI on :8081 connecting to the server on :8080. `compose config` validates.
- [x] Optional Helm chart `charts/cloacina-ui/` (deployment + service + ingress + NOTES), mirroring the `cloacina-server` chart conventions; ingress gated `enabled: false` by default. `helm lint` + `helm template` pass.
- [x] CI lockstep: new `publish-docker-ui` job in `unified_release.yml` (multi-arch buildx → `ghcr.io/<owner>/cloacina-ui`, same `<semver>`/`<major>.<minor>`/`latest` tags as the server, with a runtime-config smoke test); `publish-helm` extended to package+push the `cloacina-ui` chart and sync its version/appVersion to the tag. Both wired into the cache-prune `needs`.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
nginx serving `/`, SPA fallback to `index.html`. Runtime config via an entrypoint that writes `window.__CLOACINA_CONFIG__` (or templates nginx) from env before nginx starts — avoids rebuilding per environment. Mirror the existing cloacina Docker/Helm conventions.

### Dependencies
Blocked by CLOACI-T-0651 (needs a building app). Feeds T-0660's demo-compose profile and T-0661's e2e (which serve this image).

### Risk Considerations
CORS correctness across container origins — the compose wiring must set the server's allowed origin to the UI's. Same-origin-behind-one-ingress is a possible alternative if cross-origin proves fiddly, but the decision is separate-container/cross-origin per the initiative.

## Status Updates **[REQUIRED]**

**2026-06-11 — Implemented + verified by building/running the image.**
- `ui/Dockerfile` (multi-stage, build context = repo root): builds the sibling SDK (`clients/typescript`) then the Vite bundle, serves via `nginx:alpine`. `ui/deploy/nginx.conf` (SPA fallback, immutable `/assets`, `no-store` index), `ui/deploy/docker-entrypoint.sh` (runtime config render).
- **OQ-5 resolved:** runtime server-URL via template-render-on-start (idempotent), not a build-time bake. The read side (`ui/src/config.ts` / `index.html`) was already in place from T-0651; this task supplied the injection mechanism.
- `docker/docker-compose.ui.yml`: postgres + server + ui, CORS wired (server allows the UI origin). `.dockerignore` updated to exclude `**/node_modules/` and host `dist/` so the build context is clean.
- `charts/cloacina-ui/`: Chart.yaml, values.yaml, templates (deployment/service/ingress/_helpers/NOTES). `helm lint` + `helm template` pass.
- `.github/workflows/unified_release.yml`: `publish-docker-ui` (lockstep multi-arch image + smoke test) + extended `publish-helm` for the ui chart; both added to `prune-tag-caches.needs`. YAML validated.
- **Local verification:** `docker build -t cloacina-ui:dev -f ui/Dockerfile .` succeeded; container with `CLOACINA_SERVER_URL=http://example.test:8080` rendered that URL into the served `index.html`, `/` → 200, `/executions/abc` → SPA fallback, `index.html` `no-store`. Test container/image removed after.
- **Note:** built two new ghcr image/chart names (`cloacina-ui`, `charts/cloacina-ui`) — first publish on the next tagged release; mirrors the server's publish flow so no new failure modes expected.