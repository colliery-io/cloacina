---
id: t5-package-verbs-build-pack
level: task
title: "T5: package verbs — build/pack/publish/upload/list/inspect/delete"
short_code: "CLOACI-T-0514"
created_at: 2026-04-17T17:00:00+00:00
updated_at: 2026-04-18T01:40:07.687524+00:00
parent: CLOACI-I-0098
blocked_by: [CLOACI-T-0513]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0098
---

# T5: package verbs — build/pack/publish/upload/list/inspect/delete

## Parent Initiative

CLOACI-I-0098 — cloacinactl CLI redesign

## Objective

Implement the full `package` noun verb surface. Two local verbs (`build`, `pack`), four server verbs (`upload`, `list`, `inspect`, `delete`), and one composite (`publish` = build + pack + upload).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `package build <DIR> [--debug|--release]` — runs `cargo build [--release]` in `<DIR>`. Validates `package.toml` exists. Prints the built cdylib path.
- [ ] `package pack <DIR> [--out <PATH>] [--sign <KEY>]` — invokes `fidius_core::package::pack_package`. If `--sign` provided, signs via existing `security::package_signer`. Prints the output `.cloacina` path.
- [ ] `package publish <DIR> [--release] [--sign <KEY>]` — build + pack + upload in one shot. Temp dir cleanup on exit. Prints uploaded package UUID.
- [ ] `package upload <FILE>` — multipart POST `/v1/workflows`. Prints package UUID.
- [ ] `package list [--filter <PAT>]` — GET `/v1/workflows`. Columns: ID (truncated), NAME, VERSION, UPLOADED, TENANT.
- [ ] `package inspect <ID>` — GET `/v1/workflows/{id}`. Human-readable summary; `-o json` returns raw metadata.
- [ ] `package delete <ID> [--force]` — DELETE `/v1/workflows/{id}`. Confirmation prompt without `--force`.
- [ ] All subcommands respect global flags (`--profile`, `--tenant`, `-o`, etc.) via the T4 `CliClient`.
- [ ] Integration tests covering `build → pack → upload → inspect → delete` roundtrip against a fixture server.

## Implementation Notes

### `build`

```rust
let status = std::process::Command::new("cargo")
    .arg("build")
    .arg(if release { "--release" } else { "--debug" })
    .current_dir(&dir)
    .status()?;
```

Locate the produced cdylib by reading `target/{debug|release}` and matching the package name in `Cargo.toml`.

### `pack`

`fidius_core::package::pack_package(&source_dir, output_opt)`. `--sign` path loads the Ed25519 key via existing `security::keypair` helpers and signs via `package_signer`.

### `publish`

Orchestrates the three steps with a tempfile for the intermediate `.cloacina`. Temp dir dropped on any error — use a `tempfile::TempDir` guard.

### Confirmation prompt

`package delete` and other destructive verbs use a shared `confirm(...)` helper: reads from stdin unless `--force` or non-TTY.

## Status Updates

*To be added during implementation*
