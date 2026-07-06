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

Feature-on builds require a Node toolchain: `build.rs` runs
`npm --prefix ui run build` and embeds `ui/dist` into the binary (release
builds embed compressed bytes; debug builds read from disk so UI iteration
doesn't require recompiling Rust). A stale bundle is impossible by
construction. The default `cargo build` remains Node-free and serves no UI.

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

## Split-origin deployments

The standalone Nginx container path (`docker/docker-compose.ui.yml`) remains
fully supported for setups that want the UI and API on separate origins or
scaled independently — set `CLOACINA_CORS_ALLOWED_ORIGINS` as before.
