---
id: audience-first-docs-restructure-2
level: initiative
title: "Audience-first docs restructure (2/3): Engine & primitives (dual-language)"
short_code: "CLOACI-I-0122"
created_at: 2026-06-15T14:14:28.867223+00:00
updated_at: 2026-06-15T14:14:28.867223+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: audience-first-docs-restructure-2
---

# Audience-first docs restructure (2/3): Engine & primitives (dual-language) Initiative

## Context

Initiative **2 of 3** in the audience-first docs restructure (parent design +
ratified IA: [[CLOACI-I-0121]]; page map: [[CLOACI-T-0689]]). Runs on the shared
branch `docs/audience-first-restructure`; merges as part of the single end-of-I-3
PR.

This initiative authors the **`/engine`** section — the shared "Engine &
Primitives" area that both doors (`/embed`, `/service`) link into. The engine is
where each **core object is described once**, independent of how Cloacina is run,
with **both its Rust and Python interfaces** shown (Rust the default; Python the
peer dialect — Python is an interface on par with the Rust macros). Tutorials
(learning-by-doing) stay in the doors; the *concept + reference* of each object
lives here.

Ratified core primitives (from I-0121): **Workflow, Task, Context, Computation
Graph, Node, Reactor, Accumulator** (passthrough/stream/polling/batch/state),
**Boundary event, Trigger** (poll/cron), **Cron schedule, Package** (`.cloacina`),
**Runner** — with **Package and Runner elevated** as first-class operational
primitives, and **Node / Boundary documented coupled** to their parents (Node↔Computation
Graph, Boundary↔Accumulator/Reactor). Known Rust↔Python authoring-parity gaps
(state accumulators, packaged cron-trigger decorator) are tracked in
[[CLOACI-T-0688]] and surfaced as honest caveats here, not hidden.

This is **additive** — it creates new `/engine` pages without disturbing existing
content or nav. Relocating the old `/workflows` & `/computation-graphs` concept/
reference/explanation bodies (and redirects) happens in I-3.

## Goals & Non-Goals

**Goals:**
- A coherent **`/engine` section** grouping the primitives (suggested sub-grouping: `workflows`, `computation-graphs`, `packaging` + per-primitive pages).
- One **concept page per primitive** (12 objects), each: a plain-language definition, the mental model, relationships, and **Rust + Python interface** shown via `{{< tabs >}}` (Rust default).
- **Node** and **Boundary** documented **coupled** to their parents (not standalone orphans).
- Every code-bearing claim/example **grounded against current source** (accuracy-reviewer pass); **parity caveats** linked to [[CLOACI-T-0688]] where an interface is missing.
- The section **builds clean** (`hugo`) and reads coherently on its own.

**Non-Goals:**
- Moving the existing `/workflows` & `/computation-graphs` bodies, tutorials, door how-tos, redirects, nav, home → **I-3**.
- Closing the parity gaps themselves → [[CLOACI-T-0688]].
- Reference API pages (cli/http/api surfaces) → those consolidate into `/reference` in **I-3**; `/engine` is concept + primitive-level reference only.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design

**`/engine` structure:**

```
/engine
  _index.md                  (exists — overview + primitive list)
  workflows/                 Workflow, Task, Context, Runner (the durable-DAG cluster)
  computation-graphs/        Computation Graph, Node, Reactor, Accumulator, Boundary (the in-process cluster)
  scheduling/                Trigger (poll/cron), Cron schedule
  packaging/                 Package (.cloacina)
```

(Grouping is a guide; each primitive gets its own page. Node lives under its
graph page; Boundary under accumulator/reactor — coupled, not standalone.)

**Per-primitive page template (concept + reference, austere where reference):**
1. One-sentence definition (what it IS to a user).
2. Mental model / where it sits (relationships: contains / fires / feeds).
3. **Interfaces** — `{{< tabs >}}` with a **Rust** tab (default) and a **Python**
   tab showing the same thing in each dialect (`#[task]` ↔ `@cloaca.task`, etc.).
4. Key fields / signatures (reference), each code-traceable.
5. Parity note where an interface is missing (link [[CLOACI-T-0688]]).
6. Cross-links: to the door tutorials that *build* with it, and to `/reference`
   for the full API.

**Grounding rule (non-negotiable):** every signature/field/default/example traces
to a specific `crates/...` location. Reuse the verified surface already captured
during I-0120 (e.g. `DefaultRunnerConfig` fields, `@cloaca.task` retry kwargs,
accumulator variants, cron methods). Re-verify anything not already grounded.

**Dual-language convention:** Rust tab first/default; Python tab second. Where a
primitive is Rust-only in authoring (state accumulator; packaged cron-trigger
decorator), the Python tab states the gap and points at the runtime alternative
(e.g. Python cron via `register_cron_workflow`) + [[CLOACI-T-0688]].

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered

- **Separate Rust and Python primitive pages** — rejected: it re-silos the
  languages, the exact problem the restructure fixes. One page, two tabs.
- **Put primitives inside each door** (duplicate under `/embed` and `/service`) —
  rejected: primitives cut across both doors; duplicating them drifts. Single
  shared `/engine` home, doors link in.
- **Hide the Rust↔Python parity gaps** (only show what's symmetric) — rejected:
  dishonest. Show the gap + the workaround + the tracking ticket.
- **One mega "primitives" page** — rejected: too dense for lookup; one page per
  object, grouped, is navigable.

## Implementation Plan

Decomposed as Metis tasks (by primitive cluster, so each is independently
authorable + reviewable):

1. **`/engine` sub-structure + section landings** — create `workflows/`,
   `computation-graphs/`, `scheduling/`, `packaging/` sub-landings; refine the
   `/engine` overview.
2. **Durable-DAG cluster** — Workflow, Task, Context, Runner (concept + reference,
   dual-language; Runner elevated).
3. **In-process cluster** — Computation Graph, Node (coupled), Reactor,
   Accumulator (+5 variants), Boundary (coupled).
4. **Scheduling cluster** — Trigger (poll/cron), Cron schedule (note Python cron
   is runtime-API, packaged cron-trigger decorator is Rust-only → [[CLOACI-T-0688]]).
5. **Packaging** — Package (`.cloacina`) as an elevated primitive.
6. **Grounding + parity pass** — accuracy-reviewer over every `/engine` page;
   confirm each example traces to source; wire parity caveats. `hugo` clean build.

Each task: author the page(s), show Rust+Python tabs, ground against code, cross-link.
The cross-door tutorial cross-links may point at pages that don't move until I-3 —
acceptable on the shared branch; the full link-check runs at the end of I-3.
