---
id: identity-tenant-role-mapping
level: task
title: "Identity→tenant/role mapping policy (allowlist default, god-owned)"
short_code: "CLOACI-T-0791"
created_at: 2026-06-24T01:08:30.974333+00:00
updated_at: 2026-06-24T01:08:30.974333+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Identity→tenant/role mapping policy (allowlist default, god-owned)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Resolve a validated OIDC identity to ABAC `Principal` attributes (tenant + role) via a god-owned, config-driven ALLOWLIST policy: explicit `claim(group | email-domain | sub) → {tenant, role}` rules. An unmapped identity is denied. The default policy is allowlist; org/domain auto-map and first-login-approval are deferred variants (note them, do not build). Uses the same attribute vocabulary as the Phase 0 ABAC matcher.

## Acceptance Criteria **[REQUIRED]**

- [ ] A validated identity resolves to a `{tenant, role}` pair or is denied with a typed error when unmapped.
- [ ] Mapping rules are god-configured (single shared IdP assumption, OQ-11 god-owned).
- [ ] Tests cover mapped → correct tenant/role and unmapped → denial.

## Implementation Notes

**Scope:** the mapping policy only (allowlist). Resolves OQ-1 to the allowlist default; OQ-11 god-owned.
**Depends on:** Task 3 (validated identity); soft-depends on Phase 0 matcher (CLOACI-T-0782) for the shared Principal vocabulary — can be developed in parallel.
**References:** CLOACI-I-0118 REQ-005; OQ-1 (defaulted to allowlist), OQ-11 (god-owned).

## Status Updates **[REQUIRED]**

*To be added during implementation*
