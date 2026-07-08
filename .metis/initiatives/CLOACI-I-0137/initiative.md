---
id: cloaca-single-registrar-one
level: initiative
title: "Cloaca single registrar — one register_cloaca() source of truth, make wheel/server symbol drift impossible"
short_code: "CLOACI-I-0137"
created_at: 2026-07-08T14:20:13.910682+00:00
updated_at: 2026-07-08T14:20:13.910682+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: M
initiative_id: cloaca-single-registrar-one
---

# Cloaca single registrar — one register_cloaca() source of truth, make wheel/server symbol drift impossible Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context **[REQUIRED]**

> **Phase: discovery. Maintainer flagged CLAMP IT DOWN NOW** (2026-07-08) — surfaced by the architecture deepening review (candidate #3, Strong). Small, mechanical, and a drift class we want closed before it bites again.

**The shallowness.** The `cloaca` Python module is registered in TWO hand-synced places that must be kept identical by discipline:
- `crates/cloacina-python/src/lib.rs:89–185` — the maturin `#[pymodule]` (what the pip WHEEL exposes).
- `crates/cloacina-python/src/loader.rs:98–203` — a synthetic `ensure_cloaca_module` (`PyModule::new` injected into `sys.modules`, what the embedded SERVER exposes).

Both carry literal `// keep BOTH in sync` comments — an admission the SEAM LEAKS. Evidence of real drift:
- `py_var` / `py_var_or` are registered in `loader.rs` but **absent from `lib.rs`** — so `cloaca.var()` works on the embedded server but would be **missing from the wheel**. **This one is LATENT — it hasn't user-bitten yet (the maintainer's "hasn't hit us yet").**
- `state_accumulator` was once omitted from `ensure_cloaca_module` and DID bite: a production reconciler failure `"module 'cloaca' has no attribute 'state_accumulator'"` ([[CLOACI-T-0688]]), patched with a `hasattr` regression guard bolted onto the drift — a symptom-suppressant, not a cure.

**Deletion test — PASS:** deleting one of the two lists (both delegating to one shared registrar) makes drift STRUCTURALLY IMPOSSIBLE — the symbol set has one definition. Same disease as the version drift [[CLOACI-I-0134]] and the DAL twins [[CLOACI-I-0135]]: hand-synced duplicates always drift; the fix is one source, not more discipline.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- One `register_cloaca(m: &Bound<PyModule>)` function that adds every class/function to the module, called by BOTH the `#[pymodule]` and `ensure_cloaca_module`.
- Wheel and embedded-server `cloaca` expose an identical symbol set BY CONSTRUCTION — you cannot add a symbol to one and forget the other.
- Delete the `// keep BOTH in sync` comments and (once structurally safe) the `hasattr` regression guard.
- Close the live `py_var`/`py_var_or` wheel gap as the first proof the collapse works.

**Non-Goals:**
- Changing WHICH symbols `cloaca` exposes — this is about the registration seam, not the surface.
- The synthetic-module-injection mechanism itself (`ensure_cloaca_module` still injects into `sys.modules`; it just sources its symbols from the shared registrar).

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