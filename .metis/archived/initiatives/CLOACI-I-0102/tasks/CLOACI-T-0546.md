---
id: t-00-upgrade-fidius-0-0-5-0-2-0
level: task
title: "T-00: Upgrade fidius 0.0.5 → 0.2.0 across the workspace"
short_code: "CLOACI-T-0546"
created_at: 2026-04-28T22:26:21.924411+00:00
updated_at: 2026-04-30T03:31:57.819676+00:00
parent: CLOACI-I-0102
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0102
---

# T-00: Upgrade fidius 0.0.5 → 0.2.0 across the workspace

## Parent Initiative

[[CLOACI-I-0102]]

## Objective

Bump the workspace from `fidius 0.0.5` to `0.2.0` (the latest published versions of `fidius`, `fidius-core`, `fidius-host`) and apply whatever API/macro/host-side migration the version jump requires. Get all test suites green on the new version before any I-0102 task touches the plugin shell.

This unblocks I-0102's wire-format decision: the spike of "what does fidius do when the host calls a method index that doesn't exist on the plugin" is materially easier and more reliable on 0.2 (which has a `PluginRegistry` + `PluginDescriptor` model that may already report missing methods cleanly).

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All `Cargo.toml`s bumped from `fidius* = "0.0.5"` to `"0.2.0"`. Five crates: `cloacina`, `cloacina-compiler`, `cloacinactl`, `cloacina-python`, `cloacina-workflow-plugin`.
- [ ] `Cargo.lock` regenerated.
- [ ] All call-site changes for breaking-API differences applied. Likely surfaces:
  - `#[plugin_interface]` parameter shape (buffer strategy may have moved to `BufferStrategyKind`).
  - `#[plugin_impl]` invocation form.
  - Plugin export from cdylib — 0.2 introduces `fidius_plugin_registry!()` macro and a `PluginRegistry` top-level export.
  - Host-side `PluginHandle` / `call_method` API.
- [ ] Macro codegen updated (`cloacina-macros/src/computation_graph/codegen.rs` and any workflow-side equivalent) so emitted plugin modules compile against 0.2.
- [ ] Capture in this task's status updates: what 0.2 does when the host calls a method index the plugin doesn't implement. (Empirically — write a small test or read the source.) This locks I-0102's wire-format choice (option (a) new trait method vs. option (b) extended `GraphPackageMetadata`).
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` green.
- [ ] `angreal test integration --backend postgres` green.

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes

### Technical Approach

1. **Bump all `Cargo.toml`s** in one pass:
   - `crates/cloacina-workflow-plugin/Cargo.toml` — `fidius`, `fidius-core`
   - `crates/cloacina/Cargo.toml` — `fidius-host`, `fidius-core`
   - `crates/cloacina-python/Cargo.toml` — two `fidius-core` entries
   - `crates/cloacinactl/Cargo.toml` — `fidius-core`
   - `crates/cloacina-compiler/Cargo.toml` — `fidius-core`

2. **Regenerate `Cargo.lock`** via `cargo update -p fidius -p fidius-core -p fidius-host`.

3. **Run `cargo check --workspace --all-features`** to surface compile errors. Triage and fix.

4. **Update macro codegen** in `cloacina-macros/src/computation_graph/codegen.rs` (and workflow macro if affected) to emit whatever 0.2 expects.

5. **Probe missing-method behavior.** Write a small test where the host calls a method that the plugin doesn't implement and observe the error shape. Capture in status updates so I-0102 can lock its wire-format choice.

### Key Files

- All 5 `Cargo.toml` files listed above.
- `crates/cloacina-macros/src/computation_graph/codegen.rs` — emits the `_ffi` plugin module today.
- `crates/cloacina-macros/src/workflow_attr.rs` — workflow-side plugin emission.
- `crates/cloacina/src/computation_graph/packaging_bridge.rs` — host-side `LoadedGraphPlugin` + `call_method(3, ...)`.
- `crates/cloacina/src/registry/reconciler/loading.rs` — workflow-side plugin invocation.

### Dependencies

None. This is a workspace-wide library upgrade; no other tasks depend on it being incomplete.

### Risk Considerations

- 0.0.x → 0.2.x is a major jump with no published changelog. Be prepared for surprises in `#[plugin_impl]` ergonomics, the cdylib export mechanism, and host-side calling conventions.
- All in-tree packaged Rust crates rebuild against the new ABI. No staged migration — once we bump fidius, every cdylib compiles against 0.2 or fails. That's fine for a workspace-internal change with no external plugin authors yet.
- Cargo.lock churn — transitive dependencies likely shift.

## Status Updates

### 2026-04-29 — fidius 0.2.0 lands cleanly with one workaround

Bumped all 8 fidius dependency entries (5 Cargo.toml files) from `0.0.5` to `0.2.0`. Surprisingly small migration — fidius 0.2.0 keeps `#[plugin_interface]`, `#[plugin_impl]`, and `PluginHandle::call_method` source-level compatible for our usage; no codegen or call-site changes needed.

**One workaround required.** fidius-host 0.2.0's `build.rs` calls `pyo3_build_config::use_pyo3_cfgs()` and `pyo3_build_config::get()`, both gated behind `pyo3-build-config`'s `resolve-config` feature. fidius-host's `Cargo.toml` declares `pyo3-build-config = "0.25"` *without* enabling that feature — so the build script source fails to compile any time `resolve-config` isn't unified into the workspace's build-dep graph from elsewhere.

This manifested when running `cargo test -p cloacina --lib --features postgres,sqlite,macros` (cloacina-python excluded → no other crate enables `resolve-config` → fidius-host's build.rs won't compile). `cargo check --workspace --all-features` was fine because cloacina-python's pyo3 deps already enable the feature.

Workaround: added a no-op `crates/cloacina/build.rs` and `[build-dependencies] pyo3-build-config = { version = "0.25", features = ["resolve-config"] }` in `crates/cloacina/Cargo.toml`. Forces feature unification across the build-deps graph; fidius-host's build.rs compiles cleanly.

**Upstream fix needed.** fidius-host 0.2.0 should declare `pyo3-build-config = { version = "0.25", features = ["resolve-config"] }` (or only when its `python` feature is enabled). Once fixed, our `build.rs` + build-dep can be removed. File an issue against `colliery-io/fidius` referencing this.

**Verification:**

- `cargo check --workspace --all-features` — green.
- `cargo test -p cloacina --no-default-features --features postgres,sqlite,macros --lib` — 701 passed, 1 ignored.
- `angreal test unit` — green.
- `angreal test integration --backend sqlite` — green (28 Python scenarios + Rust integration suite).
- `angreal test integration --backend postgres` — green (28 Python scenarios + Rust integration suite).

**Missing-method behavior probe (next).** Still need to confirm what fidius 0.2 returns when the host calls a method index the plugin doesn't implement. That answer locks I-0102's wire-format choice (option (a) new trait method vs. option (b) extended `GraphPackageMetadata`). Probe pending — write a small test that calls `handle.call_method(99, ...)` against an existing in-tree plugin and observe the error variant.

### 2026-04-29 — bumped to 0.2.1, workaround removed

fidius 0.2.1 was published with a fix for the build.rs `resolve-config` feature gap. Bumped all 8 entries from 0.2.0 → 0.2.1 and dropped both the workaround `crates/cloacina/build.rs` and the `[build-dependencies] pyo3-build-config` block from `crates/cloacina/Cargo.toml`.

**Verification on 0.2.1 (no workaround):**

- `angreal test unit` — green.
- `angreal test integration --backend sqlite` — green (28 Python scenarios + Rust integration suite).
- `angreal test integration --backend postgres` — green (28 Python scenarios + Rust integration suite).

The fidius upgrade portion of T-0546 is complete. Missing-method behavior probe still pending — that work is the one remaining AC item before transitioning to completed.

### 2026-04-29 — Probe done; option (a) locked for I-0102

Read fidius 0.2.1 source directly — the answer is in the host crate's API, no test needed.

**fidius 0.2 has first-class support for optional plugin methods.** From `fidius-macro/src/lib.rs`:

```rust
#[plugin_interface(version = 1, buffer = PluginAllocated)]
pub trait Greeter: Send + Sync {
    fn greet(&self, name: String) -> String;

    #[optional(since = 2)]
    fn greet_fancy(&self, name: String) -> String;
}
```

And the host returns clean typed errors (`fidius-host/src/error.rs`):

```rust
/// Optional method is not implemented by this plugin — its capability bit is unset.
#[error("method not implemented (capability bit {bit} not set)")]
NotImplemented { bit: u32 },

#[error("invalid method index {index} (plugin has {count} method(s))")]
InvalidMethodIndex { index: usize, count: u32 },
```

Two distinct mechanisms:
- `InvalidMethodIndex` — bounds-checked at the host before dispatch. Out-of-range index never even reaches the plugin.
- `NotImplemented { bit }` — for `#[optional]` methods, plugins that didn't implement get this clean error with the capability bit. New plugins set the bit and implement; old plugins leave it unset.

**I-0102 wire-format decision: option (a) — add a fifth trait method.** The trait gains:

```rust
#[plugin_interface(version = 2, buffer = PluginAllocated)]
pub trait CloacinaPlugin: Send + Sync {
    fn get_task_metadata(&self) -> ...;
    fn execute_task(&self, ...) -> ...;
    fn get_graph_metadata(&self) -> ...;
    fn execute_graph(&self, ...) -> ...;

    #[optional(since = 2)]
    fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError>;
}
```

Old plugins → `CallError::NotImplemented` → reconciler treats as "no reactors." New plugins implement via the unified `cloacina::package!()` shell walking `inventory::iter::<ReactorEntry>`. Fully backward-compatible without the synthetic-empty-graph awkwardness of option (b).

**T-0546 complete.** All ACs satisfied:
- Workspace bumped 0.0.5 → 0.2.1.
- `Cargo.lock` regenerated.
- No call-site changes (0.2.x is source-compatible for our usage).
- Macro codegen unchanged for now — will be touched in I-0102 T-A when the unified shell macro lands.
- Missing-method behavior captured: `CallError::NotImplemented` / `CallError::InvalidMethodIndex`. Option (a) locked.
- All four test sweeps green (`cargo check --all-features`, `angreal test unit`, integration sqlite, integration postgres).
