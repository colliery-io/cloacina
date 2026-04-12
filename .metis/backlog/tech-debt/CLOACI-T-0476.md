---
id: add-daemon-health-check-endpoint
level: task
title: "Add daemon health check endpoint"
short_code: "CLOACI-T-0476"
created_at: 2026-04-11T13:45:01.709844+00:00
updated_at: 2026-04-11T13:45:01.709844+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Add daemon health check endpoint

## Objective

Add a lightweight health check HTTP endpoint to `cloacinactl daemon` mode for container orchestrator probing.

## Review Finding References

OPS-001 (from architecture review `review/10-recommendations.md` REC-004)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: Daemon has no health surface. Orchestrators can only check process existence, not detect deadlocked schedulers or DB connectivity loss.
- **Benefits of Fixing**: Container deployments can use proper health/readiness probes.
- **Risk Assessment**: No risk from not addressing immediately — daemon works fine, just not observable via health checks.

## Acceptance Criteria

- [ ] `--health-port` CLI option on daemon command (default: 9090)
- [ ] `/health` endpoint returning JSON with DB connectivity, last reconciliation time, uptime
- [ ] `HEALTHCHECK` instruction added to Dockerfile

## Implementation Notes

### Key Files
- `crates/cloacinactl/src/commands/daemon.rs`
- `Dockerfile`

### Dependencies
None.

## Status Updates

*To be added during implementation*
