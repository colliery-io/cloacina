---
id: t-05-websocket-metrics-soak
level: task
title: "T-05: WebSocket metrics + soak cardinality assertion"
short_code: "CLOACI-T-0588"
created_at: 2026-05-14T13:03:15.987499+00:00
updated_at: 2026-05-14T13:36:46.605635+00:00
parent: CLOACI-I-0099
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0099
---

# T-05: WebSocket metrics + soak cardinality assertion

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0099]]

## Objective **[REQUIRED]**

Instrument the WebSocket layer (accumulator + reactor endpoints) and add a soak-time cardinality assertion that guards the whole I-0099 metric set against accidental unbounded labels.

Metrics to add:
- `cloacina_ws_connections_active{endpoint}` — gauge. Endpoint ∈ {accumulator, reactor}.
- `cloacina_ws_messages_total{endpoint,direction}` — counter. Direction ∈ {in, out}.
- `cloacina_ws_auth_failures_total{reason}` — counter. Reason bounded: {ticket_expired, invalid_signature, tenant_mismatch, not_authorized}.

Plus: extend the CG soak (`angreal test soak server`) or `ws-integration` suite with an enumeration step that scrapes `/metrics`, groups label sets per metric, and asserts each metric is below a fixed cardinality ceiling.

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

- [ ] All three WS metrics emit on a real WebSocket handshake + message exchange.
- [ ] `connections_active` correctly decrements on disconnect (no slow leaks under soak).
- [ ] Auth-failure counter increments for each of the four bounded reasons in dedicated tests.
- [ ] Soak/integration test enumerates emitted labels and asserts cardinality ≤ documented ceiling for every `cloacina_*` metric introduced in I-0099.
- [ ] Promtool format check passes; metrics doc updated and initiative exit criteria met.

## Implementation Notes

This task is sequenced last: cardinality assertion only meaningful once T-0584..T-0587 have landed. Re-use the `/metrics` scrape helper from T-0536.

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

### 2026-05-14 — implemented

- `cloacina_ws_connections_active{endpoint}` driven by `WsConnectionGuard` RAII guard — incremented in handler entry, decremented on Drop so panics inside the handler can't leak the gauge (defense against the T-0534 leak shape).
- `cloacina_ws_messages_total{endpoint,direction}` recorded on every framed in/out message in both `handle_accumulator_socket` and `handle_reactor_socket` — ping/pong handled by axum stays excluded as documented.
- `cloacina_ws_auth_failures_total{reason}` emitted at every failure path via `record_ws_auth_failure()`: missing token → `not_authorized`, bearer validate Err → `invalid_signature`, ticket consume None → `ticket_expired`, endpoint authZ deny → `tenant_mismatch`.
- All three metrics registered in `crates/cloacina-server/src/lib.rs` with describe_* calls.
- Unit tests added: `test_ws_metrics_emit` covers each endpoint × direction + four auth-failure reasons.
- **Cardinality assertion**: `test_i0099_cardinality_within_ceiling` emits every I-0099 metric across its full bounded label domain, scrapes `/metrics`, parses each line, groups by metric name (folding `_bucket` / `_sum` / `_count` histogram suffixes), and asserts every `cloacina_*` series count is ≤ 64. Closes the door on the regression shape "someone added `tenant_id` or `event_key` as a label" — those labels would push the metric series count well above the ceiling.
- `docs/operations/metrics.md`: new rows for all three WS metrics; "Current gaps" updated — only outstanding I-0099 item is the reactor-side dedup wiring (T-0413 follow-up).
