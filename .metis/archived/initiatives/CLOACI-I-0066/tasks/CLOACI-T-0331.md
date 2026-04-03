---
id: parallel-review-pass-accuracy
level: task
title: "Parallel review pass (Accuracy, Completeness, Clarity, Diátaxis)"
short_code: "CLOACI-T-0331"
created_at: 2026-04-02T22:51:47.048126+00:00
updated_at: 2026-04-02T23:43:38.922241+00:00
parent: CLOACI-I-0066
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0066
---

# Parallel review pass (Accuracy, Completeness, Clarity, Diátaxis)

## Parent Initiative
[[CLOACI-I-0066]]

## Objective
Launch 4 parallel review agents after all writing is complete to validate the full documentation set.

## Review Agents

### 1. Accuracy Agent
- Cross-reference every claim in docs against actual codebase
- Verify all code examples compile/run conceptually
- Check API signatures, config defaults, CLI flags match code
- Flag any discrepancies

### 2. Completeness Agent
- Compare documented features against codebase feature inventory
- Identify any flags, config options, endpoints, or workflows missing from docs
- Check that every example in examples/ is referenced somewhere in docs

### 3. Clarity Agent
- Read each doc from its target audience perspective
- Flag jargon without definition, unclear steps, missing context
- Check that tutorials don't assume prior knowledge
- Verify how-to guides have clear goals and actionable steps

### 4. Diátaxis Compliance Agent
- Verify tutorials stay learning-oriented (not reference)
- Verify how-to guides stay task-oriented (not explanation)
- Verify reference docs are lookup-structured (not narrative)
- Verify explanation docs are understanding-oriented (not procedural)
- Flag any category violations

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] All 4 agents run and produce findings
- [ ] All critical findings addressed before finalizing
- [ ] No remaining placeholder text in any document

## Blocked By
- CLOACI-T-0327 (new reference docs)
- CLOACI-T-0328 (new how-to guides)
- CLOACI-T-0329 (new explanation docs)
- CLOACI-T-0330 (existing doc fixes)

## Status Updates
*To be added during implementation*
