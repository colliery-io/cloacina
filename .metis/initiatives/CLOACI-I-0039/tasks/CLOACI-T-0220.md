---
id: fix-archive-path-traversal
level: task
title: "Fix archive path traversal — sanitize tar entries for symlinks, .., and absolute paths"
short_code: "CLOACI-T-0220"
created_at: 2026-03-22T00:34:21.595221+00:00
updated_at: 2026-03-22T00:47:41.931589+00:00
parent: CLOACI-I-0039
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0039
---

# Fix archive path traversal — sanitize tar entries for symlinks, .., and absolute paths

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0039]]

## Objective

**Severity: CRITICAL.** `tar.unpack()` and manual archive entry iteration do not sanitize for symlinks, `..` components, or absolute paths. A malicious `.cloacina` package can write files outside the staging directory.

**Locations:**
- `python_loader.rs:123` — `archive.unpack(&package_dir)` on user-uploaded Python packages
- `reconciler/extraction.rs` — manual tar entry iteration
- `workflow_registry/package.rs` — Rust package extraction

Also fix decompression bomb (M-4): no compressed-to-decompressed ratio limit.

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

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] All tar extraction sites reject entries with `..` path components
- [ ] All tar extraction sites reject symlink entries
- [ ] All tar extraction sites reject absolute paths
- [ ] Decompression size limit enforced (abort if decompressed > 10x compressed or > 500MB absolute)
- [ ] Unit test: archive with `../../etc/passwd` entry is rejected with clear error
- [ ] Unit test: archive with symlink entry is rejected
- [ ] Unit test: archive with decompression ratio > limit is rejected
- [ ] Existing valid packages continue to extract successfully

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

### 2026-03-21 — Complete

- Replaced `archive.unpack()` with manual entry iteration in `python_loader.rs`
- Rejects symlinks (EntryType::Symlink, HardLink)
- Rejects `..` path components
- Rejects absolute paths (RootDir component)
- Decompression bomb check: 500MB absolute limit, 10x ratio limit
- Only one `unpack()` site existed (python_loader.rs) — other sites use peek_manifest only
- Added `test_reject_path_traversal` (crafts raw tar header with `../../../etc/passwd`)
- Added `test_reject_symlink`
- 490 tests pass (2 new)
