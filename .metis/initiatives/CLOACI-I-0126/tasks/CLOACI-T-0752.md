---
id: opinionated-task-workflow
level: task
title: "Opinionated task/workflow documentation — capture the 'what & why' and expose it via API for UI consumption"
short_code: "CLOACI-T-0752"
created_at: 2026-06-20T02:34:05.632991+00:00
updated_at: 2026-06-20T13:23:40.813688+00:00
parent: CLOACI-I-0126
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0126
---

# Opinionated task/workflow documentation — capture the 'what & why' and expose it via API for UI consumption

## Origin

Surfaced during a live demo (2026-06-18/19). We want an opinionated system for
bringing in task and workflow documentation — a way to capture the *what and
why* of a task/workflow so it can be surfaced in the UI, so an operator
understands intent and rationale, not just the DAG and status.

## Scope — AUTHORING + DATA PRESENTATION ONLY (no frontend work)

This ticket covers the **authoring convention, carry-through, and API
exposure** — making the documentation available for UI consumption. The frontend
render is explicitly **out of scope**: a designer is reviewing the current UI and
we are not churning the frontend right now. A separate, later ticket renders it.

**Out of scope:** UI components, markdown rendering/sanitization in `ui/`, any
visual treatment.
**In scope:** the opinionated authoring schema, capturing it into the package
manifest at compile time, and exposing it via the API/SDK.

## Objective

Define an opinionated, low-ceremony way for authors to document the *what & why*
of workflows and their tasks alongside the code, carry that through
compile/packaging into the manifest, and expose it over the API so it is
available for later UI consumption.

## What exists today (and why a one-line description isn't enough)

The manifest already carries a single optional `description` per task and per
package — `crates/cloacina/src/packaging/manifest_schema.rs:110`. That's a
one-liner, not "what & why" documentation: no rationale, no longer-form prose, no
structure, and it's easy to leave empty. This ticket is about an intentional,
opinionated documentation surface, not just widening the existing string.

> **Opinionated = we pick the format and the prompts.** Rather than a freeform
> doc blob, the system should ask authors for specific things (what it does, why
> it exists, when it fires, gotchas) so docs are consistent and worth surfacing.

## Backlog Item Details

### Type
- [x] Feature — authoring + packaging + API (no frontend)

### Priority
- [x] P2 — Medium (transparency/operability; strong onboarding value)

### Business Justification
- **User Value**: Captures intent and rationale at the data layer so the UI can
  later show *why* a workflow exists, not just its shape.
- **Business Value**: Self-documenting workflows; better onboarding, handoffs,
  and incident response — ready for the UI to consume when frontend work resumes.
- **Effort Estimate**: M (authoring convention + manifest/packaging carry + API).

## Design questions to settle (this is the "opinionated" part)

1. **Authoring surface** — where does the author write it?
   - Rust: a doc attribute / docstring on the task/workflow, or a `## Why` block
     in `package.toml`?
   - Python: module/function docstrings (natural fit) with an opinionated
     section convention?
   - A sidecar `WORKFLOW.md` / per-task markdown in the package?
   Recommend leaning on the language-native docstring/doc-comment where possible
   so docs live next to code (ties into CLOACI-I-0125 "just types + functions").
2. **Schema** — what fields do we *insist* on? Candidate: `what` (summary),
   `why` (rationale), `when` (trigger/fire conditions), `caveats`. Structured,
   not freeform.
3. **Carry-through** — extend the manifest (`manifest_schema.rs`) to hold the
   structured doc, populated at compile time. Consider drift control (ties into
   CLOACI-T-0736 FFI-derive manifest-from-code) so docs are derived, not
   hand-maintained in two places.
4. **API shape** — how the structured docs are returned (per workflow and per
   task), kept in sync in `cloacina-api-types` / the SDK so the frontend can
   consume them later without drift.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] An opinionated, documented convention for authoring task/workflow what & why
      in both Rust and Python.
- [ ] Documentation is captured into the package manifest at compile/package time
      (no separate hand-maintained source of truth that can drift).
- [ ] An API field/endpoint exposes the structured docs per workflow/task, with
      API types/SDK updated.
- [ ] Workflows authored without docs are represented with a clear, typed
      "undocumented" state in the API response.
- [ ] No frontend changes in this ticket; the data is exposed so the UI can
      consume it in a later ticket.

## Related work

- **CLOACI-T-0750** — sibling: expose retained source via API. Docs + source
  together make workflows legible at the data layer; consider one API surface.
- **CLOACI-I-0125** — Authoring-surface cruft removal ("just types + functions").
  The authoring convention should fit that philosophy (docstrings/attrs, minimal
  ceremony).
- **CLOACI-T-0736** (blocked) — FFI-derive manifest metadata from code. Same
  carry-through machinery; deriving docs from code avoids a new drift class.
- **CLOACI-I-0117** — web UI (the eventual, separate consumer).

## Status Updates

### 2026-06-20 — Decision: language-native docstrings (authoring surface)

Authoring surface decided: **language-native docstrings / doc-comments** (Rust
doc-comments and Python docstrings on the task/workflow). Fits I-0125 "just types
+ functions"; docs live next to code. Schema: structured `what` / `why`
(+ optional `when` / `caveats`), with a plain-summary fallback when unstructured.

### 2026-06-20 — Correction: no live description path exists

Investigation correction: per-task `description` is NOT authored today — the Rust
macro hardcodes `format!("Task: {id}")`
(`cloacina-macros/src/workflow_attr.rs:781`) and never parses `///`; the Python
decorator ignores docstrings. So this builds a new path, it does not extend a
live one.

### 2026-06-20 — Carry-path decision: compiler-side source parse

Carry docs **compiler-side**, NOT through the FFI wire struct. The compiler
already unpacks the full source (same source T-0750 surfaces); it parses Rust
doc-comments / Python docstrings at build time and merges the doc fields into the
persisted per-task metadata, bypassing `TaskMetadataEntry` (the bincode FFI
struct). Rationale: a `TaskMetadataEntry` change is a wire-format change that
collides with the in-flux fidius authoring/packaging shift and the blocked
T-0736; the compiler-side parse avoids all of that.

### 2026-06-20 — Validated implementation plan (seam located, ready to build)

The per-task metadata the UI sees is NOT the manifest file — it's extracted from
the compiled cdylib at build success and merged into `workflow_packages.metadata`,
which is a **JSON column** → **no SQL migration needed**, just serde fields with
`#[serde(default)]` for back-compat.

Injection points:
- `crates/cloacina-compiler/src/build.rs` `run_build` — already unpacks the full
  source (`source_dir`) for every language. Add a doc-parse step producing
  `HashMap<task_id, TaskDocs{ what, why }>` and thread it out of
  `BuildOutcome::Success`. Rust: parse `.rs` with `syn`, find `fn` items carrying
  `#[task(...)]`, read the `id` arg (fallback fn name) + `#[doc]` comments.
  Python: heuristic scan of `@task`/`@cloaca.task`-decorated defs + their
  docstring (documented limitation; robust AST parse deferred). New dep: `syn`
  on `cloacina-compiler` (already in the workspace via macros).
- `crates/cloacina-compiler/src/loopp.rs:81` — pass parsed docs into
  `mark_build_success`.
- `crates/cloacina/src/registry/workflow_registry/database.rs:1199`
  `mark_build_success` (+ `extract_and_merge_build_metadata` ~:1333) — merge
  `what`/`why` into each per-task entry of `PackageMetadata`.
- `TaskMetadata` / `PackageMetadata` (registry/loader types) — add
  `doc_what`/`doc_why: Option<String>` with `#[serde(default)]`.
- `build_task_graph` (database.rs:38) + `WorkflowTaskNode` (registry/types.rs +
  api-types) — surface the doc fields per task; rides along the existing
  `get_workflow`/`WorkflowDetail` mapping (same exposure pattern as T-0750).
- Python pure-package path:
  `crates/cloacina/src/registry/reconciler/loading.rs:465`
  (`persist_task_graph`) also needs the docs threaded in.

Files: ~10 across cloacina-compiler / cloacina / cloacina-api-types /
cloacina-server + one new parse module + `syn` dep. No DB migration.

**Sequencing:** largest item in the tranche; taking it after the quick wins
(T-0749 pause, T-0747). Plan captured here so it survives compaction and can be
picked up directly from these anchors.

### 2026-06-20 — IMPLEMENTED + VERIFIED (Rust end-to-end; Python parser ready)

Built compiler-side as planned, on `feat/i0126-legibility`.

Authoring convention (opinionated, degrades gracefully): a `what:` line → summary,
a `why:` line → rationale, on the task's doc-comment (Rust `///`) / docstring
(Python). With no markers the whole doc becomes `what`. Undocumented tasks carry
typed `None`.

Implementation:
- **New parser** `crates/cloacina-compiler/src/doc_parse.rs` (deps: `syn` 2.0 +
  `proc-macro2`). `parse_task_docs(source_dir, language)` walks the unpacked
  source: Rust via `syn` (`#[task]` fns, `id`=… via token scan or fn-name
  fallback, `#[doc]` lines); Python via a best-effort line scanner
  (`@task`/`@cloaca.task`/`@cloacina.task` defs + triple-quoted docstring).
  Best-effort: a parse failure yields no docs, never fails the build.
- **Carry**: `build.rs` parses after unpack → `BuildOutcome::Success { artifact,
  task_docs }`; `loopp.rs` calls new `mark_build_success_with_docs`;
  `extract_and_merge_build_metadata` overlays `what`/`why` per task by local id
  into the `workflow_packages.metadata` JSON column — **no migration**.
- **Types**: `TaskDocs` + `doc_what`/`doc_why` on `TaskMetadata`
  (`#[serde(default)]`); `doc_what`/`doc_why` on `WorkflowTaskNode` (core +
  `cloacina-api-types`); `build_task_graph` propagates; surfaced on
  `WorkflowDetail` via `get_workflow`. OpenAPI regenerated; `spec-check` passes.

Verification: `cargo check` clean on cloacina / api-types / server / compiler.
`doc_parse` unit tests: **6 passed**. `cargo test -p cloacina --lib`: **709
passed**. (Server `lib tests::` need the postgres lane — unrelated env failures.)

Acceptance criteria: [x] opinionated convention (Rust+Python, tested); [x]
captured at compile time, no drift source; [x] API exposes per-task docs; [x]
undocumented = typed `None`; [x] no frontend changes.

**Known limitation (follow-up):** Rust is end-to-end. For **pure-Python**
packages the parsed docs are NOT yet persisted — a Python build has an empty
cdylib so `extract_and_merge_build_metadata` early-returns, and per-task metadata
is written later by the reconciler's `persist_task_graph_db`
(`reconciler/loading.rs`, currently `doc_what/doc_why: None`). Wiring Python docs
needs threading them through the reconciler path (or reconciler-side re-parse).
The Python parser is in place + tested; only the pure-Python persistence carry is
deferred.
