---
id: api-surface-audit-rust-public-api
level: task
title: "API Surface Audit — Rust Public API vs cargo doc Output"
short_code: "CLOACI-T-0103"
created_at: 2026-03-13T14:30:12.506424+00:00
updated_at: 2026-03-14T02:02:39.543213+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# API Surface Audit — Rust Public API vs cargo doc Output

**Phase:** 4 — API Surface Audit (Pass 3)
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Audit all Rust public API documentation against actual `cargo doc` output. Ensure every documented type, trait, method, and module in the docs matches the current public API surface of the `cloacina`, `cloacina-workflow`, and `cloacina-macros` crates.

## Scope

Rust API references in tutorials, explanations, and any reference documentation.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Run `cargo doc --no-deps` for each crate to generate current API docs
- [ ] Compare documented public types against actual `pub` exports in `lib.rs`
- [ ] Verify documented trait methods match actual trait definitions
- [ ] Verify documented struct fields and methods match implementations
- [ ] Verify macro usage syntax matches actual proc macro signatures
- [ ] Flag any public API not mentioned in documentation (coverage gaps)
- [ ] Flag any documented API that no longer exists (stale docs)
- [ ] All discrepancies fixed in-place

## Implementation Notes

### Crates to Audit
- **cloacina** — core library: Task trait, Context, DAL, executor, scheduler, registry, security
- **cloacina-workflow** — workflow authoring types: Context, Task, TaskError, RetryPolicy
- **cloacina-macros** — proc macros: `#[task]`, `workflow!`

### Approach
1. `cargo doc --no-deps` on workspace to generate HTML docs
2. Inspect `src/lib.rs` re-exports for each crate — these define the public API
3. Compare against every type/method reference in documentation
4. Pay special attention to re-exports that may have changed names

### Known Risk Areas
- `cloacina-workflow` is a minimal subset — docs may reference types that only exist in full `cloacina`
- Re-export paths may have changed (e.g., `cloacina::Context` vs `cloacina_workflow::Context`)
- New types added in recent features may not be documented yet

## Status Updates

### Session 1 (2026-03-13)

**Validation approach:**
- Inspected lib.rs re-exports for all 3 crates
- Cross-referenced tutorial and explanation docs against actual public API

**Fix applied:**
- **Tutorial 08** (Workflow Registry): Fixed DefaultRunnerConfig from direct field assignment to builder pattern. Fields are private; must use `DefaultRunnerConfig::builder().field(value).build()`.

**Already verified in prior tasks (T-0098, T-0100):**
- Tutorial 01, 05, 07, 09, 10: all use correct API patterns
- All `pub use` re-exports in lib.rs match what tutorials import
- `TaskHandle`, `DefaultRunner`, `DefaultRunnerConfig` all correctly exported
- `cloacina-workflow` correctly exports lightweight subset

**Verification:** Hugo docs build passes.
