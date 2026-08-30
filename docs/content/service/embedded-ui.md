---
title: "Embedded Web UI"
description: "Serve the web control plane from the cloacina-server binary — one origin, no extra container."
weight: 45
---

# Embedded Web UI

`cloacina-server` can serve the `@cloacina/ui` control plane itself, so one
binary is the engine, the REST API, **and** the web UI — same origin, no
Nginx container, no CORS configuration for the bundled UI.

## Enabling

The feature is on in **released binaries and images**. Building from source:

```bash
cargo build -p cloacina-server --features embedded-ui
```

Feature-on builds require the wasm toolchain: `build.rs` runs
`trunk build --release` in `ui/` (the web UI is a Rust/Leptos app,
CLOACI-I-0141) and embeds `ui/dist` into the binary. Install
[trunk](https://trunkrs.dev) plus the `wasm32-unknown-unknown` target:

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
```

A stale bundle is impossible by construction. The default `cargo build`
stays trunk-free and serves no UI. No Node toolchain is involved anywhere
in the server build.

Container builds that pre-build the UI in a separate trunk stage can set
`CLOACINA_EMBEDDED_UI_SKIP_NPM=1` (name kept from the npm era) to make
`build.rs` use the existing `ui/dist` instead of invoking trunk.

## Behavior

- `GET /` and any client-routed path (e.g. `/executions/abc`) serve the SPA
  `index.html` (`Cache-Control: no-store`).
- Hashed assets under `/assets/…` are served immutable
  (`max-age=31536000, immutable`).
- API surfaces win: `/v1/*`, `/health`, `/ready`, `/metrics`,
  `/openapi.json`, and WebSocket routes are untouched — an unknown `/v1/…`
  path still returns the JSON 404, never the SPA.
- The UI defaults its server URL to the serving origin (relative API
  calls). The connect gate still collects the API key + tenant, and the
  server URL stays editable for pointing the bundled UI at a *different*
  server (configure CORS on that server as usual).

## Remote servers

The embedded UI can still target a *different* cloacina-server: edit the
server URL on the connect gate and set `CLOACINA_CORS_ALLOWED_ORIGINS` on
that server. (The separate Nginx UI container was retired in CLOACI-I-0130 —
the embedded UI is the deployment path.)
