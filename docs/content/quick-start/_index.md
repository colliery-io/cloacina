---
title: "Quick Start"
description: "Where to start with Cloacina, by language and goal."
weight: 10
---

# Quick Start

Welcome to Cloacina. This page is a navigation aid — it points you
into the right tutorial or reference based on what you want to do.
For substance (working code, exhaustive references, conceptual
explanations) follow the links below into the dedicated tracks.

## Install the CLI (optional, recommended)

If you'll be running a Cloacina server or compiling packages, install the
`cloacinactl` CLI first → [Installing cloacinactl]({{< ref "install" >}}) (one-line
installer, version pinning, platform support, Docker / Helm alternatives). The
embedded-mode Rust tutorials below don't require the CLI; pure Python tutorials
don't either.

## New to Cloacina?

Cloacina is an embedded workflow orchestration engine for Rust and Python
(the Python bindings are [Cloaca]({{< ref "/python" >}}), full parity with Rust).
It runs inside your application rather than as a separate service,
manages multi-step pipelines with automatic retries and state
persistence, and ships packaged workflows as `.cloacina` packages.

Get oriented first:

- [When to Use Cloacina]({{< ref "when-to-use" >}}) — does it fit your problem, and which mode/primitive?
- [Features Overview]({{< ref "features" >}}) — the full capability catalog.
- [Concepts]({{< ref "concepts" >}}) — the core primitives, defined.
- [Architecture Overview]({{< ref "/workflows/explanation/architecture-overview" >}}) — the design rationale.

**Just want to see it run?** The fastest "watch real executions" path is to
[deploy a server]({{< ref "/platform/tutorials/01-deploy-a-server" >}}) and open
[the web UI]({{< ref "/platform/tutorials/02-the-web-ui" >}}). For a code-only
first success, the [Python Quick Start]({{< ref "/python/quick-start" >}}) runs a
workflow in about five minutes.

## Pick your starting point

### "I want to run a workflow inside a Rust binary."

Embedded mode. Start here:

→ [Tutorial 01 — Your First Workflow]({{< ref "/workflows/tutorials/library/01-first-workflow" >}})

The full embedded-mode tutorial track is at
[Workflows → Tutorials]({{< ref "/workflows/tutorials" >}}). It
covers your first workflow, context handling, complex dependency
graphs, and error handling.

### "I want to run a workflow from Python."

→ [Python Quick Start]({{< ref "/python/quick-start" >}})

The full Python tutorial track is at
[Python · Workflows · Tutorials]({{< ref "/python/workflows/tutorials" >}}). Same shape
as the Rust track but Pythonic syntax and `WorkflowBuilder` context
managers. The Python-side computation-graph tutorials (09–11) live at
[Python · Computation Graphs · Tutorials]({{< ref "/python/computation-graphs/tutorials" >}}).

### "I want to ship workflows as `.cloacina` packages and load them into a server."

Service mode. Start here:

→ [Tutorial 01 — Deploy a Server]({{< ref "/platform/tutorials/01-deploy-a-server" >}})

This walks through bootstrap-key handling, tenant provisioning,
package upload, and your first execution. Once you have a server
running, branch into:

- [Migrating to Service Mode]({{< ref "/workflows/how-to-guides/migrating-to-service-mode" >}})
  — convert an embedded-mode workflow into a loadable `.cloacina` package.
- [Use cloacina-compiler Locally]({{< ref "/platform/how-to-guides/use-cloacina-compiler-locally" >}})
  — build packages on your laptop without the long-running compiler
  service.

### "I want to build a computation graph (event-driven, low-latency)."

→ [Computation Graphs Tutorials]({{< ref "/computation-graphs/tutorials" >}})

The CG track has its own progression: a single-graph embedded-mode
tutorial, accumulator patterns, full reactor pipelines, routing,
and the cross-package binding pattern at
[Tutorial 10 — Cross-Package Reactor Binding]({{< ref "/computation-graphs/tutorials/service/10-cross-package-binding" >}}).

### "I'm an operator standing up Cloacina in production."

→ [Platform Tutorials]({{< ref "/platform/tutorials" >}}) +
[Platform How-To Guides]({{< ref "/platform/how-to-guides" >}})

Highlights:
- [Deploy a Server]({{< ref "/platform/tutorials/01-deploy-a-server" >}}) — first deployment, end to end.
- [Configure a Multi-Tenant Deployment]({{< ref "/platform/how-to-guides/configure-multi-tenant-deployment" >}}) — multi-tenancy + the operational caveats you must know.
- [Production Deployment]({{< ref "/platform/how-to-guides/production-deployment" >}}) — TLS, reverse proxy, observability hookup.
- [Observe Execution State]({{< ref "/workflows/how-to-guides/observe-execution-state" >}}) — Prometheus + OpenTelemetry + structured logs.
- [Safely Unload a Package]({{< ref "/platform/how-to-guides/safely-unload-a-package" >}}) — clean teardown for cross-package deployments.

## Reference at a glance

For lookup-style information rather than learning paths:

- [CLI Reference]({{< ref "/platform/reference/cli" >}}) — every
  `cloacinactl` command, flag, env var, exit code, config-file
  schema.
- [HTTP API Reference]({{< ref "/platform/reference/http-api" >}}) —
  every endpoint, auth model, operational caveats.
- [Glossary]({{< ref "/glossary" >}}) — every term used in these
  docs, in one place.

Advanced (only if you implement package loading by hand):

- [`package!()` Macro Reference]({{< ref "/platform/reference/package-shell-macro" >}}) — the macro that wraps a Rust package for loading.
- [FFI Vtable Reference]({{< ref "/platform/reference/ffi-vtable" >}}) — the low-level plugin ABI.

## Need help?

- [Troubleshooting]({{< ref "/troubleshooting" >}}) — common
  problems and resolutions.
- [GitHub issues](https://github.com/colliery-io/cloacina/issues) —
  bug reports and feature requests.
