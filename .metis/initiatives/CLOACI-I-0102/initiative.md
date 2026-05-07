---
id: unified-cloacina-package-plugin
level: initiative
title: "Unified Cloacina package plugin shell — single FFI plugin per cdylib, inventory-driven"
short_code: "CLOACI-I-0102"
created_at: 2026-04-28T20:17:59.237354+00:00
updated_at: 2026-04-30T04:06:30.357342+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: unified-cloacina-package-plugin
---

# Unified Cloacina package plugin shell — single FFI plugin per cdylib, inventory-driven Initiative

## Context

I-0101 (CG/reactor decoupling) leaves Rust packaging in an awkward shape on two distinct axes:

1. **No plugin shell for primitive-only cdylibs.** A Rust cdylib needs exactly one `CloacinaPlugin` impl exported via fidius. Today that impl is auto-emitted by `#[computation_graph]` (and similarly the workflow macro). A crate whose only payload is `#[reactor]` and/or `#[trigger]` declarations has no plugin emitter and cannot be loaded — the reconciler can't extract metadata or invoke anything in it. By contrast Python (post-T-0545 M3a) handles primitive-only packages cleanly via the import-side-effect path.

2. **Cross-primitive references are type-path-coupled.** Today `#[computation_graph(trigger = reactor(MyReactor))]` and `#[task(invokes = computation_graph(MyHandle))]` accept Rust *type paths*. The macro emits compile-time subset/handle checks, which is great for fail-fast but it means a CG can only reference a reactor whose Rust type is in scope at compile time. Cross-package binding (a CG in package B referencing a reactor in package A, no shared types) is impossible. Workflow → trigger binding has its own asymmetry: it lives in `[[triggers]]` stanzas in `package.toml`, not in the macro layer at all.

I-0101's locked decision says *"a single package may contain workflows, CGs with a reactor trigger, CGs without a trigger, and reactors. The reconciler routes each declaration by kind at load time; no separate 'CG package' vs. 'workflow package' distinction is required. The primitive model is about declaration shape, not packaging."* For Rust to honour that, two things change:

- Package shape stops being defined by which declaration macro is present.
- Cross-primitive references become string-named, decoupled from compile-time type visibility.

This initiative is the architectural refactor that makes both true. After it lands:

- Every Rust cdylib has exactly one plugin shell, emitted by an explicit `cloacina::package!()` line at the crate root. The shell walks inventory at FFI call time to surface whatever primitives the cdylib happens to declare — tasks, computation graphs, reactors, triggers — in any combination, including the empty intersection (reactor-only, trigger-only).
- Cross-primitive references in declaration macros use string names: `trigger = reactor("R")`, `invokes = computation_graph("G")`, `triggers = ["T1", "T2"]`. Type paths are removed. Compile-time subset/handle checks go away — replaced by runtime contract validation at load time (already implemented via T-0544 M2's `check_reactor_contract_matches`).
- The `[[triggers]]` stanza in `package.toml` and the `package_type` field move into the macro layer or get removed entirely. `package.toml` is reduced to package-identity metadata that Cargo can't already provide (and could probably shrink further in a follow-up).

The result is symmetric authoring: declare your primitives with attribute macros — including their string-named upstream references — add `cloacina::package!();` once, ship. Same shape regardless of whether the cdylib has tasks, graphs, reactors, triggers, or any combination.

## Goals & Non-Goals

**Goals:**

- **One plugin shell per cdylib.** Single explicit `cloacina::package!();` at the crate root. Works for any combination of declarations. Reactor-only and trigger-only Rust cdylibs become first-class.
- **String-named cross-primitive references.** Macros accept names, not type paths. Cross-package authoring works without import gymnastics.
- **Macro-only declaration surface.** Trigger subscriptions move from `[[triggers]]` in `package.toml` to a `triggers = […]` argument on `#[workflow]`. `package_type` removed (no longer drives routing).
- **Precedence-ordered package loader.** Reconciler's `loading.rs` rebuilt around a single fixed-order pipeline: cron → custom triggers → reactors → trigger-less CGs → reactor-bound CGs → workflows. Same pipeline for Rust and Python — only the metadata-extraction step differs.
- **Pre-1.0 atomic migration.** All in-tree packaged Rust crates rewritten in a single PR to use `cloacina::package!();`, string-named references, and macro-arg trigger subscriptions. No deprecation window for the type-path forms; deprecation warnings (one cycle) for `[[triggers]]` and `package_type` in `package.toml`, then full removal in the same initiative.
- **`Trigger` trait extension** — `cron_expression() -> Option<&str>` so the unified shell can route cron-based triggers to the cron scheduler vs. custom triggers to the runtime registry.
- **fidius `#[optional(since = N)]` capability bits** carry the new optional `get_reactor_metadata` and `get_trigger_metadata` methods. Plugins built before this initiative return `CallError::NotImplemented` cleanly and the reconciler treats that as "no entries of that kind."

**Non-Goals:**

- **Changing Python authoring.** Python's import-side-effect path already gets this right; only the reconciler-side dispatch needs to align with the new precedence-ordered loader.
- **Multiple CGs or multiple workflows per Rust cdylib.** Today's "one CG, one workflow per package" constraint stays. Lifting either is a separate piece of work driven by real user need.
- **Distribution-side changes** (CLI verbs, registry surface, package archive layout). The `.cloacina` archive format is unchanged.
- **Replacing fidius.** The plugin transport stays as-is. We rely on fidius 0.2's `#[optional(since = N)]` mechanism.
- **Reducing `package.toml` further than removing `[[triggers]]` + `package_type`.** Other manifest cleanups (deriving identity from Cargo.toml, etc.) are tempting but stay scoped out — separate initiative if and when.

## Locked decisions (2026-04-30)

- **Single string form, no type-path fallback.** `trigger = reactor("R")` only — no `trigger = reactor(R)` for same-crate use. Two forms is two things to document, two test matrices, and the savings from compile-time checks are small once the runtime check exists.
- **Compile-time subset/handle checks removed.** T-0543 M4's `__cloacina_check_reactor_binding_<mod>` const-eval block goes away. T-0540 M3's `<H as TriggerlessGraph>` trait-bound check on `#[task(invokes = …)]` goes away. Runtime contract validation at load/registration time is the replacement.
- **`[[triggers]]` and `package_type`** get deprecation warnings during the I-0102 transition (both forms work in parallel through T-C); full removal in T-E once everything is migrated. Hard error if either is set in `package.toml` after the migration.
- **Wire-format evolution: option (a) — new optional trait methods.** fidius 0.2's `#[optional(since = N)]` mechanism carries `get_reactor_metadata` and `get_trigger_metadata`. T-0546's probe confirmed `CallError::NotImplemented { bit }` is the clean degradation path.
- **Shell macro single-emission check.** `cloacina::package!();` invoked twice in the same crate fails at compile time via a duplicate-mod-name pattern.
- **Loader precedence order:** cron triggers → custom triggers → reactors → trigger-less CGs → reactor-bound CGs → workflows. Within a single package and across packages.
- **Cross-package ordering is fail-fast.** A subscriber package that loads before its publisher rejects with a clear error naming the missing primitive. No pending-bindings queue.

## Architecture

### Today

```
.cloacina archive
└── library.so / library.dylib  (the cdylib)
    └── exported fidius plugin emitted by #[computation_graph] OR #[workflow]
        ├── get_task_metadata        (workflow path)
        ├── execute_task             (workflow path)
        ├── get_graph_metadata       (CG path)
        └── execute_graph            (CG path)
```

The *macro that ran* decides what plugin shape the cdylib has. A reactor-only or trigger-only crate has no such macro, so no plugin. Cross-primitive references are type-path-coupled and don't cross package boundaries.

### Target

```
.cloacina archive
└── library.so / library.dylib  (the cdylib)
    └── single fidius plugin emitted by `cloacina::package!()`
        ├── get_task_metadata        (walks inventory::iter::<TaskEntry>)
        ├── execute_task             (looks up TaskEntry by name, dispatches)
        ├── get_graph_metadata       (walks inventory::iter::<ComputationGraphEntry>)
        ├── execute_graph            (looks up ComputationGraphEntry by name, dispatches)
        ├── get_reactor_metadata     (NEW, #[optional(since = 2)]; walks ReactorEntry)
        └── get_trigger_metadata     (NEW, #[optional(since = 2)]; walks TriggerEntry)
```

`#[computation_graph]`, `#[workflow]`, `#[reactor]`, `#[trigger]`, `#[task]` continue to do the work they do today *for in-process Runtime seeding* (submit `inventory::submit!{...}` entries) but stop emitting any FFI plugin code. They're pure declarators.

The shell macro `cloacina::package!()` is the single source of plugin emission, expanded gated on `#[cfg(feature = "packaged")]`.

### Authoring contract — target user shape

**Publishers** name themselves:

```rust
#[reactor(name = "pricing_rx",
          accumulators = [orderbook, pricing],
          criteria = when_any(orderbook, pricing))]
pub struct PricingRx;

#[trigger(name = "nightly_run", cron = "0 2 * * *")]
async fn nightly_run() -> TriggerResult { /* ... */ }
```

**Subscribers** name their upstream by string:

```rust
#[computation_graph(
    name = "score_signals",
    trigger = reactor("pricing_rx"),       // string, not a type path
    entry_accumulators = [orderbook, pricing],
    graph = { ... },
)]
mod score_signals { ... }

#[workflow(
    name = "nightly_report",
    triggers = ["nightly_run"],            // strings, in the macro
)]
mod nightly_report {
    #[task(invokes = computation_graph("score_signals"))]   // string name
    async fn score_step(ctx: ...) -> ... { ... }
}
```

**Every cdylib** gets one shell line:

```rust
cloacina::package!();   // single line; required exactly once per cdylib
```

### Trade we accept by going string-only

| | Today (T-0543 M4 + T-0540 M3) | After I-0102 |
|---|---|---|
| `#[computation_graph(trigger = reactor(R))]` | Type path. Macro emits a `const fn` subset check that asserts the graph's entry accumulators ⊆ `<R as Reactor>::ACCUMULATORS` at compile time. | String name. No compile-time subset check. Runtime contract validation at bind time (T-0544 M2's `check_reactor_contract_matches`) catches mismatch — load fails with a clear error. |
| `#[task(invokes = computation_graph(H))]` | Type path. `<H as TriggerlessGraph>` trait bound gates the call at compile time; reactor-triggered graphs don't impl that trait → "trait not implemented" compile error. | String name. Runtime lookup at task registration time (already implemented in T-0541 M3 — reactor-triggered targets rejected at decoration). |

We lose **compile-time fail-fast** for cross-primitive type matching. We gain **cross-package authoring works at all**. Runtime checks still fail-fast, just at load time instead of compile time. The error messages are operator-facing rather than compiler-facing.

### Reconciler precedence-ordered loader

Single pass per package, regardless of contents. Replaces today's `package_type`-branched soup in `crates/cloacina/src/registry/reconciler/loading.rs`.

```
load_package(plugin, manifest):
  1. Cron triggers      → cron_scheduler.register
  2. Custom triggers    → runtime.register_trigger
  3. Reactors           → scheduler.load_reactor    (idempotent contract-match)
  4. Trigger-less CGs   → runtime.register_triggerless_graph  (consumed by @task(invokes=…))
  5. Reactor-bound CGs  → scheduler.bind_graph_to_reactor
                          (lookup upstream reactor by name; hard-error if missing)
  6. Workflows          → register_workflow + bind workflow → triggers
                          (lookup each name in workflow.triggers list; hard-error if missing)
```

Same pipeline for Rust and Python. Only the metadata-extraction step differs:

- **Rust:** call the plugin's `get_*_metadata` methods via fidius. Optional methods that aren't implemented return `CallError::NotImplemented` → empty list for that primitive kind.
- **Python:** post-import, walk the scoped Runtime's registries (`reactor_names()`, etc.). Same dispatch helpers as the Rust path.

Each step is fail-fast: any error aborts the package load. Cross-package ordering errors (e.g. CG loaded before its reactor, or workflow loaded before its trigger) surface as clean rejection messages.

`package_type` field — removed from routing. Deprecation warning if present in `package.toml` during the migration; hard error after T-E.

`[[triggers]]` stanzas — same: deprecation warning, then removed. Workflows declare their trigger subscriptions in `#[workflow(triggers = […])]`.

## Detailed Design

### Wire-format additions

Two new metadata structs in `cloacina-workflow-plugin/src/types.rs`:

```rust
/// Publisher-side metadata for a reactor declared in a cdylib.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactorPackageMetadata {
    pub name: String,
    pub package_name: String,
    pub reaction_mode: String,                   // "when_any" | "when_all"
    pub accumulators: Vec<AccumulatorDeclarationEntry>,
}

/// Publisher-side metadata for a trigger declared in a cdylib.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPackageMetadata {
    pub name: String,
    pub package_name: String,
    pub poll_interval: String,                   // "10s", "1m", etc.
    #[serde(default)]
    pub cron_expression: Option<String>,         // Some => routed to cron scheduler
    #[serde(default)]
    pub allow_concurrent: bool,
}
```

`PackageTasksMetadata` gains:

```rust
/// Trigger names this workflow subscribes to. Sourced from
/// `triggers = [...]` on `#[workflow]`. Replaces the deprecated
/// `[[triggers]]` stanza in package.toml.
#[serde(default)]
pub triggers: Vec<String>,
```

### `CloacinaPlugin` trait — version 2

```rust
#[fidius::plugin_interface(version = 2, buffer = PluginAllocated)]
pub trait CloacinaPlugin: Send + Sync {
    fn get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError>;          // 0
    fn execute_task(&self, req: TaskExecutionRequest)
        -> Result<TaskExecutionResult, PluginError>;                                   // 1
    fn get_graph_metadata(&self) -> Result<GraphPackageMetadata, PluginError>;        // 2
    fn execute_graph(&self, req: GraphExecutionRequest)
        -> Result<GraphExecutionResult, PluginError>;                                  // 3

    #[optional(since = 2)]
    fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError>; // 4
    #[optional(since = 2)]
    fn get_trigger_metadata(&self) -> Result<Vec<TriggerPackageMetadata>, PluginError>; // 5
}
```

### Macro changes

**Declaration macros** (publisher-side) — additive: each accepts a `name = "…"` argument that becomes the runtime identifier. Already true for `#[reactor]`, `#[trigger]`. `#[computation_graph]` already has `name`. `#[workflow]` already has `name`.

**Subscription macros** (subscriber-side) — switch to string args:

- `#[computation_graph(trigger = reactor("R"), ...)]` — string. Drops the type-path form.
- `#[task(invokes = computation_graph("G"))]` — string. Drops the type-path/`__CGHandle_<name>` form.
- `#[workflow(name = "W", triggers = ["T1", "T2"])]` — new `triggers = […]` argument; drops the manifest `[[triggers]]` stanza.

**Codegen removals:**

- T-0543 M4's `__cloacina_check_reactor_binding_<mod>` const-eval block.
- T-0540 M3's `<H as TriggerlessGraph>` trait-bound check.
- The `_ffi` module emitted by `#[computation_graph]` and the workflow macro.

**Codegen additions:**

- New shell macro `cloacina::package!()` emits the fidius `_ffi` module and the `CloacinaPackagePlugin` impl.
- `#[trigger]` codegen records the cron expression (if any) on the trait-implementing object so the unified shell can call `cron_expression()` at metadata-build time.

### Unified plugin shell — what `cloacina::package!()` expands to

Gated on `#[cfg(feature = "packaged")]`:

```rust
pub mod _ffi {
    use cloacina_workflow_plugin::*;

    pub struct CloacinaPackagePlugin;

    #[fidius::plugin_impl(CloacinaPlugin, crate = "cloacina_workflow_plugin")]
    impl CloacinaPlugin for CloacinaPackagePlugin {
        fn get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError> {
            // walk inventory::iter::<TaskEntry>() + the workflow's triggers list
        }
        fn execute_task(&self, req: TaskExecutionRequest)
            -> Result<TaskExecutionResult, PluginError>
        {
            // walk inventory::iter::<TaskEntry>(), find by name, dispatch
        }
        fn get_graph_metadata(&self) -> Result<GraphPackageMetadata, PluginError> {
            // walk inventory::iter::<ComputationGraphEntry>(); 0/1 expected
        }
        fn execute_graph(&self, req: GraphExecutionRequest)
            -> Result<GraphExecutionResult, PluginError>
        {
            // walk inventory and dispatch the (single) CG
        }
        fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError> {
            // walk inventory::iter::<ReactorEntry>()
        }
        fn get_trigger_metadata(&self) -> Result<Vec<TriggerPackageMetadata>, PluginError> {
            // walk inventory::iter::<TriggerEntry>(), call constructor to query
            // poll_interval / cron_expression / allow_concurrent on each
        }
    }
}

// Single-emission guard — duplicate cloacina::package!() invocations
// fail at compile time with a duplicate-mod-name error.
mod __cloacina_package_marker { struct Once; }
```

### Reconciler precedence-ordered loader

Replaces the per-`package_type` branching in `crates/cloacina/src/registry/reconciler/loading.rs`. New top-level `load_package(plugin, manifest)` runs:

1. **Cron triggers** — `get_trigger_metadata` entries with `cron_expression.is_some()` → cron scheduler register.
2. **Custom triggers** — `get_trigger_metadata` entries with `cron_expression.is_none()` → `runtime.register_trigger`.
3. **Reactors** — `get_reactor_metadata` → `scheduler.load_reactor` (idempotent on contract).
4. **Trigger-less CGs** — from `get_graph_metadata` if the CG has no `trigger` declared → `runtime.register_triggerless_graph` (consumed by `@task(invokes=...)`).
5. **Reactor-bound CGs** — from `get_graph_metadata` if the CG has `trigger_reactor: Some(name)` → `scheduler.bind_graph_to_reactor`. Hard-error if the upstream reactor isn't already loaded.
6. **Workflows** — from `get_task_metadata` → register tasks + bind workflow to each trigger named in `metadata.triggers`. Hard-error if any named trigger isn't already loaded.

Same pipeline for Python. Only the metadata-extraction step differs (plugin handle vs. scoped Runtime walk post-import).

`package_type` and `[[triggers]]` in `package.toml`: deprecation warnings during the migration window; hard error after T-E removes the legacy reads.

### `Trigger` trait extension

```rust
pub trait Trigger: Send + Sync {
    fn poll_interval(&self) -> Duration;
    fn allow_concurrent(&self) -> bool;
    fn poll(&self) -> /* ... */;

    /// NEW: cron expression if this is a cron-driven trigger. Default
    /// returns None — only cron-shaped triggers override.
    fn cron_expression(&self) -> Option<String> { None }
}
```

### Migration sequence

1. **T-A** — adds the new wire-format structs, the version-2 trait, the unified shell macro, and updates declaration macros to accept string-named references. Per-macro plugin emission stays in place during this task to keep the workspace green; coexistence is OK because the version-2 host gracefully handles version-1 plugins via fidius's optional-method mechanism.
2. **T-B** — rebuilds `loading.rs` around the precedence-ordered loader. Both legacy paths (per-`package_type` branches) and new path coexist via dispatch on whether the plugin's metadata methods return non-empty results.
3. **T-C** — strips per-macro plugin emission. Migrates every in-tree packaged Rust crate to `cloacina::package!();` + string-named references + macro-arg trigger subscriptions. Single PR, atomic — pre-1.0 breaking change.
4. **T-D** — fixture crates + end-to-end tests. Reactor-only Rust cdylib, trigger-only Rust cdylib, mixed package, cross-package fan-out across two cdylibs.
5. **T-E** — remove deprecated `[[triggers]]` and `package_type` reads. Hard error if either appears in `package.toml` going forward. Release notes added; folds into I-0101's T-0542 if not yet shipped.

## Alternatives Considered

- **Per-macro plugin emission with a new `cloacina::reactor_package!()` macro for reactor-only packages.** Smaller change, no migration. Rejected because authoring stays asymmetric — users must choose which shell macro to use based on package contents. Doesn't honour I-0101's "declaration shape, not packaging" framing.
- **Inject the plugin shell automatically (no user-visible macro line).** Would require build-script orchestration or linker tricks; brittle and hard to debug. Explicit is better than implicit at the FFI boundary.
- **Change `#[reactor]` to emit the plugin shell on its first invocation in a crate.** Proc macros are per-invocation and can't reliably detect "first" without a global compile-time mechanism, and multiple `#[reactor]` declarations in one crate would conflict.
- **Keep type-path references as a compile-time-checked option alongside string refs.** Two forms is two test matrices, two doc paths, two migration stories. The runtime contract check catches the same class of error at load time with operator-facing messages. Not worth the dual-maintenance.
- **Hijack `get_graph_metadata` to also carry reactor + trigger metadata** (extend `GraphPackageMetadata` rather than add new optional methods). Avoids the new methods but forces a synthetic-empty-graph pattern for primitive-only packages. fidius 0.2's `#[optional(since = N)]` mechanism is cleaner; T-0546's probe confirmed graceful `CallError::NotImplemented` degradation works.
- **Wait on the precedence-ordered loader refactor** (just dispatch reactors next to the existing CG-load code). Tempting but it leaves `loading.rs` with even more `package_type` branching. The refactor pays for itself once we start handling triggers symmetrically.
- **Move trigger declarations into `[[triggers]]` permanently** instead of into the workflow macro. Rejected because (a) it leaves an asymmetric "this primitive lives in TOML, those primitives live in Rust attributes" surface, and (b) cross-package binding through TOML doesn't compose any better than through macros — both are name-based — but TOML is harder to validate and refactor.

## Implementation Plan

Five tasks, sequential dependency chain (T-A → T-B → T-C → T-D → T-E), with T-E doing the manifest-deprecation cleanup.

1. **T-A — Unified plugin shell macro + macro-layer string refs** (CLOACI-T-0547). Adds `cloacina::package!()`, the version-2 trait with new optional methods, the new wire-format structs, and the macro changes that accept string-named cross-primitive references. Drops compile-time subset/handle checks in declaration macros. Per-macro plugin emission unchanged in this task.
2. **T-B — Reconciler precedence-ordered loader** (CLOACI-T-0548, expanded). Rebuilds `loading.rs` around the fixed-order pipeline. Wires `get_reactor_metadata` and `get_trigger_metadata` dispatch. Same pipeline for Rust and Python. Cron vs. custom trigger routing.
3. **T-C — Strip per-macro plugin emission + migrate in-tree crates** (CLOACI-T-0549, expanded). Removes `_ffi` modules from `#[computation_graph]` and `#[workflow]` codegen. Migrates every in-tree packaged Rust crate to `cloacina::package!();`, string-named subscriber refs, and macro-arg trigger subscriptions. Single atomic PR.
4. **T-D — End-to-end test matrix for cdylib shapes** (CLOACI-T-0550, expanded). New fixtures: reactor-only Rust cdylib, trigger-only Rust cdylib, mixed package. Cross-package binding tests (subscriber in one cdylib, publisher in another). Both backends.
5. **T-E — Manifest cleanup + deprecation removal** (NEW). Removes the legacy `[[triggers]]` and `package_type` reads from the reconciler. Hard error if either appears in `package.toml`. Release-notes content folded into I-0101's T-0542 if T-0542 hasn't shipped, otherwise stand-alone short release-notes task.
