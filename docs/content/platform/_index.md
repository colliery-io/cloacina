---
title: "Platform"
description: "Cross-cutting platform concerns — CLI, HTTP API, database backends, deployment, and security"
weight: 30
---

# Platform

Platform documentation covers cross-cutting concerns that apply regardless of whether you're using workflows, computation graphs, or both — and regardless of whether you're in library or service mode.

**Who this is for:** operators and developers deploying and running Cloacina as a
service (the `cloacina-server`, compiler, and agent fleet) — applies to both
Rust- and Python-authored packages. **Prerequisites:** a working package (see
[Creating Your First Package]({{< ref "/service/how-to/creating-your-first-package" >}}))
and a Postgres database for multi-tenant/production deployments.

## Topics

### Tutorials
- [Deploy a Server]({{< ref "/service/tutorials/01-deploy-a-server" >}}) — first deployment, end to end
- [The Web UI]({{< ref "/service/tutorials/02-the-web-ui" >}}) — operate and observe a running server

### Reference
- [CLI Reference]({{< ref "/reference/cli" >}}) — `cloacinactl` commands and flags
- [HTTP API]({{< ref "/reference/http-api" >}}) — REST endpoints for the server
- [WebSocket Protocol]({{< ref "/reference/websocket-protocol" >}}) — Real-time event streaming
- [Database Admin API]({{< ref "/reference/database-admin" >}}) — Tenant provisioning
- [Configuration]({{< ref "/reference/configuration" >}}) — All config options
- [Environment Variables]({{< ref "/reference/environment-variables" >}}) — Runtime settings
- [Repository Structure]({{< ref "/reference/repository-structure" >}}) — Codebase layout

### Explanation
- [Database Backends]({{< ref "/service/explanation/database-backends" >}}) — PostgreSQL vs SQLite trade-offs
- [Multi-tenancy]({{< ref "/service/explanation/multi-tenancy" >}}) — Schema isolation architecture
- [Package Format]({{< ref "/engine/explanation/package-format" >}}) — .cloacina file structure
- [FFI System]({{< ref "/engine/explanation/ffi-system" >}}) — Dynamic library loading
- [Horizontal Scaling]({{< ref "/service/explanation/horizontal-scaling" >}}) — Multi-instance coordination
- [Performance Characteristics]({{< ref "/service/explanation/performance-characteristics" >}}) — Throughput and latency

### How-to Guides
- [Production Deployment]({{< ref "/service/how-to/production-deployment" >}}) — Deploy to production
- [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}}) — Server setup
- [Running the Daemon]({{< ref "/embed/how-to/running-the-daemon" >}}) — Local scheduler
- [Performance Tuning]({{< ref "/service/how-to/performance-tuning" >}}) — Optimization guide
- [Security]({{< ref "/service/how-to/security" >}}) — Authentication and authorization
