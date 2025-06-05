# Async DAL Migration Plan

## What We're Doing

Converting Cloacina's Data Access Layer (DAL) from synchronous database operations to fully async operations to eliminate deadlocks in Python bindings and improve overall architecture.

## Why We're Doing It

### Root Cause Problem
The current DAL uses **synchronous blocking database calls** (`pool.get()`) within async contexts, causing deadlocks when:
- Background scheduler polls for active pipelines (sync DB call)
- Background executor claims ready tasks (sync DB call)
- Main execution thread polls for completion (sync DB call)
- All compete for limited SQLite connections (default: 1 connection)

### Current Anti-Pattern
```rust
// BAD: Sync blocking in async context
async fn some_scheduler_method() {
    let conn = self.dal.pool.get()?;  // BLOCKS entire async runtime!
    // ... database work
}
```

### Target Pattern
```rust
// GOOD: Fully async
async fn some_scheduler_method() {
    let conn = self.dal.pool.get().await?;  // Truly async, yields control
    // ... database work
}
```

### Benefits Beyond Python Bindings
1. **Proper async architecture** - no mixing sync/async patterns
2. **Better resource utilization** - no threads blocked on DB connections
3. **Improved backpressure handling** - async pools handle load gracefully
4. **Future-proof design** - enables other async integrations
5. **Better observability** - async pools provide superior metrics

## Implementation Plan

### Phase 1: DAL Layer Migration ✅ Target
**Goal**: Make all database operations truly async

#### 1.1 Connection Pool Migration
- [ ] **Current**: `r2d2::Pool<diesel::SqliteConnection>` (sync)
- [ ] **Target**: `deadpool_diesel::sqlite::Pool` (async)
- [ ] **Files**: `cloacina/src/database/connection.rs`

#### 1.2 DAL Method Signatures
- [ ] **Current**: `fn method(&self) -> Result<T, Error>`
- [ ] **Target**: `async fn method(&self) -> Result<T, Error>`
- [ ] **Files**: All files in `cloacina/src/dal/`

#### 1.3 Connection Acquisition
- [ ] **Current**: `let conn = self.dal.pool.get()?;`
- [ ] **Target**: `let conn = self.dal.pool.get().await?;`
- [ ] **Impact**: All DAL method implementations

### Phase 2: Caller Updates ✅ Target
**Goal**: Update all DAL usage to async/await pattern

#### 2.1 Core Services
- [ ] **TaskScheduler**: `cloacina/src/task_scheduler.rs`
- [ ] **TaskExecutor**: `cloacina/src/executor/task_executor.rs`
- [ ] **PipelineEngine**: `cloacina/src/executor/pipeline_engine.rs`
- [ ] **DefaultRunner**: `cloacina/src/runner/default_runner.rs`

#### 2.2 Update Pattern
- [ ] **Current**: `let result = dal.method()?;`
- [ ] **Target**: `let result = dal.method().await?;`

### Phase 3: Dependency Updates ✅ Target
**Goal**: Update Cargo.toml dependencies

#### 3.1 Remove Sync Dependencies
- [ ] Remove: `r2d2 = "0.8"`
- [ ] Remove: `r2d2-diesel = "1.0"`

#### 3.2 Add Async Dependencies  
- [ ] Add: `deadpool-diesel = { version = "0.6", features = ["sqlite", "postgres"] }`
- [ ] Keep: `diesel = { version = "2.2", features = ["sqlite", "postgres", "chrono", "uuid"] }`

### Phase 4: Testing & Validation ✅ Target
**Goal**: Ensure migration doesn't break existing functionality

#### 4.1 Rust Examples Validation
- [ ] Run `examples/tutorial-01` - basic execution
- [ ] Run `examples/tutorial-02` - multi-task workflows  
- [ ] Run `examples/tutorial-03` - parallel processing
- [ ] Run `examples/tutorial-04` - error handling
- [ ] Run `examples/cron-scheduling` - cron functionality

#### 4.2 Integration Tests
- [ ] Run `cargo test` for core Cloacina tests
- [ ] Verify all scheduler tests pass
- [ ] Verify all executor tests pass
- [ ] Verify multi-tenant tests pass

## Scope Limitation

**This branch focuses ONLY on Cloacina core DAL migration.**

No Python bindings or boundary layer work will be done in this branch. The goal is purely to improve Cloacina's async architecture by eliminating sync database calls in async contexts.

## Progress Tracking

### Completed ✅
- [x] Created feature branch `feature/async-dal`
- [x] Created migration plan document
- [x] Analyzed current DAL architecture

**Analysis Results:**
- **154 total method implementations** across 6 DAL traits × 2 backends
- **Current pattern**: `let mut conn = self.dal.pool.get()?;` (sync blocking)
- **Target pattern**: `let mut conn = self.dal.pool.get().await?;` (async)
- **Connection pool**: `diesel::r2d2::Pool` → `deadpool_diesel::Pool`

### In Progress 🔄  
- [ ] Fix remaining deadpool-diesel compatibility issues
- [ ] Fix remaining async conversion errors in admin.rs, types.rs

### Completed ✅
- [x] Replace r2d2 with deadpool for async connection pooling
- [x] Connection pool migration (Phase 1.1)
- [x] DAL method signatures update (Phase 1.2)  
- [x] DAL implementations converted to async (Phase 1.3)
- [x] Service layer async conversion (Phase 2)
- [x] Core deadpool connection dereferencing pattern established

### Pending ⏳
- [ ] Complete remaining file async conversions
- [ ] Testing and validation
- [ ] Python bindings simplification

## Risk Mitigation

### Breaking Changes
- **Impact**: This is a breaking change to Cloacina's internal API
- **Mitigation**: Since Cloacina is pre-GA, acceptable to make architectural improvements
- **Validation**: Extensive testing with existing examples

### Performance Impact
- **Concern**: Async overhead vs sync operations
- **Mitigation**: Modern async runtimes have minimal overhead
- **Benefit**: Better overall performance under load due to proper async behavior

### Complexity
- **Concern**: More complex async/await throughout codebase
- **Mitigation**: Cleaner, more consistent async architecture
- **Benefit**: Eliminates sync/async mixing anti-patterns

## Success Criteria

1. ✅ All Rust examples continue to work
2. ✅ All integration tests pass
3. ✅ Python bindings no longer deadlock
4. ✅ Simplified Python boundary layer
5. ✅ Better async architecture overall

## Files Requiring Changes

### Core DAL Files
- `cloacina/src/database/connection.rs` - Connection pool
- `cloacina/src/dal/mod.rs` - DAL trait definitions
- `cloacina/src/dal/sqlite_dal/` - All SQLite DAL implementations
- `cloacina/src/dal/postgres_dal/` - All PostgreSQL DAL implementations

### Service Files
- `cloacina/src/task_scheduler.rs`
- `cloacina/src/executor/task_executor.rs`
- `cloacina/src/executor/pipeline_engine.rs`
- `cloacina/src/runner/default_runner.rs`

### Configuration Files
- `cloacina/Cargo.toml` - Dependencies
- `cloaca-backend/Cargo.toml` - Python binding dependencies

---

**Last Updated**: 2025-06-05  
**Branch**: `feature/async-dal`  
**Status**: Planning Phase Complete - Ready for Implementation