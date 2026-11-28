---
title: "Compatibility Promise"
description: "What a patch, a minor and a major release of Cloacina may change, the schema rule, which agent and client versions a server accepts, the upgrade order, and why the upgrade to 0.13.0 is a full stop."
sidebar:
  order: 17
---

What one release of Cloacina keeps working from the release before it, and
what an operator does to upgrade. The steps themselves are in
[Upgrading the server](../../how-to/service/production-deployment/#upgrading-the-server)
and, on Kubernetes, in [Upgrades](../../how-to/service/deploying-to-kubernetes/#upgrades).
The check that holds each release to this page is the
[release gate](../../contributing/release-gate/).

## Version numbers

| Release | May change |
|---|---|
| A patch (0.13.1 to 0.13.2) | Nothing on this page. No migration, no change of the plugin interface or a protocol version, no removed route, field, chart value, flag or metric. Replace the images; no other step. |
| A minor (0.13 to 0.14) | Anything, with each break in `CHANGELOG.md` under `### Changed (breaking)` and the step the user takes, and each migration under `### Upgrade notes`. |
| A major | Cloacina is 0.x and has had no major. At 1.0 the minor takes the patch rule and the major takes the minor rule. |

- Upgrade one minor at a time: from any patch of one minor to any patch of
  the next. Patches inside a minor can be skipped.
- A release that breaks the patch rule is cut as a minor: the release gate
  fails a patch tag that lists any change.

## The database schema

- **Forward only.** No release reverts a migration. There is no downgrade:
  to go back, restore the backup taken before the upgrade.
- **The previous minor keeps working.** Each migration of a minor leaves the
  server of the minor before it working on the new schema. A column or table
  that the previous minor uses is removed one minor later.
- **An older server refuses a newer schema.** Each schema records the release
  that last migrated it. A server one minor behind the schema starts with no
  migration; one more than one minor behind, or of another major, refuses to
  start and names both releases. The table of cases is in
  [Schema versions](../../how-to/service/production-deployment/#schema-versions).
- **Only the server migrates** (and an embedded runner or
  `cloacinactl daemon`, which own their database). The compiler runs no
  migration; it waits for the server's.

## Version skew

| Part | Accepted by a server of minor N |
|---|---|
| Agents | minor N up to the server's own patch, and minor N−1. An older or newer agent is refused at register with both releases, and waits; its tasks wait for an agent that fits. See [Agent and server versions](../../how-to/service/deploy-an-execution-agent-fleet/#agent-and-server-versions). |
| `cloacinactl` and the SDKs | minor N and minor N−1; never newer than the server. The server refuses no client by version; `cloacinactl` warns once when its minor differs. See [`x-cloacina-version`](../http-api/#server-release-x-cloacina-version). |
| Rust packages | any package whose recorded plugin interface version an agent of the tenant loads. Dispatch sends a package only to such an agent; with none, the task waits. |
| Python packages | any agent: a Python package is source and has no plugin interface version. |

The plugin interface version changes only at a minor, named in the changelog
as `plugin_interface`. After such a minor, each team changes its package's
pin to the new minor and publishes the package again.

## Upgrade order

1. Back up the database.
2. Scale the compilers to zero and pause package uploads.
3. Upgrade the servers. With two or more replicas, roll them.
4. Start the compilers of the new release.
5. Upgrade the agents.
6. Upgrade `cloacinactl` and the SDKs.
7. Publish packages again, only when the changelog names a change of
   `plugin_interface`.

A compiler of the previous minor that runs during the roll is outside this
promise: step 2 keeps it from running.

## The upgrade to 0.13.0 is a full stop

The upgrade from 0.12 to 0.13.0 does not roll. Stop every server and every
compiler, then start 0.13.0. The service is down between the two.

Why:

- 0.13.0 moves the global tables (keys, accounts, audit, agents, pools and
  the agents' work packets) out of `public` into a schema of their own,
  `cloacina_global`, at its first start. A 0.12 server cannot run on that
  layout, and 0.12 has no schema check that would stop it.
- The schema rule above, that the previous minor keeps working on the new
  schema, first ships in 0.13.0. No 0.12 server keeps it.

What to expect, once:

- The reactive state of each named tenant's computation graphs (reactor
  state, accumulator checkpoints, boundaries and buffers, recovery events)
  starts empty. In 0.12 those rows were in `public` with no tenant, so the
  start cannot give them to their tenant. Each reactor and accumulator
  starts as at its first start. The graphs of `public` keep their state.
- `public`'s pending run-finished notices stay in `public.delivery_outbox`
  and are sent after the 0.13.0 start, when an alert webhook is set, as
  before.
- `public` now has its own connection pool beside the admin pool, so each
  replica opens more connections; named tenants stay at 2 each.

Agents of 0.12 are of the previous minor, so 0.13.0 accepts them: upgrade
them after the servers, as in the order above.

The rolling promise (the previous minor on the new schema while the replicas
roll) applies from 0.13.0 on: to the upgrade from 0.13 to 0.14 and later.
