---
title: "Use cloacina-compiler Locally"
description: "Build, pack, and inspect .cloacina packages without running the compiler service."
weight: 27
---

# How to Use `cloacina-compiler` Locally

This guide shows how to use the `cloacinactl package` commands to
build and pack `.cloacina` archives on your laptop or in CI, without
running the long-lived `cloacina-compiler` service. This is the path
most developers want for iterating on a workflow before deploying it.

> **When to use this:** local development, CI builds, ad-hoc
> packaging, smoke-testing a workflow's manifest before uploading to
> a server. **When NOT to use this:** production-grade signed
> packaging with audit trails — for that, run
> `cloacinactl compiler start` against the database build queue.

## Prerequisites

- Rust toolchain (the version specified in `rust-toolchain.toml` if
  present, or stable).
- `cloacinactl` on your `PATH`.
- A workflow crate that follows the [packaged workflow Cargo.toml
  layout]({{< ref "/workflows/how-to-guides/migrating-to-service-mode" >}}):
  `crate-type = ["cdylib", "rlib"]`, the `packaged` feature, and a
  `cloacina_build::configure()` call in `build.rs`.

## The Three-Step Local Loop

```bash
cd path/to/my-workflow

# 1. Compile the cdylib.
cloacinactl package build .

# 2. Pack the cdylib + manifest into a .cloacina archive.
cloacinactl package pack .

# 3. (Optional) Smoke-test by inspecting the archive metadata.
cloacinactl package inspect ./my-workflow.cloacina --offline
```

Step 3 inspect requires a server in the current implementation;
operators who want offline metadata extraction can use the
`fidius-host` CLI directly (the same loader the server uses). For
local development, `package pack` succeeding is usually enough.

## Step 1: `package build`

```text
cloacinactl package build <DIR> [--release]
```

This is a thin wrapper over `cargo build` that verifies the crate's
shape:

- `<DIR>/Cargo.toml` exists.
- `<DIR>/package.toml` exists (this is the Cloacina-specific manifest
  that `cloacina_build::configure()` consumes).
- `[lib]` declares `crate-type = ["cdylib", "rlib"]`.
- The `packaged` feature is enabled (or the default).

By default it builds the `dev` profile (faster, larger binary). For
production-grade builds:

```bash
cloacinactl package build . --release
```

Build artifacts land in `target/{debug,release}/` per cargo
conventions; `package pack` finds the right one.

**Common failure modes:**

- `Cargo.toml` missing the `cdylib` crate type → cargo emits an
  `rlib`-only artifact, `package pack` errors with "no cdylib found".
- `package.toml` missing → `package build` errors before invoking
  cargo.
- Dependencies still reference `cloacina` (the full crate) instead of
  the slim service-mode trio (`cloacina-workflow` +
  `cloacina-macros` + `cloacina-workflow-plugin`) → cdylib bloats to
  ~60 MB. Functional but expensive.

## Step 2: `package pack`

```text
cloacinactl package pack <DIR> [--out <PATH>] [--sign <KEY>]
```

Calls `fidius_core::package::pack_package()` to combine the cdylib
and the manifest into a single `.cloacina` archive (a zip file with
a Cloacina-specific layout).

```bash
# Default output: <crate-name>-<version>.cloacina in the current dir
cloacinactl package pack .

# Custom output path
cloacinactl package pack . --out /tmp/my-workflow-1.0.0.cloacina

# With sign — currently fail-hard (see note below)
cloacinactl package pack . --sign /path/to/private-key.pem
```

> **`--sign` is currently a fail-hard stub in the CLI.** The flag is
> accepted but `cloacinactl package pack` exits non-zero with an
> error message pointing operators at the library-side signing API
> (CLOACI-I-0103 wire-up is pending). The signature infrastructure
> exists at `cloacina::security::package_signer`
> (it produces a sidecar `.sig` file at `<archive>.sig`), but the
> CLI is not yet wired to invoke it. The flag is accepted to keep
> existing scripts compiling; the sidecar is not produced. This
> will be wired in a future release.

The packed archive is the artifact you upload to the server (`package
upload`) or drop into a daemon's watch directory.

## Step 3: `package publish` (One-Shot)

When you want build + pack + upload in a single command:

```bash
cloacinactl package publish . \
    --release \
    --tenant acme \
    --server https://cloacina.example.com \
    --api-key env:CLOACINA_API_KEY
```

Equivalent to running `package build`, `package pack`, and `package
upload` in sequence with the same arguments. Useful in deploy
scripts.

## When to Run `cloacina-compiler` Instead

The compiler service (`cloacinactl compiler start`) exists for a
different use case: a long-running process that polls a database for
pending build rows submitted by a remote workflow author. Use it
when:

- Multiple authors submit packages and you want centralized,
  reproducible builds.
- You need an audit trail of who built which package version when.
- You want signing to happen server-side with a hardware-backed key
  rather than on developer laptops.
- You want to enforce build-time policy (toolchain version, allowed
  dependencies, etc.).

For everything else — local iteration, CI builds, exploratory
packaging — `cloacinactl package build` and `pack` are simpler and
faster.

See [Run cloacina-compiler in Production]({{< ref "/service/how-to/running-the-compiler" >}}) for the service-side deployment posture: threat model, vendor curation, resource limits, audit-event reference.

## Local Loader Sanity Check

If you want to verify your `.cloacina` archive loads cleanly without
running a full server, the simplest path today is to point a
`cloacinactl daemon` instance at it:

```bash
# Set up an isolated home so this doesn't pollute your real config
export CLOACINA_HOME=/tmp/local-test
mkdir -p $CLOACINA_HOME/packages
cp my-workflow.cloacina $CLOACINA_HOME/packages/

# Start the daemon (will reconcile and load the package)
cloacinactl --home $CLOACINA_HOME daemon start &
DAEMON_PID=$!

# Watch the logs for load success/failure
tail -f $CLOACINA_HOME/logs/cloacina.log

# Clean up
kill $DAEMON_PID
rm -rf $CLOACINA_HOME
```

A successful load logs `package_loaded package_id=...
package_name=my-workflow version=1.0.0`. A failure logs the
specific reconciler step (cron triggers / custom triggers / reactors
/ trigger-less CGs / reactor-bound CGs / workflows) that errored.

## Related

- [Migrating to Service Mode]({{< ref "/workflows/how-to-guides/migrating-to-service-mode" >}}) — full Cargo.toml + build.rs setup for packaged workflows.
- [`package!()` Macro Reference]({{< ref "/reference/package-shell-macro" >}}) — what the cdylib actually exports.
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}) — what happens when a package is loaded.
- [CLI Reference]({{< ref "/reference/cli" >}}) — full `cloacinactl package` flag list.
- [Compiler Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) — running `cloacina-compiler` as a service.
