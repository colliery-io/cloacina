---
id: explicit-injectable-input
level: initiative
title: "Explicit injectable input interfaces — typed, named inputs for workflows, accumulators, and reactors"
short_code: "CLOACI-I-0128"
created_at: 2026-06-20T14:58:35.112535+00:00
updated_at: 2026-06-20T14:58:35.112535+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: explicit-injectable-input
---

# Explicit injectable input interfaces — typed, named inputs for workflows, accumulators, and reactors

## Context

Came out of the T-0747 discussion (2026-06-20). Rather than build "declared
config options" for workflows alone, the directive is broader: **make the
interface to *any* injectable surface explicit in its definition — the names and
types it accepts.** The injectable surfaces are workflow execution context,
accumulator event ingest, and reactor fire input. Today all three accept untyped
or type-erased payloads (free-form `context` blob; `Vec<u8>` on the
accumulator/reactor wire), so there is nothing for the UI to render, no typed
operator injection, and no server-side validation.

The design contract is specified in **[[CLOACI-S-0013]]** (Injectable input
interface). This initiative implements it.

## Goals & Non-Goals

**Goals:**
- A single model — `inputs: [{ name, schema, required, default? }]` — declared at
  each surface's definition site, where `schema` is **JSON Schema**.
- Workflow params: authored (attribute/type-hints) and carried compiler-side.
- Accumulator/reactor: JSON Schema **derived** from the existing boundary/source
  types (`schemars` on the Rust `Serialize` types; Python type hints).
- Expose the interface via API/SDK; **validate** injections server-side.
- Make operator injection **typed** (retire raw `Vec<u8>`), and let the UI later
  render typed forms.

**Non-Goals:**
- No frontend work (UI consumes this later; design review freeze in effect).
- No FFI wire-format change — carry is compiler-side into metadata JSON
  (consistent with T-0752; avoids the fidius/FFI shift + blocked T-0736).
- Not the full I-0116 parameterization runtime (partials, configurable
  schedule) beyond the declared-interface portion.
- The accumulator injection *endpoint* itself is opened separately
  ([[CLOACI-T-0753]]); this initiative types it once it exists.

## Decided

- **Type descriptor: JSON Schema** (via `schemars` for Rust, type hints for
  Python). Keystone decision; everything hangs off it.
- **v1 scope: all three surfaces together** (workflows + accumulators + reactors).
- **Carry: compiler-side** into the package metadata JSON.

## Detailed Design

See [[CLOACI-S-0013]] for the full model. Implementation seams (to refine on
decomposition):
- **Type descriptor plumbing**: `schemars` dependency; a `ParamSpec`/`InputSlot`
  type (`{ name, schema, required, default }`) in the shared types crate.
- **Workflow params**: authoring surface (ADR pending) → compiler-side capture
  (mirrors `cloacina-compiler/src/doc_parse.rs` from T-0752) → merge into
  `workflow_packages.metadata` → expose on `WorkflowDetail` → validate in the
  execute chokepoint (`routes/executions.rs`).
- **Accumulator/reactor**: derive JSON Schema from the boundary/source types at
  build time; surface on the CG-health detail responses; validate at the
  fire/ingest endpoints (typed input replaces `Vec<u8>` in
  `routes/health_graphs.rs::fire_reactor` and the accumulator injection from
  T-0753).
- **Validation**: typed `*_input_invalid` errors; reuse existing per-surface auth.

## Alternatives Considered
- **Minimal hand-rolled type enum** instead of JSON Schema — rejected: reinvents
  a JSON Schema subset, weaker for nested/validated types, no tooling.
- **Rust type-name strings** — rejected: opaque to UI/validation, useless
  cross-language.
- **Workflows only** for v1 — rejected by the directive; the value is the unified
  model across all injectable surfaces.

## Implementation Plan
Discovery → design (resolve the open ADRs in S-0013: workflow authoring surface,
accumulator/reactor derivation, Python type-hint toolchain) → **human check-in
before decomposition** (per initiative HITL) → implement per-surface slices →
verify via angreal unit + integration lanes.

## Relationship to I-0116 (decided 2026-06-20: separate, I-0116 builds on this)

Maintainer framing: these are **separate but adjacent**. I-0128 is the generic
foundation — *"this surface accepts these named, typed inputs at execute/inject
time."* I-0116 ("Parameterized workflow instances") is a **partial-config /
constructor** system **on top**: treat a workflow as a constructor, bind values
to its I-0128-declared inputs to produce a unique **named, registered, scheduled
instance** (partial + lifecycle + schedule-merge + packaged/Python runtime).

- **Dependency**: I-0116 → I-0128. I-0128 ships first / underneath.
- **Shared descriptor**: I-0116 reuses I-0128's `schemars`-derived JSON-Schema
  descriptor for its `ParamSpec` (revises I-0116's earlier type-name-string
  choice; same "Rust types are truth" principle, richer derived projection).
- I-0116 stays its own initiative; not folded or closed.

## Related Work
- **CLOACI-S-0013** — the spec (this initiative implements it).
- **CLOACI-I-0116** — Parameterized workflow instances; separate, builds on this
  (see section above).
- **CLOACI-T-0747** — UI manual-execute config options; consumer of the workflow
  interface (gated on this + the frontend freeze).
- **CLOACI-T-0751** — operator reactor fire; gains typed input here.
- **CLOACI-T-0753** — opens the operator-facing accumulator injection surface.
- **CLOACI-T-0752** — compiler-side carry precedent.
- **CLOACI-S-0002 / S-0004 / S-0005** — boundary/accumulator/reactor types this
  surfaces.

## Decisions (2026-06-20)
- **Type descriptor**: JSON Schema via `schemars` (Rust) + Python type hints.
- **Scope**: all three surfaces (workflows + accumulators + reactors).
- **Carry**: compiler-side into package metadata JSON (no FFI change).
- **Workflow authoring surface**: native `#[workflow(params(...))]` + Python type
  hints (matches I-0116). ADR still to be written.
- **I-0116**: separate, builds on I-0128 (see above).

## Open Decisions (pre-decomposition)
1. Python type-hint → JSON Schema toolchain.
2. Reactor per-source schema resolution from bound upstream accumulators.
3. Write the authoring-surface ADR.
