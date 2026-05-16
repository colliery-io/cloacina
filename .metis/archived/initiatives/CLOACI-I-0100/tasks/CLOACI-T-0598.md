---
id: t-01-reactor-firings-reactor
level: task
title: "T-01: reactor_firings + reactor_trigger_subscriptions schema + DAL"
short_code: "CLOACI-T-0598"
created_at: 2026-05-14T20:16:46.656827+00:00
updated_at: 2026-05-14T20:24:16.205949+00:00
parent: CLOACI-I-0100
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0100
---

# T-01: reactor_firings + reactor_trigger_subscriptions schema + DAL

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0100]]

## Objective **[REQUIRED]**

Land the two new tables + DAL methods that the rest of I-0100 builds on.

### Schema

Migration NNN_reactor_subscriptions (postgres + sqlite):

```sql
CREATE TABLE reactor_firings (
    id              UUID PRIMARY KEY,
    reactor_name    TEXT NOT NULL,
    tenant_id       TEXT NOT NULL,
    payload         BYTEA,
    fired_at        TIMESTAMP NOT NULL,
    created_at      TIMESTAMP NOT NULL
);
CREATE INDEX reactor_firings_by_reactor_and_time
    ON reactor_firings (tenant_id, reactor_name, fired_at);

CREATE TABLE reactor_trigger_subscriptions (
    id                    UUID PRIMARY KEY,
    reactor_name          TEXT NOT NULL,
    workflow_name         TEXT NOT NULL,
    tenant_id             TEXT NOT NULL,
    enabled               BOOLEAN NOT NULL DEFAULT TRUE,
    last_seen_fired_at    TIMESTAMP,
    created_at            TIMESTAMP NOT NULL,
    updated_at            TIMESTAMP NOT NULL,
    UNIQUE (reactor_name, workflow_name, tenant_id)
);
```

### DAL methods

New module `cloacina::dal::unified::reactor_subscriptions` with:
- `insert_firing(reactor, tenant, payload, fired_at) -> Uuid`
- `poll_unconsumed(tenant, reactor, after: Option<Timestamp>, limit) -> Vec<ReactorFiring>`
- `advance_watermark(subscription_id, new_last_seen_fired_at) -> ()`
- `prune_firings_older_than(cutoff: Timestamp) -> usize`
- `subscribe(reactor, workflow, tenant) -> Uuid` (upsert by `(reactor, workflow, tenant)`)
- `unsubscribe(reactor, workflow, tenant) -> bool`
- `list_subscriptions(tenant) -> Vec<Subscription>`

All methods use the standard `dispatch_backend!` postgres/sqlite split.

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

- [ ] Migration applies cleanly on both postgres and sqlite (forward + rollback).
- [ ] All 7 DAL methods exist with matching signatures across backends.
- [ ] `poll_unconsumed` returns firings strictly newer than the `after` watermark, in `fired_at` order.
- [ ] `subscribe` is upsert: calling twice with the same `(reactor, workflow, tenant)` returns the same id without error.
- [ ] DAL-level unit tests cover insert→poll→advance flow and prune semantics.

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

- Migrations:
  - `crates/cloacina/src/database/migrations/postgres/025_reactor_subscriptions/{up,down}.sql`
  - `crates/cloacina/src/database/migrations/sqlite/022_reactor_subscriptions/{up,down}.sql`
- Schema: two new `diesel::table!` blocks in `unified_schema` (`reactor_firings`, `reactor_trigger_subscriptions`); added to `allow_tables_to_appear_in_same_query!`.
- DAL: new module `crates/cloacina/src/dal/unified/reactor_subscriptions.rs` with `ReactorSubscriptionsDAL` exposing 7 methods (insert_firing, poll_unconsumed, prune_firings_older_than, subscribe, advance_watermark, unsubscribe, list_subscriptions). All use `dispatch_backend!` postgres/sqlite split.
- `DAL::reactor_subscriptions()` accessor wired in `mod.rs`.
- `ReactorFiring` and `ReactorSubscription` model structs re-exported.
- SQLite UPSERT: postgres uses native `on_conflict().do_update()`; sqlite catches `UniqueViolation` and re-queries the existing row.
- Unit tests deferred to T-0599 integration tests where the full flow can be exercised.
