---
id: fires-log-renders-state
level: task
title: "Fires log renders state-accumulator windows as null — capture_fire_inputs only decodes passthrough frames"
short_code: "CLOACI-T-0842"
created_at: 2026-07-05T22:47:18.558543+00:00
updated_at: 2026-07-05T22:59:18.794443+00:00
parent: 
blocked_by: []
archived: true

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fires log renders state-accumulator windows as null — capture_fire_inputs only decodes passthrough frames

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Filed as the "fires log shows `inputs: null` for state accumulators" cosmetic (known residual from T-0839's live verification). Investigation revealed the real shape of the bug — the state accumulator's boundary wire format, `bincode(Vec<serde_json::Value>)`, is **WRITE-ONLY**: `serde_json::Value` cannot deserialize from bincode (non-self-describing format; `Value::deserialize` needs `deserialize_any`). Nothing anywhere could ever read a state window:

- `capture_fire_inputs` (fires log) → `null` — the reported symptom
- `input_cache_to_ffi_cache` (packaged-CG FFI + fleet dispatch) → hard error
- `PythonGraphExecutor::execute`'s input loop (`cache.get::<Value>`) → silently skipped — **and this decode failure also hit passthrough frames, meaning Python CG nodes have NEVER received accumulator inputs at all** (empty `cache_values` on every fire; nodes tolerate missing args so nobody noticed)

## Fix — one wire shape for every boundary

Emit state windows in the SAME shape passthrough events use: `bincode(Vec<u8>)` wrapping JSON bytes (a JSON array of the bounded history). `state_window_frame()` helper at both emit sites (restore-time + per-append). Every existing decoder then just works — the fires log's first decode branch, the FFI cache conversion, and (fixed alongside, same bug class) the Python executor's input loop now decodes the real wire shape (`get::<Vec<u8>>` + JSON parse) instead of the impossible `get::<Value>`.

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (fires-log observability + Python CG node inputs)

### Impact Assessment
- **Affected Users**: anyone using state accumulators (window invisible everywhere) or Python CGs with accumulator-fed nodes (inputs silently empty).
- **Reproduction Steps**:
  1. Inject events into a state accumulator (e.g. `py_window`, capacity 5) until its reactor fires.
  2. `GET /v1/health/reactors/<reactor>/fires` → the state source renders `inputs: null`.
- **Expected vs Actual**: expected the bounded window as a JSON array; actual `null` (and empty node inputs on the py side).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Fires log renders a state accumulator's window as a JSON array (live-verified).
- [ ] Passthrough capture unchanged (regression test covers both shapes).
- [ ] State runtime tests updated to the new wire shape; CG suite green.
- [ ] Python CG node inputs actually populated (decode the real frame shape).

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

### 2026-07-05 — FIXED + LIVE-PROVEN (branch fix/t0842-fires-log-state-windows, commit 96774394)
First attempt (decode `bincode(Vec<Value>)` in `capture_fire_inputs`) was DISPROVEN by its own regression test — bincode cannot deserialize `Value` at all (the runtime tests that seemed to contradict this decode `Vec<i64>`, not `Value`). That proved the wire format itself was write-only and forced the correct fix at the EMIT side: `state_window_frame()` ships the window as `bincode(Vec<u8>)` of the JSON array (the passthrough shape) at both emit sites; every existing decoder then just works. The Python executor's input loop — which could never decode ANY frame shape via `get::<Value>` — now decodes the real wire (`get::<Vec<u8>>` + JSON parse), so py CG nodes receive accumulator inputs for the first time.

**Tests**: `capture_fire_inputs_decodes_state_windows_and_passthrough` (both shapes), state runtime tests updated to the new wire, CG suite 46/46.

**LIVE PROOF (demo stack)**: five injects into `py_window` (capacity 5) → `demo_py_state_rx` fires show the window GROWING in the fires log — `"inputs":{"py_window":[{"value":1.0},…,{"value":4.0}]}` then `[…,{"value":5.0}]` — previously an unbroken column of `null`. All ACs met. COMPLETE.