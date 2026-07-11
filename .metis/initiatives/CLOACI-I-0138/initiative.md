---
id: packaged-first-examples-server
level: initiative
title: "Packaged-first examples — server/daemon gold path as the standard for all examples"
short_code: "CLOACI-I-0138"
created_at: 2026-07-10T00:22:40.417461+00:00
updated_at: 2026-07-10T01:16:19.910293+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


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
- **D-3 (maintainer, 2026-07-10): ALL examples take the primary interface; the built-in scheduler is not the demo/testing vehicle.** The point is to *demonstrate every feature through the primary interface* — server/daemon via pack → upload → compile → reconcile → execute (or monitor). Examples must not show the in-process `DefaultRunner` as the way to run/test. This sharpens D-1: incremental is still the pace, but the end-state is unambiguous — no embedded-runner demos left, no "alternative embedded path" sections in READMEs. Corollary: migrating each feature example through the server path will surface any server-path feature gaps (cron via schedules, event triggers via the trigger API, multi-tenancy via the tenants API, …) — surfacing those loudly is part of the value, per the I-0137 lesson.

## Migration inventory (grounded 2026-07-10) — ≈26 units still demoing via `DefaultRunner`
- **Rust feature examples (10):** conditional-retries, cron-scheduling, deferred-tasks, event-triggers, multi-tenant, per-tenant-credentials, registry-execution, python-workflow, computation-graphs/filtered-reactor, constructor-contract/fs-grant-demo
- **Performance (3):** simple, parallel, pipeline (may stay embedded by design — they benchmark the engine, not the interface; decide at design time)
- **Rust tutorial library (6):** 01-basic-workflow … 06-multi-tenancy
- **Python tutorials (8):** 01_first_workflow … 08_packaged_triggers

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

### The gold-path shape (grounded 2026-07-09)
`simple-packaged` is the AUTHORING half (package.toml + workflow src → `.cloacina`). `registry-execution` runs a package but via an EMBEDDED `DefaultRunner` (SQLite in-mem + FilesystemRegistryStorage) — still embedded, NOT the server. Packaged-FIRST adds the missing half: **register the `.cloacina` with a running SERVER/daemon and run it there** (align with the existing `docs/content/service/tutorials/03-packaged-workflows.md`). cloacinactl exposes the nouns: `package`, `workflow`, `server`, `daemon`, `tenant`, `trigger`, `execution`.

### Canonical packaged example (THE template)
Each example dir:
- `package.toml` — manifest (name, version, `interface = cloacina-workflow-plugin`, `[metadata] workflow_name/language/description`).
- Workflow source — Rust (`Cargo.toml` minimal shell + `src/lib.rs` with `#[workflow]`/`#[task]`, per I-0125) OR Python (`workflow.py` + `package.toml`).
- `README.md` — the **gold-path run recipe**: (1) build the `.cloacina`, (2) bring up the server via the docker compose demo stack ([[feedback_use_container_stack]]), (3) register the package with the server (cloacinactl), (4) trigger it, (5) observe via API/UI. NO in-process `DefaultRunner`.

### Standard artifact
A short "Writing a Packaged Example" convention (files above + the README recipe skeleton, both languages) that NEW examples follow — the thing that makes packaged-first the default.

### Decomposition (proposed — for sign-off)
- **T-a** Canonical RUST packaged example (reference template) — author + package.toml + gold-path README against the demo stack.
- **T-b** Canonical PYTHON packaged example (reference template).
- **T-c** "Packaged example standard" doc/convention new examples follow (+ CONTRIBUTING pointer).
- **T-d+ (backlog, incremental per D-1)** convert each still-embedded feature example (cron-scheduling, event-triggers, multi-tenant, per-tenant-credentials, complex-dag, conditional-retries, deferred-tasks, python-workflow) — one task each, later.

### Open for sign-off
1. Run recipe targets the **docker compose demo stack** as the canonical "server" for examples (vs a bare `cloacinactl daemon`/`server start`)?
2. T-a/T-b build NEW reference examples, or promote `simple-packaged`/`packaged-workflows` in place as the canonical ones?

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
