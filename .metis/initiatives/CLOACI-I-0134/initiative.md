---
id: single-source-of-version-truth
level: initiative
title: "Single source of version truth — collapse the ~30-way version drift, one-command bump, CI drift guard"
short_code: "CLOACI-I-0134"
created_at: 2026-07-08T11:30:59.278344+00:00
updated_at: 2026-07-08T11:36:14.476107+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: single-source-of-version-truth
---

# Single source of version truth — collapse the ~30-way version drift, one-command bump, CI drift guard

> **Phase: discovery.** Design forks below are OPTIONS pending a maintainer check-in. Nothing committed to implementation yet.

## Context **[REQUIRED]**

Prompted by prepping the 0.10.0 bump (2026-07-08) — and by the fact that **this drift keeps recurring**: the UI shell was still displaying `v0.8.0` (`ui/src/components/Shell.tsx:190`) while the Connect page showed `v0.9.0` (`ui/src/routes/Connect.tsx:382`) and `package.json` said `0.9.0` — three different hand-typed versions in one app. There was already a T-0661 "SDK version lockstep" gate, which shows we've fought this before and only patched a subset.

**The version is hardcoded across ~30 places** with NO single source and NO bump script:
- **Rust:** `Cargo.toml` `[workspace.package] version` + 5 explicit `version = "0.9.0"` pins in `[workspace.dependencies]` (lines 22–26); per-crate path-deps that RE-pin `version = "0.9.0"` in `cloacina`, `-server`, `-compiler`, `-python`, `cloacinactl`, `-workflow-plugin`; and a straggler `cloacina-computation-graph/Cargo.toml` with its OWN `version = "0.9.0"` (not `version.workspace = true`).
- **npm:** `clients/typescript/package.json`, `ui/package.json`, `ui/harness/package.json`.
- **Python:** `clients/python/pyproject.toml` + `clients/python/src/cloacina_client/__init__.py` `__version__`; `crates/cloacina-python/pyproject.toml` is `dynamic` (good).
- **UI display:** two HAND-TYPED literals (`Shell.tsx:190`, `Connect.tsx:382`) that read from nothing.
- **Scaffold:** `crates/cloacinactl/src/nouns/package/new.rs:39` `CLOACINA_CRATE_VERSION = "0.7"` — pins generated packages' `cloacina-workflow` dep three minors behind.
- **Docs + CHANGELOG:** tutorial Cargo.toml snippets + `CHANGELOG.md`.

Release is **tag-triggered** (`.github/workflows/unified_release.yml` on `push: tags:`), so the tag is the only irreversible step; every version edit before it is safe.

The recurrence is the point: a mechanical bump fixes 0.10.0 but not the CLASS. This initiative makes version a single source with a one-command bump and a CI guard that FAILS on drift, then dogfoods it by cutting 0.10.0 through the new path.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- **Collapse the sources:** every place that repeats the version either INHERITS it (Rust `version.workspace`/`workspace = true`) or is REGENERATED from one input — no hand-typed version literals survive (esp. the UI display).
- **One-command bump:** a single `angreal`-fronted command sets the whole repo to a new version (Rust + npm + python + UI + scaffold + docs snippets + CHANGELOG stub).
- **CI drift guard:** a check that FAILS the build if any touchpoint disagrees with the source — so this can never silently recur (supersedes/extends the T-0661 SDK lockstep gate).
- **Dogfood:** perform the 0.10.0 bump through the new mechanism, then the release tag is a clean human step.

**Non-Goals:**
- Automating the release/tag/publish itself (that stays a deliberate human `git tag`); this is about the version SOURCE, not the release pipeline (`unified_release.yml` unchanged).
- Changing the versioning SCHEME (still semver, single workspace-wide version; not per-crate independent versions).
- Re-litigating ADR-decided release mechanics.

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

### Design Decisions (2026-07-08 check-in)
- **D-1 — Rust: maximize native inheritance.** The workspace version is canonical (`[workspace.package] version`). Collapse the 5 `[workspace.dependencies]` explicit `version = "0.9.0"` pins and every per-crate path-dep re-pin to `{ workspace = true }` (version defined ONCE in the dep table). Fix the `cloacina-computation-graph` straggler to `version.workspace = true`. → Rust bumps at 2 points.
- **D-2 — UI: build-time inject (chosen).** Vite `define` injects `__APP_VERSION__` from `ui/package.json`; `Shell.tsx` + `Connect.tsx` read the constant. The two hand-typed literals are DELETED — no UI version literal can drift again. (harness/ likewise if it displays a version.)
- **D-3 — One-command bump (`angreal release bump <version>`).** A single task rewrites the non-inheriting touchpoints from one input: workspace version + workspace dep table (Rust), npm `package.json`s, python `pyproject.toml` + `__init__.py`, and a CHANGELOG `[x.y.z]` stub. Idempotent; the release TAG stays a human `git tag` (non-goal to automate).
- **D-4 — Drift guard: pre-commit hook (chosen).** A `pre-commit` hook asserts every touchpoint == the workspace version and FAILS on mismatch. Rationale (maintainer): the repo's CI/CD runs the pre-commit hooks, so `--no-verify` can't sneak drift past merge — the hook is fired in the pipeline by definition. Supersedes/absorbs the T-0661 SDK-lockstep gate. Runnable locally (`pre-commit run version-lockstep`).
- **D-5 — Scaffold pin (`CLOACINA_CRATE_VERSION = "0.7"`): SPIKE first (chosen).** Before deciding track-vs-pin, investigate whether a package scaffolded with `cloacina-workflow = "0.7"` actually BUILDS against the current compiler-injected deps (I-0125 injects crate-type/features; does the version pin still matter for resolution?). Then either fold it into the bump (track) or document WHY it's deliberately pinned (so the guard whitelists it and it's not re-flagged as drift).
- **D-6 — CORE-ONLY; providers are independent by design (2026-07-08 maintainer check).** Providers version + release on their OWN cadence, decoupled from core ([[CLOACI-A-0010]]: "independently published, independently versioned crate"). This is ALREADY the structure — the first-party providers (`examples/constructor-contract/cloacina-provider-*`) are standalone crates (own `[workspace]`, own `version = "0.1.0"`) EXCLUDED from core's workspace. **NO workspace restructure needed.** So I-0134's single-source + bump + guard cover CORE (the main workspace) ONLY; they MUST NOT touch or lockstep `examples/` provider versions, and the drift guard must explicitly IGNORE providers (a provider at 0.1.0 while core is 0.10.0 is CORRECT, not drift). Deferred follow-ups: [[CLOACI-T-0871]] (are the example providers real first-party → promote out of `examples/`? only `fs` looks real) and [[CLOACI-T-0872]] (independent provider release path).
- **Dogfood:** perform the 0.10.0 bump through the new command + guard, then hand off the tag.

### The version-touchpoint map (the guard's checklist)
Rust: `Cargo.toml` (`[workspace.package] version` = SOURCE; `[workspace.dependencies]` pins) · `crates/cloacina-computation-graph/Cargo.toml` (straggler). npm: `clients/typescript/package.json` · `ui/package.json` · `ui/harness/package.json`. Python: `clients/python/pyproject.toml` · `clients/python/src/cloacina_client/__init__.py` (`__version__`). UI display: `ui/src/components/Shell.tsx` · `ui/src/routes/Connect.tsx` (→ build-time inject, D-2). Scaffold: `crates/cloacinactl/src/nouns/package/new.rs` (`CLOACINA_CRATE_VERSION`, D-5). Docs: tutorial Cargo.toml snippets (bump-managed or guard-checked). `CHANGELOG.md`.

## Alternatives Considered **[REQUIRED]**

- **Root `VERSION` file as the source.** Rejected — the Cargo `[workspace.package] version` is already the most-inherited source and is language-native for the bulk (Rust); a separate VERSION file adds a redundant source. The bump command takes the version as an arg and the guard pins everyone to the Cargo workspace version.
- **Per-crate independent versions.** Out of scope (non-goal) — cloacina ships as one lockstepped surface; independent crate versions would multiply the drift problem, not solve it.
- **Just bump 0.10.0 mechanically.** The status quo — fixes one release, not the recurring class. Rejected as the whole point.
- **A release-orchestration tool (release-plz / changesets).** Heavier than needed and would fight the angreal-everything convention + the human-tag release model; revisit only if the bump command proves insufficient.

## Implementation Plan **[REQUIRED]**

Decompose (below) into: Rust inheritance collapse, UI build-time inject, the `angreal release bump` command, the pre-commit drift guard, the scaffold-pin spike, and the 0.10.0 dogfood. Order: (Rust ∥ UI) → bump command + guard (need final touchpoint shapes) → spike feeds the command → dogfood last.

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