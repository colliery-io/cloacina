---
id: distribution-install-script-docker
level: initiative
title: "Distribution: install script, Docker image, Helm chart"
short_code: "CLOACI-I-0111"
created_at: 2026-05-14T22:44:02.988342+00:00
updated_at: 2026-05-15T12:33:06.103355+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: distribution-install-script-docker
---

# Distribution: install script, Docker image, Helm chart Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

Today there is no official install path for Cloacina. Users build from source with `cargo install` or `pip install cloaca`. The Rust crates and Python wheels publish via `unified_release.yml`, but there is no:

- Pre-built binary distribution for `cloacinactl` / `cloacina-daemon`
- Container image for `cloacina-server`
- Helm chart for Kubernetes deployment

This initiative closes that gap. Replaces the standalone backlog item CLOACI-T-0501 (archived).

Related decisions baked in from T-0501 Q&A:
- `cloacina-daemon` stays a separate binary (matches current crate layout, simpler install matrix)
- Install script targets the Rust binaries only — Python users continue with `pip install cloaca`

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- One-line install for CLI + daemon: `curl -fsSL https://get.cloacina.dev/install.sh | bash`
- Pull-and-run server: `docker pull ghcr.io/colliery-software/cloacina-server:<tag>`
- Helm-deployable server stack with managed Postgres dependency
- All three artifacts cut on every release tag via existing `unified_release.yml`

**Non-Goals:**
- Homebrew formula (secondary channel — separate task if requested)
- Windows binaries (Rust supports it; ops complexity not justified yet)
- Bundling the Rust toolchain into the server image (depends on T-0495 / I-0097 compiler service split)
- Auto-update for installed binaries (use `--version` to pin; users re-run installer to upgrade)

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design **[REQUIRED]**

Three independent deliverables, decomposed into one task each. They can ship in any order; install script is highest user-visible value, server image is highest ops value, Helm depends on the image existing.

### T-01 (CLOACI-T-0603): Install script + GitHub Releases binaries

- New release-workflow job: cross-compile `cloacinactl` and `cloacina-daemon` for `{x86_64,aarch64}-{linux-gnu,apple-darwin}` (use `cross` for Linux cross, native macOS runners for Darwin).
- Tarball + SHA256 sidecar per artifact.
- Upload to GitHub Releases on tag.
- `install.sh` shipped in-repo (`scripts/install.sh`), served from `https://get.cloacina.dev/install.sh` (DNS / static hosting setup outside this repo, but the script itself lives here).
- Script behaviour: detect OS/arch, download both binaries, verify SHA256, install to `~/.cloacina/bin`, print PATH-add instructions if needed. `--version vX.Y.Z` to pin, `--prefix /usr/local` to override location.
- `cloacinactl --version` already wired via `clap` — verify the build embeds `CARGO_PKG_VERSION`.

### T-02 (CLOACI-T-0604): Server Docker image

- `Dockerfile` at repo root using multi-stage build:
  - Stage 1: `rust:1.85-slim` builder, `cargo build --release -p cloacina-server`.
  - Stage 2: `gcr.io/distroless/cc-debian12` runtime, copy binary, expose port, healthcheck on `/v1/health`.
- No Rust toolchain in runtime stage — defers CG compilation to the compiler-service split (I-0097, already shipped).
- Tags: `<semver>`, `<major>.<minor>`, `latest`, `nightly` (latter from `nightly.yml`).
- Push to `ghcr.io/colliery-software/cloacina-server` from `unified_release.yml` on tag, `nightly.yml` on schedule.
- Smoke test in CI: `docker run --rm <image> --version` exits 0.

### T-03 (CLOACI-T-0605): Helm chart

- `charts/cloacina-server/` containing:
  - `Chart.yaml` with semver tracking the server crate version
  - `values.yaml` exposing replicas, image tag, postgres URL, API key secret ref, ingress host, resource limits
  - `templates/deployment.yaml`, `service.yaml`, `ingress.yaml`, `configmap.yaml`, `secret.yaml`, `servicemonitor.yaml`
  - Bitnami `postgresql` subchart as optional dependency (`postgresql.enabled=true|false`)
- `helm lint` + `helm template` in CI.
- Push as OCI artifact to `ghcr.io/colliery-software/charts/cloacina-server` on tag.
- `helm install cloacina oci://ghcr.io/colliery-software/charts/cloacina-server --version <tag>` install path documented in tutorial.

## Implementation Plan **[REQUIRED]**

Sequential, but each task is independently shippable.

1. **T-0603 install script** — highest leverage; unblocks contributors and demos. ~2 days.
2. **T-0604 Docker image** — unblocks server deployments. ~2 days.
3. **T-0605 Helm chart** — depends on the image. ~2 days.

Total: ~6 working days. PR-per-task per the squash-merge convention.

## Alternatives Considered **[REQUIRED]**

- **`cargo binstall` instead of curl-bash installer.** Considered — adds a `cargo` dependency the user must install first, which defeats the "no Rust toolchain" win. Will publish `cargo-binstall` metadata as a side benefit but the primary install path is the script.
- **Distroless Wolfi vs. Debian.** Wolfi has smaller base + faster CVE turnaround; Debian distroless has broader operator familiarity. Defaulting to Debian distroless; switch is a one-line change later.
- **Helm chart in a separate repo (`cloacina-charts`).** Cleaner versioning story but doubles the release dance. Keeping in-repo until the chart audience grows.

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}
