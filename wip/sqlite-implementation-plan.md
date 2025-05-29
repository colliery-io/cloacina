# Implementation Plan for SQLite Support in Cloacina


## Overview
This document outlines the concrete implementation plan for adding dual PostgreSQL/SQLite backend support to the Cloacina project, based on the lessons learned from the dual-backend-demo blog post.

## Current State Analysis
- **Database**: PostgreSQL-only implementation
- **UUID Handling**: Direct use of `uuid::Uuid` type (re-exported as `crate::uuid::UUID`)
- **Schema**: PostgreSQL-specific types throughout
- **DAL**: Single implementation targeting PostgreSQL
- **Conditional Compilation**: None for database backends

## Implementation Phases

### Phase 1: Set up Cargo Features for Dual Backend Support
- Add mutually exclusive features for `postgres` and `sqlite`
- Default to `postgres` for backward compatibility
- Update Diesel dependencies with conditional features
- Structure:
  ```toml
  [features]
  default = ["postgres"]
  postgres = ["diesel/postgres", "diesel/uuid", "diesel/chrono"]
  sqlite = ["diesel/sqlite", "diesel/returning_clauses_for_sqlite_3_35"]
  ```

### Phase 2: Create Universal Wrapper Types
- **UniversalUuid**: Wrapper that implements `ToSql`/`FromSql` for both backends
  - SQLite: Store as 16-byte BLOB (55% storage savings vs string)
  - PostgreSQL: Use native UUID type
  - Keep existing `crate::uuid::UUID` re-export for API compatibility
- **UniversalTimestamp**: Handle timestamp differences
  - SQLite: RFC3339 string in TEXT
  - PostgreSQL: Native timestamp type

### Phase 3: Create SQLite Schema with Appropriate Type Mappings
- Create `src/database/schema_sqlite.rs` with type mappings:
  - `Uuid` → `Binary` (BLOB)
  - `Timestamp` → `Text` (RFC3339 format)
  - `Varchar/Text` → `Text`
  - `Bool` → `Integer` (0/1)
- Use conditional compilation to select appropriate schema

### Phase 4: Update All Models to Use Universal Wrapper Types
- Replace `uuid::Uuid` with `UniversalUuid` in all models
- Update timestamp fields to use wrapper types
- Ensure foreign key relationships use universal types
- No conditional compilation needed in models!

### Phase 5: Create SQLite-specific DAL Implementations
- Keep existing PostgreSQL DAL unchanged
- Create parallel SQLite DAL modules
- Handle key differences:
  - Manual UUID generation for inserts (SQLite lacks `uuid_generate_v4()`)
  - No `RETURNING` clause in older SQLite versions
  - Different transaction semantics
  - Manual timestamp generation

### Phase 6: Update Connection Handling for Backend Selection
- Update `Database` struct to support both backends
- Use compile-time feature flags for connection type selection
- Handle different connection string formats:
  - PostgreSQL: `postgres://user:pass@host/db`
  - SQLite: File path or `:memory:`

### Phase 7: Create SQLite-specific Migrations
- Convert PostgreSQL migrations to SQLite syntax
- Key changes:
  - Replace `UUID PRIMARY KEY DEFAULT uuid_generate_v4()` with `BLOB PRIMARY KEY`
  - Update CHECK constraints for SQLite compatibility
  - Convert BOOLEAN columns to INTEGER with CHECK constraints
  - Handle timestamp defaults appropriately

### Phase 8: Update Tests for Dual Backend Support
- Create backend-agnostic tests that run on both databases
- Add backend-specific tests where needed
- Test matrix:
  ```bash
  cargo test --features sqlite
  cargo test --features postgres
  ```

### Phase 9: Update Examples and Documentation
- Update example projects to show both configurations
- Document feature flag usage
- Create migration guide for users

## Key Design Decisions

1. **Compile-time Backend Selection**
   - Simpler than runtime selection
   - Zero runtime overhead
   - Clear separation of concerns

2. **Separate DAL Implementations**
   - Avoid "type checker whack-a-mole"
   - Each backend can be optimized independently
   - Clearer code organization

3. **Universal Wrapper Types**
   - Models remain clean without conditional compilation
   - Type safety maintained across backends
   - Consistent API regardless of backend

4. **Storage Optimizations**
   - 16-byte BLOB for UUIDs (vs 36-byte strings)
   - RFC3339 for timestamps (human-readable, sortable)

## Implementation Order
1. Start with Cargo features (Phase 1) - establishes foundation
2. Create wrapper types (Phase 2) - needed before model updates
3. Update schemas and models together (Phases 3-4)
4. Implement DAL and connection handling (Phases 5-6)
5. Migrations, tests, and docs can proceed in parallel (Phases 7-9)

## Success Criteria
- [ ] Both backends pass all tests
- [ ] No performance regression for PostgreSQL
- [ ] SQLite performance acceptable for single-node deployments
- [ ] Migration path documented for existing users
- [ ] Examples work with both backends

## Notes
- This plan is based on the proven approach from https://github.com/colliery-io/dual-backend-demo
- The current `wip/sqlite-backend-plan-revised.md` aligns with this approach
- JSONB has already been removed, simplifying the implementation
