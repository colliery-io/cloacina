---
title: "Run the Service"
description: "Operate Cloacina as a multi-tenant server with an HTTP/WebSocket API, a web UI, and horizontal scale."
weight: 20
---

# Run the Service

Operate Cloacina as a **service**: a `cloacina-server` control plane with an
HTTP + WebSocket API, schema-per-tenant isolation, a web UI, package upload, and
optional horizontal scale via the compiler and execution-agent fleet. You ship
workflows to it as `.cloacina` packages.

If instead you want to run the engine inside your own application, see
**[Embed the Library]({{< ref "/embed" >}})**. For the core objects both doors
share, see **[Engine & Primitives]({{< ref "/engine" >}})**.

{{< toc-tree >}}
