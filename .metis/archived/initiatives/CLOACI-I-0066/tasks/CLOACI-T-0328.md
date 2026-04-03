---
id: write-new-how-to-guides-4-docs
level: task
title: "Write new How-To guides (4 docs)"
short_code: "CLOACI-T-0328"
created_at: 2026-04-02T22:51:44.257004+00:00
updated_at: 2026-04-02T23:39:06.803808+00:00
parent: CLOACI-I-0066
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0066
---

# Write new How-To guides (4 docs)

## Parent Initiative
[[CLOACI-I-0066]]

## Objective
Write 4 new How-To guides for operational tasks that have zero documentation.

## Documents to Write

### 1. how-to-guides/running-the-daemon.md
- Prerequisites: cloacinactl installed, packages to deploy
- Starting the daemon: `cloacinactl daemon`
- Configuring watch directories (--watch-dir, config.toml)
- Deploying packages (copy .cloacina files to watch dir)
- Inspecting logs (~/.cloacina/logs/)
- Configuring poll intervals, cron catchup, recovery
- Troubleshooting: package not loading, cron not firing
- Source: crates/cloacinactl/src/commands/daemon.rs

### 2. how-to-guides/deploying-the-api-server.md
- Prerequisites: PostgreSQL, cloacinactl
- Starting: `cloacinactl serve --bind --database-url`
- Bootstrap key (auto-generated vs --bootstrap-key vs env var)
- Health checks: GET /health, GET /ready
- Creating API keys via POST /auth/keys
- Production configuration (bind address, pool size, TLS termination)
- Docker deployment pattern
- Source: crates/cloacinactl/src/commands/serve.rs, server/

### 3. how-to-guides/monitoring-executions.md
- Listing executions: GET /tenants/:id/executions
- Getting execution details: GET /tenants/:id/executions/:id
- Viewing event logs: GET /tenants/:id/executions/:id/events
- Checking trigger/cron status: GET /tenants/:id/triggers
- Python API: runner.get_cron_execution_stats(), list_cron_schedules()
- Example: building a simple monitoring script
- Source: crates/cloacinactl/src/server/executions.rs, triggers.rs

### 4. how-to-guides/cleaning-up-events.md
- Why cleanup matters (event table growth)
- Using `cloacinactl admin cleanup-events`
- --older-than duration format (90d, 30d, 7d, 24h)
- --dry-run for previewing
- Automating with cron
- Source: crates/cloacinactl/src/commands/cleanup_events.rs

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] All 4 docs complete with no placeholders
- [ ] Each guide has a clear goal, prerequisites, and actionable steps
- [ ] Real CLI commands and API calls (not abstract placeholders)
- [ ] Cross-links to CLI and API reference docs

## Status Updates
*To be added during implementation*
