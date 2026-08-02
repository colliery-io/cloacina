---
title: "01 - Deploy a Server"
description: "Bootstrap a cloacina-server, capture the admin key, create a tenant, upload a package, and run your first execution."
weight: 11
aliases:
  - "/platform/tutorials/01-deploy-a-server/"

---

In this tutorial you'll stand up a `cloacina-server` from scratch,
bootstrap an admin API key, create a tenant, upload a packaged
workflow, run an execution, and verify it via the metrics and health
endpoints. By the end you'll have the operator's mental model of how
the pieces fit together.

## What you'll learn

- How to start `cloacina-server` against a fresh PostgreSQL or SQLite
  backend.
- How the bootstrap key is generated and where to find it (and why
  you only get to capture it once).
- How to provision a tenant and a tenant-scoped API key.
- How to upload a `.cloacina` package and trigger an execution.
- How to confirm the execution via the HTTP API, Prometheus metrics,
  and structured logs.

## Prerequisites

- `cloacinactl` and `cloacina-server` binaries on your `PATH` (or
  built locally and accessible via `cargo run`).
- PostgreSQL 14+ accessible from the server, **or** willingness to
  use SQLite for this tutorial. Multi-tenant production deployments
  require Postgres; for a single-tenant first-run, SQLite is fine.
- `curl` and `jq` for ad-hoc HTTP calls and JSON field extraction.

You do **not** need a pre-built `.cloacina` package — you'll scaffold
one with `cloacinactl package new` in Step 7.

## Time estimate

15–20 minutes.

---

## Step 1: Prepare the database

### PostgreSQL path

```bash
createdb cloacina
psql -d cloacina -c "CREATE USER cloacina WITH PASSWORD 'changeme';"
psql -d cloacina -c "GRANT ALL PRIVILEGES ON DATABASE cloacina TO cloacina;"

export DATABASE_URL='postgres://cloacina:changeme@localhost/cloacina'
```

### SQLite path

No setup needed; the server will create the file on first start.

```bash
export DATABASE_URL='sqlite:///tmp/cloacina-tutorial.db'
```

Pick one. The rest of the tutorial uses `$DATABASE_URL`.

## Step 2: Start the server

```bash
cloacinactl server start --bind 127.0.0.1:8080 --database-url "$DATABASE_URL"
```

You'll see output like:

```text
Starting API server
  Bind:     127.0.0.1:8080
  Database: postgres://cloacina:***@localhost/cloacina
  Home:     /home/you/.cloacina
WARNING: Server running without TLS

API server is running on http://127.0.0.1:8080
  GET  /health     — liveness check
  GET  /ready      — readiness check
  GET  /metrics    — Prometheus metrics
  ...
```

The server runs in the foreground for this tutorial. Open a second
terminal for the next steps.

## Step 3: Capture the bootstrap admin key

On first startup with no API keys in the database, the server
generated an admin key and wrote it to `~/.cloacina/bootstrap-key`
with mode `0600`. **This is the only time the plaintext is
surfaced.**

```bash
ADMIN_KEY=$(cat ~/.cloacina/bootstrap-key)
echo "Admin key captured: ${ADMIN_KEY:0:8}..."
```

> **Production note:** in a real deployment, capture this key into
> your secret manager immediately and either delete the file or
> ensure it's only readable by the operator. The bootstrap path is
> skipped on subsequent starts; if you lose the key, recovery
> requires direct database access.

Confirm the server is up:

```bash
curl -s http://127.0.0.1:8080/health | jq .
# {"status": "ok"}

curl -s http://127.0.0.1:8080/ready | jq .
# {"status": "ready"}
```

## Step 4: Create a tenant

Tenants are isolated PostgreSQL schemas. For SQLite, the "tenant"
is a logical name on the same database.

```bash
cloacinactl tenant create acme \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"
```

Response (the password is **not** returned per security policy):

```json
{"name": "acme", "username": "acme_user", "description": null}
```

List tenants to confirm:

```bash
cloacinactl tenant list \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"
# acme
```

## Step 5: Create a tenant-scoped API key

Tenant-scoped keys are how application clients authenticate. They
can't escalate to other tenants. Keys for a **named** tenant are
minted on the tenant-scoped endpoint
`POST /v1/tenants/{tenant_id}/keys` — the plaintext is in the `key`
field of the response:

```bash
ACME_KEY=$(curl -s -X POST http://127.0.0.1:8080/v1/tenants/acme/keys \
    -H "Authorization: Bearer $ADMIN_KEY" \
    -H "Content-Type: application/json" \
    -d '{"name": "acme-tutorial", "role": "write"}' | jq -r '.key')

echo "Acme key captured: ${ACME_KEY:0:8}..."
```

> The response shows the plaintext `key` **exactly once**. Capture it
> now or recreate it later. Subsequent key listings return metadata
> (`id`, `name`, role) but never the plaintext — the `id` UUID is
> *not* the key.
>
> Note: `cloacinactl key create` hits the global `POST /v1/auth/keys`
> endpoint, which always mints keys for the built-in `public` tenant.
> For a named tenant like `acme`, use the tenant-scoped endpoint as
> above.

## Step 6: Configure a profile

Save the credentials in `~/.cloacina/config.toml` so subsequent
commands don't need every flag:

```bash
cloacinactl config profile set acme-prod \
    --server http://127.0.0.1:8080 \
    --api-key "$ACME_KEY"

cloacinactl config profile use acme-prod
```

Now `cloacinactl` reads the server URL and key from the profile by
default. The key is stored as a literal string; for production use
the `env:VAR` or `file:PATH` schemes documented in [API Key
Schemes]({{< ref "/reference/cli" >}}#api-key-schemes).

## Step 7: Upload a packaged workflow

Scaffold a Python packaged workflow (Python is the default `--lang`;
it needs no compiler service — the server loads Python packages
directly, while Rust packages additionally require a running
`cloacina-compiler` service to build them):

```bash
cloacinactl package new hello-world
# created python workflow package at hello-world
# next: cloacinactl package validate hello-world

cloacinactl package validate hello-world
cloacinactl package pack hello-world
# Prints the path of the produced .cloacina archive
```

The scaffold contains a two-task workflow named `hello_world`
(`hello` → `goodbye`) using bare `@cloaca.task` decorators — the
canonical packaged-workflow shape. Upload the archive:

```bash
cloacinactl package upload hello-world/hello-world.cloacina --tenant acme
```

Response:

```json
{"package_id": "f47ac10b-...", "tenant_id": "acme"}
```

The reconciler runs through its [six-step pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}})
to load the package. Watch the server logs in your first terminal.
You'll see lines like:

```text
INFO loading package package_id=f47ac10b-...
DEBUG step_load_cron_triggers: 0 cron schedules
DEBUG step_load_custom_triggers: 0 triggers
DEBUG step_load_reactors: 0 reactors
DEBUG step_load_triggerless_cgs: 0 trigger-less graphs
DEBUG step_load_reactor_bound_cgs: 0 graphs
DEBUG step_load_workflows: 1 workflow registered (hello_world)
INFO  package loaded successfully
```

The `0` counts reflect what your specific example package declares;
a more elaborate package would show non-zero counts at each step.
The `package loaded successfully` line is the signal that all six
steps completed.

Verify the workflow is visible (allow a couple of seconds for the
reconciler to run if you're polling immediately after upload):

```bash
cloacinactl workflow list --tenant acme
# hello_world  v0.1.0  (description, task count)
```

## Step 8: Run an execution

`--context` takes a **file path** (or `-` for stdin) containing the
initial context JSON — not inline JSON:

```bash
echo '{"input": "hello"}' > /tmp/context.json

cloacinactl workflow run hello_world --tenant acme --context /tmp/context.json
# 7d8e9f0a-1b2c-3d4e-5f60-718293a4b5c6   (the execution ID)
```

Capture the execution ID and poll its status:

```bash
EXEC_ID="7d8e9f0a-1b2c-3d4e-5f60-718293a4b5c6"

cloacinactl execution status "$EXEC_ID" --tenant acme
# Status: Running
# ...

# After a few seconds:
cloacinactl execution status "$EXEC_ID" --tenant acme
# Status: Completed
```

Inspect the per-task event log:

```bash
cloacinactl execution events "$EXEC_ID" --tenant acme
# task_started, task_completed, ... per task
```

## Step 9: Verify via metrics

The server emits Prometheus metrics on every workflow and task
event. Snapshot them:

```bash
curl -s http://127.0.0.1:8080/metrics | grep -E '^cloacina_(workflows|tasks)_total'
```

Expected output (numbers vary):

```text
cloacina_workflows_total{status="completed",reason="ok"} 1
cloacina_tasks_total{status="completed",reason="ok"} 2
```

The `1` counts your one execution; `2` counts the two tasks in
the scaffolded workflow.

## Step 10: Verify via structured logs

Tail the JSON log:

```bash
tail -n 20 ~/.cloacina/logs/cloacina-server.log | jq .
```

Look for lines with your `request_id` (set in the
`x-request-id` response header on every request) and the workflow's
`execution_id`. This is how you correlate a specific API call to
the server-side handling, including any internal failures.

For more on observability, see [Observe Execution State]({{< ref "/embed/how-to/observe-execution-state" >}}).

## Step 11: Tear down

For this tutorial, stop the server with Ctrl-C in the first
terminal. To clean up:

```bash
# Drop the tenant (Postgres path)
cloacinactl tenant delete acme --force \
    --server http://127.0.0.1:8080 \
    --api-key "$ADMIN_KEY"

# Drop the package (alternative: just stop the server; the package
# stays in the registry for the next startup)
cloacinactl package delete <package_id> --tenant acme

# Remove SQLite file (SQLite path)
rm /tmp/cloacina-tutorial.db

# Forget the profile
cloacinactl config profile delete acme-prod
```

## What you've built

You now have:

- A running `cloacina-server` against either Postgres or SQLite.
- A bootstrap admin key (which you've captured into your shell or
  secret manager).
- A tenant `acme` with its own scoped API key.
- A `cloacinactl` profile that uses the tenant key by default.
- A loaded packaged workflow that has executed at least once.
- Visibility via the HTTP API, Prometheus metrics, and structured
  logs.

## Where to go next

> **Planning to add the web UI?** The server **rejects browser requests
> cross-origin by default.** Before you point the web UI at this server, allow
> its origin. With `cloacinactl server start` (used above), set the environment
> variable before starting:
> `CLOACINA_CORS_ALLOWED_ORIGINS=https://ui.example.com` (the value is the URL
> users load the UI from). The equivalent `--cors-allowed-origins` flag is
> available when you run the `cloacina-server` binary directly. See
> [Deploy the Web UI]({{< ref "/service/how-to/deploy-the-web-ui" >}})
> for the full wiring.

- **Next: [02 — The Web UI]({{< ref "/service/tutorials/02-the-web-ui" >}})** — operate and observe this server from the browser.
- [Configure a Multi-Tenant Deployment]({{< ref "/service/how-to/configure-multi-tenant-deployment" >}})
  — productionize multi-tenancy and learn the operational caveats
  (the runner-schema execution gap is critical for true isolation).
- [Production Deployment]({{< ref "/service/how-to/production-deployment" >}})
  — add TLS termination, reverse proxy, and external secret
  management.
- [Observe Execution State]({{< ref "/embed/how-to/observe-execution-state" >}})
  — wire metrics into Prometheus + OpenTelemetry tracing.
- [Reconciler Pipeline]({{< ref "/service/explanation/reconciler-pipeline" >}})
  — understand what just happened during the package upload.
