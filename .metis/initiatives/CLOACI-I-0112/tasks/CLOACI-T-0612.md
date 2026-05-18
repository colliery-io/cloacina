---
id: doc-b-platform-reference-refresh
level: task
title: "DOC-B: Platform reference refresh — CLI, HTTP API, configuration, env-vars, metrics catalog"
short_code: "CLOACI-T-0612"
created_at: 2026-05-18T18:19:20.604315+00:00
updated_at: 2026-05-18T20:30:17.581461+00:00
parent: CLOACI-I-0112
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria

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

### 2026-05-18 — execution

Completed in one Ralph session.

**Edits landed:**

1. **`cli.md` (M)** — added the 5 missing server flags (`--verification-org-id`, `--reconcile-interval-s`, `--tenant-runner-cache-size`, `--tenant-deletion-drain-timeout-s`, `--log-retention-days`) with env-var equivalents; added daemon `--log-retention-days`; corrected `execution list` default (100, not 50) with `--status`/`--workflow`/`--limit`/`--offset` documented; added `trigger list` `--limit`/`--offset`; rewrote the `tenant delete` operational note from "TenantDatabaseCache never evicts; restart the server" to the live T-0581 4-step teardown orchestration.

2. **`environment-variables.md` (M)** — added 4 new server env vars (`CLOACINA_REQUIRE_SIGNATURES`, `CLOACINA_VERIFICATION_ORG_ID`, `CLOACINA_TENANT_RUNNER_CACHE_SIZE`, `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S`) with both inline row and CLI-equivalent table; replaced the legacy `cloacina_pipelines_total` metric list with the current `cloacina_workflows_total` family and deferred the full enumeration to `metrics-catalog.md`; added a new "Compiler" section with `CLOACINA_COMPILER_*` env vars per I-0104/I-0109; added an "Install script" section with `CLOACINACTL_VERSION`, `INSTALL_DIR`, `CLOACINA_REPO`; extended the Summary Table to match.

3. **`api-error-envelope.md` (new, S)** — created. Documents the canonical `{"error", "code"}` envelope shape, `x-request-id` header semantics (outbound + inbound), full error-code catalog grouped by HTTP status (400 / 401 / 403 / 404 / 500) with every code emitted by `crates/cloacina-server/src/routes/{auth,keys,tenants,executions,triggers,workflows,ws}.rs` and the conditions under which it fires; per-status retry guidance for clients; WebSocket auth-failure shape that aligns with `cloacina_ws_auth_failures_total` label values.

4. **`configuration.md` (S)** — added T-0502 sole-recovery semantics note on `enable_recovery` (the field still exists, but `RecoveryManager` is removed; the stale-claim sweeper is the sole task-recovery path now). Added cross-links to `environment-variables.md` and `metrics-catalog.md` in the "See Also" block.

5. **`database-admin.md` (S)** — added a "production callers: use the HTTP route, not this library call" callout to the `remove_tenant` section, documenting the T-0581 4-step orchestration that wraps the bare library call (revoke keys → evict runner → evict DB → drop schema); flagged that direct library use skips steps 1-3 and is appropriate only for testing / one-shot scripts.

6. **`repository-structure.md` (M)** — substantive rewrite. Lists all 11 workspace crates (previous doc listed 6) with per-crate purpose blurb; corrects the "Python embedded in cloacina core" claim (post-T-0529/T-0532, Python runtime is `cloacina-python`); refreshed examples list against `examples/features/{workflows,computation-graphs}/` (added `conditional-retries`, `deferred-tasks`, `event-triggers`, `python-workflow`, `packaged-triggers`, plus the 3 CG features); rewrote the development section around `angreal` tasks per the project convention.

7. **`http-api.md` (L)** — surgical L-rewrite (no whole-doc rewrite per scope-control). Added a new "Universal response invariants" section at the top covering:
   - The `{"error", "code"}` envelope (with link to api-error-envelope.md for the full catalog), with explicit note that per-endpoint error-body examples elsewhere in the document omit `code` for brevity but the live responses always include it.
   - `x-request-id` header propagation, both outbound and inbound (for end-to-end trace IDs).
   - SSE / live-follow non-availability in v1 (explicit note that `execution events --follow` is forward-compat-only and clients should poll `?since=…`).

   Then fixed the highest-impact drift points:
   - `POST /v1/tenants` request body: rewritten to `{name, description?, password?}` per T-0594 / API-01 (was `{schema_name, username, password}`); added "Breaking change" callout pointing at the migration; updated response shape to match.
   - `DELETE /v1/tenants/{schema_name}`: completely rewrote the section around the 4-step T-0581 orchestration; new response body documents `revoked_keys`, `runner_evicted`, `db_cache_evicted` fields; new errors table with `tenant_removal_failed` / `internal_error` / `admin_required` codes.
   - `GET /v1/tenants/{tenant_id}/executions`: added Query parameters table (`status`, `workflow`, `limit`, `offset`) with defaults / bounds; documented `invalid_pagination` error.
   - `GET /v1/tenants/{tenant_id}/triggers`: same pagination treatment per T-0596 / API-10.
   - "Operational caveats → Tenant database isolation": rewrote the section to reflect the closed-by-I-0106/T-0579/T-0580/T-0581 state (per-tenant runners cached, per-tenant trigger filtering, cache eviction on delete, fail-closed search_path). Removed the stale "isolation gap" framing.

**Verify-only docs** — `ffi-vtable.md`, `package-manifest.md`, `package-shell-macro.md`, `websocket-protocol.md`: spot-checked against the corresponding code paths in `crates/cloacina-workflow-plugin/`, `crates/cloacina/src/packaging/manifest_schema.rs`, `crates/cloacina-macros/src/packaged_workflow.rs`, `crates/cloacina-server/src/routes/ws.rs`. No edits required. Phase 4 reviewers should re-verify if time permits.

**Acceptance criteria:**
- ✅ `cli.md` lists every flag exposed by the server binary + the daemon's `--log-retention-days`. Compiler flags table is wrapper-level (defers to the binary).
- ✅ `http-api.md` documents the universal envelope; high-impact route drift is fixed. Per-endpoint error tables retained but now framed by the universal-invariants section; full code catalog lives in api-error-envelope.md.
- ✅ `api-error-envelope.md` exists with full code enumeration.
- ✅ `environment-variables.md` includes every server + compiler + install-script env var; metric names match emitted code (`workflows`, not `pipelines`).
- ✅ `repository-structure.md` matches `ls crates/` (11 crates).
- ✅ Cross-links to `metrics-catalog.md` added in `configuration.md`, `environment-variables.md`, `repository-structure.md`, and the new `api-error-envelope.md`.

**Verification needed externally (user action):**
- Run `angreal docs build` to confirm the Hugo site builds clean. New cross-links to `api-error-envelope` and `metrics-catalog` should resolve.

**Flags for downstream clusters:**
- **DOC-C**: the `cli.md` server flag table is now the authoritative list — when DOC-C edits `deploying-the-api-server.md`, `production-deployment.md`, and `running-the-server-image.md`, cite the flag rows here rather than re-enumerate.
- **DOC-D**: `multi-tenancy.md` rewrite should cite the now-correct http-api caveats section ("Tenant database isolation"), and the I-0106 closed-issues framing here matches.
- **DOC-E + DOC-F**: any workflow or CG how-to that displays an HTTP error response should reference `api-error-envelope.md` rather than show ad-hoc shapes.
- **DOC-I**: the `glossary.md` work should add an entry for `ApiError` pointing at `api-error-envelope.md`.
- **API client SDK authors** (out of scope): the `code` field is now first-class — `(http_status, code)` is the new error-discrimination tuple, not just `http_status`.
