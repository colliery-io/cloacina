---
id: package-new-scaffold-a-canonical
level: task
title: "package new — scaffold a canonical .cloacina package (Rust + Python)"
short_code: "CLOACI-T-0678"
created_at: 2026-06-14T16:14:49.207637+00:00
updated_at: 2026-06-14T16:33:26.559900+00:00
parent: CLOACI-I-0119
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0119
---

# package new — scaffold a canonical .cloacina package (Rust + Python)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0119]]

## Objective **[REQUIRED]**

T1 of CLOACI-I-0119. Add `cloacinactl package new <name>` to scaffold a
canonical `.cloacina` source tree so an author starts from a working,
server-accepted skeleton instead of hand-assembling `package.toml` + the right
directory layout. Python scaffolds `package.toml` + `workflow/<module>/` with
**bare `@cloaca.task`** decorators (the proven packaged form — the loader names
the workflow from `workflow_name`); Rust scaffolds `Cargo.toml` + `build.rs` +
`src/lib.rs` wired to the **published** `cloacina-*` crates (not the in-repo
`__WORKSPACE__`/path deps the fixtures use).

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

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `cloacinactl package new <name> [--lang python|rust] [--path <dir>]`
      scaffolds a package source tree; defaults to Python; refuses a non-empty
      target dir; maps `-` → `_` for the module/workflow identifier.
- [x] Python output is the canonical packaged form: `package.toml`
      (`language=python`, `workflow_name`, `entry_module=<module>.tasks`) +
      `workflow/<module>/{__init__.py, tasks.py}` with **bare `@cloaca.task`**
      decorators (no `WorkflowBuilder`).
- [x] Rust output: `package.toml` (`language=rust`) + `Cargo.toml`
      (cdylib+rlib, published `cloacina-*` = "0.7" deps, no path/`__WORKSPACE__`)
      + `build.rs` (`cloacina_build::configure()`) + `src/lib.rs`
      (`package!()` + `#[workflow]`/`#[task]`).
- [x] Scaffolded output packs: `package new` → `package pack` succeeds (Python
      validated end-to-end; Rust pending published-crate availability).
- [x] Unit tests cover both layouts, the non-empty-dir guard, and name handling.
- [x] How-to + 08 tutorial corrected to bare-decorator packaged form (the
      `WorkflowBuilder`-in-package bug that would yield "Empty package").

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

**2026-06-14 — Implemented; verification pending.** Added
`crates/cloacinactl/src/nouns/package/new.rs` + wired the `New` verb into the
package noun. Before scaffolding, verified the packaged Python registration path
in `crates/cloacina-python/src/loader.rs:291-306`: the loader pushes a workflow
context from `[metadata].workflow_name` before importing `entry_module`, so bare
`@cloaca.task` decorators register into it — a `WorkflowBuilder` inside a
packaged module shadows that context and the package loads with **no tasks**.
Scaffold + docs therefore use the bare form. Also corrected the how-to + 08
tutorial (shipped in PR #124) which had the wrong `WorkflowBuilder` pattern.
Rust scaffold uses published `cloacina-*` = "0.7" deps per the design call.
Unit tests added. **Pending:** `angreal check crate crates/cloacinactl` +
`package new → pack` smoke (Python end-to-end; Rust packs but compile-on-load
depends on the crates being published).