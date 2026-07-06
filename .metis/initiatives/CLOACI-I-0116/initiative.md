---
id: parameterized-workflow-instances
level: initiative
title: "Parameterized workflow instances — declared params, partials, and configurable execute/schedule"
short_code: "CLOACI-I-0116"
created_at: 2026-06-01T14:48:51.749236+00:00
updated_at: 2026-07-06T00:45:06.158739+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: parameterized-workflow-instances
---

# Parameterized workflow instances — declared params, partials, and configurable execute/schedule Initiative

## Context **[REQUIRED]**

Today a Cloacina workflow is a **singleton defined entirely at compile time**. The `#[workflow]` macro is applied to a `pub mod` of `#[task]` async fns; it emits `inventory::submit!` entries that `Runtime::seed_from_inventory` walks at startup, registering the workflow **by name** (`crates/cloacina-macros/src/workflow_attr.rs:496-519`, `runtime.rs`). Tasks are **static async fns** — they take a `Context<serde_json::Value>` and nothing else. There is no way for a task to capture configuration, and there is no way to define a workflow once and stamp out several configured copies of it.

The only channel for input into a run is the runtime `Context`:

- **Execute**: `DefaultRunner::execute(workflow_name, context)` / `execute_async` (`crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:44-369`). The caller hand-builds a `Context<serde_json::Value>` and the runtime looks the workflow up by name and runs it.
- **Schedule (cron)**: `register_cron_workflow(workflow_name, cron_expression, timezone)` (`crates/cloacina/src/runner/default_runner/cron_api.rs:30`). The `schedules` table (`models/schedule.rs`, `dal/unified/schedule/`) has **no input/parameters column**; each cron fire synthesizes a fresh, minimal, time-only context (`scheduled_time`, `schedule_id`, `schedule_timezone`, `schedule_expression`) in `cron_trigger_scheduler.rs`. `task_configuration` is written but is **always `{}`** (`execution_planner/mod.rs:629`).
- **Packaged workflows**: the `packaged` codegen path exports an FFI plugin whose `execute_task` receives the per-task input as `context_json` across the boundary (`workflow_attr.rs:783-828`); `PackageTasksMetadata` already carries `workflow_name`, task list, `graph_data_json`, and `triggers` (`workflow_attr.rs:768-781`, loaded via `PackageLoader::extract_metadata`, `registry/loader/package_loader.rs`).
- **Triggers**: `FfiTriggerImpl::poll()` returns a `TriggerInvokeResult { fire, context_json }` (`registry/loader/ffi_trigger.rs:84-142`) — the trigger produces the firing context.
- **Python**: `cloacina-python` mirrors the runner surface (`crates/cloacina-python/src/bindings/runner.rs`) — `execute`, `register_cron_workflow`, trigger management — via pyo3, which already provides mature Rust↔Python type marshaling. Python is a **core capability**, not an afterthought ([[feedback_python_is_core]]).

**The user need.** A user wants to author a *templatized* workflow once and instantiate it many times with different configuration, then either run it now or hand it to the scheduler:

```python
# illustrative — the desired shape
inst = base_download_file(source="/path/src", dst="/path/dst", schedule="* * * * *")
inst.register()   # → into the scheduled runner
# -- or --
inst.execute()    # → run now
```

This must work for **both embedded and packaged** workflows. Today none of it exists: there is no parameter declaration, no per-instance binding, no per-schedule payload, and no human-facing notion of an "instance."

### Decisions already taken (this session)

Five directional forks were resolved with the maintainer up front and frame the whole design.

1. **A "partial" is a serializable bound-parameter value, NOT a captured closure.** The instinct is `functools.partial` — bind values into the task code. That cannot work here: the runtime does not run a closure the user holds; at execution time it resolves the workflow **by name** from the `inventory` registry and reconstructs the task `Arc`s from scratch, and the *same* workflow may run in three places — in-process (embedded), inside a `dlopen`'d `.so` (packaged, crossing FFI as `context_json`), or on a **remote fleet agent** on another host ([[CLOACI-I-0114]]). A live Rust closure crosses none of those boundaries. Therefore whatever an instance binds must be **serializable data that travels with the run** (via `Context`). The "partial" *is* the instance: a clonable, serializable `(workflow name + resolved params)` value finished by supplying a schedule and/or a runner.

2. **Params are pre-declared and typed; the type system is Rust (+ pyo3 for Python).** The workflow advertises its configurable surface; instantiation **overrides or inherits defaults**, and *required* params (no default) must be supplied or instantiation fails. There is **no new schema DSL**: the canonical declaration is Rust types, Python is covered by pyo3's existing marshaling, and the only place a serialized descriptor is needed is the **packaged manifest** (dynamic load has no compile-time types) — and that descriptor is the macro's *derived projection* of the Rust declaration, not an authoring format. One declaration, three validation modes (Rust compile-time, Python boundary, packaged load-time).

3. **Defaults snapshot at instantiation.** A registered instance stores its **fully-resolved** param set and is immutable; **re-registering** is the explicit, visible way to adopt a new default. Fire-time default resolution was rejected: an instance whose effective config silently shifts on redeploy ("it worked Tuesday") is a debugging nightmare.

4. **Instances have both a UUID and a human name.** System identity is the `schedules` row UUID (already exists); management identity is a human `instance_name` **unique per `(workflow, instance_name, tenant)`**. You disable `sync_prod` by name; the executor and history reference the UUID. This is required to operate many instances of one template in production.

5. **v1 surfaces: cron + trigger + immediate-execute. Reactors are out.** Reactors are not meant to be hyper-configurable yet ([[project_reactor_vs_computation_graph]]: reactor is a specialized-trigger noun, the CG is the execution quantum). Trigger-instances bind params that merge into the trigger-produced context; reactors stay as-is. The params-blob primitive generalizes to reactors later.

**Scope spans Rust core, Python (pyo3), and packaged `.so` workflows** — the original need explicitly requires embedded *and* packaged.

### Relationship to CLOACI-I-0128 (added 2026-06-20)

A later initiative, **[[CLOACI-I-0128]]** (Explicit injectable input interfaces, spec [[CLOACI-S-0013]]), generalizes the *input-interface declaration* across all injectable surfaces (workflows, accumulators, reactors): "this surface accepts these named, typed inputs at execute/inject time." I-0116 and I-0128 are **separate but adjacent** (maintainer call): I-0128 is the generic declared-input foundation; **I-0116 is the partial-config / constructor system built on top of it** — bind values to a workflow's I-0128-declared inputs to produce a unique named, registered, scheduled instance. Two consequences for this initiative when it's picked up:

- **Dependency**: I-0116 → I-0128 (the declared-input model + descriptor land in I-0128 first).
- **Descriptor revision**: the `ParamSpec` descriptor (decision #2 / REQ-002) should carry I-0128's **`schemars`-derived JSON Schema** rather than a bare `type_name` string. This *keeps* decision #2's principle ("Rust types are the source of truth; the descriptor is a derived projection") — JSON Schema is just a richer projection — and unifies the descriptor with the accumulator/reactor surfaces.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- A `#[workflow(params(...))]` declaration that lets a template advertise its configurable parameters with types and optional defaults, emitting (a) compile-time-checked Rust accessors and (b) a derived, serializable param **descriptor** for the packaged manifest.
- A **`WorkflowInstance` ("partial")**: a clonable, serializable value = `(workflow name + fully-resolved params)`, produced by a generic builder (`Workflow::instance(name).param(..).build()`) with required/optional/unknown-param validation and instantiation-time default snapshotting.
- `WorkflowInstance::execute(&runner)` — build a `Context` from the resolved params and run now, returning the existing `WorkflowExecutionResult`.
- `WorkflowInstance::register(&runner, schedule)` — persist the resolved params (new `schedules` column) under a human instance name, so each **cron or trigger** fire merges the bound params into the per-run context.
- A **named-instance lifecycle**: `register` / `list` / `get` / `update` / `disable` / `delete` keyed by instance name, unique per `(workflow, instance_name, tenant)`, with re-register as the path to adopt new defaults.
- **Packaged-workflow parity**: the param descriptor rides in `PackageTasksMetadata`; instantiating a dynamically-loaded package validates supplied params against the descriptor at load/instantiation time; resolved param values reach packaged tasks via the existing `context_json` FFI path.
- **Python parity (pyo3)**: the instance/partial API and the lifecycle management surface are exposed in `cloacina-python`, matching the illustrative flow above.
- Diataxis docs + `angreal`-driven integration and live-server contract coverage for embedded, packaged, and Python paths.

**Non-Goals:**
- **Reactor-driven instances.** Cron + trigger + execute only in v1; reactors deferred.
- **Closure-capturing partials.** Bound params must be `Serialize`; you can bind a path, a mode, an ID — not a live DB handle or open socket.
- **A new schema/IDL language.** Rust types are the source of truth; pyo3 covers Python; packaged descriptor is derived.
- **Mutable / fire-time-resolved defaults.** Registered instances are immutable snapshots; re-register to change them.
- **Changing the content-based workflow version semantics.** Params are *instance* data, not workflow topology — they do not alter the workflow's version hash (the instance separately *pins* the workflow version it was built against; see OQ-5).
- **A server/HTTP management surface or CLI verbs for instances.** This initiative delivers the library + Python API; exposing instance CRUD over `cloacina-server`/`cloacinactl` is a follow-on.
- **Backfill of `task_configuration`** beyond what instance params require.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### System Requirements

- REQ-001: `#[workflow(params(name: Type [= default], ...))]` parses and validates a parameter list, rejecting duplicate names and (in Rust) supplying compile-time-typed accessors. A workflow with no `params(...)` behaves exactly as today.
- REQ-002: The macro emits a serializable param **descriptor** (`[{ name, type, default, required }]`) into the embedded `WorkflowDescriptorEntry` and the packaged `PackageTasksMetadata`, so the host can enumerate a workflow's params without its Rust types.
- REQ-003: `Workflow::instance(name)` yields a builder; `.param(k, v)` binds a value; `.build()` validates against the declared descriptor — **unknown param → error**, **wrong type → error**, **required-and-omitted → error**, **omitted-with-default → default**. `build()` returns a `WorkflowInstance` holding the **fully-resolved** param set.
- REQ-004: `WorkflowInstance` is `Clone` and round-trips through serde (so it can be stored, shipped, and reconstructed).
- REQ-005: `WorkflowInstance::execute(&runner)` materializes a `Context` from the resolved params (under a defined key mapping, OQ-1) and calls the existing execute path; observable behavior equals hand-building that context today.
- REQ-006: The `schedules` table gains a nullable `params` (JSON) column and a nullable `instance_name` column via **additive migration** (ADD COLUMN + CREATE INDEX, never DROP+CREATE — [[feedback_sqlite_migration_recreate]]), on both sqlite and Postgres. `Schedule`/`NewSchedule` models and the schedule DAL (`dal/unified/schedule/`) carry them.
- REQ-007: `WorkflowInstance::register(&runner, schedule)` persists the resolved params + instance name on the schedule row. The cron fire path (`cron_trigger_scheduler.rs`) and trigger fire path merge the stored params into the synthesized/produced context before execution, with a defined precedence (OQ-3).
- REQ-008: A unique constraint enforces one `instance_name` per `(workflow_name, tenant)`; lifecycle ops (`list`/`get`/`update`/`disable`/`delete`) operate by instance name and resolve to the underlying schedule UUID.
- REQ-009: Instantiating a **dynamically-loaded packaged** workflow validates supplied params against the descriptor surfaced by `PackageLoader`; resolved values reach packaged tasks via `context_json` unchanged.
- REQ-010: `cloacina-python` exposes the builder/partial (`instance().param().build()`), `execute`, `register`, and the instance lifecycle, with pyo3 marshaling Python values to the declared param types and raising Python exceptions on validation failure.
- NFR-001: A workflow with no declared params and callers that never touch the instance API see **zero behavioral change** and no new required columns populated (full backward compatibility).
- NFR-002: Bound params are constrained to `serde_json`-serializable values; the API makes a non-serializable binding a compile-time (Rust) or boundary (Python) error, never a silent drop.
- NFR-003: The param descriptor is the single source of truth shared by embedded and packaged paths — embedded compile-time checks and packaged load-time checks validate against the *same* declaration, so they cannot drift.

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

### Use Case 1: Stamp out many configured copies of one template
- **Actor**: Workflow author / operator.
- **Scenario**: A `sync_file` workflow declares `params(source, dst, mode = Copy)`. The operator instantiates it three times — `sync_prod`, `sync_staging`, `sync_archive` — each with different `source`/`dst`, and registers each on its own cron schedule.
- **Expected Outcome**: Three independent named instances run on their own schedules, each firing with its own bound params; the author wrote the workflow once.

### Use Case 2: Run a configured instance immediately
- **Actor**: Engineer at a REPL / in application code.
- **Scenario**: They build `sync_file` with `source`/`dst` and call `.execute(&runner)` without registering anything.
- **Expected Outcome**: The workflow runs now with the bound params delivered via context; result returns as a normal `WorkflowExecutionResult`.

### Use Case 3: Same capability from Python
- **Actor**: Python user.
- **Scenario**: `inst = sync_file.instance(source="/a", dst="/b"); inst.register(runner, cron="0 * * * *")` (and the `base_download_file(...)`-style sugar where available).
- **Expected Outcome**: Identical semantics to Rust; validation errors surface as Python exceptions.

### Use Case 4: Configure a packaged workflow loaded at runtime
- **Actor**: Operator deploying a third-party `.so`.
- **Scenario**: The package declares params; the operator enumerates them from the loaded metadata, instantiates with values, and registers.
- **Expected Outcome**: Params validated against the package's descriptor at instantiation; values delivered to packaged tasks across FFI; scheduling works as for embedded.

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

### Overview

Four layers, none of which change the scheduler or the `Task` trait:

1. **Declaration (macro).** `#[workflow(params(...))]` extends `UnifiedWorkflowAttributes` (`workflow_attr.rs:50`) to parse a typed param list. It emits: typed Rust accessors/builder sugar (embedded), and a serializable param descriptor into both `WorkflowDescriptorEntry` (embedded inventory) and `PackageTasksMetadata` (packaged FFI).
2. **Instance / "partial" (core).** A new `WorkflowInstance` value type + a generic `WorkflowInstanceBuilder` (`Workflow::instance(name)`). Holds the workflow name, the resolved param map, and an optional pinned workflow version. `Clone + Serialize + Deserialize`. Two terminal methods: `execute(&runner)` and `register(&runner, schedule)`.
3. **Persistence (DAL + migration).** `schedules` gains `params` (JSON) and `instance_name` columns; uniqueness index on `(workflow_name, tenant, instance_name)`. New DAL methods for instance CRUD layered on the existing schedule DAL.
4. **Fire-time merge.** The cron and trigger execution paths merge a schedule's stored `params` into the context they already build, then proceed exactly as today.

### Sequence — register + cron fire

```
author:  Workflow::instance("sync_file").param("source","/a").param("dst","/b").build()?  → WorkflowInstance{resolved params}
caller:  instance.register(&runner, Schedule::cron("0 * * * *"), name="sync_prod")
           → DAL writes schedules row { workflow_name, cron, params:{source,dst,mode:Copy}, instance_name:"sync_prod" }
... time passes ...
cron tick → cron_trigger_scheduler builds time-context {scheduled_time,...}
           → MERGE stored params into context  (params under defined keys; precedence per OQ-3)
           → runner.execute("sync_file", merged_context)   [unchanged from here down]
```

### Sequence — execute now

```
instance.execute(&runner)
   → context = params_to_context(resolved_params)
   → runner.execute("sync_file", context)   [existing path]
```

### Reuse of existing seams

- `Context` is the sole delivery vehicle — already crosses FFI (`context_json`) and would cross to fleet agents. No new transport.
- `register_cron_workflow` / cron DAL / `Schedule` model — extended, not replaced.
- `PackageTasksMetadata` / `PackageLoader` — gains one field (descriptor), same load path.
- pyo3 runner bindings — extended with the instance surface.

## Detailed Design **[REQUIRED]**

### The param descriptor — one declaration, three validators

`params(source: String, dst: String, mode: SyncMode = SyncMode::Copy)` compiles to a `Vec<ParamSpec { name, type_name, default: Option<serde_json::Value>, required: bool }>`. This descriptor is embedded in the workflow's inventory entry (embedded) and `PackageTasksMetadata` (packaged). Validation:
- **Embedded Rust**: generated typed builder methods (`.source(impl Into<String>)`) give compile-time checking; `build()` still returns `Result` for the "required omitted" case (Rust has no keyword-arg enforcement at the type level without typestate — OQ-4).
- **Python**: pyo3 marshals supplied kwargs to the declared types; `build()` validates required/unknown and raises on failure.
- **Packaged (dynamic)**: no compile-time types available, so `build()` validates supplied JSON values against `ParamSpec` (presence, JSON-type coercion) at instantiation/load.

### `WorkflowInstance` — the partial

```rust
// sketch
pub struct WorkflowInstance {
    workflow: String,
    tenant: String,
    workflow_version: Option<String>,   // pinned at build for reproducibility (OQ-5)
    params: serde_json::Map<String, serde_json::Value>,  // FULLY RESOLVED (defaults snapshotted)
}
impl WorkflowInstance {
    pub async fn execute(&self, runner: &DefaultRunner) -> Result<WorkflowExecutionResult, _>;
    pub async fn register(&self, runner: &DefaultRunner, schedule: Schedule, name: &str) -> Result<UniversalUuid, _>;
}
```
`build()` resolves defaults **once, now**, so the value is self-describing and immutable. Cloning/serializing it is how the "partial" travels.

### Params → Context mapping (OQ-1)

Resolved params must land in the `Context` under keys tasks can read. Options: (a) flat top-level keys (collision risk with cron's reserved `scheduled_time`/`schedule_id`/…); (b) a reserved envelope key, e.g. `context["params"][name]`. Leaning (b) to avoid collisions and make "what was configured" inspectable; decide in discovery. Whatever is chosen is documented and consistent across execute and fire-time merge.

### Schedule changes (additive only)

`ALTER TABLE schedules ADD COLUMN params TEXT NULL; ADD COLUMN instance_name TEXT NULL;` + `CREATE UNIQUE INDEX ... (workflow_name, tenant, instance_name) WHERE instance_name IS NOT NULL;` — additive on both backends ([[feedback_sqlite_migration_recreate]]). Existing schedules (null params/name) behave exactly as today. The cron dedup index `(schedule_id, scheduled_time)` is unaffected — distinct instances already have distinct schedule UUIDs.

### Fire-time merge

In `cron_trigger_scheduler.rs`, after the time-context is built and before `schedule_workflow_execution`, merge the row's `params`. For triggers, the trigger produces its own `context_json`; merge bound params with a defined precedence (bound-params vs trigger-produced — OQ-3).

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

- **Unit (macro)**: `params(...)` parsing — types, defaults, duplicate/unknown rejection; descriptor codegen for embedded + packaged. Trybuild-style compile-fail cases for required-omitted where feasible.
- **Unit (instance)**: build/validate/default-snapshot; serde round-trip; params→context mapping.
- **Integration (`angreal`, fresh DBs — [[feedback_use_angreal_testing]], [[feedback_stale_db_testing]])**: register N named instances of one workflow on cron, assert each fires with its own params (sqlite + Postgres); immediate execute; update→re-register adopts new defaults; uniqueness violation rejected; disable/delete.
- **Packaged**: build a packaged workflow with declared params, load it, enumerate descriptor, instantiate + execute + schedule; verify values reach tasks across FFI.
- **Python / live-server**: pyo3 surface exercised against the real runtime, and any server-exposed path tested against a live server, not spec-vs-spec ([[feedback_sdk_live_server_drift]]).

## Alternatives Considered **[REQUIRED]**

- **Free-form context params (no declaration).** Rejected: no validation, no discoverability of a workflow's knobs, no good errors. The maintainer explicitly wants a pre-declared configurable surface.
- **Closure-capturing partial (`functools.partial` style).** Rejected: tasks are reconstructed by name from inventory and may run in a `dlopen`'d `.so` or on a remote fleet agent; a live closure crosses none of those boundaries. Params must be serializable data.
- **Fire-time default resolution.** Rejected: silent config drift on redeploy is undebuggable. Snapshot at instantiation; re-register to change.
- **Per-workflow generated factory *only* (no generic builder).** Rejected as the foundation: dynamically-loaded packaged workflows aren't known at compile time, so a generic `Workflow::instance(name)` is required. The generated `base_download_file(...)` factory is *sugar on top* of the builder, delivered last and only for compile-time-known workflows.
- **A language-neutral schema DSL as the authoring format.** Rejected: Rust types + pyo3 already cover authoring; the packaged descriptor is a derived projection, not a second place to write schemas.
- **A `params` column that stores only overrides (not resolved set).** Rejected for registered instances: it re-introduces fire-time default resolution. Store the resolved set.

## Open Questions (resolve in discovery; not blocking decomposition)

- **OQ-1 — params→context key mapping.** Flat top-level keys vs a reserved `params` envelope. Lean: envelope, to avoid collision with cron's reserved keys and keep config inspectable. Decide + document.
- **OQ-2 — typed builder ergonomics in Rust.** How far to push compile-time enforcement of "required supplied" (typestate builder vs `build() -> Result`). Lean: `build() -> Result` for v1 simplicity.
- **OQ-3 — trigger merge precedence.** When a trigger produces its own context *and* the instance has bound params, who wins on key conflict? Lean: bound params override, but confirm against trigger use cases.
- **OQ-4 — packaged param type validation.** Descriptor carries type *names*; validation at load is JSON-shape coercion. How strict (e.g. enums, nested types)? Define the supported param type set for v1 (likely scalars + strings + simple enums/maps).
- **OQ-5 — workflow version pinning.** Should a registered instance pin the workflow `version`/`fingerprint` it was built against, and what happens on cron fire if the deployed workflow's version has changed? Lean: store the fingerprint, warn/observe on drift; do not hard-fail in v1.
- **OQ-6 — `update` semantics.** Does updating an instance's params mutate the row in place or create a new version? Lean: in-place update of the resolved params + `updated_at`, treated as an explicit re-snapshot.
- **OQ-7 — instance lifecycle vs raw cron API.** How do the new named-instance ops coexist with the existing `delete_cron_schedule(uuid)` / `set_cron_schedule_enabled` — do those become the primitives the named ops delegate to? Lean: yes, name ops resolve to UUID and delegate.

## 2026-07-05 audit — what I-0128/I-0117 already delivered (maintainer-directed scope reduction)

Verified in code before resuming this initiative:
- **Phase 1 DONE** (via I-0128 T-0756): `workflow(params(...))` + schemars JSON-Schema `declared_params` in `PackageTasksMetadata` (`package_loader.rs:77`) — the exact descriptor revision anticipated above.
- **Phase 6 DONE** (I-0128): descriptor surfaces through load + API; validated at execute (`validate_declared_params`, executions.rs).
- **Execute-now DONE** (I-0128 T-0757 + I-0117 T-0747): validated execute-with-params + UI config form — Use Case 2 ships today.
- **Python declaration parity DONE** (T-0760).
- **OQ-1 resolved de facto**: the shipped execute path validates params as **flat top-level context keys** — fire-time merge must match (flat), with the cron/trigger **reserved keys stamped after params** so `scheduled_time`/`schedule_id` can't be spoofed by a binding.
- **OQ-3 resolved (maintainer lean confirmed)**: bound instance params override trigger-produced payload keys on conflict; reserved time keys win over both.

**Residual scope (the gap, M not L)**: schedules carry no configuration (no `params`/`instance_name` columns — a params-requiring workflow is effectively un-schedulable), no `WorkflowInstance` partial, no fire-time merge, no named lifecycle, no Python instance API. Phases 2–5 + 7 below; phases 1/6 dropped as done; phase 8 (sugar) deferred out of v1.

## Implementation Plan **[REQUIRED]**

Phased; each phase is a candidate task batch at decomposition. **Decomposed 2026-07-05 per maintainer "close the gaps"** into 4 tasks (spine order): [[CLOACI-T-0843]] schedule persistence, [[CLOACI-T-0844]] WorkflowInstance core, [[CLOACI-T-0845]] fire-time merge, [[CLOACI-T-0846]] named lifecycle + Python parity + docs/tests. One PR per this initiative ([[feedback_pr_is_initiative]]).

1. **Declared-params foundation (macro + descriptor).** Extend `#[workflow]` to parse `params(...)`; emit the `ParamSpec` descriptor into `WorkflowDescriptorEntry` and `PackageTasksMetadata`; generate typed Rust accessors. Exit: a workflow declares params; descriptor is enumerable in both embedded and packaged metadata; no-params workflows unchanged.
2. **`WorkflowInstance` + generic builder (core).** `Workflow::instance(name).param(..).build()` with validation + default snapshot; `Clone`/serde; `params_to_context` mapping (OQ-1); `execute(&runner)`. Exit: embedded Rust can build a partial and `.execute()` with params reaching tasks via context.
3. **Schedule persistence (migration + DAL).** Additive `params`/`instance_name` columns + uniqueness index (sqlite + Postgres); `Schedule`/`NewSchedule` + DAL CRUD. Exit: schedules persist a params blob + instance name; existing schedules unaffected.
4. **Register + fire-time merge.** `WorkflowInstance::register`; cron and trigger fire paths merge stored params into context (OQ-3). Exit: a registered cron/trigger instance fires with its bound params end-to-end.
5. **Named-instance lifecycle.** `list`/`get`/`update`/`disable`/`delete` by name, delegating to the cron schedule primitives (OQ-7); re-register adopts new defaults. Exit: full CRUD over named instances with uniqueness enforced.
6. **Packaged-workflow support.** Surface the descriptor through `PackageLoader`; validate supplied params at instantiation against it; confirm values reach packaged tasks via `context_json`. Exit: a dynamically-loaded package is instantiated, executed, and scheduled with bound params.
7. **Python bindings (pyo3).** Mirror builder/partial, `execute`, `register`, lifecycle; marshal kwargs to declared types; validation→exceptions. Exit: the illustrative Python flow works; parity with Rust.
8. **Factory sugar (optional, last).** Per-workflow generated `workflow::partial()` / `base_download_file(...)`-style constructor on top of the builder, for compile-time-known workflows (Rust + Python). Exit: ergonomic sugar; builder remains the foundation.
9. **Hardening: docs + tests.** Diataxis docs (declare params, instantiate, schedule instances, manage them); `angreal` integration (sqlite + Postgres), packaged, and live-server/Python coverage. Exit: docs published; suites green.

Sequencing: 1→2→3→4 is the spine. 5 follows 4. 6 and 7 can proceed in parallel once 4 lands. 8 needs 1+2 (+6 for packaged sugar). 9 trails the surfaces it documents.

### 2026-07-05 — spine SHIPPED (T-0843/0844/0845 completed; commit 065756d2, branch feat/i0116-workflow-instances)
- **T-0843**: additive migration 040 (both backends): `schedules.params` (JSON, fully-resolved) + `schedules.instance_name` + partial unique index `(workflow_name, instance_name)`; Schedule/NewSchedule/UnifiedSchedule + all literal sites; DAL `find_by_instance_name` (dispatch_backend). Anonymous schedules unchanged (NULLs).
- **T-0844**: `cloacina::workflow_instance` — `WorkflowInstance` (Clone + serde, REQ-004) + builder validating against `Vec<InputSlot>` (unknown/required/reserved-name errors; defaults snapshotted at build, decision #3); `params_json()`; runner `register_cron_workflow_instance(instance, name, cron, tz)` persists the resolved set (REQ-006/007). Reserved-key guard both at build AND at merge.
- **T-0845**: `merge_instance_params` shared by BOTH fire paths — cron (params merged before the reserved stamps) and trigger (bound params override trigger payload via update-or-insert per OQ-3; reserved stamped after). Flat top-level keys matching the validated execute path (OQ-1 as audited).
- Tests: builder validation/default-snapshot/serde + merge precedence green; schedule suite 72/72 (no regression).
- **Remaining**: [[CLOACI-T-0846]] — named CRUD by name over the DAL finder, pyo3 instance API, Diataxis docs, angreal integration coverage incl. a live register→fire proof. PR opens when 0846 lands (one PR per initiative).