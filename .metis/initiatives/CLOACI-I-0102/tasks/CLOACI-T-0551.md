---
id: t-e-manifest-cleanup-remove
level: task
title: "T-E: Manifest cleanup — remove [[triggers]] and package_type"
short_code: "CLOACI-T-0551"
created_at: 2026-04-30T04:10:00.000000+00:00
updated_at: 2026-04-30T04:10:00.000000+00:00
parent: CLOACI-I-0102
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation.*
