---
title: "08 — Your First Sync"
description: "Copy exchange rates from the Frankfurter API into Postgres with a sync, from the web UI."
sidebar:
  order: 18
---

In this tutorial you start a fresh server and, from the web UI only, set up a
sync that copies the latest exchange rates from the
[Frankfurter](https://frankfurter.dev) API into a Postgres table. Then you run
it and look at what it did.

## What you'll learn

- How to make a sync in the UI: a source, a stream, a destination.
- How Check and discovery tell you a side works before you save.
- How to run a sync by hand and read its passes.

## Prerequisites

- Docker with Compose.
- A checkout of the cloacina repository, for the compose file.
- Internet access from the server container, to reach `api.frankfurter.dev`.

## Step 1 — Start a server

The compose file in `deploy/docker-compose/` runs Postgres, the server and
the compiler. The server image carries the web UI and the first-party Sync
connectors (see [Put the Connectors on a Server](../../../how-to/sync/install-connectors/)).

```bash
docker compose -f deploy/docker-compose/cloacina.yml up -d
```

On its first start the server writes an admin key. Read it:

```bash
docker compose -f deploy/docker-compose/cloacina.yml \
  exec cloacina-server cat /home/cloacina/.cloacina/bootstrap-key
```

Open <http://localhost:8080> and connect:

- **Server URL:** `http://localhost:8080`
- **API key:** the key you just read
- **Tenant:** `public`

## Step 2 — The source

Open **Syncs** in the side bar and press **+ New sync**.

1. **Name:** `fx-rates`.
2. **Source:** `frankfurter (definition)`. Frankfurter is a connector
   definition: a description of a REST API that the REST connector runs.
   It needs no key and no config. Its `base_url` field is only for pointing
   it at another host; leave it empty.
3. Press **Check** under the source. "✓ The connector answered." means the
   server reached the API.

The form discovers the source's streams. Frankfurter has four.

## Step 3 — The stream

1. **Stream:** `latest`, today's rates, one row per currency pair.
2. **Sync mode:** `FullRefresh`. Each run reads the whole stream.
3. **Write mode:** `Upsert`, so a second run updates rows and adds none.
4. **Business keys:** `date, base, quote`. These pick the row an upsert
   updates.

## Step 4 — The destination

1. **Destination:** `cloacina-connector-postgres`. The form loads the
   connector's fields.
2. **url:** `postgres://cloacina:cloacina@postgres:5432/cloacina?sslmode=disable`.
   That is the compose Postgres, by its service name. The field is masked
   when the sync is read back.
3. **table:** `fx_rates`. The connector makes the table on the first write.
4. **sslmode:** `disable`. The compose Postgres has no TLS.
5. **Grants (JSON):** `{"tcp": ["postgres:5432"]}`. A connector runs in a
   sandbox and reaches only what its grants name. Without grants it can
   reach nothing.
6. Press **Check** under the destination.

Leave **Cron** empty: the sync then runs only when you start it. Press
**Create sync**.

## Step 5 — Run it

The sync's page opens. Press **▸ Run now**.

A run of a sync is a pass. The **Passes** tab shows it: Running, then
Completed once every segment of the stream is read and written. Press the
pass to see its segments, each an execution of the `sync` workflow.

- **Stream and schema** shows the stream's state: its cursor, and whether
  the last pass read it to the end. A full refresh keeps no cursor; an
  incremental sync stores the last value of its cursor field here and reads on
  from it. The tab also shows the fields the sync found and stored. When a
  later run sees a field change type, it stops and shows the drift here for
  you to accept.
- **Dead-letters** holds records the destination refused, with the reason.
  It should be empty.

## Step 6 — Look at the rows

```bash
docker compose -f deploy/docker-compose/cloacina.yml exec postgres \
  psql -U cloacina -c 'SELECT date, base, quote, rate FROM fx_rates ORDER BY quote LIMIT 5'
```

Run the sync again. The row count stays the same: the upsert updates the
rows by their business keys.

## Where next

- Set a **Cron**, for example `0 17 * * 1-5`, on the sync's **Edit** page to
  run it each weekday.
- [Connector definitions](../../../reference/connectors/definitions/) and
  [the definitions that ship](../../../reference/connectors/corpus/).
- [Sync](../../../explanation/primitives/sync/): how passes, segments and
  stream state work.
