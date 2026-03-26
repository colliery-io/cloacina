---
id: cli-rename-testing-crate
level: initiative
title: "CLI Rename + Testing Crate — Foundation Tooling"
short_code: "CLOACI-I-0048"
created_at: 2026-03-26T05:34:47.887985+00:00
updated_at: 2026-03-26T05:34:47.887985+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: S
initiative_id: cli-rename-testing-crate
---

# CLI Rename + Testing Crate — Foundation Tooling Initiative

## Context

At v0.3.2, the CLI binary lives in `crates/cloacina-cli`. The server and daemon code already reference `cloacinactl` as the canonical control tool name. The mismatch needs to be resolved. Additionally, writing tests for workflow logic currently requires a live database connection, which slows iteration and makes CI fragile.

Both changes were previously implemented on the `archive/main-pre-reset` branch (commits `8025682` and `995f8a3`) and worked well. This initiative re-applies them cleanly on the current codebase with proper test coverage.

## Goals

- Rename `crates/cloacina-cli` to `crates/cloacinactl` (package name `cloacinactl`, binary remains `cloacina`)
- Create `crates/cloacina-testing` crate providing `TestRunner`, `test_db`, and `test_dal` helpers for no-DB unit testing of workflow logic
- All existing tests pass after the changes

## Non-Goals

- New CLI commands or features beyond the rename
- Server or daemon implementation (covered by subsequent initiatives)
- Changes to the core engine or scheduling crates

## Acceptance Criteria

- [ ] `crates/cloacina-cli` directory renamed to `crates/cloacinactl`
- [ ] `Cargo.toml` package name updated to `cloacinactl`; binary name stays `cloacina`
- [ ] All workspace references updated (root `Cargo.toml`, CI configs, documentation)
- [ ] `crates/cloacina-testing` crate exists with `TestRunner`, in-memory DAL, and test DB helpers
- [ ] `cloacina-testing` depends only on `cloacina-workflow` (no database crates)
- [ ] `angreal cloacina all` passes (full test suite + lints)

## Prior Art

Reference implementation on `archive/main-pre-reset`:
- CLI rename: commit `8025682` (feat(cli): rename cloacina-cli to cloacinactl)
- Testing crate: commit `995f8a3` (feat: add cloacina-testing crate)

## Implementation Notes

**CLI Rename** — Straightforward directory rename plus updating `members` in the workspace `Cargo.toml`, CI workflow paths, and any documentation references. The archive diff shows this is a clean rename with no logic changes.

**Testing Crate** — Port `cloacina-testing` from the archive branch. Key components:
- `TestRunner` — lightweight in-process executor that runs tasks in dependency order
- In-memory DAL and DB stubs so tests never touch a real database
- Optional `continuous` feature flag gated behind `chrono` for time-dependent test helpers

**Sequencing** — CLI rename first (smaller change, fewer conflicts), testing crate second.
