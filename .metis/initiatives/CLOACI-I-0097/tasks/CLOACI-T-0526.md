---
id: t8-docker-compose-template-two
level: task
title: "T8: Docker Compose template + two-process runbook"
short_code: "CLOACI-T-0526"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-18T01:50:00+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0525]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T8: Docker Compose template + two-process runbook

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Ship the operational story for running server + compiler as a paired service. No launcher tooling — just a Docker Compose template that works out-of-the-box and prose covering native/bare-metal deployment.

## Acceptance Criteria

- [ ] `deploy/docker-compose/cloacina.yml` — three services: `postgres`, `cloacina-server`, `cloacina-compiler`. Compose bringup produces a working server + compiler against Postgres. Smoke-tested against real images.
- [ ] `deploy/docker-compose/cloacina-sqlite.yml` — two services (server + compiler) sharing a SQLite DB via a named volume. Produces a single-machine dev setup.
- [ ] `docs/operations/compiler-deployment.md` — runbook covering:
  - Why two binaries (link to ADR-0004).
  - Local: two terminal panes, one per binary, shared `DATABASE_URL`.
  - Docker Compose: which file to use when.
  - Kubernetes sketch: two Deployments (server Deployment + compiler Deployment), one StatefulSet (Postgres), no leader-election needed for the compiler (atomic claim handles it).
  - Config knobs table: heartbeat/stale/sweep intervals, cargo_flags, tmp_root.
  - Operational playbooks: "a build is stuck" → `cloacinactl package inspect`, wait for sweeper, or `package retry-build` (v1.1).
- [ ] README.md link to the new runbook.
- [ ] The Docker Compose template uses the image tags that T-0501 (distribution) will publish. Until T-0501 lands, pin to `:latest` and note the dependency.

## Implementation Notes

### Dockerfiles

Two new Dockerfiles live under `deploy/docker/`:
- `server.Dockerfile` — slim runtime base (debian-slim or distroless), just the `cloacina-server` binary. ~50 MB image goal.
- `compiler.Dockerfile` — `rust:1.85-slim` base (toolchain included), `cloacina-compiler` binary + cargo. ~1 GB is fine.

Image build is wired up by T-0501's GitHub Actions release flow; this task only ships the Compose YAML + Dockerfiles.

### docs/operations/

New directory. Top-level README in `docs/` points into it.

### K8s sketch

Just prose + a single YAML snippet per resource. Not a full Helm chart — that's T-0501.

## Status Updates

*To be added during implementation*
