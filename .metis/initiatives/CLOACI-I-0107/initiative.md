---
id: audit-cli-server-contract-and-add
level: initiative
title: "Audit CLI/server contract and add integration tests for every CLI verb"
short_code: "CLOACI-I-0107"
created_at: 2026-05-06T11:05:35.281696+00:00
updated_at: 2026-05-06T11:05:35.281696+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: audit-cli-server-contract-and-add
---

# Audit CLI/server contract and add integration tests for every CLI verb Initiative

## Context

The May 2026 review found that several published `cloacinactl` commands are non-functional in the current default deployment because no test exercises the CLI/server seam:

- `cloacinactl tenant create <name>` posts a body shape (`{schema_name, username, password}`) the server doesn't accept (server expects `{name, description, password?}`). Every CLI tenant-create has been failing.
- `cloacinactl execution list --status Failed --workflow_name foo` silently ignores both filters — it returns *only active* executions because the route's DAL call discards the query string.
- `cloacinactl tenant list`, `trigger list`, and `execution list` render empty for response shapes that don't match a hardcoded key — the renderer expects `body.<key>` and silently falls back to `body` if missing, which makes the empty-default render swallow real data.
- `cloacinactl package pack --sign <key>` accepts a flag and silently does nothing — `eprintln!` to stderr, no error.

These are operator-facing contract bugs. They've shipped because no integration test runs `cloacinactl` against a real server. The CLI/server seam is currently untested; unit tests on each side pass independently while the seam silently breaks.

## Goals & Non-Goals

**Goals:**
- Fix the four broken commands so each works end-to-end against a real server.
- Unify the REST error envelope so every server endpoint emits the same `ApiError` shape; CLI consumes that shape consistently.
- Plumb pagination on `list_triggers` and `get_trigger`.
- Resolve `--follow`: implement SSE streaming or hide the flag.
- Build a CLI integration test harness that spawns a real server (test fixture, temp Postgres or SQLite) and exercises every CLI verb end-to-end.

**Non-Goals:**
- Reworking the CLI noun-verb structure (CLOACI-I-0098 stands).
- Adding new commands. This initiative audits and tests existing surface only.
- Adding CLI commands for operator workflows that don't exist server-side. New verbs are tracked elsewhere (REC-10).

## Source Findings (May 2026 review)

- **API-01 (Critical)** — `tenant create` body-shape mismatch.
- **API-02 (Critical)** — `execution list` filters silently ignored.
- **API-03 (Critical-adjusted)** — Empty-render bug across `tenant list`, `trigger list`, `execution list`.
- **API-05 (Major)** — `--sign` flag silently ignored.
- **API-06 (Major)** — Three different REST error envelope shapes.
- **API-08 (Major)** — Broken router prefix invariant for `/v1/health` + `/v1/ws` (one Axum nest is bypassed).
- **API-10 (Major)** — Hardcoded pagination caps; no client-side limit/offset.
- **API-17 (Minor)** — `--follow` always-errors flag.

## Discovery Questions

- **Test fixture model**: real server bound to a temp Postgres (heaviest, most realistic) or a faster in-process server with shared SQLite (lighter, slightly less realistic)? Existing angreal test surface already has both patterns — pick consistent with prior investments (CLOACI-T-0518).
- **Resolution choice on API-01**: change CLI body shape to match server, or rename the server's request struct? Recommendation is to match CLI's user-friendly `{name, description, password?}` shape; confirm with API consumer expectations.
- **`--sign`**: implement now (existing infrastructure can produce a `<archive>.sig` sidecar) or fail-hard until I-0103 lands? Coupling consideration with sig-verification work.
- **`--follow`**: SSE streaming is a bounded lift (Postgres LISTEN/NOTIFY backed). Implement now or hide the flag?
- **Test coverage exit-bar**: every verb of every noun? Every error path? Smoke tests vs full coverage?

## Initial Sketch

- Days 1–3: fix four broken commands. Each is a bounded patch, parallelizable.
- Days 4–5: unify error envelope; reroute `/v1/health/*` and `/v1/ws/*` into proper `/v1/` nest.
- Days 6–10: build CLI integration test harness; cover every verb of every noun.
- Days 10+: pagination plumb-through; `--follow` decision.

The integration test harness is the load-bearing investment. Plan it for reuse beyond this initiative — future CLI additions should automatically get an integration test.

## Acceptance Criteria

- All four broken CLI commands work end-to-end against a real server.
- `cloacinactl execution list --status Failed --limit 10` returns up to 10 Failed executions.
- `cloacinactl tenant list` shows tenants in the configured output format.
- `--sign` either signs or errors hard.
- `--follow` either streams or is hidden.
- Every CLI verb in `cloacinactl tree` has at least one integration test that exercises the command end-to-end against a real server fixture.
- All server endpoints emit `ApiError` shape; CLI's `extract_message` reads the canonical format.

## References

- `review/05-api-design.md` — API-01, API-02, API-03, API-05, API-06, API-08, API-10, API-17
- `review/10-recommendations.md` — REC-04
- Prior task: CLOACI-T-0518 (End-to-end CLI integration tests against a running server fixture, completed) — extend its harness coverage.
- Prior initiative: CLOACI-I-0098 (cloacinactl CLI Redesign and Rebuild, completed)
