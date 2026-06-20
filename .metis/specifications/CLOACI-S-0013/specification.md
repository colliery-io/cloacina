---
id: injectable-input-interface-json
level: specification
title: "Injectable input interface — JSON-Schema-typed named inputs across injectable surfaces"
short_code: "CLOACI-S-0013"
created_at: 2026-06-20T14:58:42.518394+00:00
updated_at: 2026-06-20T14:58:42.518394+00:00
parent: CLOACI-I-0128
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Injectable input interface — JSON-Schema-typed named inputs across injectable surfaces

## Overview

Cloacina has several **injectable surfaces** — places where a caller pushes data
in to make something run: a workflow execution's context, an accumulator's event
ingest, and a reactor's fire input. Today each accepts an **untyped or
type-erased** payload, with no machine-readable declaration of what it expects:

- **Workflow execute** — a free-form `context: serde_json::Value` blob
  (`ExecuteRequest`); names/types are undeclared.
- **Accumulator ingest** — a typed Rust boundary exists at the type level
  (`trait Accumulator { type Output: Serialize … }`) but is erased to `Vec<u8>`
  on the wire and never surfaced.
- **Reactor fire** — sources are **named** (`SourceName` / `expected_sources`)
  but each payload is erased to `Vec<u8>`; the real type lives in the upstream
  accumulator's `Output`.

This spec defines a single, cross-cutting model: **every injectable surface
declares its input interface — a set of named, typed slots — at its definition
site, captured into package metadata and exposed over the API** so that:

- the UI can render a typed form instead of a "paste a JSON blob" box (the
  T-0747 demo gap),
- operator injection tooling (CLI/REST) accepts **typed input** instead of raw
  `Vec<u8>` (the T-0751 gap),
- the server can **validate** an injection against the declared interface before
  accepting it.

The type system used to describe a slot is **JSON Schema** (decision below).
This is the data-layer contract only; UI rendering is a downstream consumer
(currently under a frontend freeze) and out of scope here.

## System Context

### Actors
- **Workflow/graph author** — declares the input interface at the definition
  site (Rust attribute / Python type hints / boundary types).
- **Operator** — submits a typed execution or a manual injection through the UI
  or CLI; benefits from discoverability + validation.
- **Compiler service** — parses/derives the interface at build time and persists
  it into package metadata (the carry mechanism).
- **Server API + SDK** — exposes the declared interface and validates injections.

### Boundaries
- **In scope:** the interface *model* (named typed slots), its *type descriptor*
  (JSON Schema), per-surface *declaration*, compile-time *carry* into metadata,
  *API exposure*, and *server-side validation* at inject time.
- **Out of scope:** UI rendering (frontend freeze; later consumer), the
  parameterization/partials runtime of I-0116 beyond the declared-interface
  portion, and a new accumulator injection *endpoint* (tracked separately —
  [[CLOACI-T-0753]]).

## The three injectable surfaces

| Surface | Slot identity | Type today | Work |
|---|---|---|---|
| Workflow execute (`/workflows/{name}/execute`) | named context keys | none (free-form `context`) | **new declaration** |
| Accumulator ingest (`/ws/accumulator/{name}`) | the accumulator/source name | Rust `type Output: Serialize` (erased to bytes) | **surface existing type** |
| Reactor fire (`/health/reactors/{name}/fire`, `/ws/reactor/{name}`) | per-source (`SourceName`) | upstream accumulator `Output` (erased to bytes) | **surface existing type** |

Common end-state: each surface advertises `inputs: [{ name, schema, required,
default? }]`, where `schema` is a JSON Schema fragment.

## Requirements

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | Each injectable surface exposes a declared input interface: a list of slots, each with a `name`, a JSON Schema `schema`, a `required` flag, and an optional `default`. | The single contract all three surfaces share. |
| REQ-1.1.2 | Workflow param interfaces are **authored** at the definition site (see open decision on authoring surface) and carried into metadata at compile time. | Workflow context is otherwise untyped. |
| REQ-1.2.1 | Accumulator and reactor interfaces are **derived** from the existing boundary/source types (their `Serialize` types) rather than hand-declared, where possible. | The type info already exists; avoid duplicate authoring + drift. |
| REQ-1.3.1 | The type descriptor for every slot is **JSON Schema**, generated from Rust types via `schemars` and from Python type hints. | One language-agnostic representation across surfaces + both languages (decided). |
| REQ-1.4.1 | The declared interface is exposed via the API/SDK: per-workflow on the workflow detail surface; per-accumulator/reactor on the CG-health surfaces. | UI/CLI consumption. |
| REQ-1.5.1 | The server **validates** an injection (execute context / reactor fire / accumulator event) against the declared interface and rejects mismatches with a typed error. | Replaces raw, unchecked `Vec<u8>` / blob injection. |
| REQ-1.6.1 | Surfaces with no declared/derivable interface degrade gracefully to a clearly-typed "undeclared" state (free-form still accepted). | Back-compat for existing packages. |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | Carry is **compiler-side**, into the `workflow_packages.metadata` JSON (or equivalent), NOT through the bincode FFI wire struct. | Avoids the in-flux fidius/FFI authoring shift + the blocked T-0736; consistent with the T-0752 precedent. |
| NFR-1.1.2 | Schemas added to stored metadata use `#[serde(default)]` so older packages remain deserializable. | Back-compat. |
| NFR-1.2.1 | Validation reuses the existing per-surface authorization (tenant scope, reactor op auth). | No new trust surface. |

## Architecture Framing

### Decision Area: Type descriptor (DECIDED)
- **Context**: one representation of a typed slot across workflows, accumulators,
  reactors, and Rust + Python.
- **Decision**: **JSON Schema**, via `schemars` for Rust types and from Python
  type hints. Rich (nested types, enums, constraints), language-agnostic, mature
  tooling, and directly drives both UI form rendering and server validation.
- **Cost**: a `schemars` dependency + a derive on boundary/param types; a Python
  type-hint → JSON Schema step.

### Decision Area: Workflow param authoring surface (DECIDED)
- **Context**: workflow context is untyped; authors must declare params somewhere.
- **Decision (2026-06-20)**: **native — a Rust `#[workflow(params(...))]`
  attribute + Python type hints** (matches I-0116's existing `#[workflow(params)]`
  design). Docs live next to code; the JSON Schema descriptor is `schemars`-derived
  from the declared Rust types (so "Rust types are the source of truth" holds — the
  schema is a derived projection). `package.toml` block and `ctx.get` inference
  rejected. Needs an ADR to record it.

### Decision Area: Accumulator/reactor derivation (OPEN)
- **Context**: the types exist as Rust associated types; derive their JSON Schema
  automatically at build time. Confirm the Python boundary-type story and how
  reactor per-source types resolve from upstream accumulators.

### Decision Area: Validation point (lean: server-side at inject time)
- Reject mismatched execute context / reactor fire / accumulator event against
  the declared schema; return a typed `*_input_invalid` error. Client-side
  validation is an additive UI nicety, not the contract.

## Constraints

### Technical Constraints
- Compiler-side carry only (no FFI wire-format change) — see NFR-1.1.1.
- Reactor/accumulator payloads are currently `Vec<u8>` on the wire; typing is a
  metadata + validation layer over that, not a wire rewrite (v1).

## Relationship to existing specs/initiatives
- Extends **CLOACI-S-0002** (ComputationBoundary & Accumulators),
  **CLOACI-S-0004** (Accumulator trait), **CLOACI-S-0005** (Reactor) — those
  define the boundary/source types this spec surfaces as declared interfaces.
- **CLOACI-I-0116** (Parameterized workflow instances) is **separate but adjacent
  and builds on this**. Distinction (maintainer, 2026-06-20): I-0128 is the
  generic *"this surface accepts these named, typed inputs at execute/inject
  time"* foundation; I-0116 is a *"partial config / constructor"* system on top —
  treat a workflow as a constructor, bind values to its I-0128-declared inputs to
  produce a unique **named, registered, scheduled instance** (the partial +
  lifecycle + schedule-merge + packaged/Python runtime). I-0116 **reuses this
  spec's JSON-Schema descriptor** for its `ParamSpec` (revising its earlier
  type-name-string choice — same "Rust types are truth" principle, richer derived
  projection). Dependency: I-0116 → I-0128.
- **CLOACI-T-0747** (UI manual-execute config options) becomes a **consumer** of
  the workflow interface.
- **CLOACI-T-0751** (operator reactor fire) gains **typed** input from the reactor
  interface; **CLOACI-T-0753** opens the accumulator injection surface this types.
- Carry mechanism mirrors **CLOACI-T-0752** (compiler-side parse → metadata JSON).

## Open Questions
- ~~Authoring surface for workflow params~~ — DECIDED: native attribute +
  type hints (ADR still to be written to record it).
- Python type-hint → JSON Schema toolchain.
- How reactor per-source schemas resolve from the bound upstream accumulators.
- Whether `default` lives in the schema (`default` keyword) or as a sibling field.
