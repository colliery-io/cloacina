---
id: pak-key-generation-with-argon2
level: task
title: "PAK key generation with argon2 hashing"
short_code: "CLOACI-T-0187"
created_at: 2026-03-16T20:01:01.655376+00:00
updated_at: 2026-03-16T20:25:09.389847+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# PAK key generation with argon2 hashing

## Objective

Implement Prefixed API Key (PAK) generation and argon2 password hashing. This module produces keys in the `cloacina_<env>_<tenant>_<random>` format, hashes them for storage, and verifies presented keys against stored hashes.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `argon2` crate added as a dependency
- [ ] `generate_api_key(env, tenant_name) -> (full_key, prefix, hash)` produces correctly formatted PAK keys
- [ ] Random component is 16 bytes, hex-encoded (32 hex chars)
- [ ] Prefix is the first 8 characters after `cloacina_` (the env segment)
- [ ] Global keys (no tenant) use empty tenant segment: `cloacina_live__<random>`
- [ ] `verify_api_key(full_key, stored_hash) -> bool` correctly validates via argon2
- [ ] `extract_prefix(full_key) -> String` extracts the prefix used for cache/DB lookup
- [ ] Unit tests: key format validation, hash generation + verification round-trip, prefix extraction, global vs tenant-scoped key format

## Implementation Notes

### Key Format
- `cloacina_<env>_<tenant>_<random>` where random = 16 bytes hex-encoded
- Example tenant key: `cloacina_live_acme_k7f3a9b2c1d4e5f6`
- Example global key: `cloacina_live__k9a8b7c6d5e4f3g2` (double underscore, empty tenant)
- Prefix = env segment (first 8 chars after `cloacina_`), used for indexed DB lookup

### Hashing
- Use argon2id variant (recommended for API key hashing)
- Generate random salt per key via `argon2::password_hash::SaltString::generate`
- Store the full PHC-format hash string (includes algorithm, params, salt, hash)
- Verification uses `argon2::PasswordHash::new` + `PasswordVerifier::verify_password`

### Dependencies
- No DAL dependency — this is a pure crypto/formatting module
- Used by CLOACI-T-0186 (ApiKeyDAL::create) and CLOACI-T-0189 (AuthExtract verification)

## Status Updates

### 2026-03-16 — Completed
- Created security/api_keys.rs with generate_api_key(), extract_prefix(), hash_key(), verify_key()
- PAK format: cloacina_<env>_<tenant>_<random_hex>
- Prefix = env_tenant for cache lookup
- argon2 0.5 added to Cargo.toml
- 6 unit tests: format, unique keys, verify correct, verify wrong, prefix extraction, global prefix
- All pass in 0.59s
