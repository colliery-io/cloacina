---
id: dal-twin-collapse-one-backend
level: initiative
title: "DAL twin collapse — one backend-agnostic interact seam, delete ~168 postgres/sqlite method twins"
short_code: "CLOACI-I-0135"
created_at: 2026-07-08T14:20:10.497021+00:00
updated_at: 2026-07-08T22:28:34.773918+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/design"


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

### Feasibility (grounded 2026-07-08)
The pieces exist: a diesel `#[derive(MultiConnection)] enum AnyConnection { Postgres(PgConnection), Sqlite(SqliteConnection) }` (`database/connection/backend.rs:120`) and a `dispatch_backend!` macro. BUT the DAL grabs CONCRETE pooled connections — `get_postgres_connection() -> Object<PgManager>` (`PgConnection`), `get_sqlite_connection() -> Object<SqliteManager>` (`SqliteConnection`), from two SEPARATE deadpool pools. `AnyPool` is just an enum holding one concrete pool; nothing pools `AnyConnection` (deadpool-diesel managers are per-connection-type). So the twins exist because each `.interact()` closure receives a different concrete `&mut …Connection`.

### Design Decisions (2026-07-08 check-in)
- **D-1 — MACRO seam (compile-time monomorphization), chosen.** An `interact_on_backend!(dal, |conn| { …diesel… })` macro writes the closure body ONCE in source and expands (via `dispatch_backend!`) to the pg + sqlite arms — each monomorphized against its concrete connection type. Deletes the source duplication with NO change to pooling or per-backend setup (schema search_path, sqlite pragmas). The ~4 genuinely-divergent ops stay as explicit twins. Rejected: a pooled-`AnyConnection` runtime seam (re-plumbs pooling, must re-home per-backend setup, MultiConnection query-support edge cases) and a generic `for<C: Connection>` fn (a single generic closure won't compile where diesel needs the concrete backend — RETURNING, on_conflict, trait bounds).
- **KEY INSIGHT that makes D-1 sound:** a `move` closure appearing in BOTH match arms is fine — Rust permits moving the same captured variable in mutually-exclusive `match` arms (only one executes). So `interact_on_backend!` expands under dual-feature to `match dal.backend() { Postgres => { let c = get_postgres_connection…; c.interact(move |conn| { <body> })… }, Sqlite => { …get_sqlite_connection…; c.interact(move |conn| { <body> })… } }` with `<body>` authored once. Under single-feature, only the live arm compiles (cfg).
- **D-2 — SPIKE one module first, then INCREMENTAL, chosen.** Prove the macro (incl. the error-mapping ergonomics) on ONE representative DAL module end-to-end (compiles + tests green both backends), THEN migrate module-by-module — the macro and the remaining twins coexist during the transition.

### Open detail for the spike to settle
Error mapping: each twin does `.interact(...).await.map_err(interact_err)?.map_err(diesel_err)?` where the domain error varies per module (KeyError / SecretError / diesel::result::Error). The macro must either return a raw `Result<R, diesel::result::Error>` (caller maps to its domain error) or take an error-map hook. The one-module spike nails the exact ergonomics before the incremental sweep.

## Alternatives Considered **[REQUIRED]**

- **Pooled `AnyConnection` (custom deadpool Manager over the MultiConnection).** The "truest" deep module — one runtime connection type, a plain generic `interact_on_backend(dal, |conn| …)`. Rejected for the initial cut: re-plumbs the pooling layer, must re-home per-backend connection setup (pg schema search_path, sqlite pragmas), and risks diesel MultiConnection query-support edge cases — high risk for marginal gain over the macro, which already achieves the write-once goal. Revisit only if the macro proves ergonomically painful.
- **Generic `interact<F,R>(f) where F: for<C: Connection> FnOnce(&mut C)`.** No pooling change, but a single generic closure often won't compile where diesel needs the concrete backend (RETURNING, `on_conflict`, backend-specific trait bounds) — would fail for a chunk of the ~330. Rejected (the macro monomorphizes per backend, sidestepping this).
- **Leave the twins.** The status quo — a 27k-line subtree half-duplicated, and the vestigial reason (no dual-DB abstraction) is long gone.

## Implementation Plan **[REQUIRED]**

Spike → incremental. (1) The `interact_on_backend!` macro + migrate ONE representative module (settle error-mapping) — the de-risking gate. (2) Incremental migration by DAL area (dal/unified core CRUD, its sub-modules, security/), each behavior-preserving + tests green both backends. (3) An audit pass confirming the ~4 genuinely-divergent ops remain explicit twins (and are the ONLY twins left). Decompose into tasks AFTER the spike validates the macro shape.

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