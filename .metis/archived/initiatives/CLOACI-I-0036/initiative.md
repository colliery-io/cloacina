---
id: server-phase-8-deployment
level: initiative
title: "Server Phase 8: Deployment Artifacts — Docker + Config"
short_code: "CLOACI-I-0036"
created_at: 2026-03-16T01:32:40.137957+00:00
updated_at: 2026-03-18T03:09:46.790868+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: S
initiative_id: server-phase-8-deployment
---

# Server Phase 8: Deployment Artifacts — Docker + Config Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: CLOACI-I-0029 (Foundation — need something to deploy)
**Blocks**: None

## Context

No production deployment artifacts exist. Only a test Dockerfile for CI. Operators need a production-ready Docker image, a docker-compose for quick start, example configuration, and documentation to go from zero to running server.

## Goals

- Multi-stage Dockerfile (builder with Rust toolchain → runtime with minimal base image)
- docker-compose.yml: `cloacinactl serve --mode=all` + Postgres
- Example `cloacina.toml` with all options documented and sensible defaults
- Getting started guide: pull image → create config → docker-compose up → create tenant → upload workflow → trigger execution
- Architecture documentation: scaling patterns (single-node, multi-worker, multi-scheduler)

## Implementation Plan

- [ ] Production Dockerfile (multi-stage: builder → runtime)
- [ ] docker-compose.yml (cloacinactl serve + postgres)
- [ ] Example cloacina.toml with all options documented
- [ ] Getting started guide (docs/content/tutorials/)
