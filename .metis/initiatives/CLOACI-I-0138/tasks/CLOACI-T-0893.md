---
id: operational-surface-coverage
level: task
title: "Operational-surface coverage — reactor fire, accumulator inject, trigger pause/fire, execution events woven into owning examples"
short_code: "CLOACI-T-0893"
created_at: 2026-07-11T22:03:42.162382+00:00
updated_at: 2026-08-30T00:34:28.530686+00:00
parent: CLOACI-I-0138
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0138
---

# Operational-surface coverage — reactor fire, accumulator inject, trigger pause/fire, execution events woven into owning examples

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0138]]

## Objective **[REQUIRED]**

The operational half of the primary interface — the verbs an operator uses on a RUNNING system — is taught nowhere. Surfaces (all shipped, all dark): `cloacinactl reactor force-fire` / `reactor fire <name> <inputs>` (typed), `accumulator inject <name> <event>`, `trigger list/inspect` + server trigger pause/resume/fire routes, `graph list/status/accumulators`, `execution events --follow/--since`, `tenant`/`key` lifecycle, fleet provision/deprovision + tenant limits.

**Approach — weave, don't invent:** each verb belongs in the README of the example that OWNS the feature, as an "Operate it" section after "Run it":
- `packaged-graph` / the T-0891 CG tour → `accumulator inject`, `reactor fire`/`force-fire`, `graph status/accumulators`, the accumulator/reactor WebSocket/UI view
- `event-triggers` (migrated) → `trigger list/inspect`, pause/resume, manual fire
- `simple-packaged` (canonical) → `execution events --follow` (already has list/status)
- `multi-tenant` (migrated) → server-side `tenant create/list` + `key create` + fleet provision/limits — the PRIMARY-interface tenant story (vs the embedded DatabaseAdmin it uses today)

Each "Operate it" section is verified live like the Run-it recipes, and — where cheap — asserted in the example's demos-harness runner so CI exercises the verb (e.g. the CG runner injects an event and polls the reactor fire).

**Acceptance:** every listed verb appears in exactly one owning example's verified "Operate it" section; at least `accumulator inject`, `reactor fire`, and `execution events` are asserted by harness runners in CI. Depends on / sequences with the owning examples' migrations (T-0891, event-triggers + multi-tenant migrations).

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

- [x] Every listed verb appears in exactly one owning example's verified "Operate it" section — `cg-feature-tour` (graph/accumulator/reactor), `packaged-triggers` (trigger lifecycle), `simple-packaged` (execution events)
- [x] `accumulator inject`, `reactor fire` and `execution events` asserted by harness runners in CI — plus `graph list/accumulators`, `force-fire`, and the trigger verbs
- [x] Each section verified live rather than written from the source
- [x] Tenant/key/fleet — DROPPED by maintainer decision; already taught in `service/how-to/configure-multi-tenant-deployment.md`, a better home than a workflow example

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

- 2026-08-09 — most of the "blocked on migrations" framing turned out to be
  wrong, and one documented command was broken.

  The ticket sequences this behind migrating `event-triggers` and `multi-tenant`
  to packaged form. Checked each owner instead of taking that on faith:

  * **`simple-packaged` → `execution events`** — DONE. Added an "Operate it"
    section covering the trail, `--follow` (SSE), and `--since`, including the
    constraint that the two CANNOT be combined (`--follow` starts from now, so
    there is no cursor to resume from). Verified in
    `nouns/execution/mod.rs:97-108`, not guessed.

  * **`cg-feature-tour` → CG operational verbs** — DONE, and it had a REAL
    DEFECT, not just a gap: the existing section documented
    `accumulator inject ticks '{...}'`, but the CLI requires
    `--event` (`nouns/accumulator/mod.rs:46-52`), so the command as written
    fails with an unexpected-argument error. Anyone following the example
    verbatim hit a wall. Fixed, and added the verbs that were missing entirely —
    `graph list/status/accumulators`, `reactor fire --input source=<json>`
    (full-replace, omitted sources are CLEARED), and `force-fire`, with the
    distinction between the two spelled out.

  * **Trigger verbs — NOT blocked on migrating `event-triggers`.**
    `event-triggers` is indeed embedded (`src/main.rs`, no `package.toml`), but
    `packaged-triggers` already exists and IS packaged — it simply had no
    README at all. That is the right home, so no migration was needed.

    The REAL gap was the CLI: `cloacinactl trigger` had only `list`/`inspect`
    while pause/resume/fire existed as server routes with no CLI in front of
    them. Added all three (thin wrappers over the existing routes, `--event`
    following the same parse-JSON-or-treat-as-string convention as
    `accumulator inject`), and wrote `packaged-triggers/README.md` with Run it +
    Operate it.

  * **Tenant / key / fleet — the ticket's premise is superseded.** It wants
    `multi-tenant` migrated so it teaches the server-side story instead of the
    embedded `DatabaseAdmin`. But that story is ALREADY taught, and in a better
    place: `docs/content/service/how-to/configure-multi-tenant-deployment.md`
    covers `tenant create`, `key create` and profiles, with a companion
    `decommission-a-tenant` how-to. Tenant lifecycle is server administration,
    not a workflow feature — a workflow example is the wrong home for it.
    **DROPPED (maintainer decision, 2026-08-09).** Not migrating an example to
    duplicate docs that already exist and already live in the better place. The
    tenant/key/fleet verbs are out of this ticket's scope; if those docs are
    later found lacking, that is a service-docs ticket, not an example
    migration.

- 2026-08-10 — COMPLETED. PR #251 merged (squash).

  Every documented verb is now RUN by the demos lanes, not just written down.
  Verified live before merge:

      ok: graph list / graph accumulators
      ok: accumulator inject --event
      ok: reactor fire --input / reactor force-fire
      ok: trigger list / inspect / pause / resume
      ok: execution events / execution events --since

  `execution events` is asserted on the DEFAULT lane, so every auto-discovered
  packaged example exercises it rather than only the one whose README documents
  it. `--follow` is deliberately excluded — it streams SSE until Ctrl-C and
  would hang a harness step instead of checking anything; the reasoning is
  inline so the "gap" is not later closed into a wedged lane.

  THREE BROKEN VERBS, found purely by running what the docs promise:
  1. `accumulator inject` — documented as `inject <name> '<json>'` when the CLI
     requires `--event`. The README command could not work.
  2. `trigger fire` — resolves targets from the SUBSCRIPTION side only, so it
     cannot fire an `on = ".."` trigger. Filed [[CLOACI-T-0929]]. This was a
     false claim in a README written in the same PR and caught minutes later by
     the assertion — the mechanism working as intended.
  3. `execution events` — **broken outright, 100% of the time**. The server
     returned every event correctly; the CLI handed the whole
     `ExecutionEventsResponse` envelope to `render::list`, which understands
     only `items` or a bare array. A shipped operator verb that appears never to
     have worked.

  That third one sharpens this ticket's own thesis. T-0893 says the operational
  surface is "taught nowhere"; the real finding is **taught nowhere AND
  THEREFORE NEVER RUN** — which is precisely how a wholly broken command
  survives in a shipped CLI.

  ALSO SHIPPED: `cloacinactl trigger pause|resume|fire` (they existed as server
  routes with no CLI in front of them), and `packaged-triggers/README.md` — the
  packaged home for the trigger story had no README at all.

  TWO LATENT HARNESS BUGS fixed en route: `_graph_kafka_steps` and
  `_trigger_wait_steps` each returned on success from INSIDE their poll loops,
  so appended steps would be silently skipped — green without running. Same
  shape twice; worth a sweep of the rest of that file.