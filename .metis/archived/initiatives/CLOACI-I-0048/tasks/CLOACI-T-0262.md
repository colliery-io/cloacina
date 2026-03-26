---
id: rename-cloacina-cli-to-cloacinactl
level: task
title: "Rename cloacina-cli to cloacinactl"
short_code: "CLOACI-T-0262"
created_at: 2026-03-26T05:49:17.069563+00:00
updated_at: 2026-03-26T06:03:04.199657+00:00
parent: CLOACI-I-0048
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0048
---

# Rename cloacina-cli to cloacinactl

## Parent Initiative

[[CLOACI-I-0048]]

## Objective

Rename the `crates/cloacina-cli` directory and package to `cloacinactl`, aligning with the canonical control tool name used throughout the server and daemon code. The binary name stays `cloacina`. This is a clean rename with no logic changes.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `crates/cloacina-cli` directory renamed to `crates/cloacinactl`
- [ ] `Cargo.toml` package name updated to `cloacinactl`; binary name stays `cloacina`
- [ ] Workspace `Cargo.toml` `members` list updated
- [ ] CI workflow paths updated if they reference `cloacina-cli`
- [ ] Documentation references updated
- [ ] `angreal cloacina unit` passes

## Implementation Notes

### Technical Approach
1. `git mv crates/cloacina-cli crates/cloacinactl`
2. Update `package.name` in `crates/cloacinactl/Cargo.toml` to `cloacinactl`
3. Update workspace `members` in root `Cargo.toml`
4. Search for any remaining `cloacina-cli` references in CI configs, docs, angreal tasks
5. Verify build and tests pass

### Prior Art
Reference: commit `8025682` on `archive/main-pre-reset`

### Dependencies
None — this task should be completed before CLOACI-T-0263.

## Status Updates

### 2026-03-26 — Complete
- `git mv crates/cloacina-cli crates/cloacinactl` — directory renamed
- Updated `crates/cloacinactl/Cargo.toml` package name to `cloacinactl` (binary stays `cloacina`)
- Updated workspace `Cargo.toml` members list
- Searched for `cloacina-cli` references in CI (`.yml`), docs, and angreal tasks (`.py`) — none found
- `cargo check --workspace` passes
- `angreal cloacina unit` passes — 307 tests (18 workflow + 289 cloacina), 0 failures
