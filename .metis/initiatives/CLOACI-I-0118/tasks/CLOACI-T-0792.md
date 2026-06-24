---
id: short-ttl-key-minting-from-oidc
level: task
title: "Short-TTL key minting from OIDC login (provenance-tagged)"
short_code: "CLOACI-T-0792"
created_at: 2026-06-24T01:08:58.591652+00:00
updated_at: 2026-06-24T01:08:58.591652+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Short-TTL key minting from OIDC login (provenance-tagged)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

On a successful login + mapping, mint a cloacina API key via the existing `api_keys` DAL + `generate_api_key`, scoped to the resolved tenant + role, tagged with provenance (`issued_via = oidc:<issuer>:<sub>` or similar), with a short TTL (default ~15 min — OQ-3 default, revisit in hardening). The plaintext key is returned exactly once. The minted key is an ordinary `api_keys` row and flows through the existing bearer + LRU cache + Phase 0 authZ matcher unchanged.

## Acceptance Criteria **[REQUIRED]**

- [ ] Login yields a working scoped key authorized identically to an equivalent manually-created key.
- [ ] The key carries OIDC provenance and a short TTL; it appears as an ordinary `api_keys` row.
- [ ] Integration test of the mint → authorized-call path (the minted key passes the normal auth middleware).

## Implementation Notes

**Scope:** minting + provenance + TTL. Needs an `api_keys` schema touch for TTL + provenance (additive migration, no DROP+CREATE).
**Depends on:** Task 4 (mapping).
**References:** CLOACI-I-0118 REQ-006; OQ-3 (TTL default ~15 min).

## Status Updates **[REQUIRED]**

*To be added during implementation*
