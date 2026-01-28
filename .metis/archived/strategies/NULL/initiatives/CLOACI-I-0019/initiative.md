---
id: slim-packaged-workflow-ffi
level: initiative
title: "Slim Packaged Workflow FFI Interface"
short_code: "CLOACI-I-0019"
created_at: 2026-01-28T05:15:48.005647+00:00
updated_at: 2026-01-28T13:55:36.434445+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
strategy_id: NULL
initiative_id: slim-packaged-workflow-ffi
---

# Slim Packaged Workflow FFI Interface Initiative

## Context

The current `#[packaged_workflow]` macro generates code that depends on the **full cloacina crate**, resulting in bloated .so/.dylib files (~1.1MB+ for trivial workflows). Analysis shows the compiled binary includes:

- Diesel database drivers (postgres/sqlite)
- Full tokio runtime
- Complete cloacina Context and Workflow types
- Global task registry

This happens because the macro generates FFI functions that do too much:

```rust
// cloacina_execute_task creates its own tokio runtime
let runtime = match tokio::runtime::Runtime::new() { ... }

// cloacina_create_workflow uses full cloacina types
let mut workflow = cloacina::workflow::Workflow::new(...);
let task_registry = cloacina::task::global_task_registry();
```

The package is trying to be both the **definition** and the **executor**, when it should only be the definition.

## Goals & Non-Goals

**Goals:**
- Reduce packaged workflow binary size by 80%+ (target: <200KB for simple workflows)
- Remove dependency on full `cloacina` crate from workflow packages
- Packages depend only on `cloacina-workflow` (minimal types)
- Server/host provides runtime, context management, and task registry
- Support both Rust and future Python workflow packages with same slim model
- Maintain backward compatibility for existing package loading

**Non-Goals:**
- Changing the `#[task]` macro interface (task authoring stays the same)
- Removing FFI entirely (still needed for dynamic loading)
- Supporting standalone package execution (packages require a host)

## Architecture

### Current Design (Bloated)

```
┌─────────────────────────────────────────────────────────┐
│ Workflow Package (.cloacina)                            │
├─────────────────────────────────────────────────────────┤
│ Dependencies: cloacina (full), tokio (full), diesel     │
├─────────────────────────────────────────────────────────┤
│ Contents:                                               │
│ ├── Task business logic (necessary)                     │
│ ├── cloacina_execute_task() - creates own runtime       │
│ ├── cloacina_create_workflow() - uses global registry   │
│ └── Statically linked: diesel, full cloacina types      │
└─────────────────────────────────────────────────────────┘

Problem: Macro generates glue code that pulls in full cloacina crate.
The task code itself is fine - it's the wrapper functions that bloat.
```

### Target Design (Slim)

```
┌─────────────────────────────────────────────────────────┐
│ Workflow Package (.cloacina)                            │
├─────────────────────────────────────────────────────────┤
│ Dependencies: cloacina-workflow, serde_json, tokio-min  │
├─────────────────────────────────────────────────────────┤
│ Contents:                                               │
│ ├── Task business logic (same as before)                │
│ ├── cloacina_get_task_metadata() - metadata only        │
│ └── cloacina_execute_task() - uses slim Context type    │
└─────────────────────────────────────────────────────────┘

Key insight: cloacina_workflow::Context IS the Context type.
Tasks already use it. The bloat comes from the macro generating
code that imports full cloacina for Workflow/registry operations.
```

### What Changes

**Task authoring stays the same:**
```rust
// This doesn't change - cloacina_workflow::Context is the same API
#[task(id = "my_task")]
async fn my_task(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    ctx.insert("key", value)?;
    Ok(())
}
```

**What the macro generates changes:**
```rust
// REMOVE: cloacina_create_workflow() - host creates workflows from metadata
// REMOVE: imports from full cloacina crate

// KEEP: cloacina_get_task_metadata() - metadata for host
// KEEP: cloacina_execute_task() - but using cloacina_workflow types only

// The execute function becomes simpler:
#[no_mangle]
pub extern "C" fn cloacina_execute_task(...) -> i32 {
    // Use tokio runtime (still needed for async)
    // Create cloacina_workflow::Context from JSON
    // Call task function
    // Return context as JSON
    // NO dependency on full cloacina crate
}
```

### Dependency Chain

```
Current:
  Package → cloacina (full) → diesel, tokio, DAL, scheduler, etc.
                ↓
            ~1.1MB binary

Target:
  Package → cloacina-workflow → serde, chrono, async-trait
                ↓
            ~200KB binary (estimate)
```

## Detailed Design

### Phase 1: Audit Current Dependencies

1. Identify all `cloacina::` imports in generated macro code
2. Map which imports can be replaced with `cloacina_workflow::` equivalents
3. Identify any gaps in `cloacina-workflow` that need filling

### Phase 2: Refactor Macro Output

1. Remove `cloacina_create_workflow()` - host builds workflows from metadata
2. Change `cloacina_execute_task()` to use only `cloacina_workflow::Context`
3. Remove all `cloacina::workflow::*` and `cloacina::task::*` imports
4. Keep `cloacina_get_task_metadata()` as-is (already slim)

### Phase 3: Fix simple-packaged Example

1. Move `cloacina` from `[dependencies]` to `[dev-dependencies]`
2. Verify lib.rs compiles with only `cloacina-workflow`
3. Tests still use full `cloacina` (they test loading/execution)
4. Measure binary size reduction

### Phase 4: Verify & Document

1. Build and measure binary sizes before/after
2. Run all existing package loading tests
3. Update documentation for package authors
4. Update simple-packaged as reference example

## Alternatives Considered

### Alternative A: VTable Pattern for Context Operations

- **Pros**: Complete decoupling, host provides all context ops
- **Cons**: Complex FFI, harder to debug, unnecessary since cloacina_workflow::Context exists
- **Decision**: Rejected. Simpler to just use cloacina_workflow types directly.

### Alternative B: WASM Instead of Native FFI

- **Pros**: Sandboxed, portable, slim
- **Cons**: Performance overhead, async complexity, ecosystem maturity
- **Decision**: Defer to future. Native FFI is already working; optimize it first.

### Alternative C: Keep Current Design, Just Strip Binaries

- **Pros**: No code changes needed
- **Cons**: Still links unnecessary code, larger than necessary
- **Decision**: Rejected. Stripping helps but doesn't fix the fundamental issue.

## Implementation Plan

### Phase 1: Audit & Measure
- [ ] Document all `cloacina::` imports in packaged_workflow macro output
- [ ] Identify which are replaceable with `cloacina_workflow::` equivalents
- [ ] Measure current binary size of simple-packaged example
- [ ] Identify gaps in cloacina-workflow (if any)

### Phase 2: Macro Refactor
- [ ] Remove `cloacina_create_workflow()` from generated code
- [ ] Update `cloacina_execute_task()` to use `cloacina_workflow::Context`
- [ ] Remove `cloacina::workflow::*` and `cloacina::task::*` imports
- [ ] Test that macro output compiles without full cloacina

### Phase 3: Example & Verification
- [ ] Fix simple-packaged Cargo.toml (cloacina → dev-dependency)
- [ ] Verify example compiles and binary is smaller
- [ ] Run existing integration tests
- [ ] Measure final binary size

### Phase 4: Documentation
- [ ] Update package authoring guide
- [ ] Document recommended Cargo.toml for workflow packages
- [ ] Add binary size targets to CI (optional)

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Approach | Slim dependencies, not vtable | Tasks already use cloacina_workflow::Context; just stop pulling in full cloacina |
| Breaking change | Minimal | Task authoring unchanged; only macro output changes |
| cloacina_create_workflow | Remove from packages | Host creates workflows from metadata; packages don't need registry access |
