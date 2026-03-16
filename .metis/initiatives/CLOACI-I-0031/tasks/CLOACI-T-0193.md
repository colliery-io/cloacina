---
id: cli-cloacinactl-api-key-create
level: task
title: "CLI: cloacinactl api-key create/list/revoke subcommands"
short_code: "CLOACI-T-0193"
created_at: 2026-03-16T20:01:07.835939+00:00
updated_at: 2026-03-16T20:37:40.644485+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# CLI: cloacinactl api-key create/list/revoke subcommands

## Objective

Add `api-key` subcommands to `cloacinactl` for managing API keys from the command line: create, list, revoke, and a bootstrap `create-admin` command for initial setup.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ApiKey` subcommand group added to cloacinactl Commands enum
- [ ] `cloacinactl api-key create --tenant <name> --name <label> --read --write --execute --admin` creates a key, prints the secret once to stdout
- [ ] `cloacinactl api-key list --tenant <name>` lists keys for a tenant (metadata only: id, name, prefix, permissions, created_at, expires_at, revoked status — no secrets)
- [ ] `cloacinactl api-key revoke <key-id>` revokes a key by UUID
- [ ] `cloacinactl api-key create-admin` creates a global super-admin key (tenant_id = NULL, all permissions true)
- [ ] Secret key is displayed exactly once on create and never retrievable again
- [ ] Commands use TenantDAL and ApiKeyDAL for database operations
- [ ] Error handling: tenant not found, key not found, already revoked

## Implementation Notes

### CLI Structure
- Add to existing clap Commands enum in `crates/cloacinactl/src/main.rs`
- `ApiKey` variant with sub-enum: `Create`, `List`, `Revoke`, `CreateAdmin`
- Follow existing subcommand patterns in cloacinactl

### Create Flow
1. Resolve tenant by name via TenantDAL::get_by_name (for tenant-scoped keys)
2. Call `generate_api_key(env, tenant_name)` to get (full_key, prefix, hash)
3. Build NewApiKey with permission flags from CLI args
4. Call ApiKeyDAL::create(new_api_key)
5. Print full_key to stdout with warning that it won't be shown again

### Bootstrap (create-admin)
- No `--tenant` flag — creates a global key with tenant_id = NULL
- All permission bits set to true
- Intended for initial platform setup before any tenants exist

### Dependencies
- CLOACI-T-0186 (TenantDAL, ApiKeyDAL)
- CLOACI-T-0187 (generate_api_key)

## Status Updates

### 2026-03-16 — Completed
- Added ApiKey subcommand group: Create, List, Revoke, CreateAdmin
- commands/api_key.rs: create (resolve tenant, generate PAK, store hash + patterns, print secret once), list (table format), revoke, create_admin (global key with all permissions)
- Defaults to read-only if no permission flags specified
- Added list_all() to ApiKeyDAL for unfiltered listing
- `cloacinactl api-key --help` renders correctly
