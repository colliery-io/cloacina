---
title: "Use cloacina-compiler Locally"
description: "Build, validate, and pack .cloacina packages without running the compiler service."
weight: 27
aliases:
  - "/platform/how-to-guides/use-cloacina-compiler-locally/"

---

# How to Use `cloacina-compiler` Locally

This guide shows how to use the `cloacinactl package` commands to
check, validate, and pack `.cloacina` archives on your laptop or in
CI, without running the long-lived `cloacina-compiler` service. This
is the path most developers want for iterating on a workflow before
deploying it.

> **The key fact:** a `.cloacina` archive is a **source** archive
> (tar + bzip2). The shared library is compiled **server-side** by the
> `cloacina-compiler` service after upload — nothing you build locally
> ships in the archive. Local `package build` is purely a
> catch-errors-early compile check.

## Prerequisites

- Rust toolchain (stable) for Rust packages; nothing extra for Python
  packages.
- `cloacinactl` on your `PATH`.
- A package source tree in the canonical shape — scaffold one with
  `cloacinactl package new` (see
  [Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}})).
  Rust packages need **no** `[lib] crate-type`, no `packaged` feature,
  and no `build.rs`: the compiler service injects the build wiring.

## The Three-Step Local Loop

```bash
cd path/to/my-workflow

# 1. (Rust only) Compile check — catches errors before upload.
cloacinactl package build .

# 2. Validate the source tree against the canonical package format.
cloacinactl package validate .

# 3. Pack the source + manifest into a .cloacina archive.
cloacinactl package pack .
```

`package validate` also accepts a packed archive
(`cloacinactl package validate ./my-workflow.cloacina`), so you can
smoke-test the artifact you're about to upload.

## Step 1: `package build`

```text
cloacinactl package build <DIR> [--release]
```

A thin wrapper over `cargo build` run in `<DIR>`. For Python packages
(`[metadata].language = "python"` or inferred) it is a no-op — there
is nothing to compile locally.

This step is optional: the server-side compiler performs the real
build. Running it locally just means you find compile errors on your
laptop instead of in the server's build queue.

**Common failure modes:**

- `package.toml` missing → the language probe errors before invoking
  cargo.
- Dependencies still reference `cloacina` (the full crate) instead of
  the slim packaged pair (`cloacina-workflow` +
  `cloacina-workflow-plugin`) → slow builds and a bloated artifact.
  See [Migrating to Service Mode]({{< ref "/service/how-to/migrating-to-service-mode" >}}).

## Step 2: `package validate`

```text
cloacinactl package validate <DIR | ARCHIVE>
```

Checks the source tree (or archive) against the canonical package
format without uploading: manifest schema, Python `workflow/` module
layout, rejected legacy keys (`package_type`, `[[metadata.triggers]]`),
and common footguns.

## Step 3: `package pack`

```text
cloacinactl package pack <DIR> [--out <PATH>] [--sign <KEY>]
```

Validates, then packs the **source tree** into a `.cloacina` archive
(tar + bzip2, manifest resolved to its full form). Build output, VCS
dirs, and prior archives are excluded automatically.

```bash
# Default output: <DIR>/<name>.cloacina
cloacinactl package pack .

# Custom output path
cloacinactl package pack . --out /tmp/my-workflow-1.0.0.cloacina
```

> **`--sign` fails hard.** Workflow-package signing is not implemented
> (tracked under I-0103): passing `--sign` makes `package pack` (and
> `package publish`) exit non-zero with an error telling you to remove
> the flag. Do not script around it — there is currently no way to
> produce a signed workflow package from the CLI, even though the
> server can *require* signatures (`--require-signatures`). See
> [Package Signing]({{< ref "/service/how-to/security/package-signing" >}})
> for the current state.

The packed archive is the artifact you upload to the server
(`package upload`) or drop into a daemon's watch directory.

## One-Shot: `package publish`

When you want build + pack + upload in a single command:

```bash
cloacinactl package publish . \
    --release \
    --tenant acme \
    --server https://cloacina.example.com \
    --api-key env:CLOACINA_API_KEY
```

Equivalent to running `package build`, `package pack`, and
`package upload` in sequence. Useful in deploy scripts. (Remember the
upload only becomes runnable once a `cloacina-compiler` service builds
it — see below.)

## When You Still Need `cloacina-compiler`

The compiler service is not optional for **running** Rust packages: an
uploaded Rust package sits at `build_status = pending` until a
`cloacina-compiler` process polling the same database claims and
builds it. What's local-optional is only the pre-upload authoring
loop above.

Run the service (locally or in your deployment) when:

- You want an uploaded Rust package to actually load and execute.
- Multiple authors submit packages and you want centralized,
  reproducible builds.
- You want to enforce build-time policy (vendored dependencies,
  resource limits, timeouts).

```bash
cloacinactl compiler start --database-url "$DATABASE_URL"
```

Python packages skip the compiler entirely — the server imports them
directly at reconcile time.

See [Run cloacina-compiler in Production]({{< ref "/service/how-to/running-the-compiler" >}}) for the service-side deployment posture: threat model, vendor curation, resource limits, audit-event reference.

## Related

- [Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}}) — scaffold → validate → pack → upload from zero.
- [Migrating to Service Mode]({{< ref "/service/how-to/migrating-to-service-mode" >}}) — converting an embedded workflow crate to the packaged shape.
- [`package!()` Macro Reference]({{< ref "/reference/package-shell-macro" >}}) — what the compiled plugin actually exports.
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}}) — what happens when a package is loaded.
- [CLI Reference]({{< ref "/reference/cli" >}}) — full `cloacinactl package` flag list.
- [Compiler Deployment Runbook]({{< ref "/service/how-to/compiler-deployment-runbook" >}}) — running `cloacina-compiler` as a service.
