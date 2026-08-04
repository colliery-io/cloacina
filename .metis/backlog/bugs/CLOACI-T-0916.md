---
id: server-ha-truthfulness-in-memory
level: task
title: "Server HA truthfulness — in-memory agent roster and WS tickets contradict the chart HA claim"
short_code: "CLOACI-T-0916"
created_at: 2026-08-02T16:33:28.701827+00:00
updated_at: 2026-08-04T01:22:06.237472+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Server HA truthfulness — in-memory agent roster and WS tickets contradict the chart HA claim

## Objective

Make the server's multi-replica story truthful. The deep dive found HA is "80% real": the delivery substrate, leader election (advisory lock + DB cooldown), and OIDC flow state are genuinely multi-replica-safe, but two components are per-replica in-memory and nothing documents the resulting affinity requirement — while the Helm chart advertises HA-safety.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

## Findings

1. AgentRegistry (the fleet agent roster) is per-replica in-memory: behind a non-affine LB, agent heartbeats land on different replicas and the roster flaps — same-tenant agent selection sees a partial fleet, and dead-agent reclaim can fire against agents that are alive on another replica.
2. WsTicketStore (60s single-use WS tickets) is per-replica: a ticket minted on replica A fails on replica B, breaking browser WS attach nondeterministically behind an LB.
3. One crashed CG marks /ready false REPLICA-WIDE, so a single bad package can eject every replica from the LB pool simultaneously (readiness is shared-fate with tenant workload health).
4. charts/cloacina-server presents itself as HA-safe (deliberately no HPA but replica-count friendly) with no documented session-affinity requirement.

Fix options (choose per component): move roster + tickets to DB rows (both are tiny, TTL-friendly; the delivery outbox pattern already exists), or gate replicas>1 in the chart behind documented sticky-session requirements. For /ready: scope readiness to platform health, report tenant-workload health via /v1/health/* instead. Reactive-layer HA (reactor state) stays T-0851 — reference, not duplicated here.

## Acceptance Criteria

## Acceptance Criteria

- [x] Agent roster DB-backed (045_create_fleet_agents: upsert-register, CAS heartbeat, recency-filtered list_live, exactly-once CAS-delete sweep); all cross-replica reads converted; two-DAL-handle tests prove selection/reclaim see other replicas' agents
- [x] WS tickets DB-backed (044_create_ws_tickets, LoginFlowStore pattern; atomic CAS redeem); cross-replica redeem + 8-concurrent-redeemers-exactly-one-wins tests
- [x] /ready platform-scoped; crashed CG stays visible via /v1/health/graphs (test: /ready 200 with crashed graph listed)
- [x] horizontal-scaling.md (+ api-server, kubernetes how-tos) updated; residual affinity STATED (key-pool self-heal for secret dispatch; reactive placement = T-0851)

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (control-plane report; DEEPDIVE.md risk register). Verified against main @ 5216e632.
- 2026-08-03: DONE — merged to main in PR #231 (squash). Key verification that shaped the design: work packets already ride the delivery outbox and NoRoute leaves rows pending for the socket-holding replica (A-0006) — dispatch never needed affinity; the fix was reads-only. Deliberately replica-local: capacity snapshot for sync TaskExecutor trait methods; D-5 one-time key pools (now self-establishing on every replica via full-deficit reporting). Landing frictions worth remembering: the /ready reword changed the OpenAPI spec (spec-check gate) AND the generated TS client types (check:generated gate) — both regen steps are mandatory for any route-annotation change; and an unverified emit redirect truncated openapi.json to zero bytes once (always verify emitted artifacts before committing).
