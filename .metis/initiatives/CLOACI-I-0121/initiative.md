---
id: audience-first-docs-restructure-1
level: initiative
title: "Audience-first docs restructure (1/3): IA spine & redirects"
short_code: "CLOACI-I-0121"
created_at: 2026-06-15T13:52:54.733035+00:00
updated_at: 2026-06-15T13:52:54.733035+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: audience-first-docs-restructure-1
---

# Audience-first docs restructure (1/3): IA spine & redirects Initiative

## Context

The server/platform surface (server, compiler, execution-agent fleet, web UI,
multi-tenancy, OIDC) has grown a lot, and the docs' narrative gravity drifted
server-ward — undermining the **embedded-first principle** ([[project_embedded_first_philosophy]]).
Decision (2026-06-15): a **full audience-first IA redesign** with **two co-equal
front doors** — *Embed the library* and *Run the service* — and embedded-first
retained as a stated **architectural principle** (the engine is a real standalone
library; the server is built on it). You pick a door by use case (operating it as
a service vs. integrating it into your own system), not by skill level — embedded
is a permanent, production-legitimate end-state, not training wheels.

This is **initiative 1 of 3**, executed serially:
- **I-0121 (this one)** — IA spine & redirects: the new section tree, the
  dual-door home, the orientation hub, navigation, and a redirect map so URLs
  survive. Skeleton + landings only; no body content moved.
- **I-2 (next)** — Primitives: the shared "Engine & primitives" area for the
  ratified core objects, Rust + Python shown side by side.
- **I-3 (last)** — The two doors + content migration: relocate/reframe existing
  material, renumber tutorials/examples, add net-new embedded-in-production and
  embedded-first-principle pages, kill all "graduate" framing, final review gate.

Ratified core primitives (for I-2): Workflow, Task, Context, Node, Computation
Graph, Reactor, Accumulator (passthrough/stream/polling/batch/state), Trigger
(poll/cron), Cron schedule, Boundary event, Package (`.cloacina`), Runner — with
**Package and Runner elevated** as first-class operational primitives, and Node /
Boundary documented coupled to their parents. Python is an **interface** on par
with the Rust macros; primitives show both. Known Rust↔Python parity gaps are
tracked in [[CLOACI-T-0688]].

This **supersedes** CLOACI-I-0120's "improve within the current structure /
preserve URLs" decision. I-0120's content-correctness work is merged (PR #126)
and carries forward.

## Goals & Non-Goals

**Goals (this initiative):**
- Author the new top-level **IA section tree** and a page-by-page **mapping** from every current doc to its new home.
- Build the **home page** with two co-equal doors (no "recommended" badge) and a thin **`/start` orientation hub**.
- Stand up **section skeletons + landing (`_index`) pages** for the new sections (embed, service, primitives area, consolidated reference).
- Update **global navigation** (hugo-geekdoc menu) to the new IA.
- Produce a **redirect map** and add Hugo `aliases` front matter on every page whose URL moves, verified by a clean build + link-check.

**Non-Goals (deferred):**
- Rewriting or relocating page **bodies** (→ I-3).
- Authoring **primitive** content (→ I-2).
- Net-new **embedded-in-production** / embedded-first-principle pages (→ I-3).
- Closing the **Rust↔Python parity gaps** (→ [[CLOACI-T-0688]]).

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

**Target IA (strawman, to be finalized in the first task):**

```
Home (/)                     "One engine, two ways to run it."  [Embed] [Run the service]
/start                       orientation hub (audience-neutral, short)
  what-is-cloacina           two-ways framing + embedded-first PRINCIPLE
  is-cloacina-for-you        fit/not-fit · pick a door · pick a primitive
  concepts · features · install-the-cli
/embed                       DOOR A — app developers (Rust + Python)
  quick-start · tutorials · how-to · explanation
/service                     DOOR B — operators + devs shipping packages
  quick-start · tutorials · how-to · explanation
/primitives (or /engine)     SHARED — the ratified core objects, dual-language   [I-2 fills this]
  workflows · computation-graphs · packaging
/reference                   audience-neutral lookup
  python-api · rust-api · cli · http-ws-api · config · metrics · sdks · glossary · troubleshooting
```

Synthesis: **audience-first at the top** (two co-equal doors), **Diátaxis within**
each door (tutorials/how-to/explanation are audience-specific), **reference
consolidated** as neutral lookup, and **primitives shared** because they cut
across both doors. Home presents both doors with equal weight (no recommended
badge).

**Redirect strategy.** Hugo supports per-page `aliases:` front matter that emits
client-side redirects from old URLs. For every page whose URL changes we add the
old path(s) to the new page's `aliases:`. A page-by-page old→new map is a
required deliverable (task 1 + task 4). Verification: `hugo` builds clean, an
internal link-check passes (every `{{< ref >}}` resolves), and a spot-check of
old URLs 30x-redirects to the new location.

**Constraints:** Hugo + hugo-geekdoc; existing shortcodes (`{{< ref >}}`,
`{{< toc-tree >}}`, `{{< hint >}}`). Auto-generated `api-reference/` is machine
-produced — fold into `/reference` by routing/nav, do not hand-edit generated
pages.

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

- **Improve within the current structure / preserve URLs** (I-0120's approach) —
  rejected: it can't fix the server-ward gravity; the audience split needs a
  structural change.
- **Service-led single front door** (lead everything with the server, since ~70%
  of target users want it) — rejected in favor of **co-equal dual doors**, to keep
  embedding genuinely first-class rather than a buried alternative.
- **Keep "embedded-first" as the headline onboarding ramp** — rejected: it routes
  newcomers onto the path most shouldn't start on and frames embedding as training
  wheels. Embedded-first stays as the *architectural principle*, not the onboarding
  default.
- **Re-route + reorder but keep the existing tree/URLs** — rejected in favor of a
  greenfield redesign; the tree itself encodes the old (feature-area, server-heavy)
  framing.

## Implementation Plan

Tasks (decomposed below as Metis tasks):

1. **Finalize the IA tree + old→new page map** — authoritative section structure and a row for every existing doc → its new path (or "stays"). The design artifact everything else follows.
2. **Home page (dual doors) + `/start` orientation hub** — the two co-equal entry cards and the orientation landings (what-is / is-it-for-you / concepts / features / install), reusing existing orientation content by reference.
3. **Section skeletons + landing `_index` pages** — create `/embed`, `/service`, the shared primitives area, and `/reference` with `_index` + `{{< toc-tree >}}`; empty of bodies (filled by I-2/I-3).
4. **Redirect map (Hugo `aliases`)** — apply old→new aliases from task 1's map to every moved/new landing; this initiative covers the landings it creates, with the bulk of body-level redirects applied alongside the moves in I-3.
5. **Global navigation** — update the hugo-geekdoc menu to the new IA.
6. **Verify** — `hugo` clean build, internal link-check (all `{{< ref >}}` resolve), old-URL redirect spot-check, no orphaned/again-duplicated landings.

Branch: `docs/i0121-ia-spine`. One PR for the initiative. The 4-reviewer
adversarial gate runs at the end of I-3 (full content); this initiative's bar is
a clean build + link-check + the IA map reviewed by the user.
