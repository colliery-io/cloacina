---
id: write-new-reference-docs-6-docs
level: task
title: "Write new Reference docs (6 docs)"
short_code: "CLOACI-T-0327"
created_at: 2026-04-02T22:51:43.368383+00:00
updated_at: 2026-04-02T23:39:05.925664+00:00
parent: CLOACI-I-0066
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0066
---

# Write new Reference docs (6 docs)

## Parent Initiative
[[CLOACI-I-0066]]

## Objective
Write 6 new Reference documents to fill the biggest gap in the documentation site.

## Documents to Write

### 1. reference/_index.md — Un-draft and populate
- Remove `draft: true`, add overview of Reference section
- List all reference docs with brief descriptions
- Source: existing docs structure

### 2. reference/cli.md — CLI Reference (cloacinactl)
- All commands: daemon, serve, config (get/set/list), admin cleanup-events
- All flags: --verbose, --home, --bind, --database-url, --bootstrap-key, --watch-dir, --poll-interval, --older-than, --dry-run
- Environment variables: DATABASE_URL, CLOACINA_BOOTSTRAP_KEY, RUST_LOG
- config.toml schema (all sections: database, daemon, watch)
- File locations: ~/.cloacina/ (packages/, logs/, config.toml, bootstrap-key)
- Source: crates/cloacinactl/src/main.rs, commands/*.rs

### 3. reference/http-api.md — HTTP API Reference
- Public endpoints: GET /health, /ready, /metrics
- Auth endpoints: POST/GET/DELETE /auth/keys
- Tenant endpoints: POST/GET/DELETE /tenants
- Workflow endpoints: POST/GET/DELETE /tenants/:id/workflows
- Execution endpoints: POST/GET /tenants/:id/workflows/:name/execute, /executions, /executions/:id/events
- Trigger endpoints: GET /tenants/:id/triggers
- Request/response JSON schemas for every endpoint
- Authentication mechanism (Bearer token)
- Error response format
- Source: crates/cloacinactl/src/server/*.rs

### 4. reference/configuration.md — Configuration Reference
- DefaultRunnerConfig: all 30+ fields with types, defaults, descriptions
- DefaultRunnerBuilder methods
- config.toml schema (daemon section, watch section)
- Environment variables
- Source: crates/cloacina/src/runner/default_runner/config.rs

### 5. reference/macros.md — Macro Reference
- #[task] attributes: id, dependencies, retry_*, trigger_rules, on_success, on_failure
- #[workflow] attributes: name, description, features
- #[trigger] attributes: on, poll_interval, cron, timezone
- workflow! macro syntax
- Code fingerprinting behavior
- Source: crates/cloacina-macros/src/lib.rs, tasks.rs

### 6. reference/errors.md — Error Reference
- ContextError, TaskError, ValidationError, ExecutorError, RegistrationError, WorkflowError, SubgraphError, PipelineError, CronError, TriggerError
- All variants with descriptions
- Common causes and solutions
- Source: crates/cloacina/src/error.rs, cron_evaluator.rs

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] All 6 docs are complete with no placeholders
- [ ] Every claim cross-referenced against actual code
- [ ] Proper Hugo frontmatter (title, description, weight)
- [ ] Cross-links to related tutorials, how-to guides, and explanations

## Status Updates
*To be added during implementation*
