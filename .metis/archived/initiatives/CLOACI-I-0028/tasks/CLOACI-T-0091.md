---
id: c4-level-3-component-diagrams-data
level: task
title: "C4 Level 3 — Component Diagrams: Data Access Layer"
short_code: "CLOACI-T-0091"
created_at: 2026-03-13T14:29:53.811683+00:00
updated_at: 2026-03-13T15:40:12.504921+00:00
parent: CLOACI-I-0028
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0028
---

# C4 Level 3 — Component Diagrams: Data Access Layer

**Phase:** 2 — C4 Architecture Documentation
**Parent Initiative:** [[CLOACI-I-0028]]

## Objective

Create the C4 Level 3 (Component) diagram and documentation for the Data Access Layer — the repository pattern, DAL facade, backend implementations (PostgreSQL/SQLite), and schema-based multi-tenancy.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Mermaid C4 Component diagram for the Data Access Layer within the `cloacina` container
- [ ] DAL facade documented as the central interface composing all repositories
- [ ] All repository types listed and described with their responsibilities
- [ ] PostgreSQL and SQLite backend implementations shown as interchangeable
- [ ] Schema-based multi-tenancy mechanism documented (PostgreSQL `SET search_path`)
- [ ] Outbox pattern components shown: ExecutionEventRepository, TaskOutboxRepository
- [ ] All component descriptions verified against actual source in `crates/cloacina/src/dal/`

## Implementation Notes

### Components to Document
- **DAL** (`crates/cloacina/src/dal/mod.rs`) — facade composing all repositories
- **ContextRepository** — context persistence/retrieval
- **TaskExecutionRepository** — task state, claiming, sub-status, recovery
- **PipelineExecutionRepository** — pipeline state management
- **CronExecutionRepository / CronScheduleRepository** — cron scheduling state
- **TriggerExecutionRepository / TriggerScheduleRepository** — trigger state
- **ExecutionEventRepository** — outbox-based event logging
- **TaskOutboxRepository** — guaranteed delivery queue
- **WorkflowRegistryRepository / WorkflowPackagesRepository** — package management
- **SigningKeyRepository / PackageSignatureRepository** — security state
- **PostgreSQL backend** (`crates/cloacina/src/dal/postgres/`) — schema-based multi-tenancy
- **SQLite backend** (`crates/cloacina/src/dal/sqlite/`)

## Status Updates

### Completed 2026-03-13

**Created:** `docs/content/explanation/architecture/c4-data-access-layer.md`

**Components documented:** DAL facade, Database (connection mgmt), 10 domain repositories, universal types
- Facade pattern with runtime backend dispatch via macros
- All repositories verified against `crates/cloacina/src/dal/unified/`
- Multi-tenancy: PostgreSQL schema isolation + SQLite file isolation
- Transactional outbox pattern documented
- ER diagram for core tables
- Universal types cross-backend mapping table

**Build:** 96 pages, clean
