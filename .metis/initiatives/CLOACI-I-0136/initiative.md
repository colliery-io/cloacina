---
id: py-block-on-one-gil-safe-async
level: initiative
title: "py_block_on — one GIL-safe async→sync bridge module, retire the per-binding reimplementations"
short_code: "CLOACI-I-0136"
created_at: 2026-07-08T14:20:11.787079+00:00
updated_at: 2026-07-09T00:48:32.335535+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/design"


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

### Grounded site inventory (2026-07-09) — the ACTUAL `.block_on` bridge sites
Grepping `.block_on(`/`Runtime::new` in cloacina-python narrows the "3 shapes / 8 files" to the real async→sync-blocking bridges:
- **`context.rs:253 block_on_secret_access`** — CORRECT reference: `py.allow_threads(|| match Handle::try_current() { Ok(h)=>h.block_on(fut), Err(_)=>transient current-thread rt })`.
- **`admin.rs:168,187` (create_tenant/remove_tenant)** — UNSAFE: `Runtime::new()?.block_on(fut)` with NO `allow_threads` → holds the GIL across the await. THE BUG.
- **`task.rs:50 defer_until`** — already safe (`py.allow_threads(|| Handle::current().block_on(...))`), just hand-inlined.
- **`runner.rs:812`** — `rt.block_on(run_event_loop(runner, rx))`: a LONG-RUNNING actor loop on a DEDICATED std::thread (spawned without the GIL, ~line 790). Not a one-shot future.
- **`runner.rs:890/910/955`** — `rt.block_on(builder.build())` inside `spawn_runtime(|rt| …)` (the runner's dedicated-thread helper). Not GIL-holding sync-method bridges.
(trigger/constructor/computation_graph have NO `.block_on` — they use `spawn_blocking` + `with_gil`, a different, legitimate pattern; out of scope.)

### Decision (2026-07-09) — SCOPE
- **Extract `py_block_on(py, fut)` into a new shared module `crates/cloacina-python/src/gil.rs`** — context.rs's helper generalized (renamed off "secret"), `pub(crate)`. Signature: `fn py_block_on<F,T>(py: Python<'_>, fut: F) -> T where F: Future<Output=T> + Send, T: Send`. Body identical to the proven `block_on_secret_access`: `allow_threads` → `Handle::try_current()` (ambient) else transient current-thread runtime.
- **Route through it:** `context.rs` (the 2 secret sites → call the shared helper, delete the local copy), `admin.rs` (the 2 UNSAFE sites — the bug fix; add the PyO3-injected `py: Python` param, invisible to Python callers), `task.rs:50` (for consistency — already-safe, becomes shared).
- **LEAVE `runner.rs` (all sites).** The `run_event_loop` actor loop is long-running on its own OS thread and does not hold the GIL — `py_block_on` (one-shot future→value from a GIL-holding sync method) does NOT fit. The `spawn_runtime` builder sites are that same dedicated-thread architecture. This is the initiative's stated non-goal ("don't force runner.rs"). Documented, not forced.

**Net:** one auditable GIL-safety helper; the admin.rs latent deadlock fixed by construction; runner's load-bearing thread architecture untouched.

## Alternatives Considered **[REQUIRED]**

- **Also fold runner.rs onto `py_block_on`.** Rejected: `run_event_loop` is a long-running loop (not a one-shot future) on a dedicated OS thread that intentionally does NOT hold the GIL; `py_block_on`'s `allow_threads`-then-block contract is meaningless there (no GIL to release) and would misrepresent the pattern. Forcing it risks the exact runtime-ownership subtleties [[project_scenario32_cg_invocation_deadlock]] warns about.
- **A macro instead of a fn.** Rejected: a plain generic fn `py_block_on<F,T>` is sufficient (no per-site token capture needed beyond `py`+`fut`); a macro adds noise for no gain.
- **Leave admin.rs as-is (it uses a fresh runtime, lower deadlock odds).** Rejected: it still holds the GIL across blocking I/O (stalls every other Python thread) and is the unsafe SHAPE — the whole point is one safe door so drift can't reintroduce it.

## Implementation Plan **[REQUIRED]**

Single task (T-0881): add `gil.rs::py_block_on`, wire context.rs + admin.rs + task.rs, leave runner.rs (with a one-line note at each runner site pointing to why it's exempt). Verify: `cargo check` cloacina-python; the Python test suite incl. the scenario_30/32/33 GIL area (`angreal` python lane) — the admin.rs fix must not regress and the secret/defer paths stay green.

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