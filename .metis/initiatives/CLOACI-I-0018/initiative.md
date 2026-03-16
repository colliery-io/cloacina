---
id: cloacina-server-deployable
level: initiative
title: "Cloacina Server - Deployable Workflow Infrastructure"
short_code: "CLOACI-I-0018"
created_at: 2026-01-28T04:52:29.855288+00:00
updated_at: 2026-03-16T01:34:48.614448+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: cloacina-server-deployable
---

# Cloacina Server - Deployable Workflow Infrastructure Initiative

## Context

Cloacina currently exists as an embedded library — developers integrate it directly into their Rust or Python applications. While this model works well for many use cases, there's demand for a deployable service model where:

- Platform teams can provide workflow infrastructure for multiple teams/organizations
- Implementation partners can run managed workflow services for clients
- Teams can author workflows locally and deploy to a central server for execution

The core engine is mature. Since this initiative was first written (Jan 2026), significant new capability has landed:

### What Already Exists (Audit: 2026-03-16)

| Component | State | Location |
|---|---|---|
| **DefaultRunner** | Complete modular monolith | `runner/default_runner/mod.rs` — composes TaskScheduler, ThreadTaskExecutor, DefaultDispatcher, CronScheduler, CronRecovery, WorkflowRegistry, RegistryReconciler, TriggerScheduler |
| **Task claiming** | Complete | `dal/unified/task_execution/claiming.rs` — `FOR UPDATE SKIP LOCKED` at task level |
| **Work distribution** | Complete | `dispatcher/work_distributor.rs` — LISTEN/NOTIFY for Postgres, polling for SQLite |
| **Schema multi-tenancy** | Complete | `database/connection/` — per-tenant Postgres schemas, schema validation |
| **Package format** | Complete | `.cloacina` tar.gz packages, PackageLoader, I-0019 and I-0020 both completed and archived |
| **Package signing** | Complete | `security/` — Ed25519 signing, key lifecycle, verification, audit trails |
| **Recovery** | Complete | `task_scheduler/recovery.rs` + `cron_recovery.rs` — orphan detection, retry, abandonment |
| **Continuous scheduling** | Complete | `continuous/` — 17 files, reactive graph with boundary WAL, detector state, crash recovery (I-0023/I-0024/I-0025) |
| **`cloacinactl` CLI** | Complete | `cloacinactl/` — package sign/verify/inspect, key management, admin cleanup |
| **Structured logging** | Complete | `tracing` throughout, span instrumentation in services |

### What's Missing (the "server" part)

| Component | State |
|---|---|
| **HTTP/REST API** | Zero HTTP deps in any Cargo.toml. No axum, tower, hyper. |
| **API key auth (PAK + ABAC)** | Not implemented. Only package signing exists. |
| **Pipeline-level claiming** | Only task claiming exists. No `last_scheduled_at`/`last_scheduled_by` on pipelines. |
| **API-driven tenant provisioning** | Tenants hardcoded at startup, not runtime-provisioned. |
| **Prometheus metrics** | Zero metrics infrastructure. |
| **OpenTelemetry** | Not present. |
| **Deployment modes** | Not implemented. |
| **Production Dockerfile** | Only test Dockerfile exists. |
| **Server configuration** | No TOML/YAML config parsing. |
| **Continuous scheduling in server** | Config fields exist on DefaultRunnerConfig but ContinuousScheduler not wired into background services. |

### Key Decision: `cloacinactl` as Unified Binary

Rather than creating a separate `cloacina-server` crate, extend `cloacinactl` (already has clap CLI infrastructure) with a `serve` subcommand:

```
cloacinactl serve --mode=all          # Everything in one process
cloacinactl serve --mode=api          # API surface only
cloacinactl serve --mode=worker       # Task execution only
cloacinactl serve --mode=scheduler    # Scheduling/dispatch only
cloacinactl package build|sign|verify # Existing package commands
cloacinactl key generate|list|export  # Existing key commands
cloacinactl admin cleanup-events      # Existing admin commands
```

This avoids a new crate, reuses existing CLI infrastructure, and gives operators a single tool for both administration and serving.

## Goals & Non-Goals

**Goals:**
- Extend `cloacinactl` with `serve` subcommand supporting `--mode=all|api|worker|scheduler`
- Implement REST API (axum) for workflow submission, status queries, execution control, and management
- Enable horizontal scaling of workers (stateless task executors, existing task claiming)
- Enable horizontal scaling of schedulers (new pipeline-level claiming via `FOR UPDATE SKIP LOCKED`)
- API-driven multi-tenant provisioning using existing schema isolation
- API key authentication (Prefixed API Keys) with ABAC authorization
- Prometheus metrics endpoint with essential operational metrics
- OpenTelemetry tracing integration
- Wire ContinuousScheduler into server background services (graph assembly from packages, startup restore, shutdown)
- Package upload via REST API (multipart → validate signature → registry reconciliation)
- Server configuration via TOML/YAML + env vars + CLI flag overrides
- Production Dockerfile (multi-stage build) and docker-compose example
- Support the local-to-server workflow: author locally, compile, sign, upload to server

**Non-Goals:**
- Web UI for workflow management (future initiative)
- Workflow authoring in the server itself (authoring remains local)
- Support for SQLite in service mode (Postgres required for multi-tenancy)
- Managed cloud offering (this is self-hosted infrastructure)
- Separate `cloacina-server` binary (extend `cloacinactl` instead)
- gRPC API (can be added later if demand emerges; REST is sufficient for CLI/CI clients)

## Architecture

### Deployment Model: Modular Monolith via `cloacinactl serve`

Single binary (`cloacinactl`) with serve subcommand and mode selection:

```
cloacinactl serve --mode=all        # Everything in one process (small deployments)
cloacinactl serve --mode=api        # API surface only (stateless, scale freely)
cloacinactl serve --mode=worker     # Task execution only (stateless, scale freely)
cloacinactl serve --mode=scheduler  # Scheduling/dispatch (scale via pipeline claiming)
```

`DefaultRunner` already composes all engine subsystems. The serve command wraps it with HTTP, auth, and lifecycle management.

### Component Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│                       cloacinactl serve                               │
├──────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌──────────────────────┐  ┌─────────────────────┐  │
│  │   API       │  │  DefaultRunner       │  │      Workers        │  │
│  │  (axum)     │  │  ├─ TaskScheduler    │  │  (ThreadTaskExec)   │  │
│  │             │  │  ├─ CronScheduler    │  │                     │  │
│  │  REST       │  │  ├─ TriggerScheduler │  │  Claim tasks via    │  │
│  │  PAK Auth   │  │  ├─ ContinuousSched  │  │  SKIP LOCKED        │  │
│  │  ABAC       │  │  ├─ Recovery         │  │                     │  │
│  └──────┬──────┘  │  └─ RegistryRecon    │  └──────────┬──────────┘  │
│         │         └──────────┬───────────┘              │             │
│         │                    │                          │             │
│         └────────────────────┴──────────────────────────┘             │
│                              │                                       │
│                    ┌─────────▼─────────┐  ┌──────────────────────┐    │
│                    │  Database         │  │  Prometheus /metrics  │    │
│                    │  (Postgres)       │  │  OTel tracing        │    │
│                    │  LISTEN/NOTIFY    │  │  Structured JSON logs │    │
│                    └───────────────────┘  └──────────────────────┘    │
└──────────────────────────────────────────────────────────────────────┘
```

### Scaling Model

| Component | State | Scaling Strategy |
|-----------|-------|------------------|
| API | Stateless | Load balancer, add instances |
| Workers | Stateless | Add instances, they claim tasks via `FOR UPDATE SKIP LOCKED` |
| Schedulers | Stateless | Add instances, they claim pipelines via `FOR UPDATE SKIP LOCKED` |
| Recovery | Stateless | Same claiming pattern as schedulers |
| Database | Source of truth | Postgres with connection pooling |

### Pipeline Claiming (Scheduler Throughput Scaling)

Multiple schedulers claim batches of pipelines to process:

```sql
WITH claimable AS (
    SELECT id FROM pipeline_executions
    WHERE status IN ('Pending', 'Running')
    AND (last_scheduled_at IS NULL OR last_scheduled_at < NOW() - INTERVAL '1 second')
    ORDER BY last_scheduled_at NULLS FIRST
    LIMIT 100
    FOR UPDATE SKIP LOCKED
)
UPDATE pipeline_executions
SET last_scheduled_at = NOW(), last_scheduled_by = $scheduler_id
FROM claimable
WHERE pipeline_executions.id = claimable.id
RETURNING pipeline_executions.*;
```

This provides throughput scaling (not just HA) - multiple schedulers actively process work in parallel.

### Optional: Tenant Affinity for QoS

For noisy-neighbor protection, schedulers can be tagged with tenant affinity:

```
cloacina-server --mode=scheduler --tenants=tenant_a,tenant_b
```

Default mode claims from all tenants (maximum efficiency). Tenant affinity mode provides dedicated capacity guarantees.

### Multi-Tenancy

- Schema-based isolation (existing capability)
- Each tenant gets a separate Postgres schema
- Workers can be shared across tenants or dedicated per tenant

**Tenant Provisioning: API-driven**

Dynamic provisioning via API rather than static config files. This enables:
- Self-service tenant creation (with appropriate auth)
- Integration with external provisioning systems
- Runtime tenant management without server restarts

```
POST   /tenants                 # Create tenant (creates schema, initial API key)
GET    /tenants                 # List tenants
GET    /tenants/{tenant}        # Get tenant details
DELETE /tenants/{tenant}        # Deprovision tenant (soft delete? archive?)
```

## Detailed Design

### Workflow Package Format (Completed)

Package format is fully implemented (I-0019 and I-0020 both completed and archived):
- `.cloacina` packages (tar.gz with library + manifest)
- PackageLoader, WorkflowRegistry, RegistryReconciler all operational
- Ed25519 package signing and verification
- Rust and Python package formats both supported

The server needs: multipart upload endpoint → signature verification → store in registry → reconcile across workers. The reconciliation infrastructure already exists — the gap is the HTTP upload endpoint and cross-instance registry sync.

### Continuous Scheduling in Server Mode

ContinuousScheduler (I-0023/I-0024/I-0025, all completed) must run as a background service alongside the existing schedulers. The serve command needs to:

1. **Graph assembly from packages**: When workflow packages are loaded, extract `DataSource` registrations and `ContinuousTaskRegistration` declarations to assemble the `DataSourceGraph`
2. **Startup restore sequence**: `init_drain_cursors()` → `restore_pending_boundaries()` → `restore_from_persisted_state()` → `restore_detector_state()`
3. **Wire into DefaultRunner**: Add ContinuousScheduler to `start_background_services()` when `enable_continuous_scheduling` is true
4. **Graceful shutdown**: ContinuousScheduler respects the same watch-channel shutdown signal as other services
5. **Graph hot-reload**: When new packages are uploaded, the continuous graph may need to be re-assembled (or rejected if it would change active graph topology — decided against hot-reload per S-0001)
6. **Metrics**: ContinuousScheduler.graph_metrics() exposed via the metrics endpoint

### API Surface

**Decision: REST API**

Rationale:
- Primary clients are CLI tools and CI/CD pipelines - REST is sufficient
- Package upload is a key operation - REST handles files naturally via multipart
- Debugging/operations benefit from curl-ability
- No high-throughput service-to-service use case anticipated
- gRPC can be added later if demand emerges

Implementation approach:
- OpenAPI spec as source of truth (design-first)
- JSON for request/response bodies
- Multipart for file uploads
- SSE for status streaming (future)

Core API operations:
- Tenant management (create, list, configure)
- Workflow package upload/management
- Execution submission (trigger workflow run)
- Status queries (pipeline status, task status)
- Execution control (pause, resume, cancel)
- Observability endpoints (metrics, health)

### Authentication/Authorization

**Decision: Prefixed API Keys (PAK) + ABAC**

Rationale:
- Matches self-hosted, platform team use case
- CI/CD pipelines integrate easily with API keys
- No external dependencies (no IdP required)
- ABAC provides flexible, extensible authorization
- OAuth/OIDC can be added later for enterprises without breaking existing integrations

Key format (PAK scheme):
```
cloacina_<environment>_<tenant>_<key_id>

# Examples
cloacina_live_acme_k7f3a9b2c    # Production
cloacina_test_acme_k8e4b1d3f    # Test
```

ABAC attributes (normalized tables, Diesel-friendly):

```sql
-- Core permissions as columns on api_keys table
CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    key_hash TEXT NOT NULL,           -- Hashed key for lookup
    key_prefix VARCHAR(32) NOT NULL,  -- First 8 chars for identification
    can_execute BOOLEAN NOT NULL DEFAULT true,
    can_upload BOOLEAN NOT NULL DEFAULT false,
    can_manage_keys BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

-- Workflow pattern restrictions (many-to-one)
CREATE TABLE api_key_workflow_patterns (
    id UUID PRIMARY KEY,
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL  -- e.g., "etl::*", "reports::*"
);
-- Empty patterns table = unrestricted access
-- Non-empty = allow only matching workflows
```

Policy evaluation (layered middleware + handlers):

| Layer | Check | Implementation |
|-------|-------|----------------|
| Middleware | Valid API key | Tower middleware, fail fast on invalid |
| Middleware | Tenant isolation | Extract tenant from key, inject into request context |
| Route layer | `can_execute`, `can_upload`, `can_manage_keys` | axum layer on route groups |
| Handler | Workflow pattern matching | Handler logic, needs specific resource context |

This separation keeps middleware simple (auth + tenant) while allowing resource-specific authorization close to where it's enforced. Workflow pattern checks require the specific workflow name, so they must happen in handlers.

API for key management:
```
POST   /tenants/{tenant}/api-keys      # Create key (returns secret once)
GET    /tenants/{tenant}/api-keys      # List keys (metadata only)
DELETE /tenants/{tenant}/api-keys/{id} # Revoke key
```

### Configuration

Server configuration via:
- Config file (TOML/YAML)
- Environment variables
- CLI flags (override)

Key configuration areas:
- Database connection
- Mode selection (all/api/worker/scheduler)
- Tenant affinity (optional)
- Resource limits (concurrency, timeouts)
- Observability (metrics endpoint, log level)

### Observability

- **Metrics**: Prometheus-compatible endpoint
- **Tracing**: OpenTelemetry integration
- **Logging**: Structured JSON logs

**Essential Metrics for v1 (resource utilization + system pressures):**

Resource utilization:
- `cloacina_workers_active` - Currently executing tasks per worker
- `cloacina_workers_total_capacity` - Max concurrent tasks across workers
- `cloacina_db_connections_active` - Database pool utilization
- `cloacina_db_connections_idle` - Available connections

System pressures:
- `cloacina_scheduler_claim_batch_size` - Pipelines claimed per scheduler cycle
- `cloacina_scheduler_claim_duration_seconds` - Time to claim pipeline batch
- `cloacina_task_queue_depth` - Tasks in Ready state waiting for workers
- `cloacina_task_claim_wait_seconds` - Time tasks wait before being claimed
- `cloacina_task_execution_duration_seconds` - Histogram of task durations

Health indicators:
- `cloacina_pipelines_active` - Currently running pipelines
- `cloacina_pipelines_pending` - Pipelines waiting to start
- `cloacina_tasks_failed_total` - Counter of failed tasks
- `cloacina_recovery_orphaned_tasks` - Tasks recovered from stuck state

Per-tenant (if cardinality is manageable):
- `cloacina_tenant_pipelines_active{tenant="..."}` - Active pipelines per tenant
- `cloacina_tenant_tasks_executed_total{tenant="..."}` - Task throughput per tenant

### Deployment Artifacts

- Docker image (multi-stage build)
- Example docker-compose for local testing
- Helm chart (future)
- Configuration examples

## Alternatives Considered

### Deployment Model Alternatives

**Separate microservices** (api, scheduler, worker as distinct binaries):
- Pros: Clear separation, deploy only what you need
- Cons: More artifacts to build/version/distribute, operational complexity
- Decision: Modular monolith gives same flexibility with single artifact

**Leader election for schedulers** (one active, others standby):
- Pros: Simple coordination
- Cons: Only provides HA, not throughput scaling
- Decision: Pipeline claiming provides true horizontal scaling

**External coordination service** (etcd, Consul, ZooKeeper):
- Pros: Battle-tested coordination primitives
- Cons: Another system to operate, violates "database as coordination" principle
- Decision: Use Postgres for all coordination via row-level locking

### Multi-tenancy Alternatives

**Tenant partitioning for schedulers** (assign tenants to specific schedulers):
- Pros: Strong isolation, dedicated capacity
- Cons: Uneven load distribution, rebalancing complexity
- Decision: Offer as optional QoS mode, default to shared claiming for efficiency

## Implementation Plan

### Phase 1: Foundation — `cloacinactl serve`
- [ ] Add `serve` subcommand to `cloacinactl` with `--mode` flag (all/api/worker/scheduler)
- [ ] Add axum + tower + tokio-signal dependencies
- [ ] Server configuration: TOML config file parser with env var overrides and CLI flag overrides
- [ ] Config struct: bind address, port, database_url, mode, log_level, concurrency limits
- [ ] Basic HTTP server startup/shutdown lifecycle with graceful shutdown (tokio signal handling)
- [ ] Health endpoint: `GET /health` (returns 200 + version info)
- [ ] Wire DefaultRunner into serve lifecycle (start background services, stop on shutdown)

### Phase 2: Continuous Scheduling in Server
- [ ] Wire ContinuousScheduler into DefaultRunner `start_background_services()` when enabled
- [ ] Graph assembly from loaded workflow packages (extract DataSource + ContinuousTaskRegistration)
- [ ] Startup restore sequence: init_drain_cursors → restore_pending_boundaries → restore_from_persisted_state → restore_detector_state
- [ ] Graceful shutdown: ContinuousScheduler respects watch-channel shutdown signal
- [ ] Integration test: serve mode starts and stops with continuous scheduling enabled

### Phase 3: Authentication — PAK + ABAC
- [ ] Database migration: `api_keys` table with PAK fields and ABAC permissions
- [ ] Database migration: `api_key_workflow_patterns` table
- [ ] Database migration: `tenants` table (if not already schema-level)
- [ ] PAK key generation: `cloacina_<env>_<tenant>_<key_id>` format
- [ ] Key hash storage (argon2 or bcrypt)
- [ ] DAL: create_key, validate_key, revoke_key, list_keys
- [ ] Tower middleware: extract API key from `Authorization: Bearer` header
- [ ] Tower middleware: validate key, extract tenant, inject into request extensions
- [ ] Route-level layers: `can_execute`, `can_upload`, `can_manage_keys` permission checks
- [ ] Handler-level: workflow pattern matching against `api_key_workflow_patterns`

### Phase 4: Core REST API
- [ ] OpenAPI spec (design-first)
- [ ] `POST /workflows/packages` — multipart upload → verify signature → store in registry
- [ ] `GET /workflows` — list registered workflows (filterable by tenant)
- [ ] `POST /executions` — trigger workflow run (returns execution ID)
- [ ] `GET /executions/{id}` — pipeline status + task statuses
- [ ] `GET /executions` — list executions (filterable by status, workflow, tenant)
- [ ] `POST /executions/{id}/pause` — pause running pipeline
- [ ] `POST /executions/{id}/resume` — resume paused pipeline
- [ ] `DELETE /executions/{id}` — cancel execution
- [ ] Error responses: consistent JSON error format with codes

### Phase 5: Tenant Management API
- [ ] `POST /tenants` — create tenant (creates Postgres schema, returns initial API key)
- [ ] `GET /tenants` — list tenants
- [ ] `GET /tenants/{tenant}` — tenant details + stats
- [ ] `DELETE /tenants/{tenant}` — deprovision (soft delete / archive)
- [ ] `POST /tenants/{tenant}/api-keys` — create key (returns secret once)
- [ ] `GET /tenants/{tenant}/api-keys` — list keys (metadata only, no secrets)
- [ ] `DELETE /tenants/{tenant}/api-keys/{id}` — revoke key
- [ ] Optional tenant affinity flag: `--tenants=a,b` for scheduler QoS

### Phase 6: Pipeline Claiming (Scheduler Scaling)
- [ ] Database migration: add `last_scheduled_at`, `last_scheduled_by` to `pipeline_executions`
- [ ] DAL: `claim_pipeline_batch(scheduler_id, limit)` with `FOR UPDATE SKIP LOCKED`
- [ ] Modify scheduler loop to claim pipelines in batches instead of scanning all
- [ ] Integration test: two schedulers claim non-overlapping pipeline batches
- [ ] Verify existing task claiming still works within claimed pipeline scope

### Phase 7: Observability
- [ ] Add `metrics` + `prometheus` crate dependencies
- [ ] Instrument DefaultRunner services with counters/histograms/gauges
- [ ] `GET /metrics` — Prometheus-compatible scrape endpoint
- [ ] Essential metrics (from spec): workers_active, db_connections, task_queue_depth, claim_batch_size, execution_duration, pipelines_active/pending, tasks_failed, recovery_orphaned
- [ ] Per-tenant metrics with tenant label (when cardinality manageable)
- [ ] OpenTelemetry: add `opentelemetry` + `tracing-opentelemetry` dependencies
- [ ] Configure OTLP exporter via config (optional, disabled by default)
- [ ] Continuous scheduling metrics: graph_metrics() exposed at `/metrics`

### Phase 8: Deployment Artifacts
- [ ] Multi-stage Dockerfile (builder + runtime)
- [ ] docker-compose.yml: cloacinactl serve + postgres
- [ ] Example config file: `cloacina.toml` with all options documented
- [ ] Getting started guide: from zero to running server
- [ ] Architecture documentation: scaling model, deployment patterns
- [ ] Helm chart scaffold (future — not blocking)

## Related Initiatives

- **CLOACI-I-0019**: Slim Packaged Workflow FFI Interface — **Completed, archived**
- **CLOACI-I-0020**: Cloaca Workflow Package Format — **Completed, archived**
- **CLOACI-I-0023/I-0024/I-0025**: Continuous Scheduling — **Completed, archived** (server must wire this in)
- **CLOACI-I-0026**: Python/Cloaca Continuous Tasks — **Discovery** (server must support Python tasks in the continuous graph)

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Binary | Extend `cloacinactl` with `serve` | Single tool for admin + serving, reuses existing clap infrastructure, avoids new crate |
| HTTP Framework | axum | Tower middleware composability is superior. Tokio-native. |
| API Protocol | REST | CLI/CI clients, curl-ability, file uploads. gRPC later if needed. |
| Config Format | TOML only | Rust ecosystem convention (Cargo.toml precedent). No YAML. |
| Auth Model | PAK + ABAC from day 1 | Full ABAC designed upfront — workflow patterns, per-key permissions, tenant scoping. Not simplified first. |
| Tenant Provisioning | API-driven | Dynamic, integrates with external systems, runtime management |
| API Spec | Code-first (utoipa) | Generate OpenAPI from axum route definitions. Faster iteration. Design still happens, just not in YAML first. |
| Continuous Scheduling | Phase 2 (early) | Wire into DefaultRunner before API work. More integration surface upfront but catches issues early. |
| Metrics Focus | Resource utilization + system pressures | Operational visibility for scaling decisions |
| ABAC Enforcement | Layered middleware + handlers | Auth in middleware, coarse perms in route layers, resource-specific checks in handlers |
| Continuous Scheduling | Wire into DefaultRunner background services | ContinuousScheduler is a peer to CronScheduler/TriggerScheduler, same lifecycle |
| Graph Hot-Reload | Rejected (per S-0001) | Restart required for graph topology changes. New packages trigger reconciliation, not live graph mutation. |
| Phasing | All 8 phases planned. Each phase can be its own initiative if needed. | Full scope, incremental delivery. |
