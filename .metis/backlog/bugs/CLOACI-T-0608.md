---
id: fix-sqlite-memory-i-0100-reactor
level: task
title: "Fix sqlite :memory: + I-0100 reactor poll race; re-enable tutorials 03/04 on postgres lane"
short_code: "CLOACI-T-0608"
created_at: 2026-05-16T00:53:30.076692+00:00
updated_at: 2026-05-16T03:47:49.299900+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Fix sqlite :memory: + I-0100 reactor poll race; re-enable tutorials 03/04 on postgres lane

## Type
Bug

## Priority
P2 — Medium. Test surface is intact via sqlite lane; postgres lane is dark for two tutorials.

## Impact
- **Affected users**: anyone building on cloacina with `sqlite://:memory:` who also has the unified scheduler's reactor poller active (i.e. anyone on 0.7+). Tutorial 03 + 04 trip it deterministically on CI.
- **Reproduction (CI)**:
  1. `cargo build --release -p cloacina` with default features (which include the reactor poll loop wired in I-0100/T-0599).
  2. `python -m cloaca` against `sqlite://:memory:`.
  3. Trigger a workflow execution while the scheduler tick is mid-poll.
  4. Get `Database error: no such table: workflow_executions` from the dispatcher even though `migrations applied` was logged at startup.
- **Expected vs actual**: dispatcher/scheduler queries should always see the post-migration schema. Actual: under sqlite's pool-of-1, the new reactor-poll work in `cron_trigger_scheduler.rs` (see T-0599) takes the connection at startup, and there's some path where the connection that runs migrations is not the same as the connection the dispatcher checks `workflow_executions` on. Locally this passes; CI is the consistent repro.

## Workaround in place
Tutorials 03 and 04 are excluded from the postgres lane in
`.github/workflows/examples-docs.yml` (commit `ee3e6bb1`). Sqlite lane still covers them.

## Acceptance criteria
- [ ] Root-cause the connection-vs-migration mismatch under
      `sqlite::memory:` + I-0100's poll loop. Candidates: deadpool
      recycle behaviour, a separate Database instance opened by a
      subsystem, or pool-of-1 contention during startup.
- [ ] Decide between fixes:
      - Force `cache=shared` for `:memory:` URLs in `Database::build_sqlite_url`.
      - Single shared connection (no pool) for `:memory:` only.
      - Defer reactor-poll loop start until first non-migration tick.
- [ ] Re-enable the two matrix entries in `examples-docs.yml`:
      ```yaml
      - tutorial: "03"
        backend: postgres
      - tutorial: "04"
        backend: postgres
      ```
- [ ] CI green on a PR that re-enables them.

## Notes
- Local + sqlite-lane CI pass cleanly, so the race needs the postgres
  lane's docker-compose contention to surface reliably.
- The `cloacina::cron_trigger_scheduler` reactor-poll branch landed in
  `83567958` (I-0100 / T-0599). Bisecting onto main HEAD prior to that
  commit should confirm.
- Logs from the broken run are in run `25944656498` (job 76270358886).

## Related
- [[CLOACI-I-0100]] (origin)
- [[CLOACI-T-0599]] (the change that exposed the race)

## Status Updates

**2026-05-16** — Root-caused: with diesel's sqlite open path (no
`SQLITE_OPEN_URI`), every `:memory:` connection gets its own private
database. `max_size = 1` kept it latent; I-0100's new reactor poll
loop made concurrent connection-use detectable.

Fix landed: substitute `:memory:` requests for a per-Database
`NamedTempFile` on disk, wrapped in `Arc` so every Database clone
keeps the file alive and the last drop deletes it. Pool connections
all open the same real file → real sharing, no URI gotchas.

`file::memory:?cache=shared` was considered but rejected: it requires
`SQLITE_OPEN_URI` which diesel doesn't set, so it silently creates a
file literally named `:memory:` in CWD without any shared cache.

Postgres lane re-enabled for tutorials 03 + 04 in same PR.

Unit tests added:
- `test_sqlite_connection_strings_passthrough` (file paths + sqlite://
  prefix stripping)
- `test_sqlite_memory_substitutes_tempfile` (substitution + cleanup on
  owner drop, both `:memory:` and `sqlite://:memory:` inputs)