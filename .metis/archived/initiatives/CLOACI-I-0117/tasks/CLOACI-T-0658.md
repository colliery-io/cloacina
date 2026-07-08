---
id: ui-api-key-management-create-one
level: task
title: "UI API key management — create (one-time plaintext), list, revoke"
short_code: "CLOACI-T-0658"
created_at: 2026-06-11T02:19:00.689036+00:00
updated_at: 2026-06-11T12:17:58.867075+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0651]
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI API key management — create (one-time plaintext), list, revoke

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Tenant-scoped API-key management (REQ-006, drives UC-4): `/keys` list + create + revoke over `client.createTenantKey()` / `client.listKeys()` / `client.revokeKey()`. The one-time-plaintext flow is the security-sensitive bit.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `/keys` — table of keys (name, role badge, created, active/revoked) over `useKeys`; no plaintext ever rendered (the list schema `KeyInfo` carries none).
- [x] **Create**: name + role (`Select`) → `useCreateKey` → `createTenantKey`; the plaintext `key` is held only in transient component state and shown **once** in a `closeOnClickOutside={false}` modal with a `CopyButton` and a "you won't see it again" warning. Dismissing nulls the state — never written to cache or sessionStorage, never re-displayed.
- [x] **Revoke**: confirm Modal → `useRevokeKey` → `revokeKey(id)`; list invalidates so the row flips to `revoked`.
- [x] Mutation errors classified via `classifyError` (REQ-007 — a 403 from a non-admin key surfaces as "Not authorized"); create/revoke invalidate the keys-list cache.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
The created key lives only in transient component state for the one-time display — never written to sessionStorage or query cache. Copy-to-clipboard + an explicit acknowledge-and-dismiss step.

### Dependencies
Blocked by CLOACI-T-0651. Independent of the other feature tasks.

### Risk Considerations
Don't let the one-time plaintext leak into TanStack Query cache, logs, or error reports. Note: creating a tenant-scoped key requires an admin-role key (server enforces `is_admin`); surface the 403 cleanly when the user's key can't.

## Status Updates **[REQUIRED]**

**2026-06-11 — Implemented, typechecks clean.**
- `ui/src/api/keys.ts` (new): `useKeys` (`client.listKeys`), `useCreateKey` (`client.createTenantKey({name, role})`), `useRevokeKey` (`client.revokeKey(id)`); create/revoke invalidate `queryKeys.keys(tenant)`. Re-exports `KeyInfo`/`KeyRole`/`KeyCreatedResponse` types from the SDK schemas.
- `ui/src/routes/Keys.tsx` (new): list table (role + active/revoked badges), create modal (name + role Select), one-time plaintext reveal modal (`CopyButton`, `closeOnClickOutside={false}`, dismiss nulls state), revoke confirm modal. Errors via `classifyError`.
- `ui/src/App.tsx`: replaced the `/keys` placeholder with `<Keys />`. Shell nav already had the API Keys link.
- **Security:** the plaintext `key` from `KeyCreatedResponse` lives only in transient `useState` for the one-time display — never written to the query cache (mutation results aren't cached), sessionStorage, or logs. The list endpoint's `KeyInfo` schema carries no plaintext/hash.
- **Note (carried from task):** minting a tenant key needs an admin-role key server-side; a non-admin key gets a 403, surfaced cleanly as "Not authorized".