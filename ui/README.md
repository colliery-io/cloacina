# cloacina-ui

The Cloacina web UI: a tenant-scoped control plane (CLOACI-I-0117), written in
[Leptos](https://leptos.dev) as a client-side app and compiled to wasm with
[trunk](https://trunkrs.dev) (CLOACI-I-0141).

There is no separate UI service. `cloacina-server`, built with the
`embedded-ui` feature, embeds `ui/dist` and serves the UI at the root of its
own origin, next to the REST API. Paths under `/v1`, `/health`, `/ready`,
`/metrics` and `/openapi.json` stay API; every other path serves the app.

This crate is not a member of the root cargo workspace, so wasm dependencies
never enter the server build.

## Develop

Install `trunk` and the wasm target:

```bash
cargo install trunk
rustup target add wasm32-unknown-unknown
```

Run the demo stack (server on `http://localhost:8080`), then serve the UI with
live reload:

```bash
docker compose -f docker/docker-compose.demo.yml up --build
cd ui && trunk serve   # http://localhost:5173
```

A debug build prefills and connects to the demo server
(`http://localhost:8080`, the demo bootstrap key, tenant `public`); see
`src/config.rs`. A release build uses its own origin.

## Build

```bash
trunk build --release                                         # ui/dist
cargo build -p cloacina-server --features embedded-ui         # runs trunk itself
```

The `Dockerfile` builds `ui/dist` in a trunk stage and sets
`CLOACINA_EMBEDDED_UI_SKIP_NPM=1` so the server stage embeds it without
running trunk again.

## Test

Node is used only for the Playwright tests (`e2e/`) and the seed harness
(`harness/`).

```bash
angreal test e2e ui-e2e           # full lane: builds, starts a server, runs Playwright
angreal test e2e ui-e2e --smoke   # @smoke subset
```

## Layout

```
src/
  main.rs, app.rs   entry point and route map
  config.rs         runtime config (server URL, demo prefill)
  auth.rs           connection state, API key and tenant
  shell.rs          authenticated shell (nav, connection)
  components.rs     shared components
  data.rs, ops.rs   API calls through cloacina-client
  routes/           one module per view
style/              app.css
design/             Aurora design reference
e2e/                Playwright specs
harness/            seed / demo workload driver
```
