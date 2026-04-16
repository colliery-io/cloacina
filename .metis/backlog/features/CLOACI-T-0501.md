---
id: distribution-strategy-cli-daemon
level: task
title: "Distribution strategy — CLI/daemon install script, server Docker image, Helm chart"
short_code: "CLOACI-T-0501"
created_at: 2026-04-16T12:44:09.822835+00:00
updated_at: 2026-04-16T12:44:09.822835+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Distribution strategy — CLI/daemon install script, server Docker image, Helm chart

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Define and implement the distribution strategy for all Cloacina components. Three distinct artifacts with different distribution needs:

- **`cloacinactl`** (CLI) — developer/operator tool, installed on workstations
- **`cloacina-daemon`** — long-running local process, installed on dev machines or CI nodes
- **`cloacina-server`** — production service, deployed to Kubernetes or Docker

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P1 - High (important for user experience)

### Business Justification
- **User Value**: Users need a frictionless install path. `curl | bash` for CLI/daemon matches developer expectations (rustup, homebrew, nvm). Docker + Helm for server matches ops expectations. Today there's no official install path — users build from source.
- **Effort Estimate**: L

## Distribution Plan

### 1. `cloacinactl` + `cloacina-daemon` (CLI binaries)

**Channel**: GitHub Releases + install script

```
curl -fsSL https://get.cloacina.dev/install.sh | bash
```

**Artifacts** (per release tag):
- `cloacinactl-x86_64-unknown-linux-gnu.tar.gz`
- `cloacinactl-aarch64-unknown-linux-gnu.tar.gz`
- `cloacinactl-x86_64-apple-darwin.tar.gz`
- `cloacinactl-aarch64-apple-darwin.tar.gz`
- Same matrix for `cloacina-daemon`

**Install script behavior**:
- Detect OS/arch
- Download correct binary from GitHub Releases
- Verify checksum (SHA256 sidecar file)
- Install to `~/.cloacina/bin` or `/usr/local/bin`
- Add to PATH if needed
- `--version` flag to pin a release

**CI**: GitHub Actions release workflow on tag push. Cross-compile via `cross` or native runners.

### 2. `cloacina-server` (Docker image)

**Channel**: GitHub Container Registry (`ghcr.io`)

```
docker pull ghcr.io/colliery-software/cloacina-server:0.5.1
docker pull ghcr.io/colliery-software/cloacina-server:latest
```

**Image**:
- Multi-stage build: Rust builder -> minimal runtime (distroless or alpine)
- Includes `cloacinactl serve` entrypoint
- Includes Rust toolchain for CG compilation (or defer to T-0495 compiler service)
- Health check built in (`/v1/health`)
- Tags: semver (`0.5.1`), major-minor (`0.5`), `latest`, `nightly`

**CI**: Build and push on release tag. Nightly builds on main.

### 3. Helm Chart

**Channel**: GitHub Pages or OCI registry (`ghcr.io`)

```
helm repo add cloacina https://charts.cloacina.dev
helm install cloacina cloacina/cloacina-server
```

**Chart includes**:
- Server deployment (configurable replicas for horizontal scaling)
- PostgreSQL dependency (Bitnami subchart or external)
- Kafka dependency (optional, for stream accumulators)
- ConfigMap for server config
- Secret for API keys / TLS certs
- Service + Ingress for HTTP/WebSocket
- ServiceMonitor for Prometheus scraping

## Acceptance Criteria

- [ ] Install script downloads and installs `cloacinactl` on Linux (x86_64, aarch64) and macOS (x86_64, aarch64)
- [ ] Install script verifies SHA256 checksums
- [ ] Docker image builds and runs `cloacinactl serve` with health check passing
- [ ] Docker image tagged and pushed to ghcr.io on release
- [ ] Helm chart deploys server + Postgres, passes smoke test
- [ ] Nightly CI builds all artifacts (binaries, image, chart)
- [ ] `cloacinactl --version` reports correct version from build

## Implementation Notes

### Open questions
- Should the daemon be a separate binary or a `cloacinactl daemon` subcommand? (currently separate crate `cloacina-daemon`)
- Homebrew formula as a secondary channel for macOS?
- Should the install script also install `cloaca` (Python wheel)?
- Docker image size budget — full Rust toolchain adds ~1.5GB. If T-0495 (compiler extraction) lands first, the server image can be slim.

## Status Updates

*To be added during implementation*
