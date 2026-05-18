# Audit — platform/ docs (CLOACI-I-0112 Phase 2)

> Produced by parallel audit agent (general-purpose). Per-doc design.md entries; preserved verbatim for Phase 3 reference. Synthesis lives in [design.md](./design.md).

### platform/_index.md (status: existing)
- **Category:** Index
- **Audience:** All readers landing on the Platform section
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `platform/_index.md:30-35` how-to list omits `compiler-deployment-runbook` (new), `running-the-compiler`, `running-the-daemon`, `use-cloacina-compiler-locally`, `safely-unload-a-package`, `configure-multi-tenant-deployment`, `deploying-to-kubernetes`, `running-the-server-image`, `security/*` — only six of the eleven existing how-tos are linked
  - Reference list omits `package-manifest`, `package-shell-macro`, `ffi-vtable`, `metrics-catalog` (new)
  - Explanation list omits `inventory-and-runtime-seeding`, `packaged-workflow-architecture`, `reconciler-pipeline`
- **Effort:** S

### platform/explanation/_index.md (status: existing)
- **Status delta:** verify-no-changes (uses `toc-tree`)
- **Effort:** S

### platform/explanation/database-backends.md (status: existing)
- **Category:** Explanation
- **Audience:** Library/embedded-mode developer choosing a backend
- **Status delta:** rewrite
- **Drift / gaps found:**
  - VER-P-01: `database-backends.md:52` — `cloacina = "0.1.0"` (workspace is `0.6.1`)
  - VER-P-02: `database-backends.md:66, 72` — `cloacina = { version = "0.1.0", ... }`
  - Heavy how-to drift: lines 28-72 prescribe Cargo snippets (belongs in workflows quick-start, not platform/explanation)
  - "Migration Strategies" section (lines 442-490) is operational recipe content — belongs in a how-to
  - Crate name `cloacina-workflow` and the slim-crate split (T-0529 / I-0102) not mentioned
- **Coverage (May 2026 batch):** Pre-existing; should retain SQLite `:memory:` mention (T-0608) and note WAL parameters
- **Effort:** M

### platform/explanation/ffi-system.md (status: existing)
- **Category:** Explanation
- **Audience:** Plugin / packaged-workflow authors and host integrators
- **Status delta:** edit-section
- **Drift / gaps found:**
  - `ffi-system.md:107` — cross-link points to `service/07-packaged-workflows/` which IA-03 flags as a numbering collision
  - References `cloacina-workflow-plugin` correctly but doesn't mention nine-method vtable (the FFI vtable reference doc has nine; this doc only describes two methods)
  - Doesn't reflect I-0102 unified `cloacina::package!();` shell macro emission path
- **Coverage (May 2026 batch):** I-0102 (unified package shell — must cross-link to it)
- **Effort:** S

### platform/explanation/horizontal-scaling.md (status: existing)
- **Category:** Explanation
- **Audience:** Operator running multiple runner instances
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Mentions `enable_recovery` (still exists per `runner/default_runner/config.rs:74`) but doesn't mention T-0502 `RecoveryManager` removal — the heartbeat sweeper is now the sole recovery path
  - T-0487 cooperative-cancellation-on-claim-loss not mentioned; ClaimLost branch (lines 110-116) shows "stop" but doesn't say the in-flight task is cancelled cooperatively
  - "Failure Scenarios → Network Partition" (line 378-382) downplays the duplicate-execution window; with T-0487 the cooperative cancellation closes that gap
  - "See Also" links point at `architecture-overview.md`, `task-execution-sequence.md`, `dispatcher-architecture.md`, `guaranteed-execution-architecture.md` — verify these targets in `workflows/explanation/` still exist
- **Coverage (May 2026 batch):** T-0487, T-0502
- **Effort:** M

### platform/explanation/inventory-and-runtime-seeding.md (status: existing)
- **Status delta:** verify-no-changes
- **Drift / gaps found:** none — content correctly reflects I-0102 unified shell macro and the FFI bridge
- **Effort:** S

### platform/explanation/multi-tenancy.md (status: existing)
- **Category:** Explanation
- **Audience:** Operator + library developer designing multi-tenant deployment
- **Status delta:** rewrite
- **Drift / gaps found:**
  - **Major**: doesn't cover I-0106 fail-closed `SET search_path` enforcement (`set_strict_search_path` in `crates/cloacina/src/database/connection/mod.rs:125`) — a critical security-relevant behavior change
  - **Major**: doesn't cover I-0106 / T-0581 four-step `remove_tenant` teardown orchestration (revoke keys → evict runner cache → evict DB cache → drop schema) — server now DOES evict caches on tenant delete, contradicting older "TenantDatabaseCache never evicts" framing elsewhere
  - **Major**: doesn't cover I-0106 `TenantRunnerCache` (`crates/cloacina-server/src/tenant_runner_cache.rs`) or `--tenant-runner-cache-size` (default 256)
  - **Major**: doesn't cover `--tenant-deletion-drain-timeout-s` (default 30s) hard-eviction posture (`crates/cloacina-server/src/main.rs:83`)
  - Heavy how-to drift: `Production Deployment` (lines 510-530), `Backup and Recovery` (lines 541-549), `Migration Strategies` (lines 442-490) belong in how-to docs
  - Per-tenant credential discussion (lines 193-320) belongs in `platform/reference/database-admin.md`
  - Auth tenant authorization filter (triggers/graph/accumulators) per I-0106 not mentioned
- **Coverage (May 2026 batch):** I-0106
- **Effort:** L

### platform/explanation/package-format.md (status: existing)
- **Category:** Explanation
- **Audience:** Plugin authors, custom-tool builders, debug
- **Status delta:** rewrite
- **Drift / gaps found:**
  - VER-P-03: `package-format.md:164` — `cloacina-workflow = "0.2"` (workspace is `0.6.1`)
  - Major: example manifest (lines 49-105) is the **old** schema (`package.cloacina_version`, `library.symbols`, `tasks[].index`, etc.) — the I-0102 + current `manifest_schema.rs` uses `format_version: "2"`, `package.{name, version, fingerprint, targets}`, `language: "python"|"rust"`, `python.*` / `rust.library_path` blocks
  - References `cloacina-ctl/src/manifest/types.rs` (line 45) — crate name is `cloacinactl`, and the schema lives in `crates/cloacina/src/packaging/manifest_schema.rs`
  - "Required Symbols" (line 367) names `fidius_get_registry` correctly, but the manifest field `library.symbols` no longer exists in format_version 2
  - Doesn't mention `package_type` removal (I-0102) or unified `cloacina::package!();` shell
- **Coverage (May 2026 batch):** I-0102
- **Effort:** M

### platform/explanation/packaged-workflow-architecture.md (status: existing)
- **Category:** Explanation
- **Audience:** Architect planning embedded-vs-packaged mix
- **Status delta:** edit-section
- **Drift / gaps found:**
  - "Crate Structure" section (lines 350-380) lists `cloacina/`, `cloacina-workflow/`, `cloacina-macros/` only — current workspace has eleven crates
  - Doesn't mention `cloacina-workflow-plugin` (the FFI vtable trait crate)
  - Database schema sketches (lines 187-211) — verify against actual diesel schema; `binary_data` column may have moved out of `workflow_registry` into per-deployment storage backends
  - `cloacina-workflow` Cargo.toml dep ref needs update from older versions
- **Coverage (May 2026 batch):** I-0102 (unified shell), T-0529 / T-0532 (cloacina-python split)
- **Effort:** M

### platform/explanation/performance-characteristics.md (status: existing)
- **Category:** Explanation
- **Audience:** Capacity planner, perf tuner
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Methodology warning (lines 11-20) about "workflows/sec vs tasks/sec" is sound but stale — the May batch added `cloacina_task_duration_seconds` and `cloacina_active_tasks` (per `docs/operations/metrics.md:54, 63`) so task-level metrics now exist
  - Doesn't reference the new metrics catalog
  - `examples/performance/{simple,parallel,pipeline}/` paths verified
- **Coverage (May 2026 batch):** I-0099, I-0108 (metrics references)
- **Effort:** S

### platform/explanation/reconciler-pipeline.md (status: existing)
- **Status delta:** verify-no-changes
- **Drift / gaps found:** none — fully reflects the six-step ordered pipeline, FFI method indices, and I-0102 / I-0103 fail-closed signature verification at the canonical load path. Cross-link to `computation-graphs/explanation/reactor-lifecycle.md` is correct (post-S-0011 renames)
- **Coverage (May 2026 batch):** I-0102, I-0103
- **Effort:** S

### platform/how-to-guides/_index.md (status: existing)
- **Status delta:** verify-no-changes (uses `toc-tree`)
- **Effort:** S

### platform/how-to-guides/configure-multi-tenant-deployment.md (status: existing)
- **Category:** How-to
- **Audience:** Operator standing up multi-tenant Postgres
- **Status delta:** rewrite
- **Drift / gaps found:**
  - **Major**: "TenantDatabaseCache never evicts" (lines 183-189) is now WRONG per I-0106 / T-0581 — the `DELETE /v1/tenants/{name}` route does a four-step teardown that evicts both `TenantDatabaseCache` and `TenantRunnerCache`. The "restart cloacina-server after any tenant delete" guidance is stale
  - "Workflow execution scheduling is NOT tenant-scoped" (lines 168-180) — verify against I-0106. The `TenantRunnerCache` provides per-tenant `DefaultRunner` instances now, so this caveat may also be obsolete (each tenant has its own runner up to the LRU cap)
  - "Trigger list is global" caveat (lines 192-199) — I-0106 added tenant-set filtering on triggers/graph/accumulators endpoints; verify the current state
  - Doesn't cover `--require-signatures` + `--verification-org-id` (I-0103); should pair with new "require-signed-packages" how-to
  - Doesn't cover `--tenant-deletion-drain-timeout-s` knob
  - Doesn't show how to scope a `cloacinactl key create --tenant` with a non-admin caller (current example uses ADMIN_KEY end-to-end)
- **Coverage (May 2026 batch):** I-0106 (rewrite), I-0103 (cross-link), T-0581
- **Effort:** L

### platform/how-to-guides/deploying-the-api-server.md (status: existing)
- **Category:** How-to
- **Audience:** Operator wanting prose-form server setup
- **Status delta:** rewrite
- **Drift / gaps found:**
  - NOM-P-01: lines 30, 32, 38, 78, 86, 168, 174, 182 use `cloacinactl serve` — current verb is `cloacinactl server start` (per `crates/cloacinactl/src/nouns/server/`, confirmed by `platform/reference/cli.md:170`)
  - Lines 51-53, 97 show endpoints as bare `/auth/keys` — actual routes are `/v1/auth/keys` (per `crates/cloacina-server/src/lib.rs:665` mounted under `/v1/`)
  - Line 27 says PostgreSQL 16+; the reference says 14+; the deployment compose example pins 16; reconcile
  - Doesn't cover `--require-signatures` / `--verification-org-id` (I-0103)
  - Doesn't cover `--reconcile-interval-s`, `--tenant-runner-cache-size`, `--tenant-deletion-drain-timeout-s`, `--log-retention-days` (all I-0106 / I-0109)
  - Line 53 shows `DEL /auth/keys/:id` — actual is `DELETE` and path is `/v1/auth/keys/{key_id}`
  - Docker example (lines 246-266) references `your-registry/cloacinactl:latest` — should reference the official `ghcr.io/colliery-software/cloacina-server` image
  - Line 51 startup banner output is from the old `cloacinactl serve` path — the new path prints `/v1/`-prefixed endpoints (per `crates/cloacina-server/src/lib.rs:553`)
- **Coverage (May 2026 batch):** I-0103, I-0106, I-0109, I-0111
- **Effort:** L

### platform/how-to-guides/deploying-to-kubernetes.md (status: existing)
- **Category:** How-to
- **Audience:** Platform operator deploying to Kubernetes via Helm
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Line 53 says "This pulls in Bitnami's `postgresql` subchart" — T-0610 replaced the Bitnami dependency with a **local embedded** subchart at `charts/cloacina-server/charts/postgresql/` (per `charts/cloacina-server/Chart.yaml:34-48`). Update wording
  - Lines 24, 38, 47, 70, 95, 137 use `--version 0.1.0` — chart `version` is `0.1.0` (current), `appVersion: "0.6.1"` — these may or may not need bumping; verify when chart bumps
  - Doesn't reference compiler deployment via Helm (currently no compiler in the chart per `charts/cloacina-server/templates/` listing: deployment, ingress, service, servicemonitor only)
  - "Probes + signatures + tenants" table is correct against `values.yaml` but missing `tenantDeletionDrainTimeoutS`, `logRetentionDays`, `reconcileIntervalS` if they're exposed (verify against full `values.yaml`)
  - Line 53 also says credentials exposed via Helm values — true; should cross-link to a "decommission a tenant" or "rotate Postgres credentials" how-to
- **Coverage (May 2026 batch):** I-0111, T-0610
- **Effort:** M

### platform/how-to-guides/performance-tuning.md (status: existing)
- **Category:** How-to
- **Audience:** Operator tuning a deployment
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Line 119-120 references "Recovery manager" as a background service consuming connections — T-0502 removed `RecoveryManager`; heartbeat sweeper is sole recovery
  - Line 407 metric `cloacina_pipelines_total` is OLD name (per code: emitted name is `cloacina_workflows_total`, see `crates/cloacina/src/execution_planner/scheduler_loop.rs:362, 377`). Same for `cloacina_active_pipelines` if referenced
  - "Key metrics to watch" table (lines 404-409) is tiny — should defer to new `metrics-catalog.md` reference
  - Doesn't reference the May batch's new reactor / accumulator / WS metrics (I-0099, I-0108)
- **Coverage (May 2026 batch):** I-0099, I-0108, T-0502
- **Effort:** M

### platform/how-to-guides/production-deployment.md (status: existing)
- **Category:** How-to
- **Audience:** Operator pinning a small prod
- **Status delta:** rewrite
- **Drift / gaps found:**
  - NOM-P-02: lines 8, 79, 84, 114 use `cloacinactl serve` — should be `cloacinactl server start`
  - Body limit "100MB" at line 70 verified against `http-api.md:835` (correct)
  - Docker compose `command: ["serve", ...]` at line 114 — should be `["server", "start", ...]` or just `--bind` flags when using the published `cloacina-server` image directly (where the entrypoint is the server binary, not `cloacinactl`)
  - Doesn't cover `--require-signatures`, `--log-retention-days`, `--tenant-runner-cache-size`, or any I-0106 hardening
  - Doesn't link to the Helm or Docker image how-tos for richer paths
- **Coverage (May 2026 batch):** I-0103, I-0109 (gap)
- **Effort:** M

### platform/how-to-guides/running-the-compiler.md (status: existing)
- **Category:** How-to
- **Audience:** Operator running the compiler as a long-lived service
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Doesn't mention `--log-retention-days` flag (per `crates/cloacina-compiler/src/main.rs:141`, default 7 days)
  - Doesn't mention `/metrics` endpoint exposed by the compiler (per `crates/cloacina-compiler/src/health.rs:54`)
  - Doesn't mention `cloacina_compiler_*` metric family (per `docs/operations/metrics.md:154-162`) — these belong in the new metrics catalog but the runbook should reference them
  - Table at lines 156-168 is otherwise accurate against `crates/cloacina-compiler/src/config.rs:32-107`
  - Audit-event fields verified against compiler source
- **Coverage (May 2026 batch):** I-0097, I-0104, I-0109
- **Effort:** S

### platform/how-to-guides/running-the-daemon.md (status: existing)
- **Category:** How-to
- **Audience:** Local-mode operator
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Line 23 says `cloacinactl daemon` (bare) — current spelling is `cloacinactl daemon start` (per `crates/cloacinactl/src/nouns/daemon/mod.rs`). The bare form may still parse via clap default; verify
  - Doesn't mention `--log-retention-days` flag (per `crates/cloacinactl/src/nouns/daemon/start.rs:27`)
  - `config.toml` `[daemon]` schema at lines 63-88 has `cron_lost_threshold_min` — `platform/reference/configuration.md:228` notes this is NOT wired to `cron_lost_threshold_minutes`. Cross-flag this caveat here too
  - Line 14 tutorial reference: `service/07-packaged-workflows` is part of IA-03 numbering collision but cross-link is the correct path
- **Coverage (May 2026 batch):** I-0109 (log retention)
- **Effort:** S

### platform/how-to-guides/running-the-server-image.md (status: existing)
- **Category:** How-to
- **Audience:** Operator deploying via docker pull
- **Status delta:** edit-section
- **Drift / gaps found:**
  - VER-P-04: line 20 example pins `cloacina-server:0.6.0` — workspace is at `0.6.1`. Update floor example or use `<X.Y>` floating tag
  - Doesn't mention rdkafka build deps (T-0609) inherited into the runtime image (`Dockerfile:72-90`)
  - Doesn't mention `--log-retention-days`, `--reconcile-interval-s`, `--tenant-deletion-drain-timeout-s` envs (per `crates/cloacina-server/src/main.rs:79-89`)
  - The `tag scheme` table (lines 31-37) is invented copy — verify against the actual CI publishing workflow; if `<X.Y>` floating tag isn't actually published, remove
  - "Image properties" (line 113-118): non-root UID 10001 — verify against current `Dockerfile`
- **Coverage (May 2026 batch):** I-0103, I-0106, I-0109, I-0111, T-0609
- **Effort:** S

### platform/how-to-guides/safely-unload-a-package.md (status: existing)
- **Category:** How-to
- **Audience:** Operator rolling back a package
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - Line 192 uses `cloacinactl graph accumulators` — confirmed verb exists per `crates/cloacinactl/src/nouns/graph/mod.rs:46`
  - Cross-links to `computation-graphs/explanation/reactor-lifecycle.md` use post-S-0011 path
  - "Caveat: restarting the server resets the `TenantDatabaseCache`" at line 213-217 is partly stale — see multi-tenancy doc finding (T-0581 added eviction)
- **Coverage (May 2026 batch):** I-0102, T-0581 (caveat update)
- **Effort:** S

### platform/how-to-guides/security/_index.md (status: existing)
- **Status delta:** verify-no-changes (uses `toc-tree`)
- **Effort:** S

### platform/how-to-guides/security/local-development.md (status: existing)
- **Category:** How-to
- **Audience:** Developer iterating on package signing locally
- **Status delta:** rewrite
- **Drift / gaps found:**
  - This doc is mostly a code example walkthrough — heavy explanation drift; should be a true how-to (concrete steps: what to do, in what order, expected outcome)
  - Lines 14-17, 38-44, 51-90: Rust API surface (`SecurityConfig::default()`, `DefaultRunner::new(config, dal)`) may not match current API — verify against `crates/cloacina/src/security/`
  - Line 121-125 GH Actions YAML references `cloacina sign ./target/release/libworkflow.so` — there is no `cloacina sign` command; should be `cloacinactl package pack --sign <key>` (which currently **fail-hards** per `crates/cloacinactl/src/nouns/package/pack.rs:21-32`)
  - Doesn't mention that I-0103 added server-side `--require-signatures` + `--verification-org-id` enforcement (fail-closed at canonical load path)
- **Coverage (May 2026 batch):** I-0103
- **Effort:** M

### platform/how-to-guides/security/package-signing.md (status: existing)
- **Category:** How-to (drifting toward Reference)
- **Audience:** Operator setting up signing in production
- **Status delta:** rewrite
- **Drift / gaps found:**
  - This is a hybrid how-to + reference + API tour — should split: reference goes to `platform/reference/security-api.md` (new) and how-to stays as a recipe
  - Rust API examples (lines 14-90) may have stale signatures; verify against `crates/cloacina/src/security/`
  - Doesn't mention the I-0103 server-side flags (`--require-signatures`, `--verification-org-id`) — only covers library API
  - Doesn't mention the `cloacinactl package pack --sign` fail-hard CLI stub
  - Audit-log event types listed at line 238-244 — verify against current emitter
  - "Best practices" (line 259-265) is fine but generic
- **Coverage (May 2026 batch):** I-0103
- **Effort:** L

### platform/how-to-guides/use-cloacina-compiler-locally.md (status: existing)
- **Category:** How-to
- **Audience:** Local developer or CI building packages
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Line 107-112: claims `--sign` "is currently a no-op in the CLI" — actual behavior per `crates/cloacinactl/src/nouns/package/pack.rs:21-32` is **fail-hard** with error message pointing to I-0103. Update
  - Line 26 cross-link to "migrating-to-service-mode" referenced; verify the target slot in `workflows/how-to-guides/` exists
  - Line 189 last cross-link points at `docs/operations/compiler-deployment.md` (GitHub raw URL) — once that file is moved to `platform/how-to-guides/compiler-deployment-runbook.md`, update
- **Coverage (May 2026 batch):** I-0103 (signing fail-hard reference)
- **Effort:** S

### platform/reference/_index.md (status: existing)
- **Status delta:** verify-no-changes (uses `toc-tree`)
- **Effort:** S

### platform/reference/cli.md (status: existing)
- **Category:** Reference
- **Audience:** CLI user looking up a flag
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Server flags table (lines 178-184) missing `--verification-org-id`, `--reconcile-interval-s`, `--tenant-runner-cache-size`, `--tenant-deletion-drain-timeout-s`, `--log-retention-days` (all per `crates/cloacina-server/src/main.rs:39-89`)
  - Compiler flags table (lines 213-220) is wrapper-level only and notes "binary default" — should defer the underlying flag list to the new `compiler-deployment-runbook.md` cross-link
  - `trigger list` row at line 326 should note `--limit` / `--offset` (per `crates/cloacinactl/src/nouns/trigger/mod.rs:34-42`, T-0596 / API-10)
  - `execution list` row at line 289: notes "default limit: 50" — actual server default is 100 (per `crates/cloacina-server/src/routes/executions.rs:145`), and CLI supports `--status` + `--workflow` filters
  - `execution events` row at line 291: `--follow` correctly noted as "not yet implemented" (fail-hard per `crates/cloacinactl/src/nouns/execution/mod.rs:97-107`)
  - Daemon `start` should note `--log-retention-days` (per `crates/cloacinactl/src/nouns/daemon/start.rs:27`)
  - `graph` verbs at lines 298-302: correctly lists `list`, `status`, `accumulators`. The doc claims `cloacinactl graph` is "per CLOACI-S-0011" — confirmed
- **Coverage (May 2026 batch):** I-0098, I-0107, T-0538, T-0596, I-0103, I-0109
- **Effort:** M

### platform/reference/configuration.md (status: existing)
- **Category:** Reference
- **Audience:** Library developer + operator
- **Status delta:** edit-section
- **Drift / gaps found:**
  - "Concurrency" table line 35 lists `enable_recovery` — still exists per `crates/cloacina/src/runner/default_runner/config.rs:74`, but T-0502 changed its semantics (heartbeat sweeper is sole recovery). Add a note
  - The "Registry storage backend" line 66 says `"database"` is an option — verify against current `FilesystemRegistryStorage`/`DatabaseRegistryStorage` enum
  - Doesn't reference May batch additions: I-0106 fail-closed search_path, `TenantRunnerCache` cap, `tenant_deletion_drain_timeout_s` (these are server-level rather than `DefaultRunnerConfig`, but the doc should at least name them with a "see HTTP server flags" pointer)
  - Mapping table at line 224-229 says `cron_lost_threshold_min` is unwired; verify against current `crates/cloacinactl/src/commands/daemon.rs` (the doc is correct as of T-0596 / API-13 fix per archived T-0596, but re-verify)
- **Coverage (May 2026 batch):** I-0106 (cross-link), T-0502 (note)
- **Effort:** S

### platform/reference/database-admin.md (status: existing)
- **Category:** Reference
- **Audience:** Library developer integrating `DatabaseAdmin`
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Doesn't reflect that `remove_tenant` orchestration is now layered above the DB-admin call by the server (per T-0581, the HTTP route does revoke-keys → evict-runner → evict-db → drop-schema in sequence); the bare `DatabaseAdmin::remove_tenant` is **step 4** of that flow
  - "Python Bindings" section (line 263-281): verify `cloaca.DatabaseAdmin` exposure against current `crates/cloacina-python/`
  - Returns are correct against `crates/cloacina/src/database/admin.rs`
- **Coverage (May 2026 batch):** I-0106 (note)
- **Effort:** S

### platform/reference/environment-variables.md (status: existing)
- **Category:** Reference
- **Audience:** Operator
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Lines 148-154 list OLD metric names: `cloacina_pipelines_total`, `cloacina_active_pipelines`, `cloacina_pipeline_duration_seconds`. Actual emitted names are `cloacina_workflows_total`, `cloacina_active_workflows`, `cloacina_workflow_duration_seconds` (per `crates/cloacina/src/execution_planner/scheduler_loop.rs:362-394`). The note "Metrics use 'pipeline' as the internal name" at line 148 is misleading — it's the other way: code uses `workflows`, the legacy field name was `pipeline`
  - Doesn't include `CLOACINA_REQUIRE_SIGNATURES`, `CLOACINA_VERIFICATION_ORG_ID`, `CLOACINA_TENANT_RUNNER_CACHE_SIZE`, `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` (all per `crates/cloacina-server/src/main.rs:48-89`)
  - Compiler env vars (`CLOACINA_COMPILER_BUILD_TIMEOUT_S`, `CLOACINA_COMPILER_VENDOR_DIR`, `CLOACINA_COMPILER_BUILD_RLIMIT_*`) missing entirely — listed in `running-the-compiler.md:157-168` but not in the canonical env-var reference
  - `CLOACINACTL_VERSION` and `INSTALL_DIR` used by `install.sh` should at least be flagged
  - Docker Compose section at line 218-244 names `KAFKA_*` variables — kafka may have been pruned from server image, verify
  - Defer the full metrics list to the new `metrics-catalog.md` reference
- **Coverage (May 2026 batch):** I-0099, I-0103, I-0104, I-0106, I-0108, I-0109, I-0111
- **Effort:** M

### platform/reference/ffi-vtable.md (status: existing)
- **Status delta:** verify-no-changes
- **Drift / gaps found:** none — method indices, optional-since-v2 semantics, wire types are accurate against `crates/cloacina-workflow-plugin/src/lib.rs` and `types.rs`. Cross-link to `package-shell-macro.md` is correct
- **Coverage (May 2026 batch):** I-0102
- **Effort:** S

### platform/reference/http-api.md (status: existing)
- **Category:** Reference
- **Audience:** API consumer
- **Status delta:** rewrite
- **Drift / gaps found:**
  - **Major**: All error response bodies show `{"error": "..."}` only — actual `ApiError` envelope is `{"error": "...", "code": "..."}` (per `crates/cloacina-server/src/routes/error.rs:80-88`). Request-ID propagated via `x-request-id` header (line 38 of `error.rs`)
  - **Major**: `POST /v1/tenants` request body documented as `{schema_name, username, password}` — actual is `{name, description?, password?}` (per `crates/cloacina-server/src/routes/tenants.rs:51-68`, T-0594 / API-01)
  - `GET /v1/tenants/{id}/executions` shows no query params — actual route accepts `?status`, `?workflow`, `?limit`, `?offset` (per `crates/cloacina-server/src/routes/executions.rs:134-148`, T-0596 / API-11). Default limit is 100 (max 1000)
  - `GET /v1/tenants/{id}/triggers` shows no pagination — actual route accepts `?limit`, `?offset` (per `crates/cloacina-server/src/routes/triggers.rs:33-72`, T-0596 / API-10)
  - "Operational caveats → Tenant database isolation" (lines 811-829) says "TenantDatabaseCache never evicts" + "restart the server" — STALE per T-0581 four-step teardown (revoke keys → evict runner → evict DB → drop schema). Restart-only guidance only applies if the teardown route was bypassed
  - "Operational caveats → Signature verification" (lines 856-866) correct framing, but should reference `--verification-org-id` flag too
  - SSE-style `--follow` is NOT in v1 (per execution events CLI fail-hard at `nouns/execution/mod.rs:97-107`); the doc should make this explicit so readers don't write code expecting an SSE endpoint
  - Doesn't document `x-request-id` header propagation behavior
  - Doesn't describe the auth-cache 256-entry / 30s TTL at the route-doc level (it's in the caveats)
- **Coverage (May 2026 batch):** I-0106, I-0107, I-0108
- **Effort:** L

### platform/reference/package-manifest.md (status: existing)
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - Format version `"2"`, top-level structure, `package.{name, version, fingerprint, targets}`, `language: "python"|"rust"`, `python.*`, `rust.library_path`, `tasks[].{id, function, dependencies}`, `triggers[].{name, trigger_type, workflow, poll_interval, allow_concurrent, config}`, validation rules — all match `crates/cloacina/src/packaging/manifest_schema.rs`
  - Format is correct; one open question: does I-0102 add `reactors` / `graphs` / `triggerless_graphs` to the manifest? Check `manifest_schema.rs` for new fields
  - Trigger types table at lines 213-220 lists `"file_watch"`, `"http_poll"`, `"webhook"` — these may be illustrative rather than schema-defined; verify
- **Coverage (May 2026 batch):** I-0102
- **Effort:** S

### platform/reference/package-shell-macro.md (status: existing)
- **Status delta:** verify-no-changes
- **Drift / gaps found:** none — content correctly reflects I-0102 unified shell, nine-method emission, duplicate-invocation guard, ABI hash verification, `optional(since = 2)` for methods 4-8
- **Coverage (May 2026 batch):** I-0102
- **Effort:** S

### platform/reference/repository-structure.md (status: existing)
- **Category:** Reference
- **Audience:** Contributor or operator browsing the codebase
- **Status delta:** rewrite
- **Drift / gaps found:**
  - "Crates" section (lines 9-31) lists six crates — actual workspace has eleven (`crates/`: `cloacina`, `cloacina-build`, `cloacina-compiler`, `cloacina-computation-graph`, `cloacina-macros`, `cloacina-python`, `cloacina-server`, `cloacina-testing`, `cloacina-workflow`, `cloacina-workflow-plugin`, `cloacinactl`)
  - "Python Support" (lines 70-76) says Python runs through PyO3 "embedded in the `cloacina` core crate" — INCORRECT post T-0529 / T-0532: Python runtime is now isolated in `cloacina-python`
  - Tutorial directories table (lines 83-91) shows 6 tutorials with `01-basic-workflow/` etc. — verify against actual `examples/tutorials/`
  - Features table (lines 99-107) lists examples like `complex-dag/`, `cron-scheduling/`, `multi-tenant/`, `packaged-workflows/`, `per-tenant-credentials/`, `registry-execution/`, `simple-packaged/`, `validation-failures/` — verify against `examples/features/` (current structure has CG examples + filtered-reactor not in the list)
  - Cargo features list (lines 56-62) is incomplete; misses `auth`, `kafka`, `telemetry`, `extension-module`
- **Coverage (May 2026 batch):** T-0529, T-0532, all crate splits since 2026-04
- **Effort:** M

### platform/reference/websocket-protocol.md (status: existing)
- **Status delta:** verify-no-changes
- **Drift / gaps found:**
  - `ReactorCommand` variants `force_fire`, `fire_with`, `get_state`, `pause`, `resume` — all verified against `crates/cloacina-server/src/routes/ws.rs:389-405` and `crates/cloacina/src/computation_graph/reactor.rs:199-205`
  - Authorization model (line 199-208) and policies (line 308-318) verified against route
  - Rate-limiting note (line 322-332) accurate — `ApiError::too_many_requests` exists per `error.rs` (constructor enumeration); not enforced
  - Endpoints match `/v1/ws/accumulator/{name}` and `/v1/ws/reactor/{name}` (per `crates/cloacina-server/src/lib.rs:761-765`)
- **Coverage (May 2026 batch):** I-0107 (auth via ticket flow + per-command authZ)
- **Effort:** S

### platform/tutorials/_index.md (status: existing)
- **Category:** Index
- **Audience:** Operator starting first server
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Only one tutorial listed (`01-deploy-a-server`). Per IA-02 in initiative, this is thin; should call out the proposed second tutorial path (multi-tenant deployment, compiler pair, signature verification)
- **Effort:** S

### platform/tutorials/01-deploy-a-server.md (status: existing)
- **Category:** Tutorial
- **Audience:** First-time operator deploying a single server
- **Status delta:** edit-section
- **Drift / gaps found:**
  - Line 76-87 startup banner shows old `GET /metrics` etc. without `/v1/` prefix on auth routes — the actual banner per `crates/cloacina-server/src/lib.rs:553-555` does include `/v1/auth/keys`
  - Line 209-221 sample log output shows correct six-step reconciler messages (`step_load_cron_triggers`, etc.) — match `crates/cloacina/src/registry/reconciler/loading.rs` step names
  - Doesn't mention `--require-signatures` toggle or how to skip it for the tutorial (it's off by default, which is correct, but worth noting)
  - Line 173 `cloacinactl config profile set ...` — verify against `crates/cloacinactl/src/commands/config.rs`
  - Line 297 cross-link to `/workflows/how-to-guides/observe-execution-state` — verify target exists
  - Line 312 `cloacinactl package delete <package_id> --tenant acme` uses `<package_id>` placeholder; the tutorial should either show how to get the ID or use a deterministic value
- **Coverage (May 2026 batch):** I-0098 (profile model), I-0107 (CLI fixes — `tenant create` body works now)
- **Effort:** S

## New docs

### platform/how-to-guides/compiler-deployment-runbook.md (status: new)
- **Category:** How-to (long-form runbook)
- **Audience:** Platform operator deploying compiler-server pair to production
- **Status delta:** new-write (port + extend from `docs/operations/compiler-deployment.md`)
- **Coverage (May 2026 batch):** I-0097 (build queue), I-0104 (hardening Phase 1), I-0109 (compiler metrics + log retention), T-0610 (Helm chart)
- **Sources:** `docs/operations/compiler-deployment.md`, `crates/cloacina-compiler/src/`, `crates/cloacina-server/src/main.rs`, ADR-0004, Helm chart `values.yaml`
- **Key topics to cover/preserve:** Why two binaries (server + compiler); build queue mechanics (`workflow_packages.build_status`, heartbeat refresh); three deployment env recipes (bare-metal, compose, K8s); flag knobs to know; "stuck build" playbook; cross-link to short-form `running-the-compiler.md` for the threat model + vendor curation
- **Effort:** M

### platform/how-to-guides/decommission-a-tenant.md (status: new)
- **Category:** How-to
- **Audience:** Operator removing a tenant safely
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0106 (multi-tenant teardown), T-0581
- **Sources:** `crates/cloacina-server/src/routes/tenants.rs:112-220`, `crates/cloacina-server/src/main.rs:74-83`, archived I-0106 / T-0581
- **Key topics to cover/preserve:** Four-step teardown order; `--tenant-deletion-drain-timeout-s` knob; what hard-eviction means for in-flight workflows; auditing the per-step audit events; recovery if teardown half-completes; verification that `TenantRunnerCache` + `TenantDatabaseCache` were both evicted (smoke-test against deleted tenant should 404, not stale-pool error)
- **Effort:** M

### platform/how-to-guides/require-signed-packages.md (status: new)
- **Category:** How-to
- **Audience:** Operator turning on signature enforcement in prod
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0103, T-0567
- **Sources:** `crates/cloacina-server/src/main.rs:51-59`, `crates/cloacina-server/src/lib.rs:152-200`, archived I-0103
- **Key topics to cover/preserve:** Choosing a `verification_org_id`; producing the first trusted public key; the `--require-signatures` + `--verification-org-id` server flags (fail-fast on missing); package upload → 403 path; audit-log entries on success + failure; current limitation that CLI `--sign` is fail-hard (point to manual signing workflow or wait for I-0103 wire-up); recovery if you lock yourself out
- **Effort:** M

### platform/how-to-guides/use-cli-profiles.md (status: new)
- **Category:** How-to
- **Audience:** CLI user juggling multiple environments
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0098, T-0538
- **Sources:** `crates/cloacinactl/src/commands/config.rs`, ADR-0003 §3
- **Key topics to cover/preserve:** Creating named profiles via `config profile set`; default profile selection (`config profile use`); profile resolution precedence (flag > `--profile` > `default_profile`); switching between local daemon and remote server with profiles; API key schemes (`raw`, `env:`, `file:`, `keyring:` reserved); secret-rotation patterns
- **Effort:** S

### platform/reference/api-error-envelope.md (status: new — optional split)
- **Category:** Reference
- **Audience:** API client developer needing a single error-shape lookup
- **Status delta:** new-write (extract from `platform/reference/http-api.md` Operational Caveats)
- **Coverage (May 2026 batch):** I-0107
- **Sources:** `crates/cloacina-server/src/routes/error.rs`
- **Key topics to cover/preserve:** Error envelope shape `{"error", "code"}`; `x-request-id` header; full code enumeration by route (auth, tenant, workflow, execution, trigger, graph-health, ws-upgrade); HTTP status mapping; client retry guidance per code
- **Effort:** S — alternative: fold into `http-api.md` as a top section rather than a separate file. Recommend folding to keep IA flat.

### platform/reference/metrics-catalog.md (status: new)
- **Category:** Reference
- **Audience:** Platform operator, on-call engineer, dashboard author
- **Status delta:** new-write (port + extend from `docs/operations/metrics.md`)
- **Coverage (May 2026 batch):** I-0099 (cloacina_* namespace), I-0108 (SQL-derived `cloacina_active_*`, `Degraded` health, persist-failure counters), I-0109 (compiler `/metrics` + `cloacina_compiler_*` family)
- **Sources:** `docs/operations/metrics.md`, `crates/cloacina/src/` metric emission sites, `crates/cloacina-compiler/src/health.rs`, archived I-0099 / I-0108 / I-0109
- **Key topics to cover/preserve:** Every `cloacina_*` metric: name, type (counter/histogram/gauge), labels (bounded cardinality), meaning, where emitted; separate section for `cloacina_compiler_*`; quick PromQL recipes (5m task throughput, error rate, p99 latency, claim contention, stale-claim rate, graph degraded gauge alert); cross-link policy from explanation docs
- **Effort:** M

### platform/explanation/observability.md (status: new)
- **Category:** Explanation
- **Audience:** Operator deciding how to wire metrics/traces
- **Status delta:** new-write
- **Coverage (May 2026 batch):** I-0099, I-0108, I-0109
- **Sources:** archived I-0099 / I-0108 / I-0109, `docs/operations/metrics.md`, ADR-0002
- **Key topics to cover/preserve:** Why `cloacina_*` namespace and bounded labels; SQL-derived gauges vs delta-counted gauges (why I-0108 re-seeds `cloacina_active_tasks` from SQL); `Degraded` reactor health after 5 persist failures; tracing vs metrics trade-off; OpenTelemetry integration; structured logs as the third leg; **does NOT enumerate metrics** (defer to catalog)
- **Effort:** M

### platform/explanation/security-model.md (status: new — optional)
- **Category:** Explanation
- **Audience:** Security-conscious operator
- **Status delta:** new-write (optional; could fold into `multi-tenancy.md` if scope-creeps)
- **Coverage (May 2026 batch):** I-0103, I-0104, I-0106, ADR-0005
- **Sources:** ADR-0005, archived I-0103 / I-0104 / I-0106
- **Key topics to cover/preserve:** Trust model by deployment mode (local trust, server trust); auth roles (`is_admin` god mode vs role-based tenant access); bootstrap key invariants; signature verification rationale; compiler threat model (links to running-the-compiler); multi-tenant isolation guarantees and limits; `/metrics` unauthenticated trade-off
- **Effort:** M — flag as optional; many of these threads live in existing docs already. If too much overlap, drop and absorb into multi-tenancy + the security how-tos.

---

## Summary

- **Total existing files reviewed:** 31 (4 indexes + 1 tutorial + 11 how-tos including 2 in security/ + 11 references + 9 explanations + 1 explanation `_index`).
- **Total drift findings:**
  - NOM-P-01 — `cloacinactl serve` (deprecated) in `deploying-the-api-server.md` (multiple lines) and `production-deployment.md:8,79,84,114`
  - NOM-P-02 — package_type / format references in `package-format.md` (lines 45-105)
  - VER-P-01 — `cloacina = "0.1.0"` in `database-backends.md:52, 66, 72`
  - VER-P-02 — `cloacina-workflow = "0.2"` in `package-format.md:164`
  - VER-P-03 — Docker image example pinned to `0.6.0` in `running-the-server-image.md:20`
  - IA-P-01 — Platform `_index.md` lists only 6 of 11 existing how-tos
  - IA-P-02 — `platform/how-to-guides/security/` package-signing.md is hybrid how-to/reference; needs split
  - IA-P-03 — Operations docs (`compiler-deployment.md`, `metrics.md`) are still outside Hugo (per IA-01 in initiative)
  - API-P-01 — `http-api.md` shows `{"error": "..."}` envelope; actual is `{"error": "...", "code": "..."}`
  - API-P-02 — `http-api.md` `POST /v1/tenants` body shows `{schema_name, username, password}`; actual is `{name, description?, password?}`
  - API-P-03 — `http-api.md` executions/triggers list endpoints missing `?status`/`?workflow`/`?limit`/`?offset` query params
  - API-P-04 — `http-api.md` and `configure-multi-tenant-deployment.md` claim "TenantDatabaseCache never evicts"; T-0581 four-step teardown evicts both caches
  - API-P-05 — `environment-variables.md` lists old metric names (`cloacina_pipelines_total` etc.); actual emitted names use `workflows` not `pipelines`
  - API-P-06 — `performance-tuning.md` table also uses old `cloacina_pipelines_total` name
  - API-P-07 — `environment-variables.md` missing `CLOACINA_REQUIRE_SIGNATURES`, `CLOACINA_VERIFICATION_ORG_ID`, `CLOACINA_TENANT_RUNNER_CACHE_SIZE`, `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S`, all `CLOACINA_COMPILER_*` vars
  - API-P-08 — `cli.md` server flags table missing `--verification-org-id`, `--reconcile-interval-s`, `--tenant-runner-cache-size`, `--tenant-deletion-drain-timeout-s`, `--log-retention-days`
  - API-P-09 — `cli.md` `trigger list` row missing `--limit`/`--offset`; `execution list` default limit wrong (50 vs actual 100)
  - API-P-10 — `use-cloacina-compiler-locally.md` says `--sign` is a silent no-op; actual is fail-hard
  - API-P-11 — `multi-tenancy.md` doesn't cover I-0106 fail-closed `SET search_path` enforcement
  - API-P-12 — `multi-tenancy.md` doesn't cover `TenantRunnerCache`, `--tenant-runner-cache-size`, four-step `remove_tenant` orchestration
  - API-P-13 — `repository-structure.md` lists 6 crates; workspace has 11
  - API-P-14 — `repository-structure.md` says Python is embedded in `cloacina` crate; T-0529/T-0532 moved it to `cloacina-python`
  - API-P-15 — `deploying-to-kubernetes.md:53` says "Bitnami's `postgresql` subchart"; T-0610 replaced with local embedded subchart
  - API-P-16 — `performance-tuning.md:119-120` references `RecoveryManager`; T-0502 removed it
  - API-P-17 — `horizontal-scaling.md` doesn't cover T-0487 cooperative cancellation or T-0502 sole-heartbeat-recovery
  - API-P-18 — `package-format.md` example manifest is the legacy `format_version: "1"` schema, not the current `"2"`
  - API-P-19 — Multiple docs link `cloacina-ctl` (with hyphen); actual binary/crate name is `cloacinactl`
- **S-0011 nomenclature in platform docs:** Zero banned-phrase hits in `docs/content/platform/`. The platform area is already clean of the "reactive scheduler / reactive subsystem / reactive execution / ReactiveScheduler / cloacinactl reactor / /v1/health/reactors / health_reactive" set. Good signal that the renames (T-0528) landed correctly here.
- **Total new-doc proposals:** 7 (5 high-priority + 2 optional):
  - `platform/reference/metrics-catalog.md` (new, port + extend from `docs/operations/metrics.md`)
  - `platform/how-to-guides/compiler-deployment-runbook.md` (new, port + extend from `docs/operations/compiler-deployment.md`)
  - `platform/how-to-guides/decommission-a-tenant.md` (new, covers I-0106 / T-0581)
  - `platform/how-to-guides/require-signed-packages.md` (new, covers I-0103)
  - `platform/how-to-guides/use-cli-profiles.md` (new, covers I-0098 / T-0538)
  - `platform/reference/api-error-envelope.md` (new, optional — recommend fold into `http-api.md`)
  - `platform/explanation/observability.md` (new, covers I-0099 / I-0108 / I-0109)
  - `platform/explanation/security-model.md` (new, optional — overlaps existing multi-tenancy + security how-tos)
- **Effort distribution:** Roughly 6 L-effort docs (multi-tenancy rewrite, http-api rewrite, deploying-the-api-server rewrite, configure-multi-tenant-deployment rewrite, package-signing rewrite, repository-structure rewrite), 13 M-effort, 12+ S-effort. Total work concentrates in: HTTP API reference accuracy, multi-tenancy + tenant teardown, and the operations-to-platform fold-in. Recommend grouping into DOC-B (platform reference refresh), DOC-C (platform how-to + tutorials), DOC-D (platform explanation), DOC-H (operations fold-in) per the initiative's Phase 3 cluster outline.
