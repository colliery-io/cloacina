---
id: encrypted-server-side-refresh
level: task
title: "Encrypted server-side refresh-token store (oidc_sessions) + sweeper"
short_code: "CLOACI-T-0793"
created_at: 2026-06-24T01:09:28.346131+00:00
updated_at: 2026-06-24T01:09:28.346131+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Encrypted server-side refresh-token store (oidc_sessions) + sweeper

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Store the IdP refresh token server-side, encrypted at rest. Default storage (OQ-2): a DEDICATED `oidc_sessions` Postgres table (NOT extending `api_keys`), AES-GCM via the existing `aes-gcm` dependency, encryption key sourced from server config/env (KMS deferred + documented). The refresh token is never logged and never returned to the browser. State is multi-replica safe; a sweeper expires stale records using the ws-ticket/outbox sweeper pattern. Migration must be additive (ADD COLUMN / CREATE), never DROP+CREATE.

## Acceptance Criteria **[REQUIRED]**

- [ ] Refresh tokens are encrypted at rest in `oidc_sessions`; plaintext never logged or returned.
- [ ] A sweeper removes expired/stale refresh records.
- [ ] The migration is additive (no DROP+CREATE).
- [ ] Tests cover encrypt/decrypt round-trip and the sweep.

## Implementation Notes

**Scope:** the encrypted refresh store + sweeper + migration. Resolves OQ-2 to the dedicated-table + config-key default.
**Depends on:** Task 5 (minting).
**References:** CLOACI-I-0118 REQ-007, NFR-001, NFR-003; OQ-2 (defaulted to dedicated table + config/env key).

## Status Updates **[REQUIRED]**

*To be added during implementation*
