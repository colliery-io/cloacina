---
id: 001-injectable-input-interface-json
level: adr
title: "Injectable input interface — JSON Schema descriptor + native authoring surface"
number: 1
short_code: "CLOACI-A-0007"
created_at: 2026-06-20T15:40:31.662609+00:00
updated_at: 2026-07-05T16:30:51.328955+00:00
decision_date:
decision_maker: dstorey
parent: CLOACI-S-0013
archived: false

tags:
  - "#adr"
  - "#phase/decided"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Injectable input interface — JSON Schema descriptor + native authoring surface

## Context

Initiative [[CLOACI-I-0128]] / spec [[CLOACI-S-0013]] makes every injectable
surface (workflow execute context, accumulator ingest, reactor fire) declare its
input interface — named, typed slots — captured into package metadata and exposed
over the API for typed UI/CLI input and server-side validation. Two design
choices must be pinned before implementation: (1) how a typed slot is
*represented*, and (2) where a workflow's params are *authored* (accumulators and
reactors already carry the types in code; workflow context does not).

## Decision

1. **Type descriptor = JSON Schema**, generated from Rust types via the
   `schemars` crate and from Python type hints. Each slot is
   `{ name, schema, required, default? }` where `schema` is a JSON Schema
   fragment. One representation across all three surfaces and both languages.
2. **Workflow param authoring = native, at the definition site**: a Rust
   `#[workflow(params(...))]` attribute (extending the existing macro) and Python
   type hints. No `package.toml` params block; no `ctx.get` inference (inference
   may be a future best-effort fallback only).
3. **Rust types remain the single source of truth.** The JSON Schema is a
   `schemars`-*derived projection* of the declared Rust types — not a hand-written
   schema. This preserves the principle in [[CLOACI-I-0116]]'s decision #2 while
   replacing its `type_name`-string descriptor with the richer JSON Schema form.
4. **Carry = a dedicated FFI descriptor entrypoint** (decided 2026-06-20). The
   macro emits a `schemars`-derived input-interface descriptor; a **new, dedicated
   FFI function** on the built cdylib returns it as JSON, and the compiler
   captures it at build-success into the package metadata JSON
   (`workflow_packages.metadata`). This gets **full `schemars` fidelity for both
   workflow params and accumulator/reactor boundaries** (rich structs, not just
   scalars), uniformly. It deliberately does **NOT** modify the drift-prone
   `TaskMetadataEntry` bincode wire struct — which is what the "no FFI" guidance
   was really protecting (the [[CLOACI-T-0736]] drift class). Refines the earlier
   "compiler-side, no FFI" wording: a clean separate entrypoint is fine; changing
   the drift-prone metadata struct is not. (Source-parse-of-scalars was rejected
   because it can't describe accumulator/reactor struct boundaries.)
5. **Validation = server-side at inject time.** Execute context / reactor fire /
   accumulator event are validated against the declared schema; mismatch returns
   a typed `*_input_invalid` error. Client-side validation is an additive UI
   nicety, not the contract.

## Alternatives Analysis

| Option (descriptor) | Pros | Cons | Risk | Cost |
|--------|------|------|------|------|
| **JSON Schema via schemars (chosen)** | Rich (nested/enums/constraints); language-agnostic; mature tooling; drives both UI forms and validation | `schemars` dep + derive on param/boundary types; Python type-hint→schema step | Low | Medium |
| Minimal hand-rolled type enum | No new dep; fully controlled | Reinvents a JSON-Schema subset; weak for nested/validated types | Medium | Medium |
| Rust type-name strings (I-0116's original) | Cheapest | Opaque to UI/validation; useless cross-language | High (misses the goal) | Low |

| Option (authoring) | Pros | Cons |
|--------|------|------|
| **Native attribute + type hints (chosen)** | Docs next to code; types are truth; fits I-0125 | Macro work; Python type-hint extraction |
| `[params]` block in package.toml | Explicit, language-agnostic | Drifts from code; hand-written types |
| Infer from `ctx.get` | Zero authoring | reads≠requires; no types/defaults |

## Rationale

JSON Schema is the only descriptor that simultaneously (a) spans Rust + Python,
(b) covers all three surfaces, (c) is rich enough to drive a real UI form and
real server validation, and (d) can be *derived* from existing Rust types
(`schemars`) so authors keep writing plain types. Native authoring keeps the
declaration co-located with code and reuses the existing `#[workflow]` macro
surface (already designed in I-0116). Compiler-side carry sidesteps the FFI
churn we deliberately avoided in T-0752.

## Consequences

### Positive
- One typed-input contract across workflows, accumulators, reactors.
- Typed manual execute (T-0747) and typed operator injection (T-0751/T-0753)
  fall out of the same model; retires raw `Vec<u8>` injection.
- I-0116 reuses this descriptor (no divergent param model).

### Negative
- New `schemars` dependency and derives on param/boundary types.
- Python type-hint → JSON Schema needs a toolchain decision (open).
- Revises I-0116's prior `type_name` descriptor choice (consistent in spirit).

### Neutral
- v1 keeps reactor/accumulator payloads as `Vec<u8>` on the wire; typing is a
  metadata + validation layer over that, not a wire rewrite.

## Open (tracked in I-0128, not blocking this decision)
- Python type-hint → JSON Schema toolchain.
- Reactor per-source schema resolution from bound upstream accumulators.
