---
id: ui-manual-execution-surface-a
level: task
title: "UI manual execution — surface a workflow's declared configuration options at execute time"
short_code: "CLOACI-T-0747"
created_at: 2026-06-20T02:26:30.274166+00:00
updated_at: 2026-06-20T13:24:49.012931+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/blocked"


exit_criteria_met: false
initiative_id: NULL
---

# UI manual execution — surface a workflow's declared configuration options at execute time

## Origin

Surfaced during a live demo (2026-06-18/19). When a user manually executes a
workflow from the web UI, there is no way to discover *what* configuration /
context options the workflow actually accepts. The execute dialog takes a
free-form context blob with no guidance, so the operator has to already know the
expected keys, types, and defaults — or guess. This is a discoverability gap on
the most common interactive operation.

## Objective

On manual execution from the UI, present the workflow's declared configuration
surface — the parameters/context keys it accepts, with names, types, whether
each is required, and any default — so an operator can fill them in correctly
without prior knowledge of the workflow internals.

## Backlog Item Details

### Type
- [x] Feature — UI enhancement / discoverability

### Priority
- [x] P1 — High (directly hampers the primary interactive workflow; demo-visible)

### Business Justification
- **User Value**: Operators can run a workflow correctly the first time without
  reading source or docs; removes the "what do I put here?" guesswork.
- **Business Value**: Reduces failed/misconfigured manual runs; makes the UI
  demo-able and self-service for non-authors.
- **Effort Estimate**: M (depends on how much of the declared-params metadata
  already exists server-side — see Dependencies).

## Related work

- **CLOACI-I-0116** — Parameterized workflow instances (declared params, partials,
  configurable execute/schedule). This ticket is the UI consumer of that
  declared-param metadata; the two should align on the schema. If I-0116 lands
  the declared-param model, this becomes mostly a UI/SDK surfacing task.
- **CLOACI-T-0657** (completed) — UI workflow write ops, including
  execute-with-context. This is where the execute dialog lives today.
- **CLOACI-I-0117** — Cloacina web UI initiative (parent home for this work).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] The manual-execute dialog displays the workflow's declared config/context
      options: key name, type, required flag, and default (where declared).
- [ ] Required options are validated client-side before submit; missing/invalid
      values are flagged inline rather than failing at run time.
- [ ] Workflows with no declared schema degrade gracefully (current free-form
      context entry remains available, clearly labeled as unstructured).
- [ ] The schema is sourced from server/SDK metadata, not hand-maintained in the
      UI (no drift between what the workflow accepts and what the UI shows).

## Open Questions / Dependencies

- Does the server already expose per-workflow declared-param metadata via the
  API/SDK, or does that need to land first (gating on I-0116)? Confirm before
  sizing — this determines whether the task is UI-only or full-stack.
- Schema shape: reuse whatever I-0116 defines for declared params rather than
  inventing a UI-only descriptor.

## Status Updates

### 2026-06-20 — Dependency check done → BLOCKED (gated on I-0116 + frontend freeze)

Ran the open-question dependency check. Findings:
- **No declared-param metadata exists server-side.** Grep of
  `cloacina-api-types/src/workflows.rs`, `packaging/manifest_schema.rs`, and
  `registry/loader/package_loader.rs` finds no param schema / declared-params /
  input-schema of any kind. The execute API is a free-form blob:
  `ExecuteRequest { context: Option<serde_json::Value> }`
  (`cloacina-api-types/src/executions.rs:24`) — exactly the "no guidance" gap
  this ticket describes.
- **The declared-param model is owned by CLOACI-I-0116** (Parameterized workflow
  instances — declared params), which is in **discovery** (not built). There is
  no data source to surface yet.

**Blocked on two fronts:**
1. **Data**: I-0116 must land the declared-param model + expose it via the
   API/SDK before there's anything to show. (Hard dependency.)
2. **Surface**: this is a manual-execute *dialog* change in the UI — under the
   current frontend freeze (designer reviewing the UI; no churn). Even with the
   data, the UI work waits for the freeze to lift.

Recommendation: keep blocked under I-0116; pick up when I-0116 delivers the
declared-param API and the frontend freeze lifts. At that point this becomes
mostly a UI/SDK surfacing task (validate inputs from the declared schema, fall
back to the free-form blob for undeclared workflows).
