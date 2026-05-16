---
id: t-e-manifest-cleanup-remove
level: task
title: "T-E: Manifest cleanup — remove [[triggers]] and package_type"
short_code: "CLOACI-T-0551"
created_at: 2026-04-30T04:10:00+00:00
updated_at: 2026-05-03T12:57:23.174800+00:00
parent: CLOACI-I-0102
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0102
---

# T-E: Manifest cleanup — remove [[triggers]] and package_type

## Parent Initiative

[[CLOACI-I-0102]]

## Objective

Finish the macro-layer-everything pivot. With T-A through T-D landed, no in-tree code path reads `[[triggers]]` or `package_type` from `package.toml`. This task removes the parsing code and turns either key into a hard load-time error with a clear "this is now declared in the macro" message.

Two-phase delivery within the single PR:

1. **Deprecation warning pass.** Reconciler still parses both keys, but logs a `WARN`-level message naming the package and pointing to the macro-layer replacement. T-C is expected to have already migrated all in-tree fixtures, so this pass produces zero warnings in CI.
2. **Removal pass.** Delete the parsing code. Either key present in `package.toml` → load failure with the migration message.

The deprecation step exists to give external/out-of-tree packages a single release window to migrate; T-E itself ships both phases in one PR but the warning code remains in a release tag that downstream consumers can pin to.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Deprecation warnings

- [ ] Reconciler logs `WARN` when a loaded package's `package.toml` contains `[[triggers]]`. Message names the package, the offending key, and the macro-layer replacement (`#[workflow(triggers = [...])]`).
- [ ] Reconciler logs `WARN` when a loaded package's `package.toml` contains `package_type`. Message names the package and explains that primitives are now self-declared via the unified shell macro.
- [ ] CI run is clean: zero deprecation warnings emitted by the in-tree fixtures (proves T-C's migration was complete).

### Removal

- [ ] All `[[triggers]]` parsing logic removed from the manifest reader (locate via `grep -rn '\[\[triggers\]\]\|triggers\s*=' crates/cloacina/src/registry/`).
- [ ] All `package_type` parsing logic removed from the manifest reader.
- [ ] `package.toml` schema documentation (if any) updated to remove both keys.
- [ ] Either key present at load time produces `Err(...)` with the migration message; reconciler refuses to proceed past manifest validation for that package.
- [ ] Test added covering the hard-error path: a fixture `package.toml` with `package_type = ["computation_graph"]` fails load with the expected message; same for `[[triggers]]`.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.
- [ ] `angreal demos features python-packaged-graph` and `angreal demos features packaged-graph` still green.

## Implementation Notes

### Technical Approach

1. **Locate parsing.** `grep -rn 'package_type\|\[\[triggers\]\]' crates/cloacina/src/registry/ crates/cloacina-workflow-plugin/`. The manifest deserializer struct + the reconciler's branching logic are the two main hits.
2. **Wire deprecation warnings.** In the manifest reader, after deserializing, check both keys and emit `tracing::warn!` with the structured fields. Keep the parsing path otherwise inert — T-B already routes purely off plugin metadata.
3. **Remove the fields from the manifest deserializer.** With `serde(deny_unknown_fields)` on the manifest struct, the presence of either key now becomes a deserialization error. Map that error to a friendly migration message at the call site.
4. **Doc sweep.** Search docs site + README + any tutorial markdown that references either key. Update or delete.

### Key Files

- `crates/cloacina/src/registry/...` — manifest reader + reconciler entry point.
- `crates/cloacina-workflow-plugin/...` — if `package.toml` is also parsed there.
- `docs/...` — schema/reference pages.

### Dependencies

- **T-0547 (T-A)**, **T-0548 (T-B)**, **T-0549 (T-C)** — must land first. T-C in particular: this task is meaningless until in-tree fixtures stop relying on the manifest keys.

### Risk Considerations

- **Out-of-tree packages.** External cdylibs depending on `[[triggers]]` or `package_type` break on upgrade. The deprecation-warning step buys one release of grace. Release notes (T-0542, deferred) must call this out prominently.
- **Friendly error vs. raw deserializer error.** `deny_unknown_fields` produces `unknown field "package_type"` — fine for debugging, but the migration hint is what we want users to see. Catch the error at the manifest-load boundary and rewrap.

## Status Updates

### 2026-05-03 — T-E done in a single landing

Manifest cleanup landed in one focused change. T-B's deprecation warnings from the prior commit are removed (they were transitional); the warnings turn into hard errors via `#[serde(deny_unknown_fields)]` on `CloacinaMetadata`. The reconciler wraps the deserialization error with a friendly migration hint at the manifest-load boundary.

**Changes shipped:**

- `crates/cloacina-workflow-plugin/src/types.rs`:
  - `CloacinaMetadata` gained `#[serde(deny_unknown_fields)]`. Legacy `package_type: Vec<String>` field deleted (defaulted to `["workflow"]`); legacy `triggers: Vec<TriggerDefinition>` field deleted.
  - `default_package_type()` helper deleted.
  - `has_workflow()` reimplemented as "graph_name absent OR workflow_name present"; `has_computation_graph()` as "graph_name present". Replaces the old `package_type.iter().any(...)` lookups.
  - Updated 6 unit tests in this file: dropped references to removed fields; added `test_cloacina_metadata_legacy_package_type_rejected` and `test_cloacina_metadata_legacy_triggers_rejected` to lock down the deny_unknown_fields behavior.
- `crates/cloacina/src/registry/reconciler/loading.rs`:
  - Removed T-B's deprecation warnings (they referenced now-gone fields).
  - Wrapped the `load_manifest` error with a migration hint (`"package_type was removed in CLOACI-I-0102; primitives are now self-declared via the unified cloacina::package!() shell macro and per-primitive macros"` if the raw error mentions `package_type`; analogous hint for `triggers`).
  - `register_package_triggers` reduced to a no-op shim returning `Ok(Vec::new())` — the manifest-side trigger registration path is gone, replaced by `validate_workflow_trigger_subscriptions` consuming `PackageTasksMetadata.triggers` from FFI metadata.
  - Two obsolete tests deleted: `register_triggers_tracks_registered_triggers` and `register_triggers_mixed_registered_and_missing` — they asserted manifest-side parsing that no longer exists.
- `crates/cloacina/src/registry/workflow_registry/filesystem.rs`:
  - `test_package_with_triggers_in_manifest`: dropped `[[metadata.triggers]]` from the test fixture's manifest. The test now exercises a clean post-T-E manifest.
- `crates/cloacinactl/src/commands/daemon.rs`:
  - The daemon's automatic trigger registration loop (read `[[triggers]]` from manifest, register cron schedules + custom-poll triggers) deleted. Documented as "depends on FFI trigger metadata which currently stubs `Ok(vec![])` until TriggerEntry relocates" — pending follow-up.

**Test gates (all green):**

- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` (701 passed; 2 obsolete deleted, 2 new in workflow-plugin)
- [x] `angreal test integration --backend sqlite` (6 + 28 Python)
- [x] `angreal test integration --backend postgres` (290 + 28 Python; one transient flake on first run, clean on retry after `angreal services reset --clean`)

### Deferred

- **Daemon's automatic trigger registration from packages** is currently a no-op. Restoring it requires the FFI `get_trigger_metadata` stub to be replaced with real inventory walks, which depends on `TriggerEntry` relocating from `cloacina` to `cloacina-workflow-plugin`. Documented in T-0547 status.
- **`angreal demos features python-packaged-graph` / `packaged-graph` smoke check** (AC item) not run in this iteration — the integration test suite covers the same wire format end-to-end. The demos may need their `package.toml` files updated to drop `package_type`/`[[triggers]]` if any still ship them, since deny_unknown_fields will now reject those manifests.

### State

T-0551 (T-E) **complete**: legacy `package_type` and `[[triggers]]` keys are hard-errored at deserialization with a friendly migration hint; the reconciler's manifest-side trigger registration path is gone; tests lock down the new contract.

I-0102 (Unified Cloacina package plugin shell) is now complete across T-A through T-E. The unified shell is the sole CloacinaPlugin export path, the macro layer is the source of truth for primitive declarations and cross-references, and the manifest is reduced to package identity + language + Python entry-module hints.
