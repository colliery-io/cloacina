---
id: unified-workflow-macro-single
level: initiative
title: "Unified workflow macro — single #[workflow] for embedded and packaged delivery"
short_code: "CLOACI-I-0058"
created_at: 2026-03-29T19:55:20.655315+00:00
updated_at: 2026-03-30T02:04:10.321663+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: unified-workflow-macro-single
---

# Unified workflow macro — single #[workflow] for embedded and packaged delivery Initiative

## Context

Cloacina currently has two separate macro systems for defining workflows:

1. **`workflow!` + loose `#[task]` fns** — for embedded workflows linked into binaries
2. **`#[packaged_workflow]` on a module** — for distributable `.cloacina` packages loaded via FFI

This creates confusion in the API surface, documentation, and mental model. Users must learn two different patterns. The API server (I-0049) adds a third context (upload + execute remotely) that makes the terminology drift worse — "workflow" vs "packaged workflow" vs "workflow package" appear throughout.

The core insight: **the task definition is identical in both modes**. The only real difference is the delivery mechanism (binary-linked vs dynamic library), which is a build concern, not a workflow authoring concern.

## Goals & Non-Goals

**Goals:**
- Single `#[workflow]` attribute macro that replaces both `workflow!` and `#[packaged_workflow]`
- `#[task]` stays exactly the same (already unified)
- Delivery mechanism controlled by Cargo.toml crate type + a feature flag (`packaged`), not by which macro you use
- All tutorials and examples converge on one pattern
- Deprecate and eventually remove `workflow!` and `#[packaged_workflow]`
- Unified terminology: "workflow" everywhere, "package" only for the `.cloacina` archive

**Non-Goals:**
- Changing the `#[task]` macro (it's already shared)
- Changing the runtime execution engine or scheduler
- Changing the `.cloacina` archive format
- Python workflow support (separate concern)

## Detailed Design

### Unified user-facing API

Two attribute macros, each a standalone top-level declaration:

```rust
// Workflow — defines tasks, focused purely on execution logic
#[workflow(
    name = "file_pipeline",
    description = "Process incoming files",
    author = "Team",
)]
pub mod file_pipeline {
    use super::*;

    #[task(id = "validate", dependencies = [])]
    pub async fn validate(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }

    #[task(id = "process", dependencies = ["validate"])]
    pub async fn process(ctx: &mut Context<Value>) -> Result<(), TaskError> { ... }
}

// Event trigger — custom poll logic, references workflow by name
#[trigger(on = "file_pipeline", poll_interval = "5s", allow_concurrent = false)]
pub async fn inbox_watcher() -> Result<TriggerResult, TriggerError> {
    // check for new files, return Fire(ctx) or Skip
}

// Cron trigger — cron expression IS the poll logic, no function body needed
#[trigger(on = "file_pipeline", cron = "0 2 * * *", timezone = "UTC")]
```

**Key principles:**
- `#[workflow]` is purely about tasks — no scheduling concerns mixed in
- `#[trigger]` is a single macro with two modes:
  - **Custom trigger** — applied to an async function with poll logic, uses `poll_interval`
  - **Cron trigger** — standalone declaration with `cron` parameter, no function body (framework provides the poll function)
- Cron is just a trigger with a built-in poll function — not a separate concept
- Multiple triggers can independently target the same workflow via `on`
b- Mirrors how Python already works: `@cloaca.trigger(workflow="...")`
- The `on` parameter is the binding between scheduling and execution

### Delivery mechanism selection

Controlled by feature flag in the workflow crate's `Cargo.toml`:

**Embedded (default):**
```toml
[dependencies]
cloacina-workflow = "0.x"
```
- Generates `#[ctor]` auto-registration into the global workflow registry
- Tasks namespaced as `{tenant}::{package_name}::{workflow_name}::{task_id}`
- `package_name` derived from `Cargo.toml` package name

**Packaged:**
```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
cloacina-workflow = { version = "0.x", features = ["packaged"] }
```
- Generates FFI exports (`cloacina_get_task_metadata`, `cloacina_execute_task`)
- Generates C-compatible metadata structures
- Same namespace scheme, same task definitions

### What changes in `cloacina-macros`

1. New `#[workflow]` attribute macro — combines the module-based approach of `#[packaged_workflow]` with the registration of `workflow!`
2. New `#[trigger]` attribute macro — two modes:
   - With function body + `poll_interval`: generates `Trigger` trait impl from the async function
   - With `cron` parameter, no body: generates a cron-based trigger using the framework's built-in schedule poll
   - Both use `on` to bind to a workflow name
3. Both macros check for `cfg(feature = "packaged")` at expansion time:
   - Without feature: `#[ctor]` registration for workflows, triggers auto-registered in global registry
   - With feature: FFI exports, manifest metadata generation (tasks + triggers in one manifest)
4. `workflow!` and `#[packaged_workflow]` kept temporarily as deprecated aliases, then removed
5. Existing `Trigger` trait stays for advanced use cases — `#[trigger]` is sugar that generates the impl

### Namespace unification

Currently:
- Embedded: `public::embedded::{workflow_name}::{task_id}` (hardcoded "embedded")
- Packaged: `public::{package}::{workflow_name}::{task_id}` (user-specified package)

Unified: `{tenant}::{crate_name}::{workflow_name}::{task_id}` — crate name from `CARGO_PKG_NAME` env var (available at compile time).

## Alternatives Considered

1. **Keep both macros, just rename** — Doesn't solve the fundamental two-system problem. Users still learn two patterns.
2. **Runtime detection instead of feature flag** — Proc macros can't read Cargo.toml at expansion time. Feature flag is the standard Rust mechanism for conditional compilation.
3. **Attribute parameter (`packaged = true`)** — Puts delivery concern in the workflow code, which is what we're trying to avoid. The whole point is that the same source code works for both.

## Implementation Plan

Phase 1: `#[workflow]` macro — replaces `workflow!` and `#[packaged_workflow]`, `packaged` feature flag for delivery selection
Phase 2: `#[trigger]` macro — custom poll + cron modes, `on` binding, auto-registration (embedded) or manifest entry (packaged)
Phase 3: Migrate all examples and tutorials to unified macros
Phase 4: Deprecate `workflow!` and `#[packaged_workflow]`
Phase 5: Terminology cleanup — "workflow" everywhere, "package" only for `.cloacina` archives
