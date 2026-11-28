---
title: "Decommission a tenant"
description: "Safely remove a tenant from a running cloacina-server: revoke keys, drain runner, evict caches, remove agents, delete the tenant's rows, drop schema."
sidebar:
  order: 80
---

This recipe walks through removing a tenant from a running `cloacina-server` cleanly — closing the auth surface, draining in-flight work, evicting cached connections, removing the tenant's agents and rows, and dropping the schema. The server's `DELETE /v1/tenants/{name}` route performs all steps in order with bounded drain semantics per CLOACI-T-0581. A tenant made later with the same name starts with nothing of the removed one (CLOACINA-T-1095).

For the conceptual model (why the orchestration is structured this way), see [Multi-tenancy](../../../explanation/service/multi-tenancy/).

## Prerequisites

- An `is_admin` API key for the running `cloacina-server`.
- The tenant name (= schema name) you want to remove.
- A `cloacinactl` profile pointing at the target server with the admin key (see [Use CLI profiles](../use-cli-profiles/)).
- Verified that you actually want to delete this tenant. **This is destructive — the schema and all its workflow rows, execution history, schedules, and credentials are dropped.** There is no undo.

## Background

Server-side, the `DELETE /v1/tenants/{name}` route runs five steps in order, each emitting a structured audit event with duration:

1. **Revoke every still-active API key for the tenant** and delete its WebSocket tickets. Closes the auth surface so new requests against the tenant start failing immediately. The server clears its own key cache and sends a revocation notice (Postgres `NOTIFY cloacina_api_keys_revoked`); every other server clears its key cache when the notice arrives.
2. **Evict the tenant's `DefaultRunner` from `TenantRunnerCache`**, awaiting a bounded graceful drain (`--tenant-deletion-drain-timeout-s` server flag, default 30s). The runner stops its scheduler loop and closes its per-tenant DB pool. Past the timeout, the runner is **hard-evicted** — any task that ignored cooperative cancellation will error on its next DB write once step 5 lands.
3. **Evict the tenant's `Database` from `TenantDatabaseCache`.** Releases the per-tenant connection pool.
4. **Remove the tenant's agents and their rows**, holding the fleet lock so that no fleet tick starts an agent meanwhile (the delete waits up to 30 s for the lock). The actuator removes the agents of every pool (below), then the tenant's limit, desired count, pools and agent rows are deleted.
5. **Delete the tenant's rows in the admin schema and drop the schema + user via `DatabaseAdmin::remove_tenant`**, in one transaction: the rows listed below, one `tenant.removed` audit row, `DROP SCHEMA … CASCADE`, `DROP USER`, the tenant's build role (`DROP ROLE "<tenant>$build"`, CLOACINA-T-1063) and its server role (`DROP ROLE "<tenant>$server"`, CLOACINA-T-1080). A later tenant of the same name gets new roles. Each lock wait in this transaction is bounded to 5 s. A URL that the tenant's compiler held stops working here.
Between steps 3 and 4, a server that runs a compiler per tenant (see [Run cloacina-compiler in Production](../running-the-compiler/#how-many-compilers-a-deployment-runs)) removes the tenant's compiler: it deletes the Deployment `cloacina-compiler` and its NetworkPolicy in the tenant's namespace and writes an empty value over the Secret `cloacina-compiler-db`. If that fails, the delete answers 500 and the schema is not dropped. Each call of the delete writes the empty value when the Secret exists, also a retry that finds the Deployment already gone; a tenant that never had a compiler gets no Secret.

Each step is idempotent. If a step fails, earlier steps stay committed, and a retry resumes from the failure point.

### What the delete removes, and what stays

In the admin schema, the rows of the tenant are deleted: its API keys and their login sessions, WebSocket tickets, local accounts, agent limit, desired count, pools and agent rows, delivery outbox rows, reactor rows, and, under the tenant's org, its trusted keys, signing keys, package signatures, trust grants and any secrets. A tenant made again with the same name has no account, key, trusted key, pool or agent of the old one, and a package signed with an old trusted key is refused.

The audit rows of the tenant stay: they are the record. Step 5 writes one row `tenant.removed` for the tenant. `GET /v1/tenants/{name}/audit` gives only rows after the newest `tenant.removed` of that name, so a new tenant of the name does not read the old rows as its own. `GET /v1/audit?tenant={name}` (platform admin) gives all of them. The `tenant.delete` row of the route is a platform row (no tenant). Login throttle counters stay; they expire and grant nothing.

On Kubernetes, the tenant's namespace `cloacina-tenant-<name>` stays, emptied: the server has no `delete` on namespaces. The actuator deletes the Deployment and the NetworkPolicy of each pool, the tenant's NetworkPolicy `cloacina-agent-fleet`, and writes an empty value over the agent key in the Secret `cloacina-agent-key` with a patch, which makes no Secret when there is none. ServiceAccounts that pools named stay (the server has no `delete` on them). With the Docker actuator, every managed container of the tenant is stopped.

### How long a key of the deleted tenant still passes

On the server that runs the delete: not after step 1. On another server: until the revocation notice arrives. A server whose notice listener is disconnected clears its cache when the listener connects again; in the worst case its key cache answers for at most 30 s (the cache TTL). Another server stops its runner for the tenant at its next runner pass (every 30 s), also when a tenant of the same name was made again before that pass; it then makes a new runner for the new tenant.

### While a delete is part-done

- Keys of the tenant are refused (401).
- `POST /v1/tenants` with the name answers 409 `tenant_exists` until step 5 has run.
- A delete that stops in step 4 or 5 leaves no agents and, after step 4, no rows that start them. If step 5 fails (a lock held longer than 5 s, or the tenant's user owns objects outside its schema), the tenant's compiler can come back while its build queue has work, until the retry removes it again.
- A second `DELETE` finishes the job; a delete of a tenant that is gone answers `"removed"`.

### Tenants an earlier server answered "removed" for

A server before CLOACINA-T-1068 answered `"removed"` and left the schema, the user and the rows. Such a tenant is still listed: `cloacinactl tenant list`, or `SELECT nspname FROM pg_namespace` as the admin user. Delete each name again with this server; the delete removes the schema, the user and the rows.

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
  | grep -E "tenant\.teardown\.(keys_revoked|runner_evicted|db_cache_evicted|fleet_removed|schema_dropped|completed|failed)"
```

You should see six events for a successful teardown (one per step plus the overall outcome).

**Postgres confirms the schema is dropped.** From a `psql` session as the admin user:

```sql
SELECT schema_name FROM information_schema.schemata
 WHERE schema_name = 'tenant_acme';
-- expect: 0 rows
```

## Recovery if teardown half-completes

If the HTTP response is a 4xx/5xx error rather than a success body, one or more steps failed. Earlier steps are still committed (each step is idempotent), so:

- **Re-run the delete.** `cloacinactl tenant delete <name> --force` is safe to retry. It will pick up from the failure point.
- **Read the audit log** to see which step failed and why. Common failures: Postgres permission denied on `DROP SCHEMA` (admin user lost CREATEROLE); a connection that holds a lock in the tenant's schema for more than 5 s (another server's runner before its runner pass); the tenant's user owns objects outside its schema (remove them, then delete again); or runner drain hung past timeout and was hard-evicted (visible as `runner_evicted: true` with an elevated step duration — not a failure, but worth noting in change-log).
- **Last resort**: drop the schema manually via `psql`. The server's caches will hold stale entries until the next restart in this case, so prefer the route-driven path whenever possible.

## What this how-to does NOT cover

- **Backing up tenant data before deletion.** Use `pg_dump --schema=tenant_acme` before issuing the delete if you want a snapshot.
- **Bulk tenant decommissioning.** The route is one-tenant-at-a-time. For batch cleanup, script `cloacinactl tenant list -o id` piped into `xargs -n1 cloacinactl tenant delete --force`. Allow time between deletions for the runner drain.
- **Migrating workflows out of the tenant first.** If you want to preserve workflow definitions in another tenant, export package archives via `cloacinactl package list` + `package inspect` before the delete.

## See also

- [Multi-tenancy](../../../explanation/service/multi-tenancy/) — conceptual model for why the orchestration is structured this way.
- [HTTP API Reference](../../../reference/http-api/#delete-v1tenantsschema_name) — full route shape and response body fields.
- [Configure multi-tenant deployment](../configure-multi-tenant-deployment/) — provisioning side (the inverse of this recipe).
- [DatabaseAdmin API Reference](../../../reference/database-admin/) — what step 4 calls under the hood.
- **CLOACI-I-0106** — multi-tenant abstraction initiative.
- **CLOACI-T-0581** — teardown orchestration.
- **CLOACINA-T-1095** — the tenant's rows, agents and key caches go with it.
