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

Mint a short-TTL cloacina API key from an abstract **resolved principal** `{tenant, role, provenance}` — produced by EITHER OIDC mapping (T-0791) OR local login (T-0796). Build the mint path **provider-agnostic** over that handoff, not OIDC-specific types, so it can land before either producer exists. Reuse the existing `api_keys` DAL + `generate_api_key`; scope to the resolved tenant + role; tag provenance (`issued_via = <provider>:<subject>`, e.g. `oidc:<issuer>:<sub>` or `local:<account_id>`); short TTL (~15 min — OQ-3 default). Plaintext returned exactly once; the minted key is an ordinary `api_keys` row flowing through the existing bearer + LRU cache + Phase 0 authZ matcher unchanged.

## Acceptance Criteria **[REQUIRED]**

- [ ] A resolved principal (from any provider) yields a working scoped key authorized identically to an equivalent manually-created key.
- [ ] The mint API takes an abstract resolved principal `{tenant, role, provenance}` — no OIDC-specific types in its signature.
- [ ] The key carries provider provenance + a short TTL; it appears as an ordinary `api_keys` row.
- [ ] Integration test of the mint → authorized-call path (the minted key passes the normal auth middleware).

## Implementation Notes

**Scope:** minting + provenance + TTL + the provider-agnostic resolved-principal handoff type. Needs an `api_keys` schema touch for TTL + provenance (additive migration, no DROP+CREATE).
**Depends on:** T-0782 / T-0783 (matcher + middleware). The resolved-principal handoff is provider-agnostic — OIDC mapping (T-0791) and local login (T-0796) are both producers and may land before or after this task.
**References:** CLOACI-I-0118 REQ-006 + "Local accounts strand" (provider-agnostic handoff); OQ-3 (TTL ~15 min).

## Status Updates **[REQUIRED]**

*To be added during implementation*
