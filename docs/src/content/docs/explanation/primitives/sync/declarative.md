---
title: "Declarative Connectors"
description: "A connector written as a YAML connector definition and run on the shared REST runtime, with nothing compiled."
sidebar:
  order: 20
---

A **declarative connector** is a YAML **connector definition** run on one
shared REST runtime: the `rest` source and the `rest-dest` destination, two
WASM components that ship with cloacina. A definition says where an API is,
how to page through it, how to read incrementally, and how to authenticate.
Nothing is compiled per connector (COLLIERY-I-0634).

## A definition becomes a connector

A sync side names a definition instead of a provider:

```json
{
  "source_definition": "github",
  "source_config": {"base_url": "https://api.github.com"},
  "source_secret": "github-token",
  "stream": "{\"stream\": \"repositories\", \"sync_mode\": \"Incremental\", \"cursor_field\": \"updated_at\", ...}"
}
```

Wherever the sync's params are read, in each Sync task on the agent and on the
server when the sync definition is created, the side is **lowered** onto the
REST runtime:

1. The definition is found by name, and the stream by the stream's name (a
   destination's object, the same way).
2. The **compiler** renders the definition's `{{ config['key'] }}` templates
   against the side's config. A key the config lacks is an error that names
   it.
3. The result is an ordinary sync side: the `cloacina-connector-rest`
   provider, a typed `RestConfig` as its config, the definition's mapping
   ahead of the stream's, the definition's schema when the sync gives none,
   grants to the definition's host unless the side gives its own, and the
   credential the definition's auth names.

From there a declarative side runs like any other: the read task loads the
REST guest with the compiled config, and the host adds the credential to its
requests (see [Credential Injection](../credentials/)).

The server checks the lowering when the sync definition is created, so a
misspelled definition, an unknown stream or a missing config key is a `400`,
not a failed segment later.

## The config a side gives

A declarative side's `*_config` is the user's config for the definition:

- Values for the definition's templates, such as `subdomain` or `base_id`.
- `base_url`, to replace the definition's: a self-hosted API, a proxy, a test
  mock.
- `max_pages` and `max_retries`, to replace the runtime's defaults (1000 and
  4).

Secrets are not in the config. The definition's auth names the fields of a
secret, and `*_secret` names the secret (default: the binding `source_secret`
or `destination_secret`, so an instance can bind `source_secret: {"$secret":
"<name>"}` and nothing else).

## Where definitions come from

- **The corpus**: 36 sources and 2 destinations that ship with cloacina,
  embedded in the `cloacina-declarative` crate, so a server and its agents
  always agree on them. See [the corpus](../../../../reference/connectors/corpus/).
- **A definitions directory**: the `*.yaml` files of the directory named by
  `CLOACINA_SYNC_DEFINITIONS_DIR`. A file there replaces a corpus entry of the
  same name. A server and its agents must see the same directory.

A file is either cloacina's own [connector definition
format](../../../../reference/connectors/definitions/) or an Airbyte low-code
source, which is imported when it is loaded (see [Import an Airbyte
Connector](../../../../how-to/sync/import-an-airbyte-connector/)).

## The catalog

The catalog is the connectors a server can run: the compiled connector
providers it has, and its definitions. `GET /v1/tenants/{tenant_id}/connectors`
lists them, as does `cloacinactl connector list`.

## Related

- [Sync](../) for how a sync runs.
- [REST Source](../../../../reference/connectors/rest/) and
  [REST Destination](../../../../reference/connectors/rest-dest/) for what the
  runtime does.
