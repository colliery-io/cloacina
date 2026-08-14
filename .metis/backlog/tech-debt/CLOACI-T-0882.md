---
id: investigate-cloaca-retrypolicy
level: task
title: "Investigate: cloaca RetryPolicy/BackoffStrategy/RetryCondition value objects exposed but unwired"
short_code: "CLOACI-T-0882"
created_at: 2026-07-09T22:56:10.031554+00:00
updated_at: 2026-08-10T22:50:18.846076+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Investigate: cloaca RetryPolicy/BackoffStrategy/RetryCondition value objects exposed but unwired

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective **[REQUIRED]**

Surfaced during I-0137 (cloaca registrar) + the maintainer's coverage catch: `cloaca` exposes `RetryPolicy`, `RetryPolicyBuilder`, `BackoffStrategy`, `RetryCondition` value-object classes (`bindings/value_objects/retry.rs`), but **`@cloaca.task` configures retry via kwargs** (`retry_attempts=`, `retry_backoff=`, `retry_delay_ms=` → `build_retry_policy`, `task.rs:502`), NOT by accepting a `RetryPolicy` object. **Zero** usages in `examples/` or `tests/python/` — they look exposed-but-unwired (dead exports).

**Decide:** (a) wire them — `@cloaca.task(retry=RetryPolicy(...))` accepts the object (richer builder API), OR (b) drop them from the authorship contract if kwargs is the intended surface. Either way add a Python test that exercises the chosen retry-authoring path — the missing coverage is the same blind spot that hid the workflow_secrets/RetryPolicy server-drift until [[CLOACI-I-0137]]. Low priority; not a correctness bug (kwargs retry works).

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

Decision: **(a) wire them.** Rationale in the status update below.

- [x] `@cloaca.task(retry=RetryPolicy(...))` accepts the value object and applies it.
- [x] Passing `retry=` together with any `retry_*` kwarg raises `ValueError` naming
      the conflicting kwargs, rather than silently preferring one surface.
- [x] A Python test constructs a `RetryPolicy` through the full builder chain
      (`BackoffStrategy` + `RetryCondition` + jitter) and reads every getter back.
- [x] A Python test executes a workflow whose task was configured via the object.
- [x] A Python test asserts the mutual-exclusion error.
- [x] Verified live: `angreal test integration --python-file
      test_scenario_11_retry_mechanisms.py --backend sqlite` → EXIT=0,
      `PASSED: test_scenario_11_retry_mechanisms.py`.

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

- 2026-08-10 — BACKLOG AUDIT. Verdict: **still true, unchanged.** Every
  specific claim re-verified:
    * `crates/cloacina-python/src/bindings/value_objects/retry.rs` still exists
      and still exports the value objects (47 `retry` references).
    * `@cloaca.task` still takes `retry_attempts` / `retry_backoff` kwargs
      (`task.rs:594-606`) feeding `build_retry_policy`. There is still NO
      `retry=` / `retry_policy=` parameter accepting the object.
    * Usages of `RetryPolicy` across `tests/python/` and `examples/`: still
      **zero**. Searched for both the type name and a `retry=` call site.

  So the decision this ticket asks for — (a) wire the object, or (b) drop it
  from the authorship contract — is still unmade, and the exports are still
  dead.

  ONE SHARPENING, from this session's evidence rather than from re-reading the
  ticket. The filing calls this "low priority; not a correctness bug (kwargs
  retry works)", which is true of runtime behavior but understates the risk.
  T-0893 just found that `cloacinactl execution events` had NEVER worked —
  shipped, documented, and broken 100% of the time — for precisely the reason
  named here: nothing ever exercised it. An exposed-but-unexercised Python
  class is the same shape of hazard one level earlier. Nobody can currently
  tell whether `RetryPolicy(...)` even CONSTRUCTS correctly from Python,
  because no test constructs one.

  That does not change the priority (still low — no user is broken today), but
  it does argue for option (b) as the default answer unless someone wants the
  builder API. Deleting a dead export is cheap and removes a surface that can
  rot invisibly; wiring it means owning it in tests forever. Whoever picks
  this up should start by just trying to construct one in a REPL — if it
  already fails, (b) becomes obvious and the ticket closes in an hour.

  Acceptance criteria are still template placeholders and should be filled in
  once (a)/(b) is chosen; not filling them in now, since the choice is the
  point of the ticket.

- 2026-08-12 — IMPLEMENTED. Chose **(a) wire them**, reversing the audit's own
  lean toward (b). Three facts found while implementing changed the answer:

  1. **They are already contract, not accidental exports.** `lib.rs:350-353`
     lists `RetryPolicy`/`RetryPolicyBuilder`/`BackoffStrategy`/`RetryCondition`
     in the I-0137 authorship-contract assertion. Option (b) would have meant
     deliberately shrinking a contract someone deliberately widened.
  2. **The conversion already existed.** `PyRetryPolicy::to_rust()`
     (`retry.rs:304`) already produced a `cloacina::retry::RetryPolicy`, so
     wiring cost one match arm rather than a new adapter.
  3. **The kwargs surface is the weaker one.** `build_retry_policy` maps
     unknown strings through `_ => BackoffStrategy::Fixed` and
     `_ => RetryCondition::AllErrors`, so `retry_backoff="exponentail"`
     silently yields Fixed. The typed path cannot express that typo —
     `BackoffStrategy.exponential(...)` is a method, so a typo is an
     AttributeError. Wiring the object ADDS a safer surface; deleting it would
     have left only the silently-defaulting one. (That kwargs typo-swallow is
     a separate defect, still unfixed — see the residual note below.)

  WHAT THE NEW TESTS IMMEDIATELY CAUGHT (the ticket's whole premise, confirmed):
  `BackoffStrategy.exponential(base, multiplier: Option<f64>)` did
  `multiplier.unwrap_or(1.0)` — clearly intending an optional argument — but
  pyo3 does NOT infer optionality from `Option<T>` without an explicit
  `#[pyo3(signature = ...)]`. So the argument was REQUIRED and
  `BackoffStrategy.exponential(2.0)` raised
  `TypeError: missing 1 required positional argument: 'multiplier'`. The very
  first line of Python ever written against these objects hit it. Fixed with
  `#[pyo3(signature = (base, multiplier=None))]`. Audited the rest of the file:
  `exponential` was the only method with this shape.

  IMPLEMENTATION
  - `task.rs`: new `retry` param; `retry=` and `retry_*` are mutually exclusive
    and the conflict raises `ValueError` naming the offending kwargs. Silently
    preferring one surface would recreate this ticket's exact failure mode
    ("I configured it and nothing happened") one level up.
  - `retry.rs`: the `exponential` signature fix above.
  - `tests/python/test_scenario_11_retry_mechanisms.py`: three new tests.
    The construct-test asserts `calculate_delay(2) > calculate_delay(1)`,
    because a silently-defaulted Fixed strategy would return a constant and
    otherwise look identical to a working exponential.

  BLOCKER HIT AND FIXED ALONG THE WAY — see the harness note; `import cloaca`
  was fatally broken locally, so NO Python scenario could run at all until it
  was fixed. That is why this ticket touches `.angreal/test/_python_utils.py`.

  RESIDUAL (not fixed here, deliberately): `build_retry_policy`'s silent
  `_ =>` fallbacks. Fixing it means erroring on unrecognized `retry_backoff` /
  `retry_condition` strings, which is a behavior change for any existing
  workflow currently passing a typo and silently getting Fixed/AllErrors. That
  deserves its own ticket rather than riding along here.