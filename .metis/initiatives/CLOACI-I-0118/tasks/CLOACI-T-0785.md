---
id: execution-agent-endpoint-tenant
level: task
title: "Execution-agent endpoint tenant hardening — heartbeat/result guard + GET agents tenant-scoped"
short_code: "CLOACI-T-0785"
created_at: 2026-06-24T00:41:40.013101+00:00
updated_at: 2026-06-24T00:41:40.013101+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Execution-agent endpoint tenant hardening — heartbeat/result guard + GET agents tenant-scoped

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Tighten the three execution-agent endpoints that currently drop the caller's tenant. Add an in-memory guard on `POST /agent/heartbeat` and `POST /agent/result` asserting the caller key's tenant equals the registered agent's tenant (god bypasses). Lower `GET /agents` from god-only to tenant-admin, filtering the roster to the caller's tenant (god sees all). Dispatch-time isolation in `fleet_executor.rs` is already correct and is **not** touched.

## Acceptance Criteria **[REQUIRED]**

- [ ] `POST /agent/heartbeat` and `POST /agent/result` from a key whose tenant ≠ the agent's registered tenant → 403; matching tenant (or god) → unchanged behavior.
- [ ] `GET /agents` permitted for a tenant-admin; response contains only the caller tenant's agents (god: all).
- [ ] `GET /agent/artifact|source/{digest}` remain content-addressed with no tenant gate — documented as accepted (agents only learn their own dispatched digests).
- [ ] Integration tests for the heartbeat/result guard and the filtered roster. `angreal test integration` green.

## Implementation Notes

**Scope:** agent endpoint authZ hardening only; dispatch-time tenant matching is already enforced and untouched.
**Depends on:** T-0783 (route table + middleware).
**References:** I-0118 → "Phase 0 design" behavior changes 3–4; `crates/cloacina-server/src/routes/agent.rs` (heartbeat L110, result L140, list_agents L276); `crates/cloacina-server/src/agent_registry.rs` (`AgentRecord.tenant_id`); already-correct dispatch at `fleet_executor.rs:269-282`.

## Status Updates **[REQUIRED]**

*To be added during implementation*
