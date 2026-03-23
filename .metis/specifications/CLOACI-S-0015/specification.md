---
id: system-complexity-analysis-simply
level: specification
title: "System Complexity Analysis — Simply Complex, Not Complicated"
short_code: "CLOACI-S-0015"
created_at: 2026-03-23T16:09:37.899149+00:00
updated_at: 2026-03-23T16:09:37.899149+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# System Complexity Analysis — Simply Complex, Not Complicated

*This template provides structured sections for system-level design. Delete sections that don't apply to your specification.*

## Overview **[REQUIRED]**

Analysis of system complexity across the Cloacina codebase (~82k lines since v0.3.0). Finding: **the system is simply complex, not complicated.** Core architecture is well-designed. Accidental complexity is concentrated in a few specific, fixable areas.

## What's Well-Designed (Essential Complexity)

- **Crate layering**: `cloacina-workflow` → `cloacina` → `cloacinactl` → `cloacina-testing`. Dependency arrows correct.
- **Scheduler loop**: 357 lines, clean, linear, outbox-with-fallback pattern.
- **Cron saga pattern**: Audit-before-handoff is exactly right for reliability.
- **Error hierarchy**: Maps cleanly to conceptual boundaries.
- **Builder pattern for config**: Proper builder with `#[non_exhaustive]`.

## Accidental Complexity (Fixable)

### 1. DAL Duplication (~5-6k lines) — HIGH IMPACT
Every DAL operation duplicated as `_postgres`/`_sqlite` variant. Bodies often character-for-character identical. A generic `with_connection` helper would eliminate ~50% of DAL code. Covered by I-0042.

### 2. Stringly-Typed Status Machines — MEDIUM IMPACT
Task/pipeline statuses are raw strings. No compile-time validation. "Abandoned" hacked as `Failed` + error prefix. Fix: status enums with Diesel serialization.

### 3. Configuration Sprawl — MEDIUM IMPACT
`DefaultRunnerConfig` has 27 flat fields duplicating sub-configs. Fix: nested `Option<SubConfig>` where None = disabled.

### 4. Boilerplate Service Management — LOW IMPACT
`services.rs` repeats spawn-with-shutdown pattern 5 times. Fix: `ManagedService` abstraction (~300 lines saved).

### 5. Error Type Overlap — LOW IMPACT
`ValidationError` conflates graph validation with runtime errors. Has 3 duplicate variants. Fix: split or consolidate.

## Estimated Impact
~6,500-7,600 lines removable (7-9% of codebase), DAL dedup accounting for the vast majority.

## System Context **[CONDITIONAL: System-Level Spec]**

{Delete for project-level specifications}

### Actors
- **{Actor 1}**: {Role and interaction pattern}
- **{Actor 2}**: {Role and interaction pattern}

### External Systems
- **{System 1}**: {Integration description}
- **{System 2}**: {Integration description}

### Boundaries
{What is inside vs outside the system scope}

## Requirements **[REQUIRED]**

### Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| REQ-1.1.1 | {Requirement description} | {Why this is needed} |
| REQ-1.1.2 | {Requirement description} | {Why this is needed} |
| REQ-1.2.1 | {Requirement description} | {Why this is needed} |

### Non-Functional Requirements

| ID | Requirement | Rationale |
|----|-------------|-----------|
| NFR-1.1.1 | {Requirement description} | {Why this is needed} |
| NFR-1.1.2 | {Requirement description} | {Why this is needed} |

## Architecture Framing **[CONDITIONAL: System-Level Spec]**

{Delete for project-level specifications}

### Decision Area: {Area Name}
- **Context**: {What needs to be decided}
- **Constraints**: {Hard constraints that bound the decision}
- **Required Capabilities**: {What the solution must support}
- **ADR**: {Link to ADR when decision is made, e.g., PROJ-A-0001}

## Decision Log **[CONDITIONAL: Has ADRs]**

{Delete if no architectural decisions have been made yet}

| ADR | Title | Status | Summary |
|-----|-------|--------|---------|
| {PROJ-A-0001} | {Decision title} | {decided/superseded} | {One-line summary} |

## Constraints **[CONDITIONAL: Has Constraints]**

{Delete if no hard constraints exist}

### Technical Constraints
- {Constraint 1}
- {Constraint 2}

### Organizational Constraints
- {Constraint 1}

### Regulatory Constraints
- {Constraint 1}

## Changelog **[REQUIRED after publication]**

{Track significant changes after initial publication. Delete this section until the specification is published.}

| Date | Change | Rationale |
|------|--------|-----------|
| {YYYY-MM-DD} | {What changed} | {Why it changed} |
