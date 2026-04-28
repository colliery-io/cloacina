---
id: unified-cloacina-package-plugin
level: initiative
title: "Unified Cloacina package plugin shell — single FFI plugin per cdylib, inventory-driven"
short_code: "CLOACI-I-0102"
created_at: 2026-04-28T20:17:59.237354+00:00
updated_at: 2026-04-28T20:17:59.237354+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: unified-cloacina-package-plugin
---

# Unified Cloacina package plugin shell — single FFI plugin per cdylib, inventory-driven Initiative

## Context

I-0101 (CG/reactor decoupling) leaves Rust packaging asymmetric: a Rust cdylib needs exactly one `CloacinaPlugin` impl exported via fidius, and today that impl is auto-emitted by `#[computation_graph]` (and similarly for the workflow macro). A package whose only payload is `#[reactor]` declarations has no plugin emitter and cannot be loaded — the reconciler can't extract metadata or invoke anything in it. By contrast Python (post-T-0545 M3a) handles "reactor library" packages cleanly via the import side-effect path.

I-0101's locked decision says *"a single package may contain workflows, CGs with a reactor trigger, CGs without a trigger, and reactors. The reconciler routes each declaration by kind at load time; no separate 'CG package' vs. 'workflow package' distinction is required. The primitive model is about declaration shape, not packaging."* For Rust to honour that, the package shape needs to stop being defined by which declaration macro is present.

This initiative replaces the per-macro plugin emission with a single, inventory-driven plugin shell. Every Rust cdylib gets exactly one plugin (`cloacina::package!()` at the crate root), and that plugin walks the cdylib's local inventory at FFI call time to surface tasks, computation graphs, reactors, and triggers. Authoring becomes uniform: declare your primitives with attribute macros, add one shell macro, ship.

## Goals & Non-Goals

**Goals:**

- Authoring uniformity: one explicit shell macro (`cloacina::package!()`) is the only thing every Rust cdylib needs at the crate root. Adding `#[computation_graph]`, `#[workflow]`, `#[reactor]`, or `#[trigger]` declarations doesn't require choosing a different shell — the same shell handles any combination.
- Reactor-only Rust cdylibs work. A crate with only `#[reactor]` declarations (no graph, no workflow, no tasks) compiles, packages, loads, and the reconciler dispatches its reactors into the scheduler.
- Mixed packages work. A crate may legitimately declare reactors + a CG + a workflow + triggers in one cdylib; the unified plugin reports all of them.
- Existing in-tree packaged Rust crates migrate atomically (one PR adds `cloacina::package!();` to each lib.rs). No deprecation window.
- Existing FFI ABI is preserved or evolved cleanly. fidius's `interface_version = 1` stays valid for the four existing plugin trait methods. Any new method (e.g., reactor metadata) is additive with `#[serde(default)]` on the wire format and a default trait impl so older binaries continue to work.

**Non-Goals:**

- Changing how Python packages declare or package primitives. Python's import-side-effect model is already correct.
- Multiple computation graphs per Rust cdylib. Today's "one CG per package" constraint stays; lifting it is a separate piece of work driven by user need.
- Multiple workflows per cdylib. Same: existing constraint preserved.
- Distribution-side changes (CLI verbs, registry surface). The package shape on disk is unchanged — only the cdylib's exported plugin is restructured.
- Replacing fidius. The plugin transport stays as-is.

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

The *macro that ran* decides what plugin shape the cdylib has. A reactor-only crate has no such macro, so no plugin.

### Target

```
.cloacina archive
└── library.so / library.dylib  (the cdylib)
    └── single fidius plugin emitted by `cloacina::package!()`
        ├── get_task_metadata        (walks inventory::iter::<TaskEntry>)
        ├── execute_task             (looks up TaskEntry by name, dispatches)
        ├── get_graph_metadata       (walks inventory::iter::<ComputationGraphEntry>)
        ├── execute_graph            (looks up ComputationGraphEntry by name, dispatches)
        └── get_reactor_metadata     (NEW; walks inventory::iter::<ReactorEntry>)
```

`#[computation_graph]`, `#[workflow]`, `#[reactor]`, and `#[trigger]` macros each continue to do the work they already do for in-process Runtime seeding (submit inventory entries) but stop emitting any FFI plugin code. The shell macro `cloacina::package!()` is the single source of plugin emission.

### Wire format evolution

`get_reactor_metadata` is a new trait method. Two evolution options:

1. **Add a fifth trait method** (method index 4). Old plugins built before this initiative don't have it; the host's call returns a fidius "method not found" error, which the reconciler treats as "no reactors in this package" (matches today's behavior for Rust). New plugins built after the initiative populate it.
2. **Hijack `get_graph_metadata`** to also return reactor metadata via a `reactors_in_package: Vec<ReactorPackageMetadata>` field on `GraphPackageMetadata`. `#[serde(default)]` makes pre-initiative packages safe; the unified shell populates the field for new packages. Reactor-only packages return a synthetic `GraphPackageMetadata` with empty `graph_name` and populated `reactors_in_package`.

Decision deferred to discovery: option 1 is cleaner long-term but requires fidius to gracefully report "method not found"; option 2 keeps the ABI fully backward compatible at the cost of an awkward synthetic-metadata pattern for reactor-only packages.

### Authoring shape (target user contract)

```rust
// any-cdylib lib.rs

#[reactor(name = "pricing_rx", accumulators = [...], criteria = ...)]
pub struct PricingRx;

#[computation_graph(trigger = reactor(PricingRx), ...)]
mod pricing_graph { ... }

#[workflow]
mod my_workflow { ... }

cloacina::package!();   // single line; required exactly once per cdylib
```

The shell macro panics at compile time if invoked more than once in a crate (catches accidental double-emission).

## Detailed Design

### Phase 1 — unified plugin shell macro

Add `cloacina::package!()` (probably as a declarative macro in `cloacina-macros`). Macro emits, gated on `#[cfg(feature = "packaged")]`:

- A `pub mod _ffi { ... }` module containing a `pub struct CloacinaPackagePlugin` with the fidius `#[plugin_impl]` attribute.
- Method bodies that walk `inventory::iter::<EntryType>` for each plugin trait method.
- For `get_task_metadata` / `execute_task`: walk the cdylib's `TaskEntry` inventory, build `PackageTasksMetadata` from each entry, dispatch `execute_task` by name.
- For `get_graph_metadata` / `execute_graph`: walk `ComputationGraphEntry` inventory. Return error if zero entries (or empty if reactor-only); error if more than one (constraint still applies). Dispatch by name.
- For reactor metadata (whichever wire-format option lands): walk `ReactorEntry` inventory, build `Vec<ReactorPackageMetadata>`.

### Phase 2 — strip per-macro plugin emission

`#[computation_graph]` codegen: remove the `_ffi` module. Inventory submission stays.
`#[workflow]` codegen: same — drop FFI plugin emission, keep inventory.
`#[reactor]`, `#[trigger]`: unchanged (never emitted FFI plugins).

### Phase 3 — migrate in-tree crates

Find all packaged Rust crates (those with `feature = "packaged"` in their `Cargo.toml`). Add `cloacina::package!();` to each lib.rs. Identified candidates from the audit:

- `examples/features/workflows/simple-packaged`
- `examples/features/workflows/packaged-workflows`
- `examples/features/workflows/packaged-triggers`
- `examples/features/workflows/complex-dag`
- `examples/features/computation-graphs/packaged-graph`
- `examples/fixtures/compiler-broken-rust`
- `examples/fixtures/compiler-happy-rust`

(Final list verified during decomposition — there may be test fixtures under `crates/cloacina-server/tests/...` too.)

### Phase 4 — reconciler dispatch for reactor metadata

Following T-0545 M3a's Python pattern, the Rust CG branch in `crates/cloacina/src/registry/reconciler/loading.rs` dispatches reactors before the CG load. The reactor list comes from whichever wire-format option is chosen above. For reactor-only Rust packages (empty graph metadata), skip `scheduler.load_graph` entirely and only dispatch reactors.

Acceptable `package_type` values for a reactor-only Rust package: `["computation_graph"]` (degenerate, but the reconciler routes through that branch and the synthetic empty graph triggers the "skip load_graph" path) OR add `"reactor"` as an alias. Decision to be made during decomposition.

### Phase 5 — testing

- Reactor-only Rust cdylib: build under angreal harness, load, verify reactor reaches scheduler, push event, verify nothing crashes (no subscribers yet).
- Reactor-only Rust cdylib + a separate Rust CG cdylib referencing the reactor: cross-package fan-out via the reconciler-driven path. Mirrors the T-0544 M5 cross-language test but for two Rust packages.
- Existing CG/workflow integration tests: green after migration.
- `cargo check --workspace --all-features`, `angreal test unit`, `angreal test integration --backend sqlite/postgres` all green.

## Alternatives Considered

- **Per-macro plugin emission with a new `cloacina::reactor_package!()` macro for reactor-only packages.** Smaller change, no migration. Rejected because authoring stays asymmetric — users must choose which shell macro to use based on package contents. Doesn't honour I-0101's "declaration shape, not packaging" framing.
- **Inject the plugin shell automatically (no user-visible macro line).** Would require build-script orchestration or linker tricks; brittle and hard to debug. Explicit is better than implicit at the FFI boundary.
- **Change `#[reactor]` to emit the plugin shell on its first invocation in a crate.** Proc macros are per-invocation and can't reliably detect "first" without a global compile-time mechanism, and multiple `#[reactor]` declarations in one crate would conflict.

## Implementation Plan

Decomposition into tasks happens during the `decompose` phase. Likely shape:

1. **T-A — Unified plugin shell macro.** Add `cloacina::package!()`. Inventory-walking method bodies. Wire-format decision (new method vs. extended GraphPackageMetadata) settled here. Compile-test only — no migration yet.
2. **T-B — Reconciler reactor dispatch for Rust packages.** Mirror T-0545 M3a's Python pattern. Wire shell-emitted reactor metadata into `loading.rs` before `scheduler.load_graph`.
3. **T-C — Strip per-macro plugin emission + migrate in-tree crates.** Remove `_ffi` codegen from `#[computation_graph]` and `#[workflow]`. Add `cloacina::package!();` to every in-tree packaged Rust crate. Single PR; everything goes green together.
4. **T-D — Reactor-only Rust cdylib end-to-end test.** Build a minimal reactor-only crate via the angreal harness, load through the reconciler, assert dispatch + cross-package binding from a second Rust CG package. Both backends.
5. **T-E — Release notes addition.** Documents the migration: the one-line addition existing crate authors need to make. Folds into I-0101's T-0542 if T-0542 hasn't shipped, otherwise its own short release-notes task.
