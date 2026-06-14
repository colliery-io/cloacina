---
id: unify-demo-and-soak-onto-shared
level: task
title: "Unify demo and soak onto shared fixtures + infra (two sources of truth today)"
short_code: "CLOACI-T-0675"
created_at: 2026-06-13T16:49:29.641791+00:00
updated_at: 2026-06-13T17:23:04.323962+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Unify demo and soak onto shared fixtures + infra (two sources of truth today)

## Objective **[REQUIRED]**

The demo stack and the server soak exercise the same platform but through
**entirely separate code + infra**, so they drift, double the maintenance, and
collide (e.g. both want host Postgres:5432). Unify them onto one set of fixtures,
one packing path, and one parameterized stack bring-up — "demo" and "soak" become
two profiles over shared infra, not two implementations.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

{Delete this section when task is assigned to an initiative}

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

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
- **Current Problems** (two parallel implementations):
  - **Fixtures / packages — two sources of truth.** Demo uses real package dirs
    under `examples/fixtures/*` packed by `docker/pack-demo-fixtures.sh` +
    `cloacinactl`. Soak **hand-generates** packages inline in
    `.angreal/test/soak/server.py` (`create_python_source_package`,
    `create_cg_source_package`, `create_python_cg_source_package`,
    `create_python_kafka_cg_source_package`, …), tarring archives by hand. The
    same workflow/CG/Python shapes are defined twice and drift independently.
  - **Infra — two stacks.** Demo: `docker/docker-compose.demo.yml`
    (server + compiler + ui + postgres + kafka, cloaca baked into images, fixtures
    + seed via `ui/harness/src/main.mjs`). Soak: host `cargo build` binaries +
    `.angreal/docker-compose.yaml` (postgres only) + ad-hoc Kafka detection.
  - **Collisions / env coupling.** Both bind host **Postgres:5432**, so the demo
    must be torn down before a soak (hit this 2026-06-13). cloaca is baked into
    demo images but the soak depends on host-installed cloaca — Python steps
    warn-and-skip when it's absent, silently reducing coverage.
  - **Packing/loading paths differ** (cloacinactl pack vs hand-rolled tar), so a
    bug in one path isn't caught by the other.
- **Benefits of Fixing**: one place to add/maintain demo+soak workflows; the soak
  exercises the *real* packaging+image path users hit; no port juggling; Python
  coverage is guaranteed (shared cloaca-bearing image); the T-0674 regression
  assertions run against the same artifacts the demo shows.
- **Risk Assessment**: low-urgency but compounding — every new fixture/primitive
  (triggers, reactors, CG variants) currently has to be added in both places or
  silently diverges; soak gaps (skipped Python) hide real regressions.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] One canonical fixture set (`examples/fixtures/*`) is the only source — the
      soak's inline `create_*_source_package` generators are **removed**; the soak
      uses the demo-seeded fixtures.
- [x] One packing path — the soak relies on the demo compose's `fixtures` service
      (`pack-demo-fixtures.sh` + cloacinactl); no hand-rolled tar.
- [x] One parameterized stack: the soak drives `docker/docker-compose.demo.yml`
      (`--minutes` for duration); cloaca is in-image (Python never skips); no host
      5432 collision (Postgres is in-container).
- [x] The T-0674 regression assertions run against the shared artifacts (soak
      Step 3: demo-py-workflow tasks + mixed_graph/demo_py_graph topology).
- [x] `angreal test soak server` and the demo share bootstrap + seed (same compose
      `harness` seed + `clk_demo_bootstrap_key_0001`).
- [ ] **Follow-up (separate ticket):** add a Kafka service to a soak compose
      profile + restore the stream/batch-accumulator load (the demo compose has no
      broker, so Kafka load is not exercised here).

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
Likely shape (decide during design):
1. **Shared fixtures**: promote the demo `examples/fixtures/*` set to the single
   catalog. Add any soak-only shapes (kafka stream/batch CG variants) as real
   fixture dirs. Delete the inline generators in `soak/server.py`.
2. **Shared packing + seed**: both call `pack-demo-fixtures.sh` (or a small
   `cloacinactl`-based packer) and a shared seed routine (factor `ui/harness`'s
   upload/wait-for-load logic into something both the demo seed and the soak can
   call).
3. **Shared stack**: drive the soak off `docker/docker-compose.demo.yml` with a
   compose profile / env (e.g. `SOAK=1` to skip the UI, bound the run, tune
   intervals) instead of host binaries + a separate postgres compose. This gets
   cloaca-in-image for free (no host cloaca dependency / silent skips) and removes
   the 5432 collision. Soak then layers its load loop + duration on top.
4. The soak's Step 9 load loop + `--minutes` + T-0674 assertions stay; only the
   setup/fixtures/infra below them are swapped to the shared path.

### Dependencies
- Touches `docker/docker-compose.demo.yml`, `docker/pack-demo-fixtures.sh`,
  `ui/harness/src/main.mjs`, `.angreal/test/soak/server.py`, `examples/fixtures/*`.
- Related: CLOACI-T-0674 (soak regression assertions ride on the shared artifacts).

### Risk Considerations
- Soak historically runs against host binaries for fast iteration; moving to the
  compose image adds a build step — mitigate with image caching / a `--host-binary`
  escape hatch.
- Kafka: the demo compose may not include Kafka; the soak's kafka steps need a
  broker — add it to a soak profile.
- Keep a single bootstrap key + tenant convention across both.

## Status Updates **[REQUIRED]**

**2026-06-13 — Doing it this release. Concrete plan + findings.**
A 30-min soak on the OLD infra confirmed it's broken against current code: no
compiler started (Rust packages stuck `pending`), and the hand-rolled CG
`package.toml` is rejected (`unknown field 'package_type'`) → 0 packages loaded.
Killed it. Demo compose has postgres + server(:8080) + compiler(:9000) +
ui(:8082) + fixtures(pack) + harness(seed); key `clk_demo_bootstrap_key_0001`;
**no Kafka**. Load-loop demo targets: Rust `demo_slow_workflow`, Python
`demo_py_workflow`, CGs `mixed_graph`→`alpha` / `demo_py_graph`→`py_alpha` (WS
workers already target alpha/py_alpha).

Plan — rewrite `soak/server.py::server()`:
1. setup → `docker compose -f docker/docker-compose.demo.yml up -d` (+build),
   wait `/health`, poll `/workflows` until seeded fixtures are `success`.
2. retarget consts → localhost:8080, demo key, demo workflow names; crash-check
   via container health, not server_proc.
3. gate Kafka off (demo has none) — FOLLOW-UP: kafka soak compose profile.
4. keep Step 9 loop + `--minutes` + 10s heartbeat + T-0674 assertions (now vs
   demo-py-workflow tasks + demo_py_graph topology).
5. teardown `docker compose down` (keep volumes); delete dead inline generators +
   build_server/postgres helpers.
Verify with `--minutes 1` against the demo stack before done.

**2026-06-13 — DONE (core unification; Kafka deferred).** Rewrote
`soak/server.py` to drive `docker/docker-compose.demo.yml`: brings the stack up
(build + seed), waits for the seeded `examples/fixtures/*` to load + the CGs to
register, runs the T-0674 assertions (Step 3), then sustains concurrent load
(Rust + Python executions + WS market-data into alpha/py_alpha) for `--minutes`.
Deleted ~600 lines of inline `create_*_source_package` generators +
build_server/own-Postgres helpers. Needed a CG-registration wait (`/health/graphs`
lags package build after a container recreate — first run 404'd on `mixed_graph`).
Committed `36c55362`. Verified `--minutes 1`: 81/81 Rust + 48/48 Python execs,
~13k WS events, **0 api/connection errors**, server healthy. The demo and soak
now share fixtures + compiler + cloaca + bootstrap. Kafka load is the one
remaining follow-up (demo compose has no broker).