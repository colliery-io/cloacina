# SIGSEGV Troubleshooting for PostgreSQL Integration Tests

## Current Fix (as of 2024-12-02)

### Root Cause
Tests that call `package_workflow()` spawn cargo subprocesses via `fork()`. When this happens after the database connection pool has initialized OpenSSL/libpq, the fork can cause SIGSEGV on Linux due to OpenSSL's unsafe atexit handler.

See: https://github.com/diesel-rs/diesel/issues/3441

### Applied Fixes

1. **OpenSSL early initialization via #[ctor]** (`cloacina/src/database/connection.rs`)
   - Uses `#[ctor]` to call `openssl::init()` at program startup, before main()
   - This runs before ANY other code including test setup and async runtime init
   - Uses system OpenSSL (NOT vendored) to match libpq

2. **Test package caching** (`cloacina/tests/integration/dal/workflow_registry*.rs`)
   - Cache test packages with `OnceLock`
   - Ensures package building (which forks) happens before any DB init
   - Package is built once and reused across all tests

## Alternative Approaches to Try if Fix Doesn't Work

1. **Pre-build test packages before test run**
   - Use a build.rs or pre-commit hook to build packages before tests start
   - Store as static test fixtures

2. **Disable ASLR in CI for debugging**
   - Run with `setarch $(uname -m) -R cargo test ...`
   - Makes crash reproducible if ASLR-dependent

3. **Use AddressSanitizer or ThreadSanitizer**
   - Build with `RUSTFLAGS="-Z sanitizer=address"` or `sanitizer=thread`
   - May reveal the actual memory issue

4. **Use diesel_async**
   - Switch from sync diesel with deadpool to diesel_async
   - Different connection handling might avoid the issue

5. **Investigate bundled pq-sys behavior**
   - Check if `pq-sys/bundled` has different OpenSSL linking behavior
   - May need to match OpenSSL versions more carefully

6. **Isolated subprocess spawning**
   - Spawn package builds in completely isolated processes (not fork)
   - Use `std::process::Command` with explicit environment clearing

7. **Lazy database initialization**
   - Delay database pool creation until after all subprocess work is done
   - Restructure tests to do all forking first

## Debugging Tips

- GDB slows execution enough to mask race conditions - if tests pass with GDB, it's likely a timing issue
- The SIGSEGV typically occurs during program exit when OpenSSL cleanup races with connection pool threads
- Check `ldd` output to verify which OpenSSL version is linked
