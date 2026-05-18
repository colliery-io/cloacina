---
id: doc-b-platform-reference-refresh
level: task
title: "DOC-B: Platform reference refresh — CLI, HTTP API, configuration, env-vars, metrics catalog"
short_code: "CLOACI-T-0612"
created_at: 2026-05-18T18:19:20.604315+00:00
updated_at: 2026-05-18T18:19:20.604315+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0112
---

# DOC-B: Platform reference refresh — CLI, HTTP API, configuration, env-vars, metrics catalog

## Parent Initiative

[[CLOACI-I-0112]]

## Objective

Bring every reference doc in `docs/content/platform/reference/` up to current truth so downstream how-to and explanation clusters (DOC-C, DOC-D, DOC-F, DOC-I) can cross-link to a trustworthy lookup surface. Pin the unified `ApiError` envelope (I-0107), document the unmentioned server/compiler/daemon flags (I-0103, I-0106, I-0109), correct list-endpoint pagination, fix the metric-name drift (`pipelines` → `workflows`), and complete the May-2026 env-var coverage. Write `api-error-envelope.md` as a standalone reference per the locked Phase 2 decision.

## Scope

### Files in cluster (~12)

| File | Effort | Headline change |
|---|---|---|
| `platform/reference/cli.md` | M | Add server flags: `--verification-org-id`, `--reconcile-interval-s`, `--tenant-runner-cache-size`, `--tenant-deletion-drain-timeout-s`, `--log-retention-days`. Add `trigger list` `--limit`/`--offset`. Fix `execution list` default (100, not 50). Daemon `start` `--log-retention-days`. |
| `platform/reference/configuration.md` | S | Note T-0502 sole-recovery semantics on `enable_recovery`. Verify `FilesystemRegistryStorage`/`DatabaseRegistryStorage` enum. Cross-link metrics-catalog. |
| `platform/reference/database-admin.md` | S | Note `remove_tenant` is step-4 of HTTP route's 4-step orchestration (revoke keys → evict runner → evict DB → drop schema). |
| `platform/reference/environment-variables.md` | M | Replace old metric names (`cloacina_pipelines_total` → `cloacina_workflows_total` etc.). Add `CLOACINA_REQUIRE_SIGNATURES`, `CLOACINA_VERIFICATION_ORG_ID`, `CLOACINA_TENANT_RUNNER_CACHE_SIZE`, `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S`, all `CLOACINA_COMPILER_*` vars. Defer full metrics list to metrics-catalog. |
| `platform/reference/ffi-vtable.md` | S | Verify-only. |
| `platform/reference/http-api.md` | **L** | **Rewrite** for ApiError envelope (`{"error", "code"}` + `x-request-id`); fix `POST /v1/tenants` body (`{name, description?, password?}` not `{schema_name, username, password}`); add execution list params (`?status`, `?workflow`, `?limit`, `?offset`); add trigger list pagination; correct multi-tenancy caveats (T-0581 evicts caches; doc currently says they never evict); add SSE non-availability note for `--follow`. |
| `platform/reference/package-manifest.md` | S | Verify-only; spot-check for I-0102 schema additions (`reactors`, `graphs`, `triggerless_graphs` entries). |
| `platform/reference/package-shell-macro.md` | S | Verify-only. |
| `platform/reference/repository-structure.md` | M | List all 11 crates (current doc lists 6). Fix Python crate-split claim (T-0529 moved Python out of `cloacina` core). Refresh examples list against `examples/`. Complete features list (`auth`, `kafka`, `telemetry`, `extension-module`). |
| `platform/reference/websocket-protocol.md` | S | Verify-only. |
| `platform/reference/api-error-envelope.md` | **new (S)** | New file. Document envelope shape, `x-request-id` header, full code enumeration by route, HTTP status mapping, client retry guidance. |
| `platform/reference/metrics-catalog.md` | (created by DOC-H) | Cross-linked from this cluster but written by DOC-H. |

### Cross-cluster dependencies

- **Blocked by**: DOC-A (drift sweep — version pins and `cloacinactl` rename must land first), DOC-H (metrics-catalog.md must exist before this cluster's cross-links resolve)
- **Blocks**: DOC-C, DOC-D, DOC-F, DOC-I (these clusters cross-link into platform reference; reference must be correct first)

## Acceptance Criteria

- [ ] `platform/reference/cli.md` lists every flag exposed by `crates/cloacinactl/src/nouns/*/`, `crates/cloacina-server/src/main.rs`, and `crates/cloacina-compiler/src/main.rs`. Spot-check via diff vs `--help` output for each binary.
- [ ] `platform/reference/http-api.md` documents every endpoint under `crates/cloacina-server/src/routes/`. Every error response shows the `{"error", "code"}` envelope. Pagination params noted on every list endpoint.
- [ ] `platform/reference/api-error-envelope.md` exists, lists every error code emitted by `crates/cloacina-server/src/routes/error.rs` with the HTTP status it maps to.
- [ ] `platform/reference/environment-variables.md` includes every `CLOACINA_*` env consumed by server + compiler + daemon; metric names match emitted code (workflows, not pipelines).
- [ ] `platform/reference/repository-structure.md` matches `ls crates/` output (11 crates).
- [ ] All cross-links to `platform/reference/metrics-catalog.md` resolve.
- [ ] `angreal docs:build` passes.

## Implementation Notes

### Sources

- **Audit file**: `.metis/initiatives/CLOACI-I-0112/audit-platform.md` (see `### platform/reference/*` entries plus API-P-01..API-P-09 in the summary)
- **Code paths**:
  - CLI flags: `crates/cloacinactl/src/nouns/{compiler,daemon,execution,graph,key,package,server,tenant,trigger,workflow}/mod.rs` (+ each verb's `*.rs` file)
  - Server flags: `crates/cloacina-server/src/main.rs:39-90`
  - Compiler flags: `crates/cloacina-compiler/src/main.rs`
  - HTTP routes: `crates/cloacina-server/src/lib.rs:660-790`, `crates/cloacina-server/src/routes/{auth,executions,health_graphs,keys,tenants,triggers,workflows,ws,error}.rs`
  - ApiError: `crates/cloacina-server/src/routes/error.rs:30-100`
  - Manifest schema: `crates/cloacina/src/packaging/manifest_schema.rs`
  - Crate listing: `crates/` and `Cargo.toml`
- **Archived initiatives**: `CLOACI-I-0107` (CLI/server contract), `CLOACI-I-0098` (cloacinactl redesign), `CLOACI-I-0103` (signature verification), `CLOACI-I-0106` (multi-tenant), `CLOACI-I-0109` (compiler metrics, log retention)

### Approach

Reference docs are the **truth source** for downstream how-to / tutorials / explanation. Approach each file with a code-first cross-check: compare what the doc says against `cargo run -p <binary> -- --help` output, route handlers, struct definitions, and inventory entries. Any prose statement that doesn't have a code citation gets stripped or rewritten.

Recommended ordering within the cluster:
1. `cli.md` first (every other doc references it indirectly).
2. `http-api.md` (the L doc; most editing).
3. `api-error-envelope.md` (new — split out of `http-api.md` rewrite naturally).
4. `environment-variables.md` (remove duplicated metric content once `metrics-catalog.md` from DOC-H lands).
5. `repository-structure.md`.
6. `configuration.md`, `database-admin.md` (small edits).
7. Verify-only docs: `ffi-vtable.md`, `package-manifest.md`, `package-shell-macro.md`, `websocket-protocol.md`.

### Risk considerations

- `http-api.md` rewrite is L — the route surface is large. Allocate 2-3 days. Cross-check every route handler signature; don't paraphrase code without verifying.
- `repository-structure.md` references `examples/features/` and `examples/tutorials/` — these are touched by other clusters. Snapshot the structure at start-of-work and re-verify at end.
- If DOC-H hasn't landed `metrics-catalog.md` yet, leave `environment-variables.md`'s metric-table cross-link as a placeholder `{{< ref "/platform/reference/metrics-catalog" >}}` and surface a sequencing note.

## Status Updates

*To be added during implementation.*
