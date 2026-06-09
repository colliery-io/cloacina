---
id: cut-v0-7-0-release-fleet-default
level: task
title: "Cut v0.7.0 release (fleet + default-executor)"
short_code: "CLOACI-T-0641"
created_at: 2026-06-09T23:22:22.622107+00:00
updated_at: 2026-06-09T23:22:56.888543+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Cut v0.7.0 release (fleet + default-executor)

## Objective

Cut the `v0.7.0` release. The fleet + delivery-substrate work (I-0114/I-0115) and the
server-level default-executor change (CLOACI-T-0640) landed on `main` but were never
released — workspace is still pinned at `0.6.1` (== last tag), and `unified_release.yml`
hard-gates `Cargo.toml version == tag`, so no release could fire. 32 commits sit on `main`
past `v0.6.1`.

**Version rationale (0.7.0, minor bump):** T-0640 removed `Router` / `RoutingConfig` /
`RoutingRule` from the public prelude — a breaking change for library consumers, which
pre-1.0 maps to a minor bump. Fleet is also a substantial new feature.

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

## Acceptance Criteria

- [ ] Workspace version bumped `0.6.1 → 0.7.0` (workspace.package + all cloacina-internal path-dep pins)
- [ ] `Cargo.lock` regenerated (workspace members only; `deadpool-diesel` 0.6.1 left untouched)
- [ ] CHANGELOG `[Unreleased]` rolled to `[0.7.0] - 2026-06-09` with fleet (I-0114/I-0115) + default-executor (T-0640) entries
- [ ] Install snippets bumped to 0.7.0 (README + docs)
- [ ] Commit on `main`, tag `v0.7.0`, push
- [ ] `unified_release.yml` green (nightly suite → verify-version → crates.io publish + GitHub release)

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

## Status Updates

**2026-06-09** — Release prep done:
- Bumped 20 cloacina-internal Cargo.toml pins `0.6.1 → 0.7.0`; `cargo update --workspace --offline` synced the 12 workspace members in `Cargo.lock` (deadpool-diesel 0.6.1 left untouched). `cargo metadata` confirms cloacina @ 0.7.0.
- CHANGELOG `[Unreleased]` rolled to `[0.7.0] - 2026-06-09`; added fleet (I-0114/I-0115) + default-executor (T-0640) entries, plus the breaking glob-routing-removal note.
- Bumped 46 install-snippet refs across README + docs to 0.7.0 (prior 0.6.1 bump was non-exhaustive; cleared the drift).
- Next: commit to main → tag `v0.7.0` → push → `unified_release.yml` (nightly suite → verify-version → crates.io + GitHub release). Watching the run to green.
