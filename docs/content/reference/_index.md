---
title: "Reference"
description: "Lookup-style reference: APIs (Rust & Python), CLI, HTTP/WebSocket, configuration, metrics, SDKs, and the glossary."
weight: 40
aliases:
  - "/computation-graphs/reference/"
  - "/platform/reference/"
  - "/python/computation-graphs/reference/"
  - "/python/workflows/reference/"
  - "/workflows/reference/"

---

# Reference

Austere, lookup-oriented reference for every surface — independent of which path
you run. APIs for Rust and Python, the `cloacinactl` CLI, the HTTP and WebSocket
protocols, configuration and environment variables, the metrics catalog, client
SDKs, the glossary, and troubleshooting.

**Who this is for:** readers looking up a specific API, flag, endpoint, or config
value — not learning a concept for the first time.

**Prerequisites:** none. This is lookup material; start from
**[Start Here]({{< ref "/start" >}})** or **[Engine & Primitives]({{< ref "/engine" >}})**
if you need the concepts behind a symbol.

{{< toc-tree >}}

## Generated API reference

Auto-generated from source (rustdoc / pdoc) — the authoritative symbol-level API:

- [Rust API]({{< ref "/api-reference/rust" >}}) — `cloacina`, `cloacina-macros`, and the computation-graph crates.
- [Python API (`cloaca`)]({{< ref "/api-reference/cloaca" >}}) — the PyO3 bindings.
