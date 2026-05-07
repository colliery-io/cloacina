# SIGSEGV Troubleshooting for PostgreSQL Integration Tests

> **Historical document.** The `#[ctor]`-based OpenSSL early-init
> workaround described below was removed during the I-0096 ctor →
> inventory flip (the `connection.rs` single-file referenced no longer
> exists; `crates/cloacina/src/database/connection/` is now a directory
> with no `#[ctor]` calls and no `ctor` dependency). The historical
> root-cause notes are preserved here in case the failure mode resurfaces
> in CI or with a different libpq/OpenSSL combination.

## Historical Root Cause

Tests that call `package_workflow()` spawn cargo subprocesses via `fork()`. When this happens after the database connection pool has initialized OpenSSL/libpq, the fork can cause SIGSEGV on Linux due to OpenSSL's unsafe atexit handler.

See: https://github.com/diesel-rs/diesel/issues/3441

## Historical Mitigations (no longer present in code)

The following were the mitigations applied at the time the bug was
diagnosed. They have since been removed; the bug has not recurred,
likely thanks to upstream diesel + libpq updates. Document them here
in case the symptom returns:

1. **OpenSSL early initialization via #[ctor]** — Pre-I-0096 the
   `cloacina/src/database/connection.rs` module used `#[ctor]` to call
   `openssl::init()` before `main()`, ahead of any async runtime or
   test setup. Removed when `connection.rs` was reorganized into the
   `connection/` directory and the `ctor` dependency was dropped.

2. **Test package caching** with `OnceLock` to ensure the forking
   `package_workflow()` call ran once before any DB init. Currently
   integration tests rely on a different caching path; if the SIGSEGV
   resurfaces, restoring the `OnceLock` pattern is a known-good
   mitigation.

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
