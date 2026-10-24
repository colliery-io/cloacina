---
title: "Import an Airbyte Connector"
description: "Turn an Airbyte low-code source into a cloacina connector definition, and see what the REST runtime supports of it."
sidebar:
  order: 20
---

Many APIs already have an Airbyte low-code connector: a YAML file with
`type: DeclarativeSource`. `cloacinactl connector import` turns one into a
cloacina [connector definition](../../../reference/connectors/definitions/),
offline, with no server and no WASM runtime.

## 1. Import it

```bash
cloacinactl connector import airbyte/orders.yaml --out definitions/orders.yaml
```

The definition is written to `--out` (or stdout). The name is the file's stem,
or `--name`. A report goes to stderr:

```text
orders: tier A (confidence 1.00), streams: orders, customers
```

The tier is `A` when everything the file uses is supported, `B` or `C` with
gaps, `F` when it cannot be read. Each gap is listed as `not supported: ...`.
A gap means the runtime ignores that construct: read the report before you
trust the definition.

`--json` prints the report, the definition and any error as one object.

## 2. Check what it asks for

- **Auth**: an authenticator's `{{ config['key'] }}` becomes the name of the
  field of the sync's secret that holds the value. A bearer token read from
  `config['api_key']` becomes `auth: { kind: bearer, field: api_key }`, so the
  secret needs a field `api_key`.
- **Config**: the definition's remaining `{{ config['...'] }}` templates are
  what a sync's `source_config` must give.
- **Schema**: the importer needs an inline schema for each stream.

## 3. Use it

Put the file in the server's definitions directory (see [Add Connector
Definitions to a Server](../add-connector-definitions/)), or keep it beside an
embedded runner that sets the same directory. A sync then names it with
`source_definition: orders`.

What the importer reads, and what it maps each Airbyte construct to, is in
`cloacina_declarative::importer`.
