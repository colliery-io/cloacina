---
title: "REST Destination"
description: "The runtime of every declarative destination: record-by-record create, upsert and delete against a JSON API."
sidebar:
  order: 60
---

`connectors/rest-dest`, package `cloacina-connector-rest-dest`. The
destination half of the REST runtime: it writes one object of a destination
[connector definition](../definitions/), compiled into a
`cloacina_constructor_contract::sync::rest::RestDestConfig`. Moved from weir
(CLOACINA-T-0991).

## How it writes

- **One request per record.** The record is shaped by the object's
  `field_map` (identity when empty), nested under `body_wrap` when set, and
  sent to `path` with `{{ record.<field> }}` replaced by the record's value,
  percent-encoded.
- **The method** is the object's, or the write mode's: `PATCH` to upsert
  (create or update the keyed path, so a replay changes nothing), `POST`
  otherwise.
- **CDC**: inserts and updates write as above; a delete goes to
  `delete_path` with `delete_method` (default `DELETE`), and a `404` counts
  as done. A delete without a `delete_path` is a dead-letter.
- **A record the API refuses** (a 4xx) is a dead-letter with the status and
  the answer; the write goes on.
- **Retries**: 429, 5xx and transport failures retry with backoff, up to
  `max_retries`.
- **`check`** validates the config; there is no cheap probe without a write.

```bash
cargo test -p cloacina-sync --test rest_dest_connector
```
