---
id: documentation-validation-and-c4
level: initiative
title: "Documentation Validation and C4 Architecture Documentation"
short_code: "CLOACI-I-0028"
created_at: 2026-03-13T14:15:49.859772+00:00
updated_at: 2026-03-13T14:29:43.260846+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: documentation-validation-and-c4
---

# Documentation Validation and C4 Architecture Documentation Initiative

## Context

Cloacina has 50+ documentation pages across tutorials, explanations, API references, and how-to guides — covering both the Rust core and Python bindings. Documentation has been written incrementally as features were built, often by different authors at different times. No systematic validation has been performed to verify that documentation accurately reflects the current implementation.

Additionally, while Cloacina has significant architectural complexity — 5 crates, 2 language runtimes, multiple execution modes, packaging/security/multi-tenancy systems — there is no formal architecture documentation using an industry-standard model. The IcePanel C4 setup covers the future Continuous Reactive Scheduling design but does not document the **existing, shipped** system architecture.

This initiative performs an exhaustive, adversarial validation of every documentation page against the actual codebase, and produces a complete C4 architecture model (all four levels) for the system as it exists today.

### Current State

**Documentation inventory (50+ files):**
- 10 Rust tutorials (01–10)
- 9 Python tutorials (01–09)
- 15 explanation docs (architecture, systems, internals)
- 11 Python API reference docs
- 5 how-to guides (Rust + Python)
- Quick start guides, contributing guides, reference

**Architecture documentation gaps:**
- No C4 System Context diagram for the shipped system
- No C4 Container diagram showing crate boundaries and runtime components
- No C4 Component diagrams for core modules (executor, scheduler, DAL, registry, security)
- No C4 Code-level diagrams for key abstractions (Task trait hierarchy, Context flow, DAL layer)
- IcePanel C4 model exists only for the future Continuous Reactive Scheduling design
- Existing mermaid diagrams are ad-hoc and inconsistent across docs

**Known risk areas:**
- Package format doc described V1 manifest only (V2/Python was just added in CLOACI-T-0078)
- CLI command examples may reference old `cloacina-cli` naming (renamed to `cloacinactl` in CLOACI-T-0059)
- API references may lag behind implementation (new methods, changed signatures)
- Tutorials may reference deprecated APIs or missing dependencies
- Cross-references between docs may point to nonexistent pages

## Goals & Non-Goals

**Goals:**
- Validate every documentation page against the current implementation — every code example, API signature, CLI command, and architectural claim must be verified
- Produce C4 architecture documentation at all four levels (System Context, Container, Component, Code) for the shipped system
- Ensure every tutorial's code examples compile and run correctly
- Ensure every API reference matches the actual public API surface
- Ensure every explanation doc accurately describes the current implementation, not aspirational or outdated behavior
- Fix or rewrite any documentation that fails validation
- Establish architecture diagrams as the canonical system reference, hosted in the documentation site

**Non-Goals:**
- Documenting the future Continuous Reactive Scheduling system (already covered by IcePanel C4 + specifications CLOACI-S-0001 through S-0008)
- Writing new tutorials for features not yet documented (unless discovered during validation as critical gaps)
- Changing the documentation framework (Hugo) or site structure
- Translating documentation to other languages

## Detailed Design

### Validation Methodology

Each documentation page is subjected to a multi-pass validation process designed to maximize scrutiny:

**Pass 1 — Structural Audit**
- Verify frontmatter (title, description, weight, reviewer, review_date)
- Verify all internal cross-references (`{{< ref >}}` shortcodes) resolve to existing pages
- Verify all external links are reachable
- Verify Hugo shortcodes are valid (`{{< hint >}}`, not `{{< tip >}}`, etc.)
- Verify `angreal docs build` produces no warnings or errors

**Pass 2 — Code Example Validation**
- Extract every code block from every documentation page
- For Rust examples: verify they compile against current workspace dependencies
- For Python examples: verify they execute with current cloaca bindings
- For CLI examples: verify command syntax against current `cloacinactl` and `cloaca` CLIs
- For config/manifest examples: verify they parse with current schema validators
- Flag any example that uses deprecated, renamed, or nonexistent APIs

**Pass 3 — API Surface Audit**
- For each API reference doc: diff the documented API against the actual public API
- Rust: compare documented types/methods against `cargo doc` output
- Python: compare documented classes/methods against runtime introspection (`dir()`, `__doc__`, type hints)
- Flag missing methods, wrong signatures, undocumented parameters, stale descriptions
- Verify callback signatures, error types, return types

**Pass 4 — Semantic Accuracy Audit**
- For each explanation doc: read the corresponding source code and verify every claim
- Verify architectural diagrams match actual code structure
- Verify described algorithms match implementations (e.g., "uses Tarjan's algorithm" — does it?)
- Verify described data flows match actual execution paths
- Verify described configuration options exist and behave as documented
- Cross-reference ADRs (CLOACI-A-0001, A-0002) against implementation to ensure decisions were followed

**Pass 5 — Tutorial End-to-End Execution**
- Each tutorial is executed end-to-end in a clean environment
- Verify all `Cargo.toml` dependencies are correct and complete
- Verify step-by-step instructions produce the described output
- Verify screenshots/output examples match current behavior
- Test on both SQLite and PostgreSQL backends where applicable

### C4 Architecture Documentation

Architecture documentation follows Simon Brown's C4 model strictly. Each level is a separate documentation page with mermaid diagrams and prose descriptions.

**Level 1 — System Context**
Scope: Cloacina and its external relationships.

Actors and external systems:
- **Workflow Developer** — defines tasks, workflows, and scheduling rules
- **Operator** — manages deployment, keys, packages via `cloacinactl`
- **Host Application** — Rust application embedding Cloacina as a library
- **Python Application** — Python application using Cloaca bindings
- **PostgreSQL** — primary production database backend
- **SQLite** — lightweight/embedded database backend
- **External Data Sources** — APIs, files, queues polled by tasks
- **Package Storage** — filesystem or object store for `.cloacina` archives

**Level 2 — Container**
Scope: The major deployable/runnable units and how they communicate.

Containers (mapped to crates and binaries):
- **cloacina** (library) — core orchestration engine, embedded in host applications
- **cloacina-workflow** (library) — minimal types for authoring packaged workflows
- **cloacina-macros** (proc-macro) — compile-time task/workflow registration and validation
- **cloacinactl** (binary) — operator control tool (key management, package signing, admin)
- **cloaca** (Python package) — PyO3 bindings exposing Cloacina to Python
- **Database** (PostgreSQL/SQLite) — persistent state for executions, schedules, packages, keys

Show dependency arrows: which crate depends on which, which runtime components talk to the database, how the Python bridge works.

**Level 3 — Component**
Scope: Major components within the `cloacina` core library. One diagram per major subsystem:

*Execution Subsystem:*
- DefaultRunner — top-level orchestrator
- TaskScheduler — dependency resolution, state management, scheduling loop
- PipelineExecutor — pipeline lifecycle management
- ThreadTaskExecutor — concurrent task execution with semaphore-based slot management
- TaskHandle / SlotToken — deferred execution and concurrency slot management
- Dispatcher — pluggable execution routing (local, K8s, Lambda, custom)

*Data Access Layer:*
- DAL facade — unified interface over all repository types
- ContextRepository — context persistence and retrieval
- TaskExecutionRepository — task state, claiming, sub-status, recovery
- PipelineExecutionRepository — pipeline state management
- CronExecutionRepository / CronScheduleRepository — cron scheduling state
- TriggerExecutionRepository / TriggerScheduleRepository — trigger state
- ExecutionEventRepository — outbox-based event logging
- TaskOutboxRepository — guaranteed delivery queue
- WorkflowRegistryRepository / WorkflowPackagesRepository — package management
- SigningKeyRepository / PackageSignatureRepository — security state
- Database backends: PostgreSQL (schema-based multi-tenancy), SQLite (file-based)

*Registry & Packaging Subsystem:*
- WorkflowRegistryImpl — package lifecycle management
- PackageLoader — metadata extraction from .cloacina archives
- PackageValidator — security, format, size, symbol validation
- RegistryReconciler — background package change monitoring
- TaskRegistrar — runtime task registration in global registry
- PythonLoader — Python package extraction and PyO3 task import
- ManifestV2 — unified manifest supporting Rust and Python packages

*Security Subsystem:*
- DbPackageSigner — Ed25519 signing with database-backed keys
- DbKeyManager — key generation, rotation, export, trust management
- PackageVerifier — online (DB) and offline (PEM) signature verification
- KeyEncryption — AES-256-GCM encryption of private keys with master key
- AuditLogger — security event logging

*Scheduling Subsystem:*
- CronScheduler — time-based workflow triggers with timezone support
- CronEvaluator — cron expression parsing and next-fire calculation
- CronRecovery — missed execution detection and catchup
- TriggerScheduler — event-based workflow triggers with condition evaluation
- TriggerRules — composable condition evaluation (All, Any, None, TaskSuccess, ContextValue)

*Macro Subsystem:*
- `#[task]` proc macro — task struct generation, handle detection, fingerprinting, registry integration
- `workflow!` macro — workflow struct generation, graph validation, version calculation
- Compile-time registry — duplicate detection, dependency graph, cycle detection

**Level 4 — Code**
Scope: Key abstractions and trait hierarchies. Not every file — focus on the types that define the system's contracts:

- `Task` trait — `execute()`, `id()`, `dependencies()`, `retry_policy()`, `trigger_rules()`, `requires_handle()`, `code_fingerprint()`
- `Context<T>` — generic data container with `get()`, `set()`, `insert()`, `merge()`
- `TaskError` enum — `ExecutionFailed`, `ContextError`, `Timeout`, etc.
- `RetryPolicy` / `BackoffStrategy` — retry configuration
- `TaskNamespace` — hierarchical `tenant.package.workflow.task_id` addressing
- `DAL` facade — how it composes all repositories
- `RegistryStorage` trait — filesystem vs database-backed storage
- `TaskExecutorTrait` — pluggable executor interface
- `DispatcherTrait` — pluggable dispatch interface

### Validation Tracking

Each documentation page gets a validation record tracking:
- Page path
- Pass 1–5 status (pass/fail/not-applicable)
- Issues found (with severity: critical/major/minor)
- Fix status (open/fixed/wontfix)
- Validator (who performed the check)

### Deliverables

1. **Validation Report** — comprehensive audit results for all 50+ doc pages
2. **Fixed Documentation** — all issues found during validation are corrected in-place
3. **C4 System Context** — `docs/content/explanation/architecture/c4-system-context.md`
4. **C4 Container** — `docs/content/explanation/architecture/c4-container.md`
5. **C4 Component** — `docs/content/explanation/architecture/c4-components.md` (with sub-diagrams per subsystem)
6. **C4 Code** — `docs/content/explanation/architecture/c4-code-contracts.md`
8. **Updated docs build** — `angreal docs build` passes with zero warnings/errors after all changes

## Alternatives Considered

**Alternative 1: Spot-check validation (random sampling)**
Rejected. Sampling misses systematic issues (e.g., all tutorials referencing a renamed CLI). Exhaustive validation is the only way to achieve the stated goal of correctness. Resources are not a constraint.

**Alternative 2: Automated-only validation (link checking, compilation)**
Rejected as insufficient alone. Automated checks catch broken links and non-compiling code, but cannot verify semantic accuracy ("does this explanation actually describe what the code does?"). The automated checks are included as Pass 1–2, but Passes 3–5 require human/AI semantic review.

**Alternative 3: Architecture documentation using ad-hoc diagrams**
Rejected. C4 provides a standardized, hierarchical framework that prevents the common failure modes of ad-hoc architecture docs (inconsistent abstraction levels, missing context, diagrams that don't compose). C4's four-level zoom is specifically designed for the kind of multi-layer system Cloacina is.

**Alternative 4: Document architecture in IcePanel only (no in-repo docs)**
Rejected. IcePanel is excellent for interactive exploration but creates a dependency on an external service. The canonical architecture documentation must live in the repository alongside the code, rendered by `angreal docs build`. IcePanel can be a supplementary view kept in sync via the existing script-based approach.

## Implementation Plan

### Phase 1: Structural Audit & Link Validation (Pass 1)
- Automated scan of all doc pages for broken refs, invalid shortcodes, missing frontmatter
- `angreal docs build` with strict mode
- Produce initial validation tracking spreadsheet
- Fix all structural issues

### Phase 2: C4 Architecture Documentation
- Author C4 Level 1 (System Context) with mermaid diagrams
- Author C4 Level 2 (Container) mapping to crate boundaries
- Author C4 Level 3 (Component) for each major subsystem (6 diagrams minimum)
- Author C4 Level 4 (Code) for key trait hierarchies and contracts
- Create `docs/content/explanation/architecture/` section with navigation
- Update IcePanel model to include shipped system alongside CRS design

### Phase 3: Code Example Validation (Pass 2)
- Extract and compile all Rust code examples against current workspace
- Execute all Python code examples with current cloaca bindings
- Verify all CLI command examples against cloacinactl and cloaca
- Verify all config/manifest examples parse correctly
- Fix all broken examples

### Phase 4: API Surface Audit (Pass 3)
- Diff all Python API reference docs against runtime introspection
- Diff all Rust documented APIs against cargo doc output
- Identify missing, wrong, or stale API documentation
- Update all API reference pages

### Phase 5: Semantic Accuracy Audit (Pass 4)
- Read each explanation doc alongside its corresponding source code
- Verify every architectural claim, algorithm description, data flow diagram
- Cross-reference ADRs against implementation
- Rewrite any explanation that has drifted from implementation

### Phase 6: Tutorial End-to-End Execution (Pass 5)
- Execute every Rust tutorial (01–10) in clean environments
- Execute every Python tutorial (01–09) in clean environments
- Verify against both SQLite and PostgreSQL where applicable
- Fix any tutorial that does not produce the described results

### Phase 7: Final Integration & Report
- Run `angreal docs build` final verification
- Produce validation report with all findings and fixes
- Cross-reference: every doc page links to relevant C4 diagrams
- Cross-reference: C4 diagrams link to relevant detailed docs

## Task Decomposition

**25 tasks created across 7 phases:**

### Phase 1: Structural Audit (2 tasks)
- **CLOACI-T-0086** — Frontmatter, Shortcodes & Build Validation
- **CLOACI-T-0087** — Cross-Reference & External Link Validation

### Phase 2: C4 Architecture Documentation (10 tasks)
- **CLOACI-T-0088** — C4 Level 1: System Context Diagram
- **CLOACI-T-0089** — C4 Level 2: Container Diagram
- **CLOACI-T-0090** — C4 Level 3: Execution Subsystem Components
- **CLOACI-T-0091** — C4 Level 3: Data Access Layer Components
- **CLOACI-T-0092** — C4 Level 3: Registry & Packaging Components
- **CLOACI-T-0093** — C4 Level 3: Security Subsystem Components
- **CLOACI-T-0094** — C4 Level 3: Scheduling Subsystem Components
- **CLOACI-T-0095** — C4 Level 3: Macro Subsystem Components
- **CLOACI-T-0096** — C4 Level 4: Code Contracts & Trait Hierarchies

### Phase 3: Code Example Validation (4 tasks)
- **CLOACI-T-0098** — Rust Tutorials (01–10)
- **CLOACI-T-0099** — Python Tutorials (01–09)
- **CLOACI-T-0100** — Explanation & How-To Guide Code Blocks
- **CLOACI-T-0101** — CLI Commands & Config/Manifest Examples

### Phase 4: API Surface Audit (2 tasks)
- **CLOACI-T-0102** — Python API References vs Runtime Introspection
- **CLOACI-T-0103** — Rust Public API vs cargo doc Output

### Phase 5: Semantic Accuracy Audit (4 tasks)
- **CLOACI-T-0104** — Execution & Scheduling Explanation Docs
- **CLOACI-T-0105** — Packaging, Security & Registry Explanation Docs
- **CLOACI-T-0106** — Macro System, Versioning & Multi-Tenancy Docs
- **CLOACI-T-0107** — ADR Cross-Reference Against Implementation

### Phase 6: Tutorial E2E Execution (2 tasks)
- **CLOACI-T-0108** — Rust Tutorials (01–10) on SQLite & PostgreSQL
- **CLOACI-T-0109** — Python Tutorials (01–09) on SQLite & PostgreSQL

### Phase 7: Final Integration (1 task)
- **CLOACI-T-0110** — C4 Cross-References, Build Verification & Validation Report
