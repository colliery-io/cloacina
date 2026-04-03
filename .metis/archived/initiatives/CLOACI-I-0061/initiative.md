---
id: daemon-infrastructure-background
level: initiative
title: "Daemon Infrastructure — background workflow runner with SQLite"
short_code: "CLOACI-I-0061"
created_at: 2026-04-01T11:33:46.236350+00:00
updated_at: 2026-04-01T11:33:46.236350+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: daemon-infrastructure-background
---

# Daemon Infrastructure — background workflow runner with SQLite Initiative

## Context

Split from I-0049 (Server & Daemon). `cloacinactl daemon` is the lightweight local deployment mode — SQLite backend, no auth, embedded workflows and packaged workflow loading. Runs as a background process with cron/trigger scheduling, task execution, and package reconciliation.

The daemon already exists in `cloacinactl/src/commands/daemon.rs` but needs hardening: proper config, graceful shutdown, Docker compose for dev, soak testing, and documentation.

## Goals & Non-Goals

**Goals:**
- `cloacinactl daemon` with SQLite backend, configurable data directory
- Proper graceful shutdown (SIGINT/SIGTERM)
- Docker compose for local development
- Daemon soak test via angreal
- Configuration file support (TOML or env vars)
- Package reconciliation on startup and at intervals

**Non-Goals:**
- HTTP API (that's I-0049 server mode)
- Authentication (daemon is local-only)
- Multi-tenancy (single-tenant SQLite)
- Postgres backend (server mode only)

## Detailed Design

The daemon runs DefaultRunner with SQLite, starts background services (unified scheduler, reconciler, claim sweeper), loads packages from a configured directory, and runs until shutdown.

Key areas:
- Config file parsing (data dir, poll intervals, package dir)
- SQLite WAL mode defaults for concurrent access
- Package directory watching or polling for new .cloacina files
- Soak test: sustained package loading and execution over time

## Implementation Plan

1. Docker compose + daemon soak test (T-0299, moved from I-0049)
2. Config file support
3. Package directory watching
4. Documentation
