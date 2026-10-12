---
title: "Run the Service"
description: "Operate Cloacina as a multi-tenant server with an HTTP/WebSocket API, a web UI, and horizontal scale."
sidebar:
  order: 20
---

Operate Cloacina as a **service**: a `cloacina-server` control plane with an
HTTP + WebSocket API, schema-per-tenant isolation, a web UI, package upload, and
optional horizontal scale via the compiler and execution-agent fleet. You ship
workflows to it as `.cloacina` packages.

If instead you want to run the engine inside your own application, see
**[Embed the Library](../embed/)**. For the core objects both doors
share, see **[Engine & Primitives](../engine/)**.

**Who this is for:** operators running Cloacina as a server — standing it up,
securing it, and keeping it healthy.

**Prerequisites:** familiarity with Docker and basic deployment practices; access
to a PostgreSQL backend for multi-tenant production (SQLite is fine for a first run).

- [Quick Start](../tutorials/service/quick-start/)
- [Tutorials](../tutorials/service/)
  - [01 - Deploy a Server](../tutorials/service/01-deploy-a-server/)
  - [02 - The Web UI](../tutorials/service/02-the-web-ui/)
  - [03 — Packaged Workflows](../tutorials/service/03-packaged-workflows/)
  - [04 — Packaging a Computation Graph](../tutorials/service/04-packaging/)
  - [05 — WebSocket Event Injection](../tutorials/service/05-websocket-events/)
  - [06 — Kafka-Sourced Computation Graphs](../tutorials/service/06-kafka-stream/)
  - [07 — Cross-Package Reactor Binding](../tutorials/service/07-cross-package-binding/)
- [How-to Guides](../how-to/service/)
  - [Creating Your First Package](../how-to/service/creating-your-first-package/)
  - [Manage API Keys](../how-to/service/manage-api-keys/)
  - [Safely Unload a Package](../how-to/service/safely-unload-a-package/)
  - [Configure Local Accounts](../how-to/service/configure-local-accounts/)
  - [Configure a Multi-Tenant Deployment](../how-to/service/configure-multi-tenant-deployment/)
  - [Manage Secrets](../how-to/service/manage-secrets/)
  - [Configure OIDC Single Sign-On](../how-to/service/configure-oidc-sso/)
  - [Use cloacina-compiler Locally](../how-to/service/use-cloacina-compiler-locally/)
  - [Run cloacina-compiler in Production](../how-to/service/running-the-compiler/)
  - [Multi-Tenant Recovery](../how-to/service/multi-tenant-recovery/)
  - [Production Deployment](../how-to/service/production-deployment/)
  - [Back Up and Restore](../how-to/service/backup-and-restore/)
  - [Configure PostgreSQL Schema-Based Multi-Tenancy](../how-to/service/multi-tenant-setup/)
  - [Performance Tuning](../how-to/service/performance-tuning/)
  - [Security](../how-to/service/security/)
    - [Package Signing](../how-to/service/security/package-signing/)
    - [Local Development](../how-to/service/security/local-development/)
  - [Deploying the API Server](../how-to/service/deploying-the-api-server/)
  - [Migrating from Library to Service Mode](../how-to/service/migrating-to-service-mode/)
  - [Running the cloacina-server Docker image](../how-to/service/running-the-server-image/)
  - [Deploying to Kubernetes (Helm)](../how-to/service/deploying-to-kubernetes/)
  - [Deploy an Execution-Agent Fleet](../how-to/service/deploy-an-execution-agent-fleet/)
  - [Compiler + Server Deployment Runbook](../how-to/service/compiler-deployment-runbook/)
  - [Deploy the Web UI](../how-to/service/deploy-the-web-ui/)
  - [Decommission a tenant](../how-to/service/decommission-a-tenant/)
  - [Require signed packages](../how-to/service/require-signed-packages/)
  - [Use CLI profiles](../how-to/service/use-cli-profiles/)
- [Explanation](../explanation/service/)
  - [Reconciler Pipeline](../explanation/service/reconciler-pipeline/)
  - [Database Backends](../explanation/service/database-backends/)
  - [Security Model](../explanation/service/security-model/)
  - [Secrets](../explanation/service/secrets/)
  - [Horizontal Scaling](../explanation/service/horizontal-scaling/)
  - [Observability](../explanation/service/observability/)
  - [Execution-Agent Fleet](../explanation/service/execution-agent-fleet/)
  - [Multi-Tenancy Architecture](../explanation/service/multi-tenancy/)
  - [Performance Characteristics](../explanation/service/performance-characteristics/)
- [Embedded Web UI](../how-to/service/embedded-ui/)
