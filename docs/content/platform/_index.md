---
title: "Platform"
description: "Cross-cutting platform concerns — CLI, HTTP API, database backends, deployment, and security"
weight: 30
---

# Platform

Platform documentation covers cross-cutting concerns that apply regardless of whether you're using workflows, computation graphs, or both — and regardless of whether you're in library or service mode.

## Topics

### Reference
- [CLI Reference]({{< ref "/platform/reference/cli" >}}) — `cloacinactl` commands and flags
- [HTTP API]({{< ref "/platform/reference/http-api" >}}) — REST endpoints for the server
- [WebSocket Protocol]({{< ref "/platform/reference/websocket-protocol" >}}) — Real-time event streaming
- [Database Admin API]({{< ref "/platform/reference/database-admin" >}}) — Tenant provisioning
- [Configuration]({{< ref "/platform/reference/configuration" >}}) — All config options
- [Environment Variables]({{< ref "/platform/reference/environment-variables" >}}) — Runtime settings
- [Repository Structure]({{< ref "/platform/reference/repository-structure" >}}) — Codebase layout

### Explanation
- [Database Backends]({{< ref "/platform/explanation/database-backends" >}}) — PostgreSQL vs SQLite trade-offs
- [Multi-tenancy]({{< ref "/platform/explanation/multi-tenancy" >}}) — Schema isolation architecture
- [Package Format]({{< ref "/platform/explanation/package-format" >}}) — .cloacina file structure
- [FFI System]({{< ref "/platform/explanation/ffi-system" >}}) — Dynamic library loading
- [Horizontal Scaling]({{< ref "/platform/explanation/horizontal-scaling" >}}) — Multi-instance coordination
- [Performance Characteristics]({{< ref "/platform/explanation/performance-characteristics" >}}) — Throughput and latency

### How-to Guides
- [Production Deployment]({{< ref "/platform/how-to-guides/production-deployment" >}}) — Deploy to production
- [Deploying the API Server]({{< ref "/platform/how-to-guides/deploying-the-api-server" >}}) — Server setup
- [Running the Daemon]({{< ref "/platform/how-to-guides/running-the-daemon" >}}) — Local scheduler
- [Performance Tuning]({{< ref "/platform/how-to-guides/performance-tuning" >}}) — Optimization guide
- [Security]({{< ref "/platform/how-to-guides/security" >}}) — Authentication and authorization
