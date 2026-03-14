---
id: review-cli-nomenclature-and
level: task
title: "Review CLI nomenclature and subcommand organization"
short_code: "CLOACI-T-0059"
created_at: 2026-01-28T14:14:05.539285+00:00
updated_at: 2026-03-13T13:56:29.319441+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Review CLI nomenclature and subcommand organization

## Objective

Audit and standardize the Cloacina CLI structure for consistency, discoverability, and alignment with common CLI patterns.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: CLI is growing organically with package signing, key management, server commands. Risk of inconsistent naming.
- **Benefits of Fixing**: Better developer experience, easier to learn, easier to document.
- **Risk Assessment**: Low risk of not addressing immediately - can review after more CLI features exist.

## Current State

**Rust CLI (`cloacina`):** One command — `cloacina admin cleanup-events [--older-than] [--dry-run]`
**Python CLI (`cloaca`):** One command — `cloaca build [-o] [--target] [-v] [--dry-run]`
**Unexposed APIs:** Package signing (sign, verify), key management (generate, revoke, trust), audit logging — all shipped in I-0008 with no CLI surface.

## Decisions

- **Rename binary**: `cloacina` → `cloacinactl` — this is an operator control tool, not the runtime
- **`cloaca` stays independent**: Python SDK/packaging tool, standalone without Rust binary. Not a ctl — just "the name"
- **`cloacinactl` is the superset**: `cloacinactl package build` invokes cloaca packaging via embedded PyO3. Both tools can build packages, same underlying code.
- **Top-level = nouns** (domain groupings): `package`, `key`, `continuous`, `admin`
- **Subcommands = verbs**: `sign`, `verify`, `generate`, `list`, `revoke`, `prune-state`
- **Flags = kebab-case**: `--key-name`, `--older-than`, `--dry-run`
- **Positional args for primary subject**: `cloacinactl package sign <path>`, `cloacinactl key revoke <key-id>`
- **`--dry-run`** on any destructive command
- **`-v/--verbose`** stays global
- **Transport-agnostic**: CLI calls a service/boundary layer, not the DAL directly. Local (DB) vs remote (HTTP to server API) is handled at the boundary — CLI doesn't know or care. This means commands are automatically remote-capable when I-0018 lands.

## Target Command Tree

```
cloaca                              cloacinactl
├── build                           ├── package
                                    │   ├── build  ← (future) same code via PyO3
                                    │   ├── sign <path> --key-name <name>
                                    │   ├── verify <path>
                                    │   └── inspect <path>
                                    ├── key
                                    │   ├── generate --name <name>
                                    │   ├── list
                                    │   ├── export <key-id> --output <path>
                                    │   ├── revoke <key-id>
                                    │   └── trust
                                    │       ├── add <fingerprint> --name <name>
                                    │       ├── list
                                    │       └── revoke <key-id>
                                    ├── admin
                                    │   └── cleanup-events [--older-than] [--dry-run]
                                    └── continuous
                                        └── prune-state [--dry-run]  ← (I-0025)
```

## Acceptance Criteria

## Acceptance Criteria

- [x] Rename `cloacina-cli` crate → `cloacinactl`, update binary name
- [x] Restructure command tree: `package`, `key`, `key trust`, `admin` groups
- [x] Wire `package sign` using existing `PackageSigner` / `sign_package()` API
- [x] Wire `package verify` using existing `verify_package()` / `verify_package_offline()` API
- [x] Wire `package inspect` — read and display DetachedSignature from .sig file
- [x] Wire `key generate` using existing `generate_signing_keypair()` + `DbKeyManager`
- [x] Wire `key list` — query signing keys from DB
- [x] Wire `key export` — export public key (PEM or raw hex)
- [x] Wire `key revoke` — revoke a signing key via DB
- [x] Wire `key trust add` — add trusted key via `DbKeyManager` (PEM file)
- [x] Wire `key trust list` — list trusted keys
- [x] Wire `key trust revoke` — revoke trust
- [x] Move `admin cleanup-events` under new structure (no behavior change)
- [x] Update Cargo workspace references to new crate name
- [ ] `continuous prune-state` — stub or omit (no backing code until I-0025)
- [x] `package build` via PyO3 — embedded Python calls cloaca.cli.build directly

## Progress

### Session 2026-03-13
- Renamed directory `crates/cloacina-cli` → `crates/cloacinactl`
- Updated root `Cargo.toml` workspace members
- Updated crate `Cargo.toml`: package name → `cloacinactl`, binary name → `cloacinactl`
- Added deps: `hex`, `uuid`
- Restructured `main.rs`: three top-level groups (`Package`, `Key`, `Admin`) + nested `Trust`
- Added global `--org-id` / `CLOACINA_ORG_ID` for multi-tenant operations
- Created shared helpers in `commands/mod.rs`: `connect_db()`, `read_master_key()`, `parse_uuid()`
- Master key read from `CLOACINA_MASTER_KEY` env var (hex-encoded 32 bytes)
- Created `commands/package.rs`: sign (db key + .sig file), verify (online + offline), inspect
- Created `commands/key.rs`: generate, list, export (pem/raw), revoke
- Created `commands/key_trust.rs`: add (from PEM file), list, revoke
- Made `DbKeyManager::encode_public_key_pem` and `decode_public_key_pem` public (were private)
- All 11 existing unit tests pass, clean compile (no warnings in cloacinactl)
- Wired `package build` via PyO3 embedded Python — calls `cloaca.cli.build` directly, no subprocess
- Added `pyo3` (auto-initialize) and `pyo3-build-config` deps
- Added `build.rs` to set rpath for Python framework (macOS framework builds)
- Made `DbKeyManager::encode_public_key_pem` and `decode_public_key_pem` public for CLI use
- Remaining: `continuous prune-state` (deferred to I-0025)

## Implementation Plan

### Phase 1: Rename and restructure
- Rename `cloacina-cli` crate → `cloacinactl`
- Update `Cargo.toml` binary name, workspace members
- Restructure `Commands` enum: `Package`, `Key`, `Admin` top-level groups
- Move `cleanup-events` under `Admin` group (already there, just verify)
- Add `Key` group with `KeyCommands` and nested `KeyTrustCommands`
- Add `Package` group with `PackageCommands`

### Phase 2: Package commands
- `package sign <path> --key-name <name>` — load key from DB, sign archive, write .sig sidecar
- `package verify <path>` — verify signature against trusted keys in DB
- `package inspect <path>` — extract and pretty-print manifest.json from archive

### Phase 3: Key management commands
- `key generate --name <name>` — generate keypair, store in DB, display fingerprint
- `key list` — table output of signing keys (name, fingerprint, status, created)
- `key export <key-id> --output <path>` — write public key bytes to file
- `key revoke <key-id>` — mark key revoked in DB

### Phase 4: Trust chain commands
- `key trust add <fingerprint> --name <name>` — add external public key as trusted
- `key trust list` — table output of trusted keys
- `key trust revoke <key-id>` — revoke trust
