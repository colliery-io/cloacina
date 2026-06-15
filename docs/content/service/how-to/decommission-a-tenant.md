---
title: "Decommission a tenant"
description: "Safely remove a tenant from a running cloacina-server using the 4-step teardown orchestration: revoke keys, drain runner, evict caches, drop schema."
weight: 80
---

# Decommission a tenant

This recipe walks through removing a tenant from a running `cloacina-server` cleanly — closing the auth surface, draining in-flight work, evicting cached connections, and dropping the schema. The server's `DELETE /v1/tenants/{name}` route performs all four steps in order with bounded drain semantics per CLOACI-T-0581.

For the conceptual model (why the orchestration is structured this way), see [Multi-tenancy]({{< ref "/service/explanation/multi-tenancy" >}}).

## Prerequisites

- An `is_admin` API key for the running `cloacina-server`.
- The tenant name (= schema name) you want to remove.
- A `cloacinactl` profile pointing at the target server with the admin key (see [Use CLI profiles]({{< ref "use-cli-profiles" >}})).
- Verified that you actually want to delete this tenant. **This is destructive — the schema and all its workflow rows, execution history, schedules, and credentials are dropped.** There is no undo.

## Background

Server-side, the `DELETE /v1/tenants/{name}` route runs four steps in order, each emitting a structured audit event with duration:

1. **Revoke every still-active API key for the tenant.** Closes the auth surface so new requests against the tenant start failing immediately. Bypasses the auth-cache TTL by clearing the entire cache (this is the same path `key revoke` uses).
2. **Evict the tenant's `DefaultRunner` from `TenantRunnerCache`**, awaiting a bounded graceful drain (`--tenant-deletion-drain-timeout-s` server flag, default 30s). The runner stops its scheduler loop and closes its per-tenant DB pool. Past the timeout, the runner is **hard-evicted** — any task that ignored cooperative cancellation will error on its next DB write once step 4 lands.
3. **Evict the tenant's `Database` from `TenantDatabaseCache`.** Releases the per-tenant connection pool.
4. **Drop the schema + user via `DatabaseAdmin::remove_tenant`** (`DROP SCHEMA … CASCADE` + `DROP USER`).

Each step is idempotent. If a step fails, earlier steps stay committed, and a retry resumes from the failure point.

## Steps

### 1. (Optional) Stop sending new work to the tenant

If you control the upstream callers, pause their cron schedules or trigger registrations against this tenant before starting the teardown. This is not strictly necessary — step 1 of the teardown revokes the auth surface — but it makes the audit log cleaner by avoiding a burst of 401s in the seconds before the revocation lands.

### 2. Verify the tenant exists

```sh
cloacinactl --profile prod --json tenant list | jq -r '.[] | .name' | grep -F tenant_acme
```

If `grep` returns no output, the tenant is already gone (or never existed under that name); stop here.

### 3. Tune the drain timeout (optional)

The default 30-second drain is fine for typical tenants. If you know the tenant has long-running workflows you want to let finish, restart the server with a larger `--tenant-deletion-drain-timeout-s`:

```sh
cloacinactl server stop
cloacinactl server start \
  --bind 127.0.0.1:8080 \
  --database-url "$DATABASE_URL" \
  --tenant-deletion-drain-timeout-s 300   # 5 minutes
```

If you want fast teardown and are willing to have non-cancellation-aware tasks fail mid-flight, leave the default (or shorten it).

### 4. Issue the delete

```sh
cloacinactl --profile prod tenant delete tenant_acme
```

The CLI prompts for interactive confirmation unless you pass `--force`. The server returns a JSON body summarizing the teardown:

```json
{
  "status": "removed",
  "schema_name": "tenant_acme",
  "revoked_keys": 3,
  "runner_evicted": true,
  "db_cache_evicted": true
}
```

- `revoked_keys` — number of API keys revoked in step 1.
- `runner_evicted` / `db_cache_evicted` — `true` if the cache had a live entry at teardown time, `false` if the cache was cold (still a successful teardown).

### 5. Verify the teardown completed

Three orthogonal checks:

**Schema is gone.** The server should 404 every request against the deleted tenant:

```sh
cloacinactl --profile prod --tenant tenant_acme workflow list
# expect: HTTP 403/404 from server (tenant access denied or not found),
# NOT a stale-pool error (which would indicate step 3 failed silently)
```

**Audit events landed.** Tail the server logs for the structured teardown events:

```sh
journalctl -u cloacina-server --since "5 minutes ago" \
  | grep -E "tenant_teardown_(keys_revoked|runner_evicted|db_cache_evicted|schema_dropped|complete)"
```

You should see five events for a successful teardown (one per step plus the overall outcome).

**Postgres confirms the schema is dropped.** From a `psql` session as the admin user:

```sql
SELECT schema_name FROM information_schema.schemata
 WHERE schema_name = 'tenant_acme';
-- expect: 0 rows
```

## Recovery if teardown half-completes

If the HTTP response is a 4xx/5xx error rather than a success body, one or more steps failed. Earlier steps are still committed (each step is idempotent), so:

- **Re-run the delete.** `cloacinactl tenant delete <name> --force` is safe to retry. It will pick up from the failure point.
- **Read the audit log** to see which step failed and why. Common failures: Postgres permission denied on `DROP SCHEMA` (admin user lost CREATEROLE), or runner drain hung past timeout and was hard-evicted (visible as `runner_evicted: true` with an elevated step duration — not a failure, but worth noting in change-log).
- **Last resort**: drop the schema manually via `psql`. The server's caches will hold stale entries until the next restart in this case, so prefer the route-driven path whenever possible.

## What this how-to does NOT cover

- **Backing up tenant data before deletion.** Use `pg_dump --schema=tenant_acme` before issuing the delete if you want a snapshot.
- **Bulk tenant decommissioning.** The route is one-tenant-at-a-time. For batch cleanup, script `cloacinactl tenant list -o id` piped into `xargs -n1 cloacinactl tenant delete --force`. Allow time between deletions for the runner drain.
- **Migrating workflows out of the tenant first.** If you want to preserve workflow definitions in another tenant, export package archives via `cloacinactl package list` + `package inspect` before the delete.

## See also

- [Multi-tenancy]({{< ref "/service/explanation/multi-tenancy" >}}) — conceptual model for why the orchestration is structured this way.
- [HTTP API Reference]({{< ref "/platform/reference/http-api" >}}#delete-v1tenantsschema_name) — full route shape and response body fields.
- [Configure multi-tenant deployment]({{< ref "configure-multi-tenant-deployment" >}}) — provisioning side (the inverse of this recipe).
- [DatabaseAdmin API Reference]({{< ref "/platform/reference/database-admin" >}}) — what step 4 calls under the hood.
- **CLOACI-I-0106** — multi-tenant abstraction initiative.
- **CLOACI-T-0581** — 4-step teardown orchestration.
