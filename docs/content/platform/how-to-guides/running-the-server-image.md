---
title: "Running the cloacina-server Docker image"
description: "Pull, run, and configure the official cloacina-server container from ghcr.io"
weight: 56
---

# Running the `cloacina-server` Docker image

The official `cloacina-server` image is published to the GitHub Container
Registry on every release tag and nightly on `main`. This page covers
pulling, configuring, and running it.

## Pulling

```sh
# Latest stable release
docker pull ghcr.io/colliery-software/cloacina-server:latest

# Pin to a specific version
docker pull ghcr.io/colliery-software/cloacina-server:0.6.0

# Track main (rebuilt every night at 03:00 UTC)
docker pull ghcr.io/colliery-software/cloacina-server:nightly
```

The image is multi-arch (`linux/amd64` + `linux/arm64`); Docker selects
the matching manifest automatically.

## Tag scheme

| Tag | Updated | Notes |
|-----|---------|-------|
| `latest`     | every release | The most recent stable tag |
| `<X.Y.Z>`    | one-shot      | Immutable, pinned to that release |
| `<X.Y>`      | every patch   | Floats forward across patch releases |
| `nightly`    | daily 03:00 UTC | Built from `main` |

## Running

The server needs a Postgres database — `DATABASE_URL` is the only
required env var.

```sh
docker run --rm \
  -e DATABASE_URL=postgres://cloacina:cloacina@host.docker.internal:5432/cloacina \
  -p 8080:8080 \
  ghcr.io/colliery-software/cloacina-server:latest
```

On startup it auto-runs migrations and (on first boot) generates a
bootstrap API key. Watch the logs for the key, or pin one with
`-e CLOACINA_BOOTSTRAP_KEY=...`.

## Common configuration

All flags double as env vars:

| Flag | Env | Purpose |
|------|-----|---------|
| `--bind` | — | Bind address (default `127.0.0.1:8080`; usually `0.0.0.0:8080` in containers) |
| `--database-url` | `DATABASE_URL` | Postgres connection string |
| `--bootstrap-key` | `CLOACINA_BOOTSTRAP_KEY` | Pin the initial admin API key |
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | Reject unsigned packages |
| `--verification-org-id` | `CLOACINA_VERIFICATION_ORG_ID` | Org UUID for signature verification |
| `--tenant-runner-cache-size` | `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | LRU cap on per-tenant runners (default 256) |

The image's home directory is `/home/cloacina/.cloacina`
(`CLOACINA_HOME` is set so any tooling that reads it resolves correctly).

## Compose example

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: cloacina
      POSTGRES_PASSWORD: cloacina
      POSTGRES_DB: cloacina
    volumes:
      - cloacina-pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U cloacina"]
      interval: 5s
      retries: 12

  cloacina-server:
    image: ghcr.io/colliery-software/cloacina-server:latest
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://cloacina:cloacina@postgres:5432/cloacina
    command: ["--bind", "0.0.0.0:8080"]
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD-SHELL", "wget -qO- http://127.0.0.1:8080/v1/health || exit 1"]
      interval: 10s
      retries: 6

volumes:
  cloacina-pgdata:
```

The image itself does not declare a `HEALTHCHECK` — orchestrators each
have their own probe story (compose example above; Kubernetes via the
Helm chart in CLOACI-T-0605).

## Image properties

- Base: `debian:bookworm-slim` + `libpq5` + `ca-certificates`
- Compressed size: ~150 MB
- Runs as non-root (uid `10001`, gid `10001`)
- Exposes port `8080`
- Source labelled via `org.opencontainers.image.source` so ghcr links
  the image back to the repo

## Building locally

```sh
docker build -t cloacina-server:dev .
docker run --rm cloacina-server:dev --version
```

The Dockerfile lives at the repo root; `.dockerignore` keeps build
artifacts and dev metadata out of the build context.

## See also

- [Deploying the API Server]({{< ref "deploying-the-api-server" >}}) —
  full configuration reference (auth, signatures, multi-tenancy)
- [Production Deployment]({{< ref "production-deployment" >}}) —
  scaling and operational hardening
