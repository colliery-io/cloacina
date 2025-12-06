---
id: update-migration-runner-for
level: task
title: "Update migration runner for runtime backend detection"
short_code: "CLOACI-T-0007"
created_at: 2025-11-30T02:05:40.097654+00:00
updated_at: 2025-12-03T23:36:07.888755+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Update migration runner for runtime backend detection

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Update the migration runner to detect the database backend at runtime and execute the appropriate migration set (PostgreSQL or SQLite).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Migration runner detects backend from connection URL
- [ ] PostgreSQL migrations run for `postgres://` URLs
- [ ] SQLite migrations run for `sqlite://` or file path URLs
- [ ] `run_migrations()` function works with `AnyConnection`
- [ ] Schema setup (`setup_schema()`) works for PostgreSQL tenants
- [ ] Migrations can be run programmatically from application code
- [ ] Error messages clearly indicate which backend failed

## Implementation Notes

### Technical Approach

1. **Current State Analysis**

   Current migration runner location: `cloacina/src/database/mod.rs`

   Migrations are embedded via `diesel_migrations::embed_migrations!` with separate directories:
   - `cloacina/src/database/migrations/postgres/`
   - `cloacina/src/database/migrations/sqlite/`

2. **Runtime Selection Strategy**

   ```rust
   pub fn run_migrations(conn: &mut AnyConnection) -> Result<(), MigrationError> {
       match conn {
           AnyConnection::Postgres(pg_conn) => {
               run_postgres_migrations(pg_conn)
           }
           AnyConnection::Sqlite(sqlite_conn) => {
               run_sqlite_migrations(sqlite_conn)
           }
       }
   }
   ```

3. **Embedded Migrations**

   Both migration sets should be embedded when both features enabled:
   ```rust
   #[cfg(feature = "postgres")]
   mod postgres_migrations {
       diesel_migrations::embed_migrations!("src/database/migrations/postgres");
   }

   #[cfg(feature = "sqlite")]
   mod sqlite_migrations {
       diesel_migrations::embed_migrations!("src/database/migrations/sqlite");
   }
   ```

### Files to Modify

- `cloacina/src/database/mod.rs` - Migration runner updates
- `cloacina/src/database/connection.rs` - Integration with Database struct

### Dependencies

- Requires CLOACI-T-0001 (AnyConnection enum)
- Requires CLOACI-T-0002 (Database struct with backend detection)

### Risk Considerations

- Migration directories must remain separate (Diesel requirement)
- Must handle migration version tracking per backend
- Schema migrations for multi-tenancy need special handling

## Status Updates

*To be added during implementation*
