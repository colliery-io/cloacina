---
title: "Platform Tutorials"
description: "End-to-end tutorials for operators deploying and managing Cloacina."
weight: 25
---

# Platform Tutorials

Step-by-step tutorials for operators standing up a Cloacina
deployment for the first time. These complement the
[workflows]({{< ref "/workflows/tutorials" >}}) and
[computation-graphs]({{< ref "/computation-graphs/tutorials" >}})
tutorial tracks, which focus on the developer experience.

## Path

1. [Deploy a Server]({{< ref "01-deploy-a-server" >}}) — start a
   `cloacina-server`, capture the bootstrap key, create a tenant,
   upload your first package, and confirm a clean execution.

After this, branch into:

- [Configure a Multi-Tenant Deployment]({{< ref "/platform/how-to-guides/configure-multi-tenant-deployment" >}})
  — multi-tenant production layout.
- [Production Deployment]({{< ref "/platform/how-to-guides/production-deployment" >}})
  — TLS termination, reverse proxy, observability hookup.
- [Observe Execution State]({{< ref "/workflows/how-to-guides/observe-execution-state" >}})
  — wire up Prometheus, OpenTelemetry, and structured logging.
