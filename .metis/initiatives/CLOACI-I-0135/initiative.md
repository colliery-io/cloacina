---
id: dal-twin-collapse-one-backend
level: initiative
title: "DAL twin collapse — one backend-agnostic interact seam, delete ~168 postgres/sqlite method twins"
short_code: "CLOACI-I-0135"
created_at: 2026-07-08T14:20:10.497021+00:00
updated_at: 2026-07-08T14:20:10.497021+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: dal-twin-collapse-one-backend
---

# DAL twin collapse — one backend-agnostic interact seam, delete ~168 postgres/sqlite method twins Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

> **Phase: discovery.** Surfaced by the 2026-07-08 architecture deepening review (candidate #1, rated Strong — the largest leverage). Not yet designed.

**The shallowness.** ~**330 twin methods** — 167 `async fn *_postgres` + 168 `*_sqlite` — across `crates/cloacina/src/dal/unified/**` and `crates/cloacina/src/security/**` (a 27k-line subtree). The twins are MECHANICAL, not backend-specific: the 108-line pair `schedule/crud.rs::upsert_trigger_{postgres,sqlite}` differs by exactly **2 lines** (the connection accessor `.get_{postgres,sqlite}_connection()` + the `#[cfg]`); the whole diesel `interact` closure is byte-identical. `db_key_manager.rs` twins differ by ~3 lines. The backend choice LEAKS into all ~330 method bodies because each re-selects the connection by hand and re-writes the whole closure.

**Why it exists (maintainer, load-bearing history):** this is a **vestigial trait from before the dual-DB abstraction existed.** When there was no MultiConnection/backend-agnostic crate to lean on, hand-writing the postgres/sqlite twin was the lesser of two evils. Now that the dual-DB seam exists (`database/connection` — both `get_{pg,sqlite}_connection()` return a deadpool `Object` exposing the same `.interact(closure)` API; `dispatch_backend!` already centralizes the cfg branch), the duplication is purely collapsible. **Record this so no one re-adds twins thinking they're required.**

**The genuinely-divergent minority (stays explicit):** `task_execution/claiming.rs` (Postgres `FOR UPDATE SKIP LOCKED` vs SQLite `IMMEDIATE`), `schedule/crud.rs:713` (`SET TRANSACTION ISOLATION LEVEL SERIALIZABLE`), and the SQLite-lacks-`RETURNING` spots (`execution_event.rs`, `task_execution_metadata.rs`) — roughly **4 operations** where the twin actually earns its keep.

**Deletion test — PASS:** deleting the sqlite twin bodies (routing through one backend-agnostic `interact_on_backend` seam) CONCENTRATES complexity at the connection seam where backend choice belongs; it does not move it. Compatible with [[CLOACI-A-0001]] (Diesel MultiConnection) — it deepens that decision. Same disease as the version drift [[CLOACI-I-0134]] fixes: hand-copied duplicates always cost more than the seam.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- One `interact_on_backend(dal, |conn| …diesel…)` adapter over the two connection accessors + `dispatch_backend!`, so a DAL method's query is written ONCE.
- Migrate the ~330 twins to the single seam; the ~4 genuinely-divergent operations remain explicit twins (and become VISIBLE as the only things written twice).
- Net: ~168 twin methods deletable; each query becomes testable once (the interface is the test surface).

**Non-Goals:**
- Re-litigating the backend-selection decision ([[CLOACI-A-0001]] MultiConnection stays).
- Collapsing the genuinely backend-divergent SQL (SKIP LOCKED, RETURNING gaps, isolation) — those stay explicit by design.
- Changing DAL public interfaces or query semantics — pure internal deepening, behavior-preserving.

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