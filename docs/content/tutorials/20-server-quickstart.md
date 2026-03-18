---
title: "20 - Server Quick Start"
description: "Deploy the Cloacina server with Docker in 5 minutes"
weight: 30
reviewer: "dstorey"
review_date: "2026-03-16"
---

## Overview

This guide gets you from zero to a running Cloacina server with Docker Compose. By the end, you'll have a server accepting API requests, with Postgres for persistence and Prometheus metrics for monitoring.

## Prerequisites

- Docker and Docker Compose installed
- `curl` for testing

## Step 1: Start the Server

```bash
# From the cloacina repository root
docker-compose -f deploy/docker-compose.yml up -d
```

This starts:
- **PostgreSQL 16** on port 5432
- **Cloacina server** on port 8080 (all-in-one mode)

Wait a few seconds for Postgres to initialize, then verify:

```bash
curl http://localhost:8080/health
```

Expected response:

```json
{"status":"ok","version":"0.x.y","mode":"all","uptime_seconds":5}
```

(The version will match your build.)

## Step 2: Explore the API

Open the Swagger UI in your browser:

```
http://localhost:8080/api-docs/
```

Or fetch the raw OpenAPI spec:

```bash
curl http://localhost:8080/api-docs/openapi.json
```

## Step 3: Create a Super-Admin Key

Before you can use the API, you need an API key. Use `cloacinactl` inside the container:

```bash
docker-compose -f deploy/docker-compose.yml exec cloacina \
  cloacinactl api-key create-admin --name "bootstrap"
```

Save the returned key — it's shown only once:

```
API Key created successfully!

  cloacina_live__k7f3a9b2c1d4e5f6a8b9c0d1e2f3a4b5

Key ID: 550e8400-e29b-41d4-a716-446655440000
Prefix: live_

Save this key — it will not be shown again.
```

## Step 4: Create a Tenant

```bash
curl -X POST http://localhost:8080/tenants \
  -H "Authorization: Bearer cloacina_live__k7f3a9b2c1d4e5f6a8b9c0d1e2f3a4b5" \
  -H "Content-Type: application/json" \
  -d '{"name": "acme", "schema_name": "tenant_acme"}'
```

The response includes an initial admin API key for the tenant.

## Step 5: Upload a Workflow Package

Build a workflow package locally, then upload it:

```bash
# Build the package (from your workflow project directory)
cloacinactl package build --output .

# Upload via API
curl -X POST http://localhost:8080/workflows/packages \
  -H "Authorization: Bearer cloacina_live__k7f3a9b2c1d4e5f6a8b9c0d1e2f3a4b5" \
  -F "package=@my-workflow.cloacina"
```

## Step 6: Trigger an Execution

```bash
curl -X POST http://localhost:8080/executions \
  -H "Authorization: Bearer cloacina_live__k7f3a9b2c1d4e5f6a8b9c0d1e2f3a4b5" \
  -H "Content-Type: application/json" \
  -d '{"workflow_name": "my-workflow", "context": {}}'
```

Check status:

```bash
curl http://localhost:8080/executions/{execution_id} \
  -H "Authorization: Bearer cloacina_live__k7f3a9b2c1d4e5f6a8b9c0d1e2f3a4b5"
```

## Step 7: Check Metrics

Prometheus metrics are available without authentication:

```bash
curl http://localhost:8080/metrics
```

## Configuration

The server is configured via `deploy/cloacina.toml`. Key settings:

| Setting | Default | Description |
|---|---|---|
| `server.mode` | `all` | `all`, `api`, `worker`, `scheduler` |
| `server.port` | `8080` | HTTP listen port |
| `database.url` | — | PostgreSQL connection URL |
| `database.pool_size` | `10` | Connection pool size |
| `scheduler.poll_interval_ms` | `100` | Scheduler poll frequency |
| `worker.max_concurrent_tasks` | `10` | Max parallel task executions |
| `worker.task_timeout_seconds` | `300` | Per-task timeout |
| `logging.level` | `info` | Log verbosity |

Environment variables override config file values with the `CLOACINA_` prefix:

```bash
CLOACINA_DATABASE_URL=postgres://user:pass@host/db
CLOACINA_SERVER_PORT=9090
CLOACINA_LOG_LEVEL=debug
```

## Deployment Patterns

### Single Node (default)

```bash
cloacinactl serve --mode=all
```

Everything in one process. Good for small deployments.

### Separate API and Workers

```bash
# Node 1: API server
cloacinactl serve --mode=api

# Node 2-N: Workers (scale horizontally)
cloacinactl serve --mode=worker
```

Workers claim tasks via `FOR UPDATE SKIP LOCKED` — add more workers to increase throughput.

### Separate Schedulers

```bash
# Node 1: Scheduler
cloacinactl serve --mode=scheduler

# Node 2: Another scheduler (claims non-overlapping pipeline batches)
cloacinactl serve --mode=scheduler
```

Multiple schedulers use pipeline claiming to split work. Scale for throughput, not just HA.

## Stopping

```bash
docker-compose -f deploy/docker-compose.yml down

# To also remove the database volume:
docker-compose -f deploy/docker-compose.yml down -v
```

## Next Steps

- [**Tutorial 21: Server Workflow Management**]({{< ref "/tutorials/21-server-workflow-management" >}}) — Deep dive into uploading, executing, and managing workflows through the API
- [**Tutorial 22: Local Daemon Scheduler**]({{< ref "/tutorials/22-daemon-local-scheduler" >}}) — Run workflows locally with SQLite, no Docker required
- Explore the [Swagger UI](http://localhost:8080/api-docs/) for interactive API documentation
- Configure [continuous scheduling]({{< ref "/tutorials/12-continuous-scheduling" >}}) for reactive pipelines
- Set up Prometheus scraping from `/metrics` for monitoring
