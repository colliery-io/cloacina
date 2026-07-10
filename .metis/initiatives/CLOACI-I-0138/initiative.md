---
id: packaged-first-examples-server
level: initiative
title: "Packaged-first examples — server/daemon gold path as the standard for all examples"
short_code: "CLOACI-I-0138"
created_at: 2026-07-10T00:22:40.417461+00:00
updated_at: 2026-07-10T00:22:40.417461+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: packaged-first-examples-server
---

# Packaged-first examples — server/daemon gold path as the standard for all examples Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

> **Phase: discovery.** Maintainer decision (2026-07-09): "we're going packaged-first for all examples going forward." Awaiting a scoping check-in before design.

**Direction.** Every Cloacina example is **packaged-first**: author a workflow → compile to a `.cloacina` package → run it through the **server / daemon** (loader path), not by instantiating an in-process `DefaultRunner` embedded in the example. The server/daemon path is the **gold path to adoption** — the flow new users should be onboarded through.

**Why now.** The embedded-wheel path (in-process `DefaultRunner`, `import cloaca`) is the fragile, non-gold shape: it needs a runner just to stand up the runtime (the wheel-based pytest scenarios only work via the `shared_runner` fixture for exactly this reason; a fixture-less wheel import even crashes on py3.12). Surfaced concretely during [[CLOACI-I-0137]] — the wheel-based authorship test (scenario_34) was the wrong shape and was dropped in favor of the server-path `ensure_cloaca_module` contract test.

**Relationship to embedded-first ([[project_embedded_first_philosophy]]).** NOT a reversal — embedded stays a legitimate, documented production DEPLOYMENT mode. This is about EXAMPLES / onboarding leading with the packaged/server path. (Flag to reconcile in design so docs guidance stays coherent.)

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Establish packaged-first as the STANDARD for examples: a clear packaged-example template/pattern (author → compile → load into server → run via API/daemon) that new examples follow.
- Reshape example test coverage to exercise the loader/server path rather than the embedded wheel.

**Non-Goals (pending check-in):**
- Deprecating or removing the embedded deployment mode itself (embedded stays production-legitimate).
- (SCOPE TBD with maintainer) whether to migrate ALL existing embedded examples now vs. new-examples-only + incremental migration.

## Decisions (2026-07-09 check-in)
- **D-1 Scope:** NEW-first + incremental. Lock packaged-first as the standard, build the canonical packaged-example template, apply to new examples immediately; convert the existing embedded examples incrementally (own tasks).
- **D-2 Docs:** Examples tree FIRST; re-cut the Diátaxis tutorials packaged-first as a LATER phase, once the pattern is proven.

## Grounding (2026-07-09) — the pattern already exists
Reference packaged examples: `examples/features/workflows/{simple-packaged, packaged-workflows, packaged-triggers, registry-execution}` (registry-execution's README shows the compile→load-into-server→run recipe) + `computation-graphs/{packaged-graph, python-packaged-graph}`. Many `examples/fixtures/*` are packaged too but are TEST FIXTURES, not user-facing examples. Still-embedded feature examples (the incremental-conversion backlog): complex-dag, conditional-retries, cron-scheduling, deferred-tasks, event-triggers, multi-tenant, per-tenant-credentials, python-workflow.

**Proposed design (for sign-off):** canonicalize a `simple-packaged`-style reference as THE template (package.toml + workflow src + a README run-recipe through the server/daemon, both Rust and Python), write a short "packaged example standard" note new examples follow, then decompose the embedded→packaged conversions into per-example tasks. Awaiting go-ahead to draft the template.

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

## Detailed Design **[REQUIRED]**

{Technical approach and implementation details}

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

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}