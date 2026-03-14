---
id: code-example-validation-rust
level: task
title: "Code Example Validation — Rust Tutorials (01–10)"
short_code: "CLOACI-T-0098"
created_at: 2026-03-13T14:30:06.609138+00:00
updated_at: 2026-03-13T22:06:05.744609+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# Code Example Validation — Rust Tutorials (01–10)

**Phase:** 3 — Code Example Validation (Pass 2)
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Extract and validate every code example from Rust tutorials 01–10. Each code block must compile against current workspace dependencies and produce the described behavior.

## Scope

Files: `docs/content/tutorials/01-*.md` through `docs/content/tutorials/10-*.md`

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every Rust code block extracted and cataloged (file, line number, language tag)
- [ ] Each compilable example verified against current `cloacina`, `cloacina-workflow`, `cloacina-macros` crate APIs
- [ ] `Cargo.toml` dependency versions in tutorials match current workspace versions
- [ ] All `use` statements reference existing types/modules
- [ ] CLI commands (`cloacina-ctl`, `cargo` invocations) tested for correct syntax
- [ ] Any deprecated API usage flagged and fixed
- [ ] All broken examples fixed in-place
- [ ] Run `angreal demos tutorial-01` through `tutorial-06` to verify working examples match docs

## Implementation Notes

### Validation Approach
1. Extract code blocks with triple-backtick rust/toml/bash tags from each tutorial
2. For Rust blocks: check `use` paths resolve, types exist, method signatures match
3. For Cargo.toml blocks: verify dependency names, versions, features
4. For bash blocks: verify CLI command syntax (`cloacinactl package`, etc.)
5. Cross-reference with `examples/tutorials/` directory — tutorial code should match docs
6. Use `angreal demos tutorial-*` to run actual examples and verify output

### Known Risk Areas
- Tutorial examples may reference old `cloacina-cli` binary name (renamed to `cloacinactl`)
- Dependency versions may have drifted since tutorials were written
- `cloacina-workflow` vs `cloacina` dependency confusion in packaged workflow tutorials

## Status Updates

### Session 1 (2026-03-13)

**Cross-cutting fixes applied to all tutorials:**
- Added `cloacina-workflow` dependency to all 6 example Cargo.toml files (`examples/tutorials/01-06`)
- Updated version references from "0.1.0"/"0.2" to "0.3" in tutorial docs (01-04, 07)
- Added `env-filter` feature to `tracing-subscriber` in tutorial docs (01-04)
- Added `cloacina-workflow` to Cargo.toml code blocks in tutorial docs (01-04)

**Per-tutorial fixes:**
- **Tutorial 05** (Cron Scheduling): Fixed `DefaultRunnerConfig` from private field access to builder pattern in both docs and example code
- **Tutorial 06** (Multi-Tenancy): Major rewrite — replaced fictional Context API (`.with()`, `.set()`, `.get::<T>()`) with actual API (`context.insert()`, `context.get().and_then()`); fixed workflow creation to use `workflow!` macro; fixed `PipelineStatus` matching; fixed string concatenation syntax errors
- **Tutorial 07** (Packaged Workflows): Fixed binary name `cloacina-ctl` → `cloacinactl`; updated dependency version
- **Tutorial 08** (Workflow Registry): Rewrote registry setup from old API to DAL-based; fixed path references; removed obsolete `cloacina-ctl` dependency
- **Tutorial 10** (Task Handles): Fixed placeholder GitHub URL to `colliery-io`

**Verification:**
- Tutorials 01-05 pass via `angreal demos tutorial-01` through `tutorial-05`
- Tutorial 06 compiles correctly (requires PostgreSQL for full runtime)
- Hugo docs build passes cleanly
- Stale SQLite database files cleaned up (were causing false migration failures)
