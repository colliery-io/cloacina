# Design — CLOACI-I-0112 (Phase 2 Deliverable)

> Load-bearing artifact between Phase 1 (discovery, captured in `initiative.md`) and Phase 3 (decompose into write tasks).

Per-doc audits live in four supplementary files in this directory:
- [audit-workflows.md](./audit-workflows.md) — 41 docs in `workflows/`
- [audit-computation-graphs.md](./audit-computation-graphs.md) — 26 docs in `computation-graphs/`
- [audit-platform.md](./audit-platform.md) — 31 docs in `platform/`
- [audit-python-misc.md](./audit-python-misc.md) — 33 docs in `python/`, `quick-start/`, top-level, `contributing/`, operations fold-in targets, README

Total docs in scope: **131 existing + ~22 new = ~153**.

## Cross-cutting drift patterns

Nine systemic findings, ordered roughly by severity:

### 1. S-0011 nomenclature drift — concentrated in `computation-graphs/`

16 banned-phrase / stale-CLI-verb / stale-route instances in `computation-graphs/`. Platform/ is **already clean** (post-T-0528 sweep landed there). Workflows/ has 3 "reactive" adjective uses in English-language context (`workflows/how-to-guides/sequential-strategy.md:27`, `workflows/how-to-guides/testing-workflows.md:178`, `workflows/reference/testing-crate.md:142`) — borderline; Diataxis reviewer's call. Python/ has 1 (`python/_index.md:21` "Reactive graphs"). Glossary has 1 (`glossary.md:39` Computation Graph entry).

Operations source files (folding in via DOC-H) contribute 2 more: `metrics.md:202` (`/v1/health/reactors`), `metrics.md:196` ("reactive stack").

Pre-known specific instances surfaced before agent runs:
- `computation-graphs/_index.md:9` — "reactive execution engine"
- `computation-graphs/explanation/reactor-lifecycle.md:46,73` — `cloacinactl reactor force-fire` (additionally **broken**: that verb doesn't exist; force-fire is WebSocket `ManualCommand::ForceFire` only)

**Total NOM to fix: ~23.**

### 2. HTTP response-shape drift on `/v1/health/graphs`

Server returns `{"items": [...], "total": N}` per `crates/cloacina-server/src/routes/health_graphs.rs:127-131`. Docs claim `{"graphs": [...]}` or `{"reactors": [...]}`:
- `computation-graphs/how-to-guides/computation-graph-health.md:84-107`
- `computation-graphs/tutorials/service/07-packaging.md:289-318`
- `computation-graphs/tutorials/service/09-kafka-stream.md:275`

Additionally, several docs reference `fire_count` and `last_fired_at` per-graph fields that **do not exist** in the current response shape (`computation-graph-health.md`, `08-websocket-events.md`) — likely aspirational at authoring time and never landed.

### 3. ApiError envelope drift

Server emits `{"error": "...", "code": "..."}` per `crates/cloacina-server/src/routes/error.rs:80-88` and propagates `x-request-id`. Docs show bare `{"error": "..."}`:
- `platform/reference/http-api.md` — every error block
- `workflows/how-to-guides/monitoring-executions.md` — missing entirely + URL paths missing `/v1/` prefix
- All consumer-side how-tos

Additionally, `POST /v1/tenants` body documented as `{schema_name, username, password}` but the actual route accepts `{name, description?, password?}` (per `crates/cloacina-server/src/routes/tenants.rs:51-68`, T-0594 / API-01 fix).

### 4. Metric naming drift (`pipelines` → `workflows`)

Emitted names use `workflows` per `crates/cloacina/src/execution_planner/scheduler_loop.rs:362-394`:
- `cloacina_workflows_total` (NOT `cloacina_pipelines_total`)
- `cloacina_active_workflows`
- `cloacina_workflow_duration_seconds`

Docs reference the old names:
- `platform/reference/environment-variables.md:148-154`
- `platform/how-to-guides/performance-tuning.md:407`
- `workflows/explanation/cron-scheduling.md:580-642` — uses an entirely fabricated `CronMetrics::collect()` type

### 5. Removed-API references still in docs

Three significant removals not reflected in docs:

**T-0502 `RecoveryManager` removed** — heartbeat sweeper is sole task-recovery path. Stale references:
- `workflows/explanation/architecture-overview.md:244`
- `workflows/explanation/task-execution-sequence.md:383-401`
- `workflows/explanation/guaranteed-execution-architecture.md`
- `platform/explanation/horizontal-scaling.md:110`
- `platform/how-to-guides/performance-tuning.md:119-120`
- `troubleshooting.md:153-167`

**I-0096 `#[ctor]` removed** — inventory is the registration mechanism. Stale references:
- `workflows/tutorials/library/01-first-workflow.md:75-77`
- `workflows/tutorials/service/05-cron-scheduling.md:60` (Cargo.toml still lists `ctor = "0.2"`)
- `computation-graphs/reference/computation-graphs.md:818-855` (entire "Global Registry" subsection)

**Fabricated APIs** in `workflows/explanation/cron-scheduling.md`:
- `MissedExecutionPolicy::Execute/Skip/ExecuteWithDelay` (lines 319-352) — never existed; real type is `CatchupPolicy::Skip/RunAll`
- `DistributedCronScheduler`, `try_acquire_leader_lease`, `DistributedCronExecutor` (lines 464-573) — never existed; real coordination is atomic `claim_and_update` on `cron_schedules`
- `CronSchedule::new_with_timezone` (lines 413-432) — wrong constructor; timezone lives on `Schedule`/`NewSchedule`
- `CronMetrics::collect()` + `HealthStatus` types (lines 580-642) — fabricated; real metrics live in `cloacina_*` namespace

### 6. `cloacina-ctl` → `cloacinactl` rename (T-0538)

Multiple docs use the legacy `cloacina-ctl` binary/crate name or the deprecated `cloacinactl serve` verb:
- `workflows/tutorials/service/07-packaged-workflows.md` (extensive — every CLI invocation)
- `workflows/tutorials/service/08-workflow-registry.md` (extensive)
- `workflows/explanation/architecture-overview.md:36, 52, 209-212`
- `platform/how-to-guides/deploying-the-api-server.md` (lines 30, 32, 38, 78, 86, 168, 174, 182 — `cloacinactl serve`)
- `platform/how-to-guides/production-deployment.md:8, 79, 84, 114`

Current verbs (per `crates/cloacinactl/src/nouns/server/`, `daemon/`): `cloacinactl server start`, `cloacinactl daemon start`.

### 7. Stale version pins

25+ stale Cargo / Docker / install snippets across:
- 5 workflows tutorials (`cloacina = "0.1.0"` warning blocks)
- 3 platform docs (`0.5.x`, `0.6.0`)
- 2 CG docs (`cloacina-* = "0.1"` or `"0.3"`)
- 2 troubleshooting sections (`cloacina-* = "0.4"`)
- 1 install snippet (`v0.6.0`)

Current workspace: `0.6.1`.

### 8. Org-slug drift (`colliery-io` vs `colliery-software`)

README and install docs disagree:
- `README.md:8` — `colliery-io` (image URL)
- `quick-start/install.md:20` — `colliery-software` (releases URL)
- `docs/operations/compiler-deployment.md:111, 127` — `colliery-io` (image refs)

Pick one. (GitHub org appears to be `colliery-io` based on image URLs; `colliery-software` may be the legal entity.)

### 9. Python api-reference internal disagreement

Three reference files contradict each other on `DefaultRunner` and `@task` parameters:
- `configuration.md:34-41` vs `runner.md:432-438` — config field names disagree (`max_concurrent_workflows` vs `max_concurrent_tasks`; `connection_pool_size` vs `db_pool_size`)
- `configuration.md:107-115` vs `task.md:31-40` — retry config shape disagrees (nested `retry_policy={...}` dict vs flat `retry_attempts=...` kwargs)
- `exceptions.md:13-27` may be entirely aspirational — no `CloacaException` hierarchy in code; actual surface is `PyValueError`/`PyKeyError`/`PyRuntimeError`
- `workflow-builder.md` uses deprecated `register_workflow_constructor` + imperative builder throughout; tutorials use context-manager pattern
- `troubleshooting.md:891` claims `cloaca.features()` exists; not in the verified pymodule

Reconciliation against `crates/cloacina-python/src/` is required.

## Information architecture changes locked in this phase

### Operations fold-in (deletes `docs/operations/` and `docs/SIGSEGV_TROUBLESHOOTING.md`)

- `docs/operations/compiler-deployment.md` (186 lines) → **port + extend** as `docs/content/platform/how-to-guides/compiler-deployment-runbook.md`. Existing `running-the-compiler.md` stays as the short-form how-to; new doc is the long-form runbook covering bare-metal, compose, and K8s deploy with the I-0104 hardening flags, I-0109 metrics/log-retention, and T-0610 Helm subchart added.
- `docs/operations/metrics.md` (226 lines) → **port + extend** as `docs/content/platform/reference/metrics-catalog.md`. Fix NOM in source on the way in (`/v1/health/reactors` → `/v1/health/graphs`; "reactive stack" → "event-driven stack").
- `docs/SIGSEGV_TROUBLESHOOTING.md` (70 lines) → **merge** into `docs/content/troubleshooting.md` as a `### Native crash troubleshooting (historical)` subsection (the existing section 19 already references the historical workaround; replace the external GitHub-URL link with an anchor).

After fold-in: delete the three source files and the `docs/operations/` directory.

### Python IA parity restructure

Current `docs/content/python/` has tutorials split but how-to / reference / explanation are flat; `python/examples/` is a Diataxis violation. Target:

```
python/
  _index.md                   (rewrite — new nav)
  quick-start.md              (edit)
  api-reference/              (unchanged structurally — pdoc-generated)
  workflows/
    tutorials/                (8 tutorials moved from python/tutorials/workflows/)
    how-to-guides/            (4 how-tos moved from python/how-to-guides/, workflow-side by default)
    reference/                (new stub _index.md + stub environment-variables.md)
    explanation/              (new stub _index.md + stub python-runtime-architecture.md)
  computation-graphs/
    tutorials/                (3 tutorials moved from python/tutorials/computation-graphs/)
    how-to-guides/            (new stubs only)
    reference/                (new stubs only)
    explanation/              (new stubs only)
```

`python/examples/basic-workflow.md` folds into `python/workflows/tutorials/00-basic-workflow.md` (the content reads as a learning walkthrough). Old directories deleted: `python/tutorials/`, `python/how-to-guides/`, `python/examples/`.

**15 file moves, ~50 cross-link updates inside moved files, 11 new stubs.**

## New-doc inventory (24 docs)

| Path | Cluster | Category | Covers | Effort |
|---|---|---|---|---|
| `platform/reference/metrics-catalog.md` | DOC-H | Reference | I-0099, I-0108, I-0109 | M |
| `platform/how-to-guides/compiler-deployment-runbook.md` | DOC-H | How-to | I-0097, I-0104, I-0109, T-0610 | M |
| `platform/how-to-guides/decommission-a-tenant.md` | DOC-C | How-to | I-0106, T-0581 | M |
| `platform/how-to-guides/require-signed-packages.md` | DOC-C | How-to | I-0103 | M |
| `platform/how-to-guides/use-cli-profiles.md` | DOC-C | How-to | I-0098, T-0538 | S |
| `platform/explanation/observability.md` | DOC-D | Explanation | I-0099, I-0108, I-0109 | M |
| `platform/explanation/security-model.md` | DOC-D | Explanation | I-0103, I-0104, I-0106, ADR-0005 | M |
| `platform/reference/api-error-envelope.md` | DOC-B | Reference | I-0107 | S |
| `workflows/how-to-guides/decommission-a-tenant.md` | DOC-E | How-to | I-0106 (Rust-side recipe) | M |
| `workflows/how-to-guides/subscribe-workflow-to-reactor.md` | DOC-E | How-to | I-0100 | M |
| `workflows/how-to-guides/invoke-computation-graph-from-workflow.md` | DOC-E | How-to | I-0101 | M |
| `computation-graphs/how-to-guides/filter-reactor-firings-with-cel.md` | DOC-F | How-to | T-0602 | S |
| `computation-graphs/explanation/subscription-fan-out.md` | DOC-F | Explanation | I-0100 + S-0011 topology | M |
| `python/workflows/tutorials/_index.md` | DOC-G | Index | (rewrite of placeholder) | S |
| `python/workflows/reference/_index.md` | DOC-G | Index | (stub) | S |
| `python/workflows/reference/environment-variables.md` | DOC-G | Reference | T-0529, T-0532 | M |
| `python/workflows/explanation/_index.md` | DOC-G | Index | (stub) | S |
| `python/workflows/explanation/python-runtime-architecture.md` | DOC-G | Explanation | T-0529, T-0532 | M |
| `python/computation-graphs/tutorials/_index.md` | DOC-G | Index | (rewrite of placeholder) | S |
| `python/computation-graphs/how-to-guides/_index.md` | DOC-G | Index | (stub) | S |
| `python/computation-graphs/how-to-guides/package-a-python-computation-graph.md` | DOC-G | How-to | I-0101, I-0102 | M |
| `python/computation-graphs/how-to-guides/filter-reactor-subscriptions.md` | DOC-G | How-to | T-0602 (pending Python availability) | M |
| `python/computation-graphs/reference/_index.md` | DOC-G | Index | (stub) | S |
| `python/computation-graphs/reference/topology-dict-schema.md` | DOC-G | Reference | I-0101 | S |
| `python/computation-graphs/explanation/_index.md` | DOC-G | Index | (stub) | S |
| `python/computation-graphs/explanation/python-cg-decorator-surface.md` | DOC-G | Explanation | I-0101, I-0102 | M |

### Additional decisions (locked 2026-05-18)

- **`platform/explanation/security-model.md`** — **write as proposed** (separate file under DOC-D). Single landing page for the trust model (deployment-mode trust, auth roles, bootstrap key invariants, signature rationale, compiler threat model, multi-tenant isolation, `/metrics` unauth trade-off). May overlap multi-tenancy but the human-facing entry point is worth the duplication risk.
- **`platform/reference/api-error-envelope.md`** — **write as separate file** under DOC-B (not folded into `http-api.md`). Single lookup target for client developers: envelope shape, `x-request-id`, full code enumeration by route, HTTP status mapping, retry guidance.
- **`workflows/how-to-guides/sequential-strategy.md`** — **move under DOC-F** to `computation-graphs/how-to-guides/sequential-strategy.md`. Leave a one-line redirect entry in the workflows how-to-guides index.
- **`docs/content/python/examples/basic-workflow.md`** — **move under DOC-G** to `python/workflows/tutorials/00-basic-workflow.md`. Delete the `python/examples/` directory after move.
- **DOC-E and DOC-G cluster sizing** — **keep as single Metis tasks**; cluster owner spins out internal checklist if needed.
- **Org slug** — `colliery-io` is authoritative (per `install.sh:REPO="colliery-io/cloacina"` and consistent README usage). `docs/content/quick-start/install.md:20, 93` is the drift; gets fixed under DOC-A (slug sweep) and DOC-I (install.md edit).
- **`/v1/health/graphs` HTTP rename** — verified shipped in `crates/cloacina-server/src/routes/health_graphs.rs:20-21,97,134`. DOC-A locks docs to this path.

### Deferred / folded proposals

- `workflows/explanation/durable-event-log.md` (Agent A) — **deferred**. The CG-side `subscription-fan-out.md` covers the explanation surface; cross-linking from workflows is enough.
- `computation-graphs/reference/metrics.md` (Agent B) — **collapse** into the parent `platform/reference/metrics-catalog.md` (link from a CG reference pointer).

## Files to delete

- `docs/operations/compiler-deployment.md` (after fold-in)
- `docs/operations/metrics.md` (after fold-in)
- `docs/operations/` (empty directory)
- `docs/SIGSEGV_TROUBLESHOOTING.md` (after merge into `troubleshooting.md`)
- `docs/content/python/tutorials/_index.md` (after tutorial moves)
- `docs/content/python/tutorials/workflows/` (empty after moves)
- `docs/content/python/tutorials/computation-graphs/` (empty after moves)
- `docs/content/python/tutorials/` (empty)
- `docs/content/python/how-to-guides/` (empty after moves)
- `docs/content/python/examples/basic-workflow.md` (folds into `python/workflows/tutorials/00-basic-workflow.md`)
- `docs/content/python/examples/_index.md`
- `docs/content/python/examples/` (empty)

## Phase 3 cluster boundaries

Nine clusters. Ordering minimizes cross-cluster cross-link churn (drift sweep first → ops fold-in second → platform reference third → parallel content clusters → top-level last).

### DOC-A — Drift sweep (NOM + VER + removed-API)

Mechanical pass; lands first.

Files touched (~30):
- All 16 NOM-CG instances in `computation-graphs/` (per audit-computation-graphs.md)
- `python/_index.md:21`, `glossary.md:39`, the 3 English "reactive" uses in workflows (judge-the-call)
- All 25+ VER pins
- Removed-API: T-0502 (RecoveryManager), I-0096 (#[ctor]), fabricated cron APIs — edits to ~10 docs
- Org-slug pick: `colliery-io` vs `colliery-software` — sweep after human picks

Effort: **M**. Single writer task; 2-3 days.

### DOC-H — Operations fold-in

Lands second; creates new files that later clusters cross-link.

Tasks:
1. Port + extend `docs/operations/compiler-deployment.md` → `platform/how-to-guides/compiler-deployment-runbook.md` (add I-0104 hardening flags, I-0109 metrics/log-retention, T-0610 Helm subchart, fix slug)
2. Port + extend `docs/operations/metrics.md` → `platform/reference/metrics-catalog.md` (fix NOM in source; add I-0099 full CG metric set, I-0108 persist-failure counters, I-0109 compiler metrics; cross-reference architecture/observability/performance-tuning)
3. Merge `docs/SIGSEGV_TROUBLESHOOTING.md` into `troubleshooting.md:813-880` as `### Native crash troubleshooting (historical)` subsection
4. Delete `docs/operations/`, `docs/SIGSEGV_TROUBLESHOOTING.md`

Files touched: 3 new in `platform/`, 1 edit (`troubleshooting.md`), 3 deletes. Effort: **M**. 2 days.

### DOC-B — Platform reference refresh

Sequential after DOC-A + DOC-H. Reference is the cross-link target; lands before how-tos / explanations that cite it.

Files (~12 in `platform/reference/`):

| File | Effort | Headline change |
|---|---|---|
| `cli.md` | M | Add missing server/compiler/daemon flags; fix `trigger list` pagination; fix `execution list` default |
| `configuration.md` | S | Note T-0502 sole-recovery semantics; cross-link metrics-catalog |
| `database-admin.md` | S | Note `remove_tenant` is step-4 of HTTP route's 4-step orchestration |
| `environment-variables.md` | M | Add I-0103, I-0106, I-0109 env vars; strip old metric names; defer to metrics-catalog |
| `ffi-vtable.md` | S | Verify-only |
| `http-api.md` | L | **Rewrite** for ApiError envelope; fix `POST /v1/tenants` body; add execution/trigger pagination; fix multi-tenancy caveats; add SSE non-availability note |
| `package-manifest.md` | S | Verify-only; check for I-0102 additions |
| `package-shell-macro.md` | S | Verify-only |
| `repository-structure.md` | M | List all 11 crates; fix Python crate-split claim; refresh examples list |
| `websocket-protocol.md` | S | Verify-only |

Effort: **L** (driven by `http-api.md`). 4-6 days.

### DOC-C — Platform how-to + tutorials

Parallel-eligible after DOC-B.

Files (~17):
- 11 existing how-tos in `platform/how-to-guides/` (including `security/local-development.md` rewrite for I-0103, `security/package-signing.md` rewrite/split for I-0103)
- 1 existing tutorial (`tutorials/01-deploy-a-server.md`)
- 2 indexes
- 3 new how-tos: `decommission-a-tenant.md`, `require-signed-packages.md`, `use-cli-profiles.md`

Heaviest: `configure-multi-tenant-deployment.md` (L — `TenantDatabaseCache never evicts` claim is STALE post-T-0581), `deploying-the-api-server.md` (L — `cloacinactl serve` rename + missing flags), `security/package-signing.md` (L — rewrite + split).

Effort: **L**. ~7 days.

### DOC-D — Platform explanation

Parallel-eligible after DOC-B.

Files (~10):
- 9 existing in `platform/explanation/`
- 1 new: `observability.md`

Heaviest: `multi-tenancy.md` (L — rewrite for I-0106 fail-closed search_path, `remove_tenant` orchestration, `TenantRunnerCache`).

Effort: **L**. ~5 days.

### DOC-E — Workflows refresh

Parallel-eligible. Disjoint file surface from B/C/D/F/G.

Files (~44):
- 41 existing files in `workflows/`
- 3 new how-tos: `decommission-a-tenant.md`, `subscribe-workflow-to-reactor.md`, `invoke-computation-graph-from-workflow.md`

Heaviest L docs: `cron-scheduling.md` (fabricated APIs), `guaranteed-execution-architecture.md` (title/content mismatch + I-0110/T-0487/T-0502 coverage), `monitoring-executions.md` (route drift), `07-packaged-workflows.md` + `08-workflow-registry.md` (cloacina-ctl rename + I-0102 unified shell).

Effort: **L**. ~10 days.

### DOC-F — Computation graphs refresh

Parallel-eligible. Disjoint surface. The S-0011 cleanup mostly happens here (16 of 23 NOM instances), but DOC-A handles the mechanical sweep — DOC-F handles content rewrites that depend on the topology shifts (I-0100, I-0101).

Files (~29):
- 26 existing files in `computation-graphs/`
- 2 new docs: `filter-reactor-firings-with-cel.md`, `subscription-fan-out.md`

Heaviest L docs: `reference/computation-graphs.md` (pre-I-0096 ctor block + stale FFI method count + pre-I-0102 manifest example + version pins), `explanation/architecture.md` (topology rewrite for I-0100/I-0101 + S-0011 framing).

Effort: **L**. ~8 days.

### DOC-G — Python IA parity restructure

Parallel-eligible. Disjoint surface. Highest-risk cluster (touches every Python doc + adds restructure overhead).

Phases within the cluster:
1. Move 15 files into new paths
2. Update ~50 cross-links inside moved files
3. Write 11 new stub files for empty quadrant slots
4. Rewrite `python/_index.md`, `python/quick-start.md`
5. Edit 12 api-reference files (reconcile internal disagreement against `crates/cloacina-python/src/`; refresh cross-links)
6. Rewrite `python/api-reference/workflow-builder.md` (extensive — deprecated pattern throughout)
7. Delete 4 old directories + 2 files

Effort: **L**. ~10 days.

### DOC-I — Top-level, glossary, troubleshooting, contributing, README

Sequential, lands last (cross-links into everything else).

Files (~8):

| File | Effort | Headline change |
|---|---|---|
| `_index.md` | S | Add cloacinactl mention, CG as peer surface, see-also rail |
| `glossary.md` | M | S-0011 sweep; add ~10 missing entries (cloacinactl, cloacina-compiler, cloacina-server, ApiError, CEL predicate, install script, Helm chart, verification org) |
| `troubleshooting.md` | L | Version refresh + I-0096 inventory model + T-0502 recovery + I-0106 + SIGSEGV merge (the merge happens in DOC-H; this is the deeper rewrite) |
| `quick-start/_index.md` | S | Add install one-liner pointer |
| `quick-start/install.md` | S | Fix `--version v0.6.1`; correct `cargo install --tag` → `--rev`; add Docker/Helm pointer |
| `contributing/_index.md` | S | Update operations/metrics.md update step → metrics-catalog.md; add Metis pointer; add S-0011 compliance note |
| `contributing/documentation.md` | M | Fix Diataxis IA description (it's currently wrong about top-level layout); add cross-link policy; add S-0011 note |
| `README.md` | M | Slug audit; refresh repo structure (11 crates); add cloacinactl mention; add Python install; fix `bindings/cloaca-backend/` path |

Effort: **M-L**. ~3 days.

## Phase 3 ordering

```
Day 1-3:   [DOC-A] drift sweep  ──┐
Day 1-2:   [DOC-H] ops fold-in ──┤
                                  ↓
Day 4-9:   [DOC-B] platform reference (sequential — others depend)
                                  ↓
Day 10-17: [DOC-C] platform how-to  ──┐
Day 10-14: [DOC-D] platform explain ──┤
Day 10-19: [DOC-E] workflows refresh ─┤  parallel
Day 10-17: [DOC-F] CG refresh       ──┤
Day 10-19: [DOC-G] Python restructure ┘
                                  ↓
Day 20-22: [DOC-I] top-level + glossary + README
                                  ↓
Day 23-26: [Phase 4] four parallel review agents
                                  ↓
Day 27:    triage findings, close initiative
```

Total wall-clock: **~4 weeks** if all parallel slots run concurrently and review findings are tractable.

## Acceptance criteria per Phase 3 cluster task

Each cluster task transitions to `completed` when:
1. Every file in the cluster scope is edited or written (no `{placeholder}` text).
2. Every code claim in edited content is verified against a specific code path (cite in PR description).
3. Every cross-link added resolves (`angreal docs:build` passes locally).
4. NOM / VER / API findings in the cluster's audit file scope are resolved (resolution table in PR).
5. Cluster owner adds an entry to `.metis/initiatives/CLOACI-I-0112/cluster-log.md` summarizing changes for the Phase 4 reviewers.

## Open questions — RESOLVED 2026-05-18

All Phase 2 open questions resolved. Decisions locked in the "Additional decisions" subsection above.

1. ~~README org slug~~ → **`colliery-io`** (verified via `install.sh:REPO`)
2. ~~Optional new docs~~ → **Write both as proposed** (security-model + api-error-envelope as separate files)
3. ~~sequential-strategy.md move~~ → **Move under DOC-F**
4. ~~basic-workflow target~~ → **`python/workflows/tutorials/00-basic-workflow.md`**
5. ~~Cluster-task sizing~~ → **Keep DOC-E and DOC-G as single tasks**
6. ~~`/v1/health/graphs` rename~~ → **Verified shipped** in `crates/cloacina-server/src/routes/health_graphs.rs`
