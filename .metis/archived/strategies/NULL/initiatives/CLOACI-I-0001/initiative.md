---
id: migrate-to-runtime-database
level: initiative
title: "Migrate to Runtime Database Backend Selection"
short_code: "CLOACI-I-0001"
created_at: 2025-11-28T15:21:48.824029+00:00
updated_at: 2025-12-03T23:36:32.407707+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: true

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
strategy_id: NULL
initiative_id: migrate-to-runtime-database
---

# Migrate to Runtime Database Backend Selection Initiative

## Context

The current Cloacina database layer uses **compile-time feature flags** to select between PostgreSQL and SQLite backends. This architecture enforces mutual exclusivity via `compile_error!` macros, requiring separate binary compilations for each backend.

**Current Implementation:**
- Feature flags: `postgres` and `sqlite` in `cloacina/Cargo.toml`
- Enforcement: `cloacina/src/lib.rs:430-435`
- Duplicate DAL modules: `cloacina/src/dal/postgres_dal/` and `cloacina/src/dal/sqlite_dal/`
- Separate schemas: `cloacina/src/database/schema.rs` (two complete definitions)
- Separate migrations: `cloacina/src/database/migrations/postgres/` and `sqlite/`

**Problems:**
1. Cannot distribute a single binary supporting both backends
2. Poor developer experience - requires recompilation to test different backends
3. Binding consumers must choose backend at compile time
4. Significant code duplication across DAL implementations

## Goals & Non-Goals

**Goals:**
- Enable runtime database backend selection via connection string
- Produce a single binary that supports both PostgreSQL and SQLite
- Reduce code duplication in DAL implementations
- Maintain existing API compatibility for downstream consumers
- Preserve option to compile single-backend for smaller deployments

**Non-Goals:**
- Adding support for additional databases (MySQL, etc.)
- Changing the migration system (Diesel embedded migrations)
- Modifying the existing schema structure
- Performance optimization beyond current levels

## Architecture

### Overview

Migrate from compile-time feature flags to Diesel 2.0+ `MultiConnection` pattern, enabling runtime backend selection while maintaining type safety.

### Target Architecture

```
Connection String (runtime)
         |
         v
+-------------------+
| AnyConnection     |  <-- Diesel MultiConnection enum
| - Postgres(PgConn)|
| - Sqlite(SqlConn) |
+-------------------+
         |
         v
+-------------------+
| Unified DAL       |  <-- Single implementation using MultiConnection
| - context.rs      |
| - pipeline.rs     |
| - task.rs         |
| - etc.            |
+-------------------+
         |
         v
+-------------------+
| Backend-Specific  |  <-- Match statements for backend differences
| Query Handling    |
+-------------------+
```

### Key Components

1. **AnyConnection enum**: Diesel `#[derive(MultiConnection)]` wrapping both connection types
2. **Unified DAL**: Single set of DAL files using `AnyConnection`
3. **Backend matchers**: Isolated `match` blocks for backend-specific queries (e.g., UUID handling)
4. **Optional feature flags**: Compile-time flags to exclude backends for smaller binaries

## Detailed Design

### Phase 1: Enable MultiConnection

1. Update Diesel dependency to 2.0+ with multi-connection support
2. Create `AnyConnection` enum in `database/connection.rs`:
   ```rust
   #[derive(diesel::MultiConnection)]
   pub enum AnyConnection {
       Postgres(PgConnection),
       Sqlite(SqliteConnection),
   }
   ```
3. Update `Database` struct to use `AnyConnection`
4. Modify connection establishment to parse URL scheme for backend detection

### Phase 2: Unify DAL Implementations

1. Create unified DAL module replacing `postgres_dal/` and `sqlite_dal/`
2. Update queries to use `AnyConnection` type
3. Use `match` statements for backend-specific behavior:
   ```rust
   match &connection {
       AnyConnection::Postgres(conn) => { /* native UUID */ }
       AnyConnection::Sqlite(conn) => { /* UUID as blob */ }
   }
   ```
4. Retain `UniversalUuid`, `UniversalTimestamp`, `UniversalBool` wrappers for type compatibility

### Phase 3: Migration System Update

1. Keep separate migration directories (required by Diesel)
2. Update migration runner to detect backend and run appropriate migrations
3. Add runtime migration selection logic

### Phase 4: Feature Flag Refactoring

1. Change feature flags from "select one" to "include backend":
   ```toml
   [features]
   default = ["postgres", "sqlite"]
   postgres = ["diesel/postgres", ...]
   sqlite = ["diesel/sqlite", ...]
   ```
2. Remove `compile_error!` mutual exclusivity checks
3. Add compile-time guards for single-backend optimization builds

### Phase 5: Cleanup

1. Remove duplicate DAL modules
2. Update public API exports
3. Update documentation and examples
4. Update CI to test both backends in single build

## Alternatives Considered

### SeaORM Migration
- **Pros**: First-class runtime support, async-native
- **Cons**: Complete rewrite required, different query syntax, loss of existing Diesel expertise
- **Decision**: Rejected due to migration cost

### Trait Objects (Box<dyn Backend>)
- **Pros**: Maximum flexibility, plugin architecture possible
- **Cons**: Lifetime issues with Diesel connections, runtime overhead, complex implementation
- **Decision**: Rejected due to Diesel compatibility issues

### Enum Dispatch Crate
- **Pros**: Near-static performance, clean trait-based API
- **Cons**: Requires significant DAL restructuring, additional dependency
- **Decision**: Considered as future optimization if MultiConnection proves insufficient

### Status Quo (Compile-Time)
- **Pros**: Zero runtime overhead, smaller binaries
- **Cons**: Poor DX, distribution challenges, code duplication
- **Decision**: Rejected - the problems outweigh benefits

## Implementation Plan

### Phase 1: MultiConnection Foundation
- Update Diesel dependency
- Create AnyConnection enum
- Update Database struct and connection logic
- Verify both backends work with new connection type

### Phase 2: DAL Unification
- Create unified DAL module structure
- Migrate context operations first (simplest)
- Migrate remaining DAL operations
- Add backend-specific match handling where needed

### Phase 3: Migration System
- Update migration runner for runtime backend detection
- Test migrations run correctly for both backends
- Verify schema compatibility

### Phase 4: Feature Flag Update
- Refactor Cargo.toml features
- Add optional single-backend compilation
- Update downstream crate configurations

### Phase 5: Cleanup & Documentation
- Remove deprecated duplicate code
- Update all examples
- Update CI workflows
- Document new runtime selection behavior
