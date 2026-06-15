---
title: "Creating Your First Package"
description: "Scaffold, validate, pack, and upload a .cloacina package with cloacinactl package new"
weight: 15
---

# Creating Your First Package

`cloacinactl package new` scaffolds a canonical `.cloacina` source tree so you
start from a working, server-accepted skeleton instead of hand-assembling
`package.toml` and the directory layout. This guide walks the full loop:
**new → validate → pack → upload**.

## Prerequisites

- `cloacinactl` on your `PATH` (and a configured server profile for the upload
  step — see [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}})).
- For Python packages: nothing else. For Rust packages: a Rust toolchain.

## Step 1: Scaffold

```bash
cloacinactl package new data-pipeline --lang python
```

`--lang` is `python` (default) or `rust`. `--kind` selects the package shape:

| `--kind` | Python | Rust | Scaffolds |
|----------|--------|------|-----------|
| `workflow` (default) | ✅ | ✅ | tasks (`@cloaca.task` / `#[task]`) |
| `graph` | ✅ | ✅ | a computation graph (reactor + nodes) |
| `cron` | — | ✅ | a workflow fired by a cron `#[trigger(on, cron)]` |

> Cron triggers are Rust-only. Python packages use **poll** triggers
> (`@cloaca.trigger(name=…, poll_interval=…)`) inside a `workflow` package, not
> cron.

By default the package is created in `./<name>/`; pass `--path <dir>` to choose
another location. The package name's hyphens become underscores for the
module/workflow identifier (`data-pipeline` → module `data_pipeline`).

What you get (Python workflow):

```
data-pipeline/
├── package.toml
└── workflow/
    └── data_pipeline/
        ├── __init__.py
        └── tasks.py        # bare @cloaca.task decorators
```

Edit the generated tasks/nodes to do your real work. (Python packaged modules
use **bare decorators** — the loader names the workflow from `workflow_name` /
`graph_name`; don't wrap them in a `WorkflowBuilder`. See
[Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}).)

## Step 2: Validate

`package validate` runs the same checks the server would — the closed
`[metadata]` schema and the language-specific layout — plus author-time footgun
lints, against a source directory **or** a packed archive:

```bash
cloacinactl package validate data-pipeline
```

It catches, before upload:

- unknown / rejected `[metadata]` keys (`package_type`, `[[metadata.triggers]]`);
- a Python module tree not under `workflow/`, or an `entry_module` that doesn't resolve;
- a missing Rust `Cargo.toml` / `src/lib.rs`;
- an unrewritten `__WORKSPACE__` placeholder in `Cargo.toml`;
- a computation-graph package that forgot `graph_name`;
- a cron trigger mistakenly listed in `#[workflow(triggers = [...])]` (cron
  triggers bind via `on`).

## Step 3: Pack

```bash
cloacinactl package pack data-pipeline --out data-pipeline-0.1.0.cloacina
```

`pack` re-runs validation, then writes the bzip2-tar `.cloacina` archive. (Note:
the output flag is `--out`; `-o` is the global output-*format* flag.) For Rust,
run `cloacinactl package build data-pipeline` first if you want to compile-check
locally — the server compiles on load regardless.

## Step 4: Upload

```bash
cloacinactl --tenant my_tenant package upload data-pipeline-0.1.0.cloacina
```

The server stores the package and (with a `cloacina-compiler` service running)
builds + registers it. `cloacinactl package publish data-pipeline` does
build + pack + upload in one step.

## See Also

- [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}) — the Python format in depth
- [Package Format]({{< ref "/engine/explanation/package-format" >}}) — archive layout + `package.toml` schema
