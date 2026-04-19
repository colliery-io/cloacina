---
id: audit-reactor-vs-computation-graph
level: task
title: "Audit reactor vs computation_graph naming drift in core + server"
short_code: "CLOACI-T-0528"
created_at: 2026-04-18T16:32:39.189020+00:00
updated_at: 2026-04-18T16:32:39.189020+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Audit reactor vs computation_graph naming drift in core + server

## Objective

The codebase uses two distinct terms for what is conceptually a two-layer
model, and the two are not consistently applied. Sweep the internals for
drift and align names to the intended semantics.

**The model:**
- **Computation graph** — the spec. A typed DAG: nodes, edges,
  accumulator definitions, trigger rules. Pure data / structure.
- **Reactor** — the runtime that instantiates and runs a computation
  graph. Has health, pause state, current accumulator values,
  scheduler state.

The CLI already uses this distinction correctly (`cloacinactl reactor
list` = runtime observability). Parts of core and the server do too
(`Reactor`, `ReactiveScheduler`, `/v1/health/reactors`). Other parts
use `graph` / `computation_graph` where they actually mean reactor
state, and vice versa.

## Technical Debt Impact

- **Current problems**: Operators and contributors can't tell at a
  glance whether a symbol refers to spec or runtime. New code picks
  whichever term the author saw most recently. API responses mix both
  names in one payload.
- **Benefits of fixing**: One canonical term per layer. Easier to
  onboard, easier to grep, future endpoints pick the right word by
  default.
- **Risk if deferred**: Every new endpoint / DAL method / config knob
  compounds the drift. Renames get more expensive as more external
  consumers depend on the ambiguous names.

## Acceptance Criteria

- [ ] Grep sweep: every `graph` / `Graph` / `computation_graph` symbol
  in `crates/cloacina/src` + `crates/cloacina-server/src` is
  classified as either spec or runtime, and renamed if wrong.
- [ ] Same sweep for public HTTP routes, response field names, and
  config keys.
- [ ] DAL tables / migrations audited — some are correctly
  `computation_graph_state_*` (spec-level state), some may be
  mislabeled. Don't rename migrations already applied; document
  any drift instead.
- [ ] Doc pass on `docs/operations/` and `docs/content/` to use the
  terms consistently.
- [ ] Short README / CLAUDE.md note explaining the two-layer model so
  future contributors don't re-introduce drift.

## Implementation Notes

### Scope

Read-only audit first: produce a table of every identifier, its
current name, and the proposed correct name. Review the table before
starting rename PRs — some cases will be genuinely ambiguous and worth
discussing.

### Suggested rename heuristic

- Returns / touches live `ReactiveScheduler` state → `reactor`.
- Describes / parses / validates a DAG definition → `computation_graph`
  or `graph` (short form).
- Persists state that belongs to the running instance (accumulator
  buffers, dirty flags, last-tick timestamp) → `reactor` in the type
  name; DB columns can keep `computation_graph_state_*` if the schema
  intent is "state for a loaded graph".

### Don't rename

- Existing migration directories (breaks replay history).
- `computation_graph/` module path in `cloacina` core — the module
  owns both the spec types and the reactor. Renaming the module would
  be a bigger initiative.

### Dependencies

None. This is a standalone cleanup; no initiative ties.

## Status Updates

*To be added during implementation*
