---
id: independent-provider-release-path
level: task
title: "Independent provider release path — publish/tag first-party providers on their own cadence (not core's v* tag)"
short_code: "CLOACI-T-0872"
created_at: 2026-07-08T11:43:21.080493+00:00
updated_at: 2026-07-08T11:43:21.080493+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Independent provider release path — publish/tag first-party providers on their own cadence (not core's v* tag)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Surfaced during I-0134 (2026-07-08). Providers are STRUCTURALLY independent (standalone crates, own semver, [[CLOACI-A-0010]]) but there is NO operational release path for them: `.github/workflows/unified_release.yml` triggers on CORE `v*` tags only, and the first-party providers aren't published anywhere. So "independent release schedule" is not yet real — only the structure exists.

Design + build a release path for first-party providers that lets them publish/tag on their OWN cadence, decoupled from core's `v*` tag (e.g. per-provider tags like `provider-fs-vX.Y.Z` → publish that crate to crates.io). Keep it separate from the core release pipeline (I-0134 non-goal: don't entangle core's release with providers). Blocked-ish on [[CLOACI-T-0871]] (which providers are real + where they live). Do NOT re-litigate A-0010.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [ ] Bug - Production issue that needs fixing
- [ ] Feature - New functionality or enhancement
- [ ] Tech Debt - Code improvement or refactoring
- [ ] Chore - Maintenance or setup work

### Priority
- [ ] P0 - Critical (blocks users/revenue)
- [ ] P1 - High (important for user experience)
- [ ] P2 - Medium (nice to have)
- [ ] P3 - Low (when time permits)

### Impact Assessment **[CONDITIONAL: Bug]**
- **Affected Users**: {Number/percentage of users affected}
- **Reproduction Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected vs Actual**: {What should happen vs what happens}

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: {Why users need this}
- **Business Value**: {Impact on metrics/revenue}
- **Effort Estimate**: {Rough size - S/M/L/XL}

### Technical Debt Impact **[CONDITIONAL: Tech Debt]**
- **Current Problems**: {What's difficult/slow/buggy now}
- **Benefits of Fixing**: {What improves after refactoring}
- **Risk Assessment**: {Risks of not addressing this}

## Acceptance Criteria **[REQUIRED]**

- [ ] {Specific, testable requirement 1}
- [ ] {Specific, testable requirement 2}
- [ ] {Specific, testable requirement 3}

## Test Cases **[CONDITIONAL: Testing Task]**

{Delete unless this is a testing task}

### Test Case 1: {Test Case Name}
- **Test ID**: TC-001
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
  3. {Step 3}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

### Test Case 2: {Test Case Name}
- **Test ID**: TC-002
- **Preconditions**: {What must be true before testing}
- **Steps**:
  1. {Step 1}
  2. {Step 2}
- **Expected Results**: {What should happen}
- **Actual Results**: {To be filled during execution}
- **Status**: {Pass/Fail/Blocked}

## Documentation Sections **[CONDITIONAL: Documentation Task]**

{Delete unless this is a documentation task}

### User Guide Content
- **Feature Description**: {What this feature does and why it's useful}
- **Prerequisites**: {What users need before using this feature}
- **Step-by-Step Instructions**:
  1. {Step 1 with screenshots/examples}
  2. {Step 2 with screenshots/examples}
  3. {Step 3 with screenshots/examples}

### Troubleshooting Guide
- **Common Issue 1**: {Problem description and solution}
- **Common Issue 2**: {Problem description and solution}
- **Error Messages**: {List of error messages and what they mean}

### API Documentation **[CONDITIONAL: API Documentation]**
- **Endpoint**: {API endpoint description}
- **Parameters**: {Required and optional parameters}
- **Example Request**: {Code example}
- **Example Response**: {Expected response format}

## Implementation Notes **[CONDITIONAL: Technical Task]**

{Keep for technical tasks, delete for non-technical. Technical details, approach, or important considerations}

### Technical Approach
{How this will be implemented}

### Dependencies
{Other tasks or systems this depends on}

### Risk Considerations
{Technical risks and mitigation strategies}

## Status Updates **[REQUIRED]**

### 2026-07-28 — Validated model + urgency raised during 0.10.0 release prep

The consumption chain was validated in code end-to-end (user prompt), confirming this task's premise exactly:
- A consumer depends on the provider **as a plain Cargo dependency** — `provider-consumer-fixture/Cargo.toml`: "The provider, referenced exactly as a consumer would (A-0010: `from` = this name)".
- The **workflow package is what gets compiled**; the compiler resolves its deps from crates.io in production (`cloacina-compiler/src/build.rs:696-698`: "real packages resolve from crates.io" — path-dep injection is a dev/test-only shim).
- For NATIVE providers (I-0139), the compiler's provider bundler (`pack_providers`/`bundle_providers`, T-0907) builds the host cdylib while compiling the consuming workflow — so **crates.io publication of the provider crate is the complete consumer story**; `cloacinactl constructor package --native` is the standalone/manual packaging path.

**Consequence:** until this task lands, NO server-compiled workflow can use `cloacina-provider-kafka` (or any first-party provider) — every provider still carries `publish = false` "until the providers publish home (T-0871/0872) exists" per its own Cargo.toml. With kafka (I-0139, completed) now the flagship REAL provider, T-0871's audit question has a partial answer (kafka + likely fs are real; sensor/quorum/extract likely illustrative) and this task's priority rises accordingly.

(A duplicate task CLOACI-T-0909 was created 2026-07-28 in error and archived; nothing from it supersedes this task's design direction: per-provider tags `provider-<name>-vX.Y.Z` → publish that crate to crates.io, separate workflow from core's release train.)
