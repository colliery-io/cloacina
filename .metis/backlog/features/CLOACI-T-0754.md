---
id: pure-python-what-why-doc
level: task
title: "Pure-Python what/why doc persistence — carry parsed docs through the reconciler path"
short_code: "CLOACI-T-0754"
created_at: 2026-06-20T15:20:28.309039+00:00
updated_at: 2026-07-05T20:01:28.640977+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Pure-Python what/why doc persistence — carry parsed docs through the reconciler path

## Origin

Deferred follow-up from **CLOACI-T-0752** (opinionated what/why task docs, in
initiative CLOACI-I-0126). T-0752 shipped the doc model end-to-end for **Rust**
and the **Python parser exists and is tested**, but for **pure-Python packages
the parsed docs are not persisted**, so they never reach the API.

## Why it's not done yet (the architectural reason)

T-0752 carries docs at build success:
`build.rs` parses → `BuildOutcome::Success{task_docs}` →
`mark_build_success_with_docs` → `extract_and_merge_build_metadata` overlays docs
onto the per-task metadata. But a **pure-Python build produces an empty cdylib**,
so `extract_and_merge_build_metadata` early-returns (`if compiled.is_empty()`)
and never applies the overlay. Python per-task metadata is instead written
**later, by the reconciler** — `persist_task_graph_db`
(`crates/cloacina/src/registry/workflow_registry/database.rs`), invoked from
`crates/cloacina/src/registry/reconciler/loading.rs` — which currently sets
`doc_what: None` / `doc_why: None`.

So the parsed Python docs (available at build time) are dropped before the
reconciler writes the tasks.

## Objective

Persist the build-time-parsed Python `what`/`why` docs onto pure-Python tasks so
they surface on `WorkflowTaskNode` (same API field Rust already populates).

## Options to evaluate

1. **Persist docs at build time into a place the reconciler preserves**, then
   have `persist_task_graph_db` merge rather than overwrite the doc fields.
2. **Re-parse on the reconciler side**: the reconciler has access to the source;
   run the same `doc_parse` there when building the Python task graph.
3. **Thread `task_docs` from the build outcome into the reconciler load path**
   (more plumbing; the build and reconcile steps are decoupled in time).

Option (2) is likely cleanest (reuse `cloacina-compiler`'s `doc_parse` or lift it
into a shared crate the reconciler can call); confirm where the Python source is
available at reconcile time.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] A pure-Python package's `@task` docstrings (what:/why:) surface on
      `WorkflowTaskNode.doc_what` / `doc_why` via the API, matching Rust behavior.
- [ ] Undocumented Python tasks remain typed `None` (no regression).
- [ ] Verified via the angreal integration lane (Python scenarios).

## Related work
- **CLOACI-T-0752** — the Rust-complete doc feature this finishes for Python.
- **CLOACI-I-0126** — Workflow legibility (now completed; this is its one
  carried-over follow-up).
- **CLOACI-S-0013 / I-0128** — broader injectable-interface work also carries
  per-task/-surface metadata compiler-side; coordinate the Python carry approach.

## Status Updates

### 2026-07-05 — DONE via option (1), LIVE-VERIFIED (branch fix/t0754-py-task-docs, commit d032165a)
Option (1) won over the sketched option (2): no re-parse, no crate lift — the build already has the docs; they just needed a durable home. `PackageMetadata` gains a raw `task_docs: HashMap<String, TaskDocs>` (serde-default), persisted by `mark_build_success_with_docs` on BOTH merge branches (the Python `compiled.is_empty()` branch previously early-returned unless params/surfaces existed — the regression test caught that on its first run); `persist_task_graph_db` re-merges docs per task id when it writes the Python task list at load (previously hardcoded `doc_what/doc_why: None`). Rust behavior unchanged.

**Tests**: `test_python_task_docs_survive_load_time_task_rewrite` (sqlite in-memory, mirrors the real py flow: empty tasks at build → docs stored → load-time rewrite → docs surface per task; undocumented stays None). All crates + test tree compile clean.

**LIVE PROOF (demo stack)**: `demo-py-workflow` gained what:/why: docstrings on `prepare`/`transform`; uploaded as 0.1.1 → built → loaded → `GET /v1/tenants/public/workflows/demo-py-workflow` task_graph node: `"id":"prepare","doc_what":"Stage the demo batch — seed the context the downstream fan-out reads.","doc_why":"Every branch keys off the prepared flags; ..."` — Python parity with Rust achieved.

**Bonus find**: the live verification exposed [[CLOACI-T-0840]] (P1): in-place Python package version upgrades silently lose their tasks (module re-import is a no-op in the live interpreter — the trap the AGENT guards against but the server reconciler doesn't); worked around via server restart for the proof, filed with full evidence + fix directions. COMPLETE.
