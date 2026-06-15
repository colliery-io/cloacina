---
title: "Deploy the Web UI"
description: "Run the Cloacina web UI as a container against any server: image, runtime config, CORS, compose, and the demo profile."
weight: 65
aliases:
  - "/platform/how-to-guides/deploy-the-web-ui/"

---

The Cloacina web UI ships as a server-agnostic container: one image runs
against any `cloacina-server`, with the target server URL injected at
container start. This guide covers running the image, wiring CORS, the
compose profiles, and the demo profile.

## The image

The UI is a Vite-built React SPA served by nginx (`ui/Dockerfile`,
multi-stage: it builds the `@cloacina/client` SDK then the bundle). Build
it from the repo root (the build context must be the repo root so the
sibling SDK is available):

```bash
docker build -t cloacina-ui:dev -f ui/Dockerfile .
```

## Runtime server-URL config

The bundle is server-agnostic. At container start, an entrypoint renders
`index.html` from a template, injecting `CLOACINA_SERVER_URL` into
`window.__CLOACINA_CONFIG__` so the value prefills the Connect form:

```bash
docker run --rm -p 8081:80 \
  -e CLOACINA_SERVER_URL=https://cloacina.example.com \
  cloacina-ui:dev
```

Leave `CLOACINA_SERVER_URL` unset and the Connect screen simply asks for
the server URL. Because rendering happens from the template on every
start, restarting with a different URL just works — nothing is baked in.

## CORS (required)

The browser loads the SPA from the UI's origin and calls the server
**cross-origin**, so the server must allow the UI's origin. CORS is
disabled on `cloacina-server` unless you opt in:

```bash
cloacina-server \
  --cors-allowed-origins https://ui.example.com \
  ...
# or: CLOACINA_CORS_ALLOWED_ORIGINS=https://ui.example.com
```

The allowed origin is the URL users load the UI from — not the server's
own address.

## Compose profiles

Two compose files are provided (build context is the repo root):

- **`docker/docker-compose.ui.yml`** — postgres + server (CORS) + UI.
  `docker compose -f docker/docker-compose.ui.yml up --build` → UI at
  <http://localhost:8081>, server at <http://localhost:8080>.

- **`docker/docker-compose.demo.yml`** — the self-contained demo:
  postgres + server + **compiler** + a one-shot **fixtures packer** +
  the **seed harness** (loop mode) + UI. `up --build` yields a UI with
  continuous live activity to watch. The first build is heavy (the
  compiler and fixtures images compile the workspace).

## Helm

An optional chart lives at `charts/cloacina-ui` (deployment + service +
ingress, mirroring `cloacina-server`). Set the browser-facing server URL
and remember the CORS requirement:

```bash
helm install ui charts/cloacina-ui \
  --set serverUrl=https://cloacina.example.com \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=ui.example.com
```

`serverUrl` is the address the **browser** uses to reach the server (its
public/ingress URL), not the in-cluster Service DNS. Ensure the server's
`CLOACINA_CORS_ALLOWED_ORIGINS` includes the UI's public origin.

## Version lockstep

The UI is version-matched to the server it talks to (NFR-004): the
`ui`/`ui-harness` package versions track the workspace version, asserted
in CI by `scripts/check_sdk_versions.py`. Deploy the UI image whose tag
matches your server release.
