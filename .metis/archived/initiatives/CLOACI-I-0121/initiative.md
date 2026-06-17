---
id: audience-first-docs-restructure-1
level: initiative
title: "Audience-first docs restructure (1/3): IA spine & redirects"
short_code: "CLOACI-I-0121"
created_at: 2026-06-15T13:52:54.733035+00:00
updated_at: 2026-06-17T11:38:30.696427+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


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

Scope refined for the **one-big-PR** model (all three initiatives on one branch,
merged once after I-3). I-0121 is **purely additive structure** — it stands up the
new tree without disturbing existing content or nav, so every intermediate branch
state still builds. The **cutover** (home rewrite, nav flip, applied redirects,
old-section removal) is concentrated in I-3.

**Goals (this initiative):**
- Author the new top-level **IA section tree** and a page-by-page **mapping** (→ [[CLOACI-T-0689]], done).
- Stand up **section skeletons + landing (`_index`) pages** for the new sections (`/start`, `/embed` + sub-sections, `/service` + sub-sections, `/engine`, `/reference`) — additive, empty of bodies.
- Add **orientation-hub child shells** under `/start` and **door entry shells** (quick-start landings) so the full target tree exists and `hugo` builds green.

**Non-Goals (deferred):**
- **Home page** dual-door rewrite, **nav** (`hugo.toml`) cutover, applying **`aliases`** redirects, and **removing old sections** — all → I-3 (cutover).
- Relocating/reframing page **bodies**, renumbering tutorials → I-3.
- Authoring **primitive** content in `/engine` → I-2.
- Net-new **embedded-in-production** / embedded-first-principle pages → I-3.
- Closing the **Rust↔Python parity gaps** → [[CLOACI-T-0688]].

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

Tasks (decomposed as Metis tasks):

1. **IA tree + old→new page map** ([[CLOACI-T-0689]], done) — the design artifact everything follows.
2. **Section skeleton landings** (done in commit `46f3667`) — `/start`, `/embed` (+tutorials/how-to/explanation), `/service` (+same), `/engine`, `/reference`, additive, `hugo` green.
3. **Orientation-hub + door entry shells** — `/start` child shells (what-is-cloacina, is-cloacina-for-you, concepts, features, install) and per-door `quick-start` landing shells, so the full target tree exists. Empty/placeholder bodies; content authored in I-3.

Branch: `docs/audience-first-restructure` (shared by all three initiatives; one PR
at the very end of I-3). I-0121's bar: the full new tree exists and `hugo` builds
clean. The 4-reviewer adversarial gate + full link/redirect check run at the end
of I-3 against the complete content.