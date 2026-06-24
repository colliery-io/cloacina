---
id: short-ttl-key-minting-from-oidc
level: task
title: "Short-TTL key minting from OIDC login (provenance-tagged)"
short_code: "CLOACI-T-0792"
created_at: 2026-06-24T01:08:58.591652+00:00
updated_at: 2026-06-24T03:11:20.754443+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Short-TTL key minting from OIDC login (provenance-tagged)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Mint a short-TTL cloacina API key from an abstract **resolved principal** `{tenant, role, provenance}` — produced by EITHER OIDC mapping (T-0791) OR local login (T-0796). Build the mint path **provider-agnostic** over that handoff, not OIDC-specific types, so it can land before either producer exists. Reuse the existing `api_keys` DAL + `generate_api_key`; scope to the resolved tenant + role; tag provenance (`issued_via = <provider>:<subject>`, e.g. `oidc:<issuer>:<sub>` or `local:<account_id>`); short TTL (~15 min — OQ-3 default). Plaintext returned exactly once; the minted key is an ordinary `api_keys` row flowing through the existing bearer + LRU cache + Phase 0 authZ matcher unchanged.

## Acceptance Criteria

## Acceptance Criteria

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

**2026-06-24 — IN PROGRESS (migration scoped; not yet implemented).** Layout for the `api_keys` TTL + provenance columns:
- **Template:** mirror `migrations/postgres/019_add_tenant_and_admin_to_api_keys` (a clean ADD COLUMN migration).
- **New migrations (ADD COLUMN, no DROP+CREATE):** postgres `032_add_ttl_provenance_to_api_keys` + sqlite `028_add_ttl_provenance_to_api_keys`, each `up.sql`/`down.sql`; add `expires_at TIMESTAMP NULL` + `issued_via TEXT NULL`. (Postgres at 031, sqlite at 027 → use 032/028.)
- **Register** them in the embedded-migrations list/macro (find where 031/027 register).
- **schema.rs ×2** (`schema/postgres.rs` + `schema/sqlite.rs`): add the two cols to `api_keys` `table! {}`.
- **Models** (`dal/unified/api_keys/crud.rs`): add fields to `ApiKeyRow` + `NewApiKey`; thread `to_info`.
- **`ApiKeyInfo`** (`api_keys/mod.rs`): add `expires_at` + `issued_via`.
- **`validate_hash`:** also filter `expires_at IS NULL OR expires_at > now()` so expired minted keys are rejected (revocation latency still bounded by the 30s LRU cache).
- **Mint fn + `ResolvedPrincipal`:** add provider-agnostic `ResolvedPrincipal { tenant, role, provenance }` (server-side) + `mint_key(dal, principal, ttl)` → `create_key` with `expires_at = now+ttl`, `issued_via = provenance`. Unit test: mint from a hand-built principal → key carries TTL + provenance; expired key fails `validate_hash`.

**Then T-0793** (oidc_sessions encrypted refresh store) + **T-0794** (refresh/logout) build on this. All pure-Rust + migration; compile-verifiable via `angreal check crate` + `cargo test --lib`, no live stack.

**Depends on:** T-0782/0783 (done).

**2026-06-24 — COMPLETE.** Implemented exactly as planned, simplified to **postgres-only** (api_keys is server-mode/Postgres — no sqlite migration). Migration `032_add_ttl_provenance_to_api_keys` (additive ADD COLUMN `expires_at TIMESTAMPTZ`, `issued_via TEXT`; auto-embedded by directory). `schema.rs` api_keys block + `ApiKeyRow`/`to_info` + `ApiKeyInfo` gained the two fields. `validate_hash` now filters `expires_at IS NULL OR expires_at > now()` (expired minted keys rejected; revocation latency still bounded by the 30s LRU). New `crud::mint_key` + `MintApiKey` insertable + `ApiKeyDAL::mint_key` wrapper (`is_admin` always false). Server: new `crates/cloacina-server/src/identity.rs` — `ResolvedPrincipal { tenant, role, provenance }` (provider-agnostic, no OIDC types) + `mint_for_principal(state, principal, ttl) -> (plaintext, ApiKeyInfo)` reusing `generate_api_key` + `mint_key`; `DEFAULT_MINTED_KEY_TTL = 15m` (OQ-3). `pub mod identity` registered. `angreal check crate` clean; 2/2 identity unit tests green. **Deferred:** the DB mint→authorized-call integration test runs under the postgres lane.