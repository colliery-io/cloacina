---
title: "Add Connector Definitions to a Server"
description: "Load your own declarative connectors next to the corpus, and sync from one."
sidebar:
  order: 30
---

A server knows the [corpus](../../../reference/connectors/corpus/) and the
definitions in one directory of your own.

## 1. Give the server and its agents the directory

Set `CLOACINA_SYNC_DEFINITIONS_DIR` on the server **and** on every execution
agent, to a directory holding `*.yaml` files. Each file is one of:

- a cloacina [connector definition](../../../reference/connectors/definitions/)
  (a source with `streams`, or a destination with `objects`);
- an Airbyte low-code source (`type: DeclarativeSource`), imported when it is
  loaded.

A definition is named by its file's stem. A name the corpus also has replaces
the corpus entry. The directory is read once, when a process first needs it:
restart the server and the agents after a change. A file that does not load
stops every declarative sync with its error, so check a file with
`cloacinactl connector import` first.

## 2. See it in the catalog

```bash
cloacinactl connector list
```

The catalog lists the compiled connector providers and every definition, with
its roles and streams.

## 3. Store the secret and create the sync

```bash
cloacinactl secret create orders-api -f api_key=@/run/secrets/orders_key
```

Then create the sync definition, an instance of the `sync` workflow:

```json
{
  "source_definition": "orders",
  "source_config": {"subdomain": "acme"},
  "source_secret": {"$secret": "orders-api"},
  "destination_provider": "cloacina-connector-postgres",
  "destination_member": "destination",
  "destination_config": "{\"url\": \"postgres://...\", \"table\": \"orders\"}",
  "destination_grants": "{\"tcp\": [\"db.example.com:5432\"]}",
  "stream": "{\"stream\": \"orders\", \"sync_mode\": \"Incremental\", \"cursor_field\": \"updated_at\", \"primary_key\": [\"id\"], \"write_mode\": {\"Upsert\": {\"business_keys\": [\"id\"]}}}"
}
```

The server lowers the source before it creates the instance: a misspelled
definition, a stream the definition lacks, or a missing config key is a `400`
naming it. The source's grants default to the definition's host.
