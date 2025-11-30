---
id: migrate-contextdal-to-unified
level: task
title: "Migrate ContextDAL to unified implementation"
short_code: "CLOACI-T-0005"
created_at: 2025-11-30T02:05:39.703422+00:00
updated_at: 2025-11-30T02:05:39.703422+00:00
parent: CLOACI-I-0001
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0001
---

# Migrate ContextDAL to unified implementation

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Migrate `ContextDAL` to the unified implementation as the first DAL module, establishing patterns for handling backend differences that will be used in subsequent DAL migrations.

## Acceptance Criteria

- [ ] `unified/context.rs` fully implemented
- [ ] All CRUD operations work: `create`, `read`, `update`, `delete`, `list`
- [ ] Backend-specific handling for:
  - [ ] UUID generation (database default vs client-side)
  - [ ] Timestamp generation (database default vs client-side)
  - [ ] Connection acquisition (with schema vs without)
  - [ ] Insert pattern (`get_result` vs `execute`)
- [ ] Unit tests pass for both backends
- [ ] Integration tests pass for both backends
- [ ] API compatibility maintained with existing `ContextDAL`

## Implementation Notes

### Technical Approach

Use the `ContextDAL` comparison from exploration as the guide. Key differences to handle:

**PostgreSQL:**
```rust
let conn = self.dal.database.get_connection_with_schema().await?;
let db_context: DbContext = conn.interact(move |conn| {
    diesel::insert_into(contexts::table)
        .values(&new_context)
        .get_result(conn)  // Returns inserted row
}).await??;
Ok(Some(db_context.id.into()))
```

**SQLite:**
```rust
let conn = self.dal.pool.get().await?;
let id = UniversalUuid::new_v4();  // Client-side generation
let now = current_timestamp();      // Client-side timestamp
conn.interact(move |conn| {
    diesel::insert_into(contexts::table)
        .values((
            contexts::id.eq(&id),
            &new_context,
            contexts::created_at.eq(&now),
            contexts::updated_at.eq(&now),
        ))
        .execute(conn)  // Just executes, doesn't return row
}).await??;
Ok(Some(id))
```

**Unified approach:**
```rust
match self.dal.backend_type {
    BackendType::Postgres => { /* PostgreSQL path */ }
    BackendType::Sqlite => { /* SQLite path */ }
}
```

### Files to Modify

- `cloacina/src/dal/unified/context.rs` - Full implementation
- `cloacina/src/dal/unified/mod.rs` - Export `ContextDAL`

### Dependencies

- Requires CLOACI-T-0001 (AnyConnection enum)
- Requires CLOACI-T-0002 (Database struct)
- Requires CLOACI-T-0003 (Unified schema)
- Requires CLOACI-T-0004 (DAL structure)

### Risk Considerations

- This is the pattern-setting task; patterns established here will be used in all other DAL modules
- Must ensure the approach scales to more complex DAL modules
- Error handling patterns must be consistent

## Status Updates

*To be added during implementation*