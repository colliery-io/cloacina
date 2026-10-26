---
title: "Put the Connectors on a Server"
description: "Where cloacina's first-party Sync connectors live, how a local server gets them, and how to add a connector of your own."
sidebar:
  order: 5
---

A sync runs its connectors where its tasks run: on the server, or on its
agents when it has a fleet. Every process that runs Sync tasks needs the
connector packages on its provider path, `CLOACINA_PROVIDER_PATH`.

## In the images

The server and agent images carry the first-party connectors, built for the
image's cloacina version:

| Package | Roles |
| --- | --- |
| `cloacina-connector-postgres` | source, destination |
| `cloacina-connector-mssql` | source |
| `cloacina-connector-s3` | source |
| `cloacina-connector-snowflake` | source, destination |
| `cloacina-connector-rest` | source, the runtime of every source definition |
| `cloacina-connector-rest-dest` | destination, the runtime of every destination definition |

They are in `/opt/cloacina/providers`, and the images set
`CLOACINA_PROVIDER_PATH` there. Nothing else is needed: the Syncs page's
catalog lists them.

## On a local server

Build and stage them once:

```bash
angreal connectors stage --out ~/.cloacina/providers
CLOACINA_PROVIDER_PATH=~/.cloacina/providers cloacina-server …
```

`--only postgres,rest` stages a subset. Without `--out`, the task stages into
`$CLOACINA_PROVIDER_PATH`, or `./providers`. It needs the `wasm32-wasip2`
Rust target (`rustup target add wasm32-wasip2`). Run it again after you
change a connector. An agent needs the same directory.

## Add a connector of your own

A connector is a provider package: a directory with its wasm component, a
`package.toml` and a `provider.json` that names its members, their roles and
sync modes. `scripts/stage_connectors.py` shows the layout. Put the directory
next to the first-party ones on the provider path; on an image, mount a
volume over `/opt/cloacina/providers` with all of them, or point
`CLOACINA_PROVIDER_PATH` at your own directory.

To check a new connector before it ships, see
[Test a Connector with the Conformance Kit](../test-a-connector/).
