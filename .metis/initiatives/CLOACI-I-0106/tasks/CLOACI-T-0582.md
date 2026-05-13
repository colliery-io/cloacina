---
id: t-05-set-search-path-fail-closed
level: task
title: "T-05: SET search_path fail-closed + current_schemas() defense-in-depth"
short_code: "CLOACI-T-0582"
created_at: 2026-05-13T19:38:45.083326+00:00
updated_at: 2026-05-13T19:38:45.083326+00:00
parent: CLOACI-I-0106
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0106
---

# T-05: SET search_path fail-closed + current_schemas() defense-in-depth

## Parent Initiative

[[CLOACI-I-0106]]

## Objective

Today `SET search_path` failure during connection acquisition silently routes the next query to `public`. Make this fail-closed: propagate the error to the caller rather than masking. Add a `current_schemas()` defense-in-depth check on the first query after acquire so even a successful-but-wrong SET is caught. Closes COR-01 — Critical.

## Acceptance Criteria

- [ ] In the tenant-scoped connection-acquire path (likely `TenantDatabaseCache` or `with_tenant_database`), the result of `SET search_path TO <tenant_schema>` is checked. On failure: propagate as `DatabaseError::SearchPathSetFailed { tenant_id, source }` to the caller; do **not** return the connection to the pool for general use.
- [ ] After a successful SET, run `SELECT current_schemas(false)` on first query (or cache the result per-connection on acquire). Assert the expected tenant schema is in the search path. On mismatch: error with `DatabaseError::SearchPathMismatch { expected, actual, tenant_id }`.
- [ ] The defense-in-depth check is configurable via `--strict-search-path` (env `CLOACINA_STRICT_SEARCH_PATH`), default **on** in server mode. Daemon mode keeps it off (single-tenant; the check adds latency without value).
- [ ] Audit emit on either failure: `audit::log_search_path_failure(tenant_id, kind, error)`. Mismatch and SET-failure are distinct enough that operators want them distinguishable.
- [ ] Integration test (fault injection): a fixture drops `tenant_X`'s schema mid-acquisition; the in-flight request errors cleanly rather than silently writing to `public`.
- [ ] Integration test (legacy compat): `--strict-search-path=false` preserves the pre-fix behavior; the defense-in-depth check is bypassed but the fail-closed SET handling stays (you can't opt out of correctness).
- [ ] **Test harness updated as we go**: any existing test that implicitly relied on the silent-fallback behavior (running queries without setting `search_path` explicitly) needs to be reframed. Run `angreal test integration` after each handler/DAL path is updated — don't batch.

## Test Cases

- **TC-1 (happy path):** regular tenant request, SET succeeds, defense-in-depth check passes, query runs in tenant schema. No measurable extra latency under load.
- **TC-2 (SET failure):** simulated DB error during SET (e.g. permission denied on the tenant schema). Caller receives `DatabaseError::SearchPathSetFailed`, not a query result against `public`.
- **TC-3 (schema dropped mid-request):** drop `tenant_X`'s schema while a request is using it. Next query errors with `SearchPathMismatch` (or `SearchPathSetFailed` depending on timing).
- **TC-4 (strict=off):** with `--strict-search-path=false`, the defense-in-depth check is skipped; SET-failure still fails closed.
- **TC-5 (daemon path):** daemon doesn't construct tenant-scoped connections; this code path is no-op. Confirm no regression.

## Implementation Notes

### Technical Approach

- File: `crates/cloacina/src/dal/tenant_database_cache.rs` (verify path).
- The current code probably calls `conn.execute("SET search_path TO ...")` and discards the result. Fix is straightforward — `?`-propagate.
- For the defense-in-depth check: run `SELECT current_schemas(false)` once per connection acquire (not per query). Returns `text[]`; check that the expected tenant schema is at position 0.
- Caching the check result: store the verified schema name on the connection wrapper struct; subsequent queries on the same checked-out connection skip the check.
- Strict flag plumbed through `CompilerConfig`-equivalent config struct on the server's `AppState`.

### Dependencies

- Independent of T-0580/T-0581. Can land in parallel with T-0580; if both ship in the same week, the second one rebases on the first.
- T-0578's enriched spans help debug failures during this task but aren't a hard dep.

### Risk Considerations

- **Test fallout:** existing tests that bypassed `with_tenant_database` and ran queries with the connection's default search_path (usually `public` + the admin schema) will now error with `SearchPathMismatch`. Triage: most are likely incorrect assertions about admin-context behavior that should be expressed as tenant-scoped. Reframe one file at a time.
- **Perf concern:** an extra `SELECT current_schemas(false)` per connection acquire adds a sub-ms round-trip. At server-scale (hundreds of req/s, many short-lived connections), measurable. Mitigation: cache the check result per connection-checkout, so the cost is amortized.
- **Daemon path opt-out:** make sure the daemon doesn't accidentally pick up strict mode. Cleanest is to default the flag based on binary identity (`cloacina-server` defaults strict=on, `cloacinactl daemon` defaults strict=off), not via a shared default constant.

## Status Updates

*To be added during implementation.*
