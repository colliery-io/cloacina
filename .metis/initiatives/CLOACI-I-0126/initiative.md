---
id: workflow-legibility-surface-source
level: initiative
title: "Workflow legibility — surface source and what/why documentation at the data layer"
short_code: "CLOACI-I-0126"
created_at: 2026-06-20T02:37:58.981184+00:00
updated_at: 2026-06-20T02:37:58.981184+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: workflow-legibility-surface-source
---

# Workflow legibility — surface source and what/why documentation at the data layer

## Context

Surfaced during a live demo (2026-06-18/19). Compiled workflows are currently a
"black box" to anyone looking at them through the platform: you can see a
workflow's DAG, task metadata, and execution status, but not **what the code is**
or **why it exists**. Two distinct gaps:

1. The original task/workflow **source** is retained (the compiler stores the
   `.cloacina` archive in `workflow_registry.data`) but is never exposed past the
   database.
2. There is no place to author the **what & why** (intent/rationale) of a
   workflow or task beyond a single optional one-line `description` in the
   manifest.

This initiative makes workflows *legible* — but strictly at the **data layer**.
A designer is actively reviewing the current UI and giving strong feedback; we
are **not churning the frontend** right now. This initiative therefore stops at
the API/SDK boundary: it makes source and documentation **available for UI
consumption**. The UI rendering is deliberately deferred to a later, separate
effort once the design direction settles.

## Goals & Non-Goals

**Goals:**
- Expose the already-retained workflow/task source over the server API, scoped
  and authorized like other workflow reads.
- Define an opinionated, low-ceremony convention for authoring the what & why of
  workflows/tasks, carry it through compile/packaging into the manifest, and
  expose it over the API.
- Keep `cloacina-api-types` / the SDK in lockstep so a future UI can consume both
  without type drift.

**Non-Goals:**
- **No frontend work.** No React components, viewers, syntax highlighting,
  markdown rendering, or any `ui/` changes — those are a later, separate effort.
- No change to the compiler or package format for source (source is already
  retained); the only packaging change is carrying structured docs in the
  manifest.
- Not a docs-site/Diátaxis effort — this is per-workflow, in-product metadata,
  not external documentation.

## Requirements

### Functional Requirements
- REQ-001: An API endpoint returns the stored source for a workflow/task, scoped
  to the tenant, including a listing of available source files plus a requested
  file's contents.
- REQ-002: Source exposure works for both Rust and Python packages (or v1 scopes
  to one language and says so explicitly).
- REQ-003: An opinionated authoring convention captures structured what & why
  (candidate fields: what / why / when / caveats) for workflows and tasks, in
  both Rust and Python.
- REQ-004: Structured docs are captured into the package manifest at
  compile/package time — no second hand-maintained source of truth that can
  drift.
- REQ-005: An API field/endpoint exposes the structured docs per workflow/task;
  "undocumented" is represented as a clean, typed state.

### Non-Functional Requirements
- NFR-001: Source/doc reads reuse existing workflow-read authorization and tenant
  scoping; no new trust surface.
- NFR-002: Source retrieval is mindful of archive size — stream or cache unpacked
  source rather than unpacking the whole archive per request.

## Detailed Design

The work decomposes into two tasks, both data-layer only:

- **CLOACI-T-0750 — Workflow source via API.** Add a registry accessor that
  reads + unpacks the stored `.cloacina` archive
  (`crates/cloacina/src/registry/workflow_registry/mod.rs`), a
  `GET /v1/tenants/{tenant_id}/workflows/{name}/source` endpoint
  (`crates/cloacina-server/src/routes/workflows.rs`), and a source response type
  in `crates/cloacina-api-types/src/workflows.rs`. No compiler/package change.
- **CLOACI-T-0752 — Opinionated what & why docs.** Settle the authoring surface
  (lean on language-native docstrings/doc-comments where possible), define the
  structured schema, carry it into the manifest
  (`crates/cloacina/src/packaging/manifest_schema.rs`) at compile time, and
  expose it over the API/SDK.

Sequencing: the two are largely independent and can proceed in parallel; if a
shared workflow/task "detail" API surface emerges, prefer exposing source and
docs through the same response shape so the future UI has one place to read from.

## Alternatives Considered

- **Build the UI now alongside the data.** Rejected for now — the designer is
  mid-review and we don't want to churn the frontend. Decoupling the data layer
  lets the UI land later against a stable API.
- **Just widen the existing one-line `description`.** Rejected — the demo gap is
  about real intent/rationale; an opinionated, structured schema is what makes
  the docs consistent and worth surfacing.
- **Derive docs purely from code (no manifest carry).** Partially adopted —
  prefer deriving from language-native docstrings, but the structured result must
  still land in the manifest so the server can serve it without re-reading source
  (ties into CLOACI-T-0736's FFI-derive-from-code machinery to avoid drift).

## Implementation Plan

1. Discovery/design check-in (this initiative) — confirm scope split and the
   doc schema/authoring surface with the human before decomposing further.
2. CLOACI-T-0750 — source via API.
3. CLOACI-T-0752 — opinionated what & why docs through the manifest + API.
4. (Later, out of scope here) UI consumption of both — separate effort once the
   design direction settles.

## Child Tasks

- **CLOACI-T-0750** — Workflow source code — expose the retained .cloacina source
  via API for UI consumption.
- **CLOACI-T-0752** — Opinionated task/workflow documentation — capture the what &
  why and expose it via API for UI consumption.
