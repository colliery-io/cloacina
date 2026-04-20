---
id: defensive-practice-lint-ci-guard
level: task
title: "Defensive practice — lint/CI guard against credential logging in log and print statements"
short_code: "CLOACI-T-0443"
created_at: 2026-04-08T13:43:25.906031+00:00
updated_at: 2026-04-20T11:21:27.802952+00:00
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

# Defensive practice — lint/CI guard against credential logging in log and print statements

*Origin: Architecture review OPS-03, follow-up from T-0442*

## Objective

Establish a defensive practice that prevents credential leakage from recurring in log and print statements. T-0442 fixes the immediate instance, but without a systemic guard this class of bug will return.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [ ] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**: Database URLs with embedded passwords can be logged via `info!()`, `debug!()`, or `eprintln!()` anywhere in the codebase. The only protection is the developer remembering to call `mask_db_url()` — a manual practice that already failed once (Python bindings).
- **Benefits of Fixing**: Systemic prevention of credential leakage. New code paths automatically protected.
- **Risk Assessment**: Without this, every new module that touches database URLs is a potential leak.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] A `SensitiveString` or `MaskedUrl` newtype wraps database URLs, with `Display`/`Debug` impls that mask credentials automatically
- [ ] All `info!()`/`debug!()` log calls that format a database URL go through the newtype — no raw URL strings in log output
- [ ] OR: a clippy lint / CI grep check that flags `database_url` appearing in `info!()`, `debug!()`, `eprintln!()`, or `println!()` macro invocations
- [ ] OR: a pre-commit hook that greps for raw URL logging patterns
- [ ] At least one of the above approaches is implemented and running in CI

## Implementation Notes

### Approaches (pick one or combine)

**Option A — Newtype wrapper** (strongest):
Create `pub struct DatabaseUrl(String)` with `impl Display` that masks the password. Change `Database::new()` and `DefaultRunner::new()` to accept `DatabaseUrl` instead of `&str`. Compile-time enforcement — you can't accidentally format the raw string.

**Option B — CI grep guard** (quickest):
Add a CI step that greps for patterns like `eprintln!.*database_url`, `info!.*database_url`, etc. and fails the build if found. Brittle but fast to implement.

**Option C — Custom clippy lint** (most Rust-native):
Write a clippy lint that flags tracing macro invocations containing variables named `*url*` or `*password*`. Higher upfront cost but zero ongoing maintenance.

### Dependencies
- T-0442 (immediate fix) should land first — this is the systemic follow-up.

## Status Updates

- **2026-04-20**: Implemented Option B (CI grep guard).
  - Added `scripts/check_credential_logging.py`: scans git-tracked `*.rs` files, parses `info!/debug!/trace!/warn!/error!/eprintln!/println!/eprint!/print!` invocations (multi-line aware with string and char-literal skipping), and flags any invocation whose body references `database_url`, `db_url`, `connection_string`, or `password` **after** stripping (a) string literals, (b) any `mask_*(...)` helper call (e.g. `mask_db_url`, `mask_password`), and (c) lines preceded by `// allow(credential-logging): <reason>`.
  - Wired into CI: new step in `.github/workflows/ci.yml` `quick-checks` job runs `python3 scripts/check_credential_logging.py` alongside fmt/clippy.
  - Wired into angreal: new `check credential-logging` command in `.angreal/task_check.py` for local/pre-commit runs.
  - Fixed existing violations the guard surfaced:
    - `crates/cloacina/tests/fixtures.rs`: Postgres and SQLite `new_*` logged raw `db_url`; routed through `mask_db_url()`.
    - `examples/features/workflows/per-tenant-credentials/src/main.rs:76` logged admin-provided password cleartext; wrapped in `mask_password()` like its siblings.
  - Verified detection with 8 synthetic cases (raw/masked/qualified/literal-only/allow-comment variants) — all pass.
  - Final state: guard is clean across 392 Rust files; `cargo check` remains clean for `crates/cloacina` (sqlite+tests) and the per-tenant-credentials example.
