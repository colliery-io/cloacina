---
id: embedded-ui-serve-the-cloacina-ui
level: initiative
title: "Embedded UI — serve the @cloacina/ui SPA from the cloacina-server binary"
short_code: "CLOACI-I-0130"
created_at: 2026-06-21T13:40:31.071091+00:00
updated_at: 2026-07-06T10:18:35.332192+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: embedded-ui-serve-the-cloacina-ui
---

# Embedded UI — serve the @cloacina/ui SPA from the cloacina-server binary

## Context

Today the web UI (`@cloacina/ui`) ships as a **separate deployment artifact**: a
React 18 + Vite + TypeScript SPA (Mantine 7, TanStack Query over `@cloacina/client`,
React Router 7) built to static files and served by its **own Nginx container** on
a separate port (`docker/docker-compose.ui.yml`). The browser loads the SPA from
the UI origin and makes **cross-origin** REST calls to `cloacina-server` (axum), so
the deployment needs CORS opt-in (`CLOACINA_CORS_ALLOWED_ORIGINS`) and a startup
`sed`/entrypoint step that injects `CLOACINA_SERVER_URL` into `index.html` via
`window.__CLOACINA_CONFIG__`.

That two-artifact model is at odds with the project's **embedded-first philosophy**
([[project_embedded_first_philosophy]]): the README pitches cloacina as runnable
"as a library embedded inside your application … or as a standalone server", yet the
operator's web control plane currently requires a second container + reverse proxy.
This initiative closes that gap so `cloacina-server` can ship as a **single
self-contained binary serving the engine, the REST API, and the UI** from one origin.

The UI is exceptionally well-positioned for this (see assessment, 2026-06-21):
- **Pure static SPA** — client-side routing only, **no SSR / no Node runtime** at
  serve time. `npm run build` already emits `index.html` + hashed JS/CSS (~816KB).
- **Server is axum 0.8**, which has first-class static-serving — adding an embed
  route is a couple of deps + one fallback route in `build_router()`.
- **API base URL is already runtime-driven** via `window.__CLOACINA_CONFIG__`; it is
  *not* hardcoded to localhost. Embedded ⇒ **same-origin**, which makes the injection
  machinery and CORS largely unnecessary for the bundled UI.

This is a **re-plumb, not a rewrite** — no UI framework change, no API contract
change. It complements (does not block) the [[CLOACI-I-0129]] Aurora Dark redesign
and the existing standalone-container path ([[CLOACI-I-0117]] T-0659), which stays
supported.

## Goals & Non-Goals

**Goals:**
- `cloacina-server` can serve the built `@cloacina/ui` SPA from its own origin: an
  axum fallback route serving embedded assets with **SPA fallback** (unmatched
  non-API paths → `index.html`), assets at hashed paths, `index.html` `no-store`.
- Embedding is **gated behind a cargo feature** (e.g. `embedded-ui`) so the default
  server build needs **no Node toolchain** and the binary size cost is opt-in.
- A **reliable build pipeline**: the JS build (`ui/dist/`) is a deterministic
  prerequisite of the feature-on Rust build (angreal task / `build.rs` / committed
  artifact — to be decided in design), so we never ship a stale or missing `dist/`.
- **Same-origin defaults**: when served embedded, the UI defaults `serverUrl` to its
  own origin (relative API calls); the `/connect` gate becomes optional (still usable
  to point the embedded UI at a *different*/remote server).
- Deployment simplification: a compose profile / docs path where the **Nginx UI
  container is unnecessary** because the server serves the UI.

**Non-Goals:**
- No UI framework, routing, data-layer, or API-contract changes (consume the same
  hooks and `@cloacina/client`).
- Not removing the standalone Nginx container path — it remains supported for setups
  that want the UI and API on separate origins/scaled independently.
- No new auth/OIDC work ([[CLOACI-I-0118]]); same-origin may *simplify* token
  handling but reworking auth is out of scope here.
- No SSR / server-rendered UI — it stays a static SPA.

## Requirements

### System Requirements
- **Functional Requirements**
  - REQ-001: With `embedded-ui` enabled, `GET /` and any unmatched non-API,
    non-`/v1`, non-health path returns the SPA `index.html` (SPA fallback).
  - REQ-002: Hashed assets (`/assets/*`) are served from the embedded bundle with
    long-lived immutable cache headers; `index.html` is served `no-store`.
  - REQ-003: API (`/v1/*`), `/health`, `/ready`, `/metrics`, `/openapi.json`, and
    WS routes take precedence over the UI fallback and are unaffected.
  - REQ-004: When served embedded, the UI resolves its API base URL to the serving
    origin by default (relative calls), with no `__CLOACINA_CONFIG__` injection step.
  - REQ-005: The default (feature-off) `cargo build` of `cloacina-server` builds
    with **no Node/npm dependency** and does not embed UI assets.
- **Non-Functional Requirements**
  - NFR-001: Binary-size impact of embedding is bounded and documented; assets are
    compressed where the embed crate supports it.
  - NFR-002: Embedded same-origin serving removes the need for
    `CLOACINA_CORS_ALLOWED_ORIGINS` for the bundled UI (CORS still configurable for
    external origins).
  - NFR-003: The embedded `dist/` is guaranteed fresh for a given build — no path by
    which a stale bundle ships silently.

## Use Cases

### Use Case 1: Single-binary operator console
- **Actor**: Operator running an embedded/standalone cloacina deployment.
- **Scenario**: Runs one `cloacina-server` binary (feature-on) and browses to its
  host:port; the web control plane loads and talks to the same origin with no extra
  container, reverse proxy, or CORS config.
- **Expected Outcome**: Full UI works against the local server out of the box.

### Use Case 2: Embedded UI pointed at a remote server
- **Actor**: Operator who wants the bundled UI but a different API server.
- **Scenario**: Loads the embedded UI, uses the `/connect` gate to enter a remote
  `serverUrl` + key + tenant.
- **Expected Outcome**: Same-origin is the default, but the connect flow still allows
  targeting another server (CORS configured on that server as today).

## Architecture

### Overview
Add a feature-gated static layer to the axum router in
`crates/cloacina-server/src/lib.rs` (`build_router()`, ~line 1090). Under
`#[cfg(feature = "embedded-ui")]`, embed `ui/dist/` into the binary (via
`rust-embed` or `include_dir`) and mount a fallback handler **after** all API /
health / WS routes that (a) serves a matching embedded asset by path, else (b)
returns `index.html` for SPA client-routing. Feature-off compiles exactly today's
API-only server.

### Deployment
- **Embedded mode (new):** one binary, one origin, no Nginx, no CORS for the UI.
- **Standalone mode (existing):** unchanged — Nginx container + cross-origin +
  `__CLOACINA_CONFIG__` injection ([[CLOACI-I-0117]] T-0659) stays supported.

## Detailed Design

### Resolved design decisions (2026-06-22, with maintainer)

1. **Build pipeline → `build.rs` shells out to npm.** When `embedded-ui` is on,
   `crates/cloacina-server/build.rs` runs `npm --prefix ui run build` to produce
   `ui/dist/`, then `rust-embed` embeds that directory at compile time. Freshness
   (NFR-003) is therefore **automatic** — every feature-on `cargo build` rebuilds
   the SPA, so a stale bundle is impossible by construction. `dist/` stays
   gitignored (not committed). Cost accepted: **feature-on builds require a Node +
   npm toolchain** and are slower. Mitigations: gate the npm step behind the cfg so
   feature-off builds never touch Node (REQ-005 preserved); emit
   `cargo:rerun-if-changed` for `ui/src`, `ui/index.html`, `ui/package.json`,
   `ui/package-lock.json`, `ui/vite.config.ts` (and friends) so the SPA only
   rebuilds when UI inputs actually change, not on every Rust recompile; assume
   `node_modules` is already installed by the build env rather than running
   `npm ci` inside `build.rs`.
2. **Embed crate → `rust-embed`.** Debug = serve `ui/dist/` from disk (rebuild the
   SPA without recompiling Rust); release = bytes embedded in the binary, gzip on.
3. **Config / Connect gate → prefill origin, keep the gate.** `ui/src/config.ts`
   defaults `defaultServerUrl` to `window.location.origin` (instead of `""`) when no
   `__CLOACINA_CONFIG__` is injected and not in DEV. The serverUrl field stays
   **editable** so the embedded UI can still target a remote server (Use Case 2).
   The gate still collects **API key + tenant** — it does *not* disappear, because
   auth is bearer key + tenant (OIDC is [[CLOACI-I-0118]], out of scope). No new
   `/config` endpoint; rely on the origin default + relative API calls.
4. **Release default → `embedded-ui` ON in shipped releases.** The `default` feature
   list stays `["postgres"]` (bare `cargo build` is API-only, Node-free), but the
   **release build lane + demo compose profile build with `--features embedded-ui`**,
   so shipped binaries/images are engine + API + UI. The release lane gains a Node
   toolchain dependency as a result.
5. **Cache headers + fallback precedence** (baked in, no open question): the embedded
   fallback is mounted **after** all `/v1`, `/health`, `/ready`, `/metrics`,
   `/openapi.json`, and WS routes. Unknown `/v1/*` (and other API paths) keep
   returning the JSON 404 fallback — the SPA fallback only serves `index.html` for
   non-API GETs. Hashed `/assets/*` → long-lived `immutable` cache; `index.html` →
   `no-store`.

## UI/UX Design

No visual change. The only UI-facing deltas are in `ui/src/config.ts` and
`ui/src/routes/Connect.tsx`: same-origin becomes the default `serverUrl`, and the
Connect gate becomes optional/auto-satisfied when the UI is served embedded.

## Testing Strategy

- **Integration**: a feature-on server test asserting `GET /` and a deep client
  route (e.g. `/executions/abc`) both return `index.html`, `/assets/<hash>` serves
  the asset with immutable caching, and `/v1/...`, `/health`, `/openapi.json`, WS
  upgrade are unaffected by the fallback.
- **Build gate**: CI builds both feature-off (no Node) and feature-on (with
  `dist/`); a freshness check (NFR-003) that the embedded bundle matches a fresh
  `npm run build`.
- **E2E**: extend the existing Playwright UAT ([[CLOACI-I-0117]] T-0661) to run
  against an embedded-mode server (no Nginx) in addition to the standalone stack.

## Alternatives Considered

- **Keep UI-as-separate-Nginx only** — rejected as the *sole* model: it contradicts
  embedded-first and forces a second container + CORS + config injection for the
  simplest deployments. (Kept as an *option*, not the only path.)
- **Reverse-proxy the SPA through the server at runtime from disk** (serve `dist/`
  from a filesystem path rather than embedding) — rejected as the headline: loses
  the single-artifact property. May still be offered as a dev convenience via the
  embed crate's from-disk mode.
- **Server-side render / move to Next.js** — rejected: needs a Node runtime at serve
  time, a rewrite, and buys nothing for an operator console.
- **Bake the server URL into the binary at build time** — rejected: inflexible;
  same-origin-relative + optional connect gate is strictly better.

## Implementation Plan

Design **resolved** (decisions 1–5 above). Next: **decompose** (pending maintainer
go-ahead), then land as one small initiative across a couple of PRs.

**Proposed decomposition (pending `decompose`):**
1. **`build.rs` + `rust-embed` + fallback route** — add the `embedded-ui` feature
   (`rust-embed` optional dep); `build.rs` runs `npm --prefix ui run build` under the
   cfg with `rerun-if-changed` on UI inputs; embed `ui/dist/`; mount the axum fallback
   in `build_router()` after all API/health/WS routes, serving assets (`immutable`)
   + SPA `index.html` (`no-store`). Feature-off compiles exactly as today (Node-free).
2. **Same-origin UI defaults** — `ui/src/config.ts` defaults `defaultServerUrl` to
   `window.location.origin`; serverUrl field stays editable in `Connect.tsx`; gate
   keeps key + tenant. No CORS needed for the bundled UI.
3. **Release lane + compose + docs** — turn on `--features embedded-ui` in the release
   build lane and a single-binary demo compose profile (drop the Nginx UI container
   there); add the Node toolchain to that lane; document single-binary mode and keep
   the standalone Nginx path as the documented alternative.
4. **Tests** — feature-on integration test (`GET /` + deep route → `index.html`;
   `/assets/<hash>` immutable; `/v1`/health/openapi/WS unaffected); extend Playwright
   UAT ([[CLOACI-I-0117]] T-0661) against an embedded-mode server.