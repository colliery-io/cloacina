# Legibility Review

## Summary

Cloacina has a clear architectural skeleton — a small number of named concepts (workflow, computation graph, trigger, reactor, accumulator, package), a strict noun-verb CLI, a deliberate authoring/engine/binary crate split, and a published spec (CLOACI-S-0011) that fixes vocabulary and bans drift. When the codebase aligns with that skeleton it reads well: `cloacinactl` nouns are one-verb-per-file, the unified `DAL` accessor pattern is consistent, the `Runtime` registry surface is symmetric across primitive kinds, and the `dispatcher::traits` trait docs are textbook.

The legibility damage concentrates in the computation-graph layer and in the in-flight I-0102 packaging migration. Three of the most visible types in the CG runtime carry banned terminology in their primary doc comments (`/// The Reactive Scheduler.`, `/// Declaration of a computation graph to be loaded by the Reactive Scheduler.`, `/// Shared between the Reactive Scheduler …`) — the spec is published, the rename was tracked under T-0528, but the source comments missed the sweep and rustdoc HTML now ships the banned phrases publicly. The `RunningGraph` storage struct is keyed by reactor name yet bound to a variable named `graph_name` in three places, and `AccumulatorSpawnConfig.graph_name` is silently fed `reactor_name.clone()` — the reactor/graph distinction the spec works hardest to fix is leaking back through internal field names.

The second concentration is doc-vs-reality drift left behind by aggressive refactors. Tutorial-style comments still reference removed APIs (`workflow!` declarative macro, `with_global_workflows_and_recovery`, "Resolve task from global registry"); a `pub mod global_registry` exists at the public crate boundary even though its sole content is a re-export plus a comment saying the global registry was deleted; the `cloacina::package!()` macro doc says "Six methods" while the trait it builds has nine; `crates/cloacina-macros/src/workflow_attr.rs` still produces 130 lines of FFI shell tokens that are immediately discarded with `let _ = packaged_registration;`. Each individual instance is small. Together they give a newcomer the wrong map of the system.

## Key Themes

- **Banned-phrase debt in CG runtime source.** CLOACI-S-0011 R1 is published and explicit — "reactive scheduler / reactive computation graph / reactive subsystem" are banned. Three production source comments still use them, and rustdoc has propagated the strings into `docs/public/api/`.
- **Reactor/graph naming conflation in scheduler internals.** Field names, variable bindings, and config struct members named `graph_name` carry reactor names; `RunningGraph` is the wrong noun for a struct keyed by reactor. The spec drew the line; the post-T-0544 refactor crossed it.
- **Stale doc comments referencing deleted APIs.** Module docs and rustdoc examples still cite `workflow!`, `with_global_workflows`, "global registry", and a `package_type` migration that was already removed. Newcomers reading docs will write code that doesn't compile.
- **In-flight I-0102 leaves dead-code branches and misleading guard rails.** `package!()` says it emits 6 methods (it emits 9); `workflow_attr.rs` builds a packaged-FFI block then discards it; `pub mod global_registry` survives as a public re-export module name with no registry behind it.
- **The 2,458-line `reconciler/loading.rs` carries the cognitive load of all six load steps plus three language branches plus FFI plumbing**. Step helpers exist (`step_load_cron_triggers` … `step_load_workflows`) but `load_package` itself is the bottleneck — it doesn't extract the per-language driver into a separate function.
- **Three different things are called "scheduler"**, and one of them is named just `Scheduler` — making `cron_trigger_scheduler.rs` host a public `Scheduler` while the file name and module path don't match.
- **Packaged-cdylib runtime initialization is replicated four times** — `OnceLock<Runtime>` blocks inside `cloacina::package!()` for tasks, CG, triggers, and trigger-less CGs all copy the same `Builder::new_multi_thread().worker_threads(2)` body. A helper would shorten the macro and make the per-shape parameters obvious.

## Findings

### LEG-01: Banned spec-prohibited terminology lives in three production source comments and ships in rustdoc

**Severity**: Major
**Location**: `crates/cloacina/src/computation_graph/scheduler.rs:38, 266`; `crates/cloacina/src/computation_graph/registry.rs:189`
**Confidence**: High

#### Description
CLOACI-S-0011 (`Naming rules / R1`) bans "reactive scheduler", "reactive computation graph", and "reactive subsystem" outright, with a specific rollout task (T-0528) tracking the cleanup. The renames landed for type names and HTTP routes, but three doc comments on load-bearing types in the CG runtime were missed. Because rustdoc renders these into the public HTML under `docs/public/api/cloacina/computation_graph/scheduler/index.html`, the banned terms ship publicly. A reader with the spec in hand is told "this is banned" and then sees it in the doc for the very type the spec exists to rename.

#### Evidence
- `crates/cloacina/src/computation_graph/scheduler.rs:38` — `/// Declaration of a computation graph to be loaded by the Reactive Scheduler.`
- `crates/cloacina/src/computation_graph/scheduler.rs:266` — `/// The Reactive Scheduler.` directly above `pub struct ComputationGraphScheduler`. The struct itself is renamed; only the doc is stale.
- `crates/cloacina/src/computation_graph/registry.rs:189` — `/// Shared between the Reactive Scheduler (registers on spawn) and …`
- `.metis/specifications/CLOACI-S-0011/specification.md:100` — the explicit ban list.
- `docs/public/api/cloacina/computation_graph/scheduler/index.html` — confirms the strings shipped.
- A separate spec file (`.metis/specifications/CLOACI-S-0007/specification.md`) still uses "Reactive Scheduler" in section headings (lines 236, 238, 243…); per the spec's own rollout note, "Archived task docs and completed initiatives are not rewritten — they are historical records," but `S-0007` is a published specification, not an archive.

#### Suggested Resolution
Three-line patch on the source comments. For `S-0007`: either rewrite the section to use "computation graph scheduler" (it's a published spec, not historical), or add a one-line note pointing readers to `S-0011`. After the source patch, regenerate rustdoc.

---

### LEG-02: `RunningGraph` and `AccumulatorSpawnConfig.graph_name` carry reactor names — the reactor/graph distinction the spec enforces leaks through internal field names

**Severity**: Major
**Location**: `crates/cloacina/src/computation_graph/scheduler.rs:69-76, 220-252, 376, 804-825, 837`
**Confidence**: High

#### Description
The post-T-0544 fan-out reshaped the runtime so a single reactor instance can host multiple graph subscribers. The user-facing model is correct: `ComputationGraphScheduler.reactors` is a `HashMap<String, RunningGraph>` keyed by reactor name, and `graph_to_reactor` maps subscribers back. But the names of the storage struct, fields it carries, and the variable bindings around it never moved with the model. A reader walking through `load_reactor` sees a `RunningGraph` being constructed, thinks they're in the per-graph subscriber path, and is confused that there are no per-subscriber fields. The fix would have been to rename the struct to `RunningReactor` or `ReactorInstance` when the fan-out landed.

#### Evidence
- `scheduler.rs:220` — `struct RunningGraph` is the storage record for one running reactor + its accumulator handles + its subscriber map.
- `scheduler.rs:69-76` — `pub struct AccumulatorSpawnConfig { …, graph_name: String, … }` with the doc `Graph name (used as key for checkpoint persistence).`
- `scheduler.rs:376` — `graph_name: reactor_name.clone()` — passing reactor name into the field labeled "graph name". Checkpoint rows are now keyed by reactor under a field called graph.
- `scheduler.rs:804-825` — `list_graphs()` iterates `self.graph_to_reactor` and returns `GraphStatus { name: graph_name.clone(), … }`, then dereferences into the `reactors` map by `reactor_name`. Reading the body, you have to keep "the inner graph_name is a subscriber graph, the outer one is just the loop variable, and the keys of `reactors` are reactor names" in your head simultaneously.
- `scheduler.rs:837` — `for (graph_name, running) in graphs.iter_mut()` where `graphs = self.reactors.write().await`. The variable bound to `running` is `RunningGraph`, but `graph_name` here is a reactor name.
- `accumulator.rs:141-146` — `CheckpointHandle { dal, graph_name, accumulator_name }` propagates the same misnaming into the DAL key derivation.

#### Suggested Resolution
- Rename `RunningGraph` → `RunningReactor` (or `ReactorInstance`).
- Rename `AccumulatorSpawnConfig.graph_name` → `reactor_name`. Same for `CheckpointHandle.graph_name` if the storage key really is per-reactor — and if it's per-graph, fix the call site at `scheduler.rs:376`.
- Audit any `graph_name`-bound iterator variables in `scheduler.rs` that actually iterate the reactors map.

**Cross-cutting note**: Correctness should verify whether `CheckpointHandle.graph_name` is being used to key a per-reactor or per-graph storage row. If multiple graph subscribers were ever to share a reactor with distinct checkpoint state, the current key derivation would collide.

---

### LEG-03: `pub mod global_registry` and `pub mod types` are public modules that contain only re-exports plus a comment saying the registry was deleted

**Severity**: Major
**Location**: `crates/cloacina/src/computation_graph/global_registry.rs:1-25`; `crates/cloacina/src/computation_graph/types.rs:1-23`; `crates/cloacina/src/computation_graph/mod.rs:27`
**Confidence**: High

#### Description
T-0509 deleted the process-global computation-graph registry. The implementation went away; the file `global_registry.rs` did not. Today the file is 24 lines long, says explicitly "The process-global computation-graph registry was removed in CLOACI-T-0509," and re-exports three types from the leaf crate. Worse, it's `pub mod global_registry` at the public crate boundary, so anyone reading the docs sees a "Global Registry" page (`docs/content/api-reference/rust/cloacina/computation_graph/global_registry.md` — which says "Global computation graph registry" as the description) for a thing that doesn't exist. The same shape applies to `types.rs` (`pub mod types` exporting from `cloacina_computation_graph`). Module names actively misdirect.

#### Evidence
- `crates/cloacina/src/computation_graph/global_registry.rs:17-24` — module-level doc says "Computation graph constructor types. The process-global computation-graph registry was removed in CLOACI-T-0509."
- `crates/cloacina/src/computation_graph/mod.rs:27` — `pub mod global_registry;` (still public).
- `crates/cloacina/src/computation_graph/mod.rs:42` — `pub use global_registry::ComputationGraphRegistration;` — the type is already re-exported from the parent module under its real name; the inner module is dead surface.
- `docs/content/api-reference/rust/cloacina/computation_graph/global_registry.md:4` — published doc says "Global computation graph registry. Re-exported from `cloacina-computation-graph` crate." The page tells the reader something that contradicts the source comment.
- `crates/cloacina/src/computation_graph/types.rs:1-23` — same pattern.

#### Suggested Resolution
Inline the three re-exports into `computation_graph/mod.rs` and delete `global_registry.rs` and `types.rs`. Regenerate the rustdoc html so the misleading pages disappear.

---

### LEG-04: README and engine `lib.rs` quick-start examples use a `workflow!` macro syntax that does not exist

**Severity**: Major
**Location**: `README.md:73-77`; `crates/cloacina/src/lib.rs:75, 99-105`
**Confidence**: High

#### Description
Both the README's first code sample and the engine crate's top-level rustdoc walk the reader through a `workflow! { name: "…", description: "…", tasks: [process_data] }` syntax. This is a hypothetical declarative macro; the actual API is `#[workflow(name = "…", …)] pub mod my_workflow { #[task(…)] async fn …() {…} }`. A user trying the README example gets a "macro `workflow` not found in this scope" error before doing anything else. The README also pins the version at `cloacina = "0.1.0"` while the workspace is at 0.5.1.

#### Evidence
- `README.md:33` — `cloacina = "0.1.0"` (workspace `Cargo.toml:7` is `version = "0.5.1"`).
- `README.md:72-77` — `let workflow = workflow! { name: "my_workflow", description: "…", tasks: [process_data] };`
- `crates/cloacina/src/lib.rs:75` — "Create workflows with the `workflow!` macro:" introducing 19 lines of example using the same fictional declarative syntax.
- `crates/cloacina/src/execution_planner/mod.rs:99-103` — same `workflow! { name: …, tasks: […] }` shape in the `TaskScheduler` module doc.
- `crates/cloacina/src/lib.rs:120` — example shows `executor.execute(...)` but the variable name three lines up is `runner` (`let runner = DefaultRunner::new(...)`). Cut-paste rot from a prior rename.

#### Suggested Resolution
Rewrite the quick-start to match the current `#[workflow(name = "…")] pub mod` shape. Bump the version pin in README. Audit `crates/cloacina/src/lib.rs` for `executor` / `runner` mismatches.

---

### LEG-05: `cloacina::package!()` doc says "Six methods" but the trait below it has nine

**Severity**: Major
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:78-108`; trait definition starts line 712
**Confidence**: High

#### Description
The macro doc (which is what users read when they invoke `cloacina::package!();` in their crate root) describes the emitted plugin as having "Six methods: get_task_metadata, execute_task, get_graph_metadata, execute_graph, get_reactor_metadata, get_trigger_metadata." The macro body actually emits nine — the additional `invoke_trigger_poll`, `get_triggerless_graph_metadata`, and `invoke_triggerless_graph` were added in a T-0553 follow-up. The same drift shows up two ways: the macro's stated contract is incomplete, and the trait doc on line 700-711 lists "## Methods" with bullets for only the first two.

#### Evidence
- `crates/cloacina-workflow-plugin/src/lib.rs:91-94` — "Six methods: get_task_metadata, execute_task, get_graph_metadata, execute_graph, get_reactor_metadata, get_trigger_metadata."
- `crates/cloacina-workflow-plugin/src/lib.rs:128-668` — the macro body emits all nine.
- `crates/cloacina-workflow-plugin/src/lib.rs:700-711` — `CloacinaPlugin` trait doc has a `## Methods` section that documents only `get_task_metadata` and `execute_task`.
- `crates/cloacina-workflow-plugin/src/lib.rs:683-698` — every `METHOD_*` constant past index 0 has the empty doc comment `/// See [`METHOD_GET_TASK_METADATA`].`. None of them say what the method actually does.

#### Suggested Resolution
Update the `package!()` macro doc to say nine methods and list them all; rewrite the `## Methods` section in the `CloacinaPlugin` trait doc to cover every method. The `METHOD_*` constants should each have a one-line doc that names the method, not redirect.

---

### LEG-06: `workflow_attr.rs` still calls `generate_packaged_registration` and discards the result with `let _ = …` — 130 lines of dead-code emission per `#[workflow]` invocation

**Severity**: Major
**Location**: `crates/cloacina-macros/src/workflow_attr.rs:277-293, 700-829`
**Confidence**: High

#### Description
The I-0102 / T-C path is supposed to strip per-macro `_ffi` plugin emission so that the unified `cloacina::package!()` shell at the crate root is the only producer of a fidius plugin. The macro's main function `generate_workflow_attr` does call `generate_packaged_registration` (line 277), then immediately drops the result (`let _ = packaged_registration;` at line 293), with a comment explaining why. The function `generate_packaged_registration` itself is 130 lines of token construction: a fully formed `_WorkflowPlugin` struct, a `#[plugin_impl]` block, six method impls, and a `fidius_plugin_registry!()` invocation. This is dead code that runs at compile time, allocates `proc_macro2::TokenStream` chunks, and gets thrown away. A maintainer reading the macro is led through a working FFI emission path and only at the very end discovers it's unreachable. The emission path was kept "in case T-A coexistence is needed" but the comment says T-C is now the only path.

#### Evidence
- `crates/cloacina-macros/src/workflow_attr.rs:277-287` — `let packaged_registration = generate_packaged_registration(…);` (8-arg call).
- `crates/cloacina-macros/src/workflow_attr.rs:289-293` — `// I-0102 / T-C: per-macro _ffi plugin emission stripped. … let _ = packaged_registration;`
- `crates/cloacina-macros/src/workflow_attr.rs:700-829` — full body of the unused function, including `#[plugin_impl(CloacinaPlugin, …)]` and `fidius_plugin_registry!()`.

#### Suggested Resolution
Delete `generate_packaged_registration`. Delete its callers, its arguments, its inputs, and the `let _ = …` line. The migration tracking comment in the file already says T-C is final.

---

### LEG-07: `inventory_entries.rs` says "Nothing in this file reads inventory yet. That wiring lands in T-0506" — `Runtime::seed_from_inventory` does in fact read inventory now

**Severity**: Minor
**Location**: `crates/cloacina/src/inventory_entries.rs:30-31`
**Confidence**: High

#### Description
The module-level doc on `crates/cloacina/src/inventory_entries.rs` ends with the sentence "Nothing in this file reads inventory yet. That wiring lands in T-0506 together with the removal of the global static registries." Both promised steps are done (T-0506 is closed and `Runtime::seed_from_inventory` walks every entry type defined here, see `runtime.rs:127-166`). The comment is the kind of "delete me when X lands" remark that survived its X.

#### Evidence
- `crates/cloacina/src/inventory_entries.rs:30-31` — quoted above.
- `crates/cloacina/src/runtime.rs:127-166` — `seed_from_inventory` does walk the entries.

#### Suggested Resolution
One-line patch: delete the stale paragraph.

---

### LEG-08: `Runtime` doc says "all five namespaces" but it has seven; the `Debug` impl reports five

**Severity**: Minor
**Location**: `crates/cloacina/src/runtime.rs:67-69, 435-450`
**Confidence**: High

#### Description
The struct doc on `pub struct Runtime` says "All five namespaces — tasks, workflows, triggers, computation graphs, and stream backends — are registered and unregistered through the same surface." `RuntimeInner` has seven fields: tasks, workflows, triggers, computation_graphs, triggerless_graphs, reactors, stream_backends. Two namespaces (triggerless_graphs and reactors) were added later. The `Debug` impl at line 442 also enumerates only five fields, missing `triggerless_graphs` and `reactors`. So a `dbg!()` of a runtime drops two of its registries — surprising for a debugging surface.

#### Evidence
- `crates/cloacina/src/runtime.rs:67-69` — `///   namespaces — tasks, workflows, triggers, computation graphs, and / ///   stream backends — are registered`.
- `crates/cloacina/src/runtime.rs:76-84` — seven RwLock fields.
- `crates/cloacina/src/runtime.rs:435-450` — Debug impl reports tasks, workflows, triggers, computation_graphs, stream_backends only.
- `crates/cloacina/src/runtime.rs:511-516` — the test `debug_format_reports_sizes` only checks the visible-five subset, so the omission isn't caught.

#### Suggested Resolution
Update the doc to say "seven namespaces" and list all of them. Extend the `Debug` impl to include `triggerless_graphs` and `reactors`. Extend the test to check all seven.

---

### LEG-09: Stale `with_global_workflows*` and "global registry" references in the public scheduler doc and the executor

**Severity**: Minor
**Location**: `crates/cloacina/src/execution_planner/mod.rs:178-183, 199, 569-571`; `crates/cloacina/src/executor/thread_task_executor.rs:783`; `crates/cloacina/src/registry/loader/task_registrar/mod.rs:128, 210, 229-233`; `crates/cloacina/src/python_runtime.rs:55`; `crates/cloacina/src/runner/default_runner/config.rs:585, 696-697`
**Confidence**: High

#### Description
T-0509 deleted process-global registries; doc comments and inline comments across the engine still reference them. The deletion was deliberate and prominent enough to merit its own task ID, but downstream comments weren't swept. The most visible victim is `TaskScheduler`: the rustdoc example it ships demonstrates `TaskScheduler::with_global_workflows_and_recovery(database)` (line 179) which doesn't exist (`grep`'d the impl block — closest match is `TaskScheduler::new` and `with_poll_interval`). Anyone copy-pasting the doc to bootstrap a scheduler hits an unknown-method error.

#### Evidence
- `crates/cloacina/src/execution_planner/mod.rs:178-183` — rustdoc example using `TaskScheduler::with_global_workflows_and_recovery(database).await?;`
- `crates/cloacina/src/execution_planner/mod.rs:199` — inline comment "Use all workflows registered in the global registry"
- `crates/cloacina/src/execution_planner/mod.rs:570` — second example using `TaskScheduler::with_global_workflows(database)`
- `crates/cloacina/src/executor/thread_task_executor.rs:783` — `// Resolve task from global registry` (the call below uses the scoped Runtime instead).
- `crates/cloacina/src/registry/loader/task_registrar/mod.rs:128, 211, 230-233` — three "global registry" references in a still-active code path.
- `crates/cloacina/src/runner/default_runner/config.rs:696-697` — `// Create executor with the scoped runtime — skip with_global_registry() since the runtime provides task lookups and the old TaskRegistry is unused.` followed by `Arc::new(crate::TaskRegistry::new())` being passed in anyway. The `TaskRegistry` instance is created empty and consumed by an API surface that the scoped Runtime supersedes — confusing residue.

#### Suggested Resolution
Sweep the codebase for "global registry" / "global_registry" / "with_global_workflows*" in doc comments, replace the example, and either drop the empty `TaskRegistry::new()` argument from the executor constructor or document what it actually does today.

---

### LEG-10: `reconciler/loading.rs` is 2,458 lines with `load_package` carrying all six load steps plus three language branches plus FFI plumbing in one function

**Severity**: Major
**Location**: `crates/cloacina/src/registry/reconciler/loading.rs:111-…` (specifically the language branch at lines 246-418)
**Confidence**: High

#### Description
The package loader is the system's most complex single function. It's reasonable for it to be substantial — six pipeline steps (cron, custom triggers, reactors, trigger-less CGs, reactor-bound CGs, workflows), three languages (Rust, Python workflow, Python CG), FFI plumbing — but the per-language driver is inlined in `load_package`. Lines 246-418 are one big `if cloacina_manifest.metadata.language == "rust" { … } else if … python … !has_computation_graph() { … } else if … python … has_computation_graph() { … } else { error }` chain, ~170 lines of conditional density, with `step_load_*` calls interleaved with view-building, snapshot capture, spawn_blocking dispatch, and result threading. The step helpers (`step_load_cron_triggers`, `step_load_custom_triggers`, `step_load_reactors`, `step_load_triggerless_cgs`, `step_load_reactor_bound_cgs`, `step_load_workflows` — lines 1407-1836) are the right abstraction; they just aren't given a dedicated per-language driver to call them from.

#### Evidence
- Total file length: 2,458 lines (`wc -l`).
- `load_package` body from line 111 carries the dispatch logic itself.
- `step_load_*` helpers are defined later in the same file (lines 1407, 1476, 1567, 1613, 1705, 1836) but their callers are all inside the giant `load_package` chain.
- Comment at line 251: "T-0554 / I-0102: Rust path now runs the precedence-ordered pipeline" — this acknowledges the structure should be a pipeline; it just hasn't been extracted into `load_rust_package(..)` / `load_python_workflow(..)` / `load_python_cg(..)`.

#### Suggested Resolution
Extract three private async methods on `RegistryReconciler`: `load_rust_package(metadata, manifest, work_dir, library_data) -> …`, `load_python_workflow_package(…)`, `load_python_cg_package(…)`. `load_package` becomes a 60-line dispatcher that handles archive write, unpack, manifest load, then dispatches by language.

**Cross-cutting note**: This is also a Performance/Evolvability hazard — a sub-step refactor today touches the giant function's local variable threads (`rust_reactor_names`, `triggerless_graph_names`, `cron_schedule_ids`, …) all of which live in the same scope.

---

### LEG-11: The codebase has three things called "scheduler", and one is named just `Scheduler` while living in `cron_trigger_scheduler.rs`

**Severity**: Minor
**Location**: `crates/cloacina/src/cron_trigger_scheduler.rs:115` (`pub struct Scheduler`); `crates/cloacina/src/execution_planner/mod.rs:184` (`pub struct TaskScheduler`); `crates/cloacina/src/computation_graph/scheduler.rs:267` (`pub struct ComputationGraphScheduler`); plus `SchedulerLoop` background task in `crates/cloacina/src/execution_planner/scheduler_loop.rs`
**Confidence**: High

#### Description
A reader who imports `cloacina::Scheduler` (the unified cron+trigger scheduler) sees a top-level identifier with the most generic possible name; they have to know that there are also `TaskScheduler` (the workflow-task planner) and `ComputationGraphScheduler` (the CG runtime supervisor). The file containing `Scheduler` is named `cron_trigger_scheduler.rs`, which doesn't match the public type name. The module-level doc on line 19 even says "single `Scheduler` that replaces the separate `CronScheduler` and `TriggerScheduler`" — the rename happened, the file/path didn't follow.

#### Evidence
- `crates/cloacina/src/cron_trigger_scheduler.rs:17-22` — module doc.
- `crates/cloacina/src/cron_trigger_scheduler.rs:115` — `pub struct Scheduler`.
- `crates/cloacina/src/lib.rs:545` (re-export) — `pub use cron_trigger_scheduler::Scheduler;` exposes it under the bare name at the crate root.
- `crates/cloacina/src/execution_planner/mod.rs:184` — `pub struct TaskScheduler`.
- `crates/cloacina/src/computation_graph/scheduler.rs:267` — `pub struct ComputationGraphScheduler`.

#### Suggested Resolution
Rename `Scheduler` → `UnifiedScheduler` (or `TriggerScheduler` since that's what the run loop ultimately is — a unified trigger driver); rename `cron_trigger_scheduler.rs` → `unified_scheduler.rs`. Update the prelude export.

---

### LEG-12: `ManualCommand` and `ReactorCommand` are two enums with overlapping variants; `ManualCommand` is a strict subset of `ReactorCommand`

**Severity**: Minor
**Location**: `crates/cloacina/src/computation_graph/reactor.rs:163-181`
**Confidence**: High

#### Description
The reactor accepts two command types. `ManualCommand` (used as the internal channel type, has `ForceFire | FireWith(InputCache)`) and `ReactorCommand` (used over the WebSocket wire, serde-tagged, has `ForceFire | FireWith { cache: HashMap<…> } | GetState | Pause | Resume`). The names don't tell a reader which is which; `ManualCommand` sounds like it's the human-operator interface, but the WS-tagged `ReactorCommand` is what an operator actually sends. The relationship — `ManualCommand` is the strict subset that the reactor's main loop directly executes, and `ReactorCommand::{GetState, Pause, Resume}` are handled by `ReactorHandle` directly without traversing the channel — is undocumented.

#### Evidence
- `crates/cloacina/src/computation_graph/reactor.rs:163-170` — `pub enum ManualCommand { ForceFire, FireWith(InputCache) }` with the doc "Manual commands accepted by the reactor."
- `crates/cloacina/src/computation_graph/reactor.rs:172-181` — `pub enum ReactorCommand { ForceFire, FireWith { cache }, GetState, Pause, Resume }` with the doc "Commands sent by WebSocket operators to a reactor."
- `crates/cloacina/src/computation_graph/registry.rs:200` — channel typed as `mpsc::Sender<ManualCommand>`, hand-converted from `ReactorCommand` at the WS boundary.

#### Suggested Resolution
Rename `ManualCommand` → `FireCommand` (it carries only fire-related variants) or `ReactorChannelCommand`. Add a one-paragraph doc on `ReactorCommand` explaining which variants traverse the channel and which are handled synchronously through `ReactorHandle`.

---

### LEG-13: Two duplicate `tracing::debug!` lines on consecutive statements in `ThreadTaskExecutor::build_task_context`

**Severity**: Minor
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:186-198`
**Confidence**: High

#### Description
A debug log fires once with the standard format string, then immediately again with the prefix `"DEBUG: "` and the same arguments. Either is a leftover from a previous debugging session that never got rebased away, or one of them is meant to be a `trace!`. Either way, every executor build_task_context call emits two near-identical log lines. Reading the function, this is a stumbling block — a reader naturally wonders whether the second log is intentional and what subtle thing it's logging differently.

#### Evidence
```rust
// crates/cloacina/src/executor/thread_task_executor.rs:186-198
tracing::debug!(
    "Building context for task '{}' with {} dependencies: {:?}",
    claimed_task.task_name,
    dependencies.len(),
    dependencies
);
tracing::debug!(
    "DEBUG: Building context for task '{}' with {} dependencies: {:?}",
    claimed_task.task_name,
    dependencies.len(),
    dependencies
);
```

#### Suggested Resolution
Delete the second log. If the intent was a different level, change it instead.

---

### LEG-14: `cloacina::package!()` macro inlines a four-line tokio-runtime construction four times — once per primitive shape

**Severity**: Minor
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:213-225, 331-341, 477-487, 574-584`
**Confidence**: High

#### Description
Every shape that runs an async function from inside the cdylib (tasks, computation graphs, triggers, trigger-less computation graphs) gets its own `static CDYLIB_*: OnceLock<tokio::runtime::Runtime>` block with a near-identical construction body — `Builder::new_multi_thread().enable_all().worker_threads(2).thread_name("…").build().expect("…")`. The only varying part is the thread name (`"package-shell-cdylib-worker"`, `"package-shell-cg-worker"`, `"package-shell-trigger-worker"`, `"package-shell-tlcg-worker"`). The macro body is the most-read FFI surface in the codebase; four copies of the same boilerplate dilute the parts that actually differ between shapes.

#### Evidence
- `crates/cloacina-workflow-plugin/src/lib.rs:213-225` — task variant.
- `crates/cloacina-workflow-plugin/src/lib.rs:331-341` — CG variant.
- `crates/cloacina-workflow-plugin/src/lib.rs:477-487` — trigger variant.
- `crates/cloacina-workflow-plugin/src/lib.rs:574-584` — trigger-less CG variant.

#### Suggested Resolution
Add a single `pub fn cdylib_runtime(thread_name: &'static str) -> &'static tokio::runtime::Runtime` helper in `cloacina_workflow_plugin` (or a `cdylib_runtime!()` declarative macro that takes the thread name) and call it from each method body. The four shapes' bodies become a single `cloacina_workflow_plugin::cdylib_runtime("package-shell-trigger-worker").block_on(…)` line. The macro shrinks from ~600 lines of method bodies to a much more readable surface.

---

### LEG-15: `tasks.rs` macro has two near-duplicate parsers — `parse_trigger_rules_expr` and `parse_trigger_condition_expr` — for the same condition vocabulary

**Severity**: Minor
**Location**: `crates/cloacina-macros/src/tasks.rs:384-471, 485-541`
**Confidence**: High

#### Description
Two functions parse the same `task_success(...)` / `task_failed(...)` / `task_skipped(...)` / `context_value(...)` vocabulary. The differences are essentially: `parse_trigger_rules_expr` wraps each result in `{"type": "All", "conditions": [..]}` for use as a top-level rule; `parse_trigger_condition_expr` returns the bare condition for use inside `all()`/`any()`/`none()`. The shared logic could trivially be a single `parse_trigger_condition_expr` plus a thin wrapper. Right now the maintenance hazard is that any future change to the condition grammar has to be made twice.

#### Evidence
- `crates/cloacina-macros/src/tasks.rs:384-471` (87 lines) — `parse_trigger_rules_expr`.
- `crates/cloacina-macros/src/tasks.rs:485-541` (56 lines) — `parse_trigger_condition_expr`. Cases match line-for-line.

#### Suggested Resolution
Have `parse_trigger_rules_expr` call `parse_trigger_condition_expr` and wrap. Or add a flag `wrap_in_all: bool`.

---

### LEG-16: `#[reactor]` macro doc says graphs bind "by type path" — they bind by name string, the type-path form was removed in I-0102

**Severity**: Minor
**Location**: `crates/cloacina-macros/src/lib.rs:144-155`
**Confidence**: High

#### Description
The `#[reactor]` proc-macro doc (which is what shows in rustdoc when a user hovers it) says "Graphs declared with `#[computation_graph(trigger = reactor(ReactorType), ...)]` bind to it by type path." But the `#[computation_graph]` macro's `trigger = reactor(...)` clause now takes a string literal, not a type path — confirmed in `crates/cloacina-macros/src/tasks.rs:165-172` where the same change for `invokes = computation_graph(...)` is documented as "The type-path form was removed in CLOACI-I-0102 — graphs are referenced by name with runtime contract validation." The reactor doc didn't get the rewrite.

#### Evidence
- `crates/cloacina-macros/src/lib.rs:144-155` — "bind to it by type path" example.
- `crates/cloacina-macros/src/tasks.rs:165-172` — explicit error message saying type-path form was removed.

#### Suggested Resolution
Rewrite the doc on `#[reactor]` to say "by name string" and update the example to use `trigger = reactor("risk_signals")`.

---

### LEG-17: Stranded comment in `dal/unified/mod.rs` — "Helper macro for dispatching operations based on backend type." precedes a struct with no associated macro

**Severity**: Minor
**Location**: `crates/cloacina/src/dal/unified/mod.rs:78-83`
**Confidence**: High

#### Description
A doc-comment block describes a "Helper macro for dispatching operations based on backend type. This macro simplifies writing code that needs to execute different implementations based on the database backend." The lines following it are the `pub struct DAL` definition — there is no macro. The comment was either orphaned by a refactor that deleted the macro or copy-pasted from elsewhere. A reader hits the comment, looks for a macro, finds a struct.

#### Evidence
```rust
// crates/cloacina/src/dal/unified/mod.rs:78-92
/// Helper macro for dispatching operations based on backend type.
///
/// This macro simplifies writing code that needs to execute different
/// implementations based on the database backend.
///
/// The unified Data Access Layer struct.
///
/// This struct provides access to all database operations through a single
/// interface that works with both PostgreSQL and SQLite backends.
```

#### Suggested Resolution
Delete the first paragraph (lines 78-82). Keep the actual `DAL` struct doc that follows.

---

### LEG-18: `WorkflowMetadata.schedules: Vec<String>` is documented as "List of schedule names defined in this package" but in the path traced by the system overview is unpopulated

**Severity**: Minor
**Location**: `crates/cloacina/src/registry/types.rs:81-82`
**Confidence**: Medium

#### Description
The `WorkflowMetadata` struct has a public `schedules: Vec<String>` field with a published rustdoc example showing `schedules: vec!["daily_analytics".to_string()]`. According to the system overview, the field is initialized to `Vec::new()` in the path that builds it and never populated — schedules are now stored in the `schedules` table directly. A user reading the type and the example assumes the field carries the list of cron names; the field is a vestige.

#### Evidence
- `crates/cloacina/src/registry/types.rs:81-82` — field with the doc.
- `crates/cloacina/src/registry/types.rs:53` — example shows it populated.
- System overview §10 Open Question 1 calls this out specifically.

#### Suggested Resolution
Either populate the field in `WorkflowRegistryImpl::register_workflow_package` (line ~378 per the overview) — pull the cron-shaped triggers from the FFI metadata and project them — or deprecate/remove the field with a `#[deprecated]` notice and migrate users to query the `schedules` table.

**Cross-cutting note**: Correctness should verify the field's actual semantics today.

---

### LEG-19: `reconciler/loading.rs:101` says "Load a package into the **global** registries" — the registries are scoped now

**Severity**: Minor
**Location**: `crates/cloacina/src/registry/reconciler/loading.rs:101-110`
**Confidence**: High

#### Description
The doc on `RegistryReconciler::load_package` opens with "Load a package into the global registries." The registries are no longer process-global; they're scoped to a `Runtime` instance owned by a `DefaultRunner`. The function body even uses `self.runtime` (line 238: `self.runtime.as_ref().map(|rt| rt.reactor_names()…)`). The doc just wasn't updated when scoping landed in T-0509.

#### Evidence
- `crates/cloacina/src/registry/reconciler/loading.rs:101-110` — doc opening.
- `crates/cloacina/src/registry/reconciler/loading.rs:237-241` — actual scoping via `self.runtime`.

#### Suggested Resolution
Replace "global registries" with "scoped runtime registries" in the doc.

---

### LEG-20: `cloacina::package!()` macro doc warns about a coexistence-conflict scenario that the spec says T-C resolves; current branch has T-A territory but the warning is unconditional

**Severity**: Observation
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:103-108`
**Confidence**: Medium

#### Description
The `package!()` macro doc warns: "**Coexistence:** in T-A the per-macro `_ffi` emission from `#[computation_graph]` and `#[workflow]` is unchanged. A crate that adds `cloacina::package!();` AND has `#[computation_graph]` / `#[workflow]` would emit two `fidius_plugin_registry!()` calls → linker conflict. T-C strips the per-macro emission so the shell becomes the only path."

But `workflow_attr.rs:289-293` shows T-C has been done for the workflow path (the per-macro `_ffi` is built and discarded with `let _ = packaged_registration;`). The warning in the package doc isn't reflective of current code; users reading it think they need to take care that they don't have both, when in practice `workflow_attr.rs` no longer emits `fidius_plugin_registry!()`. This is the inverse of LEG-06 — the doc warns of a problem that no longer exists, while LEG-06 is the dead code that backstopped the now-resolved problem.

#### Evidence
- `crates/cloacina-workflow-plugin/src/lib.rs:103-108` — coexistence warning.
- `crates/cloacina-macros/src/workflow_attr.rs:289-293` — T-C removal already in place (per the comment).
- The active branch is `i-0102-fidius-and-plugin-shell` per the system overview, but the spec marks T-C as in-flight.

#### Suggested Resolution
After resolving LEG-06 (delete the dead code), update the macro doc: replace the coexistence warning with a one-line note that `cloacina::package!()` is the sole producer of a fidius plugin and should appear exactly once at the crate root. If the I-0102 work is still mid-flight on this branch, keep the warning but pin it to the spec's actual migration status (e.g., reference the initiative directly so the warning evaporates when the initiative closes).

---

## Positive Patterns

- **The unified `DAL` accessor pattern**. `dal.context()`, `dal.task_execution()`, `dal.workflow_packages()`, etc. — every per-table DAL is reachable through a single `&DAL` and lifetime-bound to it. Public surface is symmetric and the doc on `crates/cloacina/src/dal/unified/mod.rs:96-228` (modulo LEG-17) is consistent. A new contributor finds where to add a query immediately.
- **The `cloacinactl` noun-verb file structure**. Every noun is a directory; every verb is a one-purpose file (`cloacinactl/src/nouns/server/start.rs`, `…/stop.rs`, `…/status.rs`, `…/health.rs`). The CLI's published shape (system overview §6) maps 1:1 to filesystem paths. Adding a new verb is "make a new file in the right directory".
- **CLOACI-S-0011 itself**. As a piece of design legibility, the spec is exemplary — it isolates five primitives, documents bans with rationale, gives a Rules table mapping pre-rename → post-rename for every public surface, and ships a changelog entry tying the I-0101 multi-graph refactor back to the noun definitions. Most legibility findings in this review are about *missing* the spec — but the spec itself is the right size, in the right place, and tells the right reader the right thing.
- **`dispatcher::traits.rs` trait docs**. `Dispatcher` and `TaskExecutor` traits both have full Implementation Requirements sections, working `# Example` blocks (with `rust,ignore` so they don't break test runs), per-method Arguments / Returns documentation, and the example at `traits.rs:115-134` even sketches a Kubernetes executor showing the abstraction is real. This is the model the rest of the trait surface should aim at.
- **Universal-types pattern for cross-backend portability**. `UniversalUuid`, `UniversalTimestamp`, `UniversalBool`, `UniversalBinary` (in `crates/cloacina/src/database/universal_types.rs` per the system overview) hide the Postgres/SQLite divergence behind one type per concept. The DAL can dispatch on `dal.backend()` without DAL methods being parametric on backend. A tidy abstraction whose name communicates exactly what it does.
