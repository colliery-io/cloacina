---
title: "21 - Server Workflow Management"
description: "Upload, list, execute, monitor, and delete workflow packages through the Cloacina server HTTP API"
weight: 31
draft: true
---

## Overview

This tutorial walks through the complete workflow package lifecycle using the Cloacina server's HTTP API. You'll build a real workflow, upload it to a running server, trigger executions, poll for results, and clean up — all with `curl`.

This covers the same concepts as [Tutorial 08: Working with the Workflow Registry]({{< ref "/tutorials/08-workflow-registry" >}}), but through the server's REST API instead of the Rust library interface.

## Prerequisites

- A running Cloacina server ([Tutorial 20: Server Quick Start]({{< ref "/tutorials/20-server-quickstart" >}}))
- An admin API key (created in Tutorial 20, Step 3)
- The Rust toolchain (for building the example workflow)

For all examples below, we'll use this variable:

```bash
export API_KEY="cloacina_live__YOUR_KEY_HERE"
export SERVER="http://localhost:8080"
```

## Step 1: Build a Workflow Package

We'll use the `simple-packaged-demo` example that ships with Cloacina. It defines a three-task pipeline: `collect_data` → `process_data` → `generate_report`.

```bash
# Build the shared library
cd examples/features/simple-packaged
cargo build --release
```

This produces a `.so` (Linux) or `.dylib` (macOS) in `target/release/`. Now wrap it into a `.cloacina` package (a tar.gz archive):

```bash
cd target/release
# Linux:
tar czf simple-packaged-demo.cloacina libsimple_packaged_demo.so
# macOS:
tar czf simple-packaged-demo.cloacina libsimple_packaged_demo.dylib
```

The `.cloacina` format is a gzipped tar containing the compiled shared library. The server extracts the library, validates its symbols, reads its metadata via FFI, and stores both the binary and metadata in the database.

## Step 2: Upload the Package

```bash
curl -X POST "$SERVER/workflows/packages" \
  -H "Authorization: Bearer $API_KEY" \
  -F "package=@simple-packaged-demo.cloacina"
```

Response:

```json
{
  "id": "a172ed14-eb2a-4bee-a593-a667cdfbec6d",
  "package_name": "registered (285494 bytes)",
  "message": "Workflow package registered successfully"
}
```

The `id` is the package UUID. Save it — you'll need it to delete the package later.

### What happens on upload

1. The server extracts the `.so`/`.dylib` from the tar.gz archive
2. `PackageValidator` loads the library and checks for required FFI symbols (`cloacina_execute_task`, `cloacina_get_task_metadata`)
3. `PackageLoader` extracts metadata (package name, version, task list, dependency graph)
4. The binary is stored in the `workflow_registry` table; metadata goes into `workflow_packages`
5. Tasks are registered in the global task registry

The **registry reconciler** (running in the background every 5 seconds) then detects the new package, loads it, and registers a workflow constructor in the global workflow registry. After this, the workflow is available for execution.

### Handling duplicate uploads

If you upload the same package twice:

```json
{
  "error": {
    "code": "BAD_REQUEST",
    "message": "Failed to register workflow package: Package already exists: simple_demo v1.0.0"
  }
}
```

Delete the existing package first (Step 6), then re-upload.

## Step 3: List Registered Workflows

```bash
curl -s "$SERVER/workflows" \
  -H "Authorization: Bearer $API_KEY" | python3 -m json.tool
```

Response:

```json
[
  {
    "id": "a172ed14-eb2a-4bee-a593-a667cdfbec6d",
    "name": "simple_demo",
    "version": "1.0.0",
    "description": null,
    "tasks": ["collect_data", "process_data", "generate_report"]
  }
]
```

## Step 4: Execute a Workflow

Wait a few seconds after uploading for the reconciler to register the workflow, then trigger an execution:

```bash
curl -X POST "$SERVER/executions" \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"workflow_name": "data_processing", "context": {"source": "tutorial"}}'
```

Response:

```json
{
  "execution_id": "9cca3228-ed98-4b60-bacc-d2bcb4eb5b46",
  "status": "accepted"
}
```

{{< hint type=info title="Workflow Name vs Package Name" >}}
The `workflow_name` in the execution request is the **workflow name** from the `#[packaged_workflow(name = "data_processing")]` macro attribute, not the package name (`simple_demo`). A single package can contain one workflow with multiple tasks.
{{< /hint >}}

## Step 5: Monitor Execution

Poll the execution status:

```bash
curl -s "$SERVER/executions/9cca3228-ed98-4b60-bacc-d2bcb4eb5b46" \
  -H "Authorization: Bearer $API_KEY" | python3 -m json.tool
```

While running:

```json
{
  "execution_id": "9cca3228-ed98-4b60-bacc-d2bcb4eb5b46",
  "workflow_name": "data_processing",
  "status": "Running",
  "started_at": "2026-03-18T01:38:17+00:00",
  "completed_at": null,
  "error_message": null,
  "task_results": [
    {"task_name": "collect_data", "status": "Completed", "error_message": null},
    {"task_name": "process_data", "status": "Running", "error_message": null},
    {"task_name": "generate_report", "status": "Pending", "error_message": null}
  ]
}
```

When complete:

```json
{
  "execution_id": "9cca3228-ed98-4b60-bacc-d2bcb4eb5b46",
  "workflow_name": "data_processing",
  "status": "Completed",
  "started_at": "2026-03-18T01:38:17+00:00",
  "completed_at": "2026-03-18T01:38:18+00:00",
  "error_message": null,
  "task_results": [
    {"task_name": "collect_data", "status": "Completed", "error_message": null},
    {"task_name": "process_data", "status": "Completed", "error_message": null},
    {"task_name": "generate_report", "status": "Completed", "error_message": null}
  ]
}
```

### Execution Control

Pause a running execution:

```bash
curl -X POST "$SERVER/executions/{id}/pause" \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"reason": "investigating issue"}'
```

Resume it:

```bash
curl -X POST "$SERVER/executions/{id}/resume" \
  -H "Authorization: Bearer $API_KEY"
```

Cancel it:

```bash
curl -X DELETE "$SERVER/executions/{id}" \
  -H "Authorization: Bearer $API_KEY"
```

### List All Executions

```bash
curl -s "$SERVER/executions" \
  -H "Authorization: Bearer $API_KEY" | python3 -m json.tool
```

## Step 6: Delete a Workflow Package

To remove a registered package:

```bash
curl -X DELETE "$SERVER/workflows/packages/a172ed14-eb2a-4bee-a593-a667cdfbec6d" \
  -H "Authorization: Bearer $API_KEY"
```

Returns `204 No Content` on success. This removes the binary and metadata from the database. The reconciler will unregister the workflow from the global registry on its next tick.

## API Reference Summary

All endpoints require `Authorization: Bearer <api_key>` except `/health` and `/metrics`.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check (public) |
| `GET` | `/metrics` | Prometheus metrics (public) |
| `POST` | `/workflows/packages` | Upload a `.cloacina` package (multipart) |
| `GET` | `/workflows` | List registered workflow packages |
| `DELETE` | `/workflows/packages/{id}` | Delete a workflow package |
| `POST` | `/executions` | Trigger a workflow execution |
| `GET` | `/executions` | List recent executions |
| `GET` | `/executions/{id}` | Get execution status and task results |
| `POST` | `/executions/{id}/pause` | Pause a running execution |
| `POST` | `/executions/{id}/resume` | Resume a paused execution |
| `DELETE` | `/executions/{id}` | Cancel an execution |
| `POST` | `/tenants` | Create a tenant |
| `GET` | `/tenants` | List tenants |
| `GET` | `/tenants/{id}` | Get tenant details |
| `DELETE` | `/tenants/{id}` | Deactivate a tenant |
| `POST` | `/tenants/{id}/api-keys` | Create a tenant API key |
| `GET` | `/tenants/{id}/api-keys` | List tenant API keys |
| `DELETE` | `/tenants/{id}/api-keys/{key_id}` | Revoke a tenant API key |

## How It Works: Upload to Execution

```
 Upload (.cloacina)       Persist              Reconcile              Execute
 ─────────────────── → ───────────────── → ───────────────────── → ──────────────
 POST /workflows/       workflow_registry    Reconciler (every 5s)   POST /executions
   packages               (binary)           loads .so, registers     → scheduler
                        workflow_packages      tasks + workflow        → dispatcher
                          (metadata)           in global registry     → task executor
                        global task                                   → FFI call
                          registry
```

1. **Upload**: Multipart POST sends the `.cloacina` tar.gz. The server validates symbols, extracts metadata, and persists both binary and metadata to Postgres.
2. **Persist**: Binary data goes to `workflow_registry`; package name, version, task list, and dependency graph go to `workflow_packages`. Tasks are registered in the process-global task registry.
3. **Reconcile**: The registry reconciler runs every 5 seconds. It detects new packages, loads the shared library, and registers a workflow constructor in the global workflow registry — making the workflow available for `POST /executions`.
4. **Execute**: The scheduler creates a pipeline execution, the dispatcher routes tasks to the executor, and each task runs through the `cloacina_execute_task` FFI entry point with a full tokio runtime context.

## Next Steps

- [**Tutorial 22: Local Daemon Scheduler**]({{< ref "/tutorials/22-daemon-local-scheduler" >}}) — The same workflow lifecycle without Docker or Postgres, using SQLite and CLI commands
- Run the containerized soak test with `angreal soak` to validate the full pipeline under load
- Set up [continuous scheduling]({{< ref "/tutorials/12-continuous-scheduling" >}}) for reactive data-driven pipelines
- Explore the [Swagger UI](http://localhost:8080/api-docs/) for interactive API exploration
