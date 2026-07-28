---
id: provider-publication-path-first
level: task
title: "Provider publication path — first-party providers need a distribution channel"
short_code: "CLOACI-T-0909"
created_at: 2026-07-28T22:16:46.888161+00:00
updated_at: 2026-07-28T22:16:46.888161+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
initiative_id: NULL
---

# Provider publication path — first-party providers need a distribution channel

## Objective **[REQUIRED]**

Give first-party providers (flagship: `cloacina-provider-kafka`, CLOACI-I-0139) a real publication channel. Today there is NONE: the provider lives in `examples/constructor-contract/`, is not a workspace crate, is not in the release train's crates.io tiers, and no job builds or attaches its native per-arch `.cloacina` provider packages anywhere. A user who wants the Kafka provider must clone the repo and build it by hand.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [ ] P1 - High (important for user experience)

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: the provider authoring story (I-0132/I-0139) is only real if consumers can OBTAIN providers without building from source.
- **Business Value**: providers are the extensibility flagship; an install path is table stakes.
- **Effort Estimate**: M

## Design notes (worked 2026-07-28, pre-decision)

**What "publication" must produce:** versioned, signed, per-architecture `.cloacina` provider packages (`provider.json` runtime=native) — the artifact `cloacinactl constructor package --native` emits and a cloacina deployment ingests. Consumers upload/register them; the loader picks the artifact matching the host arch (per-arch rows coexist per (name, version, arch) — proven in the I-0139 two-arch validation).

**Constraint (ADR A-0010):** providers version INDEPENDENTLY of core — deliberately outside the core version-lockstep. The channel must respect that (its own tags/cadence, not welded to core releases).

**Providers are NOT crates (user, 2026-07-28).** A provider is a packaged set of constructor factories for reuse; the signed `.cloacina` provider package is the ONLY publication form. The source crate is a build input, never a distribution channel — crates.io is categorically off the table for providers.

**Options (both distribute the provider PACKAGE):**
- **(a) GitHub Release assets via provider-scoped tags** — a `provider_release.yml` triggered by tags like `provider-kafka-v0.1.0`: builds linux-amd64 + linux-arm64 native packages (the fleet's archs; darwin optional later), signs them, attaches `cloacina-provider-kafka-<ver>-<arch>.cloacina` to a GH release for that tag. Cheap, versioned, honors independent cadence. **Recommended first step.**
- **(b) OCI artifacts on ghcr** (like the helm chart) — most future-proof (`cloacinactl` could pull provider refs directly); more machinery. **Good v2**; don't block on it.

**Open items:**
- Signing: which key signs first-party provider packages, and where does it live as a CI secret? (Provider packages are signed per I-0132.)
- Whether `cloacinactl` should grow an install-from-URL/OCI verb to close the consumption loop.
- Whether the provider crate stays under `examples/` (fine for (a)) or moves to a `providers/` top-level directory as the roster grows.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Channel decision recorded (a/b/c or combination), with the provider-vs-core cadence question settled.
- [ ] An automated workflow produces signed per-arch `.cloacina` packages for `cloacina-provider-kafka` from a tag or manual dispatch.
- [ ] Artifacts are downloadable from a stable, versioned location and a documented consumption path exists (docs page: obtain → upload → constructor! from = "kafka@version").
- [ ] The signing-key question is resolved and wired as a CI secret.

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

*To be added during implementation*
