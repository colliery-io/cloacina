---
id: cloacina-server-deployable
level: initiative
title: "Cloacina Server - Deployable Workflow Infrastructure"
short_code: "CLOACI-I-0018"
created_at: 2026-01-28T04:52:29.855288+00:00
updated_at: 2026-01-28T04:52:29.855288+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
strategy_id: NULL
initiative_id: cloacina-server-deployable
---

# Cloacina Server - Deployable Workflow Infrastructure Initiative

*This template includes sections for various types of initiatives. Delete sections that don't apply to your specific use case.*

## Context

Cloacina currently exists as an embedded library - developers integrate it directly into their Rust or Python applications. While this model works well for many use cases, there's demand for a deployable service model where:

- Platform teams can provide workflow infrastructure for multiple teams/organizations
- Implementation partners can run managed workflow services for clients
- Teams can author workflows locally and deploy to a central server for execution

The core engine already supports the fundamentals: stateless task execution, database-backed state, multi-tenant schema isolation, and atomic task claiming. What's missing is the "application layer" - the server binary, API surface, and deployment artifacts that turn the library into infrastructure.

## Goals & Non-Goals

**Goals:**
- Create a deployable server binary (`cloacina-server`) with modular architecture
- Implement REST and/or gRPC API for workflow submission, status queries, and management
- Enable horizontal scaling of workers (stateless task executors)
- Enable horizontal scaling of schedulers (via pipeline claiming)
- Multi-tenant by default using existing schema isolation
- Provide deployment artifacts (Docker image, configuration examples)
- Support the local-to-server workflow: author locally, compile, upload to server

**Non-Goals:**
- Web UI for workflow management (future initiative)
- Workflow authoring in the server itself (authoring remains local)
- Support for SQLite in service mode (Postgres required for multi-tenancy)
- Managed cloud offering (this is self-hosted infrastructure)
- Changes to the core library API (service builds on existing capabilities)

## Architecture

### Deployment Model: Modular Monolith

Single binary with multiple operational modes:

```
cloacina-server --mode=all        # Everything in one process (small deployments)
cloacina-server --mode=api        # API surface only (stateless, scale freely)
cloacina-server --mode=worker     # Task execution only (stateless, scale freely)
cloacina-server --mode=scheduler  # Scheduling/dispatch (scale via pipeline claiming)
```

### Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     cloacina-server                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   API       │  │  Scheduler  │  │      Workers        │  │
│  │  (REST/gRPC)│  │   Loop      │  │  (Task Executors)   │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│         └────────────────┴─────────────────────┘             │
│                          │                                   │
│                    ┌─────▼─────┐                             │
│                    │  Database │                             │
│                    │ (Postgres)│                             │
│                    └───────────┘                             │
└─────────────────────────────────────────────────────────────┘
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

### Workflow Package Format

Package format design is covered by separate initiatives:
- **CLOACI-I-0019**: Slim Rust package FFI (reduce binary bloat)
- **CLOACI-I-0020**: Python/Cloaca package format (wheel-style vendored deps)

The server accepts `.cloacina` packages (tar.gz) with a unified manifest schema supporting both Rust and Python workflows. Package loading integrates with existing PackageLoader/registry infrastructure.

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

### Phase 1: Foundation
- [ ] Create `cloacina-server` crate in workspace
- [ ] Implement CLI with mode selection (clap)
- [ ] Basic server startup/shutdown lifecycle
- [ ] Configuration loading (config file + env vars)

### Phase 2: Core Services
- [ ] Integrate existing scheduler with pipeline claiming
- [ ] Integrate existing worker/executor
- [ ] Add recovery manager with claiming pattern
- [ ] Health check endpoint

### Phase 3: API Surface
- [ ] Design API schema (REST and/or gRPC)
- [ ] Workflow package upload endpoint
- [ ] Execution submission endpoint
- [ ] Status query endpoints
- [ ] Execution control endpoints (pause/resume/cancel)

### Phase 4: Multi-tenancy
- [ ] Tenant provisioning API
- [ ] Per-tenant configuration
- [ ] Optional tenant affinity for schedulers

### Phase 5: Observability & Operations
- [ ] Prometheus metrics endpoint
- [ ] Structured logging with context
- [ ] OpenTelemetry tracing integration

### Phase 6: Deployment
- [ ] Dockerfile with multi-stage build
- [ ] docker-compose example
- [ ] Configuration documentation
- [ ] Getting started guide

## Related Initiatives

- **CLOACI-I-0019**: Slim Packaged Workflow FFI Interface (Rust package optimization)
- **CLOACI-I-0020**: Cloaca Workflow Package Format (Python packaging)

## Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| API Protocol | REST | CLI/CI clients, curl-ability, file uploads |
| Auth Model | PAK + ABAC | Simple keys, flexible policies, no IdP dependency |
| Tenant Provisioning | API-driven | Dynamic, integrates with external systems |
| Metrics Focus | Resource utilization + system pressures | Operational visibility for scaling decisions |
| ABAC Enforcement | Layered middleware + handlers | Auth in middleware, coarse perms in route layers, resource-specific checks in handlers |
