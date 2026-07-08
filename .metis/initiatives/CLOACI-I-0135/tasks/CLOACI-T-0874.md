---
id: spike-interact-on-backend-macro
level: task
title: "Spike — interact_on_backend! macro + migrate db_key_manager.rs (settle error-mapping ergonomics)"
short_code: "CLOACI-T-0874"
created_at: 2026-07-08T22:40:31.932888+00:00
updated_at: 2026-07-08T22:58:35.564693+00:00
parent: CLOACI-I-0135
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0135
---

# Spike — interact_on_backend! macro + migrate db_key_manager.rs (settle error-mapping ergonomics)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0135]]

## Objective **[REQUIRED]**

{Clear statement of what this task accomplishes}

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

### 2026-07-08 — Spike COMPLETE, macro validated, recommend I-0135 proceeds as designed

**Macro built:** `interact_on_backend!($dal, |conn| <diesel body>)` added next to `dispatch_backend!` in `crates/cloacina/src/database/connection/backend.rs` (+87 lines, one-time cost amortized across the whole sweep). Expands via `dispatch_backend!($dal.backend(), <pg arm>, <sqlite arm>)`; each arm checks out its concrete pooled connection and runs `__conn.interact(move |conn| <body>)`. The KEY INSIGHT held: under dual-feature the same `move` closure is emitted into both mutually-exclusive match arms and compiles — the `--features postgres,sqlite,macros` build is clean.

**Error-mapping contract chosen — Option (a), fold-into-diesel.** The macro evaluates to `Result<R, diesel::result::Error>` (already `.await`ed; no `.await` at call site). The two non-Diesel failure layers — deadpool pool-checkout errors and `deadpool_diesel::InteractError` — are folded into `diesel::result::Error::QueryBuilderError(e.to_string().into())`. The CALL SITE then maps exactly ONE `diesel::result::Error` to its domain error with a single `.map_err(|e| …)?`.

Rationale: this is the SAME single closure each twin already used for its *inner* Diesel result, so migration is nearly mechanical — lift the existing inner `.map_err` verbatim. A genuine `diesel::result::Error::NotFound` / UNIQUE violation is the inner error and passes through UNCHANGED, so `matches!(e, NotFound)` and the duplicate-string checks keep working. Behavior preserved: pool/interact errors still land in the domain `Database(_)` variant (they don't match NotFound/UNIQUE, so they fall through the call-site closure exactly as before); only the wrapped message string differs slightly. Rejected (b) error-map-closure param (extra noise at 330 sites) and (c) crate-level `DalError` + `From` (would force every call site into a `match` to recover the diesel NotFound — strictly more verbose).

**Divergent cases in db_key_manager: NONE needed to stay twinned.** The one genuinely-divergent bit here — the duplicate-name string check (Postgres "duplicate key…" vs SQLite "UNIQUE constraint…") — collapses cleanly: `contains("duplicate") || contains("UNIQUE")` is a behavior-preserving superset for either backend (the foreign token simply never appears). So all 24 twins in this module collapsed. (The initiative's ~4 genuinely-divergent ops — SKIP LOCKED, isolation level, RETURNING gaps — live in OTHER modules and are unaffected.)

**Leverage proof (db_key_manager.rs):** 1835 → 1216 lines (−619, −34%). Two twin impl blocks (24 `*_postgres`/`*_sqlite` methods, ~668 lines) deleted; 12 single-call queries inlined directly into their trait methods, and 3 multiply-called twins collapsed into 3 backend-agnostic private helper methods (56-line impl). 13 `interact_on_backend!` call sites total; zero `dispatch_backend!`/`_postgres`/`_sqlite` references remain (bar one comment).

**Gate — all green:**
- `cargo check -p cloacina --no-default-features --features sqlite,macros` → Finished (clean).
- `cargo check -p cloacina --no-default-features --features postgres,macros` → Finished (clean).
- `cargo check -p cloacina --features postgres,sqlite,macros` → Finished (clean; proves the dual-arm move-closure expansion).
- `cargo test -p cloacina --lib --features sqlite,macros -- security::db_key_manager` → 37 passed, 0 failed (incl. NotFound mapping, revoke-not-found → NotFound/TrustNotFound, grant/list/find paths).
- `cargo fmt --all` clean.
- (No live-DB postgres test lane exercised; sqlite tests + tri-feature compile are the gate, per task.)

**Assessment — ergonomically clean; recommend the sweep proceeds as designed.** The macro reads well at the call site (query body written once, one familiar `.map_err(...)?` after), monomorphizes correctly per backend, and the fold-into-diesel contract means migrating a twin is close to copy-the-body + lift-the-inner-map_err. No pooling or per-backend-setup changes were needed. Only caveat for the sweep: divergent duplicate/constraint string handling must be reviewed per-module (here it unified via the OR superset), and the genuinely-divergent SQL ops (claiming.rs, isolation, RETURNING gaps) stay explicit twins as the initiative already scopes.