---
title: "Reference"
description: "Lookup-style reference: APIs (Rust & Python), CLI, HTTP/WebSocket, configuration, metrics, SDKs, and the glossary."
sidebar:
  order: 40
---

Austere, lookup-oriented reference for every surface — independent of which path
you run. APIs for Rust and Python, the `cloacinactl` CLI, the HTTP and WebSocket
protocols, configuration and environment variables, the metrics catalog, client
SDKs, the glossary, and troubleshooting.

**Who this is for:** readers looking up a specific API, flag, endpoint, or config
value — not learning a concept for the first time.

**Prerequisites:** none. This is lookup material; start from
**[Start Here](../start/)** or **[Engine & Primitives](../engine/)**
if you need the concepts behind a symbol.

- [CLI Reference](cli/)
- [HTTP API Reference](http-api/)
- [Configuration Reference](configuration/)
- [Macro Reference](macros/)
- [Error Reference](errors/)
- [Python Environment Variables](python-environment-variables/)
- [Repository Structure](repository-structure/)
- [Topology Dict Schema](topology-dict-schema/)
- [API Error Envelope](api-error-envelope/)
- [Metrics Catalog](metrics-catalog/)
- [DatabaseAdmin API Reference](database-admin/)
- [Package Signing API Reference](package-signing-api/)
- [Environment Variables](environment-variables/)
- [cloacina-testing API Reference](testing-crate/)
- [package! Macro Reference](package-shell-macro/)
- [FFI Vtable Reference](ffi-vtable/)
- [Computation Graph Reference](computation-graphs/)
- [Connector Contract](connector-contract/)
- [SDKs](sdks/)
  - [Rust SDK](sdks/rust/)
  - [Python SDK](sdks/python/)
  - [TypeScript SDK](sdks/typescript/)
- [WebSocket Protocol](websocket-protocol/)
- [API Reference](python-api/)
  - [Context](python-api/context/)
  - [DefaultRunner](python-api/runner/)
  - [WorkflowBuilder](python-api/workflow-builder/)
  - [Database Admin](python-api/database-admin/)
  - [Task Decorator](python-api/task/)
  - [Computation Graphs](python-api/computation-graphs/)
  - [Trigger Decorator](python-api/trigger/)
  - [Workflow](python-api/workflow/)
  - [Configuration](python-api/configuration/)
  - [WorkflowResult](python-api/pipeline-result/)
  - [Exceptions](python-api/exceptions/)
- [Package Manifest](package-manifest/)
- [Glossary](glossary/)
- [Troubleshooting](troubleshooting/)

## Generated API reference

Auto-generated from source by [plissken](https://github.com/colliery-io/plissken) (`plissken.toml` at the repo root) — the authoritative symbol-level API:

- [Rust API](api/) — `cloacina`, `cloacina-macros`, and the computation-graph crates.
- [Python API (`cloaca`)](api/cloaca/) — the PyO3 bindings.
