---
id: remove-duplicate-dal-modules-and
level: task
title: "Remove duplicate DAL modules and update exports"
short_code: "CLOACI-T-0009"
created_at: 2025-11-30T02:05:40.575567+00:00
updated_at: 2025-11-30T02:05:40.575567+00:00
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

# Remove duplicate DAL modules and update exports

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0001]]

## Objective

Remove the deprecated duplicate `postgres_dal/` and `sqlite_dal/` directories and update all module exports to use the unified DAL implementation.

## Acceptance Criteria

- [ ] `cloacina/src/dal/postgres_dal/` directory deleted
- [ ] `cloacina/src/dal/sqlite_dal/` directory deleted
- [ ] `cloacina/src/dal/mod.rs` exports only unified DAL
- [ ] All references to legacy DAL modules removed
- [ ] No dead code warnings related to DAL
- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo test` passes all tests
- [ ] `cargo doc` generates correct documentation

## Implementation Notes

### Technical Approach

1. **Verify unified DAL is complete**

   Before deletion, ensure all functionality from legacy DALs is present in unified:

   ```bash
   # Compare exported items
   diff <(grep "pub fn\|pub async fn\|pub struct" postgres_dal/*.rs | sort) \
        <(grep "pub fn\|pub async fn\|pub struct" unified/*.rs | sort)
   ```

2. **Update `dal/mod.rs`**

   Before:
   ```rust
   #[cfg(feature = "postgres")]
   mod postgres_dal;

   #[cfg(feature = "sqlite")]
   mod sqlite_dal;

   mod unified;  // Added during migration
   mod filesystem_dal;

   #[cfg(feature = "postgres")]
   pub use postgres_dal::*;

   #[cfg(feature = "sqlite")]
   pub use sqlite_dal::*;
   ```

   After:
   ```rust
   mod unified;
   mod filesystem_dal;

   pub use unified::*;
   pub use filesystem_dal::FilesystemRegistryStorage;
   ```

3. **Delete legacy directories**

   ```bash
   rm -rf cloacina/src/dal/postgres_dal/
   rm -rf cloacina/src/dal/sqlite_dal/
   ```

4. **Clean up any remaining references**

   Search for any remaining references to legacy modules:
   ```bash
   grep -r "postgres_dal\|sqlite_dal" cloacina/src/
   ```

### Files to Delete

- `cloacina/src/dal/postgres_dal/` (entire directory)
- `cloacina/src/dal/sqlite_dal/` (entire directory)

### Files to Modify

- `cloacina/src/dal/mod.rs`

### Dependencies

- Requires CLOACI-T-0006 (All DALs migrated)
- Requires CLOACI-T-0008 (Feature flags refactored)
- Must be confident all tests pass before deletion

### Risk Considerations

- Irreversible deletion (git history preserves, but workflow disruption if issues found)
- Run full test suite before and after deletion
- Consider keeping a backup branch until release is validated

## Status Updates

*To be added during implementation*
