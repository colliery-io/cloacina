---
id: workflow-source-code-expose-the
level: task
title: "Workflow source code — expose the retained .cloacina source via API for UI consumption"
short_code: "CLOACI-T-0750"
created_at: 2026-06-20T02:33:00.497712+00:00
updated_at: 2026-06-20T02:49:59.136100+00:00
parent: CLOACI-I-0126
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0126
---

# Workflow source code — expose the retained .cloacina source via API for UI consumption

## Origin

Surfaced during a live demo (2026-06-18/19). The UI does not make the workflow
code available anywhere. Since we compile the code ourselves, we should be able
to show the main code for any given task. A codebase check confirms the source
**is already retained** — it's just never exposed past the database.

## Scope — DATA PRESENTATION ONLY (no frontend work)

This ticket is **server/API only**: make the source code available for UI
consumption. The frontend is explicitly **out of scope** — a designer is
reviewing the current UI and we are not churning the frontend right now. When the
data is exposed and stable, a separate, later ticket can do the UI rendering.

**Out of scope:** React components, viewers, syntax highlighting, any `ui/`
changes.
**In scope:** registry accessor + API endpoint + API types that surface the
already-stored source.

## Key finding — the source is already stored, only the surfacing is missing

The compiler unpacks the uploaded `.cloacina` source package and the original
source tree is preserved in the registry; **no compiler or package-format change
is needed.** This is purely an API surfacing task.

- Source is stored as the `.cloacina` tar.bz2 archive in
  `workflow_registry.data` (BYTEA) —
  `crates/cloacina/src/database/migrations/postgres/006_create_workflow_registry_tables/up.sql:10`.
- The compiler retrieves and unpacks it for builds via
  `registry.get_source_for_build()` /
  `fidius_core::package::unpack_package()` —
  `crates/cloacina-compiler/src/build.rs:180`.
- The manifest carries only metadata (task id, function path, dependencies,
  description), **not** source —
  `crates/cloacina/src/packaging/manifest_schema.rs:110`. The source lives in the
  archive, keyed per task by the manifest's `function` path / source layout.
- No API endpoint exposes source today; workflow detail returns metadata + task
  graph only — `crates/cloacina-server/src/routes/workflows.rs`.

## Objective

Expose the stored source for a given workflow/task over the server API, scoped
and authorized like other workflow reads, so it is available for later UI
consumption.

## Backlog Item Details

### Type
- [x] Feature — server/API (surface existing data)

### Priority
- [x] P2 — Medium (high transparency value, low risk; data already exists)

### Business Justification
- **User Value**: Makes the source available so the platform can later show what
  a task actually does, without hunting for the original repo.
- **Business Value**: Closes the "black box" gap for compiled workflows at the
  data layer, ready for the UI to consume when frontend work resumes.
- **Effort Estimate**: S–M (registry accessor + one endpoint + API types).

## Technical Approach (from codebase investigation)

1. **Registry accessor** — add a method to read + unpack source from the stored
   archive, e.g. `get_source_code(package_id, file_path) -> Option<String>`, in
   `crates/cloacina/src/registry/workflow_registry/mod.rs`. Unpack the tar.bz2
   and pull the file(s) for the requested task.
2. **API endpoint** — e.g. `GET /v1/tenants/{tenant_id}/workflows/{name}/source`
   (optionally `?task={id}` or `?path=`) in
   `crates/cloacina-server/src/routes/workflows.rs`, returning the source text
   plus a manifest of available files. Auth scoping identical to workflow read.
3. **API types** — add a source response type in
   `crates/cloacina-api-types/src/workflows.rs` (keep separate from metadata
   responses; source can be large). Keep the SDK/type surface in sync so the
   frontend can consume it later without drift.

## Acceptance Criteria

## Acceptance Criteria

- [ ] An API endpoint returns the stored source for a workflow/task, scoped to
      the tenant and authorized like other workflow reads.
- [ ] Response includes a listing of available source files plus the requested
      file's contents.
- [ ] Works for both Rust and Python packages (or v1 scopes to one and says so).
- [ ] Large-source and "source not available" cases return clean, typed responses.
- [ ] No change to compiler or package format — source is read from the existing
      `workflow_registry` archive.
- [ ] No frontend changes in this ticket; API types/SDK are updated so the UI can
      consume the data in a later ticket.

## Open Questions

- Per-task file mapping: the manifest gives a `function` path per task — confirm
  it reliably resolves to a source file in the archive for both languages, or
  whether v1 returns the whole entry module rather than a per-task slice.
- Size/perf: stream or cache unpacked source rather than unpacking the whole
  archive on every request?

## Related work

- **CLOACI-T-0752** — sibling: expose task/workflow what & why docs via API. Same
  "make workflows legible (at the data layer)" theme; consider one API surface.
- **CLOACI-I-0117** — web UI initiative (the eventual, separate consumer).
- **CLOACI-T-0652** (completed) — UI workflows list/detail (future consumer).

## Status Updates

### 2026-06-20 — Implemented (data layer), pending build verification

Implemented on branch `feat/i0126-legibility`. Decision on the open questions:
v1 returns **all source files in the package** (whole archive, sorted by path),
not a per-task slice — the per-task `function`→file mapping is unreliable across
Rust/Python and a full read is cheap for these small archives. Unpack happens per
request in `spawn_blocking` with a 1 MiB/file cap; caching deferred (note left
below) since archives are small and this is read-rarely.

Source is read independent of build status (works for building/failed rows too),
since the `.cloacina` archive is always retained.

Files changed:
- `crates/cloacina-api-types/src/workflows.rs` — `WorkflowSourceFile`,
  `WorkflowSourceResponse` (+ re-export in `lib.rs`).
- `crates/cloacina/src/registry/types.rs` — core `WorkflowSourceFile`
  (+ re-export in `registry/mod.rs`).
- `crates/cloacina/src/registry/workflow_registry/mod.rs` —
  `get_workflow_source(package_id)` + `extract_source_files` /
  `collect_source_files` helpers (unpack via `fidius_core::package::unpack_package`,
  UTF-8 text only, binary/oversized skipped).
- `crates/cloacina-server/src/routes/workflows.rs` — `get_workflow_source`
  handler (name-or-UUID resolution mirroring `get_workflow`) +
  `detect_source_language`.
- `crates/cloacina-server/src/lib.rs` — route
  `GET /v1/tenants/{tenant_id}/workflows/{name}/source`.
- `crates/cloacina-server/src/openapi.rs` — path + schema registration.

No frontend changes (per initiative scope).

**Verification (maintainer runs — not run in-tool):**
- `angreal check crate crates/cloacina-api-types`
- `angreal check crate crates/cloacina`
- `angreal check crate crates/cloacina-server`
- Regenerate the OpenAPI spec + TS SDK so the new types reach the client
  (whatever the project's emit-openapi / SDK-gen lane is), then the type-drift
  gate.

**Deferred / follow-ups:** per-request unpack caching if this ever gets hot;
optional per-task source slicing once a reliable function→file map exists.

*To be added during implementation*
