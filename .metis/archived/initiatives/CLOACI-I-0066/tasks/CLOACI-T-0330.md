---
id: fix-existing-document-accuracy
level: task
title: "Fix existing document accuracy issues (~10 docs)"
short_code: "CLOACI-T-0330"
created_at: 2026-04-02T22:51:46.039834+00:00
updated_at: 2026-04-02T23:39:08.151899+00:00
parent: CLOACI-I-0066
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0066
---

# Fix existing document accuracy issues (~10 docs)

## Parent Initiative
[[CLOACI-I-0066]]

## Objective
Fix accuracy issues, expand stubs, and fill incomplete content in ~10 existing docs.

## Fixes Required

### Accuracy Fixes
1. **Tutorial 06 (multi-tenancy)**: API signature mismatch — uses `Context`/`PipelineError` instead of `Context<Value>`/`TaskError`. Verify against actual code in examples/tutorials/06-multi-tenancy/ and fix.
2. **Tutorial 05 (cron)**: Malformed reviewer_date "2024-04-2" → "2024-04-02". Review hypothetical output section for accuracy.
3. **Tutorials 03-04**: `rand` crate used in code examples but not listed in dependencies section. Add `rand = "0.8"` to the Cargo.toml examples shown.

### Stub Expansion
4. **how-to-guides/_index.md**: Currently one sentence. Add overview paragraph and categorized list of all how-to guides with brief descriptions.
5. **explanation/_index.md**: Currently one sentence. Add overview paragraph and categorized list of all explanation docs.
6. **how-to-guides/multi-tenant-recovery.md**: Only ~300 words. Expand with concrete recovery scenarios, code examples, and step-by-step procedures.
7. **explanation/performance-characteristics.md**: Incomplete stub with images but no real content. Add methodology, test descriptions, results interpretation, and tuning guidance.

### Cleanup
8. **reference/_index.md**: Remove `draft: true`, add overview of Reference section listing all reference docs.
9. **reference/api-test.md**: Mark as `draft: true` to hide from navigation (it's a testing page, not user-facing).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] All API signatures in tutorials match actual code
- [ ] No malformed dates in frontmatter
- [ ] All code examples include correct dependencies
- [ ] All stub pages expanded to meaningful content
- [ ] reference/_index.md is published (not draft)

## Status Updates
*To be added during implementation*
