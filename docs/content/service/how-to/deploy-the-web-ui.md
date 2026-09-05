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

## Serving the UI from a different origin

There is no standalone UI image or chart — the embedded UI **is** the
deployment path (the Nginx SPA container and its `cloacina-ui` chart were
retired with CLOACI-I-0130/I-0141). If you want the control plane on a
different origin than the server it operates, run a second
`cloacina-server` (or point any embedded UI's connect screen at the target
server's URL) and allow that origin on the target server:

- Set `CLOACINA_CORS_ALLOWED_ORIGINS` (flag `--cors-allowed-origins`) on
  the **target** server to the origin users load the UI from.
- On the connect screen, edit the server URL to the target server's
  public address; the API key and tenant scope the session as usual.

## Version lockstep

The UI is compiled into the server binary, so it is version-matched by
construction — the `cloacina-ui` crate version tracks the workspace
version, asserted in CI by `scripts/check_sdk_versions.py`.
