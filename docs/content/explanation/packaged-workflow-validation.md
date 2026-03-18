---
title: "Packaged Workflow Validation"
description: "What Cloacina checks when you build or upload a workflow package, and how to fix common issues"
weight: 15
draft: true
---

## Overview

When you build a `.cloacina` package or upload one to the server, Cloacina runs a series of validation checks before accepting it. These checks catch problems that would otherwise surface as cryptic runtime failures — panics, missing symbols, architecture mismatches — when the scheduler tries to execute your workflow.

This page explains what each check does, what errors look like, and how to fix them.

## Validation Pipeline

Every `.cloacina` package passes through these checks in order:

```
1. Size validation         — Is the package a reasonable size?
2. Format validation       — Is this a valid shared library?
3. Symbol validation       — Does it export the required FFI functions?
4. FFI smoke test          — Do the FFI functions actually work without crashing?
5. Metadata validation     — Are package name, version, and task IDs well-formed?
6. Security assessment     — Does the binary contain suspicious patterns?
```

If any check produces an error, validation fails and the package is rejected. Warnings are logged but don't block registration (unless strict mode is enabled).

## Check 1: Size Validation

**What it checks:** Package size is non-zero and under the limit (default: 100MB).

**Common errors:**

| Error | Cause | Fix |
|-------|-------|-----|
| `Package is empty` | Zero-byte file uploaded | Verify your build produced a `.so`/`.dylib` and the tar.gz contains it |
| `Package size exceeds maximum` | Binary is over 100MB | Check for debug symbols — build with `--release` and consider `strip` |

**Warnings:**
- Package under 1KB — might be a stub or corrupted build
- Package over 50MB — unusually large, may include unnecessary dependencies

## Check 2: Format Validation

**What it checks:** The binary is a valid shared library for the current platform.

**Common errors:**

| Error | Cause | Fix |
|-------|-------|-----|
| `Package cannot be loaded as dynamic library` | Not a valid ELF/Mach-O/PE file | Ensure `crate-type = ["cdylib"]` in your Cargo.toml |
| `invalid ELF header` | Wrong architecture (e.g. ARM binary on x86 server) | Build for the target platform, or cross-compile |

### Architecture Mismatches

This is one of the most common issues. If you build on macOS (ARM) and upload to a Linux server (x86_64), the binary format is wrong. The server logs will show:

```
Package cannot be loaded as dynamic library: invalid ELF header
```

**Fix:** Build inside a container that matches your server's architecture, or use the containerized build in `deploy/Dockerfile.soak` as a reference.

## Check 3: Symbol Validation

**What it checks:** The shared library exports the two required FFI symbols:
- `cloacina_execute_task` — the entry point for running tasks
- `cloacina_get_task_metadata` — the entry point for extracting package metadata

These symbols are generated automatically by the `#[packaged_workflow]` macro.

**Common errors:**

| Error | Cause | Fix |
|-------|-------|-----|
| `Required symbol 'cloacina_execute_task' not found` | Missing `#[packaged_workflow]` macro | Ensure your module uses `#[packaged_workflow(...)]` |
| `Required symbol 'cloacina_get_task_metadata' not found` | Macro not applied or `cdylib` not set | Check `crate-type = ["cdylib"]` and the macro is on the right module |
| `Cannot load library for symbol validation` | Dependencies not available at runtime | Ensure all shared library dependencies are available (check with `ldd` on Linux or `otool -L` on macOS) |

### Missing Symbol Debugging

If symbols are missing, check that your Cargo.toml has:

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
cloacina-workflow = "0.3"  # Includes macros by default
```

And your code uses the macro:

```rust
#[packaged_workflow(
    name = "my_workflow",
    package = "my_package",
)]
pub mod my_workflow {
    // ... tasks ...
}
```

You can verify symbols manually:

```bash
# Linux
nm -D target/release/libmy_package.so | grep cloacina

# macOS
nm target/release/libmy_package.dylib | grep cloacina
```

## Check 4: FFI Smoke Test

**What it checks:** Each task is actually called through the `cloacina_execute_task` FFI boundary with an empty context. This verifies the FFI path works end-to-end — the same path the server and daemon use at runtime.

This check catches issues that are **invisible to `cargo test`** because tests call task functions directly (bypassing the FFI).

**Common errors:**

| Error | Cause | Fix |
|-------|-------|-----|
| `Task 'X' panicked during smoke test: there is no reactor running` | Task uses `tokio::time::sleep` or other tokio APIs, but the FFI entry point doesn't provide a tokio runtime | Rebuild with the latest `cloacina-workflow` (v0.3.2+) which provides a tokio runtime automatically |
| `Task 'X' panicked during smoke test: ...` | Any panic in the task's FFI code path | Check the panic message — it describes what went wrong in the FFI execution |

### Why This Check Exists

Before this check was added, a workflow package could pass all its tests, get uploaded to the server, and then crash at runtime:

```
thread panicked at src/lib.rs:67:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

The root cause was that `cargo test` runs task functions inside a `#[tokio::test]` runtime, but the server loads the package via `dlopen` and calls tasks through the C FFI — a completely different execution path. The FFI smoke test reproduces this exact path during validation.

### What "Empty Context" Means

The smoke test calls each task with `{}` (empty JSON context). Most tasks will return an error because they expect input data — **that's fine**. An error return (`rc = -1`) means the FFI boundary worked correctly. What the test catches is **panics** — the task crashing in a way that would take down the server process.

### Task Errors vs Panics

| Outcome | What It Means | Validation Result |
|---------|---------------|-------------------|
| `rc = 0` (success) | Task ran successfully with empty context | Pass |
| `rc = -1` (error) | Task returned an error (expected — needs real context) | Pass |
| Panic caught | Task crashed at the FFI boundary | **Fail** — package rejected |
| Process abort | Severe crash (e.g. double panic, stack overflow) | **Fail** — not always catchable |

## Check 5: Metadata Validation

**What it checks:** Package name, version, and task definitions are well-formed.

**Common errors:**

| Error | Cause | Fix |
|-------|-------|-----|
| `Package name cannot be empty` | Missing `package` attribute in macro | Add `package = "my_package"` to `#[packaged_workflow(...)]` |
| `Duplicate task ID` | Two tasks with the same `id` attribute | Each `#[task(id = "...")]` must be unique within a workflow |

**Warnings:**
- Non-standard characters in package name (stick to `a-z`, `0-9`, `_`, `-`)
- No tasks in package (the package compiles but does nothing)
- Empty version string

## Check 6: Security Assessment

**What it checks:** The binary is scanned for patterns that might indicate malicious or unintended behavior.

**Security levels:**

| Level | Meaning |
|-------|---------|
| Safe | No suspicious patterns detected |
| Warning | 1-2 suspicious patterns (e.g. shell command strings) |
| Dangerous | 3+ suspicious patterns |

**Suspicious patterns checked:**
- Shell access: `/bin/sh`, `system(`, `exec`
- Network access: `curl`, `wget`, `nc `
- File permissions: world-writable binary

Security warnings don't block registration (they're informational), but they're logged for operator awareness.

## Where Validation Runs

| Entry Point | When | Validation |
|-------------|------|------------|
| `POST /workflows/packages` | Uploading to the server | Full pipeline (all 6 checks) |
| `cloacinactl daemon register` | Registering with the daemon | Full pipeline (all 6 checks) |
| Directory watcher | Daemon auto-detects new `.cloacina` file | Full pipeline (all 6 checks) |

The validation is built into `WorkflowRegistryImpl::register_workflow()`, so it runs regardless of how the package reaches the system.

## Troubleshooting Checklist

If your package is rejected, work through this list:

1. **Did you build with `--release`?** Debug builds are larger and may behave differently.
2. **Is `crate-type = ["cdylib"]` in your Cargo.toml?** Without it, no shared library is produced.
3. **Did you use `#[packaged_workflow(...)]`?** This generates the FFI symbols.
4. **Are you building for the right platform?** macOS `.dylib` won't work on a Linux server.
5. **Is your `cloacina-workflow` dependency up to date?** Older versions used `futures::executor::block_on` for FFI execution, which doesn't provide a tokio runtime. Version 0.3.2+ fixes this.
6. **Does your task depend on external state during initialization?** The smoke test calls tasks with an empty context. If your task panics on missing config during *initialization* (not just execution), it will fail validation.

## Related

- [Tutorial 07: Packaged Workflows]({{< ref "/tutorials/07-packaged-workflows" >}}) — How to create workflow packages
- [Packaged Workflow Architecture]({{< ref "/explanation/packaged-workflow-architecture" >}}) — How the packaging system works internally
