---
title: "Deploy the Web UI"
description: "Deploy the Cloacina web UI: embedded in cloacina-server (default), the demo stack, and the optional standalone Helm chart."
weight: 65
aliases:
  - "/platform/how-to-guides/deploy-the-web-ui/"

---

The Cloacina web UI is **embedded in the `cloacina-server` binary**: one
binary is the engine, the REST API, **and** the web control plane — same
origin, no separate Nginx container, no CORS setup for the bundled UI. The
standalone Nginx-served SPA container was retired; the embedded UI is the
deployment path.

For how the embedded UI works (enabling the `embedded-ui` feature, routing,
caching, pointing it at a remote server), see
[Embedded Web UI]({{< ref "/service/embedded-ui" >}}). This guide covers the
deployment surfaces: the demo stack and the optional standalone Helm chart.

## The demo stack

`docker/docker-compose.demo.yml` is a self-contained "stand it up and watch
it run" profile: postgres + server + **compiler** + a one-shot **fixtures
packer** + the **seed harness** (loop mode). There is **no separate UI
service** — the UI is served by the server itself:

```bash
docker compose -f docker/docker-compose.demo.yml up --build
```

The UI is then at <http://localhost:8080> (embedded — served by the server).
The harness drives a mix of fast / slow / failing runs continuously, so the
dashboard and live execution view always have something moving. The first
build is heavy (the compiler and fixtures images compile the workspace once).

## Optional: standalone UI via Helm

The `charts/cloacina-ui` chart deploys the UI as a standalone Deployment +
Service + (optional) Ingress, using the published `cloacina-ui` image
(`ghcr.io/colliery-software/cloacina-ui`). This is an optional alternative to
the embedded UI — use it when you want the UI served from its own origin,
separate from the server.

Because the browser then loads the SPA from the UI's origin and calls the
server **cross-origin**, this path requires CORS on the server:

```bash
helm install ui charts/cloacina-ui \
  --set serverUrl=https://cloacina.example.com \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=ui.example.com
```

- `serverUrl` is the address the **browser** uses to reach the server (its
  public/ingress URL), not the in-cluster Service DNS. It is injected at
  container start into `window.__CLOACINA_CONFIG__` and prefills the connect
  form; leave it empty and the connect screen asks the user for it.
- Ensure the server's `CLOACINA_CORS_ALLOWED_ORIGINS` (flag
  `--cors-allowed-origins`) includes the UI's public origin. The allowed
  origin is the URL users load the UI from — not the server's own address.

## Version lockstep

The UI is version-matched to the server it talks to: the `ui`/`ui-harness`
package versions track the workspace version, asserted in CI by
`scripts/check_sdk_versions.py`. For the standalone chart, deploy the
`cloacina-ui` image whose tag matches your server release.
