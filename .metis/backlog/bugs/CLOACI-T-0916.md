---
id: server-ha-truthfulness-in-memory
level: task
title: "Server HA truthfulness — in-memory agent roster and WS tickets contradict the chart HA claim"
short_code: "CLOACI-T-0916"
created_at: 2026-08-02T16:33:28.701827+00:00
updated_at: 2026-08-02T23:13:55.174712+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


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

## Acceptance Criteria

- [ ] Agent heartbeats + selection correct behind a round-robin LB with 2 replicas (or chart hard-documents affinity and refuses/warns on replicas>1 without it)
- [ ] WS tickets redeemable on any replica (or same affinity gate)
- [ ] A crashed CG no longer fails /ready platform-wide; its state is visible via health routes
- [ ] docs service/explanation/horizontal-scaling.md updated to the resulting truth

## Status Updates

- 2026-08-02: Filed from the architecture deep dive (control-plane report; DEEPDIVE.md risk register). Verified against main @ 5216e632.
