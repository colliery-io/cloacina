---
id: seed-demo-harness-exercise-every
level: task
title: "Seed/demo harness: exercise every UI surface — triggers, computation graphs/reactors, and Python packages"
short_code: "CLOACI-T-0664"
created_at: 2026-06-12T02:18:00+00:00
updated_at: 2026-07-05T18:00:18.465618+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Seed/demo harness: exercise every UI surface — triggers, computation graphs/reactors, and Python packages

## Objective **[REQUIRED]**

The T-0660 seed/demo harness only uploads two plain Rust task-workflows
(`demo-slow-rust`, `demo-fail-rust`), so the UI's **Triggers**, **Graphs**, and
**Accumulators** views are legitimately empty and the **Python** path is never
exercised. Extend the harness/fixtures so the demo lights up every UI surface,
including Python-defined packages — and run the whole thing in **Docker** (the
demo compose), not host processes.

## Backlog Item Details **[CONDITIONAL: Backlog Item]**

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification **[CONDITIONAL: Feature]**
- **User Value**: A demo where you can actually *see* triggers firing on a
  schedule, a computation graph + its accumulators, and both Rust and Python
  packages — the full control plane, not just task workflows.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A **trigger** fixture (code-declared `#[trigger]`/cron) so the Triggers
      view shows a schedule and runs auto-fire. (`examples/fixtures/mixed-rust`
      already bundles reactor + reactor-bound CG + trigger + workflow — reuse it.)
- [ ] A **computation-graph** fixture so Graphs + Accumulators populate. The
      Python `examples/features/computation-graphs/python-packaged-graph`
      (`market_maker`: reactor + 2 accumulators + CG) covers CG **and** Python.
- [ ] A **Python** package in the set (the CG above, and/or a Python task
      workflow) — built as a bzip2-tar `.cloacina` directly (no `cargo`), the way
      the soak harness does (`create_python_test_package`).
- [ ] The harness uploads them; the demo **runs in Docker** via
      `docker/docker-compose.demo.yml` (extend the fixtures-packer to emit the
      Rust + Python archives into the shared volume).
- [ ] Verify against a live stack what actually populates each view; file any
      server-side gaps separately.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
- Python `.cloacina` = bzip2 tar of `package.toml` (`language="python"`,
  `entry_module`) + the module tree. No Cargo.toml/cargo. (See CLOACI-T-0665 —
  `cloacinactl package pack` should learn this so we stop hand-tarring.)
- Triggers/CG/accumulators are **code-declared**, discovered at build:
  `#[reactor]`, `#[computation_graph(trigger = reactor(...))]`, `#[trigger(...)]`,
  `#[workflow(triggers=[...])]` (Rust) or the `@cloaca.reactor/@cloaca.graph`
  decorators (Python).
- Extend `docker/Dockerfile.fixtures` / `pack-demo-fixtures.sh` to also emit the
  trigger + CG + Python archives, and add them to the harness upload set.

### Risk Considerations / Open empirical questions
The running `cloacina-server` may have gaps loading these (prior notes flagged
"Python CG routing", "cloaca module", and cron wiring). **Probe (2026-06-11),
partial — Docker Desktop crashed mid-probe:**
- A hand-tarred Python **computation-graph** package (`python-packaged-graph`)
  **uploaded OK (HTTP 201)** against `cloacina-server`. Build/registration not
  yet confirmed (postgres died with Docker before the compiler finished).
- `[[metadata.triggers]]` in `package.toml` is **rejected** by the server
  manifest schema → triggers must be code-declared, not manifest-declared.
- Reinforces running the stack in Docker (the host `ui up` server fell over when
  the Docker postgres died): use `docker/docker-compose.demo.yml`.

## Status Updates **[REQUIRED]**

**2026-06-11 — Filed + initial probe.** See the empirical findings above.

**2026-06-11 — Found the proven reuse: `angreal test soak server`.**
`.angreal/test/soak/server.py` already uploads CG/reactor/Python packages to a
live `cloacina-server`, waits for the compiler, and asserts `/v1/health/reactors`
+ `/v1/health/accumulators` populate and Python workflows execute. Reusable
inline package builders (no examples/ path coupling):
- `create_cg_source_package()` — Rust `#[computation_graph]` + reactor.
- `create_python_cg_source_package()` — Python `@cloaca.reactor` + accumulator + CG.
- `create_python_source_package()` — Python `@cloaca.task` workflow.
- Compiler builds `language="python"` by skipping cargo + returning an empty
  artifact (`crates/cloacina-compiler/src/build.rs:217`); reconciler imports the
  Python module via PyO3 at load.

**Correction to the probe's worry:** the SERVER runs the cron scheduler — the
crash log showed `cloacina::cron_trigger_scheduler: Error processing triggers`
(firing; only failed because postgres died with Docker). So cron triggers
register *and* fire through the server, not only the daemon.

**Plan (run in Docker, reuse soak builders):**
1. Add demo fixtures reusing the soak builders' content: a Rust CG
   (`__WORKSPACE__`-templated Cargo.toml like the other demo fixtures — absolute
   paths resolve in the containerized compiler, vs the soak `../../../crates`), a
   Python CG, a Python task workflow, and a cron-trigger workflow.
2. Extend `docker/pack-demo-fixtures.sh`: Rust via `cloacinactl package pack`
   (workspace rewrite); Python via direct bzip2-tar (no cargo) — the soak method.
3. Harness already uploads every `.cloacina` in /packages → no harness change.
4. `docker compose -f docker/docker-compose.demo.yml up --build` → verify
   Workflows + Executions + Triggers + Graphs + Accumulators populate (Rust +
   Python); file any view that doesn't.

### 2026-07-05 — CLOSING: the plan was delivered incrementally across weeks; the two residual gaps closed today (branch feat/t0664-demo-surface-gaps, commit 396c0208)
The fixture set grew far past the original ask — the demo now packs and seeds: cron trigger (demo-cron-rust), branch/skip (demo-branch-rust), mixed reactor+accumulator+CG+trigger (mixed-rust), manual-trigger fan-out across two packages (demo-fanout-rust/-sub), acme multi-tenant, kafka stream CG, routing CG, complex DAG, Python task/CG/state/cron packages, and both constructor demos (Rust + Python) — all in Docker via pack-demo-fixtures.sh + the compose stack. A verification sweep (2026-07-05) found exactly two residual gaps, both closed today:
1. **Poll-trigger kind absent from the demo** — `demo-poll-rust` existed but was never packed. Added to pack-demo-fixtures.sh; verified live: `demo_poll_trigger` registered with `poll_interval_ms=30000` in `/v1/tenants/public/triggers`.
2. **The harness never FIRED the manual trigger** — seed mode now fires `settlement_close` once with a typed event (best-effort on cold stacks); verified live: `settle_ledger` execution Completed with `trigger_origin: "manual"` (the gold manual pill).
All five ACs met: trigger fixture(s) across all three kinds ✓ · CG fixtures (Rust + Python + kafka + routing) ✓ · Python packages (task/CG/state/cron) ✓ · Docker-native packing/upload ✓ · live-verified per view, with gaps filed separately as they surfaced (T-0744 gauges, T-0839 state-accumulator degradation — both since fixed). COMPLETE.
