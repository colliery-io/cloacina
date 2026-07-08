---
id: py-block-on-one-gil-safe-async
level: initiative
title: "py_block_on — one GIL-safe async→sync bridge module, retire the per-binding reimplementations"
short_code: "CLOACI-I-0136"
created_at: 2026-07-08T14:20:11.787079+00:00
updated_at: 2026-07-08T14:20:11.787079+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: py-block-on-one-gil-safe-async
---

# py_block_on — one GIL-safe async→sync bridge module, retire the per-binding reimplementations Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

> **Phase: discovery.** Surfaced by the 2026-07-08 architecture deepening review (candidate #2, rated Strong — highest CORRECTNESS leverage). Not yet designed.

**The shallowness.** The bridge's INTERFACE is trivial — drive one async Rust future to completion from a sync PyO3 method, GIL-safely — but its correctness-critical IMPLEMENTATION is copied in **three non-equivalent shapes across 8 files** (`crates/cloacina-python/src/{context,task,trigger,constructor,computation_graph}.rs`, `bindings/{runner,admin,trigger}.rs`):
- **The correct one** — `context.rs::block_on_secret_access`: `py.allow_threads(|| match Handle::try_current() { … block_on … })`. Its doc comment carries the load-bearing knowledge: the GIL is RELEASED before we block (the scenario-30/32/33 PyO3↔tokio deadlock history).
- **Already drifted to UNSAFE** — `bindings/admin.rs`: a fresh `tokio::runtime::Runtime::new()?.block_on(...)` with **NO `py.allow_threads`** — holds the GIL across the blocking await, the exact footgun `context.rs` documents avoiding. This is a **latent deadlock bug shipping today.**
- Two more shapes: `runner.rs` (a dedicated OS-thread actor loop) and `task.rs` (`Handle::current().block_on` + `spawn_blocking(with_gil)`).

The GIL-correctness invariant lives as tribal knowledge in one file's comment, not in code the others call — so a binding already re-implemented it wrong.

**Deletion test — PASS:** deleting admin.rs's & runner.rs's ad-hoc `Runtime::new().block_on` in favour of one `py_block_on(py, fut)` module CONCENTRATES GIL-deadlock correctness in one auditable place — and removes an already-unsafe copy. See [[project_scenario32_cg_invocation_deadlock]] for why this locality is valuable.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- One `py_block_on(py, fut)` module (context.rs's helper, generalized off "secret") that EVERY binding calls to drive an async future from sync PyO3.
- The GIL-safety invariant (release GIL via `allow_threads`, reuse the ambient `Handle`, fall back to a transient runtime) becomes a LOCALITY property of one function — auditable once.
- Fix the admin.rs latent deadlock in passing (route it through the safe door).

**Non-Goals:**
- Restructuring runner.rs's dedicated-thread actor loop unless it genuinely simplifies onto `py_block_on` (its OS-thread ownership may be load-bearing — investigate, don't force).
- Changing any Python-facing API — pure internal correctness/locality deepening.
- Re-opening the async execution model; this is only about the sync→async blocking bridge.

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