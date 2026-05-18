---
title: "Safely Unload a Package"
description: "Step-by-step recipe for unloading a .cloacina package without leaving stale registrations or dangling subscribers."
weight: 25
---

# How to Safely Unload a Package

This guide shows how to unload a packaged workflow (`.cloacina`
archive) cleanly, including how to handle cross-package reactor
subscriptions and how to recover if an unload partially fails.

> **When to use this:** rolling back a deployment, replacing a package
> with a new version, decommissioning a tenant's workflow, or
> recovering from a failed load.

## Prerequisites

- A running `cloacina-server` (or `cloacina-daemon`) with the
  package already loaded.
- An admin or write-role API key for the tenant that owns the package
  (server mode), or local filesystem access (daemon mode).
- The package's UUID or `(name, version)` pair.

## Background

The reconciler runs a [six-step pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}})
on unload, in reverse order:

1. Workflows + tasks unregistered.
2. Reactor-bound CGs unbound from their reactors.
3. Trigger-less CGs unregistered.
4. Reactors torn down (scheduler-side + Runtime-constructor cleanup;
   see [Reactor Lifecycle]({{< ref "/computation-graphs/explanation/reactor-lifecycle" >}})).
5. Custom-poll triggers unregistered.
6. Cron schedules deleted from the database.

If any step fails, the reconciler reports the error and the package
stays in a partially-unloaded state. Rerunning unload picks up where
it left off.

## Recipe 1: Unload a Standalone Package

A standalone package owns its own reactors and CGs and has no
external subscribers.

```bash
# server mode
cloacinactl package delete <package-id> --tenant my-tenant

# daemon mode (move or delete the .cloacina file)
rm ~/.cloacina/packages/my-package-1.0.0.cloacina
```

The daemon's filesystem watcher fires within `watcher_debounce_ms`
(default 500 ms). The server applies the unload immediately on the
HTTP request. Verify the unload completed:

```bash
cloacinactl graph list --tenant my-tenant
# The package's graphs should no longer appear.

cloacinactl trigger list --tenant my-tenant
# Triggers from this package should be gone.
```

If `cloacinactl graph list` still shows the package's graph, the
unload did not run all six steps successfully — see [Recovering from a
Partial Unload](#recovering-from-a-partial-unload) below.

## Recipe 2: Unload a Reactor-Owning Package with External Subscribers

If package **A** owns a reactor `R` and package **B** has a CG bound
to `R`, unloading A directly will fail:

```text
Error: reactor 'R' has 1 bound subscriber(s): ['bar']; unbind them first
```

This is the bound-subscriber guard. The reconciler refuses to tear
down a reactor that still has live cross-package subscribers — doing
so would leave B's CG with no upstream and dangling references.

**Resolution:** unload the subscribers first.

```bash
# 1. Identify the subscribers from the error message (B's bound CGs).
#    Or query directly:
cloacinactl graph list --tenant subscriber-tenant

# 2. Unload the subscriber package.
cloacinactl package delete <package-B-id> --tenant subscriber-tenant

# 3. Confirm the reactor is now unbound.
cloacinactl graph status R --tenant publisher-tenant
# Should show subscribers: 0

# 4. Unload the publisher.
cloacinactl package delete <package-A-id> --tenant publisher-tenant
```

The reconciler does **not** automatically cascade unloads across
packages. It surfaces the bound-subscriber rejection so operators can
make the call about whether subscribers should genuinely be unloaded
or whether the publisher unload was a mistake.

## Recipe 3: Replace a Package In-Place

To deploy a new version of a package without downtime:

```bash
# Server mode: upload the new version. The reconciler diffs the
# package set against the database and unloads the old version while
# loading the new one.
cloacinactl package upload my-package-1.0.1.cloacina --tenant my-tenant

# Daemon mode: drop the new file in. The watcher detects the change.
mv my-package-1.0.1.cloacina ~/.cloacina/packages/
```

In-flight executions of the old version continue to completion. New
executions use the new version. The reconciler runs unload-then-load,
not a true atomic swap — there is a brief window where neither version
serves new requests. For zero-downtime upgrades, run two
`cloacina-server` instances behind a load balancer and swap one at a
time.

## Common Errors

### `reactor 'X' has N bound subscriber(s): [...]; unbind them first`

The bound-subscriber guard refused to tear down a reactor because
a CG (in this or another package) is still subscribed. Exact wire
format:

```text
reactor 'price_reactor' has 2 bound subscriber(s):
       ['analyzer_v1', 'aggregator']; unbind them first
```

**Resolution**: identify the named subscribers, unload the packages
that own them, then retry. See [Recipe 2](#recipe-2-unload-a-reactor-owning-package-with-external-subscribers)
above. The reconciler does *not* cascade unloads automatically.

### `reactor 'X' not loaded`

You're trying to unload (or bind to) a reactor that's not in the
scheduler. Possible causes:
- The publisher package never loaded (check the server log for
  load failures during the publisher's reconciliation).
- The publisher unloaded already (check via
  `cloacinactl graph list`).
- A typo in the reactor name (case-sensitive).

For unload paths, this is treated as a clean no-op — the runtime-
side constructor cleanup still runs. You don't need to do anything;
the warning is informational. For load paths (loading a subscriber
before the publisher exists), wait for the publisher to load or
re-upload it.

### `package <id> failed: <reason>`

The reconciler logged a failure mid-load. Re-run unload to clean up
any partial state, then investigate the failure. Common causes:
malformed manifest, cdylib that doesn't expose the expected FFI
methods, signature verification mismatch when
`--require-signatures` is set.

## Recovering from a Partial Unload

If the unload errors mid-pipeline (e.g., a cron schedule delete fails
because the database is briefly unreachable), the package is left in
an inconsistent state. The recovery procedure:

1. **Re-run the unload.** The reconciler is idempotent — steps that
   already completed succeed as no-ops.

   ```bash
   cloacinactl package delete <package-id> --tenant my-tenant
   ```

2. **Inspect runtime state.** Check that all six step categories are
   actually clean:

   ```bash
   # Workflows + tasks
   cloacinactl workflow list --tenant my-tenant
   # Reactor-bound CGs
   cloacinactl graph list --tenant my-tenant
   # Reactors (look for the package's reactor names in the graph
   # accumulator output)
   cloacinactl graph accumulators
   # Custom triggers + cron schedules
   cloacinactl trigger list --tenant my-tenant
   ```

3. **If a single resource is stuck**, delete it directly. Most
   commonly: a cron schedule whose row was orphaned because the
   reactor unload failed before the cron-delete step.

   ```bash
   # Inspect the trigger to get the schedule_id.
   cloacinactl trigger inspect <trigger-name> --tenant my-tenant
   # Manual cron delete via cloacinactl admin not yet implemented;
   # delete the row in the cron_schedules table directly if needed.
   ```

4. **As a last resort, restart the server.** This re-runs the
   reconciler from a clean state on startup; the database is the
   source of truth, so any orphaned in-memory state is dropped.

   > **Updated for CLOACI-T-0581.** The pre-2026 caveat that
   > "`TenantDatabaseCache` never evicts" no longer applies — the
   > `DELETE /v1/tenants/{name}` route runs a 4-step teardown that
   > evicts both `TenantRunnerCache` and `TenantDatabaseCache`
   > before dropping the schema. Restart is therefore only needed
   > here if a different code path (e.g., direct `psql` schema
   > manipulation that bypassed the route) left stale cache state.
   > See [Decommission a tenant]({{< ref "decommission-a-tenant" >}}) for the route-driven path.

## Verification Checklist

After every unload, verify clean teardown:

- [ ] `cloacinactl workflow list` does not show the package's
  workflows.
- [ ] `cloacinactl graph list` does not show the package's CGs.
- [ ] `cloacinactl graph accumulators` does not show the package's
  accumulators (they should disappear when the parent reactor
  teardown completes step 4 of unload).
- [ ] `cloacinactl trigger list` does not show the package's
  custom-poll or cron triggers.
- [ ] Server logs show no errors with `package_id=<id>` after the
  unload completion message.

## Related

- [Reconciler Pipeline]({{< ref "/platform/explanation/reconciler-pipeline" >}}) — full ordering and rationale.
- [Reactor Lifecycle]({{< ref "/computation-graphs/explanation/reactor-lifecycle" >}}) — the dual-layer reactor teardown.
- [Configure a Multi-Tenant Deployment]({{< ref "/platform/how-to-guides/configure-multi-tenant-deployment" >}}) — for tenant-aware unloads.
- [HTTP API Reference]({{< ref "/platform/reference/http-api" >}}) — the package delete endpoint.
