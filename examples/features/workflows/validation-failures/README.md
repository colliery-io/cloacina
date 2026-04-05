# Compile-Time Validation Failure Examples

This directory contains examples that are **designed to fail compilation** to demonstrate the compile-time validation system working correctly.

## Examples

### 1. Missing Dependency (`missing_dependency.rs`)
Demonstrates detection of tasks that depend on non-existent tasks.

**Expected Error:**
```
error: Task 'invalid_task' depends on undefined task 'nonexistent_task'
```

**Test Command:**
```bash
cargo check --bin missing_dependency
```

### 2. Circular Dependency (`circular_dependency.rs`)
Demonstrates detection of circular dependencies between tasks.

**Expected Error:**
```
error: Circular dependency detected: task_a -> task_b -> task_a
```

**Test Command:**
```bash
cargo check --bin circular_dependency
```

### 3. Duplicate Task IDs (`duplicate_task_ids.rs`)
Demonstrates detection of multiple tasks with the same ID.

**Expected Error:**
```
error: Duplicate task ID 'duplicate_id'. Already defined at '...', redefined at '...'
```

**Test Command:**
```bash
cargo check --bin duplicate_task_ids
```

## Testing

These examples are tested via the Angreal task system. Run:

```bash
# Test all validation failures
angreal demos validation-failures

# Test specific failure case
angreal demos validation-failure-test missing_dependency
```

## Expected Behavior

All of these examples should **fail to compile** with helpful error messages. If any of them compile successfully, it indicates a bug in the compile-time validation system.
