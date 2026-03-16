---
id: server-phase-3-authentication-pak
level: initiative
title: "Server Phase 3: Authentication — PAK + ABAC"
short_code: "CLOACI-I-0031"
created_at: 2026-03-16T01:32:34.747687+00:00
updated_at: 2026-03-16T21:06:52.852116+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: server-phase-3-authentication-pak
---

# Server Phase 3: Authentication — PAK + ABAC Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: CLOACI-I-0029 (Foundation — need axum server running)
**Blocks**: CLOACI-I-0032 (Core API), CLOACI-I-0033 (Tenant API)

## Context

The server needs authentication and authorization before any API endpoints are useful. This phase implements Prefixed API Keys (PAK) with Attribute-Based Access Control (ABAC) as the day-1 security model, per I-0018 decisions.

Existing security infrastructure covers package signing (Ed25519) but not HTTP API auth. This is a new subsystem.

## Goals

- PAK key format: `cloacina_<env>_<tenant>_<key_id>`
- Database tables: `tenants`, `api_keys` (4 permission bits), `api_key_workflow_patterns` (ABAC rules)
- Key lifecycle: generate, list, revoke (via API and CLI)
- Tower middleware: extract key from `Authorization: Bearer` header, validate via cached lookup, inject AuthContext
- In-memory auth cache with TTL (default 60s), eager workflow pattern loading, negative caching
- 4 permission bits: `can_read`, `can_write`, `can_execute`, `can_admin`
- 3 scope levels: global (super-admin), tenant, workflow-pattern
- Route-level permission guards (Tower layers per route group)
- Handler-level workflow pattern matching for fine-grained ABAC

## Detailed Design

### Permission Model

| Bit | Meaning | Example Use |
|---|---|---|
| `can_read` | View resources (executions, workflows, status) | Monitoring dashboard |
| `can_write` | Modify resources (upload packages, manage config) | DevOps deployment |
| `can_execute` | Trigger workflow runs, pause/resume/cancel | CI/CD pipeline |
| `can_admin` | Manage tenants, create/revoke keys | Platform team |

### Scope Levels

| Scope | tenant_id | workflow_patterns | Access |
|---|---|---|---|
| Global (super-admin) | NULL | empty | All tenants, all workflows |
| Tenant | set | empty | All workflows in tenant |
| Workflow-scoped | set | non-empty | Only matching workflows in tenant |

### Database Schema

```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    schema_name VARCHAR(255) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id),  -- NULL = global/super-admin
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(32) NOT NULL,
    name VARCHAR(255),
    can_read BOOLEAN NOT NULL DEFAULT true,
    can_write BOOLEAN NOT NULL DEFAULT false,
    can_execute BOOLEAN NOT NULL DEFAULT false,
    can_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);

CREATE TABLE api_key_workflow_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL
);

CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);
CREATE INDEX idx_api_keys_tenant ON api_keys(tenant_id);
```

### PAK Key Format

```
cloacina_<env>_<tenant>_<random>
cloacina_live_acme_k7f3a9b2c1d4e5f6   -- tenant-scoped
cloacina_live__k9a8b7c6d5e4f3g2       -- global (empty tenant)
```

- Prefix (first 8 chars after `cloacina_`) used for cache lookup
- Full key hashed with argon2 for storage
- Secret returned once on creation, never retrievable

### Auth Cache

```rust
struct AuthCache {
    cache: Arc<parking_lot::RwLock<HashMap<String, Vec<CachedKey>>>>,
    ttl: Duration,  // default 60s, configurable
}

struct CachedKey {
    key_hash: String,
    key_id: Uuid,
    tenant_id: Option<Uuid>,
    can_read: bool,
    can_write: bool,
    can_execute: bool,
    can_admin: bool,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    workflow_patterns: Vec<String>,  // pre-loaded with key, no second query
    cached_at: Instant,
}
```

- **Lookup**: prefix → cache hit? → if miss or stale (> TTL), query DB with LEFT JOIN on patterns → cache
- **Negative cache**: invalid prefixes cached as empty vec for TTL (prevents DB hammering)
- **Invalidation**: explicit `cache.invalidate(prefix)` on key create/revoke (instant same-instance)
- **Cross-instance**: revocations propagate within TTL (acceptable for API keys)

### Middleware Stack

```
Request
  │
  ▼
[Layer 1: AuthExtract] — Tower middleware
  Extract Bearer token → prefix lookup in cache → hash compare
  If invalid/expired/revoked → 401 Unauthorized
  Inject AuthContext { key_id, tenant_id, permissions, patterns } into extensions
  │
  ▼
[Layer 2: TenantScope] — Tower middleware
  If tenant_id is Some → set tenant schema in request context
  If global → allow cross-tenant access
  Verify path tenant matches key tenant (unless global) → 403 if mismatch
  │
  ▼
[Route-level: PermissionGuard] — axum layer per route group
  Check required permission bit for route group
  POST /executions → requires can_execute
  POST /workflows/packages → requires can_write
  GET /executions → requires can_read
  POST /tenants → requires can_admin
  │
  ▼
[Handler-level: WorkflowPatternCheck]
  For workflow-specific operations, check glob pattern match
  If patterns non-empty and none match → 403 Forbidden
```

### Public Endpoints (no auth)

- `GET /health`
- `GET /metrics`
- `GET /api-docs/*`

## Implementation Plan

- [ ] Database migration: `tenants`, `api_keys`, `api_key_workflow_patterns` tables (Postgres + SQLite)
- [ ] Diesel schema declarations + model structs
- [ ] ApiKeyDAL: create, validate_by_prefix, list_by_tenant, revoke, load_with_patterns
- [ ] TenantDAL: create, list, get, deactivate
- [ ] PAK key generation: format, random bytes, argon2 hashing
- [ ] AuthCache: in-memory cache with TTL, prefix-based lookup, negative caching, invalidation
- [ ] AuthContext struct: key_id, tenant_id, permissions, workflow_patterns
- [ ] Tower AuthExtract middleware: Bearer extraction → cache lookup → hash verify → inject AuthContext
- [ ] Tower TenantScope middleware: tenant isolation enforcement
- [ ] PermissionGuard: axum layer checking permission bits per route group
- [ ] WorkflowPatternCheck: glob matching utility for handler-level ABAC
- [ ] Wire middleware into axum Router in serve.rs
- [ ] CLI: `cloacinactl key create-api-key --tenant --permissions`
- [ ] Unit tests: cache TTL, negative cache, permission checks, pattern matching
- [ ] Integration tests: full auth flow (create key → authenticate → access resource → revoke → 401)

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

{Delete if not a requirements-focused initiative}

### User Requirements
- **User Characteristics**: {Technical background, experience level, etc.}
- **System Functionality**: {What users expect the system to do}
- **User Interfaces**: {How users will interact with the system}

### System Requirements
- **Functional Requirements**: {What the system should do - use unique identifiers}
  - REQ-001: {Functional requirement 1}
  - REQ-002: {Functional requirement 2}
- **Non-Functional Requirements**: {How the system should behave}
  - NFR-001: {Performance requirement}
  - NFR-002: {Security requirement}

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

{Delete if not user-facing}

### Use Case 1: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

### Use Case 2: {Use Case Name}
- **Actor**: {Who performs this action}
- **Scenario**: {Step-by-step interaction}
- **Expected Outcome**: {What should happen}

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

{Delete if not technically complex}

### Overview
{High-level architectural approach}

### Component Diagrams
{Describe or link to component diagrams}

### Class Diagrams
{Describe or link to class diagrams - for OOP systems}

### Sequence Diagrams
{Describe or link to sequence diagrams - for interaction flows}

### Deployment Diagrams
{Describe or link to deployment diagrams - for infrastructure}

## Detailed Design **[REQUIRED]**

{Technical approach and implementation details}

## UI/UX Design **[CONDITIONAL: Frontend Initiative]**

{Delete if no UI components}

### User Interface Mockups
{Describe or link to UI mockups}

### User Flows
{Describe key user interaction flows}

### Design System Integration
{How this fits with existing design patterns}

## Testing Strategy **[CONDITIONAL: Separate Testing Initiative]**

{Delete if covered by separate testing initiative}

### Unit Testing
- **Strategy**: {Approach to unit testing}
- **Coverage Target**: {Expected coverage percentage}
- **Tools**: {Testing frameworks and tools}

### Integration Testing
- **Strategy**: {Approach to integration testing}
- **Test Environment**: {Where integration tests run}
- **Data Management**: {Test data strategy}

### System Testing
- **Strategy**: {End-to-end testing approach}
- **User Acceptance**: {How UAT will be conducted}
- **Performance Testing**: {Load and stress testing}

### Test Selection
{Criteria for determining what to test}

### Bug Tracking
{How defects will be managed and prioritized}

## Alternatives Considered **[REQUIRED]**

{Alternative approaches and why they were rejected}

## Implementation Plan **[REQUIRED]**

{Phases and timeline for execution}
