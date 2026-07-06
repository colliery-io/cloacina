---
id: ffi-derive-manifest-metadata
level: task
title: "FFI-derive manifest metadata (workflow_name/description/reaction_mode/input_strategy) from code — kill the T-0666 drift class"
short_code: "CLOACI-T-0736"
created_at: 2026-06-17T05:33:11.481213+00:00
updated_at: 2026-07-06T01:23:28.693212+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# FFI-derive manifest metadata (workflow_name/description/reaction_mode/input_strategy) from code — kill the T-0666 drift class

## Parent Initiative

[[CLOACI-I-0125]] — acts on the **T4 tail** of the [[CLOACI-T-0720]] sweep. Kills
the #1 drift-bug class (manifest disagreeing with code — exactly [[CLOACI-T-0666]]).

## Objective

Derive `workflow_name` / `description` / `reaction_mode` / `input_strategy` from
the macro attrs in compiled code (via the same FFI path that already derives the
task DAG), instead of hand-maintaining them in `package.toml`. Removes the
manifest-vs-code disagreement that caused T-0666.

## Type / Priority
- Tech Debt / reliability — removes a whole drift-bug class. P2 (M effort).

## Background (verified — T-0720)
- `workflow_name`/`description`/`reaction_mode`/`input_strategy` are **already in
  the macro attrs** and extractable via the same FFI path that derives the task
  DAG / dependencies / task list (`crates/cloacina-build` + `crates/cloacina/src/packaging`
  + `package!()`'s `get_task_metadata`, `crates/cloacina-workflow-plugin/src/lib.rs:128-159`).
- Today these are restated in the manifest, so the manifest can disagree with the
  code — the [[CLOACI-T-0666]] failure mode.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] These fields are derived from compiled code at pack time; manifest values
      become optional overrides (or are dropped).
- [ ] A package that omits them packs correctly with the code-derived values.
- [ ] If a manifest value disagrees with code, the behavior is defined
      (override-with-warning or hard error — pick and document).
- [ ] Regression: the T-0666 drift scenario can no longer silently mis-build.

## Implementation Notes
Extend the FFI metadata extraction (mirror how the task DAG is already pulled).
Sequence **after** [[CLOACI-T-0735]] (manifest-local defaults) so the manifest
minimization and the code-derivation land coherently. Larger than T-0735 —
touches the build/packaging FFI path.

## Status Updates

### 2026-07-05 — audit + residual shipped; CLOSING
Audit first (the I-0116 lesson): **most of this task's objective already existed.** The Rust build-success path has ALWAYS overwritten `workflow_name`/tasks/graph_data/triggers from the compiled cdylib's FFI metadata (`database.rs:1590` merge in `extract_and_merge_build_metadata`); CG `reaction_mode`/`input_strategy` flow from `get_graph_metadata`/`get_reactor_metadata` at load; Python has no code counterpart (the manifest IS the declaration — no drift class by construction); and T-0666's actual field (`language`) was killed by T-0735's inference (ambiguity errors, tested).
**Residual shipped**: the disagreement behavior is now DEFINED and loud — code wins, and a manifest `workflow_name` that disagrees with the compiled value logs a `tracing::warn!` naming both values and telling the author to drop the (now-optional) manifest line. `description` stays manifest-owned deliberately (package-level prose; code has no better source).
**AC disposition**: derive-from-code ✓ (pre-existing, verified); omitted-fields pack ✓ (T-0735 minimal manifests); disagreement behavior ✓ (code-wins + warn, documented in-code); T-0666 regression ✓ via T-0735's resolver tests (language) + the packaged-workflow e2e lane (FFI extraction needs a real cdylib — not unit-fixturable). COMPLETE.

- 2026-06-17: Filed from the T-0720 decomposition. Not started.- 2026-06-17: **BLOCKED — deferred pending fidius wasm traits.** fidius is
  introducing a wasm implementation of traits that may significantly reshape the
  authoring/packaging story (cdylib + FFI + build-shell model). Per the user,
  defer this cluster so we don't build something the wasm direction reworks.
  Unblock = fidius wasm-traits direction settles. See
  [[project_fidius_wasm_authoring_shift]].