---
title: "Run the Service"
description: "Operate Cloacina as a multi-tenant server with an HTTP/WebSocket API, a web UI, and horizontal scale."
weight: 20
aliases:
  - "/platform/"

---

# Run the Service

Operate Cloacina as a **service**: a `cloacina-server` control plane with an
HTTP + WebSocket API, schema-per-tenant isolation, a web UI, package upload, and
optional horizontal scale via the compiler and execution-agent fleet. You ship
workflows to it as `.cloacina` packages.

If instead you want to run the engine inside your own application, see
**[Embed the Library]({{< ref "/embed" >}})**. For the core objects both doors
share, see **[Engine & Primitives]({{< ref "/engine" >}})**.

**Who this is for:** operators running Cloacina as a server — standing it up,
securing it, and keeping it healthy.

**Prerequisites:** familiarity with Docker and basic deployment practices; access
to a PostgreSQL backend for multi-tenant production (SQLite is fine for a first run).

{{< toc-tree >}}
