---
id: docs-hardening-sweep-i-0125
level: task
title: "Docs hardening sweep — I-0125 authoring-shell + weekend-feature drift (4-agent review findings)"
short_code: "CLOACI-T-0856"
created_at: 2026-07-07T11:28:56.814548+00:00
updated_at: 2026-07-07T11:28:56.814548+00:00
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

# Docs hardening sweep — I-0125 authoring-shell + weekend-feature drift (4-agent review findings)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Fix the docs drift surfaced by a 4-agent Diátaxis review (accuracy/completeness/clarity/compliance) on 2026-07-07. Root cause: I-0125 authoring-cruft removal + this weekend's features (I-0105 sandbox, I-0116 instances, I-0130 embedded UI, CG fleet dispatch) shipped, but docs weren't swept.

### GROUND TRUTH — real minimal Rust package shell (crates/cloacinactl/src/nouns/package/new.rs:311-364)
Cargo.toml (WHOLE shell — NO build.rs, NO `[lib] crate-type`, NO `[features]`, NO cloacina-build, NO cloacina-macros direct dep):
`[dependencies]` = cloacina-workflow { features=["packaged","macros"] } + cloacina-workflow-plugin + serde + serde_json.
src/lib.rs: `cloacina_workflow_plugin::package!();` (un-gated; NOT `cloacina::package!()`).
Workflow package.toml: `[package] name/version` + `[metadata] workflow_name/description`.
CG package.toml: `[package] name/version/interface="cloacina-workflow-plugin"/interface_version=1/extension="cloacina"` + `[metadata] language="rust"/graph_name/description/reaction_mode/input_strategy` — NOT `type="computation_graph"`/`[graph]` tables.

### FINDINGS CHECKLIST
Tier 1 BROKEN: [ ] deploy-the-web-ui.md (deleted ui/Dockerfile+compose.ui.yml→embedded) [ ] migrating-to-service-mode.md (retired shell) [ ] reference/package-shell-macro.md (retired "crate must" + wrong macro path) [ ] reference/computation-graphs.md (package.toml won't parse)
Tier 2 STALE: [ ] running-the-compiler.md (sandbox "pending"→shipped) [ ] environment-variables.md (+COMPILER_SANDBOX, 7 OIDC_*, compiler TENANT_SCHEMA/BUILD_TARGET(_PACKAGE), PROVIDER_PATH) [ ] http-api.md (auth/session/account + missing routes)
Tier 3 DRIFT: [ ] computation-graph-scheduling.md (+fleet dispatch) [ ] compiler-sandbox.md (--unshare-all→per-namespace) [ ] macros.md (3→5 macros)
Tier 4 DIÁTAXIS: [ ] backend-selection.md (how-to→explanation) [ ] performance-optimization.md (split) [ ] multi-tenancy.md (split) [ ] python-api/workflow.md (strip to reference) [ ] security/package-signing.md (split)
Tier 5 CLARITY: [ ] packaging-python-workflows.md (bare-decorator contradiction) [ ] 07-event-triggers.md+14-packaged-triggers.md (binding) [ ] 04-error-handling.md (retry never fails) [ ] 05-cron-scheduling.md (5 vs 6 field) [ ] workflow-instances.md (declared origin+link) [ ] 09-workflow-registry.md (fwd-ref 14) [ ] 03-packaged-workflows.md (features=packaged) [ ] cross-cutting ticket-ID leak + CLI drift

Executed via file-disjoint parallel editing agents; verified with docs build/link check; single PR.

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

*To be added during implementation*